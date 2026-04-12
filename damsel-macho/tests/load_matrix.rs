use damsel_core::Architecture;
use damsel_macho::{MachoError, load};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

fn optional_fixture(names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| fixture(name))
        .find(|path| path.exists())
}

const MH_MAGIC_64: u32 = 0xfeedfacf;
const LC_SEGMENT_64: u32 = 0x19;
const LC_DYLD_INFO: u32 = 0x22;
const LC_DYLD_INFO_ONLY: u32 = 0x8000_0022;
const LC_DYLD_EXPORTS_TRIE: u32 = 0x8000_0033;
const LC_DYLD_CHAINED_FIXUPS: u32 = 0x8000_0034;

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let bytes = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) -> bool {
    let Some(target) = bytes.get_mut(offset..offset + 4) else {
        return false;
    };
    target.copy_from_slice(&value.to_le_bytes());
    true
}

fn find_chained_fixups_command(bytes: &[u8]) -> Option<(usize, usize)> {
    if read_u32_le(bytes, 0)? != MH_MAGIC_64 {
        return None;
    }
    let ncmds = read_u32_le(bytes, 16)? as usize;
    let mut offset = 32usize;
    for _ in 0..ncmds {
        let cmd = read_u32_le(bytes, offset)?;
        let cmdsize = read_u32_le(bytes, offset + 4)? as usize;
        if cmdsize < 8 {
            return None;
        }
        if cmd == LC_DYLD_CHAINED_FIXUPS {
            let dataoff = read_u32_le(bytes, offset + 8)? as usize;
            let datasize = read_u32_le(bytes, offset + 12)? as usize;
            return Some((dataoff, datasize));
        }
        offset = offset.checked_add(cmdsize)?;
    }
    None
}

fn corrupt_export_offsets(bytes: &mut [u8]) -> bool {
    if read_u32_le(bytes, 0) != Some(MH_MAGIC_64) {
        return false;
    }
    let Some(ncmds) = read_u32_le(bytes, 16).map(|value| value as usize) else {
        return false;
    };
    let mut offset = 32usize;
    let mut patched = false;
    for _ in 0..ncmds {
        let Some(cmd) = read_u32_le(bytes, offset) else {
            return patched;
        };
        let Some(cmdsize) = read_u32_le(bytes, offset + 4).map(|value| value as usize) else {
            return patched;
        };
        if cmdsize < 8 {
            return patched;
        }
        let bad_offset = (bytes.len() as u32).saturating_add(0x2000);
        match cmd {
            LC_DYLD_EXPORTS_TRIE => {
                patched |= write_u32_le(bytes, offset + 8, bad_offset);
                patched |= write_u32_le(bytes, offset + 12, 0x100);
            }
            LC_DYLD_INFO | LC_DYLD_INFO_ONLY => {
                patched |= write_u32_le(bytes, offset + 40, bad_offset);
                patched |= write_u32_le(bytes, offset + 44, 0x100);
            }
            _ => {}
        }
        let Some(next_offset) = offset.checked_add(cmdsize) else {
            return patched;
        };
        offset = next_offset;
    }
    patched
}

fn find_section_file_range(bytes: &[u8], segment: &str, section: &str) -> Option<(usize, usize)> {
    if read_u32_le(bytes, 0)? != MH_MAGIC_64 {
        return None;
    }
    let ncmds = read_u32_le(bytes, 16)? as usize;
    let mut offset = 32usize;
    for _ in 0..ncmds {
        let cmd = read_u32_le(bytes, offset)?;
        let cmdsize = read_u32_le(bytes, offset + 4)? as usize;
        if cmdsize < 72 {
            return None;
        }
        if cmd == LC_SEGMENT_64 {
            let segname = bytes
                .get(offset + 8..offset + 24)?
                .split(|byte| *byte == 0)
                .next()
                .map(String::from_utf8_lossy)?
                .to_string();
            let nsects = read_u32_le(bytes, offset + 64)? as usize;
            let mut section_offset = offset + 72;
            for _ in 0..nsects {
                let sectname = bytes
                    .get(section_offset..section_offset + 16)?
                    .split(|byte| *byte == 0)
                    .next()
                    .map(String::from_utf8_lossy)?
                    .to_string();
                let seg_for_section = bytes
                    .get(section_offset + 16..section_offset + 32)?
                    .split(|byte| *byte == 0)
                    .next()
                    .map(String::from_utf8_lossy)?
                    .to_string();
                if segname == segment && seg_for_section == segment && sectname == section {
                    let fileoff = read_u32_le(bytes, section_offset + 48)? as usize;
                    let size = u64::from_le_bytes(
                        bytes
                            .get(section_offset + 40..section_offset + 48)?
                            .try_into()
                            .ok()?,
                    ) as usize;
                    return Some((fileoff, size));
                }
                section_offset = section_offset.checked_add(80)?;
            }
        }
        offset = offset.checked_add(cmdsize)?;
    }
    None
}

fn write_temp_fixture(bytes: &[u8]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let counter = TEMP_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "damsel-test-{}-{nanos}-{counter}.bin",
        std::process::id()
    ));
    fs::write(&path, bytes).expect("write temp fixture");
    path
}

#[test]
fn loads_universal_fixture_with_arm64_slice() {
    let image = load(fixture("universal-hello")).expect("load universal fixture");
    assert_eq!(image.architecture(), Architecture::Arm64);
    assert!(image.selected_slice().is_universal);
}

#[test]
fn loads_objc_fixture_metadata() {
    let image = load(fixture("objc-sample")).expect("load objc fixture");
    assert!(!image.objc().class_names.is_empty());
    assert!(!image.objc().selector_names.is_empty());
}

#[test]
fn rejects_non_macho_fixture_without_panicking() {
    let path = write_temp_fixture(b"this is not a macho file");
    let error = load(&path).expect_err("non-mach-o should fail");
    let _ = fs::remove_file(path);
    assert!(matches!(
        error,
        MachoError::UnsupportedFileKind(_) | MachoError::Object(_) | MachoError::Goblin(_)
    ));
}

#[test]
fn rejects_truncated_fixture_without_panicking() {
    let error = load(fixture("malformed-truncated")).expect_err("expected parse failure");
    assert!(matches!(
        error,
        MachoError::Object(_) | MachoError::Goblin(_) | MachoError::UnsupportedFileKind(_)
    ));
}

#[test]
fn rejects_malformed_dysymtab_fixture_without_panicking() {
    let path = fixture("malformed-dysymtab-indirect");
    if !path.exists() {
        eprintln!("malformed-dysymtab-indirect fixture not present; skipping");
        return;
    }
    let error = load(path).expect_err("expected malformed dyld fixture to fail");
    assert!(matches!(error, MachoError::MalformedDyldPayload(_)));
}

#[test]
fn rejects_malformed_stub_helper_fixture_without_panicking() {
    let Some(path) = optional_fixture(&[
        "malformed-stub-helper-size",
        "malformed-stub-reserved2",
        "malformed-stub-helper-truncated",
        "malformed-stub-helper",
        "malformed-helper-truncated",
    ]) else {
        eprintln!("malformed stub-helper fixture not present; skipping");
        return;
    };
    let error = load(path).expect_err("expected malformed stub-helper fixture to fail");
    assert!(matches!(error, MachoError::MalformedDyldPayload(_)));
}

#[test]
fn rejects_malformed_chained_starts_without_panicking() {
    let path = fixture("arm64-symbolized");
    let mut bytes = fs::read(&path).expect("read fixture");
    let Some((fixups_offset, fixups_size)) = find_chained_fixups_command(&bytes) else {
        eprintln!("dyld chained-fixups command not present in fixture; skipping");
        return;
    };
    if fixups_size < 28 {
        eprintln!("chained-fixups payload too small in fixture; skipping");
        return;
    }
    let bad_starts_offset = fixups_size.saturating_add(0x100) as u32;
    assert!(write_u32_le(
        &mut bytes,
        fixups_offset.saturating_add(4),
        bad_starts_offset
    ));
    let malformed = write_temp_fixture(&bytes);
    let error = load(&malformed).expect_err("expected malformed chained starts to fail");
    let _ = fs::remove_file(malformed);
    match error {
        MachoError::MalformedDyldPayload(message) => {
            assert!(message.contains("chained starts offset"));
        }
        other => panic!("unexpected error kind: {other}"),
    }
}

#[test]
fn rejects_malformed_chained_import_table_without_panicking() {
    let path = fixture("arm64-symbolized");
    let mut bytes = fs::read(&path).expect("read fixture");
    let Some((fixups_offset, fixups_size)) = find_chained_fixups_command(&bytes) else {
        eprintln!("dyld chained-fixups command not present in fixture; skipping");
        return;
    };
    if fixups_size < 28 {
        eprintln!("chained-fixups payload too small in fixture; skipping");
        return;
    }
    let malformed_imports_offset = fixups_size.saturating_sub(2) as u32;
    assert!(write_u32_le(
        &mut bytes,
        fixups_offset.saturating_add(8),
        malformed_imports_offset
    ));
    assert!(write_u32_le(
        &mut bytes,
        fixups_offset.saturating_add(16),
        1
    ));
    let malformed = write_temp_fixture(&bytes);
    let error = load(&malformed).expect_err("expected malformed chained imports to fail");
    let _ = fs::remove_file(malformed);
    match error {
        MachoError::MalformedDyldPayload(message) => {
            assert!(message.contains("chained imports"));
        }
        other => panic!("unexpected error kind: {other}"),
    }
}

#[test]
fn rejects_malformed_stub_helper_opcode_without_panicking() {
    let path = fixture("import-lazy");
    if !path.exists() {
        eprintln!("import-lazy fixture not present; skipping");
        return;
    }
    let mut bytes = fs::read(&path).expect("read fixture");
    let Some((helper_off, helper_size)) =
        find_section_file_range(&bytes, "__TEXT", "__stub_helper")
    else {
        eprintln!("__TEXT,__stub_helper section not found; skipping");
        return;
    };
    if helper_size < 4 {
        eprintln!("__stub_helper section too small; skipping");
        return;
    }
    let Some(helper_slice) = bytes.get_mut(helper_off..helper_off.saturating_add(helper_size))
    else {
        eprintln!("__stub_helper section range out of file bounds; skipping");
        return;
    };
    helper_slice.fill(0);
    let malformed = write_temp_fixture(&bytes);
    let error = load(&malformed).expect_err("expected malformed helper opcode to fail");
    let _ = fs::remove_file(malformed);
    match error {
        MachoError::MalformedDyldPayload(message) => {
            assert!(message.contains("__stub_helper"));
        }
        other => panic!("unexpected error kind: {other}"),
    }
}

#[test]
fn rejects_malformed_export_trie_without_panicking() {
    let path = fixture("arm64-symbolized");
    let mut bytes = fs::read(&path).expect("read fixture");
    if !corrupt_export_offsets(&mut bytes) {
        eprintln!("export-bearing load command not present in fixture; skipping");
        return;
    }
    let malformed = write_temp_fixture(&bytes);
    let error = load(&malformed).expect_err("expected malformed export trie to fail");
    let _ = fs::remove_file(malformed);
    match error {
        MachoError::MalformedDyldPayload(message) => {
            assert!(message.contains("export trie"));
        }
        other => panic!("unexpected error kind: {other}"),
    }
}
