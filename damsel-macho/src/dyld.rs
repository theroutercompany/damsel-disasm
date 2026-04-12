use crate::errors::Result;
use damsel_core::{DyldMetadata, ExportedSymbol, Segment};
use goblin::mach::load_command;

pub(crate) fn collect_dyld_metadata(
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
