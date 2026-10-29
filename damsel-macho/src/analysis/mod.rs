//! Whole-image analysis passes (RE Power Wave 4): the bridge between the
//! existing `BinaryImage` model and the durable project store.

pub mod ingest;

pub use ingest::{IngestError, IngestOutcome, ingest_binary_image};
