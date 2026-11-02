//! External-LLM refiner (plan feature 17, slice 2): a `Refiner`
//! implementation that asks an OpenAI- or Anthropic-compatible API to
//! name stripped functions from the same provider-agnostic contexts
//! the heuristic sees. All writes flow through the annotation contract
//! (actor `model:<provider>/<model>`, provenance, undoable); the pass
//! is dry-run by default and refuses to start without an API key.
//!
//! Configuration (environment):
//! - `DAMSEL_LLM_PROVIDER` — `openai` (default) | `anthropic` |
//!   `openai-compatible`
//! - `DAMSEL_LLM_API_KEY` — required for `--llm`
//! - `DAMSEL_LLM_MODEL` — default `gpt-4o-mini` / `claude-sonnet-4-5`
//! - `DAMSEL_LLM_BASE_URL` — override endpoint (self-hosted proxies)

use damsel_recon_catalog::refine::{FunctionContext, RefinementProposal, Refiner};
use serde_json::{Value, json};

/// Provider selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    OpenAi,
    Anthropic,
}

impl LlmProvider {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "openai" | "openai-compatible" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            _ => None,
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-4o-mini",
            Self::Anthropic => "claude-sonnet-4-5",
        }
    }

    fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
        }
    }
}

/// Resolved configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl LlmConfig {
    /// Read from the environment. `Err` carries the missing-variable
    /// names for the typed CLI error.
    pub fn from_env() -> Result<Self, String> {
        let provider = std::env::var("DAMSEL_LLM_PROVIDER")
            .ok()
            .map(|raw| {
                LlmProvider::parse(&raw)
                    .ok_or_else(|| format!("DAMSEL_LLM_PROVIDER must be openai|anthropic|openai-compatible, got `{raw}`"))
            })
            .transpose()?
            .unwrap_or(LlmProvider::OpenAi);
        let api_key = std::env::var("DAMSEL_LLM_API_KEY")
            .map_err(|_| "DAMSEL_LLM_API_KEY is not set (required for --llm)".to_string())?;
        if api_key.trim().is_empty() {
            return Err("DAMSEL_LLM_API_KEY is empty".to_string());
        }
        let model = std::env::var("DAMSEL_LLM_MODEL")
            .ok()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| provider.default_model().to_string());
        let base_url = std::env::var("DAMSEL_LLM_BASE_URL")
            .ok()
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| provider.default_base_url().to_string());
        Ok(Self {
            provider,
            api_key,
            model,
            base_url,
        })
    }
}

/// The prompt for a context batch — pure, tested.
pub fn build_prompt(contexts: &[FunctionContext]) -> String {
    let payload: Vec<Value> = contexts
        .iter()
        .map(|context| {
            json!({
                "address": format!("{:#x}", context.address),
                "current_name": context.current_name,
                "imports_called": context.imports.iter()
                    .map(|(name, count)| format!("{name} x{count}"))
                    .collect::<Vec<_>>(),
                "string_reference_count": context.string_refs,
            })
        })
        .collect();
    format!(
        "You are a reverse-engineering naming assistant. For each stripped \
function below, infer a concise snake_case name from its behavior signals \
(imports called, string-reference volume). Respond with a strict JSON array \
and nothing else: [{{\"address\":\"0x...\",\"name\":\"verb_noun\",\
\"evidence\":\"short reason\"}}]. Only include functions where the signals \
justify a name.\n\n{}",
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    )
}

/// Build the request body for a provider — pure, tested.
pub fn build_request_body(provider: LlmProvider, model: &str, prompt: &str) -> Value {
    match provider {
        LlmProvider::OpenAi => json!({
            "model": model,
            "temperature": 0,
            "messages": [
                {"role": "user", "content": prompt}
            ]
        }),
        LlmProvider::Anthropic => json!({
            "model": model,
            "max_tokens": 4096,
            "messages": [
                {"role": "user", "content": prompt}
            ]
        }),
    }
}

/// Extract the assistant's text from a provider response — pure, tested.
pub fn extract_text(provider: LlmProvider, response: &Value) -> Option<String> {
    match provider {
        LlmProvider::OpenAi => response
            .get("choices")?
            .get(0)?
            .get("message")?
            .get("content")?
            .as_str()
            .map(ToString::to_string),
        LlmProvider::Anthropic => response
            .get("content")?
            .get(0)?
            .get("text")?
            .as_str()
            .map(ToString::to_string),
    }
}

/// Sanitize a proposed name: identifier-safe, length-capped.
fn sanitize_name(raw: &str) -> Option<String> {
    let name: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    let name = name.trim_matches('_').to_string();
    (!name.is_empty()).then_some(name)
}

/// Parse the completion into proposals — pure, tested. Entries whose
/// address does not match a context are dropped (the model cannot
/// invent functions); names are sanitized.
pub fn parse_completion(completion: &str, contexts: &[FunctionContext]) -> Vec<RefinementProposal> {
    // Tolerate prose around the JSON array.
    let start = completion.find('[');
    let end = completion.rfind(']');
    let Some((start, end)) = start.zip(end).filter(|(s, e)| s < e) else {
        return Vec::new();
    };
    let Ok(Value::Array(entries)) = serde_json::from_str(&completion[start..=end]) else {
        return Vec::new();
    };
    let by_address: std::collections::BTreeMap<u64, &FunctionContext> = contexts
        .iter()
        .map(|context| (context.address, context))
        .collect();
    let mut proposals = Vec::new();
    for entry in entries {
        let Some(address_raw) = entry.get("address").and_then(Value::as_str) else {
            continue;
        };
        let Ok(address) = u64::from_str_radix(address_raw.trim_start_matches("0x"), 16) else {
            continue;
        };
        let Some(context) = by_address.get(&address) else {
            continue;
        };
        let Some(name) = entry
            .get("name")
            .and_then(Value::as_str)
            .and_then(sanitize_name)
        else {
            continue;
        };
        let evidence = entry
            .get("evidence")
            .and_then(Value::as_str)
            .unwrap_or("llm")
            .chars()
            .take(120)
            .collect::<String>();
        proposals.push(RefinementProposal {
            address,
            current_name: context.current_name.clone(),
            proposed_name: name,
            evidence,
        });
    }
    proposals
}

/// The HTTP-backed refiner.
pub struct LlmRefiner {
    pub config: LlmConfig,
    http: reqwest::blocking::Client,
}

impl LlmRefiner {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            http: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("llm http client"),
        }
    }

    fn endpoint(&self) -> String {
        match self.config.provider {
            LlmProvider::OpenAi => format!("{}/chat/completions", self.config.base_url),
            LlmProvider::Anthropic => format!("{}/messages", self.config.base_url),
        }
    }

    fn send(&self, prompt: &str) -> Result<String, String> {
        let body = build_request_body(self.config.provider, &self.config.model, prompt);
        let mut request = self
            .http
            .post(self.endpoint())
            .json(&body)
            .header("Content-Type", "application/json");
        request = match self.config.provider {
            LlmProvider::OpenAi => {
                request.header("Authorization", format!("Bearer {}", self.config.api_key))
            }
            LlmProvider::Anthropic => request
                .header("x-api-key", &self.config.api_key)
                .header("anthropic-version", "2023-06-01"),
        };
        let response = request
            .send()
            .map_err(|e| format!("llm request failed: {e}"))?;
        let status = response.status();
        let text = response
            .text()
            .map_err(|e| format!("llm response read failed: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "llm http {status}: {}",
                &text[..text.len().min(300)]
            ));
        }
        let value: Value =
            serde_json::from_str(&text).map_err(|e| format!("llm response not JSON: {e}"))?;
        extract_text(self.config.provider, &value)
            .ok_or_else(|| "llm response missing content".to_string())
    }
}

impl Refiner for LlmRefiner {
    fn actor(&self) -> String {
        let provider = match self.config.provider {
            LlmProvider::OpenAi => "openai",
            LlmProvider::Anthropic => "anthropic",
        };
        format!("model:{provider}/{}", self.config.model)
    }

    fn propose(&self, contexts: &[FunctionContext]) -> Result<Vec<RefinementProposal>, String> {
        // Only functions that need names go over the wire.
        let candidates: Vec<FunctionContext> = contexts
            .iter()
            .filter(|context| {
                context
                    .current_name
                    .as_deref()
                    .is_none_or(|name| name.starts_with("sub_"))
            })
            .cloned()
            .collect();
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let completion = self.send(&build_prompt(&candidates))?;
        Ok(parse_completion(&completion, &candidates))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(address: u64, name: Option<&str>) -> FunctionContext {
        FunctionContext {
            address,
            current_name: name.map(ToString::to_string),
            imports: vec![("printf".to_string(), 3)],
            string_refs: 2,
        }
    }

    #[test]
    fn prompt_carries_contexts_without_string_content() {
        let prompt = build_prompt(&[context(0x1000, None)]);
        assert!(prompt.contains("0x1000"));
        assert!(prompt.contains("printf x3"));
        assert!(prompt.contains("strict JSON array"));
        assert!(
            !prompt.contains("hello from damsel"),
            "string content never crosses"
        );
    }

    #[test]
    fn request_bodies_match_providers() {
        let openai = build_request_body(LlmProvider::OpenAi, "m1", "p");
        assert_eq!(openai["model"], "m1");
        assert_eq!(openai["messages"][0]["content"], "p");
        let anthropic = build_request_body(LlmProvider::Anthropic, "m2", "p");
        assert_eq!(anthropic["model"], "m2");
        assert_eq!(anthropic["messages"][0]["content"], "p");
    }

    #[test]
    fn text_extraction_per_provider() {
        let openai: serde_json::Value =
            serde_json::from_str(r#"{"choices":[{"message":{"content":"hi"}}]}"#).unwrap();
        assert_eq!(
            extract_text(LlmProvider::OpenAi, &openai).as_deref(),
            Some("hi")
        );
        let anthropic: serde_json::Value =
            serde_json::from_str(r#"{"content":[{"text":"yo"}]}"#).unwrap();
        assert_eq!(
            extract_text(LlmProvider::Anthropic, &anthropic).as_deref(),
            Some("yo")
        );
    }

    #[test]
    fn completion_parsing_drops_invented_functions() {
        let contexts = vec![context(0x1000, None), context(0x2000, None)];
        let completion = r#"Here you go:
[{"address":"0x1000","name":"log_message!","evidence":"calls printf x3"},
 {"address":"0x9999","name":"invented","evidence":"ghost"},
 {"address":"0x2000","name":"","evidence":"empty name dropped"}]"#;
        let proposals = parse_completion(completion, &contexts);
        assert_eq!(proposals.len(), 1, "only the real function survives");
        assert_eq!(proposals[0].address, 0x1000);
        assert_eq!(proposals[0].proposed_name, "log_message");
    }

    #[test]
    fn providers_parse_with_defaults() {
        assert_eq!(LlmProvider::parse("openai"), Some(LlmProvider::OpenAi));
        assert_eq!(
            LlmProvider::parse("OpenAI-Compatible"),
            Some(LlmProvider::OpenAi)
        );
        assert_eq!(
            LlmProvider::parse("anthropic"),
            Some(LlmProvider::Anthropic)
        );
        assert_eq!(LlmProvider::parse("nope"), None);
        assert_eq!(LlmProvider::OpenAi.default_model(), "gpt-4o-mini");
        assert_eq!(LlmProvider::Anthropic.default_model(), "claude-sonnet-4-5");
    }
}
