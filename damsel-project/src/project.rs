//! Project identity and schema versioning.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Canonical display form of a project id: `<uuid-hex>`.
///
/// Project ids are generated once at creation and stored in the database;
/// they are stable inside the project file and never derived from content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectId(String);

impl ProjectId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Current project store schema version.
///
/// Stored in `PRAGMA user_version`. Every bump requires a migration in
/// [`crate::store::migrations`] and a checked-in N-1 fixture database
/// (evidence matrix row 4.4).
pub const SCHEMA_VERSION: i32 = 1;

/// Project metadata recorded at creation and reported by every surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisProjectMeta {
    pub project_id: ProjectId,
    pub name: String,
    /// Damsel analyzer version that created the project. Derived facts are
    /// marked stale when this differs at open time (blueprint rule).
    pub analyzer_version: String,
}
