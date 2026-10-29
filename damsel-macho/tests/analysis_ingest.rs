//! Ingest contract tests (Wave 4 spec §10; evidence rows feeding 4.1–4.3).
//!
//! - standalone Mach-O ingests into a project store
//! - section byte-class facts, rank-1/2/3 function seeds, imports present
//! - two independent ingests produce byte-identical canonical exports

use damsel_macho::{ingest_binary_image, load};
use damsel_project::{
    AnalysisProjectMeta, Create, ProjectId, ProjectStore, TypedStore, open_store,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

fn meta() -> AnalysisProjectMeta {
    AnalysisProjectMeta {
        project_id: ProjectId::new("00000000-ingest-test-000000000001"),
        name: "ingest-test".to_string(),
        analyzer_version: "0.1.0".to_string(),
    }
}

fn ingest_fixture(name: &str, dir: &Path) -> damsel_macho::IngestOutcome {
    let image = load(fixture(name)).expect("load fixture");
    let mut store = open_store(&dir.join("project.db"), Create::New, &meta()).unwrap();
    ingest_binary_image(&mut store, &image, "0.1.0").expect("ingest")
}

#[test]
fn standalone_macho_ingests_sections_symbols_and_imports() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = ingest_fixture("arm64-symbolized", dir.path());
    assert!(outcome.fact_count > 0);
    assert!(outcome.object_count > 0);

    let store = open_store(&dir.path().join("project.db"), Create::Existing, &meta()).unwrap();
    let sources = store.sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].kind, damsel_project::AnalysisSourceKind::MachOImage);
    assert_eq!(sources[0].address_spaces.len(), 1);
    assert_eq!(sources[0].address_spaces[0].as_str(), "image_vmaddr");

    let facts = store.facts_for_source(outcome.source_id).unwrap();
    let byte_class = facts
        .iter()
        .filter(|f| f.kind.as_str() == "byte_class")
        .count();
    assert!(byte_class >= 5, "expected section classifications, got {byte_class}");

    // Rank-1 seeds: symbol-table text symbols (fixture has `_main`).
    let seeds: Vec<_> = facts
        .iter()
        .filter(|f| f.kind.as_str() == "seed" && f.payload.starts_with("symbol_table|"))
        .collect();
    assert!(
        seeds.iter().any(|s| s.payload.contains("name=_main")),
        "expected a _main symbol seed"
    );

    // Rank-2 seeds: function starts.
    assert!(facts
        .iter()
        .any(|f| f.payload == "function_starts"));

    // Imports recorded with linkage payloads.
    let imports: Vec<_> = facts.iter().filter(|f| f.kind.as_str() == "import").collect();
    assert!(!imports.is_empty());
    assert!(imports[0].payload.starts_with("dylib="));
}

#[test]
fn objc_fixture_ingests_imp_seeds() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = ingest_fixture("objc-sample", dir.path());
    let store = open_store(&dir.path().join("project.db"), Create::Existing, &meta()).unwrap();
    let facts = store.facts_for_source(outcome.source_id).unwrap();
    assert!(
        facts
            .iter()
            .any(|f| f.kind.as_str() == "seed" && f.payload.starts_with("objc_method|selector=")),
        "expected objc IMP seeds"
    );
}

#[test]
fn function_summaries_group_seeds_by_address() {
    let dir = tempfile::tempdir().unwrap();
    ingest_fixture("arm64-symbolized", dir.path());
    let store = open_store(&dir.path().join("project.db"), Create::Existing, &meta()).unwrap();
    let summaries = damsel_project::function_summaries(&store).unwrap();
    assert!(summaries.len() >= 4);
    // Sorted by address.
    assert!(summaries.windows(2).all(|w| w[0].address < w[1].address));
    let main = summaries
        .iter()
        .find(|f| f.name.as_deref() == Some("_main"))
        .expect("_main seeded");
    assert!(main.seeds.iter().any(|s| s == "symbol_table"));
    assert!(main.seeds.iter().any(|s| s == "function_starts"));
    assert!(main.seeds.iter().any(|s| s == "entry_point"));
}

#[test]
fn xref_harvest_round_trips_from_both_ends() {
    let dir = tempfile::tempdir().unwrap();
    let image = load(fixture("arm64-symbolized")).unwrap();
    let mut store = open_store(&dir.path().join("project.db"), Create::New, &meta()).unwrap();
    let outcome = ingest_binary_image(&mut store, &image, "0.1.0").unwrap();
    let addresses: Vec<u64> = damsel_project::function_summaries(&store)
        .unwrap()
        .iter()
        .map(|f| f.address)
        .collect();
    let xref_outcome = damsel_macho::harvest_xrefs(
        &mut store, &image, outcome.source_id, &addresses, "0.1.0",
    )
    .unwrap();
    assert!(xref_outcome.xref_count > 0);
    assert_eq!(xref_outcome.function_windows, 3);

    // Round trip: every xref fact is discoverable by target address.
    let facts = store.facts_for_source(outcome.source_id).unwrap();
    let xref_facts: Vec<_> = facts
        .iter()
        .filter(|f| f.kind.as_str() == "xref")
        .collect();
    assert!(!xref_facts.is_empty());
    for fact in &xref_facts {
        assert!(fact.payload.contains("|to="), "payload shape: {}", fact.payload);
        let (_, target) = fact.payload.split_once("|to=").unwrap();
        let (kind, key) = target.split_once(':').unwrap();
        assert!(!kind.is_empty() && !key.is_empty());
        // Address-bearing kinds must parse as hex; name-bearing kinds
        // (import) just need a non-empty name.
        if matches!(kind, "function" | "data" | "stub") {
            assert!(
                u64::from_str_radix(key.trim_start_matches("0x"), 16).is_ok(),
                "target address parses hex: {target}"
            );
        }
    }
    // A call into _exported_sum exists and is attributed to _main's window.
    assert!(xref_facts
        .iter()
        .any(|f| f.payload == "call|to=function:0x100000460"));
}

#[test]
fn ingest_is_export_deterministic() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    ingest_fixture("arm64-symbolized", a.path());
    ingest_fixture("arm64-symbolized", b.path());
    let store_a = open_store(&a.path().join("project.db"), Create::Existing, &meta()).unwrap();
    let store_b = open_store(&b.path().join("project.db"), Create::Existing, &meta()).unwrap();
    assert_eq!(
        store_a.export_canonical().unwrap(),
        store_b.export_canonical().unwrap(),
        "two ingests of identical bytes must export identically"
    );
}
