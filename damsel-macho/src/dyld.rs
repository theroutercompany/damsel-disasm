use crate::errors::{MachoError, Result};
use damsel_core::{
    DyldMetadata, ExportFlags, ExportKind, ExportRecord, ImportBindingKind, ImportBindingRecord,
    ImportBindingSource, Segment, StubEntry, StubHelperEntry, StubKind,
};
use goblin::mach::exports::{
    EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE, EXPORT_SYMBOL_FLAGS_KIND_MASK,
    EXPORT_SYMBOL_FLAGS_KIND_THREAD_LOCAL, EXPORT_SYMBOL_FLAGS_REEXPORT,
    EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER, EXPORT_SYMBOL_FLAGS_WEAK_DEFINITION, ExportInfo,
};
use goblin::mach::{load_command, segment};

const DYLD_CHAINED_PTR_START_NONE: u16 = 0xFFFF;
const DYLD_CHAINED_PTR_START_MULTI: u16 = 0x8000;
const DYLD_CHAINED_PTR_START_LAST: u16 = 0x8000;

const DYLD_CHAINED_IMPORT: u32 = 1;
const DYLD_CHAINED_IMPORT_ADDEND: u32 = 2;
const DYLD_CHAINED_IMPORT_ADDEND64: u32 = 3;

const DYLD_CHAINED_PTR_ARM64E: u16 = 1;
const DYLD_CHAINED_PTR_64: u16 = 2;
const DYLD_CHAINED_PTR_32: u16 = 3;
const DYLD_CHAINED_PTR_64_OFFSET: u16 = 6;
const DYLD_CHAINED_PTR_ARM64E_USERLAND: u16 = 9;
const DYLD_CHAINED_PTR_ARM64E_USERLAND24: u16 = 12;
const DYLD_CHAINED_PTR_ARM64E_SEGMENTED: u16 = 14;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportAddressHint {
    pub name: String,
    pub dylib: String,
    pub address: u64,
    pub offset: Option<u64>,
    pub addend: i64,
    pub is_weak: bool,
}

#[derive(Debug, Default)]
pub(crate) struct DyldAnalysis {
    pub metadata: DyldMetadata,
    pub import_hints: Vec<ImportAddressHint>,
}

#[derive(Debug, Clone)]
struct ChainedImportEntry {
    name: String,
    dylib: String,
    addend: i64,
    is_weak: bool,
}

#[derive(Debug, Default)]
struct ChainedFixupSummary {
    has_binds: bool,
    has_rebases: bool,
    import_hints: Vec<ImportAddressHint>,
    import_bindings: Vec<ImportBindingRecord>,
}

pub(crate) fn collect_dyld_metadata(
    macho: &goblin::mach::MachO<'_>,
    bytes: &[u8],
    segments: &[Segment],
) -> Result<DyldAnalysis> {
    let image_base = segments
        .iter()
        .filter(|segment| segment.file_size > 0 && segment.address > 0)
        .map(|segment| segment.address)
        .min()
        .unwrap_or_default();
    let text_base = segments
        .iter()
        .find(|segment| segment.name == "__TEXT")
        .map(|segment| segment.address)
        .unwrap_or_default();
    let function_starts = parse_function_starts(bytes, text_base, &macho.load_commands);
    validate_export_payload_ranges(bytes, &macho.load_commands)?;
    let exported_symbols = macho
        .exports()
        .map_err(|error| {
            MachoError::MalformedDyldPayload(format!("failed to parse export trie: {error}"))
        })?
        .into_iter()
        .map(|export| build_export_record(export.name, export.offset, &export.info, image_base))
        .collect::<Vec<_>>();

    let mut has_rebases = false;
    let mut has_binds = false;
    let mut has_chained_fixups = false;
    let mut import_hints = Vec::new();
    let mut import_bindings = Vec::new();
    let mut chained_fixups = None;

    for command in &macho.load_commands {
        match &command.command {
            load_command::CommandVariant::DyldInfo(info)
            | load_command::CommandVariant::DyldInfoOnly(info) => {
                has_rebases |= info.rebase_size > 0;
                has_binds |=
                    info.bind_size > 0 || info.lazy_bind_size > 0 || info.weak_bind_size > 0;
            }
            load_command::CommandVariant::DyldChainedFixups(linkedit) => {
                has_chained_fixups = true;
                chained_fixups = Some(*linkedit);
            }
            _ => {}
        }
    }

    if let Some(linkedit) = chained_fixups {
        let summary =
            parse_chained_fixups(bytes, &linkedit, macho.libs.as_slice(), &macho.segments)?;
        has_binds |= summary.has_binds;
        has_rebases |= summary.has_rebases;
        import_hints = summary.import_hints;
        import_bindings = summary.import_bindings;
    }

    import_hints.sort_by_key(|hint| {
        (
            hint.address,
            hint.offset.unwrap_or_default(),
            hint.dylib.clone(),
            hint.name.clone(),
        )
    });
    import_hints.dedup_by(|left, right| {
        left.address == right.address
            && left.offset == right.offset
            && left.dylib == right.dylib
            && left.name == right.name
    });
    import_bindings.sort_by_key(|binding| {
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
            match binding.source {
                ImportBindingSource::ChainedFixup => 0u8,
                ImportBindingSource::IndirectSymbol => 1u8,
                ImportBindingSource::Stub => 2u8,
                ImportBindingSource::Other => 3u8,
            },
        )
    });
    import_bindings.dedup();
    let stubs = materialize_stub_entries(&import_bindings, &exported_symbols);

    Ok(DyldAnalysis {
        metadata: DyldMetadata {
            imported_dylibs: macho.libs.iter().skip(1).map(ToString::to_string).collect(),
            rpaths: macho.rpaths.iter().map(ToString::to_string).collect(),
            exported_symbols,
            function_starts,
            has_rebases,
            has_binds,
            has_chained_fixups,
            import_bindings,
            stubs,
            stub_helpers: Vec::<StubHelperEntry>::new(),
        },
        import_hints,
    })
}

fn validate_export_payload_ranges(
    bytes: &[u8],
    commands: &[goblin::mach::load_command::LoadCommand],
) -> Result<()> {
    for command in commands {
        match &command.command {
            load_command::CommandVariant::DyldInfo(info)
            | load_command::CommandVariant::DyldInfoOnly(info) => {
                if info.export_size > 0 {
                    linkedit_range(bytes, info.export_off as u64, info.export_size as u64)
                        .map_err(|error| {
                            MachoError::MalformedDyldPayload(format!(
                                "invalid export trie range from LC_DYLD_INFO*: {error}"
                            ))
                        })?;
                }
            }
            load_command::CommandVariant::DyldExportsTrie(linkedit) => {
                if linkedit.datasize > 0 {
                    linkedit_range(bytes, linkedit.dataoff as u64, linkedit.datasize as u64)
                        .map_err(|error| {
                            MachoError::MalformedDyldPayload(format!(
                                "invalid dyld export trie range: {error}"
                            ))
                        })?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn build_export_record(
    name: String,
    offset: u64,
    info: &ExportInfo<'_>,
    image_base: u64,
) -> ExportRecord {
    let raw_bits = export_raw_bits(info);
    let raw_flags = format!("{info:?};offset={offset:#x}");
    let kind = map_export_kind(info, image_base);
    ExportRecord {
        name,
        address: export_record_address(offset, info, image_base, &kind),
        raw_flags: raw_flags.clone(),
        flags: ExportFlags::from_bits(raw_bits),
        reexport_target: export_reexport_target(&kind),
        resolver_target: export_resolver_target(&kind),
        kind,
    }
}

fn export_raw_bits(info: &ExportInfo<'_>) -> u64 {
    match info {
        ExportInfo::Regular { flags, .. }
        | ExportInfo::Reexport { flags, .. }
        | ExportInfo::Stub { flags, .. } => u64::from(*flags),
    }
}

fn export_record_address(
    offset: u64,
    info: &ExportInfo<'_>,
    image_base: u64,
    kind: &ExportKind,
) -> Option<u64> {
    match info {
        ExportInfo::Regular { flags, .. } => {
            if (*flags & EXPORT_SYMBOL_FLAGS_KIND_MASK) == EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE {
                Some(offset)
            } else {
                image_base.checked_add(offset)
            }
        }
        ExportInfo::Reexport { .. } => None,
        ExportInfo::Stub { .. } => match kind {
            ExportKind::StubAndResolver { stub_address, .. } => *stub_address,
            _ => None,
        },
    }
}

fn export_reexport_target(kind: &ExportKind) -> Option<(String, Option<String>)> {
    match kind {
        ExportKind::Reexport { dylib, symbol } => Some((dylib.clone(), symbol.clone())),
        _ => None,
    }
}

fn export_resolver_target(kind: &ExportKind) -> Option<u64> {
    match kind {
        ExportKind::Resolver { resolver_address } => *resolver_address,
        ExportKind::StubAndResolver {
            resolver_address, ..
        } => *resolver_address,
        _ => None,
    }
}

fn map_export_kind(info: &ExportInfo<'_>, image_base: u64) -> ExportKind {
    let raw_bits = export_raw_bits(info);
    let kind_bits = raw_bits & u64::from(EXPORT_SYMBOL_FLAGS_KIND_MASK);
    match info {
        ExportInfo::Regular { flags, .. } => {
            if flags & EXPORT_SYMBOL_FLAGS_WEAK_DEFINITION != 0 {
                ExportKind::WeakDefinition
            } else if (flags & EXPORT_SYMBOL_FLAGS_KIND_MASK) == EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE {
                ExportKind::Absolute
            } else if (flags & EXPORT_SYMBOL_FLAGS_KIND_MASK)
                == EXPORT_SYMBOL_FLAGS_KIND_THREAD_LOCAL
            {
                ExportKind::ThreadLocal
            } else if kind_bits != 0 {
                ExportKind::Unknown(format!("flags={raw_bits:#x}"))
            } else {
                ExportKind::Regular
            }
        }
        ExportInfo::Reexport {
            lib,
            lib_symbol_name,
            ..
        } => {
            if raw_bits & u64::from(EXPORT_SYMBOL_FLAGS_REEXPORT) == 0 {
                ExportKind::Unknown(format!("flags={raw_bits:#x}"))
            } else {
                ExportKind::Reexport {
                    dylib: (*lib).to_string(),
                    symbol: lib_symbol_name.map(ToString::to_string),
                }
            }
        }
        ExportInfo::Stub {
            stub_offset,
            resolver_offset,
            ..
        } => {
            if raw_bits & u64::from(EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER) == 0 {
                ExportKind::Unknown(format!("flags={raw_bits:#x}"))
            } else {
                ExportKind::StubAndResolver {
                    stub_address: image_base.checked_add((*stub_offset).into()),
                    resolver_address: image_base.checked_add((*resolver_offset).into()),
                }
            }
        }
    }
}

pub(crate) fn detect_platform(
    commands: &[goblin::mach::load_command::LoadCommand],
) -> Option<String> {
    for command in commands {
        match &command.command {
            load_command::CommandVariant::BuildVersion(build) => {
                return Some(platform_name(build.platform).to_string());
            }
            load_command::CommandVariant::VersionMinMacosx(_) => return Some("macos".to_string()),
            load_command::CommandVariant::VersionMinIphoneos(_) => return Some("ios".to_string()),
            load_command::CommandVariant::VersionMinTvos(_) => return Some("tvos".to_string()),
            load_command::CommandVariant::VersionMinWatchos(_) => {
                return Some("watchos".to_string());
            }
            _ => {}
        }
    }

    None
}

fn platform_name(platform: u32) -> &'static str {
    match platform {
        load_command::PLATFORM_MACOS => "macos",
        load_command::PLATFORM_IOS => "ios",
        load_command::PLATFORM_TVOS => "tvos",
        load_command::PLATFORM_WATCHOS => "watchos",
        load_command::PLATFORM_MACCATALYST => "maccatalyst",
        load_command::PLATFORM_DRIVERKIT => "driverkit",
        load_command::PLATFORM_VISIONOS => "visionos",
        load_command::PLATFORM_VISIONOSSIMULATOR => "visionos-simulator",
        _ => "unknown",
    }
}

fn parse_function_starts(
    bytes: &[u8],
    text_base: u64,
    commands: &[goblin::mach::load_command::LoadCommand],
) -> Vec<u64> {
    let mut starts = Vec::new();

    for command in commands {
        if let load_command::CommandVariant::FunctionStarts(linkedit) = &command.command {
            let start = linkedit.dataoff as usize;
            let end = start.saturating_add(linkedit.datasize as usize);
            let Some(data) = bytes.get(start..end) else {
                continue;
            };

            let mut cursor = 0usize;
            let mut address = text_base;
            while cursor < data.len() {
                let Some((delta, read)) = read_uleb128(&data[cursor..]) else {
                    break;
                };
                cursor += read;
                if delta == 0 {
                    break;
                }
                address = address.saturating_add(delta);
                starts.push(address);
            }
        }
    }

    starts.sort_unstable();
    starts.dedup();
    starts
}

fn read_uleb128(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    for (index, byte) in bytes.iter().copied().enumerate() {
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, index + 1));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
    None
}

fn parse_chained_fixups(
    bytes: &[u8],
    linkedit: &load_command::LinkeditDataCommand,
    libs: &[&str],
    segments: &segment::Segments<'_>,
) -> Result<ChainedFixupSummary> {
    let chain_data = linkedit_range(bytes, linkedit.dataoff as u64, linkedit.datasize as u64)?;
    if chain_data.len() < 28 {
        return Err(MachoError::MalformedDyldPayload(
            "dyld chained fixups header is truncated".to_string(),
        ));
    }

    let starts_offset = read_u32(chain_data, 4)
        .ok_or_else(|| MachoError::MalformedDyldPayload("missing starts offset".to_string()))?
        as usize;
    let imports_offset = read_u32(chain_data, 8)
        .ok_or_else(|| MachoError::MalformedDyldPayload("missing imports offset".to_string()))?
        as usize;
    let symbols_offset = read_u32(chain_data, 12)
        .ok_or_else(|| MachoError::MalformedDyldPayload("missing symbols offset".to_string()))?
        as usize;
    let imports_count = read_u32(chain_data, 16)
        .ok_or_else(|| MachoError::MalformedDyldPayload("missing imports count".to_string()))?
        as usize;
    let imports_format = read_u32(chain_data, 20)
        .ok_or_else(|| MachoError::MalformedDyldPayload("missing imports format".to_string()))?;
    let symbols_format = read_u32(chain_data, 24)
        .ok_or_else(|| MachoError::MalformedDyldPayload("missing symbols format".to_string()))?;

    let imports = parse_chained_imports(
        chain_data,
        imports_offset,
        symbols_offset,
        imports_count,
        imports_format,
        symbols_format,
        libs,
    )?;

    let mut summary = ChainedFixupSummary {
        has_binds: !imports.is_empty(),
        ..ChainedFixupSummary::default()
    };

    let starts = chain_data.get(starts_offset..).ok_or_else(|| {
        MachoError::MalformedDyldPayload("chained starts offset out of range".to_string())
    })?;
    let seg_count = read_u32(starts, 0).ok_or_else(|| {
        MachoError::MalformedDyldPayload("missing chained segment count".to_string())
    })? as usize;

    for seg_index in 0..seg_count {
        let seg_info_offset = read_u32(starts, 4 + seg_index * 4).ok_or_else(|| {
            MachoError::MalformedDyldPayload("truncated chained segment offsets".to_string())
        })? as usize;
        if seg_info_offset == 0 {
            continue;
        }
        let segment = segments.get(seg_index).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "chained segment index {seg_index} out of range"
            ))
        })?;
        let seg_info = starts.get(seg_info_offset..).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "chained segment {seg_index} info offset {seg_info_offset:#x} out of range"
            ))
        })?;

        let seg_info_size = read_u32(seg_info, 0).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "missing chained segment {seg_index} info size"
            ))
        })? as usize;
        if seg_info_size < 22 {
            return Err(MachoError::MalformedDyldPayload(format!(
                "chained segment {seg_index} info too small: {seg_info_size}"
            )));
        }
        if seg_info_size > seg_info.len() {
            return Err(MachoError::MalformedDyldPayload(format!(
                "chained segment {seg_index} info truncated: size={seg_info_size} available={}",
                seg_info.len()
            )));
        }
        let seg_info = &seg_info[..seg_info_size];
        let page_size = read_u16(seg_info, 4).map(u64::from).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "missing chained segment {seg_index} page size"
            ))
        })?;
        let pointer_format = read_u16(seg_info, 6).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "missing chained segment {seg_index} pointer format"
            ))
        })?;
        let page_count = read_u16(seg_info, 20)
            .map(|value| value as usize)
            .ok_or_else(|| {
                MachoError::MalformedDyldPayload(format!(
                    "missing chained segment {seg_index} page count"
                ))
            })?;
        if page_size == 0 {
            return Err(MachoError::MalformedDyldPayload(format!(
                "chained segment {seg_index} declared zero page size"
            )));
        }

        let page_starts_offset = 22usize;
        let page_starts_len = page_count.saturating_mul(2);
        let page_starts_end = page_starts_offset.saturating_add(page_starts_len);
        if page_starts_end > seg_info.len() {
            return Err(MachoError::MalformedDyldPayload(format!(
                "chained segment {seg_index} page-start table is truncated"
            )));
        }

        for page_index in 0..page_count {
            let page_start =
                read_u16(seg_info, page_starts_offset + page_index * 2).ok_or_else(|| {
                    MachoError::MalformedDyldPayload(format!(
                        "missing chained segment {seg_index} page-start entry {page_index}"
                    ))
                })?;
            if page_start == DYLD_CHAINED_PTR_START_NONE {
                continue;
            }

            let mut starts_for_page = Vec::new();
            if (page_start & DYLD_CHAINED_PTR_START_MULTI) != 0 {
                let mut chain_index = usize::from(page_start & !DYLD_CHAINED_PTR_START_MULTI);
                loop {
                    let raw_start = read_u16(seg_info, page_starts_end + chain_index * 2).ok_or_else(
                        || {
                            MachoError::MalformedDyldPayload(format!(
                                "truncated chained segment {seg_index} multi-start list for page {page_index}"
                            ))
                        },
                    )?;
                    starts_for_page.push(raw_start & !DYLD_CHAINED_PTR_START_LAST);
                    chain_index = chain_index.saturating_add(1);
                    if (raw_start & DYLD_CHAINED_PTR_START_LAST) != 0 {
                        break;
                    }
                }
            } else {
                starts_for_page.push(page_start);
            }

            for start in starts_for_page {
                walk_fixup_chain(
                    segment,
                    segment.name().ok(),
                    pointer_format,
                    page_size,
                    page_index as u64,
                    u64::from(start),
                    &imports,
                    &mut summary,
                )?;
            }
        }
    }

    Ok(summary)
}

fn parse_chained_imports(
    chain_data: &[u8],
    imports_offset: usize,
    symbols_offset: usize,
    imports_count: usize,
    imports_format: u32,
    symbols_format: u32,
    libs: &[&str],
) -> Result<Vec<ChainedImportEntry>> {
    let entry_size = match imports_format {
        DYLD_CHAINED_IMPORT => 4usize,
        DYLD_CHAINED_IMPORT_ADDEND => 8usize,
        DYLD_CHAINED_IMPORT_ADDEND64 => 16usize,
        other => {
            return Err(MachoError::MalformedDyldPayload(format!(
                "unsupported chained imports format {other}"
            )));
        }
    };

    if symbols_format != 0 {
        return Err(MachoError::MalformedDyldPayload(format!(
            "unsupported chained symbols format {symbols_format}"
        )));
    }
    let symbols = chain_data.get(symbols_offset..).ok_or_else(|| {
        MachoError::MalformedDyldPayload(format!(
            "chained symbols offset out of range: {symbols_offset:#x}"
        ))
    })?;
    let mut imports = Vec::with_capacity(imports_count);

    for index in 0..imports_count {
        let entry_offset = imports_offset
            .checked_add(index.saturating_mul(entry_size))
            .ok_or_else(|| {
                MachoError::MalformedDyldPayload(format!(
                    "chained imports entry offset overflow at index {index}"
                ))
            })?;
        let entry = chain_data
            .get(entry_offset..entry_offset + entry_size)
            .ok_or_else(|| {
                MachoError::MalformedDyldPayload(format!(
                    "truncated chained imports entry at index {index}"
                ))
            })?;

        let (lib_ordinal, is_weak, name_offset, addend) = match imports_format {
            DYLD_CHAINED_IMPORT => {
                let raw = u64::from(read_u32(entry, 0).unwrap_or(0));
                (
                    ((raw & 0xff) as u8 as i8) as i32,
                    ((raw >> 8) & 0x1) != 0,
                    ((raw >> 9) & 0x7f_ffff) as usize,
                    0,
                )
            }
            DYLD_CHAINED_IMPORT_ADDEND => {
                let raw = u64::from(read_u32(entry, 0).unwrap_or(0));
                let addend = read_i32(entry, 4).unwrap_or(0) as i64;
                (
                    ((raw & 0xff) as u8 as i8) as i32,
                    ((raw >> 8) & 0x1) != 0,
                    ((raw >> 9) & 0x7f_ffff) as usize,
                    addend,
                )
            }
            DYLD_CHAINED_IMPORT_ADDEND64 => {
                let raw = read_u64(entry, 0).unwrap_or(0);
                let addend = read_u64(entry, 8).unwrap_or(0) as i64;
                (
                    ((raw & 0xffff) as u16 as i16) as i32,
                    ((raw >> 16) & 0x1) != 0,
                    (raw >> 32) as usize,
                    addend,
                )
            }
            _ => unreachable!(),
        };

        let name = read_c_string(symbols, name_offset).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "invalid chained import name offset {name_offset:#x} at index {index}"
            ))
        })?;
        let dylib = resolve_chained_dylib_name(libs, lib_ordinal);
        imports.push(ChainedImportEntry {
            name,
            dylib,
            addend,
            is_weak,
        });
    }

    Ok(imports)
}

fn walk_fixup_chain(
    segment: &segment::Segment<'_>,
    segment_name: Option<&str>,
    pointer_format: u16,
    page_size: u64,
    page_index: u64,
    chain_start: u64,
    imports: &[ChainedImportEntry],
    summary: &mut ChainedFixupSummary,
) -> Result<()> {
    let segment_name = segment_name.unwrap_or("<unknown>");
    let stride = pointer_stride(pointer_format).ok_or_else(|| {
        MachoError::MalformedDyldPayload(format!(
            "unsupported chained pointer format {pointer_format} in segment {segment_name}"
        ))
    })?;
    let pointer_width = pointer_width(pointer_format).ok_or_else(|| {
        MachoError::MalformedDyldPayload(format!(
            "unsupported chained pointer width format {pointer_format} in segment {segment_name}"
        ))
    })?;
    let mut offset_in_segment = page_index
        .saturating_mul(page_size)
        .saturating_add(chain_start);
    let mut steps = 0usize;

    while steps < 65_536 {
        let Some(entry_end) = offset_in_segment.checked_add(pointer_width as u64) else {
            break;
        };
        if entry_end > segment.filesize {
            return Err(MachoError::MalformedDyldPayload(format!(
                "chained pointer ran past segment bounds in {segment_name}: page={page_index} offset={offset_in_segment:#x}"
            )));
        }
        let segment_offset = usize::try_from(offset_in_segment).map_err(|_| {
            MachoError::MalformedDyldPayload(format!(
                "invalid chained pointer offset in {segment_name}: {offset_in_segment:#x}"
            ))
        })?;
        let raw_entry = segment
            .data
            .get(segment_offset..segment_offset + pointer_width)
            .ok_or_else(|| {
                MachoError::MalformedDyldPayload(format!(
                    "truncated chained pointer entry in {segment_name}: page={page_index} offset={offset_in_segment:#x}"
                ))
            })?;
        let pointer = decode_chained_pointer(pointer_format, raw_entry).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "failed to decode chained pointer in {segment_name}: format={pointer_format} page={page_index} offset={offset_in_segment:#x}"
            ))
        })?;

        let address = segment.vmaddr.saturating_add(offset_in_segment);
        let file_offset = segment.fileoff.saturating_add(offset_in_segment);

        if pointer.is_bind {
            summary.has_binds = true;
            let mut binding_name = None::<String>;
            let mut binding_dylib = None::<String>;
            let mut binding_addend = 0i64;
            let mut binding_is_weak = false;
            if let Some(ordinal) = pointer.bind_ordinal {
                let import = imports.get(ordinal as usize).ok_or_else(|| {
                    MachoError::MalformedDyldPayload(format!(
                        "chained bind ordinal {ordinal} out of range in {segment_name}: page={page_index} offset={offset_in_segment:#x}"
                    ))
                })?;
                binding_name = Some(import.name.clone());
                binding_dylib = Some(import.dylib.clone());
                binding_addend = import.addend;
                binding_is_weak = import.is_weak;
                summary.import_hints.push(ImportAddressHint {
                    name: import.name.clone(),
                    dylib: import.dylib.clone(),
                    address,
                    offset: Some(file_offset),
                    addend: import.addend,
                    is_weak: import.is_weak,
                });
            }
            summary.import_bindings.push(ImportBindingRecord {
                dylib: binding_dylib.unwrap_or_else(|| "<unknown-dylib>".to_string()),
                name: binding_name.unwrap_or_else(|| match pointer.bind_ordinal {
                    Some(ordinal) => format!("<ordinal:{ordinal}>"),
                    None => "<unknown-import>".to_string(),
                }),
                address: Some(address),
                offset: Some(file_offset),
                addend: binding_addend,
                ordinal: pointer.bind_ordinal,
                symbol_index: None,
                binding_kind: ImportBindingKind::ChainedFixup,
                source: ImportBindingSource::ChainedFixup,
                is_weak: binding_is_weak,
            });
        } else {
            summary.has_rebases = true;
        }

        if pointer.next == 0 {
            break;
        }
        let delta = pointer.next.checked_mul(stride).ok_or_else(|| {
            MachoError::MalformedDyldPayload(format!(
                "chained pointer delta overflow in {segment_name}: page={page_index} offset={offset_in_segment:#x}"
            ))
        })?;
        offset_in_segment = offset_in_segment.saturating_add(delta);
        steps = steps.saturating_add(1);
    }
    Ok(())
}

fn materialize_stub_entries(
    bindings: &[ImportBindingRecord],
    exported_symbols: &[ExportRecord],
) -> Vec<StubEntry> {
    let mut exports_by_address = std::collections::BTreeMap::<u64, String>::new();
    for export in exported_symbols {
        if let Some(address) = export.address {
            exports_by_address
                .entry(address)
                .or_insert(export.name.clone());
        }
    }

    let mut stubs = bindings
        .iter()
        .filter_map(|binding| {
            let pointer_address = binding.address?;
            let export_name = exports_by_address.get(&pointer_address).cloned();
            Some(StubEntry {
                stub_address: pointer_address,
                section: None,
                pointer_section: None,
                pointer_address: Some(pointer_address),
                helper_address: None,
                binding_ordinal: binding.ordinal,
                stub_kind: match binding.binding_kind {
                    ImportBindingKind::Lazy => StubKind::Lazy,
                    ImportBindingKind::NonLazy | ImportBindingKind::ChainedFixup => {
                        StubKind::NonLazy
                    }
                },
                dylib: Some(binding.dylib.clone()),
                name: Some(export_name.unwrap_or_else(|| binding.name.clone())),
                source: binding.source,
            })
        })
        .collect::<Vec<_>>();

    stubs.sort_by_key(|stub| {
        (
            stub.stub_address,
            stub.pointer_address.unwrap_or_default(),
            match stub.source {
                ImportBindingSource::ChainedFixup => 0u8,
                ImportBindingSource::IndirectSymbol => 1u8,
                ImportBindingSource::Stub => 2u8,
                ImportBindingSource::Other => 3u8,
            },
            stub.dylib.as_ref().map_or_else(String::new, Clone::clone),
            stub.name.as_ref().map_or_else(String::new, Clone::clone),
        )
    });
    stubs.dedup();
    stubs
}

struct DecodedChainedPointer {
    next: u64,
    is_bind: bool,
    bind_ordinal: Option<u32>,
}

fn decode_chained_pointer(pointer_format: u16, entry: &[u8]) -> Option<DecodedChainedPointer> {
    match pointer_format {
        DYLD_CHAINED_PTR_ARM64E | DYLD_CHAINED_PTR_ARM64E_USERLAND => {
            let raw = read_u64(entry, 0)?;
            let is_bind = ((raw >> 62) & 0x1) != 0;
            let next = (raw >> 51) & 0x7ff;
            let bind_ordinal = if is_bind {
                Some((raw & 0xffff) as u32)
            } else {
                None
            };
            Some(DecodedChainedPointer {
                next,
                is_bind,
                bind_ordinal,
            })
        }
        DYLD_CHAINED_PTR_ARM64E_USERLAND24 => {
            let raw = read_u64(entry, 0)?;
            let is_bind = ((raw >> 62) & 0x1) != 0;
            let next = (raw >> 51) & 0x7ff;
            let bind_ordinal = if is_bind {
                Some((raw & 0x00ff_ffff) as u32)
            } else {
                None
            };
            Some(DecodedChainedPointer {
                next,
                is_bind,
                bind_ordinal,
            })
        }
        DYLD_CHAINED_PTR_64 | DYLD_CHAINED_PTR_64_OFFSET => {
            let raw = read_u64(entry, 0)?;
            let is_bind = ((raw >> 63) & 0x1) != 0;
            let next = (raw >> 51) & 0x0fff;
            let bind_ordinal = if is_bind {
                Some((raw & 0x00ff_ffff) as u32)
            } else {
                None
            };
            Some(DecodedChainedPointer {
                next,
                is_bind,
                bind_ordinal,
            })
        }
        DYLD_CHAINED_PTR_32 => {
            let raw = u64::from(read_u32(entry, 0)?);
            let is_bind = ((raw >> 31) & 0x1) != 0;
            let next = (raw >> 26) & 0x1f;
            let bind_ordinal = if is_bind {
                Some((raw & 0x000f_ffff) as u32)
            } else {
                None
            };
            Some(DecodedChainedPointer {
                next,
                is_bind,
                bind_ordinal,
            })
        }
        DYLD_CHAINED_PTR_ARM64E_SEGMENTED => {
            let raw = read_u64(entry, 0)?;
            let next = (raw >> (32 + 19)) & 0x0fff;
            Some(DecodedChainedPointer {
                next,
                is_bind: false,
                bind_ordinal: None,
            })
        }
        _ => None,
    }
}

fn pointer_stride(pointer_format: u16) -> Option<u64> {
    match pointer_format {
        DYLD_CHAINED_PTR_ARM64E
        | DYLD_CHAINED_PTR_ARM64E_USERLAND
        | DYLD_CHAINED_PTR_ARM64E_USERLAND24 => Some(8),
        DYLD_CHAINED_PTR_64
        | DYLD_CHAINED_PTR_64_OFFSET
        | DYLD_CHAINED_PTR_32
        | DYLD_CHAINED_PTR_ARM64E_SEGMENTED => Some(4),
        _ => None,
    }
}

fn pointer_width(pointer_format: u16) -> Option<usize> {
    match pointer_format {
        DYLD_CHAINED_PTR_32 => Some(4),
        DYLD_CHAINED_PTR_ARM64E
        | DYLD_CHAINED_PTR_64
        | DYLD_CHAINED_PTR_64_OFFSET
        | DYLD_CHAINED_PTR_ARM64E_USERLAND
        | DYLD_CHAINED_PTR_ARM64E_USERLAND24
        | DYLD_CHAINED_PTR_ARM64E_SEGMENTED => Some(8),
        _ => None,
    }
}

fn resolve_chained_dylib_name(libs: &[&str], ordinal: i32) -> String {
    match ordinal {
        0 => "self".to_string(),
        -1 => "main-executable".to_string(),
        -2 => "flat-lookup".to_string(),
        -3 => "weak-lookup".to_string(),
        value if value > 0 => libs
            .get(value as usize)
            .copied()
            .unwrap_or("<unknown-dylib>")
            .to_string(),
        _ => format!("ordinal({ordinal})"),
    }
}

fn linkedit_range<'a>(bytes: &'a [u8], offset: u64, size: u64) -> Result<&'a [u8]> {
    let start = usize::try_from(offset).map_err(|_| MachoError::LinkeditRangeOutOfBounds {
        offset,
        size,
        file_len: bytes.len() as u64,
    })?;
    let len = usize::try_from(size).map_err(|_| MachoError::LinkeditRangeOutOfBounds {
        offset,
        size,
        file_len: bytes.len() as u64,
    })?;
    let end = start
        .checked_add(len)
        .ok_or(MachoError::LinkeditRangeOutOfBounds {
            offset,
            size,
            file_len: bytes.len() as u64,
        })?;
    bytes
        .get(start..end)
        .ok_or(MachoError::LinkeditRangeOutOfBounds {
            offset,
            size,
            file_len: bytes.len() as u64,
        })
}

fn read_c_string(data: &[u8], offset: usize) -> Option<String> {
    let tail = data.get(offset..)?;
    let end = tail.iter().position(|byte| *byte == 0)?;
    Some(String::from_utf8_lossy(&tail[..end]).into_owned())
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    let bytes = data.get(offset..offset + 8)?;
    Some(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_export_record_keeps_structured_flag_bits_for_regular_and_absolute_exports() {
        let regular = build_export_record(
            "_regular".to_string(),
            0x44,
            &ExportInfo::Regular {
                address: 0x44,
                flags: 0,
            },
            0x1000,
        );
        assert_eq!(regular.address, Some(0x1044));
        assert_eq!(regular.flags.raw_bits, 0);
        assert_eq!(regular.flags.kind_bits, 0);
        assert!(!regular.flags.is_absolute);

        let absolute = build_export_record(
            "_absolute".to_string(),
            0x1234,
            &ExportInfo::Regular {
                address: 0x1234,
                flags: EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE,
            },
            0x1000,
        );
        assert!(matches!(absolute.kind, ExportKind::Absolute));
        assert_eq!(absolute.address, Some(0x1234));
        assert!(absolute.flags.is_absolute);
        assert_eq!(absolute.flags.raw_bits, u64::from(EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE));
    }

    #[test]
    fn build_export_record_handles_hand_authored_reexport_and_stub_exports() {
        let libs = ["", "/usr/lib/libSystem.B.dylib"];
        let reexport_info = ExportInfo::parse(
            &[0x01, b'_', b'p', b'u', b't', b's', 0x00],
            &libs,
            EXPORT_SYMBOL_FLAGS_REEXPORT,
            0,
        )
        .expect("parse reexport info");
        let reexport = build_export_record("_alias_puts".to_string(), 0, &reexport_info, 0x1000);
        assert!(matches!(
            reexport.kind,
            ExportKind::Reexport {
                ref dylib,
                symbol: Some(ref symbol)
            } if dylib == "/usr/lib/libSystem.B.dylib" && symbol == "_puts"
        ));
        assert_eq!(reexport.address, None);
        assert!(reexport.flags.is_reexport);

        let stub_info = ExportInfo::parse(
            &[0x20, 0x30],
            &[],
            EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER,
            0,
        )
        .expect("parse stub export info");
        let stub = build_export_record("_resolver".to_string(), 0, &stub_info, 0x2000);
        assert!(matches!(
            stub.kind,
            ExportKind::StubAndResolver {
                stub_address: Some(0x2020),
                resolver_address: Some(0x2030)
            }
        ));
        assert_eq!(stub.address, Some(0x2020));
        assert_eq!(stub.resolver_target, Some(0x2030));
        assert!(stub.flags.is_stub_and_resolver);
    }

    #[test]
    fn build_export_record_preserves_unknown_export_flag_bits() {
        let export = build_export_record(
            "_mystery".to_string(),
            0x88,
            &ExportInfo::Regular {
                address: 0x88,
                flags: 0x80,
            },
            0x1000,
        );
        assert!(matches!(export.kind, ExportKind::Regular));
        assert_eq!(export.flags.raw_bits, 0x80);
        assert_eq!(export.flags.unknown_bits, 0x80);
    }
}
