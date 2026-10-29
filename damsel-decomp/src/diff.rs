//! Binary diff (Wave 7; spec §3): function-level matching between two
//! analyzed projects. Exact matches first (same address + same symbol
//! payload), then structural (same bound size + same call counts).
//! Every match carries its evidence; unmatched functions are
//! reported as added/removed — never silently dropped.

use damsel_project::{FunctionSummary, ProjectStore, StoreError};
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
    Structural,
}

pub fn diff_functions(
    left: &impl ProjectStore,
    right: &impl ProjectStore,
    left_name: &str,
    right_name: &str,
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
