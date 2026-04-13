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
const FAT_MAGIC: u32 = 0xcafebabe;
const FAT_MAGIC_64: u32 = 0xcafebabf;
const CPU_TYPE_ARM64: u32 = 0x0100_000c;
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
const CPU_SUBTYPE_ARM64_ALL: u32 = 0;
const CPU_SUBTYPE_ARM64E: u32 = 2;
const CPU_SUBTYPE_MASK: u32 = object::macho::CPU_SUBTYPE_MASK;
const LC_SEGMENT_64: u32 = 0x19;
const LC_BUILD_VERSION: u32 = 0x32;
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

fn write_u32_be(bytes: &mut [u8], offset: usize, value: u32) -> bool {
    let Some(target) = bytes.get_mut(offset..offset + 4) else {
        return false;
    };
    target.copy_from_slice(&value.to_be_bytes());
    true
}

fn write_u64_be(bytes: &mut [u8], offset: usize, value: u64) -> bool {
    let Some(target) = bytes.get_mut(offset..offset + 8) else {
        return false;
    };
    target.copy_from_slice(&value.to_be_bytes());
    true
}

fn make_fat32_fixture(arches: &[(u32, u32, u32, u32, u32)]) -> Vec<u8> {
    let mut bytes = vec![0u8; 8 + arches.len() * 20];
    assert!(write_u32_be(&mut bytes, 0, FAT_MAGIC));
    assert!(write_u32_be(&mut bytes, 4, arches.len() as u32));
    for (index, (cputype, cpusubtype, offset, size, align)) in arches.iter().enumerate() {
        let base = 8 + index * 20;
        assert!(write_u32_be(&mut bytes, base, *cputype));
        assert!(write_u32_be(&mut bytes, base + 4, *cpusubtype));
        assert!(write_u32_be(&mut bytes, base + 8, *offset));
        assert!(write_u32_be(&mut bytes, base + 12, *size));
        assert!(write_u32_be(&mut bytes, base + 16, *align));
    }
    bytes
}

fn make_fat64_fixture(arches: &[(u32, u32, u64, u64, u32, u32)]) -> Vec<u8> {
    let mut bytes = vec![0u8; 8 + arches.len() * 32];
    assert!(write_u32_be(&mut bytes, 0, FAT_MAGIC_64));
    assert!(write_u32_be(&mut bytes, 4, arches.len() as u32));
    for (index, (cputype, cpusubtype, offset, size, align, reserved)) in arches.iter().enumerate() {
        let base = 8 + index * 32;
        assert!(write_u32_be(&mut bytes, base, *cputype));
        assert!(write_u32_be(&mut bytes, base + 4, *cpusubtype));
        assert!(write_u64_be(&mut bytes, base + 8, *offset));
        assert!(write_u64_be(&mut bytes, base + 16, *size));
        assert!(write_u32_be(&mut bytes, base + 24, *align));
        assert!(write_u32_be(&mut bytes, base + 28, *reserved));
    }
    bytes
}

fn embed_payload(bytes: &mut Vec<u8>, offset: usize, payload: &[u8]) {
    let required_len = offset.saturating_add(payload.len());
    if bytes.len() < required_len {
        bytes.resize(required_len, 0);
    }
    bytes[offset..required_len].copy_from_slice(payload);
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

fn corrupt_bind_offsets(bytes: &mut [u8]) -> bool {
    if read_u32_le(bytes, 0) != Some(MH_MAGIC_64) {
        return false;
    }
    let Some(ncmds) = read_u32_le(bytes, 16).map(|value| value as usize) else {
        return false;
    };
    let mut offset = 32usize;
    for _ in 0..ncmds {
        let Some(cmd) = read_u32_le(bytes, offset) else {
            return false;
        };
        let Some(cmdsize) = read_u32_le(bytes, offset + 4).map(|value| value as usize) else {
            return false;
        };
        if cmdsize < 48 {
            return false;
        }
        if cmd == LC_DYLD_INFO || cmd == LC_DYLD_INFO_ONLY {
            let bad_offset = (bytes.len() as u32).saturating_add(0x2000);
            let patched_offset = write_u32_le(bytes, offset + 16, bad_offset);
            let patched_size = write_u32_le(bytes, offset + 20, 0x100);
            return patched_offset && patched_size;
        }
        let Some(next_offset) = offset.checked_add(cmdsize) else {
            return false;
        };
        offset = next_offset;
    }
    false
}

fn patch_build_version_platform(bytes: &mut [u8], platform: u32) -> bool {
    if read_u32_le(bytes, 0) != Some(MH_MAGIC_64) {
        return false;
    }
    let Some(ncmds) = read_u32_le(bytes, 16).map(|value| value as usize) else {
        return false;
    };
    let mut offset = 32usize;
    for _ in 0..ncmds {
        let Some(cmd) = read_u32_le(bytes, offset) else {
            return false;
        };
        let Some(cmdsize) = read_u32_le(bytes, offset + 4).map(|value| value as usize) else {
            return false;
        };
        if cmdsize < 8 {
            return false;
        }
        if cmd == LC_BUILD_VERSION {
            return write_u32_le(bytes, offset + 8, platform);
        }
        let Some(next_offset) = offset.checked_add(cmdsize) else {
            return false;
        };
        offset = next_offset;
    }
    false
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
fn reports_platform_consistently_for_symbolized_fixture() {
    let image = load(fixture("arm64-symbolized")).expect("load arm64 fixture");
    assert_eq!(image.platform(), Some(&damsel_core::Platform::MacOS));
    assert_eq!(image.available_slices().len(), 1);
    assert_eq!(
        image.available_slices()[0].architecture,
        Architecture::Arm64
    );
}

#[test]
fn rejects_non_macho_fixture_without_panicking() {
    let path = write_temp_fixture(b"this is not a macho file");
    let error = load(&path).expect_err("non-mach-o should fail");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::UnsupportedInputKind(_)));
}

#[test]
fn rejects_thin_macho_magic_without_complete_header_as_malformed() {
    let path = write_temp_fixture(&[0xcf, 0xfa, 0xed, 0xfe]);
    let error = load(&path).expect_err("truncated thin Mach-O header must be malformed");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_universal_without_arm64_slice_with_typed_error() {
    let bytes = make_fat32_fixture(&[(CPU_TYPE_X86_64, 3, 0x1000, 0x200, 0)]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("universal without arm64/arm64e must be rejected");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MissingArm64SliceInUniversal));
}

#[test]
fn rejects_out_of_range_fat_arm64_slice_with_typed_error() {
    let bytes = make_fat32_fixture(&[(CPU_TYPE_ARM64, 0, 0x1000, 0x200, 0)]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("out-of-range fat slice must be rejected");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::SliceOutOfBounds { .. }));
}

#[test]
fn rejects_zero_size_fat_arm64_slice_with_typed_error() {
    let bytes = make_fat32_fixture(&[(CPU_TYPE_ARM64, 0, 0x1000, 0, 0)]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("zero-size fat slice must be rejected");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_truncated_fixture_without_panicking() {
    let error = load(fixture("malformed-truncated")).expect_err("expected parse failure");
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_fat64_universal_without_arm64_slice_with_typed_error() {
    let bytes = make_fat64_fixture(&[(CPU_TYPE_X86_64, 3, 0x1000, 0x200, 0, 0)]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("fat64 universal without arm64/arm64e must be rejected");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MissingArm64SliceInUniversal));
}

#[test]
fn rejects_fat32_universal_with_zero_arch_entries_as_malformed() {
    let bytes = make_fat32_fixture(&[]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("fat32 universal with zero entries must be malformed");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_fat64_universal_with_zero_arch_entries_as_malformed() {
    let bytes = make_fat64_fixture(&[]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("fat64 universal with zero entries must be malformed");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_truncated_fat32_arch_table_as_malformed() {
    let mut bytes = vec![0u8; 8];
    assert!(write_u32_be(&mut bytes, 0, FAT_MAGIC));
    assert!(write_u32_be(&mut bytes, 4, 1));
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("truncated fat32 arch table must be malformed");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_truncated_fat64_arch_table_as_malformed() {
    let mut bytes = vec![0u8; 8];
    assert!(write_u32_be(&mut bytes, 0, FAT_MAGIC_64));
    assert!(write_u32_be(&mut bytes, 4, 1));
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("truncated fat64 arch table must be malformed");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_out_of_range_fat64_arm64_slice_with_typed_error() {
    let bytes = make_fat64_fixture(&[(CPU_TYPE_ARM64, CPU_SUBTYPE_ARM64_ALL, 0x2000, 0x80, 0, 0)]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("out-of-range fat64 arm64 slice must be rejected");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::SliceOutOfBounds { .. }));
}

#[test]
fn rejects_zero_size_fat64_arm64_slice_with_typed_error() {
    let bytes = make_fat64_fixture(&[(CPU_TYPE_ARM64, CPU_SUBTYPE_ARM64_ALL, 0x1000, 0, 0, 0)]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("zero-size fat64 arm64 slice must be rejected");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_overflowing_fat64_arm64_slice_range_with_typed_error() {
    let bytes = make_fat64_fixture(&[(
        CPU_TYPE_ARM64,
        CPU_SUBTYPE_ARM64_ALL,
        u64::MAX - 0x10,
        0x40,
        0,
        0,
    )]);
    let path = write_temp_fixture(&bytes);
    let error = load(&path).expect_err("overflowing fat64 arm64 slice range must be rejected");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::SliceOutOfBounds { .. }));
}

#[test]
fn falls_back_to_valid_arm64_when_preferred_arm64e_is_invalid_fat32() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let arm64_offset = 0x200_u32;
    let arm64e_offset = 0xf000_0000_u32;
    let arm64_size = arm64_payload.len() as u32;
    let mut bytes = make_fat32_fixture(&[
        (CPU_TYPE_ARM64, CPU_SUBTYPE_ARM64E, arm64e_offset, 0x100, 0),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            arm64_offset,
            arm64_size,
            0,
        ),
    ]);
    embed_payload(&mut bytes, arm64_offset as usize, &arm64_payload);
    let path = write_temp_fixture(&bytes);
    let image = load(&path).expect("valid arm64 should be selected when arm64e is invalid");
    let _ = fs::remove_file(path);
    assert_eq!(image.architecture(), Architecture::Arm64);
    assert_eq!(image.selected_slice().offset, arm64_offset as u64);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64_ALL);
}

#[test]
fn falls_back_to_valid_arm64_when_preferred_arm64e_is_invalid_fat64() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let arm64_offset = 0x200_u64;
    let arm64e_offset = u64::MAX - 0x20;
    let arm64_size = arm64_payload.len() as u64;
    let mut bytes = make_fat64_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64E,
            arm64e_offset,
            0x40,
            0,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            arm64_offset,
            arm64_size,
            0,
            0,
        ),
    ]);
    embed_payload(&mut bytes, arm64_offset as usize, &arm64_payload);
    let path = write_temp_fixture(&bytes);
    let image = load(&path).expect("valid arm64 should be selected when arm64e is invalid");
    let _ = fs::remove_file(path);
    assert_eq!(image.architecture(), Architecture::Arm64);
    assert_eq!(image.selected_slice().offset, arm64_offset);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64_ALL);
}

#[test]
fn falls_back_to_parseable_arm64_when_preferred_arm64e_is_non_parseable_fat32() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let arm64e_payload = vec![0u8; arm64_payload.len().max(64)];
    let arm64_offset = 0x200_u32;
    let arm64e_offset = arm64_offset + arm64_payload.len() as u32 + 0x200;
    let arm64_size = arm64_payload.len() as u32;
    let mut bytes = make_fat32_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64E,
            arm64e_offset,
            arm64e_payload.len() as u32,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            arm64_offset,
            arm64_size,
            0,
        ),
    ]);
    embed_payload(&mut bytes, arm64e_offset as usize, &arm64e_payload);
    embed_payload(&mut bytes, arm64_offset as usize, &arm64_payload);
    let path = write_temp_fixture(&bytes);
    let image =
        load(&path).expect("parseable arm64 should be selected when arm64e is non-parseable");
    let _ = fs::remove_file(path);
    assert_eq!(image.architecture(), Architecture::Arm64);
    assert_eq!(image.selected_slice().offset, arm64_offset as u64);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64_ALL);
}

#[test]
fn falls_back_to_parseable_arm64_when_preferred_arm64e_is_non_parseable_fat64() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let arm64e_payload = vec![0u8; arm64_payload.len().max(64)];
    let arm64_offset = 0x200_u64;
    let arm64e_offset = arm64_offset + arm64_payload.len() as u64 + 0x200;
    let arm64_size = arm64_payload.len() as u64;
    let mut bytes = make_fat64_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64E,
            arm64e_offset,
            arm64e_payload.len() as u64,
            0,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            arm64_offset,
            arm64_size,
            0,
            0,
        ),
    ]);
    embed_payload(&mut bytes, arm64e_offset as usize, &arm64e_payload);
    embed_payload(&mut bytes, arm64_offset as usize, &arm64_payload);
    let path = write_temp_fixture(&bytes);
    let image =
        load(&path).expect("parseable arm64 should be selected when arm64e is non-parseable");
    let _ = fs::remove_file(path);
    assert_eq!(image.architecture(), Architecture::Arm64);
    assert_eq!(image.selected_slice().offset, arm64_offset);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64_ALL);
}

#[test]
fn rejects_fat32_universal_when_all_arm64_family_slices_are_non_parseable() {
    let garbage_one = vec![0x41_u8; 128];
    let garbage_two = vec![0x42_u8; 128];
    let first_offset = 0x200_u32;
    let second_offset = first_offset + garbage_one.len() as u32 + 0x200;
    let mut bytes = make_fat32_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64E,
            first_offset,
            garbage_one.len() as u32,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            second_offset,
            garbage_two.len() as u32,
            0,
        ),
    ]);
    embed_payload(&mut bytes, first_offset as usize, &garbage_one);
    embed_payload(&mut bytes, second_offset as usize, &garbage_two);
    let path = write_temp_fixture(&bytes);
    let error = load(&path)
        .expect_err("non-parseable arm64-family slices must produce malformed universal");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn rejects_fat64_universal_when_all_arm64_family_slices_are_non_parseable() {
    let garbage_one = vec![0x51_u8; 128];
    let garbage_two = vec![0x61_u8; 128];
    let first_offset = 0x200_u64;
    let second_offset = first_offset + garbage_one.len() as u64 + 0x200;
    let mut bytes = make_fat64_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64E,
            first_offset,
            garbage_one.len() as u64,
            0,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            second_offset,
            garbage_two.len() as u64,
            0,
            0,
        ),
    ]);
    embed_payload(&mut bytes, first_offset as usize, &garbage_one);
    embed_payload(&mut bytes, second_offset as usize, &garbage_two);
    let path = write_temp_fixture(&bytes);
    let error = load(&path)
        .expect_err("non-parseable arm64-family slices must produce malformed universal");
    let _ = fs::remove_file(path);
    assert!(matches!(error, MachoError::MalformedFatBinary(_)));
}

#[test]
fn chooses_valid_same_rank_arm64_candidate_when_other_is_invalid_fat32() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let valid_offset = 0x200_u32;
    let valid_size = arm64_payload.len() as u32;
    let mut bytes = make_fat32_fixture(&[
        (CPU_TYPE_ARM64, CPU_SUBTYPE_ARM64_ALL, 0xfff0_0000, 0x100, 0),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            valid_offset,
            valid_size,
            0,
        ),
    ]);
    embed_payload(&mut bytes, valid_offset as usize, &arm64_payload);
    let path = write_temp_fixture(&bytes);
    let image = load(&path).expect("valid arm64 candidate should be selected");
    let _ = fs::remove_file(path);
    assert_eq!(image.selected_slice().offset, valid_offset as u64);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64_ALL);
}

#[test]
fn chooses_valid_same_rank_arm64_candidate_when_other_is_invalid_fat64() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let valid_offset = 0x200_u64;
    let valid_size = arm64_payload.len() as u64;
    let mut bytes = make_fat64_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            u64::MAX - 0x10,
            0x40,
            0,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            valid_offset,
            valid_size,
            0,
            0,
        ),
    ]);
    embed_payload(&mut bytes, valid_offset as usize, &arm64_payload);
    let path = write_temp_fixture(&bytes);
    let image = load(&path).expect("valid arm64 candidate should be selected");
    let _ = fs::remove_file(path);
    assert_eq!(image.selected_slice().offset, valid_offset);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64_ALL);
}

#[test]
fn preserves_arm64e_preference_when_subtype_mask_bits_are_present_fat32() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let mut arm64e_payload = arm64_payload.clone();
    assert!(write_u32_le(&mut arm64e_payload, 8, CPU_SUBTYPE_ARM64E));
    let arm64_offset = 0x200_u32;
    let arm64e_offset = 0x200 + arm64_payload.len() as u32 + 0x200;
    let arm64_size = arm64_payload.len() as u32;
    let arm64e_with_mask_bits = CPU_SUBTYPE_ARM64E | CPU_SUBTYPE_MASK;
    let mut bytes = make_fat32_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            arm64_offset,
            arm64_size,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            arm64e_with_mask_bits,
            arm64e_offset,
            arm64_size,
            0,
        ),
    ]);
    embed_payload(&mut bytes, arm64_offset as usize, &arm64_payload);
    embed_payload(&mut bytes, arm64e_offset as usize, &arm64e_payload);
    let path = write_temp_fixture(&bytes);
    let image = load(&path).expect("masked arm64e subtype should still be preferred");
    let _ = fs::remove_file(path);
    assert_eq!(image.selected_slice().offset, arm64e_offset as u64);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64E);
    assert_eq!(image.architecture(), Architecture::Arm64e);
}

#[test]
fn preserves_arm64e_preference_when_subtype_mask_bits_are_present_fat64() {
    let arm64_payload = fs::read(fixture("arm64-symbolized")).expect("read arm64 payload");
    let mut arm64e_payload = arm64_payload.clone();
    assert!(write_u32_le(&mut arm64e_payload, 8, CPU_SUBTYPE_ARM64E));
    let arm64_offset = 0x200_u64;
    let arm64e_offset = arm64_offset + arm64_payload.len() as u64 + 0x200;
    let arm64_size = arm64_payload.len() as u64;
    let arm64e_with_mask_bits = CPU_SUBTYPE_ARM64E | CPU_SUBTYPE_MASK;
    let mut bytes = make_fat64_fixture(&[
        (
            CPU_TYPE_ARM64,
            CPU_SUBTYPE_ARM64_ALL,
            arm64_offset,
            arm64_size,
            0,
            0,
        ),
        (
            CPU_TYPE_ARM64,
            arm64e_with_mask_bits,
            arm64e_offset,
            arm64_size,
            0,
            0,
        ),
    ]);
    embed_payload(&mut bytes, arm64_offset as usize, &arm64_payload);
    embed_payload(&mut bytes, arm64e_offset as usize, &arm64e_payload);
    let path = write_temp_fixture(&bytes);
    let image = load(&path).expect("masked arm64e subtype should still be preferred");
    let _ = fs::remove_file(path);
    assert_eq!(image.selected_slice().offset, arm64e_offset);
    assert_eq!(image.selected_slice().cpu_subtype, CPU_SUBTYPE_ARM64E);
    assert_eq!(image.architecture(), Architecture::Arm64e);
}

#[test]
fn maps_unknown_build_version_platform_without_host_dependency() {
    let mut bytes = fs::read(fixture("arm64-symbolized")).expect("read fixture");
    if !patch_build_version_platform(&mut bytes, u32::MAX) {
        eprintln!("build-version load command not present; skipping");
        return;
    }
    let malformed = write_temp_fixture(&bytes);
    let image = load(&malformed).expect("unknown build-version platform should still load");
    let _ = fs::remove_file(malformed);
    assert!(matches!(
        image.platform(),
        Some(damsel_core::Platform::Unknown(label)) if label == "unknown"
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

#[test]
fn rejects_malformed_bind_offsets_without_leaking_raw_parse_errors() {
    let path = fixture("import-lazy");
    if !path.exists() {
        eprintln!("import-lazy fixture not present; skipping");
        return;
    }
    let mut bytes = fs::read(&path).expect("read fixture");
    if !corrupt_bind_offsets(&mut bytes) {
        eprintln!("dyld info load command not present in fixture; skipping");
        return;
    }
    let malformed = write_temp_fixture(&bytes);
    let error = load(&malformed).expect_err("expected malformed bind offsets to fail");
    let _ = fs::remove_file(malformed);
    match error {
        MachoError::MalformedDyldPayload(message) => {
            assert!(message.contains("import table parse error"));
        }
        other => panic!("unexpected error kind: {other}"),
    }
}
