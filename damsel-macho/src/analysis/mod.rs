//! Whole-image analysis passes (RE Power Wave 4): the bridge between the
//! existing `BinaryImage` model and the durable project store.

pub mod ingest;
pub mod recovery;
pub mod strings;
pub mod xrefs;

pub use ingest::{IngestError, IngestOptions, IngestOutcome, ingest_binary_image, ingest_binary_image_with_options};
pub use recovery::{FunctionBound, RecoveryError, RecoveryOutcome, recover_functions};
pub use strings::{StringsError, StringsOutcome, harvest_strings};
pub use xrefs::{XrefError, XrefOutcome, harvest_xrefs};
