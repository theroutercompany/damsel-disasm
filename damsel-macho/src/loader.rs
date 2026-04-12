use crate::dyld::{DyldAnalysis, collect_dyld_metadata, detect_platform};
use crate::errors::{MachoError, Result};
use crate::objc::collect_objc_metadata;
use damsel_core::{
    Architecture, BinaryFormat, BinaryImage, BinarySource, DyldMetadata, Endianness, Import,
    ImportBindingRecord, ImportBindingSource, Platform, Relocation, Section, Segment,
    SliceDescriptor, SliceInfo, StubEntry, Symbol, SymbolKind,
};
use goblin::mach::Mach;
use object::macho::{
    CPU_SUBTYPE_ARM64_ALL, CPU_SUBTYPE_ARM64E, CPU_SUBTYPE_MASK, INDIRECT_SYMBOL_ABS,
    INDIRECT_SYMBOL_LOCAL, S_ATTR_PURE_INSTRUCTIONS, S_ATTR_SOME_INSTRUCTIONS,
    S_LAZY_DYLIB_SYMBOL_POINTERS, S_LAZY_SYMBOL_POINTERS, S_NON_LAZY_SYMBOL_POINTERS,
    S_SYMBOL_STUBS,
};
use object::read::macho::{
    FatArch, MachHeader, MachOFatFile32, MachOFatFile64, MachOFile64, Section as RawMachOSection,
};
use object::{
    Object, ObjectSection, ObjectSegment, ObjectSymbol, RelocationTarget, SectionFlags, SymbolFlags,
    SymbolIndex,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

pub fn load<P: AsRef<Path>>(path: P) -> Result<BinaryImage> {
    let path = path.as_ref().to_path_buf();
    let bytes_arc: Arc<[u8]> = std::fs::read(&path)?.into();
    let bytes: &[u8] = bytes_arc.as_ref();
    let (slice, available_slices) = select_slice(bytes)?;
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

    let architecture =
        map_selected_architecture(goblin_mach.header.cputype, goblin_mach.header.cpusubtype)?;
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
    augment_dysymtab_bindings_and_stubs(slice_bytes, &goblin_mach, &imports, &mut dyld.metadata)?;
    merge_import_hints(&mut imports, &dyld);
    let platform = detect_platform(&goblin_mach.load_commands)
        .as_deref()
        .map(map_platform);

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

fn select_slice(bytes: &[u8]) -> Result<(SliceInfo, Vec<SliceDescriptor>)> {
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

            let slice = SliceInfo {
                offset: 0,
                size: bytes.len() as u64,
                is_universal: false,
                cpu_subtype: mach.header.cpusubtype & !CPU_SUBTYPE_MASK,
            };
            let architecture =
                map_selected_architecture(mach.header.cputype, mach.header.cpusubtype)?;
            Ok((
                slice.clone(),
                vec![SliceDescriptor::from_selected_slice(&slice, architecture)],
            ))
        }
        object::FileKind::MachOFat32 => {
            let fat = MachOFatFile32::parse(bytes)?;
            let slice = choose_fat_arch(fat.arches(), bytes.len() as u64)?;
            let available_slices = collect_fat_slice_descriptors(fat.arches(), bytes.len() as u64, &slice);
            Ok((slice, available_slices))
        }
        object::FileKind::MachOFat64 => {
            let fat = MachOFatFile64::parse(bytes)?;
            let slice = choose_fat_arch(fat.arches(), bytes.len() as u64)?;
            let available_slices = collect_fat_slice_descriptors(fat.arches(), bytes.len() as u64, &slice);
            Ok((slice, available_slices))
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

fn map_selected_architecture(cputype: u32, cpusubtype: u32) -> Result<Architecture> {
    let subtype = cpusubtype & !CPU_SUBTYPE_MASK;
    match (cputype, subtype) {
        (object::macho::CPU_TYPE_ARM64, CPU_SUBTYPE_ARM64E) => Ok(Architecture::Arm64e),
        (object::macho::CPU_TYPE_ARM64, _) => Ok(Architecture::Arm64),
        _ => Err(MachoError::UnsupportedArchitecture(format!(
            "cputype={cputype:#x} subtype={subtype:#x}"
        ))),
    }
}

fn map_slice_architecture(kind: object::Architecture, cpusubtype: u32) -> Option<Architecture> {
    let subtype = cpusubtype & !CPU_SUBTYPE_MASK;
    match kind {
        object::Architecture::Aarch64 => Some(if subtype == CPU_SUBTYPE_ARM64E {
            Architecture::Arm64e
        } else {
            Architecture::Arm64
        }),
        object::Architecture::X86_64 => Some(Architecture::X86_64),
        _ => None,
    }
}

fn collect_fat_slice_descriptors<Fat: FatArch>(
    arches: &[Fat],
    file_len: u64,
    selected: &SliceInfo,
) -> Vec<SliceDescriptor> {
    let mut descriptors = arches
        .iter()
        .filter_map(|arch| {
            let architecture = map_slice_architecture(arch.architecture(), arch.cpusubtype())?;
            let (offset, size) = arch.file_range();
            if checked_range(offset, size, file_len).is_err() {
                return None;
            }
            Some(SliceDescriptor {
                offset,
                size,
                is_universal: true,
                cpu_subtype: arch.cpusubtype() & !CPU_SUBTYPE_MASK,
                architecture,
                selected: offset == selected.offset
                    && size == selected.size
                    && (arch.cpusubtype() & !CPU_SUBTYPE_MASK) == selected.cpu_subtype,
            })
        })
        .collect::<Vec<_>>();

    descriptors.sort_by_key(|descriptor| {
        (
            !descriptor.selected,
            descriptor.offset,
            descriptor.size,
            descriptor.cpu_subtype,
        )
    });
    descriptors
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

fn augment_dysymtab_bindings_and_stubs(
    bytes: &[u8],
    macho: &goblin::mach::MachO<'_>,
    imports: &[Import],
    dyld: &mut DyldMetadata,
) -> Result<()> {
    let macho_file = MachOFile64::<object::Endianness, &[u8]>::parse(bytes)?;
    let endian = macho_file.macho_header().endian()?;
    let mut commands = macho_file.macho_load_commands()?;
    let mut indirect_symbols = None;
    while let Some(command) = commands.next()? {
        if let Some(dysymtab) = command.dysymtab()? {
            indirect_symbols = Some(dysymtab.indirect_symbols(endian, bytes)?);
            break;
        }
    }
    let Some(indirect_symbols) = indirect_symbols else {
        return Ok(());
    };

    let imports_by_name = imports
        .iter()
        .fold(BTreeMap::<String, &Import>::new(), |mut acc, import| {
            acc.entry(normalize_import_name(&import.name)).or_insert(import);
            acc
        });
    let imports_by_dylib_and_name =
        imports
            .iter()
            .fold(BTreeMap::<(String, String), &Import>::new(), |mut acc, import| {
                acc.entry((normalize_import_name(&import.name), import.dylib.clone()))
                    .or_insert(import);
                acc
            });
    let indirect_symbol_metadata = build_indirect_symbol_metadata(macho);

    let mut pointer_slots_by_name = BTreeMap::<String, Vec<(u64, Option<u64>, String)>>::new();
    for section in macho_file.sections() {
        let raw = section.macho_section();
        let section_type = raw.section_type(endian);
        if !matches!(
            section_type,
            S_NON_LAZY_SYMBOL_POINTERS | S_LAZY_SYMBOL_POINTERS | S_LAZY_DYLIB_SYMBOL_POINTERS
        ) {
            continue;
        }
        let section_name = section.name().unwrap_or_default().to_string();
        let segment_name = section.segment_name().ok().flatten().unwrap_or_default().to_string();
        let full_name = format!("{segment_name}:{section_name}");
        let entries = raw.indirect_symbols(endian, indirect_symbols)?;
        let (file_offset, _) = section.file_range().unwrap_or((0, 0));
        for (index, raw_symbol) in entries.iter().enumerate() {
            let symbol_index = raw_symbol.get(endian);
            if is_special_indirect_symbol(symbol_index) {
                continue;
            }
            let Some(resolution) = resolve_indirect_symbol(
                &macho_file,
                SymbolIndex(symbol_index as usize),
                &indirect_symbol_metadata,
                &imports_by_name,
                &imports_by_dylib_and_name,
            )
            else {
                continue;
            };
            let pointer_address = section.address().saturating_add((index as u64) * 8);
            let pointer_offset = Some(file_offset.saturating_add((index as u64) * 8));
            dyld.import_bindings.push(ImportBindingRecord {
                dylib: resolution.import.dylib.clone(),
                name: resolution.import.name.clone(),
                address: Some(pointer_address),
                offset: pointer_offset,
                addend: resolution.import.addend,
                ordinal: resolution.ordinal,
                symbol_index: Some(symbol_index),
                source: ImportBindingSource::IndirectSymbol,
                is_weak: resolution.import.is_weak,
            });
            pointer_slots_by_name
                .entry(resolution.normalized_name)
                .or_default()
                .push((pointer_address, pointer_offset, full_name.clone()));
        }
    }

    for section in macho_file.sections() {
        let raw = section.macho_section();
        if raw.section_type(endian) != S_SYMBOL_STUBS {
            continue;
        }
        let section_name = section.name().unwrap_or_default().to_string();
        let segment_name = section.segment_name().ok().flatten().unwrap_or_default().to_string();
        let full_name = format!("{segment_name}:{section_name}");
        let stub_size = u64::from(raw.symbol_stub_size(endian));
        if stub_size == 0 {
            continue;
        }
        let entries = raw.indirect_symbols(endian, indirect_symbols)?;
        for (index, raw_symbol) in entries.iter().enumerate() {
            let symbol_index = raw_symbol.get(endian);
            if is_special_indirect_symbol(symbol_index) {
                continue;
            }
            let Some(resolution) = resolve_indirect_symbol(
                &macho_file,
                SymbolIndex(symbol_index as usize),
                &indirect_symbol_metadata,
                &imports_by_name,
                &imports_by_dylib_and_name,
            )
            else {
                continue;
            };
            let stub_address = section.address().saturating_add((index as u64) * stub_size);
            let (pointer_address, _, _) = pointer_slots_by_name
                .get(&resolution.normalized_name)
                .and_then(|values| values.get(index).or_else(|| values.first()))
                .cloned()
                .unwrap_or((0, None, String::new()));
            let pointer_address = (pointer_address != 0).then_some(pointer_address);
            dyld.stubs.push(StubEntry {
                stub_address,
                section: Some(full_name.clone()),
                pointer_section: pointer_slots_by_name
                    .get(&resolution.normalized_name)
                    .and_then(|values| values.get(index).or_else(|| values.first()))
                    .map(|(_, _, section_name)| section_name.clone()),
                pointer_address,
                helper_address: None,
                binding_ordinal: resolution.ordinal,
                dylib: Some(resolution.import.dylib.clone()),
                name: Some(resolution.import.name.clone()),
                source: ImportBindingSource::Stub,
            });
        }
    }

    dyld.import_bindings.sort_by_key(|binding| {
        (
            binding.address.unwrap_or_default(),
            binding.offset.unwrap_or_default(),
            binding.dylib.clone(),
            binding.name.clone(),
            binding_source_rank(binding.source),
        )
    });
    dyld.import_bindings.dedup_by(|left, right| {
        left.address == right.address
            && left.offset == right.offset
            && left.dylib == right.dylib
            && left.name == right.name
            && left.addend == right.addend
    });

    if dyld.stubs.iter().any(|stub| stub.section.is_some()) {
        dyld.stubs.retain(|stub| stub.section.is_some());
    }

    dyld.stubs.sort_by_key(|stub| {
        (
            stub.stub_address,
            stub.pointer_address.unwrap_or_default(),
            stub.dylib.clone().unwrap_or_default(),
            stub.name.clone().unwrap_or_default(),
        )
    });
    dyld.stubs.dedup();
    Ok(())
}

fn is_special_indirect_symbol(symbol_index: u32) -> bool {
    symbol_index == INDIRECT_SYMBOL_LOCAL
        || symbol_index == (INDIRECT_SYMBOL_LOCAL | INDIRECT_SYMBOL_ABS)
}

fn binding_source_rank(source: ImportBindingSource) -> u8 {
    match source {
        ImportBindingSource::IndirectSymbol => 0,
        ImportBindingSource::ChainedFixup => 1,
        ImportBindingSource::Stub => 2,
        ImportBindingSource::Other => 3,
    }
}

fn normalize_import_name(name: &str) -> String {
    name.trim_start_matches('_').to_ascii_lowercase()
}

#[derive(Debug, Clone)]
struct IndirectSymbolMetadata {
    normalized_name: String,
    ordinal: Option<u32>,
    dylib: Option<String>,
}

#[derive(Debug)]
struct ResolvedIndirectSymbol<'a> {
    normalized_name: String,
    ordinal: Option<u32>,
    import: &'a Import,
}

fn build_indirect_symbol_metadata(
    macho: &goblin::mach::MachO<'_>,
) -> BTreeMap<usize, IndirectSymbolMetadata> {
    let mut result = BTreeMap::new();
    let Some(symbols) = macho.symbols.as_ref() else {
        return result;
    };
    let Some(dysymtab) = macho.load_commands.iter().find_map(|command| match &command.command {
        goblin::mach::load_command::CommandVariant::Dysymtab(command) => Some(command),
        _ => None,
    }) else {
        return result;
    };

    let start = dysymtab.iundefsym as usize;
    let count = dysymtab.nundefsym as usize;
    for symbol_index in start..start.saturating_add(count) {
        let Ok((name, nlist)) = symbols.get(symbol_index) else {
            continue;
        };
        let ordinal = ((nlist.n_desc >> 8) & 0xff) as u32;
        let dylib = resolve_dylib_name_from_ordinal(macho.libs.as_slice(), ordinal);
        result.insert(
            symbol_index,
            IndirectSymbolMetadata {
                normalized_name: normalize_import_name(name),
                ordinal: Some(ordinal),
                dylib,
            },
        );
    }
    result
}

fn resolve_dylib_name_from_ordinal(libs: &[&str], ordinal: u32) -> Option<String> {
    libs.get(ordinal as usize)
        .copied()
        .filter(|name| !name.is_empty() && *name != "self")
        .map(ToString::to_string)
}

fn resolve_indirect_symbol<'a>(
    macho_file: &'a MachOFile64<'a, object::Endianness>,
    symbol_index: SymbolIndex,
    metadata: &BTreeMap<usize, IndirectSymbolMetadata>,
    imports_by_name: &BTreeMap<String, &'a Import>,
    imports_by_dylib_and_name: &BTreeMap<(String, String), &'a Import>,
) -> Option<ResolvedIndirectSymbol<'a>> {
    let metadata_entry = metadata.get(&symbol_index.0);
    let (normalized_name, ordinal, dylib_name) = if let Some(entry) = metadata_entry {
        (
            entry.normalized_name.clone(),
            entry.ordinal,
            entry.dylib.clone(),
        )
    } else {
        let symbol = macho_file.symbol_by_index(symbol_index).ok()?;
        let raw_name = symbol.name().ok()?.to_string();
        (normalize_import_name(&raw_name), None, None)
    };

    let import = dylib_name
        .as_ref()
        .and_then(|dylib| imports_by_dylib_and_name.get(&(normalized_name.clone(), dylib.clone())).copied())
        .or_else(|| imports_by_name.get(&normalized_name).copied())?;
    Some(ResolvedIndirectSymbol {
        normalized_name,
        ordinal,
        import,
    })
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
