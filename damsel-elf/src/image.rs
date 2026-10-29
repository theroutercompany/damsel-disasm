//! ELF image model (Wave 5B): format-neutral vocabulary mirroring
//! `BinaryImage`'s role. No Mach-O types cross this boundary.

use damsel_core::Architecture;

/// A loaded ELF64 image. Owns its bytes (mirroring `BinaryImage`) so
/// analysis passes can decode windows and read strings.
#[derive(Clone)]
pub struct ElfImage {
    pub(crate) label: Option<String>,
    pub(crate) data: std::sync::Arc<[u8]>,
    /// PT_LOAD mapping for vaddr→file-offset reads.
    pub(crate) loads: Vec<(u64, u64, u64)>, // (vaddr, filesz, offset)
    /// ELF class/machine identity summary.
    pub architecture: Architecture,
    /// `ET_EXEC` (static-position) or `ET_DYN` (PIE / shared object).
    pub is_position_independent: bool,
    /// `.note.gnu.build-id` hex string when present (diffing identity).
    pub build_id: Option<String>,
    /// SONAME (`DT_SONAME`) when present.
    pub soname: Option<String>,
    pub segments: Vec<ElfSegment>,
    pub sections: Vec<ElfSection>,
    /// dynsym + symtab symbols (dynsym first).
    pub symbols: Vec<ElfSymbol>,
    /// Dynamic entries of interest (needed libraries, rpath, soname).
    pub dynamic: Vec<ElfDynamicEntry>,
}

impl ElfImage {
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Raw file bytes (analysis use: decode windows, strings).
    pub fn slice_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Translate a virtual address to a file offset via PT_LOAD.
    pub fn vaddr_to_offset(&self, vaddr: u64) -> Option<u64> {
        self.loads
            .iter()
            .find(|(base, size, _)| vaddr >= *base && vaddr < base + size)
            .map(|(base, _, offset)| offset + (vaddr - base))
    }

    /// Read `len` bytes at a virtual address.
    pub fn read_at_vaddr(&self, vaddr: u64, len: usize) -> Option<&[u8]> {
        let offset = self.vaddr_to_offset(vaddr)? as usize;
        self.data.get(offset..offset + len)
    }

    pub fn executable_sections(&self) -> Vec<&ElfSection> {
        self.sections
            .iter()
            .filter(|s| s.executable && s.size > 0)
            .collect()
    }

    /// Defined text symbols inside executable sections — the Wave 4
    /// rank-1 function-seed truth (mirrors the Mach-O audit ground truth).
    pub fn text_symbols(&self) -> Vec<&ElfSymbol> {
        let exec: Vec<(u64, u64)> = self
            .sections
            .iter()
            .filter(|s| s.executable)
            .map(|s| (s.address, s.address.saturating_add(s.size)))
            .collect();
        self.symbols
            .iter()
            .filter(|s| {
                s.defined
                    && s.is_text
                    && exec.iter().any(|(a, b)| s.address >= *a && s.address < *b)
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfSegment {
    /// PT_LOAD etc. kind name (`load`, `dynamic`, `note`, `interp`).
    pub kind: String,
    pub executable: bool,
    pub writable: bool,
    pub address: u64,
    pub size: u64,
    pub file_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfSection {
    pub name: String,
    pub address: u64,
    pub size: u64,
    /// Section type name (`text`-like exec, `data`, `rodata`, `dynsym`, …)
    /// resolved from sh_type + flags, mirroring `Section.kind` strings.
    pub kind: String,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfSymbol {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub defined: bool,
    /// Inside an executable section.
    pub is_text: bool,
    /// From `.dynsym` (dynamic linkage) vs `.symtab` (static).
    pub dynamic: bool,
    /// Undefined symbols are imports.
    pub is_import: bool,
    /// Version string (`memcpy@GLIBC_2.14` → `GLIBC_2.14`) when the
    /// backend resolves `.gnu.version`.
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElfDynamicEntry {
    Needed(String),
    Soname(String),
    Rpath(String),
    Runpath(String),
    Flags(String),
}

impl std::fmt::Debug for ElfImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElfImage")
            .field("label", &self.label)
            .field("architecture", &self.architecture)
            .field("is_position_independent", &self.is_position_independent)
            .field("build_id", &self.build_id)
            .field("soname", &self.soname)
            .field("segments", &self.segments.len())
            .field("sections", &self.sections.len())
            .field("symbols", &self.symbols.len())
            .field("dynamic", &self.dynamic.len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for ElfImage {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
            && self.architecture == other.architecture
            && self.is_position_independent == other.is_position_independent
            && self.build_id == other.build_id
            && self.soname == other.soname
            && self.segments == other.segments
            && self.sections == other.sections
            && self.symbols == other.symbols
            && self.dynamic == other.dynamic
    }
}

impl Eq for ElfImage {}