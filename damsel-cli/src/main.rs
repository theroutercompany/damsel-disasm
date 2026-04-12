mod output;

use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use damsel_core::{
    BinaryImage, DecodedInstruction, DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2,
    DisassemblyTarget, ExportFlagName, Import, ImportBindingKind, ObjcNameSource,
    ObjcSelectorSource, Relocation, Section, StubKind, Symbol,
};
use damsel_macho::{disassemble_v2, load};
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
enum ObjcCategorySourceArg {
    RuntimeList,
    SymbolSynthesis,
    SymbolSynthesisWithLists,
}

impl From<ObjcCategorySourceArg> for output::ObjcCategorySourceFilter {
    fn from(value: ObjcCategorySourceArg) -> Self {
        match value {
            ObjcCategorySourceArg::RuntimeList => output::ObjcCategorySourceFilter::RuntimeList,
            ObjcCategorySourceArg::SymbolSynthesis => output::ObjcCategorySourceFilter::SymbolSynthesis,
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
}

fn main() {
    let cli = Cli::parse();
    let output_settings = output::OutputSettings {
        format: cli.format.into(),
        pretty: cli.pretty,
    };

    if let Err(error) = run(cli, output_settings) {
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

#[derive(Debug)]
struct CliRunError {
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

fn run(cli: Cli, output_settings: output::OutputSettings) -> Result<(), CliRunError> {
    match cli.command {
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
            let disasm_plan = plan_disassembly(
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
            )?;

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
                &image,
                &DisassemblyRequestV2 {
                    target: disasm_plan.decode_target,
                    range: request_range,
                    limit: request_limit,
                    options: DisassemblyOptions {
                        include_annotations: !no_annotations,
                        include_value_flow: show_values,
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

            output::print_disassembly(
                output::DisassemblyView {
                    target: &result.target,
                    start_address: disasm_plan.window_start,
                    bytes_len,
                    decoded_bytes: result.decoded_bytes,
                    end_address,
                    instruction_count: instructions.len(),
                    stop_reason: format!("{:?}", result.stop_reason),
                    window_end: disasm_plan.window_end,
                    instructions: &instructions,
                },
                output::DisassemblyRenderOptions {
                    include_annotations: !no_annotations,
                    include_references: show_references,
                    include_values: show_values,
                },
                &output_settings,
            );
        }
    }

    Ok(())
}

fn load_image(path: PathBuf) -> Result<BinaryImage, CliRunError> {
    load(path).map_err(map_macho_error)
}

#[derive(Debug)]
struct DisasmFlagArgs {
    symbol: Option<String>,
    addr: Option<u64>,
    section: Option<String>,
    count: Option<usize>,
    limit: Option<usize>,
    bytes: Option<usize>,
    from: Option<u64>,
    to: Option<u64>,
}

#[derive(Debug)]
struct DisasmPlan {
    decode_target: DisassemblyTarget,
    max_instructions: Option<usize>,
    limit: Option<DisassemblyLimit>,
    window_start: u64,
    window_end: Option<u64>,
}

fn plan_disassembly(image: &BinaryImage, args: DisasmFlagArgs) -> Result<DisasmPlan, CliRunError> {
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

fn normalize_instruction_limit(
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

fn map_disasm_error(error: damsel_macho::MachoError) -> CliRunError {
    map_macho_error(error)
}

fn map_macho_error(error: damsel_macho::MachoError) -> CliRunError {
    match error {
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
        other => CliRunError::command("load_error", other.to_string()),
    }
}

fn parse_address(value: &str) -> Result<u64, String> {
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

fn filter_instructions(
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

fn infer_bytes_len(
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
