//! Anti-analysis detection catalog (Wave 8.4).
//!
//! Named detections with severity, confidence, evidence, and
//! remediation hints. Consumed by `recon` (triage) and `diff`
//! (version archaeology: `defense_added` findings).

pub mod catalog;

pub use catalog::{
    CatalogEntry, Confidence, Detection, InstructionWindow, Severity, catalog, lookup,
    render_markdown, scan_imports, scan_instructions, scan_strings,
};
