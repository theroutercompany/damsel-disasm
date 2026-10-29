//! Recon triage (Wave 7; strategy §Recon): first-touch assessment of
//! an unknown binary — stripped-ness, entropy profile, packer
//! signatures, defensive indicators. Every finding carries an
//! evidence address or section; no vibes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconReport {
    pub stripped: bool,
    pub symbol_count: usize,
    pub section_entropies: Vec<SectionEntropy>,
    pub packer_signatures: Vec<PackerSignature>,
    pub anti_analysis_indicators: Vec<AntiAnalysisIndicator>,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionEntropy {
    pub section: String,
    pub entropy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackerSignature {
    pub name: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AntiAnalysisIndicator {
    pub name: String,
    pub evidence: String,
    /// Address, symbol, or section where the indicator was found.
    pub location: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Verdict {
    Clean,
    LikelyPacked,
    LikelyObfuscated,
    NeedsManualReview,
}

/// Shannon entropy of a byte window (0..8; > 7.0 suggests compression
/// or encryption).
pub fn shannon_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for byte in data {
        counts[*byte as usize] += 1;
    }
    let total = data.len() as f64;
    counts
        .iter()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = *c as f64 / total;
            -p * p.log2()
        })
        .sum()
}

/// Scan section bytes for known packer magic strings.
pub fn detect_packers(section_name: &str, data: &[u8]) -> Vec<PackerSignature> {
    let mut found = Vec::new();
    let text = String::from_utf8_lossy(data);
    if text.contains("UPX!") {
        found.push(PackerSignature {
            name: "UPX".into(),
            evidence: format!("UPX! magic in {section_name}"),
        });
    }
    found
}

/// Check for common anti-debug / anti-VM imports and strings.
pub fn detect_anti_analysis(
    imports: &[String],
    strings: &[String],
) -> Vec<AntiAnalysisIndicator> {
    let mut indicators = Vec::new();
    let ptrace = ["ptrace", "_ptrace"];
    let sysctl = ["sysctl", "_sysctl"];
    let anti_vm = ["vmware", "vbox", "virtualbox", "qemu", "xen"];

    for import in imports {
        let lower = import.to_ascii_lowercase();
        if ptrace.iter().any(|p| lower.contains(p)) {
            indicators.push(AntiAnalysisIndicator {
                name: "anti_debug_ptrace".into(),
                evidence: format!("import: {import}"),
                location: import.clone(),
            });
        }
        if sysctl.iter().any(|s| lower.contains(s)) {
            indicators.push(AntiAnalysisIndicator {
                name: "anti_debug_sysctl".into(),
                evidence: format!("import: {import}"),
                location: import.clone(),
            });
        }
    }

    for string in strings {
        let lower = string.to_ascii_lowercase();
        if anti_vm.iter().any(|v| lower.contains(v)) {
            indicators.push(AntiAnalysisIndicator {
                name: "anti_vm_string".into(),
                evidence: format!("string matches VM indicator: {string}"),
                location: string.clone(),
            });
        }
    }

    indicators
}

/// Build a complete recon report from image facts.
pub fn assess(
    symbol_count: usize,
    sections: &[(String, Vec<u8>)],
    imports: &[String],
    strings: &[String],
) -> ReconReport {
    let mut section_entropies = Vec::new();
    let mut packer_signatures = Vec::new();
    for (name, data) in sections {
        let entropy = shannon_entropy(data);
        section_entropies.push(SectionEntropy {
            section: name.clone(),
            entropy,
        });
        packer_signatures.extend(detect_packers(name, data));
    }

    let anti_analysis = detect_anti_analysis(imports, strings);

    let high_entropy: Vec<&SectionEntropy> = section_entropies
        .iter()
        .filter(|s| s.entropy > 7.0)
        .collect();
    let stripped = symbol_count < 10;

    let verdict = if !packer_signatures.is_empty() {
        Verdict::LikelyPacked
    } else if !anti_analysis.is_empty() || high_entropy.len() > 1 {
        Verdict::NeedsManualReview
    } else {
        Verdict::Clean
    };

    ReconReport {
        stripped,
        symbol_count,
        section_entropies,
        packer_signatures,
        anti_analysis_indicators: anti_analysis,
        verdict,
    }
}

/// CFG-flattening detection (OLLVM `-fla` output class): a dispatcher
/// block with unusually high in-degree + many switch-like back edges
/// to it from state-machine blocks. Uses the project's function_bound
/// and xref facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlatteningIndicator {
    pub function: String,
    pub dispatcher_address: u64,
    pub state_blocks: usize,
    pub evidence: String,
}

pub fn detect_cfg_flattening(
    function_bounds: &[(u64, u64)],
    branch_targets: &[u64],
) -> Vec<FlatteningIndicator> {
    let mut out = Vec::new();
    // Count in-degree for each target address
    let mut target_counts: std::collections::BTreeMap<u64, usize> = Default::default();
    for target in branch_targets {
        *target_counts.entry(*target).or_insert(0) += 1;
    }

    for (start, end) in function_bounds {
        // A dispatcher is an address inside the function with >= 6
        // incoming branches (typical of flattened switch loops).
        let candidates: Vec<(u64, usize)> = target_counts
            .iter()
            .filter(|(addr, count)| **addr >= *start && **addr < *end && **count >= 6)
            .map(|(addr, count)| (*addr, *count))
            .collect();
        if let Some((dispatcher, count)) = candidates.first() {
            out.push(FlatteningIndicator {
                function: format!("{start:#x}-{end:#x}"),
                dispatcher_address: *dispatcher,
                state_blocks: *count,
                evidence: format!("dispatcher at {dispatcher:#x} has {count} incoming branches"),
            });
        }
    }
    out
}
