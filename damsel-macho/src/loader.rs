use crate::dyld::{collect_dyld_metadata, detect_platform};
use crate::errors::{MachoError, Result};
use crate::objc::collect_objc_metadata;
use damsel_core::{
    Architecture, BinaryFormat, BinaryImage, Endianness, Import, Relocation, Section, Segment,
    SliceInfo, Symbol, SymbolKind,
};
use goblin::mach::Mach;
use memmap2::Mmap;
use object::macho::{
    CPU_SUBTYPE_ARM64_ALL, CPU_SUBTYPE_ARM64E, CPU_SUBTYPE_MASK, S_ATTR_PURE_INSTRUCTIONS,
    S_ATTR_SOME_INSTRUCTIONS,
};
use object::read::macho::{FatArch, MachOFatFile32, MachOFatFile64};
use object::{
    Object, ObjectSection, ObjectSegment, ObjectSymbol, RelocationTarget, SectionFlags, SymbolFlags,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

pub fn load<P: AsRef<Path>>(path: P) -> Result<BinaryImage> {
    let path = path.as_ref().to_path_buf();
    let file = File::open(&path)?;
    let mmap = Arc::new(unsafe { Mmap::map(&file)? });
    let bytes: &[u8] = mmap.as_ref();
    let slice = select_slice(bytes)?;
    let slice_bytes = &bytes[slice.offset as usize..(slice.offset + slice.size) as usize];
    let object_file = object::File::parse(slice_bytes)?;
    let goblin_mach = match Mach::parse(slice_bytes)? {
        Mach::Binary(binary) => binary,
        Mach::Fat(_) => {
            return Err(MachoError::UnsupportedFileKind(
                "unexpected nested fat Mach-O".to_string(),
            ));
        }
    };

    let architecture = map_architecture(goblin_mach.header.cputype, goblin_mach.header.cpusubtype)?;
    let endianness = if goblin_mach.little_endian {
        Endianness::Little
    } else {
        Endianness::Big
    };
    let segments = collect_segments(&object_file);
    let sections = collect_sections(&object_file);
    let symbols = collect_symbols(&object_file);
    let imports = collect_imports(&object_file, &goblin_mach)?;
    let relocations = collect_relocations(&object_file)?;
    let objc = collect_objc_metadata(slice_bytes, &sections);
    let dyld = collect_dyld_metadata(&goblin_mach, slice_bytes, &segments)?;
    let platform = detect_platform(&goblin_mach.load_commands);

    Ok(BinaryImage::new(
        path,
        BinaryFormat::MachO,
        architecture,
        endianness,
        Some(goblin_mach.entry),
        platform,
        slice,
        segments,
        sections,
        symbols,
        imports,
        relocations,
        objc,
        dyld,
        mmap,
    ))
}

fn select_slice(bytes: &[u8]) -> Result<SliceInfo> {
    let kind = object::FileKind::parse(bytes)?;
    match kind {
        object::FileKind::MachO64 => {
            let mach = match Mach::parse(bytes)? {
                Mach::Binary(binary) => binary,
                Mach::Fat(_) => {
                    return Err(MachoError::UnsupportedFileKind(
                        "unexpected fat parse for thin Mach-O".to_string(),
                    ));
                }
            };

            Ok(SliceInfo {
                offset: 0,
                size: bytes.len() as u64,
                is_universal: false,
                cpu_subtype: mach.header.cpusubtype & !CPU_SUBTYPE_MASK,
            })
        }
        object::FileKind::MachOFat32 => {
            let fat = MachOFatFile32::parse(bytes)?;
            choose_fat_arch(fat.arches())
        }
        object::FileKind::MachOFat64 => {
            let fat = MachOFatFile64::parse(bytes)?;
            choose_fat_arch(fat.arches())
        }
        other => Err(MachoError::UnsupportedFileKind(format!("{other:?}"))),
    }
}

fn choose_fat_arch<Fat: FatArch>(arches: &[Fat]) -> Result<SliceInfo> {
    let selected = arches
        .iter()
        .filter(|arch| arch.architecture() == object::Architecture::Aarch64)
        .max_by_key(|arch| arm64_subtype_rank(arch.cpusubtype() & !CPU_SUBTYPE_MASK))
        .ok_or_else(|| MachoError::UnsupportedArchitecture("missing arm64 slice".to_string()))?;
    let (offset, size) = selected.file_range();

    Ok(SliceInfo {
        offset,
        size,
        is_universal: true,
        cpu_subtype: selected.cpusubtype() & !CPU_SUBTYPE_MASK,
    })
}

fn arm64_subtype_rank(subtype: u32) -> u8 {
    match subtype {
        CPU_SUBTYPE_ARM64E => 2,
        CPU_SUBTYPE_ARM64_ALL => 1,
        _ => 0,
    }
}

fn map_architecture(cputype: u32, cpusubtype: u32) -> Result<Architecture> {
    let subtype = cpusubtype & !CPU_SUBTYPE_MASK;
    match (cputype, subtype) {
        (object::macho::CPU_TYPE_ARM64, CPU_SUBTYPE_ARM64E) => Ok(Architecture::Arm64e),
        (object::macho::CPU_TYPE_ARM64, _) => Ok(Architecture::Arm64),
        _ => Err(MachoError::UnsupportedArchitecture(format!(
            "cputype={cputype:#x} subtype={subtype:#x}"
        ))),
    }
}

fn collect_segments<'a>(file: &object::File<'a, &'a [u8]>) -> Vec<Segment> {
    file.segments()
        .map(|segment| {
            let (file_offset, file_size) = segment.file_range();
            let permissions = segment.permissions();
            Segment {
                name: segment
                    .name()
                    .ok()
                    .flatten()
                    .unwrap_or_default()
                    .to_string(),
                address: segment.address(),
                size: segment.size(),
                file_offset,
                file_size,
                readable: permissions.readable(),
                writable: permissions.writable(),
                executable: permissions.executable(),
            }
        })
        .collect()
}

fn collect_sections<'a>(file: &object::File<'a, &'a [u8]>) -> Vec<Section> {
    file.sections()
        .map(|section| {
            let (_, file_size) = section.file_range().unwrap_or((0, 0));
            let executable = section.kind() == object::SectionKind::Text
                || matches!(
                    section.flags(),
                    SectionFlags::MachO { flags }
                        if flags & (S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS) != 0
                );

            Section {
                segment_name: section
                    .segment_name()
                    .ok()
                    .flatten()
                    .unwrap_or_default()
                    .to_string(),
                name: section.name().unwrap_or_default().to_string(),
                address: section.address(),
                size: section.size(),
                file_offset: section.file_range().map(|(offset, _)| offset),
                file_size,
                kind: format!("{:?}", section.kind()),
                executable,
            }
        })
        .collect()
}

fn collect_symbols<'a>(file: &object::File<'a, &'a [u8]>) -> Vec<Symbol> {
    let mut seen = BTreeSet::new();
    let mut collected = Vec::new();

    for symbol in file.symbols().chain(file.dynamic_symbols()) {
        let name = symbol.name().unwrap_or_default().to_string();
        let section = symbol
            .section_index()
            .and_then(|index| file.section_by_index(index).ok())
            .and_then(|section| section.name().ok().map(str::to_string));
        let fingerprint = (name.clone(), symbol.address(), symbol.is_undefined());
        if !seen.insert(fingerprint) {
            continue;
        }

        collected.push(Symbol {
            name,
            address: symbol.address(),
            size: symbol.size(),
            kind: map_symbol_kind(symbol.kind()),
            defined: !symbol.is_undefined(),
            global: symbol.is_global(),
            weak: symbol.is_weak()
                || matches!(symbol.flags(), SymbolFlags::MachO { n_desc } if n_desc & 0x40 != 0),
            section,
        });
    }

    collected.sort_by_key(|symbol| (symbol.address, symbol.name.clone()));
    collected
}

fn map_symbol_kind(kind: object::SymbolKind) -> SymbolKind {
    match kind {
        object::SymbolKind::Text => SymbolKind::Text,
        object::SymbolKind::Data => SymbolKind::Data,
        object::SymbolKind::Section => SymbolKind::Section,
        object::SymbolKind::File => SymbolKind::File,
        object::SymbolKind::Label => SymbolKind::Label,
        other => SymbolKind::Unknown(format!("{other:?}")),
    }
}

fn collect_imports<'a>(
    file: &object::File<'a, &'a [u8]>,
    macho: &goblin::mach::MachO<'_>,
) -> Result<Vec<Import>> {
    let dylib_fallback = macho.libs.iter().skip(1).next().copied().unwrap_or("");
    let mut imports = BTreeMap::<(String, String, Option<u64>), Import>::new();

    for import in macho.imports()? {
        let key = (
            import.dylib.to_string(),
            import.name.to_string(),
            Some(import.address),
        );
        imports.insert(
            key,
            Import {
                name: import.name.to_string(),
                dylib: import.dylib.to_string(),
                address: Some(import.address),
                offset: Some(import.offset as u64),
                addend: import.addend,
                is_lazy: import.is_lazy,
                is_weak: import.is_weak,
            },
        );
    }

    for import in file.imports()? {
        let dylib = if import.library().is_empty() {
            dylib_fallback.to_string()
        } else {
            String::from_utf8_lossy(import.library()).into_owned()
        };
        let name = String::from_utf8_lossy(import.name()).into_owned();
        let key = (dylib.clone(), name.clone(), None);
        imports.entry(key).or_insert(Import {
            name,
            dylib,
            address: None,
            offset: None,
            addend: 0,
            is_lazy: false,
            is_weak: false,
        });
    }

    let mut imports = imports.into_values().collect::<Vec<_>>();
    imports.sort_by_key(|import| {
        (
            import.address.unwrap_or_default(),
            import.dylib.clone(),
            import.name.clone(),
        )
    });
    Ok(imports)
}

fn collect_relocations<'a>(file: &object::File<'a, &'a [u8]>) -> Result<Vec<Relocation>> {
    let mut relocations = Vec::new();

    for section in file.sections() {
        let section_name = format!(
            "{}:{}",
            section.segment_name().ok().flatten().unwrap_or_default(),
            section.name().unwrap_or_default()
        );
        for (offset, relocation) in section.relocations() {
            let target = match relocation.target() {
                RelocationTarget::Symbol(index) => file
                    .symbol_by_index(index)
                    .ok()
                    .and_then(|symbol| symbol.name().ok().map(str::to_string))
                    .unwrap_or_else(|| format!("symbol#{index:?}")),
                RelocationTarget::Section(index) => file
                    .section_by_index(index)
                    .ok()
                    .and_then(|section| section.name().ok().map(str::to_string))
                    .unwrap_or_else(|| format!("section#{index:?}")),
                RelocationTarget::Absolute => "absolute".to_string(),
                other => format!("{other:?}"),
            };

            relocations.push(Relocation {
                section: section_name.clone(),
                address: section.address().saturating_add(offset),
                size: relocation.size(),
                kind: format!("{:?}", relocation.kind()),
                encoding: format!("{:?}", relocation.encoding()),
                target,
                addend: relocation.addend(),
            });
        }
    }

    relocations.sort_by_key(|relocation| relocation.address);
    Ok(relocations)
}
