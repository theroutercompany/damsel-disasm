mod output;
mod ui;

use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use damsel_core::{
    BinaryImage, CacheImageRecord, CacheLookupResult, DecodedInstruction, DisassemblyLimit,
    DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget, ExportFlagName, Import,
    ImportBindingKind, ObjcNameSource, ObjcSelectorSource, ProjectedBinaryImage, Relocation,
    Section, SharedCache, SharedCacheMemberRole, StubKind, Symbol, SymbolicationMatch,
};
use damsel_macho::{disassemble_v2, inspect_shared_cache, load};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

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
            .args(["symbol", "addr", "section"])
            .required(true)
    ))]
    Disasm {
        path: PathBuf,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long, value_parser = parse_address)]
        addr: Option<u64>,
        #[arg(long)]
        section: Option<String>,
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
    },
    Ui {
        #[arg(long, default_value_t = 4317)]
        port: u16,
        #[arg(long)]
        no_open: bool,
    },
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
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
            .args(["symbol", "addr", "section"])
            .required(true)
    ))]
    Disasm {
        cache: PathBuf,
        image: String,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long, value_parser = parse_address)]
        addr: Option<u64>,
        #[arg(long)]
        section: Option<String>,
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
            addr,
            section,
            count,
            limit,
            bytes,
            from,
            to,
            no_annotations,
            show_references,
            show_values,
        } => {
            let image = load_image(path)?;
            let execution = execute_disassembly(
                &image,
                DisasmFlagArgs {
                    symbol,
                    addr,
                    section,
                    count,
                    limit,
                    bytes,
                    from,
                    to,
                },
                !no_annotations,
                show_values,
            )?;

            output::print_disassembly(
                execution.view(),
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
        Command::Cache { command } => handle_cache_command(command, &output_settings)?,
    }

    Ok(CliRunOutcome::Success)
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
            addr,
            section,
            count,
            limit,
            bytes,
            from,
            to,
            no_annotations,
            show_references,
            show_values,
        } => {
            let (projected, image_view) = load_projected_cache_image(cache, &image)?;
            let execution = execute_disassembly(
                &projected.image,
                DisasmFlagArgs {
                    symbol,
                    addr,
                    section,
                    count,
                    limit,
                    bytes,
                    from,
                    to,
                },
                !no_annotations,
                show_values,
            )?;
            output::print_cache_disassembly(
                &image_view,
                execution.view(),
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
    pub(crate) addr: Option<u64>,
    pub(crate) section: Option<String>,
    pub(crate) count: Option<usize>,
    pub(crate) limit: Option<usize>,
    pub(crate) bytes: Option<usize>,
    pub(crate) from: Option<u64>,
    pub(crate) to: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct DisasmPlan {
    decode_target: DisassemblyTarget,
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

    let (base_target, base_start) = if let Some(symbol) = args.symbol {
        let symbol_entry = image
            .symbol_by_name(&symbol)
            .ok_or_else(|| CliRunError::command("symbol_not_found", symbol.clone()))?;
        (DisassemblyTarget::Symbol(symbol), symbol_entry.address)
    } else if let Some(addr) = args.addr {
        (DisassemblyTarget::Address(addr), addr)
    } else {
        let section_name = args.section.expect("section target");
        let section_entry = image
            .section_by_name(&section_name)
            .ok_or_else(|| CliRunError::command("section_not_found", section_name.clone()))?;
        (
            DisassemblyTarget::Section(section_name),
            section_entry.address,
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
        target: result.target,
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
    if let Some(stripped) = value.strip_prefix("0x") {
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
