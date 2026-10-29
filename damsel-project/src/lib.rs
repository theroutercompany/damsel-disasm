//! `damsel-project` — the analysis project kernel for the RE Power program
//! (Wave 4 of `docs/roadmaps/re-power-roadmap.md`).
//!
//! This crate owns the durable analysis model: projects, sources, object
//! ids, facts with provenance/confidence, and the SQLite-backed project
//! store (Decision D-3: the real embedded store ships from the first
//! implementation PR; WAL mode, versioned migrations, single-writer).
//! Canonical exports — not the database file — are the determinism
//! artifact (two `analyze` runs over identical bytes must produce
//! byte-identical exports).
//!
//! Cross-format readiness rules (Wave 4 spec §3): no type in this crate may
//! reference Mach-O/ELF specifics; platform detail enters only as fact
//! payloads and source kinds.

mod address;
mod facts;
mod functions;
mod object;
mod project;
mod snapshot;
mod source;
mod store;

pub use address::AddressSpaceId;
pub use address::names;
pub use facts::{AnalysisFact, Confidence, FactKind, FactProvenance};
pub use functions::{FunctionSummary, function_summaries};
pub use object::ObjectId;
pub use project::{AnalysisProjectMeta, ProjectId, SCHEMA_VERSION};
pub use snapshot::canonical_export;
pub use source::AnalysisSourceKind;
pub use store::{
    Create, ProjectStore, SourceRecord, StoreError, StoreHandle, TypedStore,
    open_store,
};
