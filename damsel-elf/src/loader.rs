//! ELF64 loader (Wave 5B.1): `object` crate typed ELF backend, own model
//! types. Typed failure taxonomy mirrors the Mach-O loader discipline.
//!
//! v1 scope: ELF64 little-endian (ELFDATA2LSB), ET_EXEC/ET_DYN. Big-endian
//! and ELF32 are typed rejections. The segment-only `PT_DYNAMIC` walker
//! for section-header-stripped binaries lands with the `elf-nosections`
//! fixture unit (Research Sources §0 Round 3).

use crate::image::{ElfDynamicEntry, ElfImage, ElfSection, ElfSegment, ElfSymbol};
use damsel_core::Architecture;
use object::elf::FileHeader64;
use object::read::elf::{ElfFile, FileHeader};
use object::{Endianness, Object, ObjectSection, ObjectSegment, ObjectSymbol};
use std::path::Path;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ElfError>;

#[derive(Debug, Error)]
pub enum ElfError {
    #[error("input is not an ELF file (magic mismatch): {0}")]
    NotAnElf(String),
    #[error("unsupported ELF layout: {0}")]
    UnsupportedLayout(String),
    #[error("malformed ELF payload: {0}")]
    Malformed(String),
    #[error("io error reading ELF: {0}")]
    Io(#[from] std::io::Error),
}

type Header = FileHeader64<Endianness>;

pub fn load<P: AsRef<Path>>(path: P) -> Result<ElfImage> {
    let bytes = std::fs::read(path.as_ref())?;
    let label = path
        .as_ref()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    load_bytes(label, bytes)
}

pub fn load_bytes(label: Option<String>, bytes: Vec<u8>) -> Result<ElfImage> {
    if bytes.len() < 16 || &bytes[0..4] != b"\x7fELF" {
        return Err(ElfError::NotAnElf(
            label.unwrap_or_else(|| "<memory>".to_string()),
        ));
    }
    if bytes.get(4) != Some(&2) {
        return Err(ElfError::UnsupportedLayout(format!(
            "ELF class {} (only ELF64 is supported)",
            bytes.get(4).copied().unwrap_or(0)
        )));
    }
    if bytes.get(5) != Some(&1) {
        return Err(ElfError::UnsupportedLayout(
            "only little-endian ELF64 is supported (ELFDATA2LSB)".to_string(),
        ));
    }

    let header = Header::parse(bytes.as_slice())
        .map_err(|e| ElfError::Malformed(format!("header: {e}")))?;
    let endian = header.endian().map_err(|e| ElfError::Malformed(e.to_string()))?;

    let architecture = map_architecture(header.e_machine(endian))?;
    let is_position_independent = header.e_type(endian) == object::elf::ET_DYN;

    // Hard-stripped binaries (no section headers) take the segment-only
    // PT_DYNAMIC walker (Wave 5B.3) — sections/dynsym tables are absent
    // and `object` cannot fill them (Research Sources §0 Round 3).
    let sectionless = header.e_shoff(endian) == 0 || header.e_shnum(endian) == 0;
    if sectionless {
        let result = build_segment_only(
            label,
            architecture,
            is_position_independent,
            bytes.as_slice(),
            header,
            endian,
        );
        return result;
    }

    let file = ElfFile::<Header, _>::parse(bytes.as_slice())
        .map_err(|e| ElfError::Malformed(e.to_string()))?;

    // Segments via typed headers (raw p_* fields).
    let mut segments = Vec::new();
    for segment in file.segments() {
        let (executable, writable) = match segment.flags() {
            object::SegmentFlags::Elf { p_flags } => (p_flags & 1 != 0, p_flags & 2 != 0),
            _ => (false, false),
        };
        let kind = "load"; // PT type refinement lands with the PT_DYNAMIC walker unit
        segments.push(ElfSegment {
            kind: kind.to_string(),
            executable,
            writable,
            address: segment.address(),
            size: segment.size(),
            file_offset: segment.file_range().0,
        });
    }

    let mut sections = Vec::new();
    for section in file.sections() {
        let name = section.name().unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let sh_flags = match section.flags() {
            object::SectionFlags::Elf { sh_flags } => sh_flags,
            _ => 0,
        };
        let executable = sh_flags & u64::from(object::elf::SHF_EXECINSTR) != 0;
        let kind = match section.kind() {
            object::SectionKind::Text => "text",
            object::SectionKind::Data => "data",
            object::SectionKind::ReadOnlyData => "rodata",
            object::SectionKind::ReadOnlyDataWithRel => "data_rel_ro",
            object::SectionKind::Debug => "debug",
            _ => "other",
        };
        sections.push(ElfSection {
            name,
            address: section.address(),
            size: section.size(),
            kind: kind.to_string(),
            executable,
        });
    }

    // Symbols: dynamic first (linkage truth), then static symtab.
    // `.gnu.version` resolution lands with the versioned-symbols unit.
    let mut symbols = Vec::new();
    for (table, dynamic) in [(file.dynamic_symbols(), true), (file.symbols(), false)] {
        for symbol in table {
            let Ok(name_bytes) = symbol.name_bytes() else { continue };
            let name = String::from_utf8_lossy(name_bytes).into_owned();
            if name.is_empty() {
                continue;
            }
            let defined = !symbol.is_undefined();
            let address = symbol.address();
            let exec = file.sections().any(|s| {
                let flags = match s.flags() {
                    object::SectionFlags::Elf { sh_flags } => sh_flags,
                    _ => 0,
                };
                s.address() <= address
                    && address < s.address().saturating_add(s.size())
                    && flags & u64::from(object::elf::SHF_EXECINSTR) != 0
            });
            symbols.push(ElfSymbol {
                name,
                address,
                size: symbol.size(),
                defined,
                is_text: defined && exec,
                dynamic,
                is_import: !defined,
                version: None,
            });
        }
    }

    // Dynamic entries of interest (typed DynamicTable).
    let mut dynamic = Vec::new();
    let dynamic_table = file
        .elf_dynamic_table()
        .map_err(|e| ElfError::Malformed(format!("dynamic table: {e}")))?;
    for entry in dynamic_table.iter() {
        if !entry.is_string() {
            continue;
        }
        let Ok(value_bytes) = entry.string(dynamic_table.strings()) else { continue };
        let Ok(value) = std::str::from_utf8(value_bytes) else { continue };
        match entry.tag {
            object::elf::DT_NEEDED => dynamic.push(ElfDynamicEntry::Needed(value.to_string())),
            object::elf::DT_SONAME => dynamic.push(ElfDynamicEntry::Soname(value.to_string())),
            object::elf::DT_RPATH => dynamic.push(ElfDynamicEntry::Rpath(value.to_string())),
            object::elf::DT_RUNPATH => dynamic.push(ElfDynamicEntry::Runpath(value.to_string())),
            _ => {}
        }
    }
    let soname = dynamic.iter().find_map(|e| match e {
        ElfDynamicEntry::Soname(s) => Some(s.clone()),
        _ => None,
    });

    // Build-id via the Object trait (sections, PT_NOTE fallback).
    let build_id = file
        .build_id()
        .ok()
        .flatten()
        .map(|desc| desc.iter().map(|b| format!("{b:02x}")).collect());

    // PT_LOAD mapping via raw program headers (single source with the
    // segment-only path).
    let raw_loads = crate::walker::parse_segments(bytes.as_slice(), header.e_phoff(endian), header.e_phnum(endian))
        .unwrap_or_default();
    let loads: Vec<(u64, u64, u64)> = raw_loads
        .iter()
        .filter(|s| s.p_type == object::elf::PT_LOAD)
        .map(|s| (s.p_vaddr, s.p_filesz, s.p_offset))
        .collect();
    Ok(ElfImage {
        label,
        data: std::sync::Arc::from(bytes.into_boxed_slice()),
        loads,
        architecture,
        is_position_independent,
        build_id,
        soname,
        segments,
        sections,
        symbols,
        dynamic,
    })
}

fn build_segment_only(
    label: Option<String>,
    architecture: Architecture,
    is_position_independent: bool,
    bytes: &[u8],
    header: &Header,
    endian: <Header as FileHeader>::Endian,
) -> Result<ElfImage> {
    let phoff = header.e_phoff(endian);
    let phnum = header.e_phnum(endian);
    let raw = crate::walker::parse_segments(bytes, phoff, phnum)
        .ok_or_else(|| ElfError::Malformed("program headers out of range".to_string()))?;

    let mut segments = Vec::new();
    for segment in &raw {
        let kind = match segment.p_type {
            object::elf::PT_LOAD => "load",
            object::elf::PT_DYNAMIC => "dynamic",
            object::elf::PT_NOTE => "note",
            object::elf::PT_INTERP => "interp",
            object::elf::PT_GNU_EH_FRAME => "eh_frame_hdr",
            object::elf::PT_PHDR => "phdr",
            _ => "other",
        };
        segments.push(ElfSegment {
            kind: kind.to_string(),
            executable: segment.p_flags & 1 != 0,
            writable: segment.p_flags & 2 != 0,
            address: segment.p_vaddr,
            size: segment.p_filesz,
            file_offset: segment.p_offset,
        });
    }

    let walked = crate::walker::walk_dynamic(bytes, &raw);
    let (symbols, dynamic) = match walked {
        Some(crate::walker::SegmentOnlyDynamic { symbols, entries }) => (symbols, entries),
        None => (Vec::new(), Vec::new()),
    };
    let soname = dynamic.iter().find_map(|e| match e {
        ElfDynamicEntry::Soname(s) => Some(s.clone()),
        _ => None,
    });

    let loads: Vec<(u64, u64, u64)> = raw
        .iter()
        .filter(|s| s.p_type == object::elf::PT_LOAD)
        .map(|s| (s.p_vaddr, s.p_filesz, s.p_offset))
        .collect();
    Ok(ElfImage {
        label,
        data: std::sync::Arc::from(bytes.to_vec().into_boxed_slice()),
        loads,
        architecture,
        is_position_independent,
        build_id: None, // PT_NOTE walking for build-id lands with the note unit
        soname,
        segments,
        sections: Vec::new(),
        symbols,
        dynamic,
    })
}

fn map_architecture(machine: u16) -> Result<Architecture> {
    match machine {
        object::elf::EM_X86_64 => Ok(Architecture::X86_64),
        object::elf::EM_AARCH64 => Ok(Architecture::Arm64),
        other => Err(ElfError::UnsupportedLayout(format!(
            "unsupported ELF machine type {other}"
        ))),
    }
}
