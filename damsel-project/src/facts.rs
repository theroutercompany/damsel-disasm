//! Facts: typed, provenance-carrying assertions about project objects
//! (analysis-database blueprint "Fact"; Wave 4 spec §2).
//!
//! Confidence vocabulary matches the blueprint exactly. v1 fact payloads
//! are opaque canonical strings; typed payloads arrive with the ingest
//! mapping passes and carry their own versioned shapes.

use crate::object::ObjectId;
use std::fmt;

/// Confidence values from the analysis-database blueprint. Ordered from
/// strongest to weakest for precedence decisions; `UserAsserted` outranks
/// analyzer facts and `Contradicted` marks modeled conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    UserAsserted,
    Exact,
    Strong,
    Inferred,
    Ambiguous,
    Weak,
    Contradicted,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserAsserted => "user_asserted",
            Self::Exact => "exact",
            Self::Strong => "strong",
            Self::Inferred => "inferred",
            Self::Ambiguous => "ambiguous",
            Self::Weak => "weak",
            Self::Contradicted => "contradicted",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "user_asserted" => Some(Self::UserAsserted),
            "exact" => Some(Self::Exact),
            "strong" => Some(Self::Strong),
            "inferred" => Some(Self::Inferred),
            "ambiguous" => Some(Self::Ambiguous),
            "weak" => Some(Self::Weak),
            "contradicted" => Some(Self::Contradicted),
            _ => None,
        }
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a fact came from. Analyzer facts carry pass + analyzer version;
/// user facts carry actor + origin so the audit log can attribute them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactProvenance {
    /// Analyzer pass name (e.g. `ingest_v1`, `per_function_v1`) or
    /// `user` for analyst-authored facts.
    pub pass: String,
    /// Damsel analyzer version that produced the fact.
    pub analyzer_version: String,
}

impl FactProvenance {
    pub fn analyzer(pass: impl Into<String>, analyzer_version: impl Into<String>) -> Self {
        Self {
            pass: pass.into(),
            analyzer_version: analyzer_version.into(),
        }
    }
}

/// Starter fact-kind vocabulary. Wave 4 ingest adds the full family
/// (`function`, `xref`, `string`, byte classifications); kinds are strings,
/// not a closed enum, so passes can extend without store migrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactKind(String);

impl FactKind {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for FactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// A stored assertion about a project object.
///
/// Ordering (source, object, kind, payload) is the canonical export order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisFact {
    /// Owning source id (row id in the sources table).
    pub source_id: i64,
    /// Object the fact is about; `None` only for project-level facts.
    pub object_id: Option<ObjectId>,
    pub kind: FactKind,
    /// Opaque canonical payload (v1). Deterministic serialization is the
    /// payload author's contract.
    pub payload: String,
    pub confidence: Confidence,
    pub provenance: FactProvenance,
}
