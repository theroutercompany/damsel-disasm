//! Project store: open/create, schema migration, single-writer mutation
//! surface, and read queries (Decision D-3).

pub mod migrations;
pub mod sqlite;

use crate::facts::AnalysisFact;
use crate::project::AnalysisProjectMeta;
use crate::source::AnalysisSourceKind;
use crate::AddressSpaceId;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("store io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("store io error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("project store at {path} has schema version {found}; this damsel supports {supported}; open with an explicit upgrade path")]
    UnsupportedSchema {
        path: PathBuf,
        found: i32,
        supported: i32,
    },
    #[error("source id {0} not found")]
    SourceNotFound(i64),
    #[error("project {0} not found")]
    ProjectNotFound(String),
    #[error("store already exists: {0}")]
    AlreadyExists(PathBuf),
}

/// A registered source row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord {
    pub source_id: i64,
    pub kind: AnalysisSourceKind,
    /// Display label (file name or provided label). Paths are privacy-
    /// controlled: never required, never exported without opt-in.
    pub label: Option<String>,
    /// Content hash when available; used by diffing and staleness checks.
    pub content_hash: Option<String>,
    pub parent_source_id: Option<i64>,
    /// Address spaces owned by this source, in registration order.
    pub address_spaces: Vec<AddressSpaceId>,
}

/// Contract shared by every backend. The shipped backend is SQLite
/// ([`sqlite::SqliteStore`]); the trait exists for tests and future
/// in-process fakes, not as a second storage stage (D-3 supersession).
pub trait ProjectStore {
    fn meta(&self) -> Result<AnalysisProjectMeta, StoreError>;
    fn sources(&self) -> Result<Vec<SourceRecord>, StoreError>;
    fn facts(&self) -> Result<Vec<AnalysisFact>, StoreError>;

    /// Canonical, byte-deterministic export (the determinism artifact).
    fn export_canonical(&self) -> Result<Vec<u8>, StoreError>;
}

/// Marker for the concrete handle type used by callers.
pub use sqlite::SqliteStore as StoreHandle;

/// Open (or create) a project store at `path`.
///
/// Creation is explicit: pass `Create::New` for a new project; opening an
/// existing file with `Create::New` fails with
/// [`StoreError::AlreadyExists`] (no silent overwrites), and opening a
/// missing file with `Create::Existing` fails cleanly.
pub fn open_store(path: &Path, create: Create, meta: &AnalysisProjectMeta) -> Result<StoreHandle, StoreError> {
    sqlite::open(path, create, meta)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Create {
    New,
    Existing,
}

/// Typed mutation surface (single-writer discipline: mutations take the
/// handle exclusively at the type level).
pub trait TypedStore {
    /// Register a source; returns its stable row id.
    fn add_source(&mut self, record: SourceRecord) -> Result<i64, StoreError>;
    /// Upsert a fact (idempotent on the canonical tuple).
    fn put_fact(&mut self, fact: AnalysisFact) -> Result<(), StoreError>;
    /// All facts for one source in canonical order.
    fn facts_for_source(&self, source_id: i64) -> Result<Vec<AnalysisFact>, StoreError>;
}
