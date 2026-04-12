use damsel_core::{
    Annotation, Architecture, BinaryFormat, BinaryImage, DecodedInstruction, DisassemblyRequest,
    DisassemblyResult, DisassemblyTarget, DyldMetadata, Endianness, ExportedSymbol, Import,
    ObjcMetadata, Reference, Relocation, Section, Segment, SliceInfo, Symbol, SymbolKind,
};
use goblin::mach::Mach;
use goblin::mach::load_command;
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
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MachoError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("object parse error: {0}")]
    Object(#[from] object::Error),
    #[error("mach-o parse error: {0}")]
    Goblin(#[from] goblin::error::Error),
    #[error("decode error: {0}")]
    Decode(#[from] damsel_core::DecodeError),
    #[error("unsupported file kind: {0}")]
    UnsupportedFileKind(String),
    #[error("unsupported architecture: {0}")]
    UnsupportedArchitecture(String),
    #[error("symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("section not found: {0}")]
    SectionNotFound(String),
    #[error("address {0:#x} is not mapped")]
    AddressNotMapped(u64),
    #[error("section `{0}` has no file-backed contents")]
    SectionHasNoFileData(String),
}

pub type Result<T> = std::result::Result<T, MachoError>;

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

pub fn disassemble(image: &BinaryImage, request: &DisassemblyRequest) -> Result<DisassemblyResult> {
    let (target, start_address, bytes) = resolve_disassembly_target(image, request)?;
    let mut instructions =
        damsel_core::decode_aarch64(bytes, start_address, request.max_instructions)?;
    annotate_instructions(image, &mut instructions);

    Ok(DisassemblyResult {
        target,
        start_address,
        bytes_len: bytes.len(),
        instructions,
    })
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

fn collect_objc_metadata(bytes: &[u8], sections: &[Section]) -> ObjcMetadata {
    let mut metadata = ObjcMetadata::default();

    for section in sections {
        let Some(offset) = section.file_offset else {
            continue;
        };
        let start = offset as usize;
        let end = start.saturating_add(section.file_size as usize);
        let Some(contents) = bytes.get(start..end) else {
            continue;
        };

        match section.name.as_str() {
            "__objc_classname" => metadata.class_names.extend(read_c_strings(contents)),
            "__objc_methname" => {
                let names = read_c_strings(contents);
                metadata.method_names.extend(names.clone());
                metadata.selector_names.extend(names);
            }
            "__objc_imageinfo" if contents.len() >= 8 => {
                metadata.image_info_flags = Some(u32::from_le_bytes([
                    contents[4],
                    contents[5],
                    contents[6],
                    contents[7],
                ]));
            }
            _ => {}
        }
    }

    metadata.class_names.sort();
    metadata.class_names.dedup();
    metadata.method_names.sort();
    metadata.method_names.dedup();
    metadata.selector_names.sort();
    metadata.selector_names.dedup();
    metadata
}

fn collect_dyld_metadata(
    macho: &goblin::mach::MachO<'_>,
    bytes: &[u8],
    segments: &[Segment],
) -> Result<DyldMetadata> {
    let text_base = segments
        .iter()
        .find(|segment| segment.name == "__TEXT")
        .map(|segment| segment.address)
        .unwrap_or_default();
    let function_starts = parse_function_starts(bytes, text_base, &macho.load_commands);
    let exported_symbols = macho
        .exports()
        .unwrap_or_default()
        .into_iter()
        .map(|export| ExportedSymbol {
            name: export.name,
            address: Some(export.offset),
            flags: format!("{:?}", export.info),
        })
        .collect::<Vec<_>>();

    let mut has_rebases = false;
    let mut has_binds = false;
    let mut has_chained_fixups = false;

    for command in &macho.load_commands {
        match &command.command {
            load_command::CommandVariant::DyldInfo(info)
            | load_command::CommandVariant::DyldInfoOnly(info) => {
                has_rebases |= info.rebase_size > 0;
                has_binds |= info.bind_size > 0 || info.lazy_bind_size > 0;
            }
            load_command::CommandVariant::DyldChainedFixups(_) => has_chained_fixups = true,
            _ => {}
        }
    }

    Ok(DyldMetadata {
        imported_dylibs: macho.libs.iter().skip(1).map(ToString::to_string).collect(),
        rpaths: macho.rpaths.iter().map(ToString::to_string).collect(),
        exported_symbols,
        function_starts,
        has_rebases,
        has_binds,
        has_chained_fixups,
    })
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

fn detect_platform(commands: &[goblin::mach::load_command::LoadCommand]) -> Option<String> {
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

fn resolve_disassembly_target<'a>(
    image: &'a BinaryImage,
    request: &DisassemblyRequest,
) -> Result<(String, u64, &'a [u8])> {
    match &request.target {
        DisassemblyTarget::Section(section_name) => {
            let section = image
                .section_by_name(section_name)
                .ok_or_else(|| MachoError::SectionNotFound(section_name.clone()))?;
            let data = image
                .bytes_for_section(section)
                .ok_or_else(|| MachoError::SectionHasNoFileData(section.full_name()))?;
            let limit_bytes = request
                .max_instructions
                .map(|count| count.saturating_mul(4));
            Ok((
                section.full_name(),
                section.address,
                clamp_bytes(data, limit_bytes),
            ))
        }
        DisassemblyTarget::Address(address) => {
            let section = image
                .containing_section(*address)
                .ok_or(MachoError::AddressNotMapped(*address))?;
            let file_offset = section
                .file_offset
                .ok_or_else(|| MachoError::SectionHasNoFileData(section.full_name()))?;
            let delta = address.saturating_sub(section.address);
            let available = section.size.saturating_sub(delta) as usize;
            let size = request
                .max_instructions
                .map(|count| count.saturating_mul(4))
                .unwrap_or(available)
                .min(available);
            let data = image
                .bytes_for_file_range(file_offset + delta, size as u64)
                .ok_or(MachoError::AddressNotMapped(*address))?;
            Ok((format!("{address:#x}"), *address, data))
        }
        DisassemblyTarget::Symbol(symbol_name) => {
            let symbol = image
                .symbol_by_name(symbol_name)
                .ok_or_else(|| MachoError::SymbolNotFound(symbol_name.clone()))?;
            let section = image
                .containing_section(symbol.address)
                .ok_or(MachoError::AddressNotMapped(symbol.address))?;
            let file_offset = section
                .file_offset
                .ok_or_else(|| MachoError::SectionHasNoFileData(section.full_name()))?;
            let delta = symbol.address.saturating_sub(section.address);
            let inferred_size = if symbol.size > 0 {
                symbol.size
            } else {
                next_symbol_boundary(image, symbol, section)
                    .unwrap_or(section.address.saturating_add(section.size))
                    .saturating_sub(symbol.address)
            };
            let size = request
                .max_instructions
                .map(|count| (count.saturating_mul(4)) as u64)
                .unwrap_or(inferred_size)
                .min(inferred_size)
                .min(section.size.saturating_sub(delta));
            let data = image
                .bytes_for_file_range(file_offset + delta, size)
                .ok_or(MachoError::AddressNotMapped(symbol.address))?;
            Ok((symbol.name.clone(), symbol.address, data))
        }
    }
}

fn next_symbol_boundary(image: &BinaryImage, symbol: &Symbol, section: &Section) -> Option<u64> {
    image
        .symbols
        .iter()
        .filter(|candidate| {
            candidate.defined
                && candidate.address > symbol.address
                && image
                    .containing_section(candidate.address)
                    .map(|candidate_section| candidate_section.full_name() == section.full_name())
                    .unwrap_or(false)
        })
        .map(|candidate| candidate.address)
        .min()
}

fn clamp_bytes(bytes: &[u8], limit: Option<usize>) -> &[u8] {
    match limit {
        Some(limit) => &bytes[..bytes.len().min(limit)],
        None => bytes,
    }
}

fn annotate_instructions(image: &BinaryImage, instructions: &mut [DecodedInstruction]) {
    let symbol_map = image
        .symbols
        .iter()
        .filter(|symbol| symbol.defined && symbol.address != 0)
        .fold(BTreeMap::<u64, Vec<&Symbol>>::new(), |mut acc, symbol| {
            acc.entry(symbol.address).or_default().push(symbol);
            acc
        });
    let import_map =
        image
            .imports
            .iter()
            .fold(BTreeMap::<u64, Vec<&Import>>::new(), |mut acc, import| {
                if let Some(address) = import.address {
                    acc.entry(address).or_default().push(import);
                }
                acc
            });

    for instruction in instructions {
        if let Some(symbols) = symbol_map.get(&instruction.address) {
            for symbol in symbols {
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::Symbol(symbol.name.clone()),
                );
            }
        }

        for reference in instruction.references.clone() {
            match reference {
                Reference::Call { target }
                | Reference::Branch { target }
                | Reference::Page { target }
                | Reference::Data { target } => {
                    if let Some(symbols) = symbol_map.get(&target) {
                        for symbol in symbols {
                            push_annotation(
                                &mut instruction.annotations,
                                Annotation::TargetSymbol {
                                    address: target,
                                    name: symbol.name.clone(),
                                },
                            );
                        }
                    }
                    if let Some(imports) = import_map.get(&target) {
                        for import in imports {
                            push_annotation(
                                &mut instruction.annotations,
                                Annotation::Import {
                                    dylib: import.dylib.clone(),
                                    name: import.name.clone(),
                                },
                            );
                        }
                    }
                }
                Reference::Import { .. } => {}
            }
        }

        for relocation in image.relocations.iter().filter(|relocation| {
            relocation.address >= instruction.address
                && relocation.address
                    < instruction
                        .address
                        .saturating_add(u64::from(instruction.size))
        }) {
            push_annotation(
                &mut instruction.annotations,
                Annotation::Note(format!(
                    "reloc {} -> {}",
                    relocation.kind, relocation.target
                )),
            );
        }
    }
}

fn push_annotation(target: &mut Vec<Annotation>, annotation: Annotation) {
    if !target.contains(&annotation) {
        target.push(annotation);
    }
}

fn read_c_strings(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|slice| !slice.is_empty())
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/bin")
            .join(name)
    }

    #[test]
    fn loads_universal_binary_arm64_slice() {
        let image = load(fixture_path("universal-hello")).expect("load universal binary");
        assert_eq!(image.architecture, Architecture::Arm64);
        assert!(image.slice.is_universal);
    }

    #[test]
    fn stripped_binary_exposes_sections_imports_and_disassembly() {
        let image = load(fixture_path("arm64-stripped")).expect("load stripped fixture");
        assert!(!image.sections.is_empty());
        assert!(!image.imports.is_empty());

        let request = DisassemblyRequest {
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: Some(12),
        };
        let result = disassemble(&image, &request).expect("disassemble stripped fixture");
        assert!(!result.instructions.is_empty());
    }

    #[test]
    fn symbolized_binary_resolves_symbol_annotations() {
        let image = load(fixture_path("arm64-symbolized")).expect("load symbolized fixture");
        let request = DisassemblyRequest {
            target: DisassemblyTarget::Symbol("_main".to_string()),
            max_instructions: Some(12),
        };
        let result = disassemble(&image, &request).expect("disassemble main");
        let annotations = result
            .instructions
            .iter()
            .flat_map(|instruction| instruction.annotations.iter())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert!(
            annotations
                .iter()
                .any(|annotation| annotation.contains("_main"))
        );
    }

    #[test]
    fn objc_fixture_exposes_objc_metadata() {
        let image = load(fixture_path("objc-sample")).expect("load objc fixture");
        assert!(
            image
                .objc
                .class_names
                .iter()
                .any(|class_name| class_name.contains("Greeter"))
        );
        assert!(
            image
                .objc
                .selector_names
                .iter()
                .any(|selector| selector.contains("greeting"))
        );
    }

    #[test]
    fn truncated_fixture_returns_error_without_panicking() {
        let error = load(fixture_path("malformed-truncated")).expect_err("expected parse failure");
        assert!(matches!(
            error,
            MachoError::Object(_) | MachoError::Goblin(_) | MachoError::UnsupportedFileKind(_)
        ));
    }
}
