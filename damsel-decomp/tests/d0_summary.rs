//! D0 summary contract tests: one Mach-O and one ELF fixture, proving
//! the summary surface is format-neutral end to end.

mod support;

use damsel_decomp::summarize_named;
use damsel_project::{AnalysisProjectMeta, Create, ProjectId, ProjectStore, TypedStore, open_store};

fn meta() -> AnalysisProjectMeta {
    AnalysisProjectMeta {
        project_id: ProjectId::new("00000000-decomp-d0-000000000001"),
        name: "decomp-d0".to_string(),
        analyzer_version: "0.1.0".to_string(),
    }
}

#[test]
fn macho_main_summary_lists_calls_imports_and_strings() {
    let dir = tempfile::tempdir().unwrap();
    let image = damsel_macho::load(support::fixture("arm64-symbolized")).unwrap();
    let mut store = open_store(&dir.path().join("project.db"), Create::New, &meta()).unwrap();
    let outcome = damsel_macho::ingest_binary_image(&mut store, &image, "0.1.0").unwrap();
    let seeds: Vec<u64> = damsel_project::function_summaries(&store)
        .unwrap()
        .iter()
        .map(|f| f.address)
        .collect();
    damsel_macho::harvest_xrefs(&mut store, &image, outcome.source_id, &seeds, "0.1.0").unwrap();
    damsel_macho::recover_functions(&mut store, &image, outcome.source_id, &seeds, "0.1.0").unwrap();
    damsel_macho::harvest_strings(&mut store, &image, outcome.source_id, "0.1.0").unwrap();

    let report = summarize_named(&store, outcome.source_id, "_main").unwrap();
    assert_eq!(report.name.as_deref(), Some("_main"));
    assert!(report.bound.as_deref().is_some_and(|b| b.contains("stop=return")));
    // Direct calls inside _main's recovered window.
    assert!(report.calls.iter().any(|c| c.kind == "call" && !c.unresolved));
    // The import of _puts (via stub) is referenced.
    assert!(report.imports.iter().any(|i| i.contains("puts")));
    // The fixture string is joined through a data reference.
    assert!(!report.strings.is_empty(), "expected a joined string");
    assert!(report.problems.is_empty());
}

#[test]
fn elf_exported_func_summary_lists_call_target() {
    let dir = tempfile::tempdir().unwrap();
    let image = damsel_elf::load_bytes(None, damsel_elf::synth::shared_object()).unwrap();
    let mut store = open_store(&dir.path().join("project.db"), Create::New, &meta()).unwrap();
    let outcome = damsel_elf::ingest_elf_image(&mut store, &image, "0.1.0").unwrap();
    let seeds: Vec<u64> = damsel_project::function_summaries(&store)
        .unwrap()
        .iter()
        .map(|f| f.address)
        .collect();
    damsel_elf::harvest_elf_xrefs(&mut store, &image, outcome.source_id, &seeds, "0.1.0").unwrap();
    damsel_elf::harvest_elf_strings(&mut store, &image, outcome.source_id, "0.1.0").unwrap();

    let report = summarize_named(&store, outcome.source_id, "exported_func").unwrap();
    assert_eq!(report.name.as_deref(), Some("exported_func"));
    assert!(report.bound.as_deref().is_some_and(|b| b.contains("stop=return")));
    // The blob's call to the local helper (0xf7) is summarized.
    assert!(report
        .calls
        .iter()
        .any(|c| c.kind == "call" && c.target == "function:0xf7"));
}

#[test]
fn unknown_and_ambiguous_selectors_are_typed() {
    let dir = tempfile::tempdir().unwrap();
    let image = damsel_elf::load_bytes(None, damsel_elf::synth::shared_object()).unwrap();
    let mut store = open_store(&dir.path().join("project.db"), Create::New, &meta()).unwrap();
    let outcome = damsel_elf::ingest_elf_image(&mut store, &image, "0.1.0").unwrap();

    assert!(matches!(
        summarize_named(&store, outcome.source_id, "no_such_fn"),
        Err(damsel_decomp::SummaryError::FunctionNotFound(_))
    ));
    assert!(matches!(
        summarize_named(&store, outcome.source_id, "0xdeadbeef"),
        Err(damsel_decomp::SummaryError::FunctionNotFound(_))
    ));
}
