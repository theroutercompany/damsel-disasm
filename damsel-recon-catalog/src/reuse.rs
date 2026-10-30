//! Static-link symbol reuse detection (Wave 8.2): identify stripped
//! functions that come from known libraries by fingerprint matching.
//!
//! A function fingerprint combines: distinctive string references,
//! distinctive constants, call-target counts, and instruction count.
//! When a stripped binary statically links a known library, matching
//! fingerprints recover the library names — reported with evidence,
//! never silently.

use serde::{Deserialize, Serialize};

/// A function fingerprint for matching.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionFingerprint {
    /// Distinctive string constants referenced by the function.
    pub strings: Vec<String>,
    /// Distinctive numeric constants (immediates ≥ 0x100).
    pub constants: Vec<u64>,
    /// Number of call instructions.
    pub call_count: usize,
    /// Total instruction count.
    pub instruction_count: usize,
}

/// A known library function to match against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownFunction {
    pub library: String,
    pub name: String,
    pub fingerprint: FunctionFingerprint,
}

/// A match result with evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReuseMatch {
    pub library: String,
    pub name: String,
    /// Match score 0.0–1.0.
    pub score: f64,
    /// Evidence components that contributed to the score.
    pub evidence: Vec<String>,
}

/// Compute a match score between a stripped function's fingerprint
/// and a known function's fingerprint.
pub fn match_score(stripped: &FunctionFingerprint, known: &KnownFunction) -> ReuseMatch {
    let mut evidence = Vec::new();
    let mut score = 0.0f64;

    // String overlap — 0.5 weight (strongest signal; distinctive
    // strings are rare and near-deterministic).
    if known.fingerprint.strings.is_empty() && stripped.strings.is_empty() {
        score += 0.5; // both empty: no signal either way
    } else if known.fingerprint.strings.is_empty() {
        // Known has no strings but stripped does: weak negative —
        // distinctive strings point elsewhere.
        score += 0.1;
    } else if stripped.strings.is_empty() {
        score += 0.0;
    } else {
        let matched: Vec<&String> = known
            .fingerprint
            .strings
            .iter()
            .filter(|s| stripped.strings.contains(s))
            .collect();
        let overlap = matched.len() as f64 / known.fingerprint.strings.len() as f64;
        score += overlap * 0.5;
        if !matched.is_empty() {
            evidence.push(format!(
                "strings: {}/{} matched (e.g. {:?})",
                matched.len(),
                known.fingerprint.strings.len(),
                matched[0]
            ));
        }
    }

    // Constant overlap — 0.3 weight.
    if known.fingerprint.constants.is_empty() && stripped.constants.is_empty() {
        score += 0.3; // both empty: neutral
    } else if known.fingerprint.constants.is_empty() {
        score += 0.1; // weak negative
    } else if stripped.constants.is_empty() {
        score += 0.0;
    } else {
        let matched = known
            .fingerprint
            .constants
            .iter()
            .filter(|c| stripped.constants.contains(c))
            .count();
        let overlap = matched as f64 / known.fingerprint.constants.len() as f64;
        score += overlap * 0.3;
        if matched > 0 {
            evidence.push(format!(
                "constants: {}/{} matched",
                matched,
                known.fingerprint.constants.len()
            ));
        }
    }

    // Structural similarity — 0.2 weight (size + call count).
    let size_ratio = if known.fingerprint.instruction_count > 0 {
        let ratio =
            stripped.instruction_count as f64 / known.fingerprint.instruction_count as f64;
        (1.0 - (ratio - 1.0).abs()).max(0.0)
    } else {
        1.0
    };
    score += size_ratio * 0.15;

    let call_ratio = if known.fingerprint.call_count > 0 {
        let ratio = stripped.call_count as f64 / known.fingerprint.call_count as f64;
        (1.0 - (ratio - 1.0).abs()).max(0.0)
    } else {
        1.0
    };
    score += call_ratio * 0.05;

    if size_ratio > 0.8 {
        evidence.push(format!(
            "size: {}/{} instructions (ratio {:.2})",
            stripped.instruction_count, known.fingerprint.instruction_count, size_ratio
        ));
    }

    ReuseMatch {
        library: known.library.clone(),
        name: known.name.clone(),
        score: score.min(1.0),
        evidence,
    }
}

/// Match a stripped function against a corpus of known functions.
/// Returns matches above the threshold, sorted by score.
pub fn find_reuse(
    stripped: &FunctionFingerprint,
    corpus: &[KnownFunction],
    threshold: f64,
) -> Vec<ReuseMatch> {
    let mut matches: Vec<ReuseMatch> = corpus
        .iter()
        .map(|k| match_score(stripped, k))
        .filter(|m| m.score >= threshold)
        .collect();
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    matches
}

/// A minimal built-in corpus of glibc constants for identification.
/// These are distinctive constants that appear in specific glibc functions.
pub fn glibc_minimal_corpus() -> Vec<KnownFunction> {
    vec![
        KnownFunction {
            library: "glibc".into(),
            name: "memcpy".into(),
            fingerprint: FunctionFingerprint {
                strings: vec![],
                constants: vec![0x8080808080808080, 0x0101010101010101],
                call_count: 0,
                instruction_count: 50,
            },
        },
        KnownFunction {
            library: "glibc".into(),
            name: "strlen".into(),
            fingerprint: FunctionFingerprint {
                strings: vec![],
                constants: vec![0x7f7f7f7f7f7f7f7f],
                call_count: 0,
                instruction_count: 30,
            },
        },
        KnownFunction {
            library: "glibc".into(),
            name: "malloc".into(),
            fingerprint: FunctionFingerprint {
                strings: vec!["malloc(): memory corruption".into()],
                constants: vec![],
                call_count: 4,
                instruction_count: 106,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_string_match() {
        let stripped = FunctionFingerprint {
            strings: vec!["malloc(): memory corruption".into()],
            constants: vec![],
            call_count: 4,
            instruction_count: 106,
        };
        let corpus = glibc_minimal_corpus();
        let matches = find_reuse(&stripped, &corpus, 0.5);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].name, "malloc");
        assert!(matches[0].score >= 0.9);
    }

    #[test]
    fn exact_constant_match() {
        let stripped = FunctionFingerprint {
            strings: vec![],
            constants: vec![0x7f7f7f7f7f7f7f7f, 0x0101],
            call_count: 0,
            instruction_count: 30,
        };
        let corpus = glibc_minimal_corpus();
        let matches = find_reuse(&stripped, &corpus, 0.5);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].name, "strlen");
    }

    #[test]
    fn no_match_below_threshold() {
        let stripped = FunctionFingerprint {
            strings: vec!["unrelated".into()],
            constants: vec![0xdeadbeef],
            call_count: 10,
            instruction_count: 500,
        };
        let corpus = glibc_minimal_corpus();
        let matches = find_reuse(&stripped, &corpus, 0.5);
        assert!(matches.is_empty());
    }

    #[test]
    fn score_includes_evidence() {
        let stripped = FunctionFingerprint {
            strings: vec!["malloc(): memory corruption".into()],
            constants: vec![],
            call_count: 4,
            instruction_count: 106,
        };
        let known = &glibc_minimal_corpus()[2];
        let m = match_score(&stripped, known);
        assert!(!m.evidence.is_empty());
        assert!(m.evidence.iter().any(|e| e.contains("strings")));
    }
}
