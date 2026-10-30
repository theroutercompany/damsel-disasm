mod mcp;
mod output;
mod ui;

use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use damsel_core::{
    BinaryImage, CacheDependentRecord, CacheImageDependencyRecord, CacheImageRecord,
    CacheLookupResult, CacheReexportRecord, CacheSymbolImporterRecord, CacheSymbolProviderRecord,
    DecodedInstruction, DisassemblyAnalysis, DisassemblyCacheLinkSummary, DisassemblyCacheLinkUse,
    DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget, ExportFlagName,
    Import, ImportBindingKind, ObjcMethodRecord, ObjcNameSource, ObjcSelectorSource,
    ProjectedBinaryImage, Relocation, Section, SharedCache, SharedCacheMemberRole, StubKind,
    Symbol, SymbolKind, SymbolicationMatch,
};
use damsel_macho::{
    SharedCacheSession, analyze_disassembly_with_image, disassemble_v2, ingest_binary_image,
    inspect_shared_cache, load,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

#[derive(Debug, Parser)]
#[command(name = "damsel", about = "Static Mach-O arm64 disassembler")]
struct Cli {
    #[arg(long, global = true, value_enum, default_value_t = OutputFormatArg::Text)]
    format: OutputFormatArg,
    #[arg(long, global = true)]
    pretty: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormatArg {
    Text,
    Json,
}

impl From<OutputFormatArg> for output::OutputFormat {
    fn from(value: OutputFormatArg) -> Self {
        match value {
            OutputFormatArg::Text => output::OutputFormat::Text,
            OutputFormatArg::Json => output::OutputFormat::Json,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SectionSortArg {
    Address,
    Name,
    Size,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SymbolSortArg {
    Address,
    Name,
    Size,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ImportSortArg {
    Address,
    Dylib,
    Name,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RelocationSortArg {
    Address,
    Section,
    Size,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DyldSortArg {
    Address,
    Name,
    Dylib,
    Source,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CacheImageSortArg {
    Name,
    Address,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CacheExportSortArg {
    Address,
    Name,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CacheDependencyKindArg {
    Internal,
    External,
    All,
}

impl From<DyldSortArg> for output::DyldSortKey {
    fn from(value: DyldSortArg) -> Self {
        match value {
            DyldSortArg::Address => output::DyldSortKey::Address,
            DyldSortArg::Name => output::DyldSortKey::Name,
            DyldSortArg::Dylib => output::DyldSortKey::Dylib,
            DyldSortArg::Source => output::DyldSortKey::Source,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DyldSourceArg {
    ChainedFixup,
    IndirectSymbol,
    Stub,
    Other,
}

impl From<DyldSourceArg> for damsel_core::ImportBindingSource {
    fn from(value: DyldSourceArg) -> Self {
        match value {
            DyldSourceArg::ChainedFixup => damsel_core::ImportBindingSource::ChainedFixup,
            DyldSourceArg::IndirectSymbol => damsel_core::ImportBindingSource::IndirectSymbol,
            DyldSourceArg::Stub => damsel_core::ImportBindingSource::Stub,
            DyldSourceArg::Other => damsel_core::ImportBindingSource::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DyldBindingKindArg {
    Lazy,
    NonLazy,
    ChainedFixup,
}

impl From<DyldBindingKindArg> for ImportBindingKind {
    fn from(value: DyldBindingKindArg) -> Self {
        match value {
            DyldBindingKindArg::Lazy => ImportBindingKind::Lazy,
            DyldBindingKindArg::NonLazy => ImportBindingKind::NonLazy,
            DyldBindingKindArg::ChainedFixup => ImportBindingKind::ChainedFixup,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DyldStubKindArg {
    Lazy,
    NonLazy,
}

impl From<DyldStubKindArg> for StubKind {
    fn from(value: DyldStubKindArg) -> Self {
        match value {
            DyldStubKindArg::Lazy => StubKind::Lazy,
            DyldStubKindArg::NonLazy => StubKind::NonLazy,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DyldExportKindArg {
    Regular,
    Reexport,
    Resolver,
    StubAndResolver,
    WeakDefinition,
    Absolute,
    ThreadLocal,
    Unknown,
}

impl From<DyldExportKindArg> for output::ExportKindFilter {
    fn from(value: DyldExportKindArg) -> Self {
        match value {
            DyldExportKindArg::Regular => output::ExportKindFilter::Regular,
            DyldExportKindArg::Reexport => output::ExportKindFilter::Reexport,
            DyldExportKindArg::Resolver => output::ExportKindFilter::Resolver,
            DyldExportKindArg::StubAndResolver => output::ExportKindFilter::StubAndResolver,
            DyldExportKindArg::WeakDefinition => output::ExportKindFilter::WeakDefinition,
            DyldExportKindArg::Absolute => output::ExportKindFilter::Absolute,
            DyldExportKindArg::ThreadLocal => output::ExportKindFilter::ThreadLocal,
            DyldExportKindArg::Unknown => output::ExportKindFilter::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DyldExportFlagArg {
    WeakDefinition,
    Reexport,
    StubAndResolver,
    ThreadLocal,
    Absolute,
}

impl From<DyldExportFlagArg> for ExportFlagName {
    fn from(value: DyldExportFlagArg) -> Self {
        match value {
            DyldExportFlagArg::WeakDefinition => ExportFlagName::WeakDefinition,
            DyldExportFlagArg::Reexport => ExportFlagName::Reexport,
            DyldExportFlagArg::StubAndResolver => ExportFlagName::StubAndResolver,
            DyldExportFlagArg::ThreadLocal => ExportFlagName::ThreadLocal,
            DyldExportFlagArg::Absolute => ExportFlagName::Absolute,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ObjcDetailArg {
    Summary,
    Classes,
    Protocols,
    Categories,
    Methods,
    Properties,
    Ivars,
    All,
}

impl From<ObjcDetailArg> for output::ObjcDetail {
    fn from(value: ObjcDetailArg) -> Self {
        match value {
            ObjcDetailArg::Summary => output::ObjcDetail::Summary,
            ObjcDetailArg::Classes => output::ObjcDetail::Classes,
            ObjcDetailArg::Protocols => output::ObjcDetail::Protocols,
            ObjcDetailArg::Categories => output::ObjcDetail::Categories,
            ObjcDetailArg::Methods => output::ObjcDetail::Methods,
            ObjcDetailArg::Properties => output::ObjcDetail::Properties,
            ObjcDetailArg::Ivars => output::ObjcDetail::Ivars,
            ObjcDetailArg::All => output::ObjcDetail::All,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ObjcNameSourceArg {
    Runtime,
    PointerTable,
    LegacyPool,
    Unresolved,
}

impl From<ObjcNameSourceArg> for ObjcNameSource {
    fn from(value: ObjcNameSourceArg) -> Self {
        match value {
            ObjcNameSourceArg::Runtime => ObjcNameSource::Runtime,
            ObjcNameSourceArg::PointerTable => ObjcNameSource::PointerTable,
            ObjcNameSourceArg::LegacyPool => ObjcNameSource::LegacyPool,
            ObjcNameSourceArg::Unresolved => ObjcNameSource::Unresolved,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ObjcSelectorSourceArg {
    Direct,
    Relative,
    LegacyPool,
    Unresolved,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DoctorCheckArg {
    All,
    MachoAnalysis,
    FixtureRebuild,
    FixtureDriftCheck,
    BenchCompile,
    BenchRuntime,
}

impl From<DoctorCheckArg> for output::DoctorCheckTarget {
    fn from(value: DoctorCheckArg) -> Self {
        match value {
            DoctorCheckArg::All => output::DoctorCheckTarget::All,
            DoctorCheckArg::MachoAnalysis => output::DoctorCheckTarget::MachoAnalysis,
            DoctorCheckArg::FixtureRebuild => output::DoctorCheckTarget::FixtureRebuild,
            DoctorCheckArg::FixtureDriftCheck => output::DoctorCheckTarget::FixtureDriftCheck,
            DoctorCheckArg::BenchCompile => output::DoctorCheckTarget::BenchCompile,
            DoctorCheckArg::BenchRuntime => output::DoctorCheckTarget::BenchRuntime,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DoctorRequireStatusArg {
    Supported,
    SupportedWithDegradedFeatures,
}

impl From<DoctorRequireStatusArg> for output::DoctorRequireStatus {
    fn from(value: DoctorRequireStatusArg) -> Self {
        match value {
            DoctorRequireStatusArg::Supported => output::DoctorRequireStatus::Supported,
            DoctorRequireStatusArg::SupportedWithDegradedFeatures => {
                output::DoctorRequireStatus::SupportedWithDegradedFeatures
            }
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ObjcCategorySourceArg {
    RuntimeList,
    SymbolSynthesis,
    SymbolSynthesisWithLists,
}

impl From<ObjcCategorySourceArg> for output::ObjcCategorySourceFilter {
    fn from(value: ObjcCategorySourceArg) -> Self {
        match value {
            ObjcCategorySourceArg::RuntimeList => output::ObjcCategorySourceFilter::RuntimeList,
            ObjcCategorySourceArg::SymbolSynthesis => {
                output::ObjcCategorySourceFilter::SymbolSynthesis
            }
            ObjcCategorySourceArg::SymbolSynthesisWithLists => {
                output::ObjcCategorySourceFilter::SymbolSynthesisWithLists
            }
        }
    }
}

impl From<ObjcSelectorSourceArg> for ObjcSelectorSource {
    fn from(value: ObjcSelectorSourceArg) -> Self {
        match value {
            ObjcSelectorSourceArg::Direct => ObjcSelectorSource::Direct,
            ObjcSelectorSourceArg::Relative => ObjcSelectorSource::Relative,
            ObjcSelectorSourceArg::LegacyPool => ObjcSelectorSource::LegacyPool,
            ObjcSelectorSourceArg::Unresolved => ObjcSelectorSource::Unresolved,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ObjcMethodKindArg {
    Instance,
    Class,
}

impl ObjcMethodKindArg {
    fn is_class_method(self) -> bool {
        matches!(self, Self::Class)
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(
        about = "Report host compatibility and optionally enforce capability thresholds",
        long_about = "Report host compatibility and optionally enforce capability thresholds.\n\n`--check all` validates the CI capability set (`macho-analysis`, `fixture-rebuild`, `fixture-drift-check`, `bench-compile`, `bench-runtime`). Because `fixture-rebuild` is macOS-only and `bench-runtime` is only fully supported on linux arm64, `--check all` is intentionally non-portable across hosts.\n\nCI guidance:\n  macOS runners: `doctor --check macho-analysis --require-status supported`, `doctor --check fixture-rebuild --require-status supported`, `doctor --check fixture-drift-check --require-status supported`, `doctor --check bench-compile --require-status supported`, and `doctor --check bench-runtime --require-status supported-with-degraded-features`\n  linux x86_64 runners: `doctor --check macho-analysis --require-status supported`, `doctor --check fixture-drift-check --require-status supported`, `doctor --check bench-compile --require-status supported`, and `doctor --check bench-runtime --require-status supported-with-degraded-features`\n  linux arm64 runners: `doctor --check macho-analysis --require-status supported`, `doctor --check fixture-drift-check --require-status supported`, `doctor --check bench-compile --require-status supported`, and `doctor --check bench-runtime --require-status supported`\n\nExit codes:\n  0 => success\n  1 => typed command error\n  2 => doctor threshold failure or argument parsing failure\n\nBehavior note:\n  threshold failures still print a doctor report to stdout\n  argument parsing failures come from clap on stderr and do not print a doctor report"
    )]
    Doctor {
        #[arg(
            long,
            value_enum,
            help = "Capability target to enforce; `all` includes macOS-only fixture-rebuild"
        )]
        check: Option<DoctorCheckArg>,
        #[arg(
            long,
            value_enum,
            requires = "check",
            help = "Minimum required capability status (defaults to `supported` when omitted)"
        )]
        require_status: Option<DoctorRequireStatusArg>,
    },
    Info {
        path: PathBuf,
    },
    Sections {
        path: PathBuf,
        #[arg(long = "exec")]
        executable_only: bool,
        #[arg(long)]
        segment: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum)]
        sort: Option<SectionSortArg>,
    },
    Symbols {
        path: PathBuf,
        #[arg(long)]
        defined: bool,
        #[arg(long)]
        global: bool,
        #[arg(long)]
        section: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum)]
        sort: Option<SymbolSortArg>,
    },
    Imports {
        path: PathBuf,
        #[arg(long)]
        dylib: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        lazy: bool,
        #[arg(long)]
        weak: bool,
        #[arg(long)]
        resolved: bool,
        #[arg(long, value_enum)]
        sort: Option<ImportSortArg>,
    },
    Relocs {
        path: PathBuf,
        #[arg(long)]
        section: Option<String>,
        #[arg(long, value_enum)]
        sort: Option<RelocationSortArg>,
    },
    Dyld {
        path: PathBuf,
        #[arg(long)]
        dylibs: bool,
        #[arg(long)]
        rpaths: bool,
        #[arg(long)]
        exports: bool,
        #[arg(long = "function-starts")]
        function_starts: bool,
        #[arg(long)]
        bindings: bool,
        #[arg(long)]
        stubs: bool,
        #[arg(long)]
        helpers: bool,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        dylib: Option<String>,
        #[arg(long, value_enum)]
        source: Option<DyldSourceArg>,
        #[arg(long, value_enum)]
        binding_kind: Option<DyldBindingKindArg>,
        #[arg(long, value_enum)]
        stub_kind: Option<DyldStubKindArg>,
        #[arg(long, value_enum)]
        export_kind: Option<DyldExportKindArg>,
        #[arg(long, value_enum)]
        export_flag: Option<DyldExportFlagArg>,
        #[arg(long)]
        ordinal: Option<u32>,
        #[arg(long, value_enum)]
        sort: Option<DyldSortArg>,
    },
    Slices {
        path: PathBuf,
    },
    Objc {
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = ObjcDetailArg::All)]
        detail: ObjcDetailArg,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long, value_enum)]
        name_source: Option<ObjcNameSourceArg>,
        #[arg(long, value_enum)]
        selector_source: Option<ObjcSelectorSourceArg>,
        #[arg(long, value_enum)]
        category_source: Option<ObjcCategorySourceArg>,
    },
    #[command(group(
        ArgGroup::new("target")
            .args(["symbol", "swift_symbol", "addr", "section", "objc_selector"])
            .required(true)
    ))]
    Disasm {
        path: PathBuf,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long = "swift-symbol")]
        swift_symbol: Option<String>,
        #[arg(long, value_parser = parse_address)]
        addr: Option<u64>,
        #[arg(long)]
        section: Option<String>,
        #[arg(long = "objc-owner", requires = "objc_selector")]
        objc_owner: Option<String>,
        #[arg(long = "objc-selector", requires = "objc_owner")]
        objc_selector: Option<String>,
        #[arg(long = "objc-method-kind", value_enum, requires = "objc_selector")]
        objc_method_kind: Option<ObjcMethodKindArg>,
        #[arg(long)]
        count: Option<usize>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        bytes: Option<usize>,
        #[arg(long, value_parser = parse_address)]
        from: Option<u64>,
        #[arg(long, value_parser = parse_address)]
        to: Option<u64>,
        #[arg(long)]
        no_annotations: bool,
        #[arg(long)]
        show_references: bool,
        #[arg(long)]
        show_values: bool,
        #[arg(long, alias = "summary")]
        analysis: bool,
    },
    Ui {
        #[arg(long, default_value_t = 4317)]
        port: u16,
        #[arg(long)]
        no_open: bool,
    },
    #[command(
        about = "Analyze a binary into a persistent project database (RE Power Wave 4)",
        long_about = "Analyze a binary into a persistent project database.\n\nCreates `<target>.damsel/` next to the target with a SQLite store, ingests sections/symbols/imports/ObjC as seed facts, and prints a summary. Existing projects require `--refresh` to rebuild."
    )]
    Analyze {
        path: PathBuf,
        #[arg(long, help = "Delete and rebuild an existing project directory")]
        refresh: bool,
    },
    #[command(about = "Inspect or export an analysis project database")]
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    #[command(about = "List functions discovered in an analyzed project")]
    Functions {
        project: PathBuf,
        #[arg(long, help = "Filter by name substring")]
        name: Option<String>,
        #[arg(long, help = "Maximum rows to print")]
        limit: Option<usize>,
    },
    #[command(about = "First-touch triage: stripped-ness, entropy, packers, anti-analysis (Wave 7)")]
    Recon { path: PathBuf },
    #[command(about = "Diff two analyzed projects (function-level, Wave 7)")]
    Diff {
        left: PathBuf,
        right: PathBuf,
        #[arg(long, help = "Output Markdown report instead of text/JSON")]
        markdown: bool,
        #[arg(long, help = "Output self-contained HTML report")]
        html: bool,
    },
    #[command(about = "Run as an MCP server exposing typed analysis tools (Wave 7)")]
    Serve {
        #[arg(long, help = "Default project path for tools that accept relative targets")]
        project: Option<PathBuf>,
    },
    #[command(about = "Batch-analyze a list of binaries, writing JSONL results (Wave 7)")]
    Batch {
        #[arg(long, help = "File with one binary path per line")]
        inputs: PathBuf,
        #[arg(long, help = "Output directory for per-target results")]
        out: PathBuf,
        #[arg(long, help = "Continue on per-target failures (default) or stop")]
        fail_fast: bool,
    },
    #[command(about = "Decompile a function to provenance-carrying pseudocode (Wave 6 D5)")]
    Decompile {
        project: PathBuf,
        #[arg(long, help = "Function name or 0x address")]
        function: Option<String>,
        #[arg(long, help = "Decompile all functions (batch mode)")]
        all: bool,
        #[arg(long, help = "Output directory for --all (one .pseudo file per function)")]
        out: Option<PathBuf>,
    },
    #[command(about = "D0 function summary: calls, imports, strings, problems (Wave 6)")]
    Summary {
        project: PathBuf,
        #[arg(long, help = "Function name or 0x address")]
        function: String,
        #[arg(long, help = "Output Markdown report")]
        markdown: bool,
    },
    #[command(about = "Search functions, symbols, imports, sections, and strings")]
    Search {
        project: PathBuf,
        query: String,
        #[arg(long, help = "Restrict to one kind (function, symbol, import, section, string)")]
        kind: Option<String>,
        #[arg(long, help = "Maximum rows to print")]
        limit: Option<usize>,
    },
    #[command(about = "List strings harvested from an analyzed project")]
    Strings {
        project: PathBuf,
        #[arg(long, help = "Filter by value substring (requires the target binary on disk)")]
        name: Option<String>,
        #[arg(long, help = "Filter by class (url, path, format, selector_like)")]
        class: Option<String>,
        #[arg(long, help = "Minimum value length in characters")]
        min_len: Option<usize>,
        #[arg(long, help = "Maximum rows to print")]
        limit: Option<usize>,
    },
    #[command(about = "Query stored cross-references by direction")]
    Xrefs {
        project: PathBuf,
        #[arg(long, value_parser = parse_address, help = "Target address (0x…); references TO it")]
        to: Option<u64>,
        #[arg(long, value_parser = parse_address, help = "Source address (0x…); references FROM it")]
        from: Option<u64>,
        #[arg(long, help = "Filter by xref kind (call, branch, data, import, …)")]
        kind: Option<String>,
        #[arg(long, help = "Maximum rows to print")]
        limit: Option<usize>,
    },
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    #[command(about = "Show project metadata, sources, and fact counts")]
    Show { project: PathBuf },
    #[command(about = "Write the canonical deterministic export")]
    Export {
        project: PathBuf,
        #[arg(long, help = "Output file (defaults to stdout, bytes verbatim)")]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum CacheCommand {
    Info {
        cache: PathBuf,
    },
    Images {
        cache: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, value_enum, default_value_t = CacheImageSortArg::Name)]
        sort: CacheImageSortArg,
    },
    Image {
        cache: PathBuf,
        image: String,
    },
    Exports {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum)]
        kind: Option<DyldExportKindArg>,
        #[arg(long, value_enum)]
        flag: Option<DyldExportFlagArg>,
        #[arg(long, value_enum, default_value_t = CacheExportSortArg::Address)]
        sort: CacheExportSortArg,
    },
    LookupAddress {
        cache: PathBuf,
        #[arg(value_parser = parse_address)]
        vmaddr: u64,
    },
    ResolveSymbol {
        cache: PathBuf,
        symbol: String,
        #[arg(long)]
        image: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    ImageDeps {
        cache: PathBuf,
        image: String,
        #[arg(long, value_enum, default_value_t = CacheDependencyKindArg::All)]
        kind: CacheDependencyKindArg,
        #[arg(long)]
        limit: Option<usize>,
    },
    Dependents {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    SymbolProviders {
        cache: PathBuf,
        symbol: String,
        #[arg(long)]
        image: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    SymbolImporters {
        cache: PathBuf,
        symbol: String,
        #[arg(long)]
        image: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    Reexports {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    Sections {
        cache: PathBuf,
        image: String,
        #[arg(long = "exec")]
        executable_only: bool,
        #[arg(long)]
        segment: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum)]
        sort: Option<SectionSortArg>,
    },
    Symbols {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        defined: bool,
        #[arg(long)]
        global: bool,
        #[arg(long)]
        section: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum)]
        sort: Option<SymbolSortArg>,
    },
    Imports {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        dylib: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        lazy: bool,
        #[arg(long)]
        weak: bool,
        #[arg(long)]
        resolved: bool,
        #[arg(long, value_enum)]
        sort: Option<ImportSortArg>,
    },
    Dyld {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        dylibs: bool,
        #[arg(long)]
        rpaths: bool,
        #[arg(long)]
        exports: bool,
        #[arg(long = "function-starts")]
        function_starts: bool,
        #[arg(long)]
        bindings: bool,
        #[arg(long)]
        stubs: bool,
        #[arg(long)]
        helpers: bool,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        dylib: Option<String>,
        #[arg(long, value_enum)]
        source: Option<DyldSourceArg>,
        #[arg(long, value_enum)]
        binding_kind: Option<DyldBindingKindArg>,
        #[arg(long, value_enum)]
        stub_kind: Option<DyldStubKindArg>,
        #[arg(long, value_enum)]
        export_kind: Option<DyldExportKindArg>,
        #[arg(long, value_enum)]
        export_flag: Option<DyldExportFlagArg>,
        #[arg(long)]
        ordinal: Option<u32>,
        #[arg(long, value_enum)]
        sort: Option<DyldSortArg>,
    },
    Objc {
        cache: PathBuf,
        image: String,
        #[arg(long, value_enum, default_value_t = ObjcDetailArg::All)]
        detail: ObjcDetailArg,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long, value_enum)]
        name_source: Option<ObjcNameSourceArg>,
        #[arg(long, value_enum)]
        selector_source: Option<ObjcSelectorSourceArg>,
        #[arg(long, value_enum)]
        category_source: Option<ObjcCategorySourceArg>,
    },
    #[command(group(
        ArgGroup::new("target")
            .args(["symbol", "swift_symbol", "addr", "section", "objc_selector"])
            .required(true)
    ))]
    Disasm {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long = "swift-symbol")]
        swift_symbol: Option<String>,
        #[arg(long, value_parser = parse_address)]
        addr: Option<u64>,
        #[arg(long)]
        section: Option<String>,
        #[arg(long = "objc-owner", requires = "objc_selector")]
        objc_owner: Option<String>,
        #[arg(long = "objc-selector", requires = "objc_owner")]
        objc_selector: Option<String>,
        #[arg(long = "objc-method-kind", value_enum, requires = "objc_selector")]
        objc_method_kind: Option<ObjcMethodKindArg>,
        #[arg(long)]
        count: Option<usize>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        bytes: Option<usize>,
        #[arg(long, value_parser = parse_address)]
        from: Option<u64>,
        #[arg(long, value_parser = parse_address)]
        to: Option<u64>,
        #[arg(long)]
        no_annotations: bool,
        #[arg(long)]
        show_references: bool,
        #[arg(long)]
        show_values: bool,
        #[arg(long, alias = "summary")]
        analysis: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let output_settings = output::OutputSettings {
        format: cli.format.into(),
        pretty: cli.pretty,
    };

    match run(cli, output_settings) {
        Ok(CliRunOutcome::Success) => {}
        Ok(CliRunOutcome::DoctorCheckFailed) => std::process::exit(2),
        Err(error) => {
            output::print_error(
                output::ErrorResponse {
                    code: error.code,
                    message: error.message,
                    details: error.details,
                },
                &output_settings,
            );
            std::process::exit(1);
        }
    }
}

#[derive(Debug)]
pub(crate) struct CliRunError {
    code: &'static str,
    message: String,
    details: Option<String>,
}

impl CliRunError {
    fn invalid_args(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_args",
            message: message.into(),
            details: None,
        }
    }

    fn command(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub(crate) fn to_error_response(&self) -> output::ErrorResponse {
        output::ErrorResponse {
            code: self.code,
            message: self.message.clone(),
            details: self.details.clone(),
        }
    }
}

impl fmt::Display for CliRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.details {
            Some(details) => write!(f, "{}: {} ({details})", self.code, self.message),
            None => write!(f, "{}: {}", self.code, self.message),
        }
    }
}

impl Error for CliRunError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliRunOutcome {
    Success,
    DoctorCheckFailed,
}

fn run(cli: Cli, output_settings: output::OutputSettings) -> Result<CliRunOutcome, CliRunError> {
    match cli.command {
        Command::Doctor {
            check,
            require_status,
        } => {
            let check_request = check.map(|target| output::DoctorCheckRequest {
                target: target.into(),
                require_status: require_status
                    .unwrap_or(DoctorRequireStatusArg::Supported)
                    .into(),
            });
            let check_outcome = output::print_doctor(&output_settings, check_request);
            if check_request.is_some() && check_outcome == output::DoctorCheckOutcome::Failed {
                return Ok(CliRunOutcome::DoctorCheckFailed);
            }
        }
        Command::Info { path } => {
            let image = load_image(path)?;
            output::print_info(&image, &output_settings);
        }
        Command::Sections {
            path,
            executable_only,
            segment,
            name,
            sort,
        } => {
            let image = load_image(path)?;
            let mut sections = image.sections().to_vec();
            if executable_only {
                sections.retain(|section| section.executable);
            }
            if let Some(segment) = segment {
                sections
                    .retain(|section| contains_case_insensitive(&section.segment_name, &segment));
            }
            if let Some(name) = name {
                sections.retain(|section| contains_case_insensitive(&section.name, &name));
            }
            if let Some(sort) = sort {
                sort_sections(&mut sections, sort);
            }
            output::print_sections(&sections, &output_settings);
        }
        Command::Symbols {
            path,
            defined,
            global,
            section,
            name,
            sort,
        } => {
            let image = load_image(path)?;
            let mut symbols = image.symbols().to_vec();
            if defined {
                symbols.retain(|symbol| symbol.defined);
            }
            if global {
                symbols.retain(|symbol| symbol.global);
            }
            if let Some(section) = section {
                symbols.retain(|symbol| {
                    symbol
                        .section
                        .as_deref()
                        .is_some_and(|value| contains_case_insensitive(value, &section))
                });
            }
            if let Some(name) = name {
                symbols.retain(|symbol| contains_case_insensitive(&symbol.name, &name));
            }
            if let Some(sort) = sort {
                sort_symbols(&mut symbols, sort);
            }
            output::print_symbols(&symbols, &output_settings);
        }
        Command::Imports {
            path,
            dylib,
            name,
            lazy,
            weak,
            resolved,
            sort,
        } => {
            let image = load_image(path)?;
            let mut imports = image.imports().to_vec();
            if let Some(dylib) = dylib {
                imports.retain(|import| contains_case_insensitive(&import.dylib, &dylib));
            }
            if let Some(name) = name {
                imports.retain(|import| contains_case_insensitive(&import.name, &name));
            }
            if lazy {
                imports.retain(|import| import.is_lazy);
            }
            if weak {
                imports.retain(|import| import.is_weak);
            }
            if resolved {
                imports.retain(|import| import.address.is_some());
            }
            if let Some(sort) = sort {
                sort_imports(&mut imports, sort);
            }
            output::print_imports(&imports, &output_settings);
        }
        Command::Relocs {
            path,
            section,
            sort,
        } => {
            let image = load_image(path)?;
            let mut relocations = image.relocations().to_vec();
            if let Some(section) = section {
                relocations
                    .retain(|relocation| contains_case_insensitive(&relocation.section, &section));
            }
            if let Some(sort) = sort {
                sort_relocations(&mut relocations, sort);
            }
            output::print_relocations(&relocations, &output_settings);
        }
        Command::Dyld {
            path,
            dylibs,
            rpaths,
            exports,
            function_starts,
            bindings,
            stubs,
            helpers,
            name,
            dylib,
            source,
            binding_kind,
            stub_kind,
            export_kind,
            export_flag,
            ordinal,
            sort,
        } => {
            let image = load_image(path)?;
            let show_any =
                dylibs || rpaths || exports || function_starts || bindings || stubs || helpers;
            let mut view_options = if show_any {
                output::DyldViewOptions {
                    show_dylibs: dylibs,
                    show_rpaths: rpaths,
                    show_exports: exports,
                    show_function_starts: function_starts,
                    show_bindings: bindings,
                    show_stubs: stubs,
                    show_helpers: helpers,
                    name_filter: None,
                    dylib_filter: None,
                    source_filter: None,
                    binding_kind_filter: None,
                    stub_kind_filter: None,
                    export_kind_filter: None,
                    export_flag_filter: None,
                    ordinal_filter: None,
                    sort: None,
                }
            } else {
                output::DyldViewOptions::all()
            };
            view_options.name_filter = name;
            view_options.dylib_filter = dylib;
            view_options.source_filter = source.map(Into::into);
            view_options.binding_kind_filter = binding_kind.map(Into::into);
            view_options.stub_kind_filter = stub_kind.map(Into::into);
            view_options.export_kind_filter = export_kind.map(Into::into);
            view_options.export_flag_filter = export_flag.map(Into::into);
            view_options.ordinal_filter = ordinal;
            view_options.sort = sort.map(Into::into);
            output::print_dyld(&image, &view_options, &output_settings);
        }
        Command::Slices { path } => {
            let image = load_image(path)?;
            output::print_slices(&image, &output_settings);
        }
        Command::Objc {
            path,
            detail,
            owner,
            name_source,
            selector_source,
            category_source,
        } => {
            let image = load_image(path)?;
            output::print_objc(
                &image,
                &output::ObjcViewOptions {
                    detail: detail.into(),
                    owner_filter: owner,
                    name_source_filter: name_source.map(Into::into),
                    selector_source_filter: selector_source.map(Into::into),
                    category_source_filter: category_source.map(Into::into),
                },
                &output_settings,
            );
        }
        Command::Disasm {
            path,
            symbol,
            swift_symbol,
            addr,
            section,
            objc_owner,
            objc_selector,
            objc_method_kind,
            count,
            limit,
            bytes,
            from,
            to,
            no_annotations,
            show_references,
            show_values,
            analysis,
        } => {
            let image = load_image(path)?;
            let execution = execute_disassembly(
                &image,
                DisasmFlagArgs {
                    symbol,
                    swift_symbol,
                    addr,
                    section,
                    objc_owner,
                    objc_selector,
                    objc_method_kind,
                    count,
                    limit,
                    bytes,
                    from,
                    to,
                },
                !no_annotations,
                show_values || analysis,
            )?;

            let analysis = analysis.then(|| {
                analyze_disassembly_with_image(
                    &image,
                    execution.target.clone(),
                    execution.start_address,
                    execution.end_address,
                    &execution.instructions,
                )
            });

            output::print_disassembly(
                analysis
                    .as_ref()
                    .map(|analysis| execution.view_with_analysis(analysis))
                    .unwrap_or_else(|| execution.view()),
                output::DisassemblyRenderOptions {
                    include_annotations: !no_annotations,
                    include_references: show_references,
                    include_values: show_values,
                },
                &output_settings,
            );
        }
        Command::Ui { port, no_open } => {
            ui::run(ui::UiConfig {
                port,
                open_browser: !no_open,
            })?;
        }
        Command::Analyze { path, refresh } => {
            handle_analyze(path, refresh, &output_settings)?;
        }
        Command::Project { command } => handle_project_command(command, &output_settings)?,
        Command::Functions { project, name, limit } => {
            handle_functions(project, name, limit, &output_settings)?;
        }
        Command::Xrefs { project, to, from, kind, limit } => {
            handle_xrefs(project, to, from, kind, limit, &output_settings)?;
        }
        Command::Strings { project, name, class, min_len, limit } => {
            handle_strings(project, name, class, min_len, limit, &output_settings)?;
        }
        Command::Search { project, query, kind, limit } => {
            handle_search(project, &query, kind, limit, &output_settings)?;
        }
        Command::Summary { project, function, markdown } => {
            handle_summary(project, &function, markdown, &output_settings)?;
        }
        Command::Decompile { project, function, all, out } => {
            if all {
                handle_decompile_all(project, out, &output_settings)?;
            } else {
                let function = function.ok_or_else(|| {
                    CliRunError::invalid_args("decompile requires --function or --all")
                })?;
                handle_decompile(project, &function, &output_settings)?;
            }
        }
        Command::Recon { path } => {
            handle_recon(path, &output_settings)?;
        }
        Command::Batch { inputs, out, fail_fast } => {
            handle_batch(inputs, out, fail_fast, &output_settings)?;
        }
        Command::Diff { left, right, markdown, html } => {
            handle_diff(left, right, markdown, html, &output_settings)?;
        }
        Command::Serve { project } => {
            mcp::run_mcp(mcp::McpConfig { project })?;
        }
        Command::Cache { command } => handle_cache_command(command, &output_settings)?,
    }

    Ok(CliRunOutcome::Success)
}

fn analyzer_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn fresh_project_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{nanos:016x}-{}", std::process::id())
}

fn project_dir_for(target: &Path) -> PathBuf {
    let file_name = format!("{}.damsel", target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    target.with_file_name(file_name)
}

fn resolve_project_dir(arg: &Path) -> Result<PathBuf, CliRunError> {
    let has_store = |dir: &Path| dir.join("project.db").is_file();
    if arg.is_dir() && has_store(arg) {
        return Ok(arg.to_path_buf());
    }
    let adjacent = project_dir_for(arg);
    if adjacent.is_dir() && has_store(&adjacent) {
        return Ok(adjacent);
    }
    Err(CliRunError::command(
        "project_not_found",
        format!("no project database found for `{}`", arg.display()),
    ))
}

fn map_store_error(error: damsel_project::StoreError) -> CliRunError {
    match error {
        damsel_project::StoreError::AlreadyExists(path) => CliRunError::command(
            "project_exists",
            format!(
                "project already exists at `{}`; pass --refresh to rebuild",
                path.parent().map(|p| p.display().to_string()).unwrap_or_default()
            ),
        ),
        damsel_project::StoreError::UnsupportedSchema { path, found, supported } => CliRunError {
            code: "stale_schema",
            message: format!(
                "project at `{}` has schema version {found}; this damsel supports {supported}",
                path.display()
            ),
            details: None,
        },
        damsel_project::StoreError::ProjectNotFound(p) => {
            CliRunError::command("project_not_found", p)
        }
        other => CliRunError {
            code: "store_error",
            message: other.to_string(),
            details: None,
        },
    }
}

fn map_ingest_error(error: damsel_macho::IngestError) -> CliRunError {
    CliRunError::command("ingest_error", error.to_string())
}

fn project_json(value: serde_json::Value) -> String {
    serde_json::to_string(&value).unwrap_or_default()
}

fn handle_analyze(
    path: PathBuf,
    refresh: bool,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{AnalysisProjectMeta, ProjectId, ProjectStore, open_store};
    use damsel_macho::{harvest_strings, harvest_xrefs, recover_functions};

    // Format branch (Wave 5B.2): ELF ingests seeds only — engine passes
    // (xrefs/recovery/strings) wait on the x86_64 decode mapper.
    let is_elf = std::fs::read(&path)
        .map(|bytes| bytes.len() >= 4 && bytes[0..4] == *b"\x7fELF")
        .unwrap_or(false);
    if is_elf {
        return handle_analyze_elf(path, refresh, output_settings);
    }

    let image = load_image(path.clone())?;
    let project_dir = project_dir_for(&path);
    if project_dir.exists() && !refresh {
        return Err(CliRunError::command(
            "project_exists",
            format!(
                "project already exists at `{}`; pass --refresh to rebuild",
                project_dir.display()
            ),
        ));
    }
    if refresh && project_dir.exists() {
        std::fs::remove_dir_all(&project_dir).map_err(|e| CliRunError::command(
            "store_error",
            format!("failed to remove {}: {e}", project_dir.display()),
        ))?;
    }
    let name = path
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string());
    let meta = AnalysisProjectMeta {
        project_id: ProjectId::new(fresh_project_id()),
        name,
        analyzer_version: analyzer_version().to_string(),
    };
    let db_path = project_dir.join("project.db");
    let mut store = open_store(&db_path, damsel_project::Create::New, &meta)
        .map_err(map_store_error)?;
    let outcome = ingest_binary_image(&mut store, &image, analyzer_version())
        .map_err(map_ingest_error)?;
    // Xref pass over seeded function windows (Wave 4 §4 pass 4–5).
    let function_addresses: Vec<u64> = damsel_project::function_summaries(&store)
        .map_err(map_store_error)?
        .iter()
        .map(|f| f.address)
        .collect();
    let xref_outcome = harvest_xrefs(
        &mut store,
        &image,
        outcome.source_id,
        &function_addresses,
        analyzer_version(),
    )
    .map_err(|e| CliRunError::command("xref_error", e.to_string()))?;
    let recovery_outcome = recover_functions(
        &mut store,
        &image,
        outcome.source_id,
        &function_addresses,
        analyzer_version(),
    )
    .map_err(|e| CliRunError::command("recovery_error", e.to_string()))?;
    let strings_outcome = harvest_strings(
        &mut store,
        &image,
        outcome.source_id,
        analyzer_version(),
    )
    .map_err(|e| CliRunError::command("strings_error", e.to_string()))?;
    let source_count = store.sources().map_err(map_store_error)?.len();
    let store_meta = store.meta().map_err(map_store_error)?;
    drop(store);

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "analyze",
            "project": store_meta,
            "project_path": project_dir.display().to_string(),
            "sources": source_count,
            "objects": outcome.object_count,
            "facts": outcome.fact_count,
            "function_windows": xref_outcome.function_windows,
            "xrefs": xref_outcome.xref_count,
            "function_bounds": recovery_outcome.bounds,
            "rank6_seeded": recovery_outcome.rank6_seeded,
            "recovery_problems": recovery_outcome.problems,
            "strings": strings_outcome.strings,
        });
        println!("{}", project_json(value));
    } else {
        println!("analyzed -> {}", project_dir.display());
        println!("  project_id  = {}", store_meta.project_id);
        println!("  sources     = {source_count}");
        println!("  objects     = {}", outcome.object_count);
        println!("  facts       = {}", outcome.fact_count);
        println!("  xrefs       = {} ({} function windows)", xref_outcome.xref_count, xref_outcome.function_windows);
        println!("  bounds      = {} ({} rank-6, {} problems)", recovery_outcome.bounds, recovery_outcome.rank6_seeded, recovery_outcome.problems);
        println!("  strings     = {}", strings_outcome.strings);
    }
    Ok(())
}

fn handle_batch(
    inputs: PathBuf,
    out_dir: PathBuf,
    fail_fast: bool,
    _output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{AnalysisProjectMeta, Create, ProjectId, ProjectStore, TypedStore, open_store};

    let list = std::fs::read_to_string(&inputs)
        .map_err(|e| CliRunError::command("invalid_args", format!("{}: {e}", inputs.display())))?;
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| CliRunError::command("store_error", format!("{}: {e}", out_dir.display())))?;
    let results_path = out_dir.join("batch.jsonl");
    let mut results = std::fs::File::create(&results_path)
        .map_err(|e| CliRunError::command("store_error", format!("{}: {e}", results_path.display())))?;

    use std::io::Write;
    let mut passed = 0usize;
    let mut failed = 0usize;
    for line in list.lines().filter(|l| !l.trim().is_empty()) {
        let path = std::path::PathBuf::from(line.trim());
        let entry = match run_batch_target(&path, &out_dir) {
            Ok(entry) => {
                passed += 1;
                entry
            }
            Err(entry) => {
                failed += 1;
                if fail_fast {
                    let _ = writeln!(results, "{}", serde_json::to_string(&entry).unwrap_or_default());
                    return Err(CliRunError::command(
                        "batch_failed",
                        format!("{} failed (fail-fast)", path.display()),
                    ));
                }
                entry
            }
        };
        let _ = writeln!(results, "{}", serde_json::to_string(&entry).unwrap_or_default());
    }

    println!(
        "batch: {passed} passed, {failed} failed -> {}",
        results_path.display()
    );
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn run_batch_target(path: &std::path::Path, out_dir: &std::path::Path) -> Result<serde_json::Value, serde_json::Value> {
    let name = path
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "target".into());

    let is_elf = std::fs::read(path)
        .map(|bytes| bytes.len() >= 4 && bytes[0..4] == *b"\x7fELF")
        .unwrap_or(false);

    let project_dir = out_dir.join(format!("{name}.damsel"));
    let _ = std::fs::remove_dir_all(&project_dir);
    let meta = damsel_project::AnalysisProjectMeta {
        project_id: damsel_project::ProjectId::new(format!("batch-{name}")),
        name: name.clone(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let db_path = project_dir.join("project.db");
    let Ok(mut store) = damsel_project::open_store(&db_path, damsel_project::Create::New, &meta) else {
        return Err(serde_json::json!({"input": line_of(path), "status": "error", "error": "store_create_failed"}));
    };

    let outcome = if is_elf {
        match damsel_elf::load(path) {
            Ok(image) => damsel_elf::ingest_elf_image(&mut store, &image, env!("CARGO_PKG_VERSION"))
                .map(|o| (o.source_id, o.fact_count))
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        match damsel_macho::load(path) {
            Ok(image) => damsel_macho::ingest_binary_image(&mut store, &image, env!("CARGO_PKG_VERSION"))
                .map(|o| (o.source_id, o.fact_count))
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    };

    match outcome {
        Ok((source_id, fact_count)) => Ok(serde_json::json!({
            "input": line_of(path),
            "status": "ok",
            "target": name,
            "format": if is_elf { "elf" } else { "macho" },
            "facts": fact_count,
        })),
        Err(error) => Err(serde_json::json!({
            "input": line_of(path),
            "status": "error",
            "error": error,
        })),
    }
}

fn line_of(path: &std::path::Path) -> String {
    path.display().to_string()
}

fn handle_diff(
    left: PathBuf,
    right: PathBuf,
    markdown: bool,
    html: bool,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, open_store};

    let left_dir = resolve_project_dir(&left)?;
    let right_dir = resolve_project_dir(&right)?;
    let meta_l = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let meta_r = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let left_store = open_store(&left_dir.join("project.db"), Create::Existing, &meta_l)
        .map_err(map_store_error)?;
    let right_store = open_store(&right_dir.join("project.db"), Create::Existing, &meta_r)
        .map_err(map_store_error)?;
    let left_name = left_store.meta().map_err(map_store_error)?.name;
    let right_name = right_store.meta().map_err(map_store_error)?.name;

    let report = damsel_decomp::diff_functions(&left_store, &right_store, &left_name, &right_name)
        .map_err(|e| CliRunError::command("diff_error", e.to_string()))?;

    if markdown {
        print!("{}", damsel_decomp::render_diff_markdown(&report));
        return Ok(());
    }
    if html {
        print!("{}", damsel_decomp::render_diff_html(&report));
        return Ok(());
    }

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "diff",
            "report": serde_json::to_value(&report).unwrap_or_default(),
        });
        println!("{}", project_json(value));
    } else {
        println!("diff: {} -> {}", report.left_name, report.right_name);
        println!("  matched: {}", report.matched.len());
        for m in &report.matched {
            let name = m.name.as_deref().unwrap_or("?");
            let strategy = match m.strategy {
                damsel_decomp::MatchStrategy::ExactSymbol => "symbol",
                damsel_decomp::MatchStrategy::ExactAddress => "address",
                damsel_decomp::MatchStrategy::Structural => "structural",
            };
            println!("  = {name:<32} {:#010x} -> {:#010x} ({strategy})", m.left_address, m.right_address);
        }
        println!("  added:   {}", report.added.len());
        for f in &report.added {
            let name = f.name.as_deref().unwrap_or("?");
            println!("  + {name:<32} {:#010x}", f.address);
        }
        println!("  removed: {}", report.removed.len());
        for f in &report.removed {
            let name = f.name.as_deref().unwrap_or("?");
            println!("  - {name:<32} {:#010x}", f.address);
        }
    }
    Ok(())
}

fn handle_recon(path: PathBuf, output_settings: &output::OutputSettings) -> Result<(), CliRunError> {
    use damsel_decomp::{Verdict, assess, shannon_entropy};

    let is_elf = std::fs::read(&path)
        .map(|bytes| bytes.len() >= 4 && bytes[0..4] == *b"\x7fELF")
        .unwrap_or(false);

    let (symbol_count, sections, imports, strings) = if is_elf {
        let image = damsel_elf::load(&path)
            .map_err(|e| CliRunError::command("unsupported_input", e.to_string()))?;
        let sections: Vec<(String, Vec<u8>)> = image
            .sections
            .iter()
            .filter_map(|s| {
                image
                    .read_at_vaddr(s.address, s.size.min(4096) as usize)
                    .map(|b| (s.name.clone(), b.to_vec()))
            })
            .collect();
        let imports: Vec<String> = image
            .symbols
            .iter()
            .filter(|s| s.is_import)
            .map(|s| s.name.clone())
            .collect();
        let strings: Vec<String> = Vec::new(); // string harvest needs a project
        (image.symbols.len(), sections, imports, strings)
    } else {
        let image = load_image(path)?;
        let sections: Vec<(String, Vec<u8>)> = image
            .sections()
            .iter()
            .filter_map(|s| {
                image
                    .bytes_for_file_range(s.file_offset?, s.file_size.min(4096))
                    .map(|b| (s.name.clone(), b.to_vec()))
            })
            .collect();
        let imports: Vec<String> = image.imports().iter().map(|i| i.name.clone()).collect();
        let strings: Vec<String> = Vec::new();
        (image.symbols().len(), sections, imports, strings)
    };

    let report = assess(symbol_count, &sections, &imports, &strings);

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "recon",
            "report": serde_json::to_value(&report).unwrap_or_default(),
        });
        println!("{}", project_json(value));
    } else {
        let verdict = match report.verdict {
            Verdict::Clean => "clean",
            Verdict::LikelyPacked => "LIKELY PACKED",
            Verdict::LikelyObfuscated => "LIKELY OBFUSCATED",
            Verdict::NeedsManualReview => "needs manual review",
        };
        println!("recon: {verdict}");
        println!("  symbols        {symbol_count}{}", if report.stripped { " (stripped)" } else { "" });
        println!("  note           for CFG-flattening and opaque-predicate detection, run `damsel analyze` + `damsel decompile`");
        for se in &report.section_entropies {
            let bar = "█".repeat((se.entropy * 8.0) as usize);
            println!("  {:<16} {:.2} {}", se.section, se.entropy, bar);
        }
        for packer in &report.packer_signatures {
            println!("  packer: {} ({})", packer.name, packer.evidence);
        }
        for indicator in &report.anti_analysis_indicators {
            println!("  indicator: {} — {}", indicator.name, indicator.evidence);
        }
    }
    Ok(())
}

fn handle_decompile(
    project: PathBuf,
    selector: &str,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    // v1 lift path: Mach-O arm64 via the engine's symbol/addr windows.
    // Full project-fact-driven lifting (both formats) follows with D2/D4.
    let target_path = resolve_project_dir(&project)
        .ok()
        .and_then(|dir| {
            dir.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".damsel"))
                .map(|stem| dir.with_file_name(stem))
        });
    let Some(target) = target_path.filter(|p| p.is_file()) else {
        return Err(CliRunError::command(
            "project_not_found",
            "decompile requires the target binary next to the project",
        ));
    };
    // ELF path (Wave 6.5): decode via the x86_64 mapper inside the
    // function's recovered bound; Mach-O keeps the engine path.
    let is_elf = std::fs::read(&target)
        .map(|bytes| bytes.len() >= 4 && bytes[0..4] == *b"\x7fELF")
        .unwrap_or(false);
    if is_elf {
        return handle_decompile_elf(project, selector, output_settings);
    }

    let image = load_image(target)?;
    let request = damsel_core::DisassemblyRequestV2 {
        target: if let Some(hex) = selector.strip_prefix("0x") {
            damsel_core::DisassemblyTarget::Address(
                u64::from_str_radix(hex, 16)
                    .map_err(|_| CliRunError::invalid_args("bad address"))?,
            )
        } else {
            damsel_core::DisassemblyTarget::Symbol(selector.to_string())
        },
        range: None,
        limit: damsel_core::DisassemblyLimit::Unlimited,
        options: damsel_core::DisassemblyOptions {
            include_annotations: true,
            include_value_flow: false,
        },
    };
    let result = disassemble_v2(&image, &request)
        .map_err(map_macho_error)?;
    if result.instructions.is_empty() {
        return Err(CliRunError::command("function_not_found", selector.to_string()));
    }
    let function = damsel_decomp::lift_arm64(&result.instructions);
    let pseudocode = damsel_decomp::render_pseudocode(&function, Some(selector));

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "decompile",
            "pseudocode": serde_json::to_value(&pseudocode).unwrap_or_default(),
            "frame": serde_json::to_value(damsel_decomp::analyze_stack_frame(&function)).unwrap_or_default(),
            "unknown_count": function.unknown_count,
        });
        println!("{}", project_json(value));
    } else {
        for line in &pseudocode.lines {
            let addresses = line
                .source_addresses
                .iter()
                .map(|a| format!("{a:#x}"))
                .collect::<Vec<_>>()
                .join(",");
            println!("{:>10}  {}", if addresses.is_empty() { String::new() } else { addresses }, line.text);
        }
    }
    Ok(())
}

fn handle_decompile_all(
    project: PathBuf,
    out_dir: Option<PathBuf>,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, TypedStore, open_store};

    let project_dir = resolve_project_dir(&project)?;
    let meta_for_open = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
        .map_err(map_store_error)?;

    let summaries = damsel_project::function_summaries(&store)
        .map_err(map_store_error)?;

    let Some(out) = out_dir else {
        // Print all to stdout (JSONL)
        for summary in &summaries {
            if let Some(name) = &summary.name {
                let target_path = project_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_suffix(".damsel"))
                    .map(|stem| project_dir.with_file_name(stem));
                if let Some(target) = target_path.filter(|p| p.is_file()) {
                    let image = load_image(target)?;
                    let request = damsel_core::DisassemblyRequestV2 {
                        target: damsel_core::DisassemblyTarget::Symbol(name.clone()),
                        range: None,
                        limit: damsel_core::DisassemblyLimit::Unlimited,
                        options: damsel_core::DisassemblyOptions {
                            include_annotations: true,
                            include_value_flow: false,
                        },
                    };
                    if let Ok(result) = disassemble_v2(&image, &request) {
                        let function = damsel_decomp::lift_arm64(&result.instructions);
                        let pseudocode = damsel_decomp::render_pseudocode(&function, Some(name));
                        let value = serde_json::json!({
                            "function": name,
                            "address": format!("{:#x}", summary.address),
                            "unknown_count": function.unknown_count,
                            "line_count": pseudocode.lines.len(),
                            "lines": pseudocode.lines.iter().map(|l| &l.text).collect::<Vec<_>>(),
                        });
                        println!("{}", project_json(value));
                    }
                }
            }
        }
        return Ok(());
    };

    // Write one .pseudo file per function
    std::fs::create_dir_all(&out)
        .map_err(|e| CliRunError::command("store_error", format!("{}: {e}", out.display())))?;
    let mut count = 0;
    for summary in &summaries {
        let Some(name) = &summary.name else { continue };
        let target_path = project_dir
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".damsel"))
            .map(|stem| project_dir.with_file_name(stem));
        let Some(target) = target_path.filter(|p| p.is_file()) else { continue };
        let image = load_image(target)?;
        let request = damsel_core::DisassemblyRequestV2 {
            target: damsel_core::DisassemblyTarget::Symbol(name.clone()),
            range: None,
            limit: damsel_core::DisassemblyLimit::Unlimited,
            options: damsel_core::DisassemblyOptions {
                include_annotations: true,
                include_value_flow: false,
            },
        };
        let Ok(result) = disassemble_v2(&image, &request) else { continue };
        let function = damsel_decomp::lift_arm64(&result.instructions);
        let pseudocode = damsel_decomp::render_pseudocode(&function, Some(name));
        let safe_name = name.replace(char::from(47), "_");
        let file_path = out.join(format!("{safe_name}.pseudo"));
        let text: Vec<&str> = pseudocode.lines.iter().map(|l| l.text.as_str()).collect();
        std::fs::write(&file_path, text.join("\n"))
            .map_err(|e| CliRunError::command("store_error", format!("{}: {e}", file_path.display())))?;
        count += 1;
    }
    println!("decompiled {count} functions -> {}", out.display());
    Ok(())
}

fn handle_decompile_elf(
    project: PathBuf,
    selector: &str,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, TypedStore, open_store};

    let project_dir = resolve_project_dir(&project)?;
    let target_path = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".damsel"))
        .map(|stem| project_dir.with_file_name(stem));
    let Some(target_path) = target_path.filter(|p| p.is_file()) else {
        return Err(CliRunError::command(
            "project_not_found",
            "decompile requires the target binary next to the project",
        ));
    };
    let image = damsel_elf::load(&target_path)
        .map_err(|e| CliRunError::command("unsupported_input", e.to_string()))?;

    let meta_for_open = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
        .map_err(map_store_error)?;
    let source = store
        .sources()
        .map_err(map_store_error)?
        .first()
        .ok_or_else(|| CliRunError::command("project_not_found", "no sources"))?
        .clone();

    // Resolve the function (by name or address) and its bound.
    let summaries = damsel_project::function_summaries(&store).map_err(map_store_error)?;
    let summary = summaries
        .iter()
        .find(|f| {
            f.name.as_deref() == Some(selector)
                || format!("{:#x}", f.address) == selector
        })
        .ok_or_else(|| CliRunError::command("function_not_found", selector.to_string()))?;

    let facts = store.facts_for_source(source.source_id).map_err(map_store_error)?;
    let bound = facts
        .iter()
        .find(|f| {
            f.kind.as_str() == "function_bound"
                && f.object_id
                    .as_ref()
                    .is_some_and(|o| o.key() == format!("{:#x}", summary.address))
        })
        .and_then(|f| {
            let (range, _) = f.payload.split_once("|stop=")?;
            let (start, end) = range.split_once('-')?;
            Some((
                u64::from_str_radix(start.trim_start_matches("0x"), 16).ok()?,
                u64::from_str_radix(end.trim_start_matches("0x"), 16).ok()?,
            ))
        })
        .unwrap_or((summary.address, summary.address));

    let length = (bound.1 - bound.0) as usize;
    let Some(bytes) = image.read_at_vaddr(bound.0, length) else {
        return Err(CliRunError::command(
            "function_not_found",
            "function bytes unreadable",
        ));
    };
    let instructions = damsel_core::decode_x86_64(bytes, bound.0, None)
        .map_err(|e| CliRunError::command("engine_error", e.to_string()))?;
    let function = damsel_decomp::lift_x86_64(&instructions);
    let pseudocode = damsel_decomp::render_pseudocode(&function, Some(selector));

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "decompile",
            "pseudocode": serde_json::to_value(&pseudocode).unwrap_or_default(),
            "unknown_count": function.unknown_count,
        });
        println!("{}", project_json(value));
    } else {
        for line in &pseudocode.lines {
            println!("{}", line.text);
        }
    }
    Ok(())
}

fn handle_summary(
    project: PathBuf,
    selector: &str,
    markdown: bool,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, TypedStore, open_store};

    let project_dir = resolve_project_dir(&project)?;
    let meta_for_open = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
        .map_err(map_store_error)?;
    let sources = store.sources().map_err(map_store_error)?;
    let Some(source) = sources.first() else {
        return Err(CliRunError::command("project_not_found", "project has no sources"));
    };
    let report = damsel_decomp::summarize_named(&store, source.source_id, selector)
        .map_err(|e| CliRunError::command("function_not_found", e.to_string()))?;

    if markdown {
        print!("{}", damsel_decomp::render_summary_markdown(&report));
        return Ok(());
    }
    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "summary",
            "summary": serde_json::to_value(&report).unwrap_or_default(),
        });
        println!("{}", project_json(value));
    } else {
        println!("{} ({})", report.function, report.name.as_deref().unwrap_or(""));
        if let Some(bound) = &report.bound {
            println!("  bound   {bound}");
        }
        println!("  seeds   [{}]", report.seeds.join(","));
        for call in &report.calls {
            let flag = if call.unresolved { " (unresolved)" } else { "" };
            println!("  {}  {} -> {}{}", call.from, call.kind, call.target, flag);
        }
        for import in &report.imports {
            println!("  import  {import}");
        }
        for string in &report.strings {
            println!("  string  {string}");
        }
        for problem in &report.problems {
            println!("  problem {problem}");
        }
    }
    Ok(())
}

fn handle_search(
    project: PathBuf,
    query: &str,
    kind_filter: Option<String>,
    limit: Option<usize>,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, function_summaries, open_store};

    let project_dir = resolve_project_dir(&project)?;
    let meta_for_open = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
        .map_err(map_store_error)?;
    let facts = store.facts().map_err(map_store_error)?;

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    struct Row {
        rank: u8,        // 0 exact, 1 prefix, 2 substring
        kind_priority: u8,
        name: String,
        kind: &'static str,
        object: String,
        address: Option<u64>,
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut push = |rank: u8, kind_priority: u8, kind: &'static str, name: &str, object: String, address: Option<u64>| {
        if let Some(want) = &kind_filter {
            if want != kind {
                return;
            }
        }
        let rank = if name == query {
            0
        } else if name.starts_with(query) {
            rank.min(1)
        } else if name.contains(query) {
            2
        } else {
            return;
        };
        rows.push(Row {
            rank,
            kind_priority,
            name: name.to_string(),
            kind,
            object,
            address,
        });
    };

    // Functions (kind priority 0) from seed summaries.
    for f in function_summaries(&store).map_err(map_store_error)? {
        if let Some(name) = &f.name {
            push(u8::MAX, 0, "function", name, format!("function:{:#x}", f.address), Some(f.address));
        }
    }
    // Symbols (1), imports (2), sections (3) from fact objects.
    for fact in &facts {
        let Some(object) = &fact.object_id else { continue };
        match object.kind() {
            "symbol" if fact.kind.as_str() == "symbol" => {
                push(u8::MAX, 1, "symbol", object.key(), object.as_str().to_string(), None);
            }
            "import" if fact.kind.as_str() == "import" => {
                push(u8::MAX, 2, "import", object.key(), object.as_str().to_string(), None);
            }
            "section" if fact.kind.as_str() == "byte_class" => {
                push(u8::MAX, 3, "section", object.key(), object.as_str().to_string(), None);
            }
            _ => {}
        }
    }
    // Strings (4): searchable by value when the target binary is present.
    let target_path = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".damsel"))
        .map(|stem| project_dir.with_file_name(stem));
    let elf_image = target_path
        .as_deref()
        .filter(|p| p.is_file())
        .and_then(|p| damsel_elf::load(p.clone()).ok());
    let image = if elf_image.is_some() { None } else {
        target_path
            .as_deref()
            .filter(|p| p.is_file())
            .and_then(|p| load(p.clone()).ok())
    };
    let strings_searchable = image.is_some();
    if let Some(image) = &image {
        for fact in &facts {
            if fact.kind.as_str() != "string" {
                continue;
            }
            let Some(object) = &fact.object_id else { continue };
            let Ok(address) =
                u64::from_str_radix(object.key().trim_start_matches("0x"), 16)
            else {
                continue;
            };
            let (_, len_part) = fact.payload.split_once("|len=").unwrap_or(("utf8", "0"));
            let len: usize = len_part.parse().unwrap_or(0);
            if let Some(value) = image.read_c_string_at_address(address, len.saturating_add(1)) {
                push(u8::MAX, 4, "string", &value, object.as_str().to_string(), Some(address));
            }
        }
    }

    rows.sort();
    rows.dedup_by(|a, b| a.object == b.object && a.rank == b.rank);
    let total = rows.len();
    let truncated = limit.is_some_and(|l| rows.len() > l);
    if let Some(limit) = limit {
        rows.truncate(limit);
    }

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "search",
            "query": query,
            "count": total,
            "truncated": truncated,
            "strings_searchable": strings_searchable,
            "results": rows.iter().map(|r| serde_json::json!({
                "kind": r.kind,
                "name": r.name,
                "id": r.object,
                "address": r.address.map(|a| format!("{a:#x}")),
            })).collect::<Vec<_>>(),
        });
        println!("{}", project_json(value));
    } else {
        println!("search `{query}`: {total}");
        for r in &rows {
            let address = r
                .address
                .map(|a| format!("{a:#010x}"))
                .unwrap_or_else(|| "          ".to_string());
            println!("  {address}  {:<8} {}", r.kind, r.name);
        }
    }
    Ok(())
}

fn handle_strings(
    project: PathBuf,
    name_filter: Option<String>,
    class_filter: Option<String>,
    min_len: Option<usize>,
    limit: Option<usize>,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, open_store};

    let project_dir = resolve_project_dir(&project)?;
    let meta_for_open = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
        .map_err(map_store_error)?;
    let facts = store.facts().map_err(map_store_error)?;

    // String facts: object `string:<addr>`, payload `utf8|len=N`.
    // Class facts attach to the same objects.
    let mut classes: std::collections::BTreeMap<String, String> = Default::default();
    for fact in &facts {
        if fact.kind.as_str() == "string_class" {
            if let Some(object) = &fact.object_id {
                classes.insert(object.as_str().to_string(), fact.payload.clone());
            }
        }
    }
    let mut rows: Vec<(u64, &str, usize, Option<&str>)> = Vec::new();
    for fact in &facts {
        if fact.kind.as_str() != "string" {
            continue;
        }
        let Some(object) = &fact.object_id else { continue };
        let Ok(address) =
            u64::from_str_radix(object.key().trim_start_matches("0x"), 16)
        else {
            continue;
        };
        let (encoding, len_part) = fact.payload.split_once("|len=").unwrap_or(("utf8", "0"));
        let byte_len: usize = len_part.parse().unwrap_or(0);
        let class = classes.get(object.as_str()).map(String::as_str);
        if let Some(want) = &class_filter {
            if class != Some(want.as_str()) {
                continue;
            }
        }
        rows.push((address, encoding, byte_len, class));
    }
    rows.sort();

    // Value display requires the target binary (bytes stay out of the
    // project by privacy design). Resolve it best-effort.
    let target_path = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".damsel"))
        .map(|stem| project_dir.with_file_name(stem));
    let elf_image = target_path
        .as_deref()
        .filter(|p| p.is_file())
        .and_then(|p| damsel_elf::load(p.clone()).ok());
    let image = if elf_image.is_some() { None } else {
        target_path
            .as_deref()
            .filter(|p| p.is_file())
            .and_then(|p| load(p.clone()).ok())
    };

    let mut display: Vec<(u64, &str, usize, Option<&str>, Option<String>)> = rows
        .into_iter()
        .filter_map(|(address, encoding, len, class)| {
            let value = image
                .as_ref()
                .and_then(|img| img.read_c_string_at_address(address, len.saturating_add(1)))
                .or_else(|| {
                    elf_image
                        .as_ref()
                        .and_then(|img| damsel_elf::read_elf_c_string(img, address, len + 1))
                });
            if let Some(min) = min_len {
                let chars = value.as_deref().map(|v| v.chars().count()).unwrap_or(len);
                if chars < min {
                    return None;
                }
            }
            if let Some(pattern) = &name_filter {
                let Some(v) = &value else { return None };
                if !v.contains(pattern.as_str()) {
                    return None;
                }
            }
            Some((address, encoding, len, class, value))
        })
        .collect();
    let total = display.len();
    let truncated = limit.is_some_and(|l| display.len() > l);
    if let Some(limit) = limit {
        display.truncate(limit);
    }

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "strings",
            "count": total,
            "truncated": truncated,
            "values_available": image.is_some(),
            "strings": display.iter().map(|(address, encoding, len, class, value)| {
                serde_json::json!({
                    "address": format!("{address:#x}"),
                    "encoding": encoding,
                    "bytes": len,
                    "class": class,
                    "value": value,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", project_json(value));
    } else {
        println!("strings: {total}");
        for (address, encoding, _len, class, value) in &display {
            let class_text = class.unwrap_or("");
            let value_text = value.as_deref().unwrap_or("");
            println!("  {address:#010x}  {encoding:<8} {class_text:<12} {value_text}");
        }
    }
    Ok(())
}

fn handle_xrefs(
    project: PathBuf,
    to: Option<u64>,
    from: Option<u64>,
    kind_filter: Option<String>,
    limit: Option<usize>,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, function_summaries, open_store};

    if to.is_none() && from.is_none() {
        return Err(CliRunError::invalid_args(
            "xrefs requires --to <addr> or --from <addr>",
        ));
    }
    let project_dir = resolve_project_dir(&project)?;
    let meta_for_open = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
        .map_err(map_store_error)?;

    let names: std::collections::BTreeMap<u64, String> = function_summaries(&store)
        .map_err(map_store_error)?
        .into_iter()
        .filter_map(|f| f.name.map(|n| (f.address, n)))
        .collect();

    let facts = store.facts().map_err(map_store_error)?;
    let mut rows: Vec<(u64, String, String)> = Vec::new(); // (from_addr, kind, to_object)
    for fact in &facts {
        if fact.kind.as_str() != "xref" {
            continue;
        }
        let Some(object) = &fact.object_id else { continue };
        if object.kind() != "instruction" {
            continue;
        }
        let Some(from_addr) =
            u64::from_str_radix(object.key().trim_start_matches("0x"), 16).ok()
        else {
            continue;
        };
        let Some((kind, target)) = fact.payload.split_once("|to=") else {
            continue;
        };
        if let Some(want) = from {
            if from_addr != want {
                continue;
            }
        }
        if let Some(want_addr) = to {
            // Match any target object whose address component equals the
            // requested address (function/data/stub objects).
            let Some((_, key)) = target.split_once(':') else { continue };
            let Ok(target_addr) = u64::from_str_radix(key.trim_start_matches("0x"), 16) else {
                continue;
            };
            if target_addr != want_addr {
                continue;
            }
        }
        if let Some(kind_want) = &kind_filter {
            if kind != kind_want.as_str() {
                continue;
            }
        }
        rows.push((from_addr, kind.to_string(), target.to_string()));
    }
    rows.sort();
    rows.dedup();
    let total = rows.len();
    let truncated = limit.is_some_and(|l| rows.len() > l);
    if let Some(limit) = limit {
        rows.truncate(limit);
    }

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "xrefs",
            "count": total,
            "truncated": truncated,
            "xrefs": rows.iter().map(|(from, kind, target)| {
                // v1 attribution: nearest seeded function at or below the
                // call site (true containment lands with recovery bounds).
                let from_function = names
                    .range(..=*from)
                    .next_back()
                    .map(|(_, name)| name.clone());
                serde_json::json!({
                    "from": format!("{from:#x}"),
                    "from_function": from_function,
                    "kind": kind,
                    "to": target,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", project_json(value));
    } else {
        println!("xrefs: {total}");
        for (from, kind, target) in &rows {
            println!("  {from:#010x}  {kind:<12} -> {target}");
        }
    }
    Ok(())
}

fn handle_functions(
    project: PathBuf,
    name_filter: Option<String>,
    limit: Option<usize>,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, function_summaries, open_store};

    let project_dir = resolve_project_dir(&project)?;
    let meta_for_open = AnalysisProjectMetaShim::meta_for_open(analyzer_version());
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
        .map_err(map_store_error)?;
    let mut summaries = function_summaries(&store).map_err(map_store_error)?;
    if let Some(pattern) = &name_filter {
        summaries.retain(|f| {
            f.name
                .as_deref()
                .is_some_and(|n| n.contains(pattern.as_str()))
        });
    }
    let total = summaries.len();
    let truncated = limit.is_some_and(|l| summaries.len() > l);
    if let Some(limit) = limit {
        summaries.truncate(limit);
    }

    // Join stored bounds (recovery_v1) onto summaries by address.
    let bound_by_address: std::collections::BTreeMap<u64, String> = store
        .facts()
        .map_err(map_store_error)?
        .iter()
        .filter_map(|fact| {
            if fact.kind.as_str() != "function_bound" {
                return None;
            }
            let object = fact.object_id.as_ref()?;
            let address = u64::from_str_radix(object.key().trim_start_matches("0x"), 16).ok()?;
            Some((address, fact.payload.clone()))
        })
        .collect();

    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "functions",
            "count": total,
            "truncated": truncated,
            "functions": summaries.iter().map(|f| serde_json::json!({
                "address": format!("{:#x}", f.address),
                "name": f.name,
                "seeds": f.seeds,
                "bound": bound_by_address.get(&f.address),
            })).collect::<Vec<_>>(),
        });
        println!("{}", project_json(value));
    } else {
        println!("functions: {total}");
        for f in &summaries {
            let name = f.name.as_deref().unwrap_or("");
            let bound = bound_by_address
                .get(&f.address)
                .map(|b| b.as_str())
                .unwrap_or("-");
            println!("  {:#010x}  {:<24} [{:<28}] {}", f.address, name, f.seeds.join(","), bound);
        }
    }
    Ok(())
}

fn handle_analyze_elf(
    path: PathBuf,
    refresh: bool,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{AnalysisProjectMeta, ProjectId, ProjectStore, open_store};

    let image = damsel_elf::load(&path)
        .map_err(|e| CliRunError::command("unsupported_input", e.to_string()))?;
    let project_dir = project_dir_for(&path);
    if project_dir.exists() && !refresh {
        return Err(CliRunError::command(
            "project_exists",
            format!(
                "project already exists at `{}`; pass --refresh to rebuild",
                project_dir.display()
            ),
        ));
    }
    if refresh && project_dir.exists() {
        std::fs::remove_dir_all(&project_dir).map_err(|e| {
            CliRunError::command("store_error", format!("{}: {e}", project_dir.display()))
        })?;
    }
    let name = path
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string());
    let meta = AnalysisProjectMeta {
        project_id: ProjectId::new(fresh_project_id()),
        name,
        analyzer_version: analyzer_version().to_string(),
    };
    let mut store = open_store(&project_dir.join("project.db"), damsel_project::Create::New, &meta)
        .map_err(map_store_error)?;
    let outcome = damsel_elf::ingest_elf_image(&mut store, &image, analyzer_version())
        .map_err(|e| CliRunError::command("ingest_error", e.to_string()))?;
    // Engine passes for ELF (xrefs + bounds via the x86_64 decoder).
    let seed_addresses: Vec<u64> = damsel_project::function_summaries(&store)
        .map_err(map_store_error)?
        .iter()
        .map(|f| f.address)
        .collect();
    let engine = damsel_elf::harvest_elf_xrefs(
        &mut store,
        &image,
        outcome.source_id,
        &seed_addresses,
        analyzer_version(),
    )
    .map_err(|e| CliRunError::command("engine_error", e.to_string()))?;
    let strings_outcome = damsel_elf::harvest_elf_strings(
        &mut store,
        &image,
        outcome.source_id,
        analyzer_version(),
    )
    .map_err(|e| CliRunError::command("strings_error", e.to_string()))?;
    let source_count = store.sources().map_err(map_store_error)?.len();
    drop(store);
    if output_settings.format == output::OutputFormat::Json {
        let value = serde_json::json!({
            "command": "analyze",
            "project": meta,
            "project_path": project_dir.display().to_string(),
            "sources": source_count,
            "objects": outcome.object_count,
            "facts": outcome.fact_count,
            "xrefs": engine.xref_count,
            "function_bounds": engine.bounds,
            "strings": strings_outcome.strings,
        });
        println!("{}", project_json(value));
    } else {
        println!("analyzed -> {}", project_dir.display());
        println!("  project_id  = {}", meta.project_id);
        println!(
            "  arch        = {} (ELF, {})",
            image.architecture,
            if image.is_position_independent { "PIE" } else { "static" }
        );
        if let Some(soname) = &image.soname {
            println!("  soname      = {soname}");
        }
        println!("  objects     = {}", outcome.object_count);
        println!("  facts       = {}", outcome.fact_count);
        println!("  xrefs       = {} ({} function bounds)", engine.xref_count, engine.bounds);
        println!("  strings     = {}", strings_outcome.strings);
    }
    Ok(())
}

fn handle_project_command(
    command: ProjectCommand,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    use damsel_project::{Create, ProjectStore, open_store};

    let version = analyzer_version().to_string();
    match command {
        ProjectCommand::Show { project } => {
            let project_dir = resolve_project_dir(&project)?;
            let meta_for_open = AnalysisProjectMetaShim::meta_for_open(&version);
            let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
                .map_err(map_store_error)?;
            let meta = store.meta().map_err(map_store_error)?;
            let sources = store.sources().map_err(map_store_error)?;
            let facts = store.facts().map_err(map_store_error)?;
            if output_settings.format == output::OutputFormat::Json {
                let value = serde_json::json!({
                    "command": "project_show",
                    "project": meta,
                    "sources": sources.iter().map(|s| serde_json::json!({
                        "source_id": s.source_id,
                        "kind": s.kind.as_str(),
                        "label": s.label,
                        "address_spaces": s.address_spaces.iter().map(|a| a.as_str()).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                    "facts": facts.len(),
                });
                println!("{}", project_json(value));
            } else {
                println!("project {} ({})", meta.name, meta.project_id);
                println!("  analyzer = {}", meta.analyzer_version);
                for source in &sources {
                    println!(
                        "  source {}/{} {} {:?}",
                        source.source_id,
                        source.kind,
                        source.label.as_deref().unwrap_or(""),
                        source.address_spaces.iter().map(|a| a.as_str()).collect::<Vec<_>>()
                    );
                }
                println!("  facts    = {}", facts.len());
            }
        }
        ProjectCommand::Export { project, out } => {
            let project_dir = resolve_project_dir(&project)?;
            let meta_for_open = AnalysisProjectMetaShim::meta_for_open(&version);
            let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta_for_open)
                .map_err(map_store_error)?;
            let bytes = store.export_canonical().map_err(map_store_error)?;
            match out {
                Some(path) => {
                    std::fs::write(&path, &bytes).map_err(|e| CliRunError::command(
                        "store_error",
                        format!("failed to write {}: {e}", path.display()),
                    ))?;
                    if output_settings.format == output::OutputFormat::Text {
                        println!("wrote {} bytes to {}", bytes.len(), path.display());
                    } else {
                        println!("{}", project_json(serde_json::json!({
                            "command": "project_export",
                            "path": path.display().to_string(),
                            "bytes": bytes.len(),
                        })));
                    }
                }
                None => {
                    use std::io::Write;
                    std::io::stdout()
                        .write_all(&bytes)
                        .map_err(|e| CliRunError::command("store_error", e.to_string()))?;
                }
            }
        }
    }
    Ok(())
}

/// `project show/export` open with the current analyzer version; the store
/// records observed-version drift without rewriting stored metadata.
struct AnalysisProjectMetaShim;
impl AnalysisProjectMetaShim {
    fn meta_for_open(version: &str) -> damsel_project::AnalysisProjectMeta {
        damsel_project::AnalysisProjectMeta {
            project_id: damsel_project::ProjectId::new("open"),
            name: String::new(),
            analyzer_version: version.to_string(),
        }
    }
}

fn handle_cache_command(
    command: CacheCommand,
    output_settings: &output::OutputSettings,
) -> Result<(), CliRunError> {
    match command {
        CacheCommand::Info { cache } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            output::print_cache_info(&cache_info_view(session.cache()), output_settings);
        }
        CacheCommand::Images {
            cache,
            name,
            limit,
            sort,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let mut images = session
                .cache()
                .images()
                .iter()
                .map(|image| cache_image_view(session.cache(), image))
                .collect::<Vec<_>>();
            if let Some(name_filter) = name {
                images.retain(|image| {
                    contains_case_insensitive(&image.id, &name_filter)
                        || contains_case_insensitive(&image.install_name, &name_filter)
                        || contains_case_insensitive(&image.basename, &name_filter)
                });
            }
            match sort {
                CacheImageSortArg::Name => images.sort_by_key(|image| image.install_name.clone()),
                CacheImageSortArg::Address => images.sort_by_key(|image| image.image_base_vmaddr),
            }
            let metadata = collection_metadata(images.len(), normalize_limit(limit, "--limit")?);
            let output_images = apply_limit(images, metadata.returned);
            output::print_cache_images(&metadata, &output_images, output_settings);
        }
        CacheCommand::Image { cache, image } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let image = session.resolve_image(&image).map_err(map_cache_error)?;
            output::print_cache_image(&cache_image_view(session.cache(), image), output_settings);
        }
        CacheCommand::Exports {
            cache,
            image,
            name,
            kind,
            flag,
            sort,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let cache_image = session.resolve_image(&image).map_err(map_cache_error)?;
            let mut exports = session.exports_for_image(&image).map_err(map_cache_error)?;
            if let Some(name_filter) = name.as_ref() {
                exports.retain(|export| contains_case_insensitive(&export.name, name_filter));
            }
            if let Some(kind_filter) = kind {
                let expected = output::ExportKindFilter::from(kind_filter);
                exports.retain(|export| matches_cache_export_kind(&export.kind, expected));
            }
            if let Some(flag_filter) = flag {
                exports.retain(|export| cache_export_matches_flag(&export.flags, flag_filter));
            }
            match sort {
                CacheExportSortArg::Address => exports.sort_by_key(|export| export.cache_vmaddr),
                CacheExportSortArg::Name => exports.sort_by_key(|export| export.name.clone()),
            }
            let metadata = collection_metadata(exports.len(), None);
            output::print_cache_exports(
                &cache_image_view(session.cache(), cache_image),
                &metadata,
                &exports
                    .iter()
                    .map(|export| output::CacheExportView {
                        name: export.name.clone(),
                        cache_vmaddr: export.cache_vmaddr,
                        kind: export.kind.clone(),
                        flags: export.flags.clone(),
                    })
                    .collect::<Vec<_>>(),
                output_settings,
            );
        }
        CacheCommand::LookupAddress { cache, vmaddr } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let result = session
                .lookup_cache_vmaddr(vmaddr)
                .map_err(map_cache_error)?;
            output::print_cache_lookup_address(&cache_lookup_view(&result), output_settings);
        }
        CacheCommand::ResolveSymbol {
            cache,
            symbol,
            image,
            limit,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let matches = session
                .resolve_exact_symbol(&symbol, image.as_deref(), None)
                .map_err(map_cache_error)?
                .into_iter()
                .map(cache_symbolication_view)
                .collect::<Vec<_>>();
            let metadata = collection_metadata(matches.len(), normalize_limit(limit, "--limit")?);
            let output_matches = apply_limit(matches, metadata.returned);
            output::print_cache_resolve_symbol(&metadata, &output_matches, output_settings);
        }
        CacheCommand::ImageDeps {
            cache,
            image,
            kind,
            limit,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let cache_image = session.resolve_image(&image).map_err(map_cache_error)?;
            let mut dependencies = session
                .image_dependencies(&image)
                .map_err(map_cache_error)?
                .into_iter()
                .filter(|record| match kind {
                    CacheDependencyKindArg::Internal => record.within_cache,
                    CacheDependencyKindArg::External => !record.within_cache,
                    CacheDependencyKindArg::All => true,
                })
                .map(cache_dependency_view)
                .collect::<Vec<_>>();
            dependencies.sort_by_key(|record| record.target_dylib_install_name.clone());
            let metadata =
                collection_metadata(dependencies.len(), normalize_limit(limit, "--limit")?);
            let output_dependencies = apply_limit(dependencies, metadata.returned);
            output::print_cache_image_deps(
                &cache_image_view(session.cache(), cache_image),
                &metadata,
                &output_dependencies,
                output_settings,
            );
        }
        CacheCommand::Dependents {
            cache,
            image,
            limit,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let cache_image = session.resolve_image(&image).map_err(map_cache_error)?;
            let mut dependents = session
                .dependents(&image)
                .map_err(map_cache_error)?
                .into_iter()
                .map(cache_dependent_view)
                .collect::<Vec<_>>();
            dependents.sort_by_key(|record| record.install_name.clone());
            let metadata =
                collection_metadata(dependents.len(), normalize_limit(limit, "--limit")?);
            let output_dependents = apply_limit(dependents, metadata.returned);
            output::print_cache_dependents(
                &cache_image_view(session.cache(), cache_image),
                &metadata,
                &output_dependents,
                output_settings,
            );
        }
        CacheCommand::SymbolProviders {
            cache,
            symbol,
            image,
            limit,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let mut providers = session
                .symbol_providers(&symbol)
                .map_err(map_cache_error)?
                .into_iter()
                .filter(|record| {
                    image.as_ref().map_or(true, |needle| {
                        contains_case_insensitive(&record.provider_image.install_name, needle)
                            || contains_case_insensitive(
                                &record.provider_image.image_id.to_string(),
                                needle,
                            )
                            || contains_case_insensitive(&record.provider_image.basename, needle)
                    })
                })
                .map(cache_provider_view)
                .collect::<Vec<_>>();
            providers.sort_by_key(|record| {
                (
                    record.install_name.clone(),
                    record.provider_kind.clone(),
                    record.target_dylib.clone(),
                    record.target_symbol.clone(),
                )
            });
            if providers.is_empty() {
                return Err(CliRunError::command(
                    "symbol_not_found",
                    format!("symbol not found: {symbol}"),
                ));
            }
            let metadata = collection_metadata(providers.len(), normalize_limit(limit, "--limit")?);
            let output_providers = apply_limit(providers, metadata.returned);
            output::print_cache_symbol_providers(
                &symbol,
                &metadata,
                &output_providers,
                output_settings,
            );
        }
        CacheCommand::SymbolImporters {
            cache,
            symbol,
            image,
            limit,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let mut importers = session
                .symbol_importers(&symbol)
                .map_err(map_cache_error)?
                .into_iter()
                .filter(|record| {
                    image.as_ref().map_or(true, |needle| {
                        contains_case_insensitive(&record.importer_image.install_name, needle)
                            || contains_case_insensitive(
                                &record.importer_image.image_id.to_string(),
                                needle,
                            )
                            || contains_case_insensitive(&record.importer_image.basename, needle)
                    })
                })
                .map(cache_importer_view)
                .collect::<Vec<_>>();
            importers.sort_by_key(|record| {
                (
                    record.install_name.clone(),
                    record.dylib_name.clone(),
                    record.symbol_name.clone(),
                    record.import_binding_kind.clone(),
                    record.import_binding_source.clone(),
                )
            });
            if importers.is_empty() {
                return Err(CliRunError::command(
                    "symbol_not_found",
                    format!("symbol not found: {symbol}"),
                ));
            }
            let metadata = collection_metadata(importers.len(), normalize_limit(limit, "--limit")?);
            let output_importers = apply_limit(importers, metadata.returned);
            output::print_cache_symbol_importers(
                &symbol,
                &metadata,
                &output_importers,
                output_settings,
            );
        }
        CacheCommand::Reexports {
            cache,
            image,
            name,
            limit,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let cache_image = session.resolve_image(&image).map_err(map_cache_error)?;
            let mut reexports = session
                .reexports(&image)
                .map_err(map_cache_error)?
                .into_iter()
                .filter(|record| {
                    name.as_ref()
                        .map(|needle| contains_case_insensitive(&record.export_name, needle))
                        .unwrap_or(true)
                })
                .map(cache_reexport_view)
                .collect::<Vec<_>>();
            reexports.sort_by_key(|record| {
                (
                    record.export_name.clone(),
                    record.target_dylib.clone(),
                    record.target_symbol.clone(),
                )
            });
            let metadata = collection_metadata(reexports.len(), normalize_limit(limit, "--limit")?);
            let output_reexports = apply_limit(reexports, metadata.returned);
            output::print_cache_reexports(
                &cache_image_view(session.cache(), cache_image),
                &metadata,
                &output_reexports,
                output_settings,
            );
        }
        CacheCommand::Sections {
            cache,
            image,
            executable_only,
            segment,
            name,
            sort,
        } => {
            let (projected, image_view) = load_projected_cache_image(cache, &image)?;
            let mut sections = projected.image.sections().to_vec();
            if executable_only {
                sections.retain(|section| section.executable);
            }
            if let Some(segment) = segment {
                sections
                    .retain(|section| contains_case_insensitive(&section.segment_name, &segment));
            }
            if let Some(name) = name {
                sections.retain(|section| contains_case_insensitive(&section.name, &name));
            }
            if let Some(sort) = sort {
                sort_sections(&mut sections, sort);
            }
            output::print_cache_sections(&image_view, &sections, output_settings);
        }
        CacheCommand::Symbols {
            cache,
            image,
            defined,
            global,
            section,
            name,
            sort,
        } => {
            let (projected, image_view) = load_projected_cache_image(cache, &image)?;
            let mut symbols = projected.image.symbols().to_vec();
            if defined {
                symbols.retain(|symbol| symbol.defined);
            }
            if global {
                symbols.retain(|symbol| symbol.global);
            }
            if let Some(section) = section {
                symbols.retain(|symbol| {
                    symbol
                        .section
                        .as_deref()
                        .is_some_and(|value| contains_case_insensitive(value, &section))
                });
            }
            if let Some(name) = name {
                symbols.retain(|symbol| contains_case_insensitive(&symbol.name, &name));
            }
            if let Some(sort) = sort {
                sort_symbols(&mut symbols, sort);
            }
            output::print_cache_symbols(&image_view, &symbols, output_settings);
        }
        CacheCommand::Imports {
            cache,
            image,
            dylib,
            name,
            lazy,
            weak,
            resolved,
            sort,
        } => {
            let (projected, image_view) = load_projected_cache_image(cache, &image)?;
            let mut imports = projected.image.imports().to_vec();
            if let Some(dylib) = dylib {
                imports.retain(|import| contains_case_insensitive(&import.dylib, &dylib));
            }
            if let Some(name) = name {
                imports.retain(|import| contains_case_insensitive(&import.name, &name));
            }
            if lazy {
                imports.retain(|import| import.is_lazy);
            }
            if weak {
                imports.retain(|import| import.is_weak);
            }
            if resolved {
                imports.retain(|import| import.address.is_some());
            }
            if let Some(sort) = sort {
                sort_imports(&mut imports, sort);
            }
            output::print_cache_imports(&image_view, &imports, output_settings);
        }
        CacheCommand::Dyld {
            cache,
            image,
            dylibs,
            rpaths,
            exports,
            function_starts,
            bindings,
            stubs,
            helpers,
            name,
            dylib,
            source,
            binding_kind,
            stub_kind,
            export_kind,
            export_flag,
            ordinal,
            sort,
        } => {
            let (projected, image_view) = load_projected_cache_image(cache, &image)?;
            let show_any =
                dylibs || rpaths || exports || function_starts || bindings || stubs || helpers;
            let mut view_options = if show_any {
                output::DyldViewOptions {
                    show_dylibs: dylibs,
                    show_rpaths: rpaths,
                    show_exports: exports,
                    show_function_starts: function_starts,
                    show_bindings: bindings,
                    show_stubs: stubs,
                    show_helpers: helpers,
                    name_filter: None,
                    dylib_filter: None,
                    source_filter: None,
                    binding_kind_filter: None,
                    stub_kind_filter: None,
                    export_kind_filter: None,
                    export_flag_filter: None,
                    ordinal_filter: None,
                    sort: None,
                }
            } else {
                output::DyldViewOptions::all()
            };
            view_options.name_filter = name;
            view_options.dylib_filter = dylib;
            view_options.source_filter = source.map(Into::into);
            view_options.binding_kind_filter = binding_kind.map(Into::into);
            view_options.stub_kind_filter = stub_kind.map(Into::into);
            view_options.export_kind_filter = export_kind.map(Into::into);
            view_options.export_flag_filter = export_flag.map(Into::into);
            view_options.ordinal_filter = ordinal;
            view_options.sort = sort.map(Into::into);
            output::print_cache_dyld(
                &image_view,
                &projected.image,
                &view_options,
                output_settings,
            );
        }
        CacheCommand::Objc {
            cache,
            image,
            detail,
            owner,
            name_source,
            selector_source,
            category_source,
        } => {
            let (projected, image_view) = load_projected_cache_image(cache, &image)?;
            output::print_cache_objc(
                &image_view,
                &projected.image,
                &output::ObjcViewOptions {
                    detail: detail.into(),
                    owner_filter: owner,
                    name_source_filter: name_source.map(Into::into),
                    selector_source_filter: selector_source.map(Into::into),
                    category_source_filter: category_source.map(Into::into),
                },
                output_settings,
            );
        }
        CacheCommand::Disasm {
            cache,
            image,
            symbol,
            swift_symbol,
            addr,
            section,
            objc_owner,
            objc_selector,
            objc_method_kind,
            count,
            limit,
            bytes,
            from,
            to,
            no_annotations,
            show_references,
            show_values,
            analysis,
        } => {
            let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
            let projected = session.project_image(&image).map_err(map_cache_error)?;
            let image_view = cache_image_view_from_projected(&projected);
            let execution = execute_disassembly(
                &projected.image,
                DisasmFlagArgs {
                    symbol,
                    swift_symbol,
                    addr,
                    section,
                    objc_owner,
                    objc_selector,
                    objc_method_kind,
                    count,
                    limit,
                    bytes,
                    from,
                    to,
                },
                !no_annotations,
                show_values || analysis,
            )?;
            let analysis = if analysis {
                let mut analysis = analyze_disassembly_with_image(
                    &projected.image,
                    execution.target.clone(),
                    execution.start_address,
                    execution.end_address,
                    &execution.instructions,
                );
                enrich_cache_analysis(&mut analysis, &session)?;
                Some(analysis)
            } else {
                None
            };
            output::print_cache_disassembly(
                &image_view,
                analysis
                    .as_ref()
                    .map(|analysis| execution.view_with_analysis(analysis))
                    .unwrap_or_else(|| execution.view()),
                output::DisassemblyRenderOptions {
                    include_annotations: !no_annotations,
                    include_references: show_references,
                    include_values: show_values,
                },
                output_settings,
            );
        }
    }
    Ok(())
}

fn cache_info_view(cache: &SharedCache) -> output::CacheInfoView {
    output::CacheInfoView {
        cache_path: cache.path().display().to_string(),
        cache_uuid: cache.header().cache_uuid.clone(),
        architecture: cache.header().architecture.to_string(),
        member_count: cache.members().len(),
        image_count: cache.images().len(),
        has_local_symbols: cache.header().has_local_symbols,
        members: cache
            .members()
            .iter()
            .enumerate()
            .map(|(index, member)| output::CacheMemberView {
                name: member_name(cache, index),
                role: match member.role {
                    SharedCacheMemberRole::Root => "root",
                    SharedCacheMemberRole::Subcache => "subcache",
                    SharedCacheMemberRole::Symbols => "symbols",
                }
                .to_string(),
                path: member.path.display().to_string(),
            })
            .collect(),
    }
}

fn cache_image_view(cache: &SharedCache, image: &CacheImageRecord) -> output::CacheImageView {
    output::CacheImageView {
        id: image.id.to_string(),
        image_index: image.image_index as u64,
        install_name: image.install_name.clone(),
        basename: image.basename.clone(),
        image_base_vmaddr: image.image_base_vmaddr,
        member_name: member_name(cache, image.member_index),
    }
}

fn cache_image_view_from_projected(projected: &ProjectedBinaryImage) -> output::CacheImageView {
    output::CacheImageView {
        id: projected.provenance.image_id.to_string(),
        image_index: projected.provenance.image_id.image_index() as u64,
        install_name: projected.provenance.install_name.clone(),
        basename: projected.provenance.basename.clone(),
        image_base_vmaddr: projected.provenance.image_base_vmaddr,
        member_name: projected.provenance.member_name.clone(),
    }
}

fn load_projected_cache_image(
    cache: PathBuf,
    image: &str,
) -> Result<(ProjectedBinaryImage, output::CacheImageView), CliRunError> {
    let session = inspect_shared_cache(cache).map_err(map_cache_error)?;
    let projected = session.project_image(image).map_err(map_cache_error)?;
    let image_view = cache_image_view_from_projected(&projected);
    Ok((projected, image_view))
}

fn member_name(cache: &SharedCache, member_index: usize) -> String {
    cache
        .members()
        .get(member_index)
        .and_then(|member| member.path.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("member-{member_index}"))
}

fn cache_lookup_view(result: &CacheLookupResult) -> output::CacheLookupResultView {
    match result {
        CacheLookupResult::ExactSymbol {
            cache_vmaddr,
            mapping,
            image,
            symbol,
        } => output::CacheLookupResultView {
            kind: output::CacheLookupKind::ExactSymbol,
            cache_vmaddr: *cache_vmaddr,
            member_name: mapping.member_name.clone(),
            mapping_base_vmaddr: mapping.mapping_base_vmaddr,
            mapping_size: mapping.mapping_size,
            member_file_offset: mapping.member_file_offset,
            image_id: Some(symbol.image_id.to_string()),
            install_name: Some(symbol.image_install_name.clone()),
            image_base_vmaddr: Some(image.image_base_vmaddr),
            image_offset: Some(symbol.image_offset),
            symbol: Some(symbol.symbol_name.clone()),
            symbol_source: Some(symbol.symbol_source.to_string()),
            symbol_address: Some(symbol.symbol_vmaddr),
            offset_from_symbol: Some(0),
        },
        CacheLookupResult::NearestSymbol {
            cache_vmaddr,
            mapping,
            image,
            symbol,
            distance,
        } => output::CacheLookupResultView {
            kind: output::CacheLookupKind::NearestSymbol,
            cache_vmaddr: *cache_vmaddr,
            member_name: mapping.member_name.clone(),
            mapping_base_vmaddr: mapping.mapping_base_vmaddr,
            mapping_size: mapping.mapping_size,
            member_file_offset: mapping.member_file_offset,
            image_id: Some(symbol.image_id.to_string()),
            install_name: Some(symbol.image_install_name.clone()),
            image_base_vmaddr: Some(image.image_base_vmaddr),
            image_offset: Some(symbol.image_offset),
            symbol: Some(symbol.symbol_name.clone()),
            symbol_source: Some(symbol.symbol_source.to_string()),
            symbol_address: Some(symbol.symbol_vmaddr),
            offset_from_symbol: Some(*distance),
        },
        CacheLookupResult::MappingOnly {
            cache_vmaddr,
            mapping,
            image,
        } => output::CacheLookupResultView {
            kind: output::CacheLookupKind::MappingOnly,
            cache_vmaddr: *cache_vmaddr,
            member_name: mapping.member_name.clone(),
            mapping_base_vmaddr: mapping.mapping_base_vmaddr,
            mapping_size: mapping.mapping_size,
            member_file_offset: mapping.member_file_offset,
            image_id: image.as_ref().map(|image| image.image_id.to_string()),
            install_name: image.as_ref().map(|image| image.install_name.clone()),
            image_base_vmaddr: image.as_ref().map(|image| image.image_base_vmaddr),
            image_offset: image
                .as_ref()
                .and_then(|image| cache_vmaddr.checked_sub(image.image_base_vmaddr)),
            symbol: None,
            symbol_source: None,
            symbol_address: None,
            offset_from_symbol: None,
        },
    }
}

fn collection_metadata(total: usize, limit: Option<usize>) -> output::CacheCollectionMetadata {
    let returned = limit.unwrap_or(total).min(total);
    output::CacheCollectionMetadata {
        total,
        returned,
        truncated: returned < total,
    }
}

fn apply_limit<T>(mut values: Vec<T>, returned: usize) -> Vec<T> {
    values.truncate(returned);
    values
}

fn normalize_limit(limit: Option<usize>, flag: &str) -> Result<Option<usize>, CliRunError> {
    if matches!(limit, Some(0)) {
        return Err(CliRunError::invalid_args(format!(
            "{flag} must be greater than 0"
        )));
    }
    Ok(limit)
}

fn matches_cache_export_kind(kind: &str, filter: output::ExportKindFilter) -> bool {
    matches!(
        (kind, filter),
        ("regular", output::ExportKindFilter::Regular)
            | ("reexport", output::ExportKindFilter::Reexport)
            | ("resolver", output::ExportKindFilter::Resolver)
            | (
                "stub-and-resolver",
                output::ExportKindFilter::StubAndResolver
            )
            | ("weak-definition", output::ExportKindFilter::WeakDefinition)
            | ("absolute", output::ExportKindFilter::Absolute)
            | ("thread-local", output::ExportKindFilter::ThreadLocal)
            | ("unknown", output::ExportKindFilter::Unknown)
    )
}

fn cache_export_matches_flag(flags: &str, flag: DyldExportFlagArg) -> bool {
    let pattern = match flag {
        DyldExportFlagArg::WeakDefinition => "weak=true",
        DyldExportFlagArg::Reexport => "reexport=true",
        DyldExportFlagArg::StubAndResolver => "stub_and_resolver=true",
        DyldExportFlagArg::ThreadLocal => "thread_local=true",
        DyldExportFlagArg::Absolute => "absolute=true",
    };
    flags.contains(pattern)
}

fn cache_symbolication_view(entry: SymbolicationMatch) -> output::CacheSymbolicationMatchView {
    output::CacheSymbolicationMatchView {
        name: entry.symbol_name,
        image_id: entry.image_id.to_string(),
        install_name: entry.image_install_name,
        member_name: entry.member_name,
        cache_vmaddr: entry.cache_vmaddr,
        image_base_vmaddr: entry.image_base_vmaddr,
        image_offset: entry.image_offset,
        member_file_offset: entry.member_file_offset,
        source: entry.symbol_source.to_string(),
    }
}

fn cache_dependency_view(entry: CacheImageDependencyRecord) -> output::CacheImageDependencyView {
    output::CacheImageDependencyView {
        target_dylib_install_name: entry.target_dylib_install_name,
        target_image_id: entry
            .target_image
            .as_ref()
            .map(|image| image.image_id.to_string()),
        target_install_name: entry
            .target_image
            .as_ref()
            .map(|image| image.install_name.clone()),
        target_member_name: entry
            .target_image
            .as_ref()
            .map(|image| image.member_name.clone()),
        within_cache: entry.within_cache,
        reference_count: entry.reference_count,
    }
}

fn cache_dependent_view(entry: CacheDependentRecord) -> output::CacheDependentView {
    output::CacheDependentView {
        image_id: entry.dependent_image.image_id.to_string(),
        install_name: entry.dependent_image.install_name,
        member_name: entry.dependent_image.member_name,
        dependency_count: entry.dependency_count,
    }
}

fn cache_provider_view(entry: CacheSymbolProviderRecord) -> output::CacheSymbolProviderView {
    output::CacheSymbolProviderView {
        image_id: entry.provider_image.image_id.to_string(),
        install_name: entry.provider_image.install_name,
        member_name: entry.provider_image.member_name,
        symbol_name: entry.symbol_name,
        provider_kind: entry.provider_kind.to_string(),
        target_dylib: entry.target_dylib,
        target_symbol: entry.target_symbol,
        resolved_target_image_id: entry
            .resolved_target_image
            .as_ref()
            .map(|image| image.image_id.to_string()),
        resolved_target_install_name: entry
            .resolved_target_image
            .as_ref()
            .map(|image| image.install_name.clone()),
        resolved_target_member_name: entry
            .resolved_target_image
            .as_ref()
            .map(|image| image.member_name.clone()),
    }
}

fn enrich_cache_analysis(
    analysis: &mut DisassemblyAnalysis,
    session: &SharedCacheSession,
) -> Result<(), CliRunError> {
    let mut seen = BTreeSet::new();
    let mut cache_links = Vec::new();

    for import in &analysis.imports {
        let providers = match session.symbol_providers(&import.name) {
            Ok(providers) => providers,
            Err(damsel_macho::MachoError::SymbolNotFound(_)) => continue,
            Err(error) => return Err(map_cache_error(error)),
        };

        for provider in providers
            .into_iter()
            .filter(|provider| cache_provider_matches_import(provider, &import.dylib))
        {
            let key = (
                import.instruction_address,
                import.name.clone(),
                import.dylib.clone(),
                provider.provider_image.image_id.to_string(),
                provider.target_dylib.clone(),
                provider.target_symbol.clone(),
            );
            if !seen.insert(key) {
                continue;
            }

            cache_links.push(DisassemblyCacheLinkUse {
                instruction_address: import.instruction_address,
                symbol_name: import.name.clone(),
                dylib: import.dylib.clone(),
                provider_image_id: provider.provider_image.image_id.to_string(),
                provider_install_name: provider.provider_image.install_name,
                provider_member_name: provider.provider_image.member_name,
                provider_kind: provider.provider_kind.to_string(),
                target_dylib: provider.target_dylib,
                target_symbol: provider.target_symbol,
                resolved_target_image_id: provider
                    .resolved_target_image
                    .as_ref()
                    .map(|image| image.image_id.to_string()),
                resolved_target_install_name: provider
                    .resolved_target_image
                    .as_ref()
                    .map(|image| image.install_name.clone()),
                resolved_target_member_name: provider
                    .resolved_target_image
                    .as_ref()
                    .map(|image| image.member_name.clone()),
            });
        }
    }

    cache_links.sort_by(|left, right| {
        (
            left.instruction_address,
            left.symbol_name.as_str(),
            left.dylib.as_str(),
            left.provider_install_name.as_str(),
            left.provider_kind.as_str(),
        )
            .cmp(&(
                right.instruction_address,
                right.symbol_name.as_str(),
                right.dylib.as_str(),
                right.provider_install_name.as_str(),
                right.provider_kind.as_str(),
            ))
    });
    analysis.summary.cache_link_count = cache_links.len();
    analysis.overview.cache_links = summarize_cache_link_uses(&cache_links);
    analysis.cache_links = cache_links;
    Ok(())
}

fn summarize_cache_link_uses(
    entries: &[DisassemblyCacheLinkUse],
) -> Vec<DisassemblyCacheLinkSummary> {
    let mut by_link: BTreeMap<
        (
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
        DisassemblyCacheLinkSummary,
    > = BTreeMap::new();

    for entry in entries {
        let key = (
            entry.symbol_name.clone(),
            entry.dylib.clone(),
            entry.provider_image_id.clone(),
            entry.provider_install_name.clone(),
            entry.provider_member_name.clone(),
            entry.provider_kind.clone(),
            entry.target_dylib.clone(),
            entry.target_symbol.clone(),
            entry.resolved_target_image_id.clone(),
            entry.resolved_target_install_name.clone(),
            entry.resolved_target_member_name.clone(),
        );
        by_link
            .entry(key)
            .and_modify(|summary| {
                summary.count += 1;
                summary.first_instruction_address = summary
                    .first_instruction_address
                    .min(entry.instruction_address);
            })
            .or_insert(DisassemblyCacheLinkSummary {
                symbol_name: entry.symbol_name.clone(),
                dylib: entry.dylib.clone(),
                provider_image_id: entry.provider_image_id.clone(),
                provider_install_name: entry.provider_install_name.clone(),
                provider_member_name: entry.provider_member_name.clone(),
                provider_kind: entry.provider_kind.clone(),
                target_dylib: entry.target_dylib.clone(),
                target_symbol: entry.target_symbol.clone(),
                resolved_target_image_id: entry.resolved_target_image_id.clone(),
                resolved_target_install_name: entry.resolved_target_install_name.clone(),
                resolved_target_member_name: entry.resolved_target_member_name.clone(),
                count: 1,
                first_instruction_address: entry.instruction_address,
            });
    }

    let mut rows = by_link.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            left.first_instruction_address,
            left.symbol_name.as_str(),
            left.dylib.as_str(),
            left.provider_install_name.as_str(),
            left.provider_kind.as_str(),
        )
            .cmp(&(
                right.first_instruction_address,
                right.symbol_name.as_str(),
                right.dylib.as_str(),
                right.provider_install_name.as_str(),
                right.provider_kind.as_str(),
            ))
    });
    rows
}

fn cache_provider_matches_import(provider: &CacheSymbolProviderRecord, dylib: &str) -> bool {
    provider.provider_image.install_name == dylib
        || provider.target_dylib.as_deref() == Some(dylib)
        || provider
            .resolved_target_image
            .as_ref()
            .is_some_and(|image| image.install_name == dylib)
}

fn cache_importer_view(entry: CacheSymbolImporterRecord) -> output::CacheSymbolImporterView {
    output::CacheSymbolImporterView {
        image_id: entry.importer_image.image_id.to_string(),
        install_name: entry.importer_image.install_name,
        member_name: entry.importer_image.member_name,
        symbol_name: entry.symbol_name,
        dylib_name: entry.dylib_name,
        import_binding_kind: entry.import_binding_kind.map(|kind| format!("{kind:?}")),
        import_binding_source: entry
            .import_binding_source
            .map(|source| format!("{source:?}")),
        resolved_provider_image_id: entry
            .resolved_provider_image
            .as_ref()
            .map(|image| image.image_id.to_string()),
        resolved_provider_install_name: entry
            .resolved_provider_image
            .as_ref()
            .map(|image| image.install_name.clone()),
        resolved_provider_member_name: entry
            .resolved_provider_image
            .as_ref()
            .map(|image| image.member_name.clone()),
    }
}

fn cache_reexport_view(entry: CacheReexportRecord) -> output::CacheReexportView {
    output::CacheReexportView {
        export_name: entry.export_name,
        target_dylib: entry.target_dylib,
        target_symbol: entry.target_symbol,
        resolved_target_image_id: entry
            .resolved_target_image
            .as_ref()
            .map(|image| image.image_id.to_string()),
        resolved_target_install_name: entry
            .resolved_target_image
            .as_ref()
            .map(|image| image.install_name.clone()),
        resolved_target_member_name: entry
            .resolved_target_image
            .as_ref()
            .map(|image| image.member_name.clone()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedObjcMethodTarget {
    address: u64,
    display_target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObjcMethodCandidate {
    owner_display: String,
    owner_kind: &'static str,
    selector: String,
    is_class_method: bool,
    address: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedSwiftSymbolTarget {
    address: u64,
    display_target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SwiftSymbolCandidate {
    mangled: String,
    normalized_mangled: String,
    demangled: Option<String>,
    address: u64,
}

fn resolve_swift_symbol_target(
    image: &BinaryImage,
    query: &str,
) -> Result<ResolvedSwiftSymbolTarget, CliRunError> {
    let mut candidates = swift_symbol_candidates(image);
    let exact_mangled = candidates
        .iter()
        .filter(|candidate| candidate.mangled == query || candidate.normalized_mangled == query)
        .cloned()
        .collect::<Vec<_>>();
    if !exact_mangled.is_empty() || swift_mangled_payload(query).is_some() {
        return swift_symbol_match(query, exact_mangled);
    }

    if candidates.is_empty() {
        return Err(CliRunError::command(
            "swift_symbol_not_found",
            format!("Swift symbol not found: {query}"),
        ));
    }

    let mangled_symbols = candidates
        .iter()
        .map(|candidate| candidate.mangled.clone())
        .collect::<Vec<_>>();
    let demangled = demangle_swift_symbols(&mangled_symbols)?;
    for candidate in &mut candidates {
        candidate.demangled = demangled.get(&candidate.mangled).cloned();
    }

    let exact_demangled = candidates
        .iter()
        .filter(|candidate| candidate.demangled.as_deref() == Some(query))
        .cloned()
        .collect::<Vec<_>>();
    if !exact_demangled.is_empty() {
        return swift_symbol_match(query, exact_demangled);
    }

    let partial_demangled = candidates
        .into_iter()
        .filter(|candidate| {
            candidate
                .demangled
                .as_deref()
                .is_some_and(|value| contains_case_insensitive(value, query))
        })
        .collect::<Vec<_>>();
    swift_symbol_match(query, partial_demangled)
}

fn swift_symbol_match(
    query: &str,
    mut matches: Vec<SwiftSymbolCandidate>,
) -> Result<ResolvedSwiftSymbolTarget, CliRunError> {
    matches.sort_by(|left, right| {
        (
            left.demangled.as_deref().unwrap_or(""),
            left.mangled.as_str(),
            left.address,
        )
            .cmp(&(
                right.demangled.as_deref().unwrap_or(""),
                right.mangled.as_str(),
                right.address,
            ))
    });
    matches.dedup_by(|left, right| {
        left.address == right.address
            && left.mangled == right.mangled
            && left.demangled == right.demangled
    });

    match matches.as_slice() {
        [] => Err(CliRunError::command(
            "swift_symbol_not_found",
            format!("Swift symbol not found: {query}"),
        )),
        [candidate] => Ok(ResolvedSwiftSymbolTarget {
            address: candidate.address,
            display_target: format!(
                "swift:{}",
                candidate.demangled.as_deref().unwrap_or(&candidate.mangled)
            ),
        }),
        _ => Err(CliRunError::command(
            "swift_symbol_ambiguous",
            format!("Swift symbol is ambiguous: {query}"),
        )),
    }
}

fn swift_symbol_candidates(image: &BinaryImage) -> Vec<SwiftSymbolCandidate> {
    let mut candidates = image
        .symbols()
        .iter()
        .filter(|symbol| symbol.defined && is_executable_symbol(image, symbol))
        .filter_map(|symbol| {
            let normalized_mangled = swift_mangled_payload(&symbol.name)?.to_string();
            Some(SwiftSymbolCandidate {
                mangled: symbol.name.clone(),
                normalized_mangled,
                demangled: None,
                address: symbol.address,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (left.address, left.mangled.as_str()).cmp(&(right.address, right.mangled.as_str()))
    });
    candidates
        .dedup_by(|left, right| left.address == right.address && left.mangled == right.mangled);
    candidates
}

fn is_executable_symbol(image: &BinaryImage, symbol: &Symbol) -> bool {
    matches!(symbol.kind, SymbolKind::Text | SymbolKind::Label)
        || image
            .containing_section(symbol.address)
            .is_some_and(|section| section.executable)
}

fn swift_mangled_payload(name: &str) -> Option<&str> {
    let trimmed = name.strip_prefix('_').unwrap_or(name);
    ["$s", "$S", "$e", "$E", "$T", "_T"]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
        .then_some(trimmed)
}

fn demangle_swift_symbols(symbols: &[String]) -> Result<BTreeMap<String, String>, CliRunError> {
    let Some(tool) = resolve_swift_demangle_tool() else {
        return Err(CliRunError::command(
            "swift_demangler_unavailable",
            "Swift demangled-name lookup requires `swift-demangle` or `xcrun --find swift-demangle`",
        ));
    };

    let mut demangled = BTreeMap::new();
    for chunk in symbols.chunks(128) {
        let output = ProcessCommand::new(&tool)
            .args(chunk)
            .output()
            .map_err(|error| {
                CliRunError::command(
                    "swift_demangler_unavailable",
                    format!("failed to run swift demangler: {error}"),
                )
            })?;
        if !output.status.success() {
            return Err(CliRunError::command(
                "swift_demangler_failed",
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        demangled.extend(parse_swift_demangle_output(&String::from_utf8_lossy(
            &output.stdout,
        )));
    }
    Ok(demangled)
}

fn resolve_swift_demangle_tool() -> Option<PathBuf> {
    if ProcessCommand::new("swift-demangle")
        .arg("--help")
        .output()
        .is_ok()
    {
        return Some(PathBuf::from("swift-demangle"));
    }

    let output = ProcessCommand::new("xcrun")
        .args(["--find", "swift-demangle"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(PathBuf::from(path))
}

fn parse_swift_demangle_output(output: &str) -> BTreeMap<String, String> {
    output
        .lines()
        .filter_map(|line| {
            let (mangled, demangled) = line.split_once(" ---> ")?;
            Some((mangled.to_string(), demangled.to_string()))
        })
        .collect()
}

fn resolve_objc_method_target(
    image: &BinaryImage,
    owner: &str,
    selector: &str,
    is_class_method: Option<bool>,
) -> Result<ResolvedObjcMethodTarget, CliRunError> {
    let objc = image.objc();
    let mut candidates = Vec::new();

    for class_record in &objc.classes {
        if class_record.name.as_deref() == Some(owner) {
            let owner_display = class_record
                .name
                .clone()
                .unwrap_or_else(|| format!("{:#x}", class_record.class_pointer));
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "class",
                false,
                &class_record.methods,
                selector,
                is_class_method,
            );
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "metaclass",
                true,
                &class_record.class_methods,
                selector,
                is_class_method,
            );
        }
    }

    for category_record in &objc.categories {
        let class_name = category_record.class_name.as_deref();
        let category_name = category_record.name.as_deref();
        let owner_matches = category_name == Some(owner)
            || class_name == Some(owner)
            || class_name
                .zip(category_name)
                .is_some_and(|(class_name, category_name)| {
                    format!("{class_name}({category_name})") == owner
                });
        if owner_matches {
            let owner_display = match (class_name, category_name) {
                (Some(class_name), Some(category_name)) => {
                    format!("{class_name}({category_name})")
                }
                (_, Some(category_name)) => category_name.to_string(),
                _ => format!("{:#x}", category_record.pointer),
            };
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "category",
                false,
                &category_record.methods,
                selector,
                is_class_method,
            );
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "category-metaclass",
                true,
                &category_record.class_methods,
                selector,
                is_class_method,
            );
        }
    }

    for protocol_record in &objc.protocols {
        if protocol_record.name.as_deref() == Some(owner) {
            let owner_display = protocol_record
                .name
                .clone()
                .unwrap_or_else(|| format!("{:#x}", protocol_record.pointer));
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "protocol",
                false,
                &protocol_record.required_instance_methods,
                selector,
                is_class_method,
            );
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "protocol",
                true,
                &protocol_record.required_class_methods,
                selector,
                is_class_method,
            );
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "protocol",
                false,
                &protocol_record.optional_instance_methods,
                selector,
                is_class_method,
            );
            collect_objc_method_candidates(
                &mut candidates,
                owner_display.as_str(),
                "protocol",
                true,
                &protocol_record.optional_class_methods,
                selector,
                is_class_method,
            );
        }
    }

    candidates.sort_by(|left, right| {
        (
            left.owner_display.as_str(),
            left.owner_kind,
            left.is_class_method,
            left.selector.as_str(),
            left.address,
        )
            .cmp(&(
                right.owner_display.as_str(),
                right.owner_kind,
                right.is_class_method,
                right.selector.as_str(),
                right.address,
            ))
    });
    candidates.dedup();

    match candidates.as_slice() {
        [] => Err(CliRunError::command(
            "objc_method_not_found",
            format!("Objective-C method not found: owner={owner} selector={selector}"),
        )),
        [candidate] => Ok(ResolvedObjcMethodTarget {
            address: candidate.address,
            display_target: format!(
                "objc:{}[{} {}]",
                if candidate.is_class_method { "+" } else { "-" },
                candidate.owner_display,
                candidate.selector
            ),
        }),
        _ => Err(CliRunError::command(
            "objc_method_ambiguous",
            format!(
                "Objective-C method is ambiguous: owner={owner} selector={selector}; refine with `--objc-method-kind`"
            ),
        )),
    }
}

fn collect_objc_method_candidates(
    out: &mut Vec<ObjcMethodCandidate>,
    owner_display: &str,
    owner_kind: &'static str,
    is_class_method: bool,
    methods: &[ObjcMethodRecord],
    selector: &str,
    class_method_filter: Option<bool>,
) {
    if class_method_filter.is_some_and(|expected| expected != is_class_method) {
        return;
    }

    for method in methods {
        if method.selector.as_deref() != Some(selector) {
            continue;
        }
        let Some(address) = method.implementation else {
            continue;
        };
        out.push(ObjcMethodCandidate {
            owner_display: owner_display.to_string(),
            owner_kind,
            selector: selector.to_string(),
            is_class_method,
            address,
        });
    }
}

fn load_image(path: PathBuf) -> Result<BinaryImage, CliRunError> {
    load(path).map_err(map_macho_error)
}

pub(crate) fn map_cache_error(error: damsel_macho::MachoError) -> CliRunError {
    match error {
        damsel_macho::MachoError::UnsupportedInputKind(kind) => CliRunError::command(
            "unsupported_input",
            format!("unsupported input kind: {kind}"),
        ),
        damsel_macho::MachoError::UnsupportedSharedCacheArchitecture(_) => {
            CliRunError::command("unsupported_architecture", error.to_string())
        }
        damsel_macho::MachoError::UnsupportedThinArchitecture { .. }
        | damsel_macho::MachoError::MissingArm64SliceInUniversal
        | damsel_macho::MachoError::UnsupportedArchitecture(_) => {
            CliRunError::command("unsupported_architecture", error.to_string())
        }
        damsel_macho::MachoError::IncompleteSharedCacheSet(_) => {
            CliRunError::command("cache_incomplete", error.to_string())
        }
        damsel_macho::MachoError::CacheImageNotFound(_) => {
            CliRunError::command("cache_image_not_found", error.to_string())
        }
        damsel_macho::MachoError::CacheImageAmbiguous(_) => {
            CliRunError::command("cache_image_ambiguous", error.to_string())
        }
        damsel_macho::MachoError::AddressNotMapped(address) => CliRunError::command(
            "address_not_mapped",
            format!("address {address:#x} is not mapped"),
        ),
        damsel_macho::MachoError::SymbolNotFound(symbol) => {
            CliRunError::command("symbol_not_found", format!("symbol not found: {symbol}"))
        }
        damsel_macho::MachoError::MalformedSharedCache(_) => {
            CliRunError::command("malformed_input", error.to_string())
        }
        damsel_macho::MachoError::MalformedFatBinary(_)
        | damsel_macho::MachoError::MalformedDyldPayload(_)
        | damsel_macho::MachoError::SliceOutOfBounds { .. }
        | damsel_macho::MachoError::LinkeditRangeOutOfBounds { .. } => {
            CliRunError::command("malformed_input", error.to_string())
        }
        other => CliRunError::command("load_error", other.to_string()),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DisasmFlagArgs {
    pub(crate) symbol: Option<String>,
    pub(crate) swift_symbol: Option<String>,
    pub(crate) addr: Option<u64>,
    pub(crate) section: Option<String>,
    pub(crate) objc_owner: Option<String>,
    pub(crate) objc_selector: Option<String>,
    pub(crate) objc_method_kind: Option<ObjcMethodKindArg>,
    pub(crate) count: Option<usize>,
    pub(crate) limit: Option<usize>,
    pub(crate) bytes: Option<usize>,
    pub(crate) from: Option<u64>,
    pub(crate) to: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct DisasmPlan {
    decode_target: DisassemblyTarget,
    display_target: Option<String>,
    max_instructions: Option<usize>,
    limit: Option<DisassemblyLimit>,
    window_start: u64,
    window_end: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct DisassemblyExecution {
    target: String,
    start_address: u64,
    bytes_len: usize,
    decoded_bytes: usize,
    end_address: u64,
    instruction_count: usize,
    stop_reason: String,
    window_end: Option<u64>,
    instructions: Vec<DecodedInstruction>,
}

impl DisassemblyExecution {
    pub(crate) fn view(&self) -> output::DisassemblyView<'_> {
        output::DisassemblyView {
            target: &self.target,
            start_address: self.start_address,
            bytes_len: self.bytes_len,
            decoded_bytes: self.decoded_bytes,
            end_address: self.end_address,
            instruction_count: self.instruction_count,
            stop_reason: self.stop_reason.clone(),
            window_end: self.window_end,
            instructions: &self.instructions,
            analysis: None,
        }
    }

    pub(crate) fn view_with_analysis<'a>(
        &'a self,
        analysis: &'a DisassemblyAnalysis,
    ) -> output::DisassemblyView<'a> {
        output::DisassemblyView {
            target: &self.target,
            start_address: self.start_address,
            bytes_len: self.bytes_len,
            decoded_bytes: self.decoded_bytes,
            end_address: self.end_address,
            instruction_count: self.instruction_count,
            stop_reason: self.stop_reason.clone(),
            window_end: self.window_end,
            instructions: &self.instructions,
            analysis: Some(analysis),
        }
    }
}

pub(crate) fn plan_disassembly(
    image: &BinaryImage,
    args: DisasmFlagArgs,
) -> Result<DisasmPlan, CliRunError> {
    let instruction_limit = normalize_instruction_limit(args.count, args.limit)?;

    if args.bytes.is_some() && instruction_limit.is_some() {
        return Err(CliRunError::invalid_args(
            "`--bytes` cannot be combined with `--count` or `--limit`",
        ));
    }
    if args.to.is_some() && args.bytes.is_some() {
        return Err(CliRunError::invalid_args(
            "`--to` cannot be combined with `--bytes`",
        ));
    }
    if args.to.is_some() && instruction_limit.is_some() {
        return Err(CliRunError::invalid_args(
            "`--to` cannot be combined with `--count` or `--limit`",
        ));
    }
    if let Some(bytes) = args.bytes
        && bytes == 0
    {
        return Err(CliRunError::invalid_args(
            "`--bytes` must be greater than 0",
        ));
    }

    let (base_target, base_start, display_target) = if let Some(symbol) = args.symbol {
        let symbol_entry = image
            .symbol_by_name(&symbol)
            .ok_or_else(|| CliRunError::command("symbol_not_found", symbol.clone()))?;
        (
            DisassemblyTarget::Symbol(symbol),
            symbol_entry.address,
            None,
        )
    } else if let Some(swift_symbol) = args.swift_symbol {
        let resolved = resolve_swift_symbol_target(image, &swift_symbol)?;
        (
            DisassemblyTarget::Address(resolved.address),
            resolved.address,
            Some(resolved.display_target),
        )
    } else if let Some(addr) = args.addr {
        (DisassemblyTarget::Address(addr), addr, None)
    } else if let Some(selector) = args.objc_selector {
        let owner = args
            .objc_owner
            .expect("objc-owner is required by clap when objc-selector is present");
        let resolved = resolve_objc_method_target(
            image,
            &owner,
            &selector,
            args.objc_method_kind
                .map(ObjcMethodKindArg::is_class_method),
        )?;
        (
            DisassemblyTarget::Address(resolved.address),
            resolved.address,
            Some(resolved.display_target),
        )
    } else {
        let section_name = args.section.expect("section target");
        let section_entry = image
            .section_by_name(&section_name)
            .ok_or_else(|| CliRunError::command("section_not_found", section_name.clone()))?;
        (
            DisassemblyTarget::Section(section_name),
            section_entry.address,
            None,
        )
    };

    if let (Some(addr), Some(from)) = (args.addr, args.from)
        && addr != from
    {
        return Err(CliRunError::invalid_args(
            "`--from` must match `--addr` when both are provided",
        ));
    }

    let window_start = args.from.unwrap_or(base_start);
    if let Some(to) = args.to
        && to <= window_start
    {
        return Err(CliRunError::invalid_args(
            "`--to` must be greater than the decode start",
        ));
    }

    let window_end =
        if let Some(to) = args.to {
            Some(to)
        } else if let Some(bytes) = args.bytes {
            Some(window_start.checked_add(bytes as u64).ok_or_else(|| {
                CliRunError::invalid_args("decode window overflows address space")
            })?)
        } else {
            None
        };

    let max_instructions = if let Some(limit) = instruction_limit {
        Some(limit)
    } else if let Some(end) = window_end {
        let window_bytes = end.saturating_sub(window_start) as usize;
        Some((window_bytes.saturating_add(3)) / 4)
    } else {
        None
    };

    let request_limit = if let Some(limit) = instruction_limit {
        Some(DisassemblyLimit::Instructions(limit))
    } else if let Some(end) = window_end {
        Some(DisassemblyLimit::Bytes(
            end.saturating_sub(window_start) as usize
        ))
    } else {
        None
    };

    let decode_target = if args.from.is_some() {
        DisassemblyTarget::Address(window_start)
    } else {
        base_target
    };

    Ok(DisasmPlan {
        decode_target,
        display_target,
        max_instructions,
        limit: request_limit,
        window_start,
        window_end,
    })
}

pub(crate) fn normalize_instruction_limit(
    count: Option<usize>,
    limit: Option<usize>,
) -> Result<Option<usize>, CliRunError> {
    match (count, limit) {
        (Some(left), Some(right)) if left != right => Err(CliRunError::invalid_args(
            "`--count` and `--limit` cannot differ when both are provided",
        )),
        (Some(0), _) | (_, Some(0)) => Err(CliRunError::invalid_args(
            "instruction limit must be greater than 0",
        )),
        (Some(value), _) => Ok(Some(value)),
        (_, Some(value)) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

pub(crate) fn map_disasm_error(error: damsel_macho::MachoError) -> CliRunError {
    map_macho_error(error)
}

pub(crate) fn execute_disassembly(
    image: &BinaryImage,
    args: DisasmFlagArgs,
    include_annotations: bool,
    include_value_flow: bool,
) -> Result<DisassemblyExecution, CliRunError> {
    let disasm_plan = plan_disassembly(image, args)?;
    let request_limit = disasm_plan.limit.unwrap_or_else(|| {
        disasm_plan
            .max_instructions
            .map(DisassemblyLimit::Instructions)
            .unwrap_or(DisassemblyLimit::Unlimited)
    });
    let request_range = disasm_plan
        .window_end
        .map(|end| disasm_plan.window_start..end);
    let result = disassemble_v2(
        image,
        &DisassemblyRequestV2 {
            target: disasm_plan.decode_target,
            range: request_range,
            limit: request_limit,
            options: DisassemblyOptions {
                include_annotations,
                include_value_flow,
            },
        },
    )
    .map_err(map_disasm_error)?;

    let instructions = filter_instructions(
        &result.instructions,
        disasm_plan.window_start,
        disasm_plan.window_end,
    );
    let bytes_len = match disasm_plan.window_end {
        Some(end) => end.saturating_sub(disasm_plan.window_start) as usize,
        None => infer_bytes_len(disasm_plan.window_start, None, &instructions),
    };
    let end_address = instructions
        .last()
        .map(|instruction| {
            instruction
                .address
                .saturating_add(u64::from(instruction.size))
        })
        .unwrap_or(disasm_plan.window_start);

    Ok(DisassemblyExecution {
        target: disasm_plan.display_target.unwrap_or(result.target),
        start_address: disasm_plan.window_start,
        bytes_len,
        decoded_bytes: result.decoded_bytes,
        end_address,
        instruction_count: instructions.len(),
        stop_reason: format!("{:?}", result.stop_reason),
        window_end: disasm_plan.window_end,
        instructions,
    })
}

pub(crate) fn map_macho_error(error: damsel_macho::MachoError) -> CliRunError {
    match error {
        damsel_macho::MachoError::UnsupportedInputKind(kind) => CliRunError::command(
            "unsupported_input",
            format!("unsupported input kind: {kind}"),
        ),
        damsel_macho::MachoError::UnsupportedThinArchitecture { .. }
        | damsel_macho::MachoError::MissingArm64SliceInUniversal
        | damsel_macho::MachoError::UnsupportedArchitecture(_) => {
            CliRunError::command("unsupported_architecture", error.to_string())
        }
        damsel_macho::MachoError::SymbolNotFound(symbol) => {
            CliRunError::command("symbol_not_found", format!("symbol not found: {symbol}"))
        }
        damsel_macho::MachoError::SectionNotFound(section) => {
            CliRunError::command("section_not_found", format!("section not found: {section}"))
        }
        damsel_macho::MachoError::AddressNotMapped(address) => CliRunError::command(
            "address_not_mapped",
            format!("address {address:#x} is not mapped"),
        ),
        damsel_macho::MachoError::Decode(inner) => {
            CliRunError::command("decode_error", format!("decode error: {inner}"))
        }
        damsel_macho::MachoError::MalformedFatBinary(_)
        | damsel_macho::MachoError::MalformedDyldPayload(_)
        | damsel_macho::MachoError::SliceOutOfBounds { .. }
        | damsel_macho::MachoError::LinkeditRangeOutOfBounds { .. } => {
            CliRunError::command("malformed_input", error.to_string())
        }
        other => CliRunError::command("load_error", other.to_string()),
    }
}

pub(crate) fn parse_address(value: &str) -> Result<u64, String> {
    let value = value.trim();
    if let Some(stripped) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(stripped, 16).map_err(|error| error.to_string())
    } else {
        value.parse::<u64>().map_err(|error| error.to_string())
    }
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn sort_sections(sections: &mut [Section], sort: SectionSortArg) {
    match sort {
        SectionSortArg::Address => sections.sort_by_key(|section| section.address),
        SectionSortArg::Name => sections.sort_by_key(|section| section.full_name()),
        SectionSortArg::Size => sections.sort_by_key(|section| section.size),
    }
}

fn sort_symbols(symbols: &mut [Symbol], sort: SymbolSortArg) {
    match sort {
        SymbolSortArg::Address => symbols.sort_by_key(|symbol| symbol.address),
        SymbolSortArg::Name => symbols.sort_by_key(|symbol| symbol.name.clone()),
        SymbolSortArg::Size => symbols.sort_by_key(|symbol| symbol.size),
    }
}

fn sort_imports(imports: &mut [Import], sort: ImportSortArg) {
    match sort {
        ImportSortArg::Address => imports.sort_by_key(|import| import.address.unwrap_or_default()),
        ImportSortArg::Dylib => {
            imports.sort_by_key(|import| (import.dylib.clone(), import.name.clone()))
        }
        ImportSortArg::Name => imports.sort_by_key(|import| import.name.clone()),
    }
}

fn sort_relocations(relocations: &mut [Relocation], sort: RelocationSortArg) {
    match sort {
        RelocationSortArg::Address => relocations.sort_by_key(|relocation| relocation.address),
        RelocationSortArg::Section => {
            relocations.sort_by_key(|relocation| (relocation.section.clone(), relocation.address))
        }
        RelocationSortArg::Size => relocations.sort_by_key(|relocation| relocation.size),
    }
}

pub(crate) fn filter_instructions(
    instructions: &[DecodedInstruction],
    from: u64,
    to: Option<u64>,
) -> Vec<DecodedInstruction> {
    instructions
        .iter()
        .filter(|instruction| {
            instruction.address >= from && to.is_none_or(|end| instruction.address < end)
        })
        .cloned()
        .collect()
}

pub(crate) fn infer_bytes_len(
    start: u64,
    range_end: Option<u64>,
    instructions: &[DecodedInstruction],
) -> usize {
    if let Some(end) = range_end {
        return end.saturating_sub(start) as usize;
    }
    match instructions.last() {
        Some(last) => last
            .address
            .saturating_add(u64::from(last.size))
            .saturating_sub(start) as usize,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use damsel_core::{
        Architecture, BinaryFormat, Endianness, ObjcMetadata, Platform, Section, SliceInfo,
    };

    fn synthetic_swift_image(symbols: Vec<Symbol>) -> BinaryImage {
        BinaryImage::from_memory_bytes(
            Some("swift-symbol-test".to_string()),
            BinaryFormat::MachO,
            Architecture::Arm64,
            Endianness::Little,
            None,
            Some(Platform::unknown("macos")),
            SliceInfo {
                offset: 0,
                size: 128,
                is_universal: false,
                cpu_subtype: 0,
            },
            vec![],
            vec![Section {
                segment_name: "__TEXT".to_string(),
                name: "__text".to_string(),
                address: 0x1000,
                size: 128,
                file_offset: Some(0),
                file_size: 128,
                kind: "Text".to_string(),
                executable: true,
            }],
            symbols,
            vec![],
            vec![],
            ObjcMetadata::default(),
            damsel_core::DyldMetadata::default(),
            vec![0u8; 128].into(),
        )
    }

    fn text_symbol(name: &str, address: u64) -> Symbol {
        Symbol {
            name: name.to_string(),
            address,
            size: 16,
            kind: SymbolKind::Text,
            defined: true,
            global: true,
            weak: false,
            section: Some("__text".to_string()),
        }
    }

    #[test]
    fn parse_address_accepts_uppercase_hex_prefix() {
        assert_eq!(parse_address("0X1000004b0"), Ok(0x1000004b0));
    }

    #[test]
    fn swift_symbol_target_accepts_exact_macho_mangled_symbol() {
        let image = synthetic_swift_image(vec![
            text_symbol("_$s11SwiftSample9publicAddyS2iF", 0x1020),
            text_symbol("_main", 0x1000),
        ]);

        let resolved = resolve_swift_symbol_target(&image, "_$s11SwiftSample9publicAddyS2iF")
            .expect("resolve exact mangled Swift symbol");

        assert_eq!(resolved.address, 0x1020);
        assert_eq!(
            resolved.display_target,
            "swift:_$s11SwiftSample9publicAddyS2iF"
        );
    }

    #[test]
    fn swift_symbol_target_accepts_normalized_mangled_symbol() {
        let image =
            synthetic_swift_image(vec![text_symbol("_$s11SwiftSample9publicAddyS2iF", 0x1020)]);

        let resolved = resolve_swift_symbol_target(&image, "$s11SwiftSample9publicAddyS2iF")
            .expect("resolve normalized Swift symbol");

        assert_eq!(resolved.address, 0x1020);
    }

    #[test]
    fn swift_symbol_target_ignores_non_swift_symbols() {
        let image = synthetic_swift_image(vec![text_symbol("_main", 0x1000)]);

        let error = resolve_swift_symbol_target(&image, "_$s11SwiftSample9publicAddyS2iF")
            .expect_err("non-Swift symbols should not satisfy Swift target lookup");

        assert_eq!(error.code, "swift_symbol_not_found");
    }

    #[test]
    fn swift_symbol_match_prefers_demangled_display_when_available() {
        let resolved = swift_symbol_match(
            "SwiftSample.publicAdd",
            vec![SwiftSymbolCandidate {
                mangled: "_$s11SwiftSample9publicAddyS2iF".to_string(),
                normalized_mangled: "$s11SwiftSample9publicAddyS2iF".to_string(),
                demangled: Some("SwiftSample.publicAdd(Swift.Int) -> Swift.Int".to_string()),
                address: 0x1020,
            }],
        )
        .expect("resolve demangled candidate");

        assert_eq!(resolved.address, 0x1020);
        assert_eq!(
            resolved.display_target,
            "swift:SwiftSample.publicAdd(Swift.Int) -> Swift.Int"
        );
    }

    #[test]
    fn swift_symbol_match_reports_ambiguous_demangled_queries() {
        let error = swift_symbol_match(
            "SwiftSample.publicAdd",
            vec![
                SwiftSymbolCandidate {
                    mangled: "_$s11SwiftSample9publicAddyS2iF".to_string(),
                    normalized_mangled: "$s11SwiftSample9publicAddyS2iF".to_string(),
                    demangled: Some("SwiftSample.publicAdd(Swift.Int) -> Swift.Int".to_string()),
                    address: 0x1020,
                },
                SwiftSymbolCandidate {
                    mangled: "_$s11SwiftSample9publicAddySdF".to_string(),
                    normalized_mangled: "$s11SwiftSample9publicAddySdF".to_string(),
                    demangled: Some(
                        "SwiftSample.publicAdd(Swift.Double) -> Swift.Double".to_string(),
                    ),
                    address: 0x1040,
                },
            ],
        )
        .expect_err("overloaded demangled query should be ambiguous");

        assert_eq!(error.code, "swift_symbol_ambiguous");
    }

    #[test]
    fn parse_swift_demangle_output_reads_tool_lines() {
        let parsed = parse_swift_demangle_output(
            "_$s11SwiftSample9publicAddyS2iF ---> SwiftSample.publicAdd(Swift.Int) -> Swift.Int\n",
        );

        assert_eq!(
            parsed
                .get("_$s11SwiftSample9publicAddyS2iF")
                .map(String::as_str),
            Some("SwiftSample.publicAdd(Swift.Int) -> Swift.Int")
        );
    }

    #[test]
    fn exact_swift_symbol_plan_uses_address_target() {
        let image =
            synthetic_swift_image(vec![text_symbol("_$s11SwiftSample9publicAddyS2iF", 0x1020)]);

        let plan = plan_disassembly(
            &image,
            DisasmFlagArgs {
                symbol: None,
                swift_symbol: Some("_$s11SwiftSample9publicAddyS2iF".to_string()),
                addr: None,
                section: None,
                objc_owner: None,
                objc_selector: None,
                objc_method_kind: None,
                count: None,
                limit: Some(8),
                bytes: None,
                from: None,
                to: None,
            },
        )
        .expect("plan Swift symbol");

        assert_eq!(plan.decode_target, DisassemblyTarget::Address(0x1020));
        assert_eq!(
            plan.display_target.as_deref(),
            Some("swift:_$s11SwiftSample9publicAddyS2iF")
        );
    }
}
