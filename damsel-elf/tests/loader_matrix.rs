//! Loader matrix tests for synthetic ELF64 fixtures (Wave 5B.1).
//! Mirrors the Mach-O matrix discipline: valid payloads, typed
//! failures, no panics.

mod support;

use damsel_elf::{ElfDynamicEntry, ElfError, load_bytes};
use support::synth;

#[test]
fn static_exec_loads_sections_symbols_and_build_id() {
    let image = load_bytes(Some("static-exec".into()), synth::static_exec())
        .expect("synthetic static exec parses");
    assert_eq!(image.architecture, damsel_core::Architecture::X86_64);
    assert!(!image.is_position_independent);
    assert_eq!(
        image.build_id.as_deref(),
        Some("aabbccddee112233"),
        "GNU build-id note must decode to hex"
    );

    let text = image
        .sections
        .iter()
        .find(|s| s.name == ".text")
        .expect(".text section");
    assert!(text.executable);
    assert_eq!(text.kind, "text");
    assert!(image.executable_sections().len() >= 1);

    let truth = image.text_symbols();
    assert_eq!(truth.len(), 2, "main_func + helper_func");
    assert!(truth.iter().any(|s| s.name == "main_func"));
    assert!(truth.iter().any(|s| s.name == "helper_func"));
}

#[test]
fn shared_object_loads_dynamic_linkage() {
    let image = load_bytes(Some("shared".into()), synth::shared_object())
        .expect("synthetic shared object parses");
    assert!(image.is_position_independent);
    assert_eq!(image.soname.as_deref(), Some("fake.so.1"));
    assert!(image
        .dynamic
        .iter()
        .any(|e| matches!(e, ElfDynamicEntry::Needed(n) if n == "libfake.so.1")));

    let exported = image
        .symbols
        .iter()
        .find(|s| s.name == "exported_func")
        .expect("exported symbol");
    assert!(exported.defined && exported.dynamic && exported.is_text);

    let import = image
        .symbols
        .iter()
        .find(|s| s.name == "malloc")
        .expect("undefined import");
    assert!(!import.defined && import.is_import);

    let dynamic_segment = image
        .segments
        .iter()
        .find(|s| s.executable && s.writable);
    assert!(dynamic_segment.is_none(), "no segment is both X and W");
}

#[test]
fn elf_ingest_feeds_wave4_queries() {
    use damsel_project::{Create, ProjectStore, open_store};

    let dir = tempfile::tempdir().unwrap();
    let mut store = open_store(&dir.path().join("project.db"), Create::New, &meta()).unwrap();
    let image = load_bytes(Some("libfake".into()), synth::shared_object()).unwrap();
    let outcome = damsel_elf::ingest_elf_image(&mut store, &image, "0.1.0").unwrap();
    assert!(outcome.fact_count > 0);

    let sources = store.sources().unwrap();
    assert_eq!(sources[0].kind, damsel_project::AnalysisSourceKind::ElfObject);
    assert_eq!(sources[0].address_spaces[0].as_str(), "elf_vaddr");

    // Wave 4 query surface works unchanged across formats.
    let summaries = damsel_project::function_summaries(&store).unwrap();
    assert!(summaries.iter().any(|f| f.name.as_deref() == Some("exported_func")));

    let facts = store.facts().unwrap();
    assert!(facts
        .iter()
        .any(|f| f.kind.as_str() == "linkage" && f.payload == "dt_needed|libfake.so.1"));
    assert!(facts
        .iter()
        .any(|f| f.kind.as_str() == "import" && f.object_id.as_ref().unwrap().key() == "malloc"));
}

fn meta() -> damsel_project::AnalysisProjectMeta {
    damsel_project::AnalysisProjectMeta {
        project_id: damsel_project::ProjectId::new("00000000-elf-ingest-000000000001"),
        name: "elf-ingest".to_string(),
        analyzer_version: "0.1.0".to_string(),
    }
}

/// Wave 5B.3: section-header-stripped binaries parse via the segment-only
/// PT_DYNAMIC walker — the gap both `object` and `goblin` punt on.
#[test]
fn nosections_loads_via_segment_only_walker() {
    let image = load_bytes(Some("hard-stripped".into()), synth::elf_nosections())
        .expect("hard-stripped ELF parses via PT_DYNAMIC walker");
    assert!(image.sections.is_empty(), "no section headers to read");
    assert_eq!(image.architecture, damsel_core::Architecture::X86_64);
    assert!(image.is_position_independent);
    assert_eq!(image.soname.as_deref(), Some("fake.so.1"));
    assert!(image
        .dynamic
        .iter()
        .any(|e| matches!(e, ElfDynamicEntry::Needed(n) if n == "libfake.so.1")));

    // Walker-resolved symbols: same truth as the section-ful variant.
    let exported = image
        .symbols
        .iter()
        .find(|s| s.name == "exported_func")
        .expect("exported_func via walker");
    assert!(exported.defined);
    let import = image
        .symbols
        .iter()
        .find(|s| s.name == "malloc")
        .expect("malloc import via walker");
    assert!(import.is_import);

    // Segment truth survives stripping: PT_LOAD + PT_DYNAMIC present.
    assert!(image.segments.iter().any(|s| s.kind == "load"));
    assert!(image.segments.iter().any(|s| s.kind == "dynamic"));
}

#[test]
fn versioned_symbols_resolve_on_both_paths() {
    // Section-ful path.
    let image = load_bytes(Some("versioned".into()), synth::shared_object()).unwrap();
    let exported = image
        .symbols
        .iter()
        .find(|s| s.name == "exported_func")
        .expect("exported");
    assert_eq!(exported.version.as_deref(), Some("FAKE_1.0"));
    let import = image
        .symbols
        .iter()
        .find(|s| s.name == "malloc")
        .expect("import");
    assert_eq!(import.version.as_deref(), Some("GLIBC_2.2.5"));

    // Segment-only walker path (same bytes, section headers zeroed).
    let stripped = load_bytes(Some("versioned-stripped".into()), synth::elf_nosections())
        .unwrap();
    let exported = stripped
        .symbols
        .iter()
        .find(|s| s.name == "exported_func")
        .expect("exported via walker");
    assert_eq!(exported.version.as_deref(), Some("FAKE_1.0"));
    let import = stripped
        .symbols
        .iter()
        .find(|s| s.name == "malloc")
        .expect("import via walker");
    assert_eq!(import.version.as_deref(), Some("GLIBC_2.2.5"));
}

#[test]
fn truncated_and_malformed_inputs_fail_typed() {
    let full = synth::static_exec();
    for cut in [4usize, 16, 40, 64, 100] {
        let mut bytes = full.clone();
        bytes.truncate(cut);
        let result = load_bytes(None, bytes);
        assert!(
            matches!(
                result,
                Err(ElfError::NotAnElf(_)) | Err(ElfError::Malformed(_)) | Err(ElfError::UnsupportedLayout(_))
            ),
            "truncation at {cut} must be a typed failure, got {result:?}"
        );
    }
    // Class mismatch.
    let mut elf32 = full.clone();
    elf32[4] = 1;
    assert!(matches!(
        load_bytes(None, elf32),
        Err(ElfError::UnsupportedLayout(_))
    ));
    // Endianness mismatch.
    let mut be = full.clone();
    be[5] = 2;
    assert!(matches!(
        load_bytes(None, be),
        Err(ElfError::UnsupportedLayout(_))
    ));
}

#[test]
fn load_via_file_path_matches_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.so");
    std::fs::write(&path, synth::shared_object()).unwrap();
    let from_file = damsel_elf::load(&path).expect("file load");
    let from_bytes = load_bytes(None, synth::shared_object()).expect("bytes load");
    assert_eq!(from_file.build_id, from_bytes.build_id);
    assert_eq!(from_file.soname, from_bytes.soname);
    assert_eq!(from_file.sections, from_bytes.sections);
    assert_eq!(from_file.symbols.len(), from_bytes.symbols.len());
}
