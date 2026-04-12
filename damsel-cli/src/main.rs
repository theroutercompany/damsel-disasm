mod output;

use clap::{ArgGroup, Parser, Subcommand};
use damsel_core::{DisassemblyRequest, DisassemblyTarget};
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
        Command::Info { path } => output::print_info(&load(path)?),
        Command::Sections { path } => output::print_sections(&load(path)?),
        Command::Symbols { path } => output::print_symbols(&load(path)?),
        Command::Imports { path } => output::print_imports(&load(path)?),
        Command::Objc { path } => output::print_objc(&load(path)?),
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
            output::print_disassembly(&result);
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
