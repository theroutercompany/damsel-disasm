use memmap2::Mmap;
use std::fmt;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFormat {
    MachO,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    Arm64,
    Arm64e,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arm64 => f.write_str("arm64"),
            Self::Arm64e => f.write_str("arm64e"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceInfo {
    pub offset: u64,
    pub size: u64,
    pub is_universal: bool,
    pub cpu_subtype: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub segment_name: String,
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub file_offset: Option<u64>,
    pub file_size: u64,
    pub kind: String,
    pub executable: bool,
}

impl Section {
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.segment_name, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Text,
    Data,
    Section,
    File,
    Label,
    Unknown(String),
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => f.write_str("text"),
            Self::Data => f.write_str("data"),
            Self::Section => f.write_str("section"),
            Self::File => f.write_str("file"),
            Self::Label => f.write_str("label"),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub kind: SymbolKind,
    pub defined: bool,
    pub global: bool,
    pub weak: bool,
    pub section: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub name: String,
    pub dylib: String,
    pub address: Option<u64>,
    pub offset: Option<u64>,
    pub addend: i64,
    pub is_lazy: bool,
    pub is_weak: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    pub section: String,
    pub address: u64,
    pub size: u8,
    pub kind: String,
    pub encoding: String,
    pub target: String,
    pub addend: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedSymbol {
    pub name: String,
    pub address: Option<u64>,
    pub flags: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjcMetadata {
    pub class_names: Vec<String>,
    pub selector_names: Vec<String>,
    pub method_names: Vec<String>,
    pub image_info_flags: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DyldMetadata {
    pub imported_dylibs: Vec<String>,
    pub rpaths: Vec<String>,
    pub exported_symbols: Vec<ExportedSymbol>,
    pub function_starts: Vec<u64>,
    pub has_rebases: bool,
    pub has_binds: bool,
    pub has_chained_fixups: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Register(String),
    ImmediateSigned(i64),
    ImmediateUnsigned(u64),
    FloatingImmediate(u32),
    Label(u64),
    Memory {
        base: String,
        index: Option<String>,
        displacement: i64,
        writeback: Option<String>,
    },
    ShiftedRegister {
        register: String,
        shift: String,
    },
    QualifiedRegister {
        register: String,
        qualifier: char,
    },
    MultiRegister(Vec<String>),
    SystemRegister(String),
    Condition(String),
    Name(String),
    Other(String),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Register(reg) => f.write_str(reg),
            Self::ImmediateSigned(value) => write!(f, "#{value:#x}"),
            Self::ImmediateUnsigned(value) => write!(f, "#{value:#x}"),
            Self::FloatingImmediate(bits) => write!(f, "#{bits:#x}"),
            Self::Label(target) => write!(f, "{target:#x}"),
            Self::Memory {
                base,
                index,
                displacement,
                writeback,
            } => {
                write!(f, "[{base}")?;
                if let Some(index) = index {
                    write!(f, ", {index}")?;
                }
                if *displacement != 0 {
                    write!(f, ", #{displacement:#x}")?;
                }
                write!(f, "]")?;
                if let Some(writeback) = writeback {
                    write!(f, "{writeback}")?;
                }
                Ok(())
            }
            Self::ShiftedRegister { register, shift } => write!(f, "{register}, {shift}"),
            Self::QualifiedRegister {
                register,
                qualifier,
            } => write!(f, "{register}.{qualifier}"),
            Self::MultiRegister(registers) => write!(f, "{{{}}}", registers.join(", ")),
            Self::SystemRegister(reg) => f.write_str(reg),
            Self::Condition(cond) => f.write_str(cond),
            Self::Name(value) => f.write_str(value),
            Self::Other(value) => f.write_str(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    Call {
        target: u64,
    },
    Branch {
        target: u64,
    },
    Page {
        target: u64,
    },
    Data {
        target: u64,
    },
    Import {
        name: String,
        dylib: String,
        address: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Annotation {
    Symbol(String),
    TargetSymbol { address: u64, name: String },
    Import { dylib: String, name: String },
    Note(String),
}

impl fmt::Display for Annotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Symbol(name) => write!(f, "symbol {name}"),
            Self::TargetSymbol { address, name } => write!(f, "target {name} ({address:#x})"),
            Self::Import { dylib, name } => write!(f, "import {dylib}:{name}"),
            Self::Note(note) => f.write_str(note),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub address: u64,
    pub size: u8,
    pub opcode: u32,
    pub mnemonic: String,
    pub operands: Vec<Operand>,
    pub references: Vec<Reference>,
    pub annotations: Vec<Annotation>,
}

impl DecodedInstruction {
    pub fn render(&self) -> String {
        if self.operands.is_empty() {
            self.mnemonic.clone()
        } else {
            format!(
                "{} {}",
                self.mnemonic,
                self.operands
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisassemblyTarget {
    Symbol(String),
    Address(u64),
    Section(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyRequest {
    pub target: DisassemblyTarget,
    pub max_instructions: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyResult {
    pub target: String,
    pub start_address: u64,
    pub bytes_len: usize,
    pub instructions: Vec<DecodedInstruction>,
}

pub struct BinaryImage {
    pub path: PathBuf,
    pub format: BinaryFormat,
    pub architecture: Architecture,
    pub endianness: Endianness,
    pub entry_point: Option<u64>,
    pub platform: Option<String>,
    pub slice: SliceInfo,
    pub segments: Vec<Segment>,
    pub sections: Vec<Section>,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub relocations: Vec<Relocation>,
    pub objc: ObjcMetadata,
    pub dyld: DyldMetadata,
    data: Arc<Mmap>,
}

impl fmt::Debug for BinaryImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BinaryImage")
            .field("path", &self.path)
            .field("format", &self.format)
            .field("architecture", &self.architecture)
            .field("endianness", &self.endianness)
            .field("entry_point", &self.entry_point)
            .field("platform", &self.platform)
            .field("slice", &self.slice)
            .field("segments", &self.segments)
            .field("sections", &self.sections)
            .field("symbols", &self.symbols)
            .field("imports", &self.imports)
            .field("relocations", &self.relocations)
            .field("objc", &self.objc)
            .field("dyld", &self.dyld)
            .finish()
    }
}

impl BinaryImage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        path: PathBuf,
        format: BinaryFormat,
        architecture: Architecture,
        endianness: Endianness,
        entry_point: Option<u64>,
        platform: Option<String>,
        slice: SliceInfo,
        segments: Vec<Segment>,
        sections: Vec<Section>,
        symbols: Vec<Symbol>,
        imports: Vec<Import>,
        relocations: Vec<Relocation>,
        objc: ObjcMetadata,
        dyld: DyldMetadata,
        data: Arc<Mmap>,
    ) -> Self {
        Self {
            path,
            format,
            architecture,
            endianness,
            entry_point,
            platform,
            slice,
            segments,
            sections,
            symbols,
            imports,
            relocations,
            objc,
            dyld,
            data,
        }
    }

    pub fn slice_bytes(&self) -> &[u8] {
        let start = self.slice.offset as usize;
        let end = start + self.slice.size as usize;
        &self.data[start..end]
    }

    pub fn bytes_for_file_range(&self, file_offset: u64, size: u64) -> Option<&[u8]> {
        let start = self.slice.offset.checked_add(file_offset)? as usize;
        let end = start.checked_add(size as usize)?;
        self.data.get(start..end)
    }

    pub fn bytes_for_section(&self, section: &Section) -> Option<&[u8]> {
        let offset = section.file_offset?;
        self.bytes_for_file_range(offset, section.file_size)
    }

    pub fn section_by_name(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|section| {
            section.name == name
                || section.full_name() == name
                || section.full_name() == format!("__TEXT:{name}")
        })
    }

    pub fn symbol_by_name(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| symbol.name == name)
    }

    pub fn containing_section(&self, address: u64) -> Option<&Section> {
        self.sections.iter().find(|section| {
            let range = section.address..section.address.saturating_add(section.size);
            range.contains(&address)
        })
    }

    pub fn bytes_for_virtual_range(&self, address: u64, size: usize) -> Option<(&Section, &[u8])> {
        let section = self.containing_section(address)?;
        let file_offset = section.file_offset?;
        let section_end = section.address.checked_add(section.size)?;
        if address.checked_add(size as u64)? > section_end {
            return None;
        }
        let section_offset = address.checked_sub(section.address)? as usize;
        let data = self.bytes_for_file_range(file_offset + section_offset as u64, size as u64)?;
        Some((section, data))
    }

    pub fn virtual_range_for_section(&self, section: &Section) -> Range<u64> {
        section.address..section.address.saturating_add(section.size)
    }
}
