//! Address spaces: named address vocabularies per source (blueprint rule:
//! no cross-source xref uses a naked integer).
//!
//! v1 registers the reserved vocabulary; typed `AddressIn<T>` wrappers and
//! conversion failures arrive with the ingest mapping.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AddressSpaceId(String);

impl AddressSpaceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for AddressSpaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Reserved address-space names (Wave 4 spec §3.2).
pub mod names {
    /// Standalone image vmaddr.
    pub const IMAGE_VMADDR: &str = "image_vmaddr";
    /// Dyld cache vmaddr.
    pub const CACHE_VMADDR: &str = "cache_vmaddr";
    /// ELF image vaddr (reserved for Wave 5B).
    pub const ELF_VADDR: &str = "elf_vaddr";
    /// File offset space.
    pub const FILE_OFFSET: &str = "file_offset";
}
