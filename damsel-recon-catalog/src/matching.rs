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
use std::collections::{BTreeMap, BTreeSet};
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

/// A masked instruction-token *sequence* (mnemonic + operand-kind
/// stream) — the fuzzy-matching substrate (plan feature 15, tier 2).
/// Immediates and addresses never enter the tokens; similarity is
/// position-aware (LCS), so reorders and inserts read as what they
/// are instead of inflating shared boilerplate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzySignature {
    pub tokens: Vec<u64>,
    pub instruction_count: usize,
}

/// Compute the token sequence of an instruction window: one hashed
/// token per instruction (`mnemonic + operand kinds`).
pub fn fuzzy_signature(instructions: &[DecodedInstruction]) -> FuzzySignature {
    let tokens = instructions
        .iter()
        .map(|instruction| {
            let mut acc = instruction.mnemonic.as_bytes().to_vec();
            acc.push(0x1f);
            for operand in &instruction.operands {
                acc.push(operand_kind(operand));
            }
            fnv1a(&acc)
        })
        .collect();
    FuzzySignature {
        tokens,
        instruction_count: instructions.len(),
    }
}

/// Position-aware similarity: the Dice coefficient 2·LCS(a,b)/(len_a +
/// len_b). 1.0 = the same token sequence; balanced inserts and deletes
/// reduce proportionally on *both* sides, so a small function that
/// grew (its every instruction surviving) still scores high while a
/// small function merely *contained in* a big one does not.
pub fn sequence_similarity(left: &FuzzySignature, right: &FuzzySignature) -> f64 {
    let (a, b) = (&left.tokens, &right.tokens);
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut previous = vec![0usize; b.len() + 1];
    let mut current = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            current[j] = if a[i - 1] == b[j - 1] {
                previous[j - 1] + 1
            } else {
                previous[j].max(current[j - 1])
            };
        }
        std::mem::swap(&mut previous, &mut current);
        current.iter_mut().for_each(|v| *v = 0);
    }
    let lcs = previous[b.len()];
    (2.0 * lcs as f64) / (a.len() + b.len()) as f64
}

/// One fuzzy-matched pair with its score.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FuzzyMatch {
    pub left_address: u64,
    pub right_address: u64,
    pub right_name: Option<String>,
    pub score: f64,
    pub instruction_count: usize,
}

/// Best-mutual-pairs fuzzy matching over the unmatched leftovers of
/// the exact tier: every cross pair scores, pairs accept greedily by
/// descending score (each function used at most once), and only pairs
/// at or above `threshold` survive. Never guesses below the threshold.
pub fn match_fuzzy(
    left: &[(u64, usize, FuzzySignature, Option<String>)],
    right: &[(u64, usize, FuzzySignature, Option<String>)],
    threshold: f64,
) -> Vec<FuzzyMatch> {
    let mut scored: Vec<(f64, usize, usize)> = Vec::new();
    for (i, (_, _, left_sig, _)) in left.iter().enumerate() {
        for (j, (_, _, right_sig, _)) in right.iter().enumerate() {
            let score = sequence_similarity(left_sig, right_sig);
            if score >= threshold {
                scored.push((score, i, j));
            }
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut left_used = vec![false; left.len()];
    let mut right_used = vec![false; right.len()];
    let mut matches = Vec::new();
    for (score, i, j) in scored {
        if left_used[i] || right_used[j] {
            continue;
        }
        left_used[i] = true;
        right_used[j] = true;
        matches.push(FuzzyMatch {
            left_address: left[i].0,
            right_address: right[j].0,
            right_name: right[j].3.clone(),
            score: (score * 10_000.0).round() / 10_000.0,
            instruction_count: left[i].1,
        });
    }
    matches.sort_by(|a, b| a.left_address.cmp(&b.left_address));
    matches
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

/// Shape map over a project's bounded functions (address → (hash, count))
/// for the diff shape tier.
pub fn shape_map(
    store: &StoreHandle,
    image: &damsel_core::BinaryImage,
) -> Result<std::collections::BTreeMap<u64, (u64, usize)>, String> {
    use damsel_project::function_summaries;
    let mut bounds: std::collections::BTreeMap<u64, (u64, u64)> = Default::default();
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
        let Some(start) = fact
            .payload
            .split('-')
            .next()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        else {
            continue;
        };
        let Some(end) = fact
            .payload
            .split('-')
            .nth(1)
            .map(|rest| rest.split('|').next().unwrap_or(""))
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        else {
            continue;
        };
        if end > start {
            bounds.insert(address, (start, end));
        }
    }

    let mut map = std::collections::BTreeMap::new();
    for summary in function_summaries(store).map_err(|e| e.to_string())? {
        let Some((start, end)) = bounds.get(&summary.address) else {
            continue;
        };
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
        let signature = shape_signature(&result.instructions);
        map.insert(
            summary.address,
            (signature.hash, signature.instruction_count),
        );
    }
    Ok(map)
}

/// Full two-tier match (plan feature 15, tier 2): shape-exact first,
/// then fuzzy (LCS token-similarity) over the leftovers at `threshold`.
/// Returns both tiers; fuzzy pairs are clearly labeled and scored.
pub fn match_projects_tiered(
    left_store: &StoreHandle,
    right_store: &StoreHandle,
    left_image: &damsel_core::BinaryImage,
    right_image: &damsel_core::BinaryImage,
    fuzzy_threshold: f64,
) -> Result<(Vec<FunctionMatch>, Vec<FuzzyMatch>), String> {
    use damsel_project::function_summaries;

    let windows = |store: &StoreHandle,
                   image: &damsel_core::BinaryImage|
     -> Result<
        Vec<(u64, usize, ShapeSignature, FuzzySignature, Option<String>)>,
        String,
    > {
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
            let Some(start) = fact
                .payload
                .split('-')
                .next()
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            else {
                continue;
            };
            let Some(end) = fact
                .payload
                .split('-')
                .nth(1)
                .map(|r| r.split('|').next().unwrap_or(""))
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            else {
                continue;
            };
            if end > start {
                bounds.insert(address, (start, end));
            }
        }
        let mut rows: Vec<(u64, usize, ShapeSignature, FuzzySignature, Option<String>)> =
            Vec::new();
        for summary in function_summaries(store).map_err(|e| e.to_string())? {
            let Some((start, end)) = bounds.get(&summary.address) else {
                continue;
            };
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
            rows.push((
                summary.address,
                result.instructions.len(),
                shape_signature(&result.instructions),
                fuzzy_signature(&result.instructions),
                summary.name,
            ));
        }
        Ok(rows)
    };

    let left_rows = windows(left_store, left_image)?;
    let right_rows = windows(right_store, right_image)?;

    // Tier 1: shape-exact (mirrors match_projects' pairing).
    let mut right_by_shape: BTreeMap<(u64, usize), Vec<usize>> = BTreeMap::new();
    for (index, (_address, count, shape, _, _)) in right_rows.iter().enumerate() {
        right_by_shape
            .entry((shape.hash, *count))
            .or_default()
            .push(index);
    }
    let mut exact = Vec::new();
    let mut left_matched: BTreeSet<usize> = BTreeSet::new();
    let mut right_matched: BTreeSet<usize> = BTreeSet::new();
    for (left_index, (address, count, shape, _, _)) in left_rows.iter().enumerate() {
        if let Some(indices) = right_by_shape.get(&(shape.hash, *count)) {
            if let Some(&right_index) = indices.first() {
                if right_matched.insert(right_index) {
                    left_matched.insert(left_index);
                    exact.push(FunctionMatch {
                        left_address: *address,
                        right_address: right_rows[right_index].0,
                        right_name: right_rows[right_index].4.clone(),
                        instruction_count: *count,
                        tier: "shape-exact",
                    });
                }
            }
        }
    }

    // Tier 2: fuzzy over the leftovers.
    let fuzzy_left: Vec<_> = left_rows
        .iter()
        .enumerate()
        .filter(|(index, _)| !left_matched.contains(index))
        .map(|(_, row)| (row.0, row.1, row.3.clone(), row.4.clone()))
        .collect();
    let fuzzy_right: Vec<_> = right_rows
        .iter()
        .enumerate()
        .filter(|(index, _)| !right_matched.contains(index))
        .map(|(_, row)| (row.0, row.1, row.3.clone(), row.4.clone()))
        .collect();
    let fuzzy = match_fuzzy(&fuzzy_left, &fuzzy_right, fuzzy_threshold);

    exact.sort_by_key(|m| m.left_address);
    Ok((exact, fuzzy))
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

#[cfg(test)]
mod fuzzy_tests {
    use super::*;
    use damsel_core::{
        DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget,
    };

    fn window(binary: &str, address: u64) -> Vec<DecodedInstruction> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/bin")
            .join(binary);
        let image = damsel_macho::load(&path).unwrap();
        let request = DisassemblyRequestV2 {
            target: DisassemblyTarget::Address(address),
            range: Some(address..address + 0x60),
            limit: DisassemblyLimit::Unlimited,
            options: DisassemblyOptions {
                include_annotations: false,
                include_value_flow: false,
            },
        };
        damsel_macho::disassemble_v2(&image, &request)
            .unwrap()
            .instructions
    }

    #[test]
    fn identical_windows_score_one() {
        let instructions = window("arm64-symbolized", 0x1000004a0);
        let sig = fuzzy_signature(&instructions);
        assert_eq!(sequence_similarity(&sig, &sig), 1.0);
    }

    #[test]
    fn restructured_helper_scores_high_but_not_exact() {
        // v1 helper (add) vs v3 helper (mul + add): shape-exact fails,
        // the gram profile stays close.
        let left = fuzzy_signature(&window("arm64-symbolized", 0x1000004a0));
        let right = fuzzy_signature(&window("arm64-symbolized-v3", 0x1000004a0));
        let score = sequence_similarity(&left, &right);
        assert!(
            (0.6..1.0).contains(&score),
            "restructured pair should score high, got {score}"
        );
        // Dice on the recovered bounds (6 -> 10 instructions): every
        // original instruction survives, four were added.
        let _ = &score;
        // And they are not shape-identical.
        assert_ne!(
            shape_signature(&window("arm64-symbolized", 0x1000004a0)),
            shape_signature(&window("arm64-symbolized-v3", 0x1000004a0))
        );
    }

    #[test]
    fn unrelated_pairs_separate_from_related_ones() {
        // Position-aware LCS separates: a restructured pair keeps most
        // of its sequence; an unrelated pair shares only scaffolding,
        // and length normalization exposes the difference.
        let helper = fuzzy_signature(&window("arm64-symbolized", 0x1000004a0));
        let v3_helper = fuzzy_signature(&window("arm64-symbolized-v3", 0x1000004a0));
        let rich_main = fuzzy_signature(&window("import-rich", 0x100000460));
        let related = sequence_similarity(&helper, &v3_helper);
        let cross_program = sequence_similarity(&rich_main, &helper);

        // Dice: the restructured twin (every original instruction
        // survives, four added) lands at 0.75; unrelated programs stay
        // far below the 0.7 working threshold.
        assert!(related >= 0.7, "related pair above threshold: {related}");
        assert!(
            cross_program < 0.5,
            "cross-program noise below threshold: {cross_program}"
        );
    }

    #[test]
    fn fuzzy_matching_is_mutual_best_and_thresholded() {
        let sig = |tokens: Vec<u64>| FuzzySignature {
            tokens,
            instruction_count: 4,
        };
        let left = vec![
            (0x1000u64, 4usize, sig(vec![1, 2, 3]), None),
            (0x2000, 4, sig(vec![9, 9, 9]), None),
        ];
        let right = vec![
            (0x5000u64, 4usize, sig(vec![1, 2, 3]), Some("twin".into())),
            (0x6000, 4, sig(vec![7, 7, 7]), None),
        ];
        let matches = match_fuzzy(&left, &right, 0.99);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].left_address, 0x1000);
        assert_eq!(matches[0].right_address, 0x5000);
        assert_eq!(matches[0].score, 1.0);
        assert_eq!(matches[0].right_name.as_deref(), Some("twin"));

        // Nothing survives an unreachable threshold.
        assert!(match_fuzzy(&left, &right, 1.1).is_empty());
    }
}
