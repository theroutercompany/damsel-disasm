use crate::dyld::{DyldAnalysis, collect_dyld_metadata, detect_platform};
use crate::errors::{MachoError, Result};
use crate::objc::collect_objc_metadata;
use damsel_core::{
    Architecture, BinaryFormat, BinaryImage, BinarySource, DyldMetadata, Endianness, Import,
    ImportBindingKind, ImportBindingRecord, ImportBindingSource, Platform, Relocation, Section,
    Segment, SliceDescriptor, SliceInfo, StubEntry, StubHelperEntry, StubKind, Symbol, SymbolKind,
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
    Object, ObjectSection, ObjectSegment, ObjectSymbol, RelocationTarget, SectionFlags,
    SymbolFlags, SymbolIndex,
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
    let objc = collect_objc_metadata(slice_bytes, &sections, &symbols);
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
            let available_slices =
                collect_fat_slice_descriptors(fat.arches(), bytes.len() as u64, &slice);
            Ok((slice, available_slices))
        }
        object::FileKind::MachOFat64 => {
            let fat = MachOFatFile64::parse(bytes)?;
            let slice = choose_fat_arch(fat.arches(), bytes.len() as u64)?;
            let available_slices =
                collect_fat_slice_descriptors(fat.arches(), bytes.len() as u64, &slice);
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

    let imports_by_name = imports.iter().fold(
        BTreeMap::<String, Vec<&Import>>::new(),
        |mut acc, import| {
            acc.entry(normalize_import_name(&import.name))
                .or_default()
                .push(import);
            acc
        },
    );
    let imports_by_dylib_and_name = imports.iter().fold(
        BTreeMap::<(String, String), &Import>::new(),
        |mut acc, import| {
            acc.entry((normalize_import_name(&import.name), import.dylib.clone()))
                .or_insert(import);
            acc
        },
    );
    let lazy_bindings_by_sequence = macho
        .imports()?
        .into_iter()
        .filter(|import| import.is_lazy)
        .map(|import| {
            (
                u32::try_from(import.start_of_sequence_offset).unwrap_or(u32::MAX),
                LazyBindMetadata {
                    address: import.address,
                    name: import.name.to_string(),
                    dylib: import.dylib.to_string(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let indirect_symbol_metadata = build_indirect_symbol_metadata(macho);

    let mut pointer_slots_by_symbol_index = BTreeMap::<u32, Vec<PointerSlotMetadata>>::new();
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
        let segment_name = section
            .segment_name()
            .ok()
            .flatten()
            .unwrap_or_default()
            .to_string();
        let full_name = format!("{segment_name}:{section_name}");
        let binding_kind = match section_type {
            S_NON_LAZY_SYMBOL_POINTERS => PointerBindingKind::NonLazy,
            S_LAZY_SYMBOL_POINTERS | S_LAZY_DYLIB_SYMBOL_POINTERS => PointerBindingKind::Lazy,
            _ => continue,
        };
        let entries = raw.indirect_symbols(endian, indirect_symbols)?;
        let (file_offset, _) = section.file_range().unwrap_or((0, 0));
        for (index, raw_symbol) in entries.iter().enumerate() {
            let symbol_index = raw_symbol.get(endian);
            if is_special_indirect_symbol(symbol_index) {
                continue;
            }
            if macho_file
                .symbol_by_index(SymbolIndex(symbol_index as usize))
                .is_err()
            {
                return Err(MachoError::MalformedDyldPayload(format!(
                    "indirect symbol index out of range: {symbol_index}"
                )));
            }
            let Some(resolution) = resolve_indirect_symbol(
                &macho_file,
                SymbolIndex(symbol_index as usize),
                &indirect_symbol_metadata,
                &imports_by_name,
                &imports_by_dylib_and_name,
            )?
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
                binding_kind: match binding_kind {
                    PointerBindingKind::Lazy => ImportBindingKind::Lazy,
                    PointerBindingKind::NonLazy => ImportBindingKind::NonLazy,
                },
                source: ImportBindingSource::IndirectSymbol,
                is_weak: resolution.import.is_weak,
            });
            pointer_slots_by_symbol_index
                .entry(symbol_index)
                .or_default()
                .push(PointerSlotMetadata {
                    pointer_address,
                    section_name: full_name.clone(),
                    binding_kind,
                    ordinal: resolution.ordinal,
                    dylib: resolution.import.dylib.clone(),
                    name: resolution.import.name.clone(),
                });
        }
    }

    for section in macho_file.sections() {
        let raw = section.macho_section();
        if raw.section_type(endian) != S_SYMBOL_STUBS {
            continue;
        }
        let section_name = section.name().unwrap_or_default().to_string();
        let segment_name = section
            .segment_name()
            .ok()
            .flatten()
            .unwrap_or_default()
            .to_string();
        let full_name = format!("{segment_name}:{section_name}");
        let stub_size = u64::from(raw.symbol_stub_size(endian));
        if stub_size == 0 {
            continue;
        }
        if stub_size < 4 || stub_size % 4 != 0 {
            return Err(MachoError::MalformedDyldPayload(format!(
                "invalid symbol stub size {stub_size} for section {full_name}"
            )));
        }
        let entries = raw.indirect_symbols(endian, indirect_symbols)?;
        for (index, raw_symbol) in entries.iter().enumerate() {
            let symbol_index = raw_symbol.get(endian);
            if is_special_indirect_symbol(symbol_index) {
                continue;
            }
            if macho_file
                .symbol_by_index(SymbolIndex(symbol_index as usize))
                .is_err()
            {
                return Err(MachoError::MalformedDyldPayload(format!(
                    "indirect symbol index out of range: {symbol_index}"
                )));
            }
            let Some(resolution) = resolve_indirect_symbol(
                &macho_file,
                SymbolIndex(symbol_index as usize),
                &indirect_symbol_metadata,
                &imports_by_name,
                &imports_by_dylib_and_name,
            )?
            else {
                continue;
            };
            let stub_address = section.address().saturating_add((index as u64) * stub_size);
            let pointer_slot = pointer_slots_by_symbol_index
                .get(&symbol_index)
                .map(|values| select_pointer_slot_for_stub(values, &resolution, symbol_index))
                .transpose()?
                .flatten()
                .cloned();
            let pointer_address = pointer_slot
                .as_ref()
                .map(|slot| slot.pointer_address)
                .filter(|value| *value != 0);
            dyld.stubs.push(StubEntry {
                stub_address,
                section: Some(full_name.clone()),
                pointer_section: pointer_slot.as_ref().map(|slot| slot.section_name.clone()),
                pointer_address,
                helper_address: None,
                binding_ordinal: resolution.ordinal,
                stub_kind: match pointer_slot
                    .as_ref()
                    .map(|slot| slot.binding_kind)
                    .unwrap_or(PointerBindingKind::NonLazy)
                {
                    PointerBindingKind::Lazy => StubKind::Lazy,
                    PointerBindingKind::NonLazy => StubKind::NonLazy,
                },
                dylib: Some(resolution.import.dylib.clone()),
                name: Some(resolution.import.name.clone()),
                source: ImportBindingSource::Stub,
            });
        }
    }

    populate_stub_helpers(&macho_file, &lazy_bindings_by_sequence, dyld)?;

    dyld.import_bindings.sort_by_key(|binding| {
        (
            binding.address.unwrap_or_default(),
            binding.offset.unwrap_or_default(),
            match binding.binding_kind {
                ImportBindingKind::ChainedFixup => 0u8,
                ImportBindingKind::NonLazy => 1u8,
                ImportBindingKind::Lazy => 2u8,
            },
            binding.dylib.clone(),
            binding.name.clone(),
            binding_source_rank(binding.source),
            binding.ordinal.unwrap_or_default(),
            binding.symbol_index.unwrap_or_default(),
            binding.is_weak,
        )
    });
    dyld.import_bindings.dedup_by(|left, right| {
        left.address == right.address
            && left.offset == right.offset
            && left.dylib == right.dylib
            && left.name == right.name
            && left.addend == right.addend
            && left.binding_kind == right.binding_kind
            && left.source == right.source
            && left.ordinal == right.ordinal
            && left.symbol_index == right.symbol_index
            && left.is_weak == right.is_weak
    });

    if dyld.stubs.iter().any(|stub| stub.section.is_some()) {
        dyld.stubs.retain(|stub| stub.section.is_some());
    }

    dyld.stubs.sort_by_key(|stub| {
        (
            stub.stub_address,
            stub.pointer_address.unwrap_or_default(),
            stub.binding_ordinal.unwrap_or_default(),
            stub_kind_rank(&stub.stub_kind),
            binding_source_rank(stub.source),
            stub.dylib.clone().unwrap_or_default(),
            stub.name.clone().unwrap_or_default(),
        )
    });
    dyld.stubs.dedup_by(|left, right| {
        left.stub_address == right.stub_address
            && left.section == right.section
            && left.pointer_section == right.pointer_section
            && left.pointer_address == right.pointer_address
            && left.helper_address == right.helper_address
            && left.binding_ordinal == right.binding_ordinal
            && left.stub_kind == right.stub_kind
            && left.dylib == right.dylib
            && left.name == right.name
            && left.source == right.source
    });
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

fn stub_kind_rank(kind: &StubKind) -> u8 {
    match kind {
        StubKind::Lazy => 0,
        StubKind::NonLazy => 1,
    }
}

fn populate_stub_helpers(
    macho_file: &MachOFile64<'_, object::Endianness>,
    lazy_bindings_by_sequence: &BTreeMap<u32, LazyBindMetadata>,
    dyld: &mut DyldMetadata,
) -> Result<()> {
    let Some(helper_section) = macho_file
        .sections()
        .find(|section| section.name().unwrap_or_default() == "__stub_helper")
    else {
        return Ok(());
    };

    let mut lazy_stubs_by_pointer = dyld
        .stubs
        .iter_mut()
        .filter(|stub| stub.stub_kind == StubKind::Lazy)
        .filter_map(|stub| {
            stub.pointer_address
                .map(|pointer_address| (pointer_address, stub))
        })
        .collect::<BTreeMap<_, _>>();
    if lazy_stubs_by_pointer.is_empty() {
        return Ok(());
    }

    let (file_offset, file_size) = helper_section.file_range().unwrap_or((0, 0));
    let start = usize::try_from(file_offset).map_err(|_| {
        MachoError::MalformedDyldPayload("invalid __stub_helper file offset".to_string())
    })?;
    let size = usize::try_from(file_size).map_err(|_| {
        MachoError::MalformedDyldPayload("invalid __stub_helper file size".to_string())
    })?;
    let end = start.checked_add(size).ok_or_else(|| {
        MachoError::MalformedDyldPayload("invalid __stub_helper file range".to_string())
    })?;
    let helper_bytes = bytes_slice(macho_file, start, end).ok_or_else(|| {
        MachoError::MalformedDyldPayload("truncated __stub_helper section".to_string())
    })?;

    let helper_entries = decode_stub_helper_entries(helper_section.address(), helper_bytes)?;
    if helper_entries.is_empty() {
        return Err(MachoError::MalformedDyldPayload(
            "__stub_helper section did not contain any decodable helper entries".to_string(),
        ));
    }

    for helper_entry in helper_entries {
        let lazy_binding = lazy_bindings_by_sequence
            .get(&helper_entry.lazy_bind_offset)
            .ok_or_else(|| {
                MachoError::MalformedDyldPayload(format!(
                    "missing lazy bind sequence for helper offset {:#x}",
                    helper_entry.lazy_bind_offset
                ))
            })?;
        let stub = lazy_stubs_by_pointer
            .get_mut(&lazy_binding.address)
            .ok_or_else(|| {
                MachoError::MalformedDyldPayload(format!(
                    "missing lazy stub for helper offset {:#x} at pointer {:#x}",
                    helper_entry.lazy_bind_offset, lazy_binding.address
                ))
            })?;
        stub.helper_address = Some(helper_entry.helper_address);
        dyld.stub_helpers.push(StubHelperEntry {
            helper_address: helper_entry.helper_address,
            target_stub: Some(stub.stub_address),
            stub_section: stub.section.clone(),
            pointer_address: stub.pointer_address,
            pointer_section: stub.pointer_section.clone(),
            binding_ordinal: stub.binding_ordinal,
            dylib: Some(lazy_binding.dylib.clone()),
            name: Some(lazy_binding.name.clone()),
        });
    }

    dyld.stub_helpers.sort_by_key(|entry| {
        (
            entry.helper_address,
            entry.target_stub.unwrap_or_default(),
            entry.stub_section.clone().unwrap_or_default(),
            entry.pointer_address.unwrap_or_default(),
            entry.pointer_section.clone().unwrap_or_default(),
            entry.binding_ordinal.unwrap_or_default(),
            entry.dylib.clone().unwrap_or_default(),
            entry.name.clone().unwrap_or_default(),
        )
    });
    dyld.stub_helpers.dedup_by(|left, right| {
        left.helper_address == right.helper_address
            && left.target_stub == right.target_stub
            && left.stub_section == right.stub_section
            && left.pointer_address == right.pointer_address
            && left.pointer_section == right.pointer_section
            && left.binding_ordinal == right.binding_ordinal
            && left.dylib == right.dylib
            && left.name == right.name
    });
    Ok(())
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
    ordinal: Option<u32>,
    import: &'a Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointerBindingKind {
    Lazy,
    NonLazy,
}

#[derive(Debug, Clone)]
struct PointerSlotMetadata {
    pointer_address: u64,
    section_name: String,
    binding_kind: PointerBindingKind,
    ordinal: Option<u32>,
    dylib: String,
    name: String,
}

fn select_pointer_slot_for_stub<'a>(
    slots: &'a [PointerSlotMetadata],
    resolution: &ResolvedIndirectSymbol<'_>,
    symbol_index: u32,
) -> Result<Option<&'a PointerSlotMetadata>> {
    if slots.is_empty() {
        return Ok(None);
    }

    if let Some(ordinal) = resolution.ordinal {
        let ordinal_matches = slots
            .iter()
            .filter(|slot| slot.ordinal == Some(ordinal))
            .collect::<Vec<_>>();
        if ordinal_matches.len() == 1 {
            return Ok(ordinal_matches.first().copied());
        }
        if ordinal_matches.len() > 1 {
            return Err(MachoError::MalformedDyldPayload(format!(
                "ambiguous pointer slots for symbol index {symbol_index}: multiple ordinal matches for ordinal {ordinal}"
            )));
        }
    }

    let dylib_name_matches = slots
        .iter()
        .filter(|slot| {
            slot.dylib == resolution.import.dylib
                && normalize_import_name(&slot.name)
                    == normalize_import_name(&resolution.import.name)
        })
        .collect::<Vec<_>>();
    if dylib_name_matches.len() == 1 {
        return Ok(dylib_name_matches.first().copied());
    }
    if dylib_name_matches.len() > 1 {
        return Err(MachoError::MalformedDyldPayload(format!(
            "ambiguous pointer slots for symbol index {symbol_index}: multiple dylib/name matches for {}:{}",
            resolution.import.dylib, resolution.import.name
        )));
    }

    let lazy_matches = slots
        .iter()
        .filter(|slot| slot.binding_kind == PointerBindingKind::Lazy)
        .collect::<Vec<_>>();
    if lazy_matches.len() == 1 {
        return Ok(lazy_matches.first().copied());
    }
    if lazy_matches.len() > 1 {
        return Err(MachoError::MalformedDyldPayload(format!(
            "ambiguous pointer slots for symbol index {symbol_index}: multiple lazy candidates"
        )));
    }

    if slots.len() == 1 {
        return Ok(slots.first());
    }

    Err(MachoError::MalformedDyldPayload(format!(
        "ambiguous pointer slots for symbol index {symbol_index}: {} candidates",
        slots.len()
    )))
}

#[derive(Debug, Clone)]
struct LazyBindMetadata {
    address: u64,
    name: String,
    dylib: String,
}

#[derive(Debug, Clone, Copy)]
struct StubHelperDecodeEntry {
    helper_address: u64,
    lazy_bind_offset: u32,
}

fn bytes_slice<'data>(
    macho_file: &MachOFile64<'data, object::Endianness>,
    start: usize,
    end: usize,
) -> Option<&'data [u8]> {
    let bytes = macho_file.data();
    bytes.get(start..end)
}

fn decode_stub_helper_entries(
    section_address: u64,
    helper_bytes: &[u8],
) -> Result<Vec<StubHelperDecodeEntry>> {
    let Some(entry_start) = find_stub_helper_entry_start(section_address, helper_bytes) else {
        return Err(MachoError::MalformedDyldPayload(format!(
            "__stub_helper section at {section_address:#x} (size={:#x}) did not contain a decodable helper trampoline",
            helper_bytes.len()
        )));
    };
    validate_stub_helper_trampoline(section_address, helper_bytes, entry_start)?;

    let mut entries = Vec::new();
    let mut cursor = entry_start;
    let remaining_table = helper_bytes.len().saturating_sub(entry_start);
    if remaining_table % 12 != 0 {
        return Err(MachoError::MalformedDyldPayload(
            "__stub_helper table is not a multiple of 12-byte entries".to_string(),
        ));
    }
    let section_end = section_address.saturating_add(helper_bytes.len() as u64);
    while cursor < helper_bytes.len() {
        let remaining = helper_bytes.len().saturating_sub(cursor);
        if remaining < 12 {
            return Err(MachoError::MalformedDyldPayload(
                "truncated __stub_helper entry".to_string(),
            ));
        }
        let ldr = read_u32_le(&helper_bytes[cursor..cursor + 4]);
        let branch = read_u32_le(&helper_bytes[cursor + 4..cursor + 8]);
        let helper_address = section_address.saturating_add(cursor as u64);
        if !is_stub_helper_literal_load(ldr) {
            return Err(MachoError::MalformedDyldPayload(format!(
                "unexpected __stub_helper literal-load instruction {ldr:#010x} at {helper_address:#x}"
            )));
        }
        let branch_target =
            decode_unconditional_branch_target(helper_address.saturating_add(4), branch)
                .ok_or_else(|| {
                    MachoError::MalformedDyldPayload(format!(
                        "unexpected __stub_helper branch instruction {branch:#010x} at {:#x}",
                        helper_address.saturating_add(4),
                    ))
                })?;
        if branch_target != section_address {
            return Err(MachoError::MalformedDyldPayload(format!(
                "__stub_helper branch target {branch_target:#x} did not point at trampoline base {section_address:#x}"
            )));
        }
        let literal_target = decode_literal_load_target(helper_address, ldr).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "failed to decode __stub_helper literal-load target at {helper_address:#x}"
            ))
        })?;
        if !(section_address..section_end).contains(&literal_target) {
            return Err(MachoError::MalformedDyldPayload(format!(
                "__stub_helper literal target {literal_target:#x} is out of section range"
            )));
        }
        let literal_offset = literal_target.saturating_sub(section_address) as usize;
        let Some(literal_bytes) =
            helper_bytes.get(literal_offset..literal_offset.saturating_add(4))
        else {
            return Err(MachoError::MalformedDyldPayload(format!(
                "__stub_helper literal target {literal_target:#x} is truncated"
            )));
        };
        let literal = read_u32_le(literal_bytes);
        entries.push(StubHelperDecodeEntry {
            helper_address,
            lazy_bind_offset: literal,
        });
        cursor = cursor.saturating_add(12);
    }
    Ok(entries)
}

fn validate_stub_helper_trampoline(
    section_address: u64,
    helper_bytes: &[u8],
    entry_start: usize,
) -> Result<()> {
    if entry_start == 0 {
        return Ok(());
    }
    if entry_start != 24 || helper_bytes.len() < entry_start {
        return Err(MachoError::MalformedDyldPayload(format!(
            "unsupported __stub_helper trampoline size before helper entries: {entry_start}"
        )));
    }

    let words = helper_bytes[..entry_start]
        .chunks_exact(4)
        .map(read_u32_le)
        .collect::<Vec<_>>();
    let checks = [
        is_stub_helper_trampoline_adrp_x17(words[0]),
        is_stub_helper_trampoline_add_x17(words[1]),
        words[2] == 0xa9bf47f0,
        is_stub_helper_trampoline_adrp_x16(words[3]),
        is_stub_helper_trampoline_ldr_x16(words[4]),
        words[5] == 0xd61f0200,
    ];
    if let Some(index) = checks.iter().position(|value| !value) {
        let address = section_address.saturating_add((index * 4) as u64);
        return Err(MachoError::MalformedDyldPayload(format!(
            "unexpected __stub_helper literal-load instruction or trampoline opcode {:#010x} at {address:#x}",
            words[index]
        )));
    }
    Ok(())
}

fn find_stub_helper_entry_start(section_address: u64, helper_bytes: &[u8]) -> Option<usize> {
    let section_end = section_address.checked_add(helper_bytes.len() as u64)?;
    for offset in (0..helper_bytes.len().saturating_sub(11)).step_by(4) {
        let ldr = read_u32_le(&helper_bytes[offset..offset + 4]);
        let branch = read_u32_le(&helper_bytes[offset + 4..offset + 8]);
        let helper_address = section_address.saturating_add(offset as u64);
        let literal_target = decode_literal_load_target(helper_address, ldr);
        if is_stub_helper_literal_load(ldr)
            && decode_unconditional_branch_target(helper_address.saturating_add(4), branch)
                == Some(section_address)
            && literal_target.is_some_and(|target| (section_address..section_end).contains(&target))
        {
            return Some(offset);
        }
    }
    None
}

fn is_stub_helper_literal_load(word: u32) -> bool {
    word & 0xff00_001f == 0x1800_0010
}

fn is_stub_helper_trampoline_adrp_x17(word: u32) -> bool {
    word & 0x9f00_001f == 0x9000_0011
}

fn is_stub_helper_trampoline_add_x17(word: u32) -> bool {
    word & 0xffc0_03ff == 0x9100_0231
}

fn is_stub_helper_trampoline_adrp_x16(word: u32) -> bool {
    word & 0x9f00_001f == 0x9000_0010
}

fn is_stub_helper_trampoline_ldr_x16(word: u32) -> bool {
    word & 0xffc0_03ff == 0xf940_0210
}

fn decode_unconditional_branch_target(instruction_address: u64, word: u32) -> Option<u64> {
    if word & 0x7c00_0000 != 0x1400_0000 {
        return None;
    }
    let imm26 = (word & 0x03ff_ffff) as i32;
    let signed = (imm26 << 6) >> 6;
    let delta = i64::from(signed) * 4;
    if delta >= 0 {
        instruction_address.checked_add(delta as u64)
    } else {
        instruction_address.checked_sub(delta.unsigned_abs())
    }
}

fn decode_literal_load_target(instruction_address: u64, word: u32) -> Option<u64> {
    if !is_stub_helper_literal_load(word) {
        return None;
    }
    let imm19 = ((word >> 5) & 0x7ffff) as i32;
    let signed = (imm19 << 13) >> 13;
    let delta = i64::from(signed) * 4;
    if delta >= 0 {
        instruction_address.checked_add(delta as u64)
    } else {
        instruction_address.checked_sub(delta.unsigned_abs())
    }
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn build_indirect_symbol_metadata(
    macho: &goblin::mach::MachO<'_>,
) -> BTreeMap<usize, IndirectSymbolMetadata> {
    let mut result = BTreeMap::new();
    let Some(symbols) = macho.symbols.as_ref() else {
        return result;
    };
    let Some(dysymtab) = macho
        .load_commands
        .iter()
        .find_map(|command| match &command.command {
            goblin::mach::load_command::CommandVariant::Dysymtab(command) => Some(command),
            _ => None,
        })
    else {
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
    imports_by_name: &BTreeMap<String, Vec<&'a Import>>,
    imports_by_dylib_and_name: &BTreeMap<(String, String), &'a Import>,
) -> Result<Option<ResolvedIndirectSymbol<'a>>> {
    let metadata_entry = metadata.get(&symbol_index.0);
    let (normalized_name, ordinal, dylib_name) = if let Some(entry) = metadata_entry {
        (
            entry.normalized_name.clone(),
            entry.ordinal,
            entry.dylib.clone(),
        )
    } else {
        let symbol = macho_file.symbol_by_index(symbol_index).map_err(|error| {
            MachoError::MalformedDyldPayload(format!(
                "failed to resolve indirect symbol index {}: {error}",
                symbol_index.0
            ))
        })?;
        let raw_name = symbol.name().map_err(|error| {
            MachoError::MalformedDyldPayload(format!(
                "failed to read indirect symbol {} name: {error}",
                symbol_index.0
            ))
        })?;
        let raw_name = raw_name.to_string();
        (normalize_import_name(&raw_name), None, None)
    };

    let import = if let Some(dylib) = dylib_name.as_ref() {
        if let Some(import) = imports_by_dylib_and_name
            .get(&(normalized_name.clone(), dylib.clone()))
            .copied()
        {
            Some(import)
        } else {
            let name_matches = imports_by_name.get(&normalized_name);
            match name_matches.map_or(0, Vec::len) {
                0 => {
                    return Err(MachoError::MalformedDyldPayload(format!(
                        "indirect symbol {} ({normalized_name}) did not resolve to import {}",
                        symbol_index.0, dylib
                    )));
                }
                1 => {
                    return Err(MachoError::MalformedDyldPayload(format!(
                        "indirect symbol {} ({normalized_name}) resolved to mismatched dylib for ordinal {:?}",
                        symbol_index.0, ordinal
                    )));
                }
                count => {
                    return Err(MachoError::MalformedDyldPayload(format!(
                        "indirect symbol {} ({normalized_name}) is ambiguous across {count} imports for ordinal {:?}",
                        symbol_index.0, ordinal
                    )));
                }
            }
        }
    } else {
        match imports_by_name.get(&normalized_name) {
            None => None,
            Some(matches) if matches.len() == 1 => Some(matches[0]),
            Some(matches) => {
                return Err(MachoError::MalformedDyldPayload(format!(
                    "indirect symbol {} ({normalized_name}) is ambiguous across {} imports",
                    symbol_index.0,
                    matches.len()
                )));
            }
        }
    };
    Ok(import.map(|import| ResolvedIndirectSymbol { ordinal, import }))
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
