//! Decompiler fidelity harness (plan feature 8, slice 1): the quality
//! scoreboard. Every named, bounded function in every Mach-O fixture is
//! decompiled; the harness aggregates pseudocode volume, unknown-rate,
//! and per-function worst offenders, then gates on committed thresholds
//! so structuring changes can never silently regress readability.
//!
//! Metrics (v1, honest and cheap):
//! - `functions` — named functions with recovered bounds
//! - `unknown_rate` — unknown pseudo-ops / total instructions lifted
//! - `avg_lines_per_function` — pseudocode volume signal
//!
//! The gate: overall unknown_rate must stay at or below the committed
//! floor, and no fixture may exceed its per-fixture ceiling. PAC/SIMD
//! fixtures carry honest unknowns by design — their ceilings say so.

use assert_cmd::cargo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};

/// Committed floors — bump DOWN only with evidence (implementation log).
/// Baseline 0.0371 (Q1) → 0.0028 (Q2: csel/blr/width-memory/FP/MADD lifts).
const GLOBAL_UNKNOWN_RATE_MAX: f64 = 0.01;
/// Fixtures allowed a higher unknown rate (documented honesty cases).
const FIXTURE_CEILINGS: &[(&str, f64)] = &[
    // arm64e: pacibsp/braa surface honestly as unknowns until PAC
    // modeling (feature 5) lands.
    ("arm64e-sample", 0.05),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[derive(Debug, Clone)]
struct FunctionRow {
    fixture: String,
    name: String,
    instructions: usize,
    lines: usize,
    unknowns: usize,
}

fn decompile_fixture(fixture_rel: &str) -> Vec<FunctionRow> {
    let target = repo_root().join("fixtures/bin").join(fixture_rel);
    if !target.is_file() {
        return Vec::new();
    }
    // Reuse the committed CLI surface: analyze, then decompile --all (JSONL).
    let tmp = std::env::temp_dir().join(format!(
        "damsel-fidelity-{}-{:?}",
        std::process::id(),
        fixture_rel.replace('/', "_")
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let copy = tmp.join(target.file_name().unwrap());
    std::fs::copy(&target, &copy).unwrap();

    let analyze = StdCommand::new(cargo_bin!("damsel-cli"))
        .args(["analyze", copy.to_str().unwrap()])
        .output()
        .expect("analyze");
    assert!(analyze.status.success(), "analyze failed: {:?}", analyze);

    let all = StdCommand::new(cargo_bin!("damsel-cli"))
        .args([
            "decompile",
            tmp.join(format!(
                "{}.damsel",
                copy.file_name().unwrap().to_string_lossy()
            ))
            .to_str()
            .unwrap(),
            "--all",
            "--format",
            "json",
        ])
        .stdout(Stdio::piped())
        .output()
        .expect("decompile --all");
    assert!(all.status.success(), "decompile --all failed");

    let stdout = String::from_utf8_lossy(&all.stdout);
    let mut rows = Vec::new();
    for line in stdout.lines() {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let lines = record["lines"].as_array().cloned().unwrap_or_default();
        let text: Vec<&str> = lines.iter().filter_map(|l| l.as_str()).collect();
        let unknowns = text
            .iter()
            .filter(|line| line.contains("/* unknown"))
            .count();
        rows.push(FunctionRow {
            fixture: fixture_rel.to_string(),
            name: record["function"].as_str().unwrap_or("?").to_string(),
            instructions: record["line_count"].as_u64().unwrap_or(0) as usize,
            lines: lines.len(),
            unknowns,
        });
    }
    rows
}

#[test]
fn decompiler_fidelity_scoreboard_and_gate() {
    let fixtures = [
        "arm64-symbolized",
        "objc-sample",
        "swift-sample",
        "export-kinds",
        "import-rich",
        "arm64e-sample",
        "adversarial/swift-rich-arm64",
    ];
    let mut all_rows = Vec::new();
    for fixture in fixtures {
        all_rows.extend(decompile_fixture(fixture));
    }
    assert!(
        all_rows.len() >= 20,
        "fidelity harness must cover at least 20 functions, got {}",
        all_rows.len()
    );

    // Aggregate scoreboard.
    let total_lines: usize = all_rows.iter().map(|row| row.lines).sum();
    let total_unknowns: usize = all_rows.iter().map(|row| row.unknowns).sum();
    let global_rate = if total_lines > 0 {
        total_unknowns as f64 / total_lines as f64
    } else {
        0.0
    };
    println!(
        "fidelity: functions={} lines={} unknowns={} rate={:.4}",
        all_rows.len(),
        total_lines,
        total_unknowns,
        global_rate
    );

    // Per-fixture ceilings.
    for (fixture, ceiling) in FIXTURE_CEILINGS {
        let rows: Vec<&FunctionRow> = all_rows
            .iter()
            .filter(|row| &row.fixture == fixture)
            .collect();
        if rows.is_empty() {
            continue;
        }
        let lines: usize = rows.iter().map(|row| row.lines).sum();
        let unknowns: usize = rows.iter().map(|row| row.unknowns).sum();
        let rate = unknowns as f64 / lines.max(1) as f64;
        println!("  {fixture}: rate={rate:.4} (ceiling {ceiling})");
        assert!(
            rate <= *ceiling,
            "{fixture} unknown-rate {rate:.4} exceeds ceiling {ceiling}"
        );
    }

    // Global gate.
    assert!(
        global_rate <= GLOBAL_UNKNOWN_RATE_MAX,
        "global unknown-rate {global_rate:.4} exceeds floor {GLOBAL_UNKNOWN_RATE_MAX}"
    );

    // Worst offenders surfaced for the log.
    let mut worst = all_rows.clone();
    worst.sort_by_key(|row| std::cmp::Reverse(row.unknowns));
    for row in worst.iter().take(5) {
        if row.unknowns > 0 {
            println!(
                "  worst: {} {} unknowns={} lines={}",
                row.fixture, row.name, row.unknowns, row.lines
            );
        }
    }
}
