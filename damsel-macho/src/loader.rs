use crate::dyld::{DyldAnalysis, collect_dyld_metadata, detect_platform};
use crate::errors::{MachoError, Result};
use crate::objc::collect_objc_metadata;
use damsel_core::{
    Architecture, BinaryFormat, BinaryImage, BinarySource, Endianness, Import, Platform,
    Relocation, Section, Segment, SliceDescriptor, SliceInfo, Symbol, SymbolKind,
};
use goblin::mach::Mach;
use object::macho::{
    CPU_SUBTYPE_ARM64_ALL, CPU_SUBTYPE_ARM64E, CPU_SUBTYPE_MASK, S_ATTR_PURE_INSTRUCTIONS,
    S_ATTR_SOME_INSTRUCTIONS,
};
use object::read::macho::{FatArch, MachOFatFile32, MachOFatFile64};
use object::{
    Object, ObjectSection, ObjectSegment, ObjectSymbol, RelocationTarget, SectionFlags, SymbolFlags,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

pub fn load<P: AsRef<Path>>(path: P) -> Result<BinaryImage> {
    let path = path.as_ref().to_path_buf();
    let bytes_arc: Arc<[u8]> = std::fs::read(&path)?.into();
    let bytes: &[u8] = bytes_arc.as_ref();
    let slice = select_slice(bytes)?;
    let slice_range = checked_range(slice.offset, slice.size, bytes.len() as u64)?;
    let slice_bytes = &bytes[slice_range];
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
    let slice_len = slice_bytes.len() as u64;
    let segments = collect_segments(&object_file, slice_len);
    let sections = collect_sections(&object_file, slice_len);
    let symbols = collect_symbols(&object_file);
    let mut imports = collect_imports(&object_file, &goblin_mach)?;
    let relocations = collect_relocations(&object_file)?;
    let objc = collect_objc_metadata(slice_bytes, &sections);
    let mut dyld = collect_dyld_metadata(&goblin_mach, slice_bytes, &segments)?;
    rebase_stub_addresses(&mut dyld, &sections);
    merge_import_hints(&mut imports, &dyld);
    let platform = detect_platform(&goblin_mach.load_commands)
        .as_deref()
        .map(map_platform);

    let available_slices = vec![SliceDescriptor::from_selected_slice(&slice, architecture.clone())];
    Ok(BinaryImage::new(
        BinarySource::File(path.clone()),
        path,
        BinaryFormat::MachO,
        architecture,
        endianness,
        Some(goblin_mach.entry),
        platform,
        slice,
        available_slices,
        segments,
        sections,
        symbols,
        imports,
        relocations,
        objc,
        dyld.metadata,
        bytes_arc,
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
            choose_fat_arch(fat.arches(), bytes.len() as u64)
        }
        object::FileKind::MachOFat64 => {
            let fat = MachOFatFile64::parse(bytes)?;
            choose_fat_arch(fat.arches(), bytes.len() as u64)
        }
        other => Err(MachoError::UnsupportedFileKind(format!("{other:?}"))),
    }
}

fn choose_fat_arch<Fat: FatArch>(arches: &[Fat], file_len: u64) -> Result<SliceInfo> {
    let selected = arches
        .iter()
        .filter(|arch| arch.architecture() == object::Architecture::Aarch64)
        .max_by_key(|arch| arm64_subtype_rank(arch.cpusubtype() & !CPU_SUBTYPE_MASK))
        .ok_or_else(|| MachoError::UnsupportedArchitecture("missing arm64 slice".to_string()))?;
    let (offset, size) = selected.file_range();
    if size == 0 {
        return Err(MachoError::MalformedFatBinary(
            "selected arm64 slice has zero size".to_string(),
        ));
    }
    let end = offset
        .checked_add(size)
        .ok_or(MachoError::SliceOutOfBounds {
            offset,
            size,
            file_len,
        })?;
    if end > file_len {
        return Err(MachoError::SliceOutOfBounds {
            offset,
            size,
            file_len,
        });
    }

    Ok(SliceInfo {
        offset,
        size,
        is_universal: true,
        cpu_subtype: selected.cpusubtype() & !CPU_SUBTYPE_MASK,
    })
}

fn checked_range(offset: u64, size: u64, file_len: u64) -> Result<std::ops::Range<usize>> {
    let end = offset
        .checked_add(size)
        .ok_or(MachoError::SliceOutOfBounds {
            offset,
            size,
            file_len,
        })?;
    if end > file_len {
        return Err(MachoError::SliceOutOfBounds {
            offset,
            size,
            file_len,
        });
    }

    let start = usize::try_from(offset).map_err(|_| MachoError::SliceOutOfBounds {
        offset,
        size,
        file_len,
    })?;
    let end = usize::try_from(end).map_err(|_| MachoError::SliceOutOfBounds {
        offset,
        size,
        file_len,
    })?;
    Ok(start..end)
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

fn map_platform(platform: &str) -> Platform {
    match platform {
        "macos" => Platform::MacOS,
        "ios" => Platform::IOS,
        "tvos" => Platform::TVOS,
        "watchos" => Platform::WatchOS,
        "maccatalyst" => Platform::MacCatalyst,
        "driverkit" => Platform::DriverKit,
        "visionos" => Platform::VisionOS,
        "visionos-simulator" => Platform::VisionOSSimulator,
        _ => Platform::Unknown(platform.to_string()),
    }
}

fn collect_segments<'a>(file: &object::File<'a, &'a [u8]>, file_len: u64) -> Vec<Segment> {
    file.segments()
        .map(|segment| {
            let (file_offset, raw_file_size) = segment.file_range();
            let file_size = clamped_file_size(file_offset, raw_file_size, file_len);
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

fn collect_sections<'a>(file: &object::File<'a, &'a [u8]>, file_len: u64) -> Vec<Section> {
    file.sections()
        .map(|section| {
            let (raw_file_offset, raw_file_size) = section.file_range().unwrap_or((0, 0));
            let file_size = clamped_file_size(raw_file_offset, raw_file_size, file_len);
            let file_offset = if file_size == 0 {
                None
            } else {
                Some(raw_file_offset)
            };
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
                size: section.size().min(file_size),
                file_offset,
                file_size,
                kind: format!("{:?}", section.kind()),
                executable,
            }
        })
        .collect()
}

fn clamped_file_size(file_offset: u64, file_size: u64, file_len: u64) -> u64 {
    if file_offset >= file_len {
        return 0;
    }
    let remaining = file_len.saturating_sub(file_offset);
    file_size.min(remaining)
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
        let has_resolved =
            imports
                .keys()
                .any(|(candidate_dylib, candidate_name, candidate_address)| {
                    candidate_dylib == &dylib
                        && candidate_name == &name
                        && candidate_address.is_some()
                });
        if has_resolved {
            continue;
        }
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

fn merge_import_hints(imports: &mut Vec<Import>, dyld: &DyldAnalysis) {
    for hint in &dyld.import_hints {
        if let Some(existing) = imports.iter_mut().find(|import| {
            import.name == hint.name
                && import.dylib == hint.dylib
                && import.address == Some(hint.address)
        }) {
            existing.offset = existing.offset.or(hint.offset);
            existing.addend = hint.addend;
            existing.is_weak |= hint.is_weak;
            continue;
        }

        if let Some(existing) = imports.iter_mut().find(|import| {
            import.name == hint.name && import.dylib == hint.dylib && import.address.is_none()
        }) {
            existing.address = Some(hint.address);
            existing.offset = hint.offset;
            existing.addend = hint.addend;
            existing.is_weak |= hint.is_weak;
            continue;
        }

        imports.push(Import {
            name: hint.name.clone(),
            dylib: hint.dylib.clone(),
            address: Some(hint.address),
            offset: hint.offset,
            addend: hint.addend,
            is_lazy: false,
            is_weak: hint.is_weak,
        });
    }

    let mut resolved_names = BTreeSet::new();
    for import in imports.iter().filter(|import| import.address.is_some()) {
        resolved_names.insert((import.dylib.clone(), import.name.clone()));
    }
    imports.retain(|import| {
        import.address.is_some()
            || !resolved_names.contains(&(import.dylib.clone(), import.name.clone()))
    });

    imports.sort_by_key(|import| {
        (
            import.address.unwrap_or_default(),
            import.dylib.clone(),
            import.name.clone(),
        )
    });
    imports.dedup_by(|left, right| {
        left.address == right.address && left.dylib == right.dylib && left.name == right.name
    });
}

fn rebase_stub_addresses(dyld: &mut DyldAnalysis, sections: &[Section]) {
    let Some(stubs_section) = sections
        .iter()
        .find(|section| section.segment_name == "__TEXT" && section.name == "__stubs")
    else {
        return;
    };

    let stub_count = dyld.metadata.stubs.len();
    if stub_count == 0 {
        return;
    }

    let inferred_stub_size = if stubs_section.size >= stub_count as u64
        && stubs_section.size % stub_count as u64 == 0
    {
        stubs_section.size / stub_count as u64
    } else {
        12
    };

    for (index, stub) in dyld.metadata.stubs.iter_mut().enumerate() {
        let stub_address = stubs_section
            .address
            .saturating_add((index as u64).saturating_mul(inferred_stub_size));
        stub.stub_address = stub_address;
        stub.source = damsel_core::ImportBindingSource::Stub;
    }
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
