//! Function summaries: query facade over the `ingest_v1` fact vocabulary
//! (Wave 4 spec §5 seed fusion, read side).
//!
//! Groups every `function:<0xADDR>` object with its seed facts into a
//! deterministic summary. The recovery worker (bounds, blocks, conflicts)
//! extends this in later units; the read model stays fact-derived so the
//! canonical export remains the single source of truth.

use crate::facts::AnalysisFact;
use crate::store::{ProjectStore, StoreError};
use std::collections::BTreeMap;

/// Seed-source tags, mirroring the ingest payload vocabulary.
#[allow(dead_code)] // consumers land with the recovery worker + tests
pub const SEED_SYMBOL_TABLE: &str = "symbol_table";
pub const SEED_FUNCTION_STARTS: &str = "function_starts";
pub const SEED_OBJC_METHOD: &str = "objc_method";
pub const SEED_ENTRY_POINT: &str = "entry_point";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSummary {
    /// Function address in the owning source's image space.
    pub address: u64,
    /// Best name: from the lexicographically smallest `symbol_table` seed
    /// payload (deterministic when several symbols share an address).
    pub name: Option<String>,
    /// Seed sources contributing to this function, sorted, deduplicated.
    pub seeds: Vec<String>,
}

impl FunctionSummary {
    /// True when at least one exact-rank seed (symbol table, function
    /// starts, ObjC IMP, entry point) claims this address.
    pub fn is_exact_seed(&self) -> bool {
        !self.seeds.is_empty()
    }
}

fn parse_function_address(object_key: &str) -> Option<u64> {
    let hex = object_key.trim_start_matches("0x");
    u64::from_str_radix(hex, 16).ok()
}

fn seed_tag(payload: &str) -> &str {
    // Payloads are `<source>|...` — the first segment is the tag.
    payload.split('|').next().unwrap_or(payload)
}

fn symbol_name(payload: &str) -> Option<&str> {
    payload
        .split('|')
        .find_map(|part| part.strip_prefix("name="))
}

/// Collect function summaries from stored facts, sorted by address.
pub fn function_summaries(store: &impl ProjectStore) -> Result<Vec<FunctionSummary>, StoreError> {
    let facts = store.facts()?;
    let mut by_address: BTreeMap<u64, FunctionSummary> = BTreeMap::new();

    for fact in &facts {
        if fact.kind.as_str() != "seed" {
            continue;
        }
        let Some(object) = &fact.object_id else { continue };
        if object.kind() != "function" {
            continue;
        }
        let Some(address) = parse_function_address(object.key()) else {
            continue;
        };
        let entry = by_address.entry(address).or_insert_with(|| FunctionSummary {
            address,
            name: None,
            seeds: Vec::new(),
        });
        let tag = seed_tag(&fact.payload);
        if !entry.seeds.iter().any(|s| s == tag) {
            entry.seeds.push(tag.to_string());
        }
        if tag == SEED_SYMBOL_TABLE {
            if let Some(name) = symbol_name(&fact.payload) {
                let better = entry
                    .name
                    .as_ref()
                    .is_none_or(|current| name < current.as_str());
                if better {
                    entry.name = Some(name.to_string());
                }
            }
        }
    }

    let mut out: Vec<FunctionSummary> = by_address.into_values().collect();
    for summary in &mut out {
        summary.seeds.sort();
        summary.seeds.dedup();
    }
    Ok(out)
}

/// Keep only facts relevant to function summaries (used by tests and the
/// upcoming recovery worker).
#[allow(dead_code)]
pub fn seed_facts(facts: &[AnalysisFact]) -> Vec<&AnalysisFact> {
    facts
        .iter()
        .filter(|f| f.kind.as_str() == "seed")
        .collect()
}
