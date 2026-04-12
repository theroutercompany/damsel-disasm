mod output;

use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use damsel_core::{
    DecodedInstruction, DisassemblyLimit, DisassemblyRequest, DisassemblyTarget, Import,
    Relocation, Section, Symbol,
};
use damsel_macho::{disassemble, load};
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
    },
    Slices {
        path: PathBuf,
    },
    Objc {
        path: PathBuf,
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
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let output_settings = output::OutputSettings {
        format: cli.format.into(),
        pretty: cli.pretty,
    };

    match cli.command {
        Command::Info { path } => output::print_info(&load(path)?, &output_settings),
        Command::Sections {
            path,
            executable_only,
            segment,
            name,
            sort,
        } => {
            let image = load(path)?;
            let mut sections = image.sections.clone();
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
            let image = load(path)?;
            let mut symbols = image.symbols.clone();
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
            let image = load(path)?;
            let mut imports = image.imports.clone();
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
            let image = load(path)?;
            let mut relocations = image.relocations.clone();
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
        } => {
            let image = load(path)?;
            let show_any = dylibs || rpaths || exports || function_starts;
            let view_options = if show_any {
                output::DyldViewOptions {
                    show_dylibs: dylibs,
                    show_rpaths: rpaths,
                    show_exports: exports,
                    show_function_starts: function_starts,
                }
            } else {
                output::DyldViewOptions::all()
            };
            output::print_dyld(&image, view_options, &output_settings);
        }
        Command::Slices { path } => {
            let image = load(path)?;
            output::print_slices(&image, &output_settings);
        }
        Command::Objc { path } => output::print_objc(&load(path)?, &output_settings),
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
        } => {
            if let (Some(from), Some(to)) = (from, to) {
                if to <= from {
                    return Err("`--to` must be greater than `--from`".into());
                }
            }

            let image = load(path)?;
            let target = if let Some(symbol) = symbol {
                DisassemblyTarget::Symbol(symbol)
            } else if let Some(addr) = addr {
                DisassemblyTarget::Address(addr)
            } else {
                DisassemblyTarget::Section(section.expect("section target"))
            };
            let count_limit = count.or(limit);
            let request_limit = match (count_limit, bytes) {
                (Some(count), Some(bytes)) => {
                    Some(DisassemblyLimit::Instructions(count.min(bytes.div_ceil(4))))
                }
                (Some(count), None) => Some(DisassemblyLimit::Instructions(count)),
                (None, Some(bytes)) => Some(DisassemblyLimit::Bytes(bytes)),
                (None, None) => None,
            };
            let instruction_limit = request_limit.and_then(|limit| match limit {
                DisassemblyLimit::Instructions(count) => Some(count),
                DisassemblyLimit::Bytes(bytes) => Some(bytes.div_ceil(4)),
                DisassemblyLimit::Unlimited => None,
            });
            let result = disassemble(
                &image,
                &DisassemblyRequest {
                    target,
                    max_instructions: instruction_limit,
                    limit: request_limit,
                    include_annotations: !no_annotations,
                },
            )?;

            let range_start = from.unwrap_or(result.start_address);
            let range_end =
                to.or_else(|| bytes.map(|count| range_start.saturating_add(count as u64)));
            let instructions = filter_instructions(&result.instructions, range_start, range_end);
            let bytes_len = if from.is_none() && range_end.is_none() {
                result.bytes_len
            } else {
                infer_bytes_len(range_start, range_end, &instructions)
            };

            output::print_disassembly(
                output::DisassemblyView {
                    target: &result.target,
                    start_address: range_start,
                    bytes_len,
                    instructions: &instructions,
                },
                output::DisassemblyRenderOptions {
                    include_annotations: !no_annotations,
                    include_references: show_references,
                },
                &output_settings,
            );
        }
    }

    Ok(())
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
            instruction.address >= from && to.map_or(true, |end| instruction.address < end)
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
