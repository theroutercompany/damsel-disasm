//! Canonical export: the determinism artifact (Decision D-3).
//!
//! Contract (Wave 4 spec §9): schema-versioned, BTree-ordered
//! serialization, no timestamps, no map-iteration order, byte-deterministic
//! for identical inputs regardless of insertion order. Doubles as the CI
//! fixture and diff format. The mutation event log is database-only and is
//! excluded by design.

use crate::facts::AnalysisFact;
use crate::project::{AnalysisProjectMeta, SCHEMA_VERSION};
use crate::store::SourceRecord;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Serialize the canonical export. No trailing newline; callers write
/// bytes verbatim.
pub fn canonical_export(
    meta: &AnalysisProjectMeta,
    sources: &[SourceRecord],
    facts: &[AnalysisFact],
) -> Vec<u8> {
    let mut root = BTreeMap::new();
    root.insert(
        "schema_version".to_string(),
        Value::from(SCHEMA_VERSION),
    );
    root.insert(
        "project".to_string(),
        canonical_project(meta),
    );
    root.insert("sources".to_string(), canonical_sources(sources));
    root.insert("facts".to_string(), canonical_facts(facts));
    serde_json::to_vec(&Value::Object(Map::from_iter(root)))
        .expect("canonical export serialization is infallible for these types")
}

fn canonical_project(meta: &AnalysisProjectMeta) -> Value {
    let mut m = BTreeMap::new();
    m.insert("project_id".to_string(), Value::from(meta.project_id.as_str()));
    m.insert("name".to_string(), Value::from(meta.name.as_str()));
    m.insert(
        "analyzer_version".to_string(),
        Value::from(meta.analyzer_version.as_str()),
    );
    Value::Object(Map::from_iter(m))
}

fn canonical_sources(sources: &[SourceRecord]) -> Value {
    // Deterministic order: source_id. Content is kind + optional fields;
    // labels/hashes only when present (privacy: labels opt-in by callers).
    let mut sorted: Vec<&SourceRecord> = sources.iter().collect();
    sorted.sort_by_key(|s| s.source_id);
    Value::Array(
        sorted
            .iter()
            .map(|s| {
                let mut m = BTreeMap::new();
                m.insert("source_id".to_string(), Value::from(s.source_id));
                m.insert("kind".to_string(), Value::from(s.kind.as_str()));
                if let Some(label) = &s.label {
                    m.insert("label".to_string(), Value::from(label.as_str()));
                }
                if let Some(hash) = &s.content_hash {
                    m.insert("content_hash".to_string(), Value::from(hash.as_str()));
                }
                if let Some(parent) = s.parent_source_id {
                    m.insert("parent_source_id".to_string(), Value::from(parent));
                }
                if !s.address_spaces.is_empty() {
                    m.insert(
                        "address_spaces".to_string(),
                        Value::Array(
                            s.address_spaces
                                .iter()
                                .map(|a| Value::from(a.as_str()))
                                .collect(),
                        ),
                    );
                }
                Value::Object(Map::from_iter(m))
            })
            .collect(),
    )
}

fn canonical_facts(facts: &[AnalysisFact]) -> Value {
    // Canonical order: (source, object string, kind, payload). Project-level
    // facts (no object) sort before objects within a source via empty key.
    let mut sorted: Vec<&AnalysisFact> = facts.iter().collect();
    sorted.sort_by(|a, b| {
        a.source_id
            .cmp(&b.source_id)
            .then_with(|| object_key(a).cmp(&object_key(b)))
            .then_with(|| a.kind.as_str().cmp(b.kind.as_str()))
            .then_with(|| a.payload.cmp(&b.payload))
    });
    Value::Array(
        sorted
            .iter()
            .map(|f| {
                let mut m = BTreeMap::new();
                m.insert("source_id".to_string(), Value::from(f.source_id));
                m.insert(
                    "object".to_string(),
                    f.object_id
                        .as_ref()
                        .map(|o| Value::from(o.as_str()))
                        .unwrap_or(Value::Null),
                );
                m.insert("kind".to_string(), Value::from(f.kind.as_str()));
                m.insert("payload".to_string(), Value::from(f.payload.as_str()));
                m.insert("confidence".to_string(), Value::from(f.confidence.as_str()));
                let mut p = BTreeMap::new();
                p.insert("pass".to_string(), Value::from(f.provenance.pass.as_str()));
                p.insert(
                    "analyzer_version".to_string(),
                    Value::from(f.provenance.analyzer_version.as_str()),
                );
                m.insert("provenance".to_string(), Value::Object(Map::from_iter(p)));
                Value::Object(Map::from_iter(m))
            })
            .collect(),
    )
}

fn object_key(fact: &AnalysisFact) -> String {
    fact.object_id.as_ref().map(|o| o.as_str().to_string()).unwrap_or_default()
}
