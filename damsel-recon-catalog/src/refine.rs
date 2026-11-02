//! Model refinement pass (plan feature 17, slice 1): propose function
//! renames from decompiler context and write them through the annotation
//! layer — provenance-tagged (`model:heuristic-v1`), reviewable, undoable.
//!
//! v1 is a deterministic heuristic model: a stripped function's referenced
//! strings and called imports are its context, and the strongest string
//! becomes a name hint (`fn_<addr>_<hint>`). The *architecture* is the
//! feature: every proposal is a normal annotation (facts + audit events,
//! actor `model:*`, undo via the annotation layer) — never a silent
//! overwrite, and dry-run by default. External LLMs plug in through the
//! same surface (`damsel_refine` MCP tool or the library API) later.

use damsel_project::{
    AnalysisSourceKind, AnnotationKind, ProjectStore, StoreHandle, list_annotations, put_annotation,
};
use std::collections::BTreeMap;

/// The v1 model identity — appears in provenance and undo scoping.
pub const MODEL_ACTOR: &str = "model:heuristic-v1";

/// One proposed rename.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RefinementProposal {
    pub address: u64,
    pub current_name: Option<String>,
    pub proposed_name: String,
    /// Why the model proposed this (evidence line).
    pub evidence: String,
}

fn sanitize_hint(raw: &str) -> String {
    let mut hint: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(24)
        .collect();
    while hint.starts_with('_') {
        hint.remove(0);
    }
    hint
}

/// Provider-agnostic context for one function (plan feature 17): what
/// any refiner — heuristic or external LLM — sees. String *content*
/// stays out (privacy by design); imports called (with counts) and
/// string-reference counts are the signals.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FunctionContext {
    pub address: u64,
    pub current_name: Option<String>,
    /// Imports the function calls: (name, count), most-called first.
    pub imports: Vec<(String, usize)>,
    pub string_refs: usize,
}

/// A rename proposer. Implementations: the deterministic heuristic
/// (v1, default) and external LLM adapters (damsel-cli).
pub trait Refiner {
    /// Annotation actor for writes: `model:<identity>`.
    fn actor(&self) -> String;
    /// Propose renames from contexts. Deterministic implementations
    /// return address-ordered proposals.
    fn propose(&self, contexts: &[FunctionContext]) -> Result<Vec<RefinementProposal>, String>;
}

/// The deterministic v1 refiner (default; no network).
#[derive(Debug, Clone, Copy, Default)]
pub struct HeuristicRefiner;

impl Refiner for HeuristicRefiner {
    fn actor(&self) -> String {
        MODEL_ACTOR.to_string()
    }

    fn propose(&self, contexts: &[FunctionContext]) -> Result<Vec<RefinementProposal>, String> {
        let mut proposals = Vec::new();
        for context in contexts {
            let needs_name = context
                .current_name
                .as_deref()
                .is_none_or(|name| name.starts_with("sub_"));
            if !needs_name {
                continue;
            }
            let hint = context
                .imports
                .first()
                .map(|(name, count)| {
                    (
                        format!("calls_{}", sanitize_hint(name)),
                        format!("imports: {name} x{count}"),
                    )
                })
                .filter(|_| context.string_refs == 0)
                .or_else(|| {
                    (context.string_refs > 0).then(|| {
                        (
                            format!("refs_{}_str", context.string_refs),
                            format!("string references: {}", context.string_refs),
                        )
                    })
                });
            if let Some((name, evidence)) = hint {
                proposals.push(RefinementProposal {
                    address: context.address,
                    current_name: context.current_name.clone(),
                    proposed_name: format!("fn_{:x}_{}", context.address, name),
                    evidence,
                });
            }
        }
        Ok(proposals)
    }
}

/// Collect per-function contexts from a project's facts. Deterministic:
/// imports sorted by (count desc, name) per function.
pub fn collect_contexts(store: &StoreHandle) -> Result<Vec<FunctionContext>, String> {
    let facts = store.facts().map_err(|e| e.to_string())?;

    // String addresses → values come from string facts; the values live
    // in the image, so hints come from the xref targets joined against
    // string addresses with their lengths — the string *content* is not
    // in the store (privacy by design). Context therefore uses:
    // (a) imports the function calls, (b) how many strings it references.
    // Import calls are the strong signal available store-side.
    let mut import_names: BTreeMap<String, String> = BTreeMap::new();
    for fact in &facts {
        if fact.kind.as_str() == "import" {
            if let Some(object) = &fact.object_id {
                import_names.insert(object.as_str().to_string(), object.key().to_string());
            }
        }
    }

    // Bounds for instruction containment.
    let mut bounds: Vec<(u64, u64)> = Vec::new();
    for fact in &facts {
        if fact.kind.as_str() != "function_bound" {
            continue;
        }
        let (Some(start), Some(end)) = (
            fact.payload
                .split('-')
                .next()
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
            fact.payload
                .split('-')
                .nth(1)
                .map(|r| r.split('|').next().unwrap_or(""))
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
        ) else {
            continue;
        };
        if end > start {
            bounds.push((start, end));
        }
    }
    bounds.sort_unstable();
    let containing = |address: u64| -> Option<u64> {
        let index = bounds.partition_point(|(start, _)| *start <= address);
        if index == 0 {
            return None;
        }
        let (start, end) = bounds[index - 1];
        (address < end).then_some(start)
    };

    // Per-function context: called imports (named) + string-reference count.
    let mut called_imports: BTreeMap<u64, BTreeMap<String, usize>> = BTreeMap::new();
    let mut string_refs: BTreeMap<u64, usize> = BTreeMap::new();
    for fact in &facts {
        if fact.kind.as_str() != "xref" {
            continue;
        }
        let Some(object) = &fact.object_id else {
            continue;
        };
        let Some(instruction) = u64::from_str_radix(object.key().trim_start_matches("0x"), 16).ok()
        else {
            continue;
        };
        let Some(function) = containing(instruction) else {
            continue;
        };
        if fact.payload.starts_with("call|to=") || fact.payload.starts_with("import|to=") {
            let target = fact.payload.split("to=").nth(1).unwrap_or("");
            if let Some(name) = import_names
                .keys()
                .find(|key| key.ends_with(&format!(":{target}")))
            {
                *called_imports
                    .entry(function)
                    .or_default()
                    .entry(import_names[name].clone())
                    .or_insert(0) += 1;
            }
        } else if fact.payload.starts_with("data|to=") {
            *string_refs.entry(function).or_insert(0) += 1;
        }
    }

    // Contexts in address order; imports sorted by (count desc, name).
    let summaries = damsel_project::function_summaries(store).map_err(|e| e.to_string())?;
    let mut contexts = Vec::new();
    for summary in summaries {
        let mut imports: Vec<(String, usize)> = called_imports
            .get(&summary.address)
            .map(|map| map.iter().map(|(k, v)| (k.clone(), *v)).collect())
            .unwrap_or_default();
        imports.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        contexts.push(FunctionContext {
            address: summary.address,
            current_name: summary.name,
            imports,
            string_refs: string_refs.get(&summary.address).copied().unwrap_or(0),
        });
    }
    Ok(contexts)
}

/// Propose with the default heuristic refiner (compatibility shim).
pub fn propose(store: &StoreHandle) -> Result<Vec<RefinementProposal>, String> {
    let contexts = collect_contexts(store)?;
    HeuristicRefiner.propose(&contexts)
}

/// Apply proposals through the annotation layer with the given actor
/// (`model:*`). Returns the applied (address, name) pairs.
pub fn apply_as(
    store: &mut StoreHandle,
    proposals: &[RefinementProposal],
    actor: &str,
) -> Result<Vec<(u64, String)>, String> {
    let mut applied = Vec::new();
    for proposal in proposals {
        let object = damsel_project::ObjectId::new("function", format!("{:#x}", proposal.address));
        put_annotation(
            store,
            &object,
            AnnotationKind::Rename,
            &proposal.proposed_name,
            actor,
        )
        .map_err(|e| e.to_string())?;
        applied.push((proposal.address, proposal.proposed_name.clone()));
    }
    Ok(applied)
}

/// Apply with the default heuristic actor (compatibility shim).
pub fn apply(
    store: &mut StoreHandle,
    proposals: &[RefinementProposal],
) -> Result<Vec<(u64, String)>, String> {
    apply_as(store, proposals, MODEL_ACTOR)
}

/// Refinements currently attributed to the model actor (review surface).
pub fn model_annotations(
    store: &StoreHandle,
) -> Result<Vec<damsel_project::AnnotationRecord>, String> {
    Ok(list_annotations(store)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|annotation| annotation.actor.starts_with("model:"))
        .collect())
}

/// Reject any source kinds this pass cannot serve yet.
pub fn supported(store: &StoreHandle) -> Result<bool, String> {
    let sources = store.sources().map_err(|e| e.to_string())?;
    Ok(sources
        .iter()
        .all(|source| source.kind == AnalysisSourceKind::MachOImage))
}
