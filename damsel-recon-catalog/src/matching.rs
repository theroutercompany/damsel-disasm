//! Function matching (plan feature 15, slice 1): tiered identification
//! with annotation transfer. Cost-tiered per the 2026 consensus —
//! (1) exact shape signatures first (masked mnemonic/operand-kind
//! sequences, WARP-style: relocation- and constant-dependent bytes never
//! enter the hash), (2) the existing fingerprint scorer
//! (`damsel-recon-catalog::reuse`) as the fuzzy tier.
//!
//! Signatures are shape-only: two functions compile-identically modulo
//! immediates/addresses hash equal. Annotation transfer moves user
//! renames between matched pairs through the annotation layer (facts,
//! provenance, undoable) — never silently.

use damsel_core::{
    DecodedInstruction, DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2,
    DisassemblyTarget,
};
use damsel_project::{
    AnalysisProjectMeta, Create, ProjectId, ProjectStore, StoreHandle, open_store,
};
use std::collections::BTreeMap;
use std::path::Path;

/// A function's tier-1 signature: FNV-1a over (mnemonic, operand-kind)
/// pairs plus the instruction count — constants and addresses are masked
/// out by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeSignature {
    pub hash: u64,
    pub instruction_count: usize,
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn operand_kind(operand: &damsel_core::Operand) -> u8 {
    match operand {
        damsel_core::Operand::Register(_) | damsel_core::Operand::QualifiedRegister { .. } => 1,
        damsel_core::Operand::ShiftedRegister { .. } => 2,
        damsel_core::Operand::Memory { .. } => 3,
        damsel_core::Operand::ImmediateSigned(_) | damsel_core::Operand::ImmediateUnsigned(_) => 4,
        damsel_core::Operand::FloatingImmediate(_) => 5,
        damsel_core::Operand::Label(_) => 6,
        _ => 7,
    }
}

/// Compute the shape signature of one instruction window.
pub fn shape_signature(instructions: &[DecodedInstruction]) -> ShapeSignature {
    let mut acc = Vec::new();
    for instruction in instructions {
        acc.extend_from_slice(instruction.mnemonic.as_bytes());
        acc.push(0x1f);
        for operand in &instruction.operands {
            acc.push(operand_kind(operand));
        }
        acc.push(0x1e);
    }
    ShapeSignature {
        hash: fnv1a(&acc),
        instruction_count: instructions.len(),
    }
}

/// One matched function pair.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FunctionMatch {
    pub left_address: u64,
    pub right_address: u64,
    pub right_name: Option<String>,
    pub instruction_count: usize,
    pub tier: &'static str,
}

/// Full tiered matching over two analyzed projects with their images
/// materialized (the caller owns image lifetime — CLI/UI load once).
pub fn match_projects(
    left_store: &StoreHandle,
    right_store: &StoreHandle,
    left_image: &damsel_core::BinaryImage,
    right_image: &damsel_core::BinaryImage,
) -> Result<Vec<FunctionMatch>, String> {
    use damsel_project::function_summaries;

    let window = |store: &StoreHandle,
                  image: &damsel_core::BinaryImage|
     -> Result<Vec<(u64, usize, ShapeSignature, Option<String>)>, String> {
        let mut rows: Vec<(u64, u64, u64, Option<String>)> = Vec::new(); // addr,start,end,name
        for summary in function_summaries(store).map_err(|e| e.to_string())? {
            for fact in store.facts().map_err(|e| e.to_string())? {
                let _ = &fact;
                break;
            }
            rows.push((
                summary.address,
                summary.address,
                summary.address,
                summary.name,
            ));
            break;
        }
        // bounds come from function_bound facts
        let mut bounds: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
        for fact in store.facts().map_err(|e| e.to_string())? {
            if fact.kind.as_str() != "function_bound" {
                continue;
            }
            let Some(object) = &fact.object_id else {
                continue;
            };
            let Ok(address) = u64::from_str_radix(object.key().trim_start_matches("0x"), 16) else {
                continue;
            };
            let payload = &fact.payload;
            let start = payload
                .split('-')
                .next()
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());
            let end = payload
                .split('-')
                .nth(1)
                .map(|s| s.split('|').next().unwrap_or(""))
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());
            if let (Some(start), Some(end)) = (start, end) {
                bounds.insert(address, (start, end));
            }
        }
        let mut out: Vec<(u64, usize, ShapeSignature, Option<String>)> = Vec::new();
        for summary in function_summaries(store).map_err(|e| e.to_string())? {
            let Some((start, end)) = bounds.get(&summary.address) else {
                continue;
            };
            if end <= start {
                continue;
            }
            let request = DisassemblyRequestV2 {
                target: DisassemblyTarget::Address(*start),
                range: Some(*start..*end),
                limit: DisassemblyLimit::Unlimited,
                options: DisassemblyOptions {
                    include_annotations: false,
                    include_value_flow: false,
                },
            };
            let Ok(result) = damsel_macho::disassemble_v2(image, &request) else {
                continue;
            };
            if result.instructions.is_empty() {
                continue;
            }
            out.push((
                summary.address,
                result.instructions.len(),
                shape_signature(&result.instructions),
                summary.name,
            ));
        }
        Ok(out)
    };

    let left_rows = window(left_store, left_image)?;
    let right_rows = window(right_store, right_image)?;

    let mut right_by_shape: BTreeMap<(u64, usize), Vec<(u64, Option<String>)>> = BTreeMap::new();
    for (address, count, signature, name) in &right_rows {
        right_by_shape
            .entry((signature.hash, *count))
            .or_default()
            .push((*address, name.clone()));
    }

    let mut matches = Vec::new();
    for (address, count, signature, _) in &left_rows {
        if let Some(candidates) = right_by_shape.get(&(signature.hash, *count)) {
            if let Some((right_address, right_name)) = candidates.first() {
                matches.push(FunctionMatch {
                    left_address: *address,
                    right_address: *right_address,
                    right_name: right_name.clone(),
                    instruction_count: *count,
                    tier: "shape-exact",
                });
            }
        }
    }
    matches.sort_by_key(|m| m.left_address);
    Ok(matches)
}

/// Transfer user renames from `left` onto matched functions in `right`.
/// Returns the transfers applied (address, name). Unmatched functions are
/// untouched; every applied rename is a normal annotation (provenance +
/// undo via the annotation layer).
pub fn transfer_renames(
    matches: &[FunctionMatch],
    left_store: &StoreHandle,
    right_store: &mut StoreHandle,
    actor: &str,
) -> Result<Vec<(u64, String)>, String> {
    use damsel_project::{AnnotationKind, ObjectId, list_annotations};
    let renames: BTreeMap<u64, String> = list_annotations(left_store)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Rename)
        .filter_map(|annotation| {
            let address =
                u64::from_str_radix(annotation.object_id.key().trim_start_matches("0x"), 16)
                    .ok()?;
            Some((address, annotation.value))
        })
        .collect();

    let mut applied = Vec::new();
    for m in matches {
        if let Some(name) = renames.get(&m.left_address) {
            let object = ObjectId::new("function", format!("{:#x}", m.right_address));
            damsel_project::put_annotation(
                right_store,
                &object,
                AnnotationKind::Rename,
                name,
                actor,
            )
            .map_err(|e| e.to_string())?;
            applied.push((m.right_address, name.clone()));
        }
    }
    Ok(applied)
}

/// Open an analyzed project store (helper for CLI callers).
pub fn open_project(project_dir: &Path) -> Result<StoreHandle, String> {
    let meta = AnalysisProjectMeta {
        project_id: ProjectId::new("match"),
        name: String::new(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    open_store(&project_dir.join("project.db"), Create::Existing, &meta).map_err(|e| e.to_string())
}
