//! Analysis sources: provenance anchors for every fact in a project
//! (Wave 4 spec §2 / analysis-database blueprint "Source").
//!
//! Format-neutral by rule: ELF arrives as `ElfObject` in Wave 5B with no
//! changes to this vocabulary.

use std::fmt;

/// Kinds of sources a project can register. Extensible per wave: `ElfObject`
/// and friends land with Wave 5B, dyld-cache projections with the cache
/// ingest mapping.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnalysisSourceKind {
    /// A standalone Mach-O image (`BinaryImage` ingest).
    MachOImage,
    /// A projected dyld shared-cache image (Wave 2/3 provenance model).
    SharedCacheProjectedImage,
    /// Reserved for Wave 5B; never constructed before that wave lands.
    ElfObject,
    /// A sidecar symbol map or external names source.
    ExternalSymbolMap,
}

impl AnalysisSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MachOImage => "macho_image",
            Self::SharedCacheProjectedImage => "shared_cache_projected_image",
            Self::ElfObject => "elf_object",
            Self::ExternalSymbolMap => "external_symbol_map",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "macho_image" => Some(Self::MachOImage),
            "shared_cache_projected_image" => Some(Self::SharedCacheProjectedImage),
            "elf_object" => Some(Self::ElfObject),
            "external_symbol_map" => Some(Self::ExternalSymbolMap),
            _ => None,
        }
    }
}

impl fmt::Display for AnalysisSourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
