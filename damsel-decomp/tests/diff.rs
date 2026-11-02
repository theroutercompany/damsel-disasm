//! Diff contract tests: exact symbol + address matching, add/remove
//! reporting, determinism.

mod support;

use damsel_decomp::{MatchStrategy, diff_functions};
use damsel_project::{
    AnalysisProjectMeta, Create, ProjectId, ProjectStore, TypedStore, open_store,
};

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
    assert!(
        report.matched.len() >= 3,
        "expected _main etc., got {}",
        report.matched.len()
    );
    assert!(
        report
            .matched
            .iter()
            .all(|m| m.strategy == MatchStrategy::ExactSymbol)
    );
    assert!(report.added.is_empty());
    assert!(report.removed.is_empty());

    // Deterministic
    let again = diff_functions(&left, &right, "left", "right").unwrap();
    assert_eq!(
        serde_json::to_string(&report).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
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
    assert!(
        report.matched.len() >= 2,
        "expected address matches, got {}",
        report.matched.len()
    );
    assert!(
        report
            .matched
            .iter()
            .any(|m| m.strategy == MatchStrategy::ExactAddress)
    );
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
    assert!(
        !report.removed.is_empty(),
        "hello functions absent from objc"
    );
    assert!(!report.added.is_empty(), "objc functions absent from hello");
}

/// Diff v2 shape tier (plan feature 16): same body, different address
/// and name — the signature-based pass pairs what symbol/address tiers
/// miss, uniquely and never ambiguously.
#[test]
fn shape_tier_matches_moved_functions() {
    use damsel_decomp::{ShapeMap, diff_functions_with_shapes};
    use damsel_project::{AnalysisFact, Confidence, FactKind, FactProvenance};

    let dir = tempfile::tempdir().unwrap();
    let mut left = open_store(&dir.path().join("l.db"), Create::New, &meta("l")).unwrap();
    let mut right = open_store(&dir.path().join("r.db"), Create::New, &meta("r")).unwrap();

    // Fabricated stores need a source row before facts.
    use damsel_project::{AddressSpaceId, AnalysisSourceKind, SourceRecord};
    let source = SourceRecord {
        source_id: 0,
        kind: AnalysisSourceKind::MachOImage,
        label: Some("t".to_string()),
        content_hash: None,
        parent_source_id: None,
        address_spaces: vec![AddressSpaceId::new("image_vmaddr")],
    };
    left.add_source(source.clone()).unwrap();
    right.add_source(source).unwrap();

    let seed = |address: &str, symbol: &str| AnalysisFact {
        source_id: 1,
        object_id: Some(damsel_project::ObjectId::new("function", address)),
        kind: FactKind::new("seed"),
        payload: format!("symbol_table|name={symbol}|size=16"),
        confidence: Confidence::Strong,
        provenance: FactProvenance::analyzer("t", "0"),
    };
    // Left: 0x1000 "alpha". Right: same body relocated to 0x9000, renamed
    // "beta". Symbol and address tiers both miss; the shape tier pairs.
    left.put_fact(seed("0x1000", "alpha")).unwrap();
    right.put_fact(seed("0x9000", "beta")).unwrap();

    let mut left_shapes = ShapeMap::new();
    left_shapes.insert(0x1000, (0xdeadbeef, 12));
    let mut right_shapes = ShapeMap::new();
    right_shapes.insert(0x9000, (0xdeadbeef, 12));

    let report = diff_functions_with_shapes(
        &left,
        &right,
        "l",
        "r",
        Some(&left_shapes),
        Some(&right_shapes),
    )
    .unwrap();
    assert_eq!(report.matched.len(), 1);
    assert_eq!(report.matched[0].strategy, MatchStrategy::Shape);
    assert_eq!(report.matched[0].left_address, 0x1000);
    assert_eq!(report.matched[0].right_address, 0x9000);
    assert!(report.added.is_empty() && report.removed.is_empty());

    // Ambiguous shapes (two candidates) stay added/removed — never guessed.
    let mut right_shapes2 = ShapeMap::new();
    right_shapes2.insert(0x9000, (0xdeadbeef, 12));
    right_shapes2.insert(0xa000, (0xdeadbeef, 12));
    right.put_fact(seed("0xa000", "beta2")).unwrap();
    let report = diff_functions_with_shapes(
        &left,
        &right,
        "l",
        "r",
        Some(&left_shapes),
        Some(&right_shapes2),
    )
    .unwrap();
    assert!(
        report
            .matched
            .iter()
            .all(|m| m.strategy != MatchStrategy::Shape)
    );
    assert_eq!(report.added.len(), 2);
    assert_eq!(report.removed.len(), 1);

    // Without shape maps the legacy two-tier behavior is identical.
    let legacy = diff_functions(&left, &right, "l", "r").unwrap();
    assert!(legacy.matched.is_empty());
    assert_eq!(legacy.added.len(), 2);
    assert_eq!(legacy.removed.len(), 1);
}
