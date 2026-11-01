//! User annotations: renames, comments, and tags persisted as
//! `UserAsserted` facts with a full audit trail, plus undo.
//!
//! Fact vocabulary (`user_v1`):
//! - kind `annotation` on the target object, confidence `user_asserted`
//! - payload `rename|to=<name>` | `comment|text=<text>` | `tag|name=<tag>`
//! - provenance pass `user_v1`; the provenance version carries the actor
//!
//! Every annotation also writes an `annotation_added` event (actor,
//! origin, payload). Undo pairs events: the newest `annotation_added`
//! whose seq is not already referenced by an `annotation_undone` event is
//! retired — its fact is deleted and an `annotation_undone` event records
//! the original seq. Annotations are facts, so they flow into canonical
//! exports — unlike target descriptors they are analysis state.
//!
//! Renames replace: a second rename on the same object retires the first
//! so `function_summaries` stays deterministic without event ordering.

use crate::facts::AnalysisFact;
use crate::object::ObjectId;
use crate::store::sqlite::SqliteStore;
use crate::store::{ProjectStore, StoreError, TypedStore};
use crate::{Confidence, FactKind, FactProvenance};
use std::fmt;

pub const ANNOTATION_PASS: &str = "user_v1";
pub const ANNOTATION_FACT_KIND: &str = "annotation";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationKind {
    Rename,
    Comment,
    Tag,
}

impl AnnotationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rename => "rename",
            Self::Comment => "comment",
            Self::Tag => "tag",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "rename" => Some(Self::Rename),
            "comment" => Some(Self::Comment),
            "tag" => Some(Self::Tag),
            _ => None,
        }
    }
}

impl fmt::Display for AnnotationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A stored annotation as returned by [`list_annotations`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationRecord {
    pub source_id: i64,
    pub object_id: ObjectId,
    pub kind: AnnotationKind,
    pub value: String,
    pub actor: String,
}

/// Result of storing an annotation; `event_seq` is the audit-log row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredAnnotation {
    pub source_id: i64,
    pub event_seq: i64,
}

pub(crate) fn payload_for(kind: AnnotationKind, value: &str) -> String {
    match kind {
        AnnotationKind::Rename => format!("rename|to={value}"),
        AnnotationKind::Comment => format!("comment|text={value}"),
        AnnotationKind::Tag => format!("tag|name={value}"),
    }
}

/// Parse an annotation payload into `(kind, value)`.
pub(crate) fn parse_payload(payload: &str) -> Option<(AnnotationKind, &str)> {
    let (kind, rest) = payload.split_once('|')?;
    let kind = AnnotationKind::from_str(kind)?;
    let (prefix, value) = rest.split_once('=')?;
    if !matches!(prefix, "to" | "text" | "name") {
        return None;
    }
    Some((kind, value))
}

/// True when the fact is a user annotation from this pass.
pub(crate) fn is_user_annotation(fact: &AnalysisFact) -> bool {
    fact.kind.as_str() == ANNOTATION_FACT_KIND
        && fact.confidence == Confidence::UserAsserted
        && fact.provenance.pass == ANNOTATION_PASS
}

/// Store one annotation. Renames are replace-semantics: an existing rename
/// on the same object is retired first.
pub fn put_annotation(
    store: &mut SqliteStore,
    object_id: &ObjectId,
    kind: AnnotationKind,
    value: &str,
    actor: &str,
) -> Result<StoredAnnotation, StoreError> {
    let source_id = store
        .source_for_object(object_id)?
        .or_else(|| {
            store
                .sources()
                .ok()
                .and_then(|s| s.first().map(|r| r.source_id))
        })
        .ok_or_else(|| StoreError::ProjectNotFound("project has no sources".to_string()))?;
    if matches!(kind, AnnotationKind::Rename) {
        store.delete_annotation_facts(source_id, object_id, AnnotationKind::Rename)?;
    }
    store.put_fact(annotation_fact(
        source_id,
        object_id.clone(),
        kind,
        value,
        actor,
    ))?;
    let event_seq = store.log_annotation_event(
        actor,
        "annotation_added",
        &serde_json::json!({
            "source_id": source_id,
            "object": object_id.as_str(),
            "kind": kind.as_str(),
            "value": value,
        })
        .to_string(),
    )?;
    if matches!(kind, AnnotationKind::Rename) {
        // Replace-semantics must retire the superseded rename's audit
        // event too, or undo would pop a fact that no longer exists.
        store.retire_rename_events_for(object_id, event_seq)?;
    }
    Ok(StoredAnnotation {
        source_id,
        event_seq,
    })
}

/// The newest not-yet-undone annotation for `actor`, with its event seq.
pub fn last_annotation_for(
    store: &SqliteStore,
    actor: &str,
) -> Result<Option<(i64, AnnotationRecord)>, StoreError> {
    let Some((seq, payload)) = store.latest_active_annotation_event(actor)? else {
        return Ok(None);
    };
    let value: serde_json::Value = serde_json::from_str(&payload).map_err(|error| {
        StoreError::ProjectNotFound(format!("corrupt annotation event: {error}"))
    })?;
    let object = value
        .get("object")
        .and_then(|v| v.as_str())
        .and_then(ObjectId::parse)
        .ok_or_else(|| StoreError::ProjectNotFound("corrupt annotation object".to_string()))?;
    let kind = value
        .get("kind")
        .and_then(|v| v.as_str())
        .and_then(AnnotationKind::from_str)
        .ok_or_else(|| StoreError::ProjectNotFound("corrupt annotation kind".to_string()))?;
    let text = value
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let source_id = value
        .get("source_id")
        .and_then(|v| v.as_i64())
        .unwrap_or_default();
    Ok(Some((
        seq,
        AnnotationRecord {
            source_id,
            object_id: object,
            kind,
            value: text,
            actor: actor.to_string(),
        },
    )))
}

/// Undo the newest annotation for `actor`: remove its fact and record
/// `annotation_undone` referencing the original event seq. Returns the
/// removed record.
pub fn undo_last_annotation(
    store: &mut SqliteStore,
    actor: &str,
) -> Result<Option<AnnotationRecord>, StoreError> {
    let Some((seq, record)) = last_annotation_for(store, actor)? else {
        return Ok(None);
    };
    store.delete_annotation_fact_exact(
        record.source_id,
        &record.object_id,
        record.kind,
        &record.value,
    )?;
    store.log_annotation_event(
        actor,
        "annotation_undone",
        &serde_json::json!({
            "added_seq": seq,
            "object": record.object_id.as_str(),
            "kind": record.kind.as_str(),
            "value": record.value,
        })
        .to_string(),
    )?;
    Ok(Some(record))
}

/// All annotations in canonical order (by object, kind, value).
pub fn list_annotations(store: &impl ProjectStore) -> Result<Vec<AnnotationRecord>, StoreError> {
    let mut out = Vec::new();
    for fact in store.facts()? {
        if !is_user_annotation(&fact) {
            continue;
        }
        let Some(object_id) = &fact.object_id else {
            continue;
        };
        let Some((kind, value)) = parse_payload(&fact.payload) else {
            continue;
        };
        out.push(AnnotationRecord {
            source_id: fact.source_id,
            object_id: object_id.clone(),
            kind,
            value: value.to_string(),
            actor: fact.provenance.analyzer_version.clone(),
        });
    }
    out.sort_by(|left, right| {
        (
            left.object_id.as_str(),
            left.kind.as_str(),
            left.value.as_str(),
        )
            .cmp(&(
                right.object_id.as_str(),
                right.kind.as_str(),
                right.value.as_str(),
            ))
    });
    Ok(out)
}

fn annotation_fact(
    source_id: i64,
    object_id: ObjectId,
    kind: AnnotationKind,
    value: &str,
    actor: &str,
) -> AnalysisFact {
    AnalysisFact {
        source_id,
        object_id: Some(object_id),
        kind: FactKind::new(ANNOTATION_FACT_KIND),
        payload: payload_for(kind, value),
        confidence: Confidence::UserAsserted,
        provenance: FactProvenance::analyzer(ANNOTATION_PASS, actor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_payload_round_trips() {
        for (kind, value) in [
            (AnnotationKind::Rename, "my_func"),
            (AnnotationKind::Comment, "checks the license"),
            (AnnotationKind::Tag, "crypto"),
        ] {
            let payload = payload_for(kind, value);
            assert_eq!(parse_payload(&payload), Some((kind, value)));
        }
        assert_eq!(parse_payload("seed|symbol_table"), None);
        assert_eq!(parse_payload("rename|wrong=shape"), None);
    }
}
