//! D0 function summaries (Wave 6 spec §1 stage "D0 summaries"):
//! fact-derived, honest, format-neutral. Every entry carries its fact
//! provenance implicitly (the store's pass names); unresolved evidence
//! is surfaced, never dropped.

use damsel_project::{ProjectStore, StoreError, TypedStore, function_summaries};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SummaryError {
    #[error("store error during summary: {0}")]
    Store(#[from] StoreError),
    #[error("function not found: {0}")]
    FunctionNotFound(String),
    #[error("ambiguous function name: {0}")]
    AmbiguousName(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct CallSite {
    /// Instruction address inside the function.
    pub from: String,
    /// `call`, `branch`, `indirect_call`, `indirect_branch`, `data`,
    /// `import`, … (xref fact kind segment).
    pub kind: String,
    /// Target object id (`function:0x…`, `import:name`, …).
    pub target: String,
    /// True when the engine could not resolve a concrete target.
    pub unresolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FunctionSummaryReport {
    pub function: String,
    pub name: Option<String>,
    /// Recovered bound payload (`0xA-0xB|stop=…`) when present.
    pub bound: Option<String>,
    pub seeds: Vec<String>,
    pub calls: Vec<CallSite>,
    /// Import names referenced by instructions in this function.
    pub imports: Vec<String>,
    /// String object ids referenced by instructions in this function.
    pub strings: Vec<String>,
    /// Problem fact payloads attached to this function.
    pub problems: Vec<String>,
}

/// Build the D0 summary for the function at `address` in `source_id`.
pub fn summarize_function<S: ProjectStore + TypedStore>(
    store: &S,
    source_id: i64,
    address: u64,
) -> Result<FunctionSummaryReport, SummaryError> {
    let facts = store.facts_for_source(source_id)?;
    let function_id = format!("function:{address:#x}");

    // Name + seeds + bound + problems from function-scoped facts.
    let summaries = function_summaries(store)?;
    let own = summaries
        .iter()
        .find(|s| s.address == address)
        .ok_or_else(|| SummaryError::FunctionNotFound(function_id.clone()))?;
    let bound = facts
        .iter()
        .find(|f| {
            f.object_id.as_ref().is_some_and(|o| o.as_str() == function_id)
                && f.kind.as_str() == "function_bound"
        })
        .map(|f| f.payload.clone());
    let problems: Vec<String> = facts
        .iter()
        .filter(|f| {
            f.object_id.as_ref().is_some_and(|o| o.as_str() == function_id)
                && f.kind.as_str() == "function_problem"
        })
        .map(|f| f.payload.clone())
        .collect();

    // Window: bound range when recovered, else the bound-less function
    // still summarizes its instruction-level evidence via xref facts
    // attributed by nearest-below (v1, same heuristic as the CLI).
    let window = bound
        .as_deref()
        .and_then(|payload| {
            let (range, _stop) = payload.split_once("|stop=")?;
            let (start, end) = range.split_once('-')?;
            let start = u64::from_str_radix(start.trim_start_matches("0x"), 16).ok()?;
            let end = u64::from_str_radix(end.trim_start_matches("0x"), 16).ok()?;
            Some(start..end)
        })
        .unwrap_or(address..address);

    // String objects (for data-target joins).
    let string_objects: std::collections::BTreeSet<String> = facts
        .iter()
        .filter(|f| f.kind.as_str() == "string")
        .filter_map(|f| f.object_id.as_ref().map(|o| o.as_str().to_string()))
        .collect();

    let mut calls = Vec::new();
    let mut imports = Vec::new();
    let mut strings = Vec::new();
    for fact in &facts {
        if fact.kind.as_str() != "xref" {
            continue;
        }
        let Some(object) = &fact.object_id else { continue };
        if object.kind() != "instruction" {
            continue;
        }
        let Ok(from_addr) = u64::from_str_radix(object.key().trim_start_matches("0x"), 16)
        else {
            continue;
        };
        if !window.contains(&from_addr) {
            continue;
        }
        let Some((kind, target)) = fact.payload.split_once("|to=") else {
            continue;
        };
        let unresolved = kind.starts_with("indirect") || target.starts_with("register:");
        calls.push(CallSite {
            from: format!("{from_addr:#x}"),
            kind: kind.to_string(),
            target: target.to_string(),
            unresolved,
        });
        if kind == "import" {
            if let Some(name) = target.strip_prefix("import:") {
                imports.push(name.to_string());
            }
        }
        if let Some(data_target) = target.strip_prefix("data:") {
            let string_id = format!("string:{data_target}");
            if string_objects.contains(&string_id) {
                strings.push(string_id);
            }
        }
    }
    calls.sort();
    calls.dedup_by(|a, b| a.from == b.from && a.target == b.target);
    imports.sort();
    imports.dedup();
    strings.sort();
    strings.dedup();

    Ok(FunctionSummaryReport {
        function: function_id,
        name: own.name.clone(),
        bound,
        seeds: own.seeds.clone(),
        calls,
        imports,
        strings,
        problems,
    })
}

/// Resolve a function by name (or `0x…` address) and summarize it.
pub fn summarize_named<S: ProjectStore + TypedStore>(
    store: &S,
    source_id: i64,
    selector: &str,
) -> Result<FunctionSummaryReport, SummaryError> {
    if let Some(hex) = selector.strip_prefix("0x") {
        let address =
            u64::from_str_radix(hex, 16).map_err(|_| SummaryError::FunctionNotFound(selector.into()))?;
        return summarize_function(store, source_id, address);
    }
    let matches: Vec<u64> = function_summaries(store)?
        .into_iter()
        .filter(|s| s.name.as_deref() == Some(selector))
        .map(|s| s.address)
        .collect();
    match matches.as_slice() {
        [address] => summarize_function(store, source_id, *address),
        [] => Err(SummaryError::FunctionNotFound(selector.to_string())),
        _ => Err(SummaryError::AmbiguousName(selector.to_string())),
    }
}
