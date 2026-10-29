//! Store contract tests (evidence matrix rows 4.1, 4.3, 4.4-partial).
//!
//! - create/reopen round trip
//! - WAL + foreign_keys + user_version assertions
//! - canonical export byte-determinism across two independently built
//!   databases, including shuffled insertion order
//! - schema-version refusal for future databases
//! - idempotent fact upserts and canonical ordering

use damsel_project::{
    AddressSpaceId, AnalysisFact, AnalysisProjectMeta, AnalysisSourceKind, Confidence,
    FactKind, FactProvenance, ObjectId, ProjectId, ProjectStore, TypedStore, open_store,
    Create, SCHEMA_VERSION,
};

fn meta() -> AnalysisProjectMeta {
    AnalysisProjectMeta {
        project_id: ProjectId::new("01a183ce-ed53-743d-bec2-6451f8ed4bc8"),
        name: "contract-fixture".to_string(),
        analyzer_version: "0.1.0".to_string(),
    }
}

fn sample_source(kind: AnalysisSourceKind) -> damsel_project::SourceRecord {
    damsel_project::SourceRecord {
        source_id: 0,
        kind,
        label: Some("fixture-binary".to_string()),
        content_hash: Some("sha256:deadbeef".to_string()),
        parent_source_id: None,
        address_spaces: vec![
            AddressSpaceId::new(damsel_project::names::IMAGE_VMADDR),
        ],
    }
}

fn sample_facts(source_id: i64) -> Vec<AnalysisFact> {
    let f = |object: Option<&str>, kind: &str, payload: &str, confidence: Confidence| AnalysisFact {
        source_id,
        object_id: object.map(|o| ObjectId::parse(o).unwrap()),
        kind: FactKind::new(kind),
        payload: payload.to_string(),
        confidence,
        provenance: FactProvenance::analyzer("ingest_v1", "0.1.0"),
    };
    vec![
        f(Some("section:__TEXT"), "byte_class", "code", Confidence::Exact),
        f(Some("function:0x100005a40"), "seed", "symbol_table", Confidence::Exact),
        f(Some("string:0x100008000"), "class", "literal", Confidence::Inferred),
        f(None, "project_note", "ingest_complete", Confidence::Exact),
    ]
}

#[test]
fn create_reopen_and_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("p.damsel").join("project.db");

    let source_id = {
        let mut store = open_store(&db, Create::New, &meta()).unwrap();
        let (mode, fk, version) = store.pragmas().unwrap();
        assert_eq!(mode, "wal");
        assert!(fk, "foreign_keys must be ON");
        assert_eq!(version, SCHEMA_VERSION);
        store.add_source(sample_source(AnalysisSourceKind::MachOImage)).unwrap()
    };

    {
        let store = open_store(&db, Create::Existing, &meta()).unwrap();
        let sources = store.sources().unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_id, source_id);
        assert_eq!(sources[0].kind, AnalysisSourceKind::MachOImage);
        assert_eq!(
            sources[0].address_spaces[0].as_str(),
            damsel_project::names::IMAGE_VMADDR
        );
    }
}

#[test]
fn facts_round_trip_and_canonical_order() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("p.damsel").join("project.db");
    let source_id = {
        let mut store = open_store(&db, Create::New, &meta()).unwrap();
        let id = store.add_source(sample_source(AnalysisSourceKind::MachOImage)).unwrap();
        for fact in sample_facts(id) {
            store.put_fact(fact).unwrap();
        }
        id
    };
    let store = open_store(&db, Create::Existing, &meta()).unwrap();
    let facts = store.facts_for_source(source_id).unwrap();
    assert_eq!(facts.len(), 4);
    // Canonical order: project-level facts (no object) sort first within
    // the source, then by object string, kind, payload.
    assert_eq!(facts[0].object_id, None);
    assert!(facts[0].kind.as_str() == "project_note");
    assert_eq!(facts[1].object_id.as_ref().unwrap().as_str(), "function:0x100005a40");
    assert_eq!(facts[3].object_id.as_ref().unwrap().as_str(), "string:0x100008000");

    // Idempotent upsert: same tuple again is a no-op.
    let mut store = open_store(&db, Create::Existing, &meta()).unwrap();
    store
        .put_fact(sample_facts(source_id).into_iter().next().unwrap())
        .unwrap();
    assert_eq!(store.facts_for_source(source_id).unwrap().len(), 4);
}

#[test]
fn canonical_export_is_byte_deterministic_regardless_of_insertion_order() {
    let build = |shuffle: bool| -> Vec<u8> {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("p.damsel").join("project.db");
        let mut store = open_store(&db, Create::New, &meta()).unwrap();
        let id = store.add_source(sample_source(AnalysisSourceKind::MachOImage)).unwrap();
        let mut facts = sample_facts(id);
        if shuffle {
            facts.reverse();
        }
        for fact in facts {
            store.put_fact(fact).unwrap();
        }
        store.export_canonical().unwrap()
    };
    let a = build(false);
    let b = build(true);
    assert_eq!(a, b, "canonical exports must be byte-identical");
    assert!(!a.is_empty());
}

#[test]
fn refuses_to_create_over_existing_and_to_open_missing() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("p.damsel").join("project.db");
    let mut store = open_store(&db, Create::New, &meta()).unwrap();
    store.add_source(sample_source(AnalysisSourceKind::MachOImage)).unwrap();
    drop(store);

    assert!(matches!(
        open_store(&db, Create::New, &meta()),
        Err(damsel_project::StoreError::AlreadyExists(_))
    ));
    assert!(matches!(
        open_store(&dir.path().join("missing.db"), Create::Existing, &meta()),
        Err(damsel_project::StoreError::ProjectNotFound(_))
    ));
}

#[test]
fn put_fact_rejects_unknown_source() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("p.damsel").join("project.db");
    let mut store = open_store(&db, Create::New, &meta()).unwrap();
    let fact = AnalysisFact {
        source_id: 999,
        object_id: None,
        kind: FactKind::new("project_note"),
        payload: "orphan".to_string(),
        confidence: Confidence::Exact,
        provenance: FactProvenance::analyzer("ingest_v1", "0.1.0"),
    };
    assert!(matches!(
        store.put_fact(fact),
        Err(damsel_project::StoreError::SourceNotFound(999))
    ));
}

#[test]
fn future_schema_version_is_refused_with_typed_error() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("p.damsel").join("project.db");
    {
        let mut store = open_store(&db, Create::New, &meta()).unwrap();
        store.add_source(sample_source(AnalysisSourceKind::MachOImage)).unwrap();
    }
    // Simulate a future damsel having bumped the schema.
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1).unwrap();
    }
    assert!(matches!(
        open_store(&db, Create::Existing, &meta()),
        Err(damsel_project::StoreError::UnsupportedSchema { found, .. }) if found == SCHEMA_VERSION + 1
    ));
}
