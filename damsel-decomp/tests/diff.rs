//! Diff contract tests: exact symbol + address matching, add/remove
//! reporting, determinism.

mod support;

use damsel_decomp::{MatchStrategy, diff_functions};
use damsel_project::{AnalysisProjectMeta, Create, ProjectId, ProjectStore, TypedStore, open_store};

fn meta(name: &str) -> AnalysisProjectMeta {
    AnalysisProjectMeta {
        project_id: ProjectId::new(format!("diff-{name}")),
        name: name.to_string(),
        analyzer_version: "0.1.0".to_string(),
    }
}

fn analyze_fixture(dir: &std::path::Path, name: &str) -> i64 {
    let image = damsel_macho::load(support::fixture(name)).unwrap();
    let mut store = open_store(&dir.join(format!("{name}.db")), Create::New, &meta(name)).unwrap();
    let outcome = damsel_macho::ingest_binary_image(&mut store, &image, "0.1.0").unwrap();
    outcome.source_id
}

#[test]
fn identical_binaries_match_all_functions() {
    let dir = tempfile::tempdir().unwrap();
    // Two separate stores from the same fixture
    let mut left = open_store(&dir.path().join("left.db"), Create::New, &meta("left")).unwrap();
    let mut right = open_store(&dir.path().join("right.db"), Create::New, &meta("right")).unwrap();
    let image = damsel_macho::load(support::fixture("arm64-symbolized")).unwrap();
    damsel_macho::ingest_binary_image(&mut left, &image, "0.1.0").unwrap();
    damsel_macho::ingest_binary_image(&mut right, &image, "0.1.0").unwrap();

    let report = diff_functions(&left, &right, "left", "right").unwrap();
    assert!(report.matched.len() >= 3, "expected _main etc., got {}", report.matched.len());
    assert!(report.matched.iter().all(|m| m.strategy == MatchStrategy::ExactSymbol));
    assert!(report.added.is_empty());
    assert!(report.removed.is_empty());

    // Deterministic
    let again = diff_functions(&left, &right, "left", "right").unwrap();
    assert_eq!(serde_json::to_string(&report).unwrap(), serde_json::to_string(&again).unwrap());
}

#[test]
fn stripped_vs_symbolized_finds_common_functions() {
    let dir = tempfile::tempdir().unwrap();
    let mut left = open_store(&dir.path().join("sym.db"), Create::New, &meta("sym")).unwrap();
    let mut right = open_store(&dir.path().join("str.db"), Create::New, &meta("str")).unwrap();
    let sym = damsel_macho::load(support::fixture("arm64-symbolized")).unwrap();
    let stripped = damsel_macho::load(support::fixture("arm64-stripped")).unwrap();
    damsel_macho::ingest_binary_image(&mut left, &sym, "0.1.0").unwrap();
    damsel_macho::ingest_binary_image(&mut right, &stripped, "0.1.0").unwrap();

    let report = diff_functions(&left, &right, "symbolized", "stripped").unwrap();
    // The stripped variant has function_starts seeds at the same
    // addresses as the symbolized variant → address matches.
    assert!(report.matched.len() >= 2, "expected address matches, got {}", report.matched.len());
    assert!(report.matched.iter().any(|m| m.strategy == MatchStrategy::ExactAddress));
}

#[test]
fn different_binaries_report_adds_and_removes() {
    let dir = tempfile::tempdir().unwrap();
    let mut left = open_store(&dir.path().join("hello.db"), Create::New, &meta("hello")).unwrap();
    let mut right = open_store(&dir.path().join("obje.db"), Create::New, &meta("obje")).unwrap();
    let hello = damsel_macho::load(support::fixture("arm64-symbolized")).unwrap();
    let objc = damsel_macho::load(support::fixture("objc-sample")).unwrap();
    damsel_macho::ingest_binary_image(&mut left, &hello, "0.1.0").unwrap();
    damsel_macho::ingest_binary_image(&mut right, &objc, "0.1.0").unwrap();

    let report = diff_functions(&left, &right, "hello", "objc").unwrap();
    assert!(!report.removed.is_empty(), "hello functions absent from objc");
    assert!(!report.added.is_empty(), "objc functions absent from hello");
}
