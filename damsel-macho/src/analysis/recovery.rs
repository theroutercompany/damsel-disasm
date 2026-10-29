//! Function recovery worker v1 (Wave 4 spec §5; function-recovery plan
//! "Recovery Worker").
//!
//! From each seeded address, linear-decode the function window and record
//! the first hard terminator (`Return`/`Trap`, or an unconditional
//! transfer whose successor is outside the window) as the bound. Rank-6
//! discovery promotes observed direct-call targets that validate by
//! decoding. Overlaps become `function_problem` facts, never silent
//! merges.
//!
//! Fact vocabulary (`recovery_v1`):
//! - `function_bound` on `function:<start>` → `0xSTART-0xEND|stop=<reason>`
//!   (`return` | `trap` | `seed_boundary` | `section_end`), confidence
//!   `strong` for rank-1..5 seeds, `inferred` for rank-6 call targets.
//! - `seed` on new rank-6 functions → `call_target`.
//! - `function_problem` → `overlap|<other function id>|<range>`.

use crate::MachoError;
use damsel_core::{
    DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2,
    DisassemblyTarget,
};
use crate::disasm::disassemble_v2;
use damsel_project::{AnalysisFact, Confidence, FactKind, FactProvenance, ObjectId, TypedStore};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("project store error during recovery: {0}")]
    Store(#[from] damsel_project::StoreError),
    #[error("disassembly error during recovery: {0}")]
    Disassembly(#[from] MachoError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryOutcome {
    pub bounds: usize,
    pub rank6_seeded: usize,
    pub problems: usize,
}

const PASS: &str = "recovery_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionBound {
    pub start: u64,
    pub end: u64,
    pub stop_reason: &'static str,
    pub rank6: bool,
}

/// Recover bounds for seeded functions and discover rank-6 call targets.
///
/// `seed_addresses` are the rank-1..5 seeds (ascending order not
/// required). Returns the bounds also persisted as facts.
pub fn recover_functions(
    store: &mut impl TypedStore,
    image: &damsel_core::BinaryImage,
    source_id: i64,
    seed_addresses: &[u64],
    analyzer_version: &str,
) -> Result<RecoveryOutcome, RecoveryError> {
    let mut seeds: Vec<u64> = seed_addresses.to_vec();
    seeds.sort_unstable();
    seeds.dedup();

    let executable_ranges: Vec<(u64, u64)> = image
        .sections()
        .iter()
        .filter(|s| s.executable && s.size > 0)
        .map(|s| (s.address, s.address.saturating_add(s.size)))
        .collect();
    // Stub sections (Mach-O `__stubs`/`__stub_helpers`; ELF PLT in Wave 5)
    // are not functions — rank-6 candidates inside them are skipped.
    let stub_ranges: Vec<(u64, u64)> = image
        .sections()
        .iter()
        .filter(|s| {
            s.executable
                && s.size > 0
                && s.name.to_ascii_lowercase().contains("stub")
        })
        .map(|s| (s.address, s.address.saturating_add(s.size)))
        .collect();
    let in_stub = |address: u64| {
        stub_ranges
            .iter()
            .any(|(start, end)| address >= *start && address < *end)
    };
    let section_end = |address: u64| {
        executable_ranges
            .iter()
            .find(|(start, end)| address >= *start && address < *end)
            .map(|&(_, end)| end)
    };

    let provenance = FactProvenance::analyzer(PASS, analyzer_version);
    let mut outcome = RecoveryOutcome {
        bounds: 0,
        rank6_seeded: 0,
        problems: 0,
    };

    // Rank-6 discovery: observed call targets from prior xref facts.
    let mut call_targets: Vec<u64> = Vec::new();
    for fact in store.facts_for_source(source_id)? {
        if fact.kind.as_str() != "xref" {
            continue;
        }
        if let Some(target) = fact.payload.strip_prefix("call|to=function:") {
            if let Ok(address) = u64::from_str_radix(target.trim_start_matches("0x"), 16) {
                call_targets.push(address);
            }
        }
    }
    call_targets.sort_unstable();
    call_targets.dedup();

    let mut all_work: Vec<(u64, bool)> = seeds.iter().map(|&a| (a, false)).collect();
    for target in call_targets {
        if !seeds.contains(&target) && section_end(target).is_some() && !in_stub(target) {
            all_work.push((target, true));
        }
    }
    all_work.sort();

    let mut bounds: Vec<FunctionBound> = Vec::new();
    for &(address, is_rank6) in &all_work {
        let Some(hard_end) = section_end(address) else {
            continue;
        };
        let next_seed = all_work
            .iter()
            .map(|&(a, _)| a)
            .filter(|&a| a > address)
            .min()
            .unwrap_or(hard_end);
        let window_end = next_seed.min(hard_end);

        let request = DisassemblyRequestV2 {
            target: DisassemblyTarget::Address(address),
            range: Some(address..window_end),
            limit: DisassemblyLimit::Unlimited,
            options: DisassemblyOptions {
                include_annotations: false,
                include_value_flow: false,
            },
        };
        let Ok(result) = disassemble_v2(image, &request) else {
            continue;
        };
        if result.instructions.is_empty() {
            continue;
        }

        // First hard terminator ends the function (v1 policy).
        let mut bound_end = None;
        let mut stop_reason = "seed_boundary";
        for instruction in &result.instructions {
            let lower = instruction.mnemonic.to_ascii_lowercase();
            let is_return = matches!(lower.as_str(), "ret" | "retab" | "retaa" | "braa" | "brab")
                || lower.starts_with("ret");
            let is_trap = matches!(lower.as_str(), "brk" | "hlt" | "udf");
            if is_return || is_trap {
                bound_end = Some(instruction.address + instruction.size as u64);
                stop_reason = if is_return { "return" } else { "trap" };
                break;
            }
        }
        let end = bound_end.unwrap_or(window_end);
        if bound_end.is_none() {
            stop_reason = if window_end == hard_end {
                "section_end"
            } else {
                "seed_boundary"
            };
        }

        let bound = FunctionBound {
            start: address,
            end,
            stop_reason,
            rank6: is_rank6,
        };
        bounds.push(bound);
    }

    // Persist facts + overlap problems.
    for index in 1..bounds.len() {
        let window = bounds[index];
        let previous = bounds[index - 1];
        if previous.end > window.start {
            for function in [previous, window] {
                let object = ObjectId::new("function", format!("{:#x}", function.start));
                store.put_fact(AnalysisFact {
                    source_id,
                    object_id: Some(object),
                    kind: FactKind::new("function_problem"),
                    payload: format!(
                        "overlap|function:{:#x}|{:#x}-{:#x}",
                        window.start, previous.start, previous.end
                    ),
                    confidence: Confidence::Ambiguous,
                    provenance: provenance.clone(),
                })?;
                outcome.problems += 1;
            }
        }
    }
    for bound in &bounds {
        let object = ObjectId::new("function", format!("{:#x}", bound.start));
        store.put_fact(AnalysisFact {
            source_id,
            object_id: Some(object.clone()),
            kind: FactKind::new("function_bound"),
            payload: format!(
                "{:#x}-{:#x}|stop={}",
                bound.start, bound.end, bound.stop_reason
            ),
            confidence: if bound.rank6 {
                Confidence::Inferred
            } else {
                Confidence::Strong
            },
            provenance: provenance.clone(),
        })?;
        outcome.bounds += 1;
        if bound.rank6 {
            store.put_fact(AnalysisFact {
                source_id,
                object_id: Some(object),
                kind: FactKind::new("seed"),
                payload: "call_target".to_string(),
                confidence: Confidence::Strong,
                provenance: provenance.clone(),
            })?;
            outcome.rank6_seeded += 1;
        }
    }

    Ok(outcome)
}
