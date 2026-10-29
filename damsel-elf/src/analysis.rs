//! ELF engine passes (Wave 5B.4): xrefs and recovery over the x86_64
//! decoder — the same fact vocabulary as the Mach-O passes, so Wave 4
//! queries work unchanged. Strings arrive with the ELF strings unit.

use crate::image::ElfImage;
use damsel_core::{ControlFlowClass, Reference, decode_x86_64_with_limit};
use damsel_project::{AnalysisFact, Confidence, FactKind, FactProvenance, ObjectId, TypedStore};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ElfAnalysisError {
    #[error("project store error during ELF analysis: {0}")]
    Store(#[from] damsel_project::StoreError),
    #[error("decode error during ELF analysis: {0}")]
    Decode(#[from] damsel_core::DecodeError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfXrefOutcome {
    pub function_windows: usize,
    pub xref_count: usize,
    pub bounds: usize,
}

const PASS: &str = "elf_analysis_v1";

/// Harvest xrefs over seeded function windows and recover bounds in one
/// pass (windows are small in v1; splitting mirrors the Mach-O passes
/// when the recovery worker grows).
pub fn harvest_elf_xrefs(
    store: &mut impl TypedStore,
    image: &ElfImage,
    source_id: i64,
    seed_addresses: &[u64],
    analyzer_version: &str,
) -> Result<ElfXrefOutcome, ElfAnalysisError> {
    let mut seeds = seed_addresses.to_vec();
    seeds.sort_unstable();
    seeds.dedup();

    let exec_ranges: Vec<(u64, u64)> = image
        .sections
        .iter()
        .filter(|s| s.executable && s.size > 0)
        .map(|s| (s.address, s.address.saturating_add(s.size)))
        .collect();

    let provenance = FactProvenance::analyzer(PASS, analyzer_version);
    let mut outcome = ElfXrefOutcome {
        function_windows: 0,
        xref_count: 0,
        bounds: 0,
    };

    for (index, &address) in seeds.iter().enumerate() {
        let Some(&(_, section_end)) = exec_ranges
            .iter()
            .find(|(start, end)| address >= *start && address < *end)
        else {
            continue;
        };
        let window_end = seeds
            .get(index + 1)
            .copied()
            .filter(|next| *next > address && *next <= section_end)
            .unwrap_or(section_end);

        let Some(_offset) = image.vaddr_to_offset(address) else {
            continue;
        };
        let Some(bytes) = image.read_at_vaddr(address, (window_end - address) as usize) else {
            continue;
        };
        let instructions = decode_x86_64_with_limit(
            bytes,
            address,
            damsel_core::DisassemblyLimit::Unlimited,
            damsel_core::DisassemblyOptions::default(),
        )?;
        if instructions.is_empty() {
            continue;
        }
        outcome.function_windows += 1;

        // First hard terminator ends the function (v1 bound policy).
        let mut bound_end = None;
        let mut stop_reason = "seed_boundary";
        for instruction in &instructions {
            let lower = instruction.mnemonic.to_ascii_lowercase();
            match damsel_core::semantics_x86_64::classify(
                &lower,
                damsel_core::semantics_x86_64::Transfer::Direct,
            ) {
                ControlFlowClass::Return => {
                    bound_end = Some(instruction.address + instruction.size as u64);
                    stop_reason = "return";
                    break;
                }
                ControlFlowClass::Trap => {
                    bound_end = Some(instruction.address + instruction.size as u64);
                    stop_reason = "trap";
                    break;
                }
                _ => {}
            }
        }
        let end = bound_end.unwrap_or(window_end);
        if bound_end.is_none() {
            stop_reason = if window_end == section_end {
                "section_end"
            } else {
                "seed_boundary"
            };
        }

        // References → xref facts on instruction objects.
        for instruction in &instructions {
            for reference in &instruction.references {
                let payload = match reference {
                    Reference::Call { target } => format!("call|to=function:{target:#x}"),
                    Reference::Branch { target } => format!("branch|to=function:{target:#x}"),
                    Reference::IndirectCall { via } => format!("indirect_call|to=register:{via}"),
                    Reference::IndirectBranch { via } => {
                        format!("indirect_branch|to=register:{via}")
                    }
                    Reference::Data { target } => format!("data|to=data:{target:#x}"),
                    _ => continue,
                };
                let object = ObjectId::new("instruction", format!("{:#x}", instruction.address));
                store.put_fact(AnalysisFact {
                    source_id,
                    object_id: Some(object),
                    kind: FactKind::new("xref"),
                    payload,
                    confidence: Confidence::Strong,
                    provenance: provenance.clone(),
                })?;
                outcome.xref_count += 1;
            }
        }

        // Function bound fact (same payload vocabulary as recovery_v1).
        let object = ObjectId::new("function", format!("{address:#x}"));
        store.put_fact(AnalysisFact {
            source_id,
            object_id: Some(object),
            kind: FactKind::new("function_bound"),
            payload: format!("{address:#x}-{end:#x}|stop={stop_reason}"),
            confidence: Confidence::Strong,
            provenance: provenance.clone(),
        })?;
        outcome.bounds += 1;
    }

    Ok(outcome)
}
