use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFormat {
    MachO,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinarySource {
    File(PathBuf),
    Memory { label: Option<String> },
}

impl BinarySource {
    pub fn file_path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path.as_path()),
            Self::Memory { .. } => None,
        }
    }

    pub fn memory_label(&self) -> Option<&str> {
        match self {
            Self::File(_) => None,
            Self::Memory { label } => label.as_deref(),
        }
    }

    pub fn default_path(&self) -> PathBuf {
        match self {
            Self::File(path) => path.clone(),
            Self::Memory { label } => label
                .clone()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("<memory>")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    Arm64,
    Arm64e,
    X86_64,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arm64 => f.write_str("arm64"),
            Self::Arm64e => f.write_str("arm64e"),
            Self::X86_64 => f.write_str("x86_64"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    IOS,
    TVOS,
    WatchOS,
    MacCatalyst,
    DriverKit,
    VisionOS,
    VisionOSSimulator,
    Unknown(String),
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MacOS => f.write_str("macos"),
            Self::IOS => f.write_str("ios"),
            Self::TVOS => f.write_str("tvos"),
            Self::WatchOS => f.write_str("watchos"),
            Self::MacCatalyst => f.write_str("maccatalyst"),
            Self::DriverKit => f.write_str("driverkit"),
            Self::VisionOS => f.write_str("visionos"),
            Self::VisionOSSimulator => f.write_str("visionos-simulator"),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

impl Platform {
    pub fn unknown(raw: impl Into<String>) -> Self {
        Self::Unknown(raw.into())
    }

    pub fn parse(value: &str) -> Self {
        match normalize_identifier(value).as_str() {
            "macos" => Self::MacOS,
            "ios" => Self::IOS,
            "tvos" => Self::TVOS,
            "watchos" => Self::WatchOS,
            "maccatalyst" => Self::MacCatalyst,
            "driverkit" => Self::DriverKit,
            "visionos" => Self::VisionOS,
            "visionossimulator" => Self::VisionOSSimulator,
            _ => Self::Unknown(value.to_string()),
        }
    }

    pub fn raw_identifier(&self) -> Option<&str> {
        match self {
            Self::Unknown(raw) => Some(raw.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceInfo {
    pub offset: u64,
    pub size: u64,
    pub is_universal: bool,
    pub cpu_subtype: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceDescriptor {
    pub offset: u64,
    pub size: u64,
    pub is_universal: bool,
    pub cpu_subtype: u32,
    pub architecture: Architecture,
    pub selected: bool,
}

impl SliceDescriptor {
    pub fn from_selected_slice(slice: &SliceInfo, architecture: Architecture) -> Self {
        Self {
            offset: slice.offset,
            size: slice.size,
            is_universal: slice.is_universal,
            cpu_subtype: slice.cpu_subtype,
            architecture,
            selected: true,
        }
    }

    pub fn file_range(&self) -> Option<Range<u64>> {
        self.offset
            .checked_add(self.size)
            .map(|end| self.offset..end)
    }

    pub fn contains_file_offset(&self, file_offset: u64) -> bool {
        self.file_range()
            .map(|range| range.contains(&file_offset))
            .unwrap_or(false)
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionKind {
    Text,
    Data,
    ReadOnlyData,
    ReadOnlyString,
    UninitializedData,
    Tls,
    Metadata,
    Other(String),
}

impl SectionKind {
    pub fn parse(value: &str) -> Self {
        match normalize_identifier(value).as_str() {
            "text" => Self::Text,
            "data" => Self::Data,
            "readonlydata" => Self::ReadOnlyData,
            "readonlystring" => Self::ReadOnlyString,
            "uninitializeddata" => Self::UninitializedData,
            "tls" | "threadlocaldata" => Self::Tls,
            "metadata" => Self::Metadata,
            _ => Self::Other(value.to_string()),
        }
    }
}

impl fmt::Display for SectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => f.write_str("text"),
            Self::Data => f.write_str("data"),
            Self::ReadOnlyData => f.write_str("readonly-data"),
            Self::ReadOnlyString => f.write_str("readonly-string"),
            Self::UninitializedData => f.write_str("uninitialized-data"),
            Self::Tls => f.write_str("tls"),
            Self::Metadata => f.write_str("metadata"),
            Self::Other(value) => f.write_str(value),
        }
    }
}

impl Section {
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.segment_name, self.name)
    }

    pub fn kind_typed(&self) -> SectionKind {
        SectionKind::parse(&self.kind)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelocationKindId(String);

impl RelocationKindId {
    pub fn parse(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelocationKindId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelocationEncodingId(String);

impl RelocationEncodingId {
    pub fn parse(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelocationEncodingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelocationTargetKind {
    Absolute,
    Symbol(String),
    Section(String),
    Other(String),
}

impl RelocationTargetKind {
    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("absolute") {
            return Self::Absolute;
        }
        if let Some(symbol) = value.strip_prefix("symbol#") {
            return Self::Symbol(symbol.to_string());
        }
        if let Some(section) = value.strip_prefix("section#") {
            return Self::Section(section.to_string());
        }
        Self::Other(value.to_string())
    }
}

impl Relocation {
    pub fn kind_typed(&self) -> RelocationKindId {
        RelocationKindId::parse(self.kind.clone())
    }

    pub fn encoding_typed(&self) -> RelocationEncodingId {
        RelocationEncodingId::parse(self.encoding.clone())
    }

    pub fn target_typed(&self) -> RelocationTargetKind {
        RelocationTargetKind::parse(&self.target)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    Regular,
    Reexport {
        dylib: String,
        symbol: Option<String>,
    },
    Resolver {
        resolver_address: Option<u64>,
    },
    StubAndResolver {
        stub_address: Option<u64>,
        resolver_address: Option<u64>,
    },
    WeakDefinition,
    Absolute,
    ThreadLocal,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRecord {
    pub name: String,
    pub address: Option<u64>,
    pub raw_flags: String,
    pub flags: ExportFlags,
    pub kind: ExportKind,
    pub reexport_target: Option<(String, Option<String>)>,
    pub resolver_target: Option<u64>,
}

pub type ExportedSymbol = ExportRecord;

const EXPORT_FLAG_KIND_MASK: u64 = 0x03;
const EXPORT_FLAG_KIND_REGULAR: u8 = 0x00;
const EXPORT_FLAG_KIND_THREAD_LOCAL: u8 = 0x01;
const EXPORT_FLAG_KIND_ABSOLUTE: u8 = 0x02;
const EXPORT_FLAG_WEAK_DEFINITION: u64 = 0x04;
const EXPORT_FLAG_REEXPORT: u64 = 0x08;
const EXPORT_FLAG_STUB_AND_RESOLVER: u64 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExportFlags {
    pub raw_bits: u64,
    pub kind_bits: u8,
    pub is_weak_definition: bool,
    pub is_reexport: bool,
    pub is_stub_and_resolver: bool,
    pub is_thread_local: bool,
    pub is_absolute: bool,
    pub unknown_bits: u64,
}

impl ExportFlags {
    pub const fn from_bits(raw_bits: u64) -> Self {
        let kind_bits = (raw_bits & EXPORT_FLAG_KIND_MASK) as u8;
        let known_bits = EXPORT_FLAG_KIND_MASK
            | EXPORT_FLAG_WEAK_DEFINITION
            | EXPORT_FLAG_REEXPORT
            | EXPORT_FLAG_STUB_AND_RESOLVER;
        Self {
            raw_bits,
            kind_bits,
            is_weak_definition: raw_bits & EXPORT_FLAG_WEAK_DEFINITION != 0,
            is_reexport: raw_bits & EXPORT_FLAG_REEXPORT != 0,
            is_stub_and_resolver: raw_bits & EXPORT_FLAG_STUB_AND_RESOLVER != 0,
            is_thread_local: kind_bits == EXPORT_FLAG_KIND_THREAD_LOCAL,
            is_absolute: kind_bits == EXPORT_FLAG_KIND_ABSOLUTE,
            unknown_bits: raw_bits & !known_bits,
        }
    }

    // Compatibility helper for older tests/callers that constructed flags from descriptive text.
    pub fn parse(value: impl AsRef<str>) -> Self {
        let value = value.as_ref().to_ascii_lowercase();
        let mut raw_bits = 0u64;
        if value.contains("weak") {
            raw_bits |= EXPORT_FLAG_WEAK_DEFINITION;
        }
        if value.contains("reexport") {
            raw_bits |= EXPORT_FLAG_REEXPORT;
        }
        if value.contains("stub-and-resolver")
            || value.contains("stub_and_resolver")
            || value.contains("stub and resolver")
            || (value.contains("stub") && value.contains("resolver"))
        {
            raw_bits |= EXPORT_FLAG_STUB_AND_RESOLVER;
        }
        if value.contains("thread-local")
            || value.contains("thread_local")
            || value.contains("threadlocal")
        {
            raw_bits |= u64::from(EXPORT_FLAG_KIND_THREAD_LOCAL);
        } else if value.contains("absolute") {
            raw_bits |= u64::from(EXPORT_FLAG_KIND_ABSOLUTE);
        } else {
            raw_bits |= u64::from(EXPORT_FLAG_KIND_REGULAR);
        }
        Self::from_bits(raw_bits)
    }

    pub const fn kind_name(&self) -> &'static str {
        match self.kind_bits {
            EXPORT_FLAG_KIND_REGULAR => "regular",
            EXPORT_FLAG_KIND_THREAD_LOCAL => "thread_local",
            EXPORT_FLAG_KIND_ABSOLUTE => "absolute",
            _ => "unknown",
        }
    }
}

impl fmt::Display for ExportFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "raw_bits={:#x} kind_bits={:#x} kind={} weak={} reexport={} stub_and_resolver={} thread_local={} absolute={} unknown_bits={:#x}",
            self.raw_bits,
            self.kind_bits,
            self.kind_name(),
            self.is_weak_definition,
            self.is_reexport,
            self.is_stub_and_resolver,
            self.is_thread_local,
            self.is_absolute,
            self.unknown_bits
        )
    }
}

impl ExportRecord {
    pub fn flags_typed(&self) -> ExportFlags {
        self.flags.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjcMetadata {
    pub class_names: Vec<String>,
    pub selector_names: Vec<String>,
    pub method_names: Vec<String>,
    pub image_info_flags: Option<u32>,
    pub pointer_refs: Vec<ObjcPointerRef>,
    pub classes: Vec<ObjcClassRecord>,
    pub protocols: Vec<ObjcProtocolRecord>,
    pub categories: Vec<ObjcCategoryRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjcPointerKind {
    SelRef,
    ClassRef,
    ClassList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjcPointerRef {
    pub kind: ObjcPointerKind,
    pub table_address: u64,
    pub raw_pointer: u64,
    pub resolved_address: Option<u64>,
    pub resolved_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjcClassRecord {
    pub class_pointer: u64,
    pub name: Option<String>,
    pub name_source: ObjcNameSource,
    pub superclass_pointer: Option<u64>,
    pub superclass_name: Option<String>,
    pub metaclass_pointer: Option<u64>,
    pub ro_pointer: Option<u64>,
    pub method_list_pointer: Option<u64>,
    pub property_list_pointer: Option<u64>,
    pub protocol_list_pointer: Option<u64>,
    pub ivar_list_pointer: Option<u64>,
    pub methods: Vec<ObjcMethodRecord>,
    pub class_methods: Vec<ObjcMethodRecord>,
    pub properties: Vec<ObjcPropertyRecord>,
    pub ivars: Vec<ObjcIvarRecord>,
    pub adopted_protocols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjcProtocolRecord {
    pub pointer: u64,
    pub name: Option<String>,
    pub name_source: ObjcNameSource,
    pub required_instance_methods: Vec<ObjcMethodRecord>,
    pub required_class_methods: Vec<ObjcMethodRecord>,
    pub optional_instance_methods: Vec<ObjcMethodRecord>,
    pub optional_class_methods: Vec<ObjcMethodRecord>,
    pub properties: Vec<ObjcPropertyRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjcCategoryRecord {
    pub pointer: u64,
    pub name: Option<String>,
    pub name_source: ObjcNameSource,
    pub record_source: ObjcCategoryRecordSource,
    pub class_pointer: Option<u64>,
    pub class_name: Option<String>,
    pub class_name_source: ObjcNameSource,
    pub property_list_pointer: Option<u64>,
    pub protocol_list_pointer: Option<u64>,
    pub methods: Vec<ObjcMethodRecord>,
    pub class_methods: Vec<ObjcMethodRecord>,
    pub properties: Vec<ObjcPropertyRecord>,
    pub adopted_protocols: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjcCategoryRecordSource {
    RuntimeList,
    SymbolSynthesis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjcMethodOwnerKind {
    Class,
    Metaclass,
    Protocol,
    Category,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjcMethodRecord {
    pub owner_pointer: u64,
    pub owner_kind: ObjcMethodOwnerKind,
    pub is_class_method: bool,
    pub selector: Option<String>,
    pub selector_source: ObjcSelectorSource,
    pub implementation: Option<u64>,
    pub type_encoding: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjcNameSource {
    Runtime,
    PointerTable,
    LegacyPool,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjcSelectorSource {
    Direct,
    Relative,
    LegacyPool,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjcPropertyRecord {
    pub owner_pointer: u64,
    pub name: Option<String>,
    pub name_source: ObjcNameSource,
    pub attributes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjcIvarRecord {
    pub owner_pointer: u64,
    pub name: Option<String>,
    pub name_source: ObjcNameSource,
    pub type_encoding: Option<String>,
    pub offset: Option<u64>,
}

impl ObjcMetadata {
    pub fn pointer_refs_of_kind(
        &self,
        kind: ObjcPointerKind,
    ) -> impl Iterator<Item = &ObjcPointerRef> {
        self.pointer_refs
            .iter()
            .filter(move |pointer_ref| pointer_ref.kind == kind)
    }

    pub fn class_by_name(&self, name: &str) -> Option<&ObjcClassRecord> {
        self.classes
            .iter()
            .find(|class_record| class_record.name.as_deref() == Some(name))
    }

    pub fn protocol_by_name(&self, name: &str) -> Option<&ObjcProtocolRecord> {
        self.protocols
            .iter()
            .find(|protocol_record| protocol_record.name.as_deref() == Some(name))
    }

    pub fn class_by_pointer(&self, pointer: u64) -> Option<&ObjcClassRecord> {
        self.classes
            .iter()
            .find(|class_record| class_record.class_pointer == pointer)
    }

    pub fn protocol_by_pointer(&self, pointer: u64) -> Option<&ObjcProtocolRecord> {
        self.protocols
            .iter()
            .find(|protocol_record| protocol_record.pointer == pointer)
    }

    pub fn category_by_name(&self, name: &str) -> Option<&ObjcCategoryRecord> {
        self.categories
            .iter()
            .find(|category_record| category_record.name.as_deref() == Some(name))
    }

    pub fn category_by_pointer(&self, pointer: u64) -> Option<&ObjcCategoryRecord> {
        self.categories
            .iter()
            .find(|category_record| category_record.pointer == pointer)
    }

    pub fn category_by_class_and_name(
        &self,
        class_name: &str,
        category_name: &str,
    ) -> Option<&ObjcCategoryRecord> {
        self.categories.iter().find(|category_record| {
            category_record.class_name.as_deref() == Some(class_name)
                && category_record.name.as_deref() == Some(category_name)
        })
    }

    pub fn categories_for_class<'a>(
        &'a self,
        class_name: &'a str,
    ) -> impl Iterator<Item = &'a ObjcCategoryRecord> {
        self.categories.iter().filter(move |category_record| {
            category_record.class_name.as_deref() == Some(class_name)
        })
    }

    pub fn methods_with_selector_source(
        &self,
        source: ObjcSelectorSource,
    ) -> impl Iterator<Item = &ObjcMethodRecord> {
        self.classes
            .iter()
            .flat_map(|record| record.methods.iter().chain(record.class_methods.iter()))
            .chain(self.protocols.iter().flat_map(|record| {
                record
                    .required_instance_methods
                    .iter()
                    .chain(record.required_class_methods.iter())
                    .chain(record.optional_instance_methods.iter())
                    .chain(record.optional_class_methods.iter())
            }))
            .chain(
                self.categories
                    .iter()
                    .flat_map(|record| record.methods.iter().chain(record.class_methods.iter())),
            )
            .filter(move |record| record.selector_source == source)
    }
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
    pub import_bindings: Vec<ImportBindingRecord>,
    pub stubs: Vec<StubEntry>,
    pub stub_helpers: Vec<StubHelperEntry>,
}

impl DyldMetadata {
    pub fn export_by_name(&self, name: &str) -> Option<&ExportRecord> {
        self.exported_symbols
            .iter()
            .find(|export_record| export_record.name == name)
    }

    pub fn export_by_address(&self, address: u64) -> Option<&ExportRecord> {
        self.exported_symbols
            .iter()
            .find(|export_record| export_record.address == Some(address))
    }

    pub fn exports_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a ExportRecord> {
        self.exported_symbols
            .iter()
            .filter(move |export_record| export_record.name == name)
    }

    pub fn binding_for_symbol(&self, dylib: &str, name: &str) -> Option<&ImportBindingRecord> {
        self.import_bindings
            .iter()
            .find(|binding| binding.dylib == dylib && binding.name == name)
    }

    pub fn binding_for_ordinal(&self, ordinal: u32) -> Option<&ImportBindingRecord> {
        self.import_bindings
            .iter()
            .find(|binding| binding.ordinal == Some(ordinal))
    }

    pub fn binding_for_symbol_index(&self, symbol_index: u32) -> Option<&ImportBindingRecord> {
        self.import_bindings
            .iter()
            .find(|binding| binding.symbol_index == Some(symbol_index))
    }

    pub fn bindings_at_address(&self, address: u64) -> impl Iterator<Item = &ImportBindingRecord> {
        self.import_bindings
            .iter()
            .filter(move |binding| binding.address == Some(address))
    }

    pub fn stub_for_address(&self, address: u64) -> Option<&StubEntry> {
        self.stubs
            .iter()
            .find(|stub_entry| stub_entry.stub_address == address)
    }

    pub fn stub_for_pointer_address(&self, address: u64) -> Option<&StubEntry> {
        self.stubs
            .iter()
            .find(|stub_entry| stub_entry.pointer_address == Some(address))
    }

    pub fn stub_for_helper_address(&self, address: u64) -> Option<&StubEntry> {
        self.stubs
            .iter()
            .find(|stub_entry| stub_entry.helper_address == Some(address))
    }

    pub fn helper_for_address(&self, address: u64) -> Option<&StubHelperEntry> {
        self.stub_helpers
            .iter()
            .find(|helper_entry| helper_entry.helper_address == address)
    }

    pub fn helper_for_stub_address(&self, address: u64) -> Option<&StubHelperEntry> {
        self.stub_helpers
            .iter()
            .find(|helper_entry| helper_entry.target_stub == Some(address))
    }

    pub fn helper_for_pointer_address(&self, address: u64) -> Option<&StubHelperEntry> {
        self.stub_helpers
            .iter()
            .find(|helper_entry| helper_entry.pointer_address == Some(address))
    }

    pub fn helper_for_binding_ordinal(&self, ordinal: u32) -> Option<&StubHelperEntry> {
        self.stub_helpers
            .iter()
            .find(|helper_entry| helper_entry.binding_ordinal == Some(ordinal))
    }

    pub fn helpers_for_symbol<'a>(
        &'a self,
        dylib: &'a str,
        name: &'a str,
    ) -> impl Iterator<Item = &'a StubHelperEntry> {
        self.stub_helpers.iter().filter(move |helper_entry| {
            helper_entry.dylib.as_deref() == Some(dylib)
                && helper_entry.name.as_deref() == Some(name)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportBindingSource {
    ChainedFixup,
    IndirectSymbol,
    Stub,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportBindingKind {
    Lazy,
    NonLazy,
    ChainedFixup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportBindingRecord {
    pub dylib: String,
    pub name: String,
    pub address: Option<u64>,
    pub offset: Option<u64>,
    pub addend: i64,
    pub ordinal: Option<u32>,
    pub symbol_index: Option<u32>,
    pub binding_kind: ImportBindingKind,
    pub source: ImportBindingSource,
    pub is_weak: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StubKind {
    Lazy,
    NonLazy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubEntry {
    pub stub_address: u64,
    pub section: Option<String>,
    pub pointer_section: Option<String>,
    pub pointer_address: Option<u64>,
    pub helper_address: Option<u64>,
    pub binding_ordinal: Option<u32>,
    pub stub_kind: StubKind,
    pub dylib: Option<String>,
    pub name: Option<String>,
    pub source: ImportBindingSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubHelperEntry {
    pub helper_address: u64,
    pub target_stub: Option<u64>,
    pub stub_section: Option<String>,
    pub pointer_address: Option<u64>,
    pub pointer_section: Option<String>,
    pub binding_ordinal: Option<u32>,
    pub dylib: Option<String>,
    pub name: Option<String>,
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
    IndirectCall {
        via: String,
    },
    IndirectBranch {
        via: String,
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
    ImportBinding {
        dylib: String,
        name: String,
        address: Option<u64>,
        offset: Option<u64>,
        addend: i64,
        binding_kind: ImportBindingKind,
        source: ImportBindingSource,
        is_weak: bool,
    },
    Stub {
        stub_address: u64,
        section: Option<String>,
        pointer_section: Option<String>,
        pointer_address: Option<u64>,
        helper_address: Option<u64>,
        binding_ordinal: Option<u32>,
        stub_kind: StubKind,
        dylib: Option<String>,
        name: Option<String>,
        source: ImportBindingSource,
    },
    StubHelper {
        helper_address: u64,
        target_stub: Option<u64>,
        stub_section: Option<String>,
        pointer_address: Option<u64>,
        pointer_section: Option<String>,
        binding_ordinal: Option<u32>,
        dylib: Option<String>,
        name: Option<String>,
    },
    RelocationEvidence {
        address: u64,
        kind: String,
        encoding: String,
        target: String,
        addend: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Annotation {
    Symbol(String),
    TargetSymbol {
        address: u64,
        name: String,
    },
    Import {
        dylib: String,
        name: String,
    },
    IndirectControlFlow {
        kind: String,
        via: String,
    },
    Relocation {
        address: u64,
        kind: String,
        encoding: String,
        target: String,
        addend: i64,
    },
    ImportBinding {
        dylib: String,
        name: String,
        address: Option<u64>,
        offset: Option<u64>,
        addend: i64,
        binding_kind: ImportBindingKind,
        source: ImportBindingSource,
    },
    ImportBindingEvidence {
        dylib: String,
        name: String,
        address: Option<u64>,
        offset: Option<u64>,
        addend: i64,
        binding_kind: ImportBindingKind,
        source: ImportBindingSource,
    },
    RelocationEvidence {
        address: u64,
        kind: String,
        encoding: String,
        target: String,
        addend: i64,
    },
    JumpTableCandidate {
        base: u64,
        index_register: String,
        element_size: u8,
    },
    IndirectTargetResolved {
        via: String,
        target: u64,
        reason: IndirectTargetReason,
    },
    Note(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTargetReason {
    HelperTarget,
    StubTarget,
    ExportAddress,
    FunctionPointer,
    ImportPointer,
    RegisterState,
}

impl fmt::Display for IndirectTargetReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::HelperTarget => "helper-target",
            Self::StubTarget => "stub-target",
            Self::ExportAddress => "export-address",
            Self::FunctionPointer => "function-pointer",
            Self::ImportPointer => "import-pointer",
            Self::RegisterState => "register-state",
        };
        f.write_str(text)
    }
}

impl fmt::Display for Annotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Symbol(name) => write!(f, "symbol {name}"),
            Self::TargetSymbol { address, name } => write!(f, "target {name} ({address:#x})"),
            Self::Import { dylib, name } => write!(f, "import {dylib}:{name}"),
            Self::IndirectControlFlow { kind, via } => {
                write!(f, "indirect {kind} via {via}")
            }
            Self::Relocation {
                address,
                kind,
                encoding,
                target,
                addend,
            } => write!(
                f,
                "reloc kind={kind} encoding={encoding} target={target} addend={addend} addr={address:#x}"
            ),
            Self::ImportBinding {
                dylib,
                name,
                address,
                offset,
                addend,
                binding_kind,
                source,
            } => write!(
                f,
                "binding {dylib}:{name} addr={} off={} addend={addend} kind={binding_kind:?} source={source:?}",
                address
                    .map(|value| format!("{value:#x}"))
                    .unwrap_or_else(|| "-".to_string()),
                offset
                    .map(|value| format!("{value:#x}"))
                    .unwrap_or_else(|| "-".to_string())
            ),
            Self::ImportBindingEvidence {
                dylib,
                name,
                address,
                offset,
                addend,
                binding_kind,
                source,
            } => write!(
                f,
                "binding-evidence {dylib}:{name} addr={} off={} addend={addend} kind={binding_kind:?} source={source:?}",
                address
                    .map(|value| format!("{value:#x}"))
                    .unwrap_or_else(|| "-".to_string()),
                offset
                    .map(|value| format!("{value:#x}"))
                    .unwrap_or_else(|| "-".to_string())
            ),
            Self::RelocationEvidence {
                address,
                kind,
                encoding,
                target,
                addend,
            } => write!(
                f,
                "reloc-evidence kind={kind} encoding={encoding} target={target} addend={addend} addr={address:#x}"
            ),
            Self::JumpTableCandidate {
                base,
                index_register,
                element_size,
            } => write!(
                f,
                "jump-table base={base:#x} index={index_register} elem_size={element_size}"
            ),
            Self::IndirectTargetResolved {
                via,
                target,
                reason,
            } => write!(f, "indirect-target via {via} -> {target:#x} ({reason})"),
            Self::Note(note) => f.write_str(note),
        }
    }
}

impl Reference {
    pub fn from_binding(binding: &ImportBindingRecord) -> Self {
        Self::ImportBinding {
            dylib: binding.dylib.clone(),
            name: binding.name.clone(),
            address: binding.address,
            offset: binding.offset,
            addend: binding.addend,
            binding_kind: binding.binding_kind,
            source: binding.source,
            is_weak: binding.is_weak,
        }
    }

    pub fn from_stub(stub: &StubEntry) -> Self {
        Self::Stub {
            stub_address: stub.stub_address,
            section: stub.section.clone(),
            pointer_section: stub.pointer_section.clone(),
            pointer_address: stub.pointer_address,
            helper_address: stub.helper_address,
            binding_ordinal: stub.binding_ordinal,
            stub_kind: stub.stub_kind.clone(),
            dylib: stub.dylib.clone(),
            name: stub.name.clone(),
            source: stub.source,
        }
    }

    pub fn from_stub_helper(helper: &StubHelperEntry) -> Self {
        Self::StubHelper {
            helper_address: helper.helper_address,
            target_stub: helper.target_stub,
            stub_section: helper.stub_section.clone(),
            pointer_address: helper.pointer_address,
            pointer_section: helper.pointer_section.clone(),
            binding_ordinal: helper.binding_ordinal,
            dylib: helper.dylib.clone(),
            name: helper.name.clone(),
        }
    }
}

impl Annotation {
    pub fn import_binding_evidence(binding: &ImportBindingRecord) -> Self {
        Self::ImportBindingEvidence {
            dylib: binding.dylib.clone(),
            name: binding.name.clone(),
            address: binding.address,
            offset: binding.offset,
            addend: binding.addend,
            binding_kind: binding.binding_kind,
            source: binding.source,
        }
    }

    pub fn relocation_evidence(relocation: &Relocation) -> Self {
        Self::RelocationEvidence {
            address: relocation.address,
            kind: relocation.kind.clone(),
            encoding: relocation.encoding.clone(),
            target: relocation.target.clone(),
            addend: relocation.addend,
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
    pub recovered_values: Vec<RecoveredValue>,
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
    pub limit: Option<DisassemblyLimit>,
    pub include_annotations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisassemblyOptions {
    pub include_annotations: bool,
    pub include_value_flow: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyResult {
    pub target: String,
    pub start_address: u64,
    pub bytes_len: usize,
    pub decoded_bytes: usize,
    pub end_address: u64,
    pub instruction_count: usize,
    pub stop_reason: DisassemblyStopReason,
    pub instructions: Vec<DecodedInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyRequestV2 {
    pub target: DisassemblyTarget,
    pub range: Option<Range<u64>>,
    pub limit: DisassemblyLimit,
    pub options: DisassemblyOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyResultV2 {
    pub target: String,
    pub start_address: u64,
    pub decoded_bytes: usize,
    pub end_address: u64,
    pub instruction_count: usize,
    pub stop_reason: DisassemblyStopReason,
    pub instructions: Vec<DecodedInstruction>,
}

impl DisassemblyRequestV2 {
    pub fn with_range(mut self, range: Option<Range<u64>>) -> Self {
        self.range = range;
        self
    }

    pub fn has_valid_range(&self) -> bool {
        self.range
            .as_ref()
            .map(|range| range.start < range.end)
            .unwrap_or(true)
    }

    pub fn range_contains(&self, address: u64) -> bool {
        self.range
            .as_ref()
            .map(|range| range.contains(&address))
            .unwrap_or(true)
    }

    pub fn effective_instruction_cap(&self) -> Option<usize> {
        self.limit.instruction_cap()
    }
}

impl DisassemblyResultV2 {
    pub fn bytes_len_compat(&self) -> usize {
        self.decoded_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisassemblyLimit {
    Instructions(usize),
    Bytes(usize),
    Unlimited,
}

impl DisassemblyLimit {
    pub fn instruction_cap(self) -> Option<usize> {
        match self {
            Self::Instructions(count) => Some(count),
            Self::Bytes(bytes) => Some(bytes.div_ceil(4)),
            Self::Unlimited => None,
        }
    }
}

pub fn effective_instruction_limit_v2(request: &DisassemblyRequestV2) -> Option<usize> {
    request.effective_instruction_cap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisassemblyStopReason {
    InstructionLimitReached,
    ByteLimitReached,
    WindowClipped,
    InputExhausted,
    DecodeHalt,
    TargetRangeEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredValueKind {
    Address,
    CString,
    Literal,
    ImportPointer,
    StubAddress,
    ExportAddress,
    FunctionPointer,
    ObjcSelector,
    ObjcClass,
    ObjcMethodList,
    JumpTableBase,
    UnknownData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredValueSource {
    Adr,
    AdrpAdd,
    AdrpLoad,
    LiteralLoad,
    MoveWide,
    StubMetadata,
    ObjcMetadata,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredValue {
    pub register: String,
    pub value: u64,
    pub kind: RecoveredValueKind,
    pub source: RecoveredValueSource,
}

impl Default for DisassemblyOptions {
    fn default() -> Self {
        Self {
            include_annotations: true,
            include_value_flow: false,
        }
    }
}

impl DisassemblyRequest {
    pub fn legacy(target: DisassemblyTarget, max_instructions: Option<usize>) -> Self {
        Self {
            target,
            max_instructions,
            limit: None,
            include_annotations: true,
        }
    }

    pub fn options(&self) -> DisassemblyOptions {
        DisassemblyOptions {
            include_annotations: self.include_annotations,
            include_value_flow: false,
        }
    }

    pub fn effective_limit(&self) -> DisassemblyLimit {
        match self.limit {
            Some(limit) => limit,
            None => self
                .max_instructions
                .map(DisassemblyLimit::Instructions)
                .unwrap_or(DisassemblyLimit::Unlimited),
        }
    }

    pub fn to_v2(&self) -> DisassemblyRequestV2 {
        DisassemblyRequestV2::from(self)
    }
}

impl From<&DisassemblyRequest> for DisassemblyRequestV2 {
    fn from(value: &DisassemblyRequest) -> Self {
        Self {
            target: value.target.clone(),
            range: None,
            limit: value.effective_limit(),
            options: value.options(),
        }
    }
}

impl DisassemblyResult {
    pub fn to_v2(&self) -> DisassemblyResultV2 {
        DisassemblyResultV2::from(self)
    }
}

impl From<&DisassemblyResult> for DisassemblyResultV2 {
    fn from(value: &DisassemblyResult) -> Self {
        Self {
            target: value.target.clone(),
            start_address: value.start_address,
            decoded_bytes: value.decoded_bytes,
            end_address: value.end_address,
            instruction_count: value.instruction_count,
            stop_reason: value.stop_reason,
            instructions: value.instructions.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryImageValidationError {
    MissingAvailableSlices,
    MissingSelectedSlice,
    MultipleSelectedSlices,
    SelectedSliceMismatch,
}

impl fmt::Display for BinaryImageValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAvailableSlices => {
                f.write_str("binary image must expose at least one slice")
            }
            Self::MissingSelectedSlice => {
                f.write_str("binary image must expose one selected slice descriptor")
            }
            Self::MultipleSelectedSlices => {
                f.write_str("binary image cannot expose multiple selected slice descriptors")
            }
            Self::SelectedSliceMismatch => {
                f.write_str("selected slice descriptor must match slice info and architecture")
            }
        }
    }
}

impl std::error::Error for BinaryImageValidationError {}

#[derive(Debug, Clone)]
pub struct BinaryImageBuilder {
    source: BinarySource,
    path: PathBuf,
    format: BinaryFormat,
    architecture: Architecture,
    endianness: Endianness,
    entry_point: Option<u64>,
    platform: Option<Platform>,
    slice: SliceInfo,
    available_slices: Vec<SliceDescriptor>,
    segments: Vec<Segment>,
    sections: Vec<Section>,
    symbols: Vec<Symbol>,
    imports: Vec<Import>,
    relocations: Vec<Relocation>,
    objc: ObjcMetadata,
    dyld: DyldMetadata,
    data: Arc<[u8]>,
}

pub struct BinaryImage {
    source: BinarySource,
    path: PathBuf,
    format: BinaryFormat,
    architecture: Architecture,
    endianness: Endianness,
    entry_point: Option<u64>,
    platform: Option<Platform>,
    slice: SliceInfo,
    available_slices: Vec<SliceDescriptor>,
    segments: Vec<Segment>,
    sections: Vec<Section>,
    symbols: Vec<Symbol>,
    imports: Vec<Import>,
    relocations: Vec<Relocation>,
    objc: ObjcMetadata,
    dyld: DyldMetadata,
    data: Arc<[u8]>,
    section_name_index_cache: OnceLock<BTreeMap<String, Vec<usize>>>,
    symbol_name_index_cache: OnceLock<BTreeMap<String, Vec<usize>>>,
    import_name_index_cache: OnceLock<BTreeMap<String, Vec<usize>>>,
    relocation_address_index_cache: OnceLock<BTreeMap<u64, Vec<usize>>>,
}

impl fmt::Debug for BinaryImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BinaryImage")
            .field("path", &self.path)
            .field("format", &self.format)
            .field("source", &self.source)
            .field("architecture", &self.architecture)
            .field("endianness", &self.endianness)
            .field("entry_point", &self.entry_point)
            .field("platform", &self.platform)
            .field("slice", &self.slice)
            .field("available_slices", &self.available_slices)
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
        source: BinarySource,
        path: PathBuf,
        format: BinaryFormat,
        architecture: Architecture,
        endianness: Endianness,
        entry_point: Option<u64>,
        platform: Option<Platform>,
        slice: SliceInfo,
        available_slices: Vec<SliceDescriptor>,
        segments: Vec<Segment>,
        sections: Vec<Section>,
        symbols: Vec<Symbol>,
        imports: Vec<Import>,
        relocations: Vec<Relocation>,
        objc: ObjcMetadata,
        dyld: DyldMetadata,
        data: Arc<[u8]>,
    ) -> Self {
        BinaryImageBuilder {
            source,
            path,
            format,
            architecture,
            endianness,
            entry_point,
            platform,
            slice,
            available_slices,
            segments,
            sections,
            symbols,
            imports,
            relocations,
            objc,
            dyld,
            data,
        }
        .build()
        .expect("BinaryImage::new received invalid slice inventory")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_file_bytes(
        path: PathBuf,
        format: BinaryFormat,
        architecture: Architecture,
        endianness: Endianness,
        entry_point: Option<u64>,
        platform: Option<Platform>,
        slice: SliceInfo,
        segments: Vec<Segment>,
        sections: Vec<Section>,
        symbols: Vec<Symbol>,
        imports: Vec<Import>,
        relocations: Vec<Relocation>,
        objc: ObjcMetadata,
        dyld: DyldMetadata,
        data: Arc<[u8]>,
    ) -> Self {
        Self::builder(
            BinarySource::File(path.clone()),
            path,
            format,
            architecture,
            endianness,
            entry_point,
            platform,
            slice.clone(),
            segments,
            sections,
            symbols,
            imports,
            relocations,
            objc,
            dyld,
            data,
        )
        .with_available_slices(vec![SliceDescriptor::from_selected_slice(
            &slice,
            architecture,
        )])
        .build()
        .expect("BinaryImage::from_file_bytes received invalid slice inventory")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_memory_bytes(
        label: Option<String>,
        format: BinaryFormat,
        architecture: Architecture,
        endianness: Endianness,
        entry_point: Option<u64>,
        platform: Option<Platform>,
        slice: SliceInfo,
        segments: Vec<Segment>,
        sections: Vec<Section>,
        symbols: Vec<Symbol>,
        imports: Vec<Import>,
        relocations: Vec<Relocation>,
        objc: ObjcMetadata,
        dyld: DyldMetadata,
        data: Arc<[u8]>,
    ) -> Self {
        let source = BinarySource::Memory {
            label: label.clone(),
        };
        Self::builder(
            source,
            BinarySource::Memory { label }.default_path(),
            format,
            architecture,
            endianness,
            entry_point,
            platform,
            slice.clone(),
            segments,
            sections,
            symbols,
            imports,
            relocations,
            objc,
            dyld,
            data,
        )
        .with_available_slices(vec![SliceDescriptor::from_selected_slice(
            &slice,
            architecture,
        )])
        .build()
        .expect("BinaryImage::from_memory_bytes received invalid slice inventory")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn builder(
        source: BinarySource,
        path: PathBuf,
        format: BinaryFormat,
        architecture: Architecture,
        endianness: Endianness,
        entry_point: Option<u64>,
        platform: Option<Platform>,
        slice: SliceInfo,
        segments: Vec<Segment>,
        sections: Vec<Section>,
        symbols: Vec<Symbol>,
        imports: Vec<Import>,
        relocations: Vec<Relocation>,
        objc: ObjcMetadata,
        dyld: DyldMetadata,
        data: Arc<[u8]>,
    ) -> BinaryImageBuilder {
        BinaryImageBuilder {
            source,
            path,
            format,
            architecture,
            endianness,
            entry_point,
            platform,
            slice,
            available_slices: Vec::new(),
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

    pub fn source_path(&self) -> Option<&Path> {
        self.source.file_path()
    }

    pub fn source(&self) -> &BinarySource {
        &self.source
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn format(&self) -> BinaryFormat {
        self.format
    }

    pub fn architecture(&self) -> Architecture {
        self.architecture
    }

    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    pub fn entry_point(&self) -> Option<u64> {
        self.entry_point
    }

    pub fn platform(&self) -> Option<&Platform> {
        self.platform.as_ref()
    }

    pub fn source_label(&self) -> Option<&str> {
        self.source.memory_label()
    }

    pub fn platform_raw_identifier(&self) -> Option<&str> {
        self.platform.as_ref().and_then(Platform::raw_identifier)
    }

    pub fn selected_slice(&self) -> &SliceInfo {
        &self.slice
    }

    pub fn available_slices(&self) -> &[SliceDescriptor] {
        &self.available_slices
    }

    pub fn selected_slice_descriptor(&self) -> Option<&SliceDescriptor> {
        self.available_slices
            .iter()
            .find(|descriptor| descriptor.selected)
    }

    pub fn with_available_slices(mut self, available_slices: Vec<SliceDescriptor>) -> Self {
        self.available_slices = available_slices;
        self
    }

    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn imports(&self) -> &[Import] {
        &self.imports
    }

    pub fn relocations(&self) -> &[Relocation] {
        &self.relocations
    }

    pub fn objc(&self) -> &ObjcMetadata {
        &self.objc
    }

    pub fn dyld(&self) -> &DyldMetadata {
        &self.dyld
    }

    pub fn read_c_string_at_address(&self, address: u64, max_len: usize) -> Option<String> {
        let (section, data) = self.bytes_for_virtual_range(address, max_len)?;
        if !section.kind.to_ascii_lowercase().contains("string")
            && !section.name.contains("cstring")
            && !section.name.contains("objc")
        {
            return None;
        }
        let length = data
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(data.len());
        if length == 0 {
            return None;
        }
        let value = String::from_utf8_lossy(&data[..length]).into_owned();
        let printable = value
            .chars()
            .all(|ch| ch.is_ascii_graphic() || ch == ' ' || ch == ':');
        printable.then_some(value)
    }

    pub fn objc_selector_name_at_address(&self, address: u64) -> Option<&str> {
        self.objc
            .pointer_refs
            .iter()
            .find(|entry| {
                entry.kind == ObjcPointerKind::SelRef && entry.resolved_address == Some(address)
            })
            .and_then(|entry| entry.resolved_name.as_deref())
    }

    pub fn objc_class_name_at_address(&self, address: u64) -> Option<&str> {
        self.objc
            .classes
            .iter()
            .find(|record| {
                record.class_pointer == address || record.metaclass_pointer == Some(address)
            })
            .and_then(|record| record.name.as_deref())
            .or_else(|| {
                self.objc
                    .pointer_refs
                    .iter()
                    .find(|entry| {
                        matches!(
                            entry.kind,
                            ObjcPointerKind::ClassRef | ObjcPointerKind::ClassList
                        ) && entry.resolved_address == Some(address)
                    })
                    .and_then(|entry| entry.resolved_name.as_deref())
            })
    }

    pub fn objc_method_list_owner_at_address(&self, address: u64) -> Option<&str> {
        self.objc
            .classes
            .iter()
            .find(|record| {
                record.method_list_pointer == Some(address)
                    || record.protocol_list_pointer == Some(address)
                    || record.property_list_pointer == Some(address)
                    || record.ivar_list_pointer == Some(address)
            })
            .and_then(|record| record.name.as_deref())
    }

    pub fn section_by_short_name(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|section| section.name == name)
    }

    pub fn section_by_full_name(&self, name: &str) -> Option<&Section> {
        self.sections
            .iter()
            .find(|section| section.full_name() == name)
    }

    pub fn symbol_by_name_exact(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| symbol.name == name)
    }

    pub fn slice_bytes(&self) -> &[u8] {
        self.try_slice_bytes().unwrap_or(&[])
    }

    pub fn try_slice_bytes(&self) -> Option<&[u8]> {
        let start = usize::try_from(self.slice.offset).ok()?;
        let size = usize::try_from(self.slice.size).ok()?;
        let end = start.checked_add(size)?;
        self.data.get(start..end)
    }

    pub fn bytes_for_file_range(&self, file_offset: u64, size: u64) -> Option<&[u8]> {
        let absolute_start = self.slice.offset.checked_add(file_offset)?;
        let absolute_end = absolute_start.checked_add(size)?;
        let start = usize::try_from(absolute_start).ok()?;
        let end = usize::try_from(absolute_end).ok()?;
        self.data.get(start..end)
    }

    pub fn bytes_for_section(&self, section: &Section) -> Option<&[u8]> {
        let offset = section.file_offset?;
        self.bytes_for_file_range(offset, section.file_size)
    }

    pub fn section_by_name(&self, name: &str) -> Option<&Section> {
        self.section_by_short_name(name)
            .or_else(|| self.section_by_full_name(name))
            .or_else(|| self.section_by_full_name(&format!("__TEXT:{name}")))
    }

    pub fn symbol_by_name(&self, name: &str) -> Option<&Symbol> {
        self.symbol_by_name_exact(name)
    }

    pub fn section_name_index(&self) -> BTreeMap<String, Vec<usize>> {
        self.section_name_index_cached().clone()
    }

    pub fn symbol_name_index(&self) -> BTreeMap<String, Vec<usize>> {
        self.symbol_name_index_cached().clone()
    }

    pub fn import_name_index(&self) -> BTreeMap<String, Vec<usize>> {
        self.import_name_index_cached().clone()
    }

    pub fn relocation_address_index(&self) -> BTreeMap<u64, Vec<usize>> {
        self.relocation_address_index_cached().clone()
    }

    pub fn section_name_index_cached(&self) -> &BTreeMap<String, Vec<usize>> {
        self.section_name_index_cache
            .get_or_init(|| Self::build_section_name_index(&self.sections))
    }

    pub fn symbol_name_index_cached(&self) -> &BTreeMap<String, Vec<usize>> {
        self.symbol_name_index_cache
            .get_or_init(|| Self::build_symbol_name_index(&self.symbols))
    }

    pub fn import_name_index_cached(&self) -> &BTreeMap<String, Vec<usize>> {
        self.import_name_index_cache
            .get_or_init(|| Self::build_import_name_index(&self.imports))
    }

    pub fn relocation_address_index_cached(&self) -> &BTreeMap<u64, Vec<usize>> {
        self.relocation_address_index_cache
            .get_or_init(|| Self::build_relocation_address_index(&self.relocations))
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
        let readable_size = section.size.min(section.file_size);
        let section_end = section.address.checked_add(readable_size)?;
        let size_u64 = u64::try_from(size).ok()?;
        if address.checked_add(size_u64)? > section_end {
            return None;
        }
        let section_offset = address.checked_sub(section.address)?;
        let data = self.bytes_for_file_range(file_offset.checked_add(section_offset)?, size_u64)?;
        Some((section, data))
    }

    pub fn virtual_range_for_section(&self, section: &Section) -> Range<u64> {
        section.address..section.address.saturating_add(section.size)
    }

    pub fn file_backed_virtual_range_for_section(&self, section: &Section) -> Range<u64> {
        let mapped_size = section.size.min(section.file_size);
        section.address..section.address.saturating_add(mapped_size)
    }

    pub fn effective_instruction_limit(&self, request: &DisassemblyRequest) -> Option<usize> {
        self.effective_instruction_limit_v2(&request.to_v2())
    }

    pub fn effective_instruction_limit_v2(&self, request: &DisassemblyRequestV2) -> Option<usize> {
        effective_instruction_limit_v2(request)
    }

    fn build_section_name_index(sections: &[Section]) -> BTreeMap<String, Vec<usize>> {
        let mut index = BTreeMap::<String, Vec<usize>>::new();
        for (position, section) in sections.iter().enumerate() {
            index
                .entry(section.name.clone())
                .or_default()
                .push(position);
            index.entry(section.full_name()).or_default().push(position);
        }
        index
    }

    fn build_symbol_name_index(symbols: &[Symbol]) -> BTreeMap<String, Vec<usize>> {
        let mut index = BTreeMap::<String, Vec<usize>>::new();
        for (position, symbol) in symbols.iter().enumerate() {
            index.entry(symbol.name.clone()).or_default().push(position);
        }
        index
    }

    fn build_import_name_index(imports: &[Import]) -> BTreeMap<String, Vec<usize>> {
        let mut index = BTreeMap::<String, Vec<usize>>::new();
        for (position, import) in imports.iter().enumerate() {
            index
                .entry(format!("{}:{}", import.dylib, import.name))
                .or_default()
                .push(position);
        }
        index
    }

    fn build_relocation_address_index(relocations: &[Relocation]) -> BTreeMap<u64, Vec<usize>> {
        let mut index = BTreeMap::<u64, Vec<usize>>::new();
        for (position, relocation) in relocations.iter().enumerate() {
            index.entry(relocation.address).or_default().push(position);
        }
        index
    }
}

impl BinaryImageBuilder {
    pub fn with_available_slices(mut self, available_slices: Vec<SliceDescriptor>) -> Self {
        self.available_slices = available_slices;
        self
    }

    pub fn build(self) -> Result<BinaryImage, BinaryImageValidationError> {
        let available_slices = if self.available_slices.is_empty() {
            vec![SliceDescriptor::from_selected_slice(
                &self.slice,
                self.architecture,
            )]
        } else {
            self.available_slices
        };
        if available_slices.is_empty() {
            return Err(BinaryImageValidationError::MissingAvailableSlices);
        }

        let mut selected_iter = available_slices
            .iter()
            .filter(|descriptor| descriptor.selected);
        let Some(selected) = selected_iter.next() else {
            return Err(BinaryImageValidationError::MissingSelectedSlice);
        };
        if selected_iter.next().is_some() {
            return Err(BinaryImageValidationError::MultipleSelectedSlices);
        }
        if selected.offset != self.slice.offset
            || selected.size != self.slice.size
            || selected.cpu_subtype != self.slice.cpu_subtype
            || selected.is_universal != self.slice.is_universal
            || selected.architecture != self.architecture
        {
            return Err(BinaryImageValidationError::SelectedSliceMismatch);
        }

        Ok(BinaryImage {
            source: self.source,
            path: self.path,
            format: self.format,
            architecture: self.architecture,
            endianness: self.endianness,
            entry_point: self.entry_point,
            platform: self.platform,
            slice: self.slice,
            available_slices,
            segments: self.segments,
            sections: self.sections,
            symbols: self.symbols,
            imports: self.imports,
            relocations: self.relocations,
            objc: self.objc,
            dyld: self.dyld,
            data: self.data,
            section_name_index_cache: OnceLock::new(),
            symbol_name_index_cache: OnceLock::new(),
            import_name_index_cache: OnceLock::new(),
            relocation_address_index_cache: OnceLock::new(),
        })
    }
}

fn normalize_identifier(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}
