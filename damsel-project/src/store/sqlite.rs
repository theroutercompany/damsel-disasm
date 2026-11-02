//! The shipped project store: SQLite via rusqlite (bundled), WAL mode,
//! `foreign_keys = ON`, single-writer discipline.
//!
//! Concurrency posture (sqlite.org/wal.html, line-cited in Research
//! Sources §0): WAL readers do not block the writer and the writer does
//! not block readers; there can only be one writer at a time. v1 enforces
//! the single writer at the type level: all mutations take
//! `&mut SqliteStore`. A reader pool arrives with the scheduler work.

use super::migrations;
use super::{Create, ProjectStore, SourceRecord, StoreError, TypedStore};
use crate::facts::AnalysisFact;
use crate::object::ObjectId;
use crate::project::AnalysisProjectMeta;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub struct SqliteStore {
    conn: Connection,
}

/// Open (or create) the store at `path`.
pub fn open(
    path: &Path,
    create: Create,
    meta: &AnalysisProjectMeta,
) -> Result<SqliteStore, StoreError> {
    match create {
        Create::New => {
            if path.exists() {
                return Err(StoreError::AlreadyExists(path.to_path_buf()));
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Create::Existing => {
            if !path.exists() {
                return Err(StoreError::ProjectNotFound(path.display().to_string()));
            }
        }
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::migrate(&conn, path)?;

    match create {
        Create::New => {
            let insert = |key: &str, value: &str| -> Result<(), StoreError> {
                conn.execute(
                    "INSERT INTO project_meta (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, value],
                )?;
                Ok(())
            };
            insert("project_id", meta.project_id.as_str())?;
            insert("name", &meta.name)?;
            insert("analyzer_version", &meta.analyzer_version)?;
        }
        Create::Existing => {
            // Analyzer-version drift marks derived facts stale at the
            // analysis layer, not here; the store only records it.
            let stored: Option<String> = conn
                .query_row(
                    "SELECT value FROM project_meta WHERE key = 'analyzer_version'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if stored.as_deref() != Some(meta.analyzer_version.as_str()) {
                conn.execute(
                    "INSERT INTO project_meta (key, value) VALUES ('analyzer_version_observed', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    rusqlite::params![meta.analyzer_version],
                )?;
            }
        }
    }

    Ok(SqliteStore { conn })
}

impl SqliteStore {
    /// WAL mode + foreign keys + schema version assertions (test surface).
    pub fn pragmas(&self) -> Result<(String, bool, i32), StoreError> {
        let mode: String = self
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        let fk: bool = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
        let version: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        Ok((mode, fk, version))
    }

    /// Begin one write transaction. Analysis passes batch their fact
    /// writes so a large image commits with a single fsync instead of one
    /// per fact. Pair with [`Self::commit_batch`] or
    /// [`Self::rollback_batch`]; single-writer discipline means the caller
    /// already owns exclusivity.
    pub fn begin_batch(&mut self) -> Result<(), StoreError> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        Ok(())
    }

    /// Commit the batch opened by [`Self::begin_batch`].
    pub fn commit_batch(&mut self) -> Result<(), StoreError> {
        self.conn.execute_batch("COMMIT")?;
        Ok(())
    }

    /// Roll back the batch opened by [`Self::begin_batch`].
    pub fn rollback_batch(&mut self) -> Result<(), StoreError> {
        self.conn.execute_batch("ROLLBACK")?;
        Ok(())
    }

    /// Source that owns an interned object, if any.
    pub fn source_for_object(&self, object_id: &ObjectId) -> Result<Option<i64>, StoreError> {
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT source_id FROM objects WHERE kind = ?1 AND key = ?2 LIMIT 1",
                rusqlite::params![object_id.kind(), object_id.key()],
                |r| r.get(0),
            )
            .optional()?;
        Ok(existing)
    }

    /// Delete all annotation facts of one kind for an object (rename
    /// replace-semantics). Only ever touches `user_asserted` facts.
    pub(crate) fn delete_annotation_facts(
        &mut self,
        source_id: i64,
        object_id: &ObjectId,
        kind: crate::annotations::AnnotationKind,
    ) -> Result<(), StoreError> {
        let prefix = format!("{}|", kind.as_str());
        self.conn.execute(
            "DELETE FROM facts
             WHERE source_id = ?1
               AND object_id = (SELECT object_id FROM objects
                                WHERE source_id = ?1 AND kind = ?2 AND key = ?3)
               AND kind = ?4
               AND confidence = 'user_asserted'
               AND payload LIKE ?5 || '%'",
            rusqlite::params![
                source_id,
                object_id.kind(),
                object_id.key(),
                crate::annotations::ANNOTATION_FACT_KIND,
                prefix
            ],
        )?;
        Ok(())
    }

    /// Delete the exact annotation fact for `(object, kind, value)`.
    pub(crate) fn delete_annotation_fact_exact(
        &mut self,
        source_id: i64,
        object_id: &ObjectId,
        kind: crate::annotations::AnnotationKind,
        value: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM facts
             WHERE source_id = ?1
               AND object_id = (SELECT object_id FROM objects
                                WHERE source_id = ?1 AND kind = ?2 AND key = ?3)
               AND kind = ?4
               AND confidence = 'user_asserted'
               AND payload = ?5",
            rusqlite::params![
                source_id,
                object_id.kind(),
                object_id.key(),
                crate::annotations::ANNOTATION_FACT_KIND,
                crate::annotations::payload_for(kind, value),
            ],
        )?;
        Ok(())
    }

    /// Append an annotation audit event; returns its seq.
    pub(crate) fn log_annotation_event(
        &self,
        actor: &str,
        kind: &str,
        payload: &str,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO events (actor, origin, kind, payload) VALUES (?1, 'annotation', ?2, ?3)",
            rusqlite::params![actor, kind, payload],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Newest `annotation_added` event for `actor` whose seq is not
    /// already retired by an `annotation_undone` event (retire rows can be
    /// written by any actor — replace-semantics — so undones are global).
    pub(crate) fn latest_active_annotation_event(
        &self,
        actor: &str,
    ) -> Result<Option<(i64, String)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, kind, payload FROM events
             WHERE (kind = 'annotation_added' AND actor = ?1)
                OR kind = 'annotation_undone'
             ORDER BY seq",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![actor], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut added: Vec<(i64, String)> = Vec::new();
        let mut undone = std::collections::BTreeSet::new();
        for (seq, kind, payload) in rows {
            if kind == "annotation_added" {
                added.push((seq, payload));
            } else if let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) {
                if let Some(seq) = value.get("added_seq").and_then(|v| v.as_i64()) {
                    undone.insert(seq);
                }
            }
        }
        Ok(added
            .into_iter()
            .rev()
            .find(|(seq, _)| !undone.contains(seq)))
    }

    /// Retire every active `rename` annotation event for an object except
    /// `except_seq` (replace-semantics on the audit log).
    pub(crate) fn retire_rename_events_for(
        &self,
        object_id: &ObjectId,
        except_seq: i64,
    ) -> Result<(), StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, kind, payload FROM events
             WHERE kind IN ('annotation_added', 'annotation_undone')
             ORDER BY seq",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut active: Vec<(i64, String)> = Vec::new();
        let mut undone = std::collections::BTreeSet::new();
        for (seq, kind, payload) in rows {
            if kind == "annotation_added" {
                active.push((seq, payload));
            } else if let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) {
                if let Some(seq) = value.get("added_seq").and_then(|v| v.as_i64()) {
                    undone.insert(seq);
                }
            }
        }
        for (seq, payload) in active {
            if seq == except_seq || undone.contains(&seq) {
                continue;
            }
            let matches = serde_json::from_str::<serde_json::Value>(&payload)
                .ok()
                .and_then(|value| {
                    let object = value.get("object").and_then(|v| v.as_str());
                    let kind = value.get("kind").and_then(|v| v.as_str());
                    Some(object == Some(object_id.as_str()) && kind == Some("rename"))
                })
                .unwrap_or(false);
            if !matches {
                continue;
            }
            self.conn.execute(
                "INSERT INTO events (actor, origin, kind, payload)
                 VALUES ('damsel', 'annotation', 'annotation_undone', ?1)",
                rusqlite::params![serde_json::json!({ "added_seq": seq }).to_string()],
            )?;
        }
        Ok(())
    }

    fn intern_object(&mut self, source_id: i64, object_id: &ObjectId) -> Result<i64, StoreError> {
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT object_id FROM objects WHERE source_id = ?1 AND kind = ?2 AND key = ?3",
                rusqlite::params![source_id, object_id.kind(), object_id.key()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        self.conn.execute(
            "INSERT INTO objects (source_id, kind, key) VALUES (?1, ?2, ?3)",
            rusqlite::params![source_id, object_id.kind(), object_id.key()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn record_event(&self, kind: &str, payload: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO events (actor, origin, kind, payload) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["damsel", "api", kind, payload],
        )?;
        Ok(())
    }
}

impl ProjectStore for SqliteStore {
    fn meta(&self) -> Result<AnalysisProjectMeta, StoreError> {
        let rows: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare("SELECT key, value FROM project_meta")?;
            let it = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            let mut out = Vec::new();
            for row in it {
                out.push(row?);
            }
            out
        };
        let get = |key: &str| -> Result<String, StoreError> {
            rows.iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| StoreError::ProjectNotFound(key.to_string()))
        };
        Ok(AnalysisProjectMeta {
            project_id: crate::ProjectId::new(get("project_id")?),
            name: get("name")?,
            analyzer_version: get("analyzer_version")?,
        })
    }

    fn sources(&self) -> Result<Vec<SourceRecord>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id, kind, label, content_hash, parent_source_id
             FROM sources ORDER BY source_id",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for (id, kind, label, hash, parent) in rows {
            let kind =
                crate::AnalysisSourceKind::from_str(&kind).ok_or(StoreError::SourceNotFound(id))?;
            let mut spaces_stmt = self.conn.prepare(
                "SELECT space_name FROM source_address_spaces
                 WHERE source_id = ?1 ORDER BY ordinal",
            )?;
            let spaces = spaces_stmt
                .query_map(rusqlite::params![id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            out.push(SourceRecord {
                source_id: id,
                kind,
                label,
                content_hash: hash,
                parent_source_id: parent,
                address_spaces: spaces.into_iter().map(crate::AddressSpaceId::new).collect(),
            });
        }
        Ok(out)
    }

    fn facts(&self) -> Result<Vec<AnalysisFact>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT f.source_id, o.kind, o.key, f.kind, f.payload, f.confidence,
                    f.provenance_pass, f.analyzer_version
             FROM facts f LEFT JOIN objects o ON o.object_id = f.object_id
             ORDER BY f.source_id, o.kind, o.key, f.kind, f.payload",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for (source, okind, okey, kind, payload, confidence, pass, version) in rows {
            let object_id = match (okind, okey) {
                (Some(k), Some(key)) => Some(
                    ObjectId::parse(&format!("{k}:{key}")).unwrap_or_else(|| ObjectId::new(k, key)),
                ),
                _ => None,
            };
            out.push(AnalysisFact {
                source_id: source,
                object_id,
                kind: crate::FactKind::new(kind),
                payload,
                confidence: crate::Confidence::from_str(&confidence)
                    .unwrap_or(crate::Confidence::Inferred),
                provenance: crate::FactProvenance::analyzer(pass, version),
            });
        }
        Ok(out)
    }

    fn export_canonical(&self) -> Result<Vec<u8>, StoreError> {
        Ok(crate::snapshot::canonical_export(
            &self.meta()?,
            &self.sources()?,
            &self.facts()?,
        ))
    }
}

impl TypedStore for SqliteStore {
    fn add_source(&mut self, record: SourceRecord) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO sources (kind, label, content_hash, parent_source_id)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                record.kind.as_str(),
                record.label,
                record.content_hash,
                record.parent_source_id
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        for (ordinal, space) in record.address_spaces.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO source_address_spaces (source_id, space_name, ordinal)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![id, space.as_str(), ordinal as i64],
            )?;
        }
        self.record_event("source_added", record.kind.as_str())?;
        Ok(id)
    }

    fn put_fact(&mut self, fact: AnalysisFact) -> Result<(), StoreError> {
        // Source must exist (FK enforced, but fail with our typed error).
        let exists: bool = self
            .conn
            .query_row(
                "SELECT COUNT(1) FROM sources WHERE source_id = ?1",
                rusqlite::params![fact.source_id],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n > 0)?;
        if !exists {
            return Err(StoreError::SourceNotFound(fact.source_id));
        }
        let object_row = match &fact.object_id {
            Some(oid) => Some(self.intern_object(fact.source_id, oid)?),
            None => None,
        };
        self.conn.execute(
            "INSERT INTO facts (source_id, object_id, kind, payload, confidence,
                                provenance_pass, analyzer_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (source_id, object_id, kind, payload) DO NOTHING",
            rusqlite::params![
                fact.source_id,
                object_row,
                fact.kind.as_str(),
                fact.payload,
                fact.confidence.as_str(),
                fact.provenance.pass,
                fact.provenance.analyzer_version
            ],
        )?;
        Ok(())
    }

    fn facts_for_source(&self, source_id: i64) -> Result<Vec<AnalysisFact>, StoreError> {
        Ok(self
            .facts()?
            .into_iter()
            .filter(|f| f.source_id == source_id)
            .collect())
    }
}
