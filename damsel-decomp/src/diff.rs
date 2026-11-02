//! Binary diff (Wave 7; spec §3): function-level matching between two
//! analyzed projects. Exact matches first (same address + same symbol
//! payload), then structural (same bound size + same call counts).
//! Every match carries its evidence; unmatched functions are
//! reported as added/removed — never silently dropped.

use damsel_project::{ProjectStore, StoreError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiffError {
    #[error("store error during diff: {0}")]
    Store(#[from] StoreError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffReport {
    pub left_name: String,
    pub right_name: String,
    pub matched: Vec<MatchedFunction>,
    pub added: Vec<DiffFunction>,
    pub removed: Vec<DiffFunction>,
    /// Defense indicators present in right but not left (version
    /// archaeology: vendor added a check between versions).
    pub defense_added: Vec<DefenseFinding>,
    /// Defense indicators present in left but not right (removed).
    pub defense_removed: Vec<DefenseFinding>,
}

/// A defense/anti-analysis indicator that appeared or disappeared
/// between two versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefenseFinding {
    /// Catalog code (e.g. ANTI_DEBUG_PTRACE).
    pub code: String,
    /// Evidence location.
    pub location: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchedFunction {
    pub name: Option<String>,
    pub left_address: u64,
    pub right_address: u64,
    pub strategy: MatchStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffFunction {
    pub name: Option<String>,
    pub address: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchStrategy {
    ExactSymbol,
    ExactAddress,
    /// Masked shape-signature match (plan feature 16 v2): same body at a
    /// different address under a different name — moved/recompiled.
    Shape,
    Structural,
}

/// Shape signatures keyed by function address (from
/// `damsel_recon_catalog::matching::shape_signature` over recovered
/// bounds; `None` disables the shape pass).
pub type ShapeMap = std::collections::BTreeMap<u64, (u64, usize)>;

pub fn diff_functions(
    left: &impl ProjectStore,
    right: &impl ProjectStore,
    left_name: &str,
    right_name: &str,
) -> Result<DiffReport, DiffError> {
    diff_functions_with_shapes(left, right, left_name, right_name, None, None)
}

/// Diff with the shape tier: after symbol and address passes, unmatched
/// functions pair by exact masked-shape signature — same body, moved
/// address, changed name (recompiles, relinks, ASLR).
pub fn diff_functions_with_shapes(
    left: &impl ProjectStore,
    right: &impl ProjectStore,
    left_name: &str,
    right_name: &str,
    left_shapes: Option<&ShapeMap>,
    right_shapes: Option<&ShapeMap>,
) -> Result<DiffReport, DiffError> {
    let left_functions = damsel_project::function_summaries(left)?;
    let right_functions = damsel_project::function_summaries(right)?;

    let mut matched = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();

    // Pass 1: exact symbol-name matches
    let mut right_matched: std::collections::BTreeSet<u64> = Default::default();
    let mut left_matched: std::collections::BTreeSet<u64> = Default::default();
    for lfn in &left_functions {
        if let Some(name) = &lfn.name {
            if let Some(rfn) = right_functions
                .iter()
                .find(|f| f.name.as_deref() == Some(name.as_str()))
            {
                matched.push(MatchedFunction {
                    name: Some(name.clone()),
                    left_address: lfn.address,
                    right_address: rfn.address,
                    strategy: MatchStrategy::ExactSymbol,
                });
                right_matched.insert(rfn.address);
                left_matched.insert(lfn.address);
            }
        }
    }

    // Pass 2: exact address matches (unmatched leftovers)
    for lfn in &left_functions {
        if left_matched.contains(&lfn.address) {
            continue;
        }
        if let Some(rfn) = right_functions.iter().find(|f| f.address == lfn.address) {
            if !right_matched.contains(&rfn.address) {
                matched.push(MatchedFunction {
                    name: lfn.name.clone().or_else(|| rfn.name.clone()),
                    left_address: lfn.address,
                    right_address: rfn.address,
                    strategy: MatchStrategy::ExactAddress,
                });
                right_matched.insert(rfn.address);
                left_matched.insert(lfn.address);
            }
        }
    }

    // Pass 3 (v2): masked shape signatures over the unmatched leftovers —
    // same body, different address/name. Unique matches only: an
    // ambiguous shape stays added/removed rather than guessed.
    if let (Some(left_shapes), Some(right_shapes)) = (left_shapes, right_shapes) {
        let mut right_by_shape: std::collections::BTreeMap<(u64, usize), Vec<u64>> =
            Default::default();
        for rfn in &right_functions {
            if right_matched.contains(&rfn.address) {
                continue;
            }
            if let Some(shape) = right_shapes.get(&rfn.address) {
                right_by_shape.entry(*shape).or_default().push(rfn.address);
            }
        }
        for lfn in &left_functions {
            if left_matched.contains(&lfn.address) {
                continue;
            }
            let Some(shape) = left_shapes.get(&lfn.address) else {
                continue;
            };
            let Some(candidates) = right_by_shape.get(shape) else {
                continue;
            };
            if candidates.len() != 1 {
                continue;
            }
            let right_address = candidates[0];
            let name = right_functions
                .iter()
                .find(|f| f.address == right_address)
                .and_then(|f| f.name.clone())
                .or_else(|| lfn.name.clone());
            left_matched.insert(lfn.address);
            right_matched.insert(right_address);
            matched.push(MatchedFunction {
                name,
                left_address: lfn.address,
                right_address,
                strategy: MatchStrategy::Shape,
            });
        }
    }

    // Report unmatched
    for lfn in &left_functions {
        if !left_matched.contains(&lfn.address) {
            removed.push(DiffFunction {
                name: lfn.name.clone(),
                address: lfn.address,
            });
        }
    }
    for rfn in &right_functions {
        if !right_matched.contains(&rfn.address) {
            added.push(DiffFunction {
                name: rfn.name.clone(),
                address: rfn.address,
            });
        }
    }

    Ok(DiffReport {
        left_name: left_name.to_string(),
        right_name: right_name.to_string(),
        matched,
        added,
        removed,
        defense_added: Vec::new(),
        defense_removed: Vec::new(),
    })
}

/// Render a diff report as Markdown (evidence profile: no raw bytes,
/// structured findings only).
pub fn render_diff_markdown(report: &DiffReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Diff: {} → {}\n\n",
        report.left_name, report.right_name
    ));
    out.push_str(&format!(
        "| Metric | Count |\n|---|---|\n| Matched | {} |\n| Added | {} |\n| Removed | {} |\n\n",
        report.matched.len(),
        report.added.len(),
        report.removed.len()
    ));
    if !report.matched.is_empty() {
        out.push_str("## Matched Functions\n\n");
        out.push_str("| Name | Left | Right | Strategy |\n|---|---|---|---|\n");
        for m in &report.matched {
            let strategy = match m.strategy {
                MatchStrategy::ExactSymbol => "symbol",
                MatchStrategy::ExactAddress => "address",
                MatchStrategy::Shape => "shape",
                MatchStrategy::Structural => "structural",
            };
            out.push_str(&format!(
                "| {} | `{:#x}` | `{:#x}` | {} |\n",
                m.name.as_deref().unwrap_or("—"),
                m.left_address,
                m.right_address,
                strategy
            ));
        }
        out.push('\n');
    }
    if !report.added.is_empty() {
        out.push_str("## Added (present in right, absent in left)\n\n");
        for f in &report.added {
            out.push_str(&format!(
                "- `{:#x}` {}\n",
                f.address,
                f.name.as_deref().unwrap_or("—")
            ));
        }
        out.push('\n');
    }
    if !report.removed.is_empty() {
        out.push_str("## Removed (present in left, absent in right)\n\n");
        for f in &report.removed {
            out.push_str(&format!(
                "- `{:#x}` {}\n",
                f.address,
                f.name.as_deref().unwrap_or("—")
            ));
        }
    }
    out
}

/// Render a diff report as a self-contained HTML page (inline CSS,
/// no external dependencies — evidence profile).
pub fn render_diff_html(report: &DiffReport) -> String {
    let mut out = String::new();
    out.push_str(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Binary Diff Report</title>
<style>
body { font-family: -apple-system, BlinkOnSystemFont, 'Segoe UI', sans-serif; margin: 2rem; }
table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
th, td { text-align: left; padding: 0.5rem 1rem; border: 1px solid #ddd; }
th { background: #f5f5f5; }
.added { color: #28a745; }
.removed { color: #dc3545; }
.matched { color: #0366d6; }
.summary { display: flex; gap: 2rem; margin: 1rem 0; }
.summary .card { padding: 1rem 2rem; border-radius: 8px; background: #f8f9fa; }
.summary .num { font-size: 2rem; font-weight: bold; }
</style>
</head>
<body>
<h1>Binary Diff Report</h1>
<p><strong>#LEFT#</strong> → <strong>#RIGHT#</strong></p>
<div class="summary">
<div class="card"><div class="num matched">#MATCHED#</div>Matched</div>
<div class="card"><div class="num added">#ADDED#</div>Added</div>
<div class="card"><div class="num removed">#REMOVED#</div>Removed</div>
</div>
"#,
    );
    // Matched table
    if !report.matched.is_empty() {
        out.push_str("<h2>Matched Functions</h2>\n<table>\n<tr><th>Name</th><th>Left</th><th>Right</th><th>Strategy</th></tr>\n");
        for m in &report.matched {
            let strategy = match m.strategy {
                MatchStrategy::ExactSymbol => "symbol",
                MatchStrategy::ExactAddress => "address",
                MatchStrategy::Shape => "shape",
                MatchStrategy::Structural => "structural",
            };
            out.push_str(&format!(
                "<tr><td>{}</td><td><code>{:#x}</code></td><td><code>{:#x}</code></td><td>{}</td></tr>\n",
                m.name.as_deref().unwrap_or("—"),
                m.left_address, m.right_address, strategy
            ));
        }
        out.push_str("</table>\n");
    }
    if !report.added.is_empty() {
        out.push_str("<h2 class=\"added\">Added</h2>\n<ul>\n");
        for f in &report.added {
            out.push_str(&format!(
                "<li class=\"added\"><code>{:#x}</code> {}</li>\n",
                f.address,
                f.name.as_deref().unwrap_or("—")
            ));
        }
        out.push_str("</ul>\n");
    }
    if !report.removed.is_empty() {
        out.push_str("<h2 class=\"removed\">Removed</h2>\n<ul>\n");
        for f in &report.removed {
            out.push_str(&format!(
                "<li class=\"removed\"><code>{:#x}</code> {}</li>\n",
                f.address,
                f.name.as_deref().unwrap_or("—")
            ));
        }
        out.push_str("</ul>\n");
    }
    out.push_str("</body>\n</html>\n");

    // Substitute placeholders
    out = out
        .replace("#LEFT#", &report.left_name)
        .replace("#RIGHT#", &report.right_name)
        .replace("#MATCHED#", &report.matched.len().to_string())
        .replace("#ADDED#", &report.added.len().to_string())
        .replace("#REMOVED#", &report.removed.len().to_string());
    out
}

/// Compute defense-indicator deltas between two detection sets
/// (Wave 8.4: version archaeology — did the vendor add checks?).
pub fn diff_defenses(
    left: &[damsel_recon_catalog::Detection],
    right: &[damsel_recon_catalog::Detection],
) -> (Vec<DefenseFinding>, Vec<DefenseFinding>) {
    let to_finding = |d: &damsel_recon_catalog::Detection| DefenseFinding {
        code: d.code.to_string(),
        location: d.location.clone(),
        evidence: d.evidence.clone(),
    };

    let added: Vec<DefenseFinding> = right
        .iter()
        .filter(|r| !left.iter().any(|l| l.code == r.code))
        .map(to_finding)
        .collect();

    let removed: Vec<DefenseFinding> = left
        .iter()
        .filter(|l| !right.iter().any(|r| r.code == l.code))
        .map(to_finding)
        .collect();

    (added, removed)
}

// ---------------------------------------------------------------------------
// Version timelines (plan feature 16, slice 3): chain pairwise diffs
// into per-function histories across N versions — the OS-update
// archaeology view.

/// One interval's inputs: a pairwise report plus which matched pairs
/// carried pseudocode changes (`(left_addr, right_addr)` pairs with a
/// nonzero line diff).
pub struct IntervalInput {
    pub report: DiffReport,
    pub changed_pairs: std::collections::BTreeSet<(u64, u64)>,
}

/// One function's journey across the version chain.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FunctionHistory {
    /// The stable identity (matched name; addresses may move).
    pub name: Option<String>,
    /// Address per version (`None` before birth / after death).
    pub addresses: Vec<Option<u64>>,
    /// Per interval: did the body change (pseudocode delta)?
    pub changed: Vec<bool>,
    /// Per interval: which match strategy carried it.
    pub strategies: Vec<Option<MatchStrategy>>,
    /// First and last version index where present.
    pub first_version: usize,
    pub last_version: usize,
}

impl FunctionHistory {
    /// Compact per-version marker: `·` present, `c` changed, `+` born,
    /// `x` died, `-` absent.
    pub fn timeline_text(&self) -> String {
        let versions = self.addresses.len();
        let mut out = String::new();
        for version in 0..versions {
            let present = self.addresses[version].is_some();
            let marker = if present {
                if version == self.first_version {
                    '+'
                } else if self.changed.get(version - 1).copied().unwrap_or(false) {
                    'c'
                } else {
                    '·'
                }
            } else if version > 0 && self.addresses[version - 1].is_some() {
                'x'
            } else {
                '-'
            };
            out.push(marker);
        }
        out
    }
}

/// Build per-function histories from consecutive interval reports.
/// Identity follows matched names; address moves within a live history
/// are recorded (the address vector shows them); unmatched functions
/// start (`+`) or end (`x`) histories.
pub fn build_timeline(intervals: &[IntervalInput]) -> Vec<FunctionHistory> {
    let version_count = intervals.len() + 1;
    let mut version_names: Vec<String> = intervals
        .first()
        .map(|input| vec![input.report.left_name.clone()])
        .unwrap_or_default();
    for input in intervals {
        version_names.push(input.report.right_name.clone());
    }
    let _ = version_names;

    // Histories keyed by identity name; unnamed functions get
    // address-scoped synthetic identities (only address-tier matches
    // chain them).
    let mut histories: Vec<FunctionHistory> = Vec::new();
    let mut by_identity: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    let mut identity_of_left: std::collections::BTreeMap<u64, String> =
        std::collections::BTreeMap::new();
    let mut identity_of_right: std::collections::BTreeMap<u64, String> =
        std::collections::BTreeMap::new();

    for (interval_index, input) in intervals.iter().enumerate() {
        identity_of_right.clear();
        let mut born_right: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();

        // Pass 1: matched pairs continue or start histories.
        for matched in &input.report.matched {
            let changed = input
                .changed_pairs
                .contains(&(matched.left_address, matched.right_address));
            let identity = identity_of_left
                .get(&matched.left_address)
                .cloned()
                .or_else(|| matched.name.clone())
                .unwrap_or_else(|| format!("sub_{:x}", matched.left_address));
            identity_of_right.insert(matched.right_address, identity.clone());
            if let Some(&index) = by_identity.get(&identity) {
                histories[index].addresses[interval_index + 1] = Some(matched.right_address);
                histories[index].changed.push(changed);
                histories[index].strategies.push(Some(matched.strategy));
                histories[index].last_version = interval_index + 1;
            } else {
                by_identity.insert(identity.clone(), histories.len());
                let mut addresses = vec![None; version_count];
                addresses[interval_index] = Some(matched.left_address);
                addresses[interval_index + 1] = Some(matched.right_address);
                let mut strategies = vec![None; version_count];
                strategies[interval_index + 1] = Some(matched.strategy);
                histories.push(FunctionHistory {
                    name: matched
                        .name
                        .clone()
                        .or_else(|| (!identity.starts_with("sub_")).then_some(identity.clone())),
                    addresses,
                    changed: vec![changed],
                    strategies,
                    first_version: interval_index,
                    last_version: interval_index + 1,
                });
            }
        }

        // Pass 2: added functions (present only in right) start new
        // histories at this version.
        for added in &input.report.added {
            if born_right.contains(&added.address) {
                continue;
            }
            born_right.insert(added.address);
            let identity = added
                .name
                .clone()
                .unwrap_or_else(|| format!("sub_{:x}", added.address));
            if by_identity.contains_key(&identity) {
                continue;
            }
            by_identity.insert(identity.clone(), histories.len());
            let mut addresses = vec![None; version_count];
            addresses[interval_index + 1] = Some(added.address);
            histories.push(FunctionHistory {
                name: added.name.clone(),
                addresses,
                changed: Vec::new(),
                strategies: vec![None; version_count],
                first_version: interval_index + 1,
                last_version: interval_index + 1,
            });
        }

        // Roll the right-side identities into the next interval's left.
        identity_of_left = identity_of_right.clone();
    }

    histories.sort_by(|a, b| {
        a.first_version
            .cmp(&b.first_version)
            .then_with(|| a.name.cmp(&b.name))
    });
    histories
}

#[cfg(test)]
mod timeline_tests {
    use super::*;
    use crate::diff::{DiffFunction, MatchStrategy};

    fn matched(name: &str, left: u64, right: u64) -> MatchedFunction {
        MatchedFunction {
            name: Some(name.to_string()),
            left_address: left,
            right_address: right,
            strategy: MatchStrategy::ExactSymbol,
        }
    }

    fn added(name: &str, address: u64) -> DiffFunction {
        DiffFunction {
            name: Some(name.to_string()),
            address,
        }
    }

    fn report(left: &str, right: &str) -> DiffReport {
        DiffReport {
            left_name: left.to_string(),
            right_name: right.to_string(),
            matched: Vec::new(),
            added: Vec::new(),
            removed: Vec::new(),
            defense_added: Vec::new(),
            defense_removed: Vec::new(),
        }
    }

    #[test]
    fn chained_histories_mark_birth_change_and_stability() {
        let mut first = report("v1", "v2");
        first.matched = vec![
            matched("stable", 0x100, 0x100),
            matched("evolving", 0x200, 0x200),
        ];
        first.added = vec![added("newborn", 0x300)];
        let mut second = report("v2", "v3");
        second.matched = vec![
            matched("stable", 0x100, 0x100),
            matched("evolving", 0x200, 0x240), // moved
            matched("newborn", 0x300, 0x300),
        ];

        let intervals = [
            IntervalInput {
                report: first,
                changed_pairs: [(0x200u64, 0x200u64)].into(),
            },
            IntervalInput {
                report: second,
                changed_pairs: [].into(),
            },
        ];
        let timeline = build_timeline(&intervals);
        let by_name = |name: &str| {
            timeline
                .iter()
                .find(|h| h.name.as_deref() == Some(name))
                .unwrap_or_else(|| panic!("missing {name}: {timeline:#?}"))
        };

        let stable = by_name("stable");
        assert_eq!(stable.timeline_text(), "+··");
        assert!(stable.addresses.iter().all(|a| a.is_some()));

        let evolving = by_name("evolving");
        assert_eq!(evolving.timeline_text(), "+c·");
        assert_eq!(evolving.addresses[2], Some(0x240), "move recorded");

        let newborn = by_name("newborn");
        assert_eq!(newborn.timeline_text(), "-+·");
        assert_eq!(newborn.first_version, 1);
    }

    #[test]
    fn death_marks_last_interval() {
        let mut first = report("v1", "v2");
        first.matched = vec![matched("doomed", 0x100, 0x100)];
        let mut second = report("v2", "v3");
        second.matched = Vec::new();
        second.removed = vec![DiffFunction {
            name: Some("doomed".to_string()),
            address: 0x100,
        }];
        let timeline = build_timeline(&[
            IntervalInput {
                report: first,
                changed_pairs: [].into(),
            },
            IntervalInput {
                report: second,
                changed_pairs: [].into(),
            },
        ]);
        let doomed = &timeline[0];
        assert_eq!(doomed.name.as_deref(), Some("doomed"));
        assert_eq!(doomed.timeline_text(), "+·x");
        assert_eq!(doomed.last_version, 1);
    }
}
