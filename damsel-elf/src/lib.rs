//! `damsel-elf` — ELF64 loading and analysis for the RE Power program
//! (Wave 5B of `docs/roadmaps/re-power-roadmap.md`).
//!
//! Scope v1: userspace ELF64 (little-endian first), ET_EXEC and ET_DYN,
//! via the `object` crate as parse backend. Owns identity (build-id,
//! SONAME), segments/sections/symbols/imports. The segment-only
//! `PT_DYNAMIC` walker for section-header-stripped binaries (Research
//! Sources §0 Round 3: both `object` and `goblin` punt here) lands with
//! the `elf-nosections` fixture unit.

mod analysis;
mod image;
mod ingest;
mod loader;
mod strings;
mod walker;

pub use image::{ElfDynamicEntry, ElfImage, ElfSection, ElfSegment, ElfSymbol};
pub use analysis::{ElfAnalysisError, ElfXrefOutcome, harvest_elf_xrefs};
pub use ingest::{ElfIngestError, ElfIngestOutcome, ingest_elf_image};
pub use strings::{ElfStringsError, ElfStringsOutcome, harvest_elf_strings, read_elf_c_string};
pub use loader::{ElfError, Result, load, load_bytes};

/// Synthetic ELF fixture builders (deterministic in-process bytes).
///
/// `#[doc(hidden)]` test-support surface shared with the CLI contract
/// tests until the checked-in fixture lane (Wave 5C) replaces it.
#[doc(hidden)]
pub mod synth {
    include!("synth_support.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elf_load_rejects_garbage_with_typed_error() {
        let error = load_bytes(Some("garbage".to_string()), vec![0u8; 64])
            .expect_err("garbage must be rejected");
        assert!(
            matches!(error, ElfError::NotAnElf(_)),
            "expected NotAnElf, got {error:?}"
        );
    }
}
