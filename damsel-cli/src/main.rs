use clap::{ArgGroup, Parser, Subcommand};
use damsel_core::{BinaryImage, DisassemblyRequest, DisassemblyTarget};
use damsel_macho::{disassemble, load};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "damsel", about = "Static Mach-O arm64 disassembler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Info {
        path: PathBuf,
    },
    Sections {
        path: PathBuf,
    },
    Symbols {
        path: PathBuf,
    },
    Imports {
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
        limit: Option<usize>,
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

    match cli.command {
        Command::Info { path } => print_info(&load(path)?),
        Command::Sections { path } => print_sections(&load(path)?),
        Command::Symbols { path } => print_symbols(&load(path)?),
        Command::Imports { path } => print_imports(&load(path)?),
        Command::Objc { path } => print_objc(&load(path)?),
        Command::Disasm {
            path,
            symbol,
            addr,
            section,
            limit,
        } => {
            let image = load(path)?;
            let target = if let Some(symbol) = symbol {
                DisassemblyTarget::Symbol(symbol)
            } else if let Some(addr) = addr {
                DisassemblyTarget::Address(addr)
            } else {
                DisassemblyTarget::Section(section.expect("section target"))
            };
            let result = disassemble(
                &image,
                &DisassemblyRequest {
                    target,
                    max_instructions: limit,
                },
            )?;
            print_disassembly(&result);
        }
    }

    Ok(())
}

fn print_info(image: &BinaryImage) {
    println!("path: {}", image.path.display());
    println!("format: {:?}", image.format);
    println!("architecture: {}", image.architecture);
    println!("endianness: {:?}", image.endianness);
    if let Some(entry) = image.entry_point {
        println!("entry: {entry:#x}");
    }
    if let Some(platform) = &image.platform {
        println!("platform: {platform}");
    }
    println!(
        "slice: offset={:#x} size={:#x} universal={} subtype={:#x}",
        image.slice.offset, image.slice.size, image.slice.is_universal, image.slice.cpu_subtype
    );
    println!(
        "counts: segments={} sections={} symbols={} imports={} relocations={}",
        image.segments.len(),
        image.sections.len(),
        image.symbols.len(),
        image.imports.len(),
        image.relocations.len()
    );
    println!(
        "objc: classes={} selectors={} methods={}",
        image.objc.class_names.len(),
        image.objc.selector_names.len(),
        image.objc.method_names.len()
    );
    println!(
        "dyld: dylibs={} rpaths={} exports={} function_starts={} rebases={} binds={} chained_fixups={}",
        image.dyld.imported_dylibs.len(),
        image.dyld.rpaths.len(),
        image.dyld.exported_symbols.len(),
        image.dyld.function_starts.len(),
        image.dyld.has_rebases,
        image.dyld.has_binds,
        image.dyld.has_chained_fixups
    );
    if !image.dyld.imported_dylibs.is_empty() {
        println!("dylibs:");
        for dylib in &image.dyld.imported_dylibs {
            println!("  {dylib}");
        }
    }
}

fn print_sections(image: &BinaryImage) {
    for section in &image.sections {
        println!(
            "{:>#18x} {:>#8x} {:<8} {:<18} {}",
            section.address,
            section.size,
            if section.executable { "exec" } else { "-" },
            section.segment_name,
            section.name
        );
    }
}

fn print_symbols(image: &BinaryImage) {
    for symbol in &image.symbols {
        println!(
            "{:>#18x} {:>#8x} {:<8} {:<6} {:<5} {}{}",
            symbol.address,
            symbol.size,
            symbol.kind,
            if symbol.defined { "def" } else { "undef" },
            if symbol.global { "glob" } else { "loc" },
            symbol.name,
            symbol
                .section
                .as_ref()
                .map(|section| format!(" [{section}]"))
                .unwrap_or_default()
        );
    }
}

fn print_imports(image: &BinaryImage) {
    for import in &image.imports {
        println!(
            "{:>#18} {:<5} {:<5} {:<30} {}",
            import
                .address
                .map(|address| format!("{address:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            if import.is_lazy { "lazy" } else { "-" },
            if import.is_weak { "weak" } else { "-" },
            import.dylib,
            import.name
        );
    }
}

fn print_objc(image: &BinaryImage) {
    if let Some(flags) = image.objc.image_info_flags {
        println!("image_info_flags: {flags:#x}");
    }
    println!("classes:");
    for class_name in &image.objc.class_names {
        println!("  {class_name}");
    }
    println!("selectors:");
    for selector in &image.objc.selector_names {
        println!("  {selector}");
    }
    println!("methods:");
    for method in &image.objc.method_names {
        println!("  {method}");
    }
}

fn print_disassembly(result: &damsel_core::DisassemblyResult) {
    println!(
        "target: {} start={:#x} bytes={:#x}",
        result.target, result.start_address, result.bytes_len
    );
    for instruction in &result.instructions {
        let rendered = instruction.render();
        let annotations = instruction
            .annotations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if annotations.is_empty() {
            println!(
                "{:>#18x}  {:08x}  {}",
                instruction.address, instruction.opcode, rendered
            );
        } else {
            println!(
                "{:>#18x}  {:08x}  {} ; {}",
                instruction.address,
                instruction.opcode,
                rendered,
                annotations.join(" | ")
            );
        }
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
