//! Object ids: the shared identity vocabulary for CLI, UI, scripting, and
//! future MCP surfaces (blueprint rule: object ids are opaque to clients,
//! deterministic where practical, and resolve to routes).
//!
//! v1 representation is the canonical string `<kind>:<key>`; the store maps
//! it to a stable integer row id per project.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId {
    kind: String,
    key: String,
    value: String,
}

impl ObjectId {
    /// Construct an object id from a kind and a deterministic key.
    ///
    /// Kind and key must be non-empty and must not contain `:`.
    pub fn new(kind: impl Into<String>, key: impl Into<String>) -> Self {
        let kind = kind.into();
        let key = key.into();
        debug_assert!(!kind.is_empty() && !key.is_empty(), "object id parts must be non-empty");
        debug_assert!(
            !kind.contains(':') && !key.contains(':'),
            "object id parts must not contain ':'"
        );
        let value = format!("{kind}:{key}");
        Self { kind, key, value }
    }

    pub fn kind(&self) -> &str {
        self.kind.as_str()
    }

    pub fn key(&self) -> &str {
        self.key.as_str()
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    /// Parse the canonical `<kind>:<key>` form. The key may itself contain
    /// `:`; the first separator splits.
    pub fn parse(value: &str) -> Option<Self> {
        let (kind, key) = value.split_once(':')?;
        if kind.is_empty() || key.is_empty() {
            return None;
        }
        Some(Self {
            kind: kind.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        })
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value.as_str())
    }
}
