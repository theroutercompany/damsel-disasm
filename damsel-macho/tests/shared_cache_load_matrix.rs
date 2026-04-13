use damsel_core::Architecture;
use damsel_macho::{MachoError, SharedCacheMemberRole, load_shared_cache};
use object::LittleEndian;
use object::macho::{
    DyldCacheHeader, DyldCacheImageInfo, DyldCacheMappingAndSlideInfo, DyldCacheMappingInfo,
    DyldSubCacheEntryV1,
};
use std::fs;
use std::mem::offset_of;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_CACHE_COUNTER: AtomicU64 = AtomicU64::new(0);

const MAGIC_ARM64: [u8; 16] = *b"dyld_v1   arm64\0";
const MAGIC_ARM64E: [u8; 16] = *b"dyld_v1  arm64e\0";
const MAGIC_X86_64: [u8; 16] = *b"dyld_v1  x86_64\0";

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut directory = std::env::temp_dir();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drift");
    let serial = TEMP_CACHE_COUNTER.fetch_add(1, Ordering::Relaxed);
    directory.push(format!(
        "damsel-{prefix}-{}-{}-{serial}",
        std::process::id(),
        now.as_nanos()
    ));
    fs::create_dir_all(&directory).expect("create temp directory");
    directory
}

fn add_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().expect("cache file name").to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
    let target = bytes.get_mut(offset..offset + 4).expect("write u32 bounds");
    target.copy_from_slice(&value.to_le_bytes());
}

fn write_u64_le(bytes: &mut [u8], offset: usize, value: u64) {
    let target = bytes.get_mut(offset..offset + 8).expect("write u64 bounds");
    target.copy_from_slice(&value.to_le_bytes());
}

fn write_header(
    bytes: &mut [u8],
    magic: [u8; 16],
    uuid: [u8; 16],
    mapping_offset: u32,
    mapping_count: u32,
    images_offset_old: u32,
    images_count_old: u32,
    images_offset_new: u32,
    images_count_new: u32,
    subcache_offset: u32,
    subcache_count: u32,
    symbol_uuid: [u8; 16],
) {
    let magic_offset = offset_of!(DyldCacheHeader<LittleEndian>, magic);
    bytes[magic_offset..magic_offset + 16].copy_from_slice(&magic);

    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, mapping_offset),
        mapping_offset,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, mapping_count),
        mapping_count,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, mapping_with_slide_offset),
        mapping_offset,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, mapping_with_slide_count),
        mapping_count,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, images_offset_old),
        images_offset_old,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, images_count_old),
        images_count_old,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, images_offset),
        images_offset_new,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, images_count),
        images_count_new,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, sub_cache_array_offset),
        subcache_offset,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, sub_cache_array_count),
        subcache_count,
    );
    write_u32_le(
        bytes,
        offset_of!(DyldCacheHeader<LittleEndian>, platform),
        1,
    );

    let uuid_offset = offset_of!(DyldCacheHeader<LittleEndian>, uuid);
    bytes[uuid_offset..uuid_offset + 16].copy_from_slice(&uuid);

    let symbols_uuid_offset = offset_of!(DyldCacheHeader<LittleEndian>, symbol_file_uuid);
    bytes[symbols_uuid_offset..symbols_uuid_offset + 16].copy_from_slice(&symbol_uuid);
}

fn write_mapping(
    bytes: &mut [u8],
    mapping_offset: usize,
    address: u64,
    size: u64,
    file_offset: u64,
    max_prot: u32,
    init_prot: u32,
) {
    write_u64_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingInfo<LittleEndian>, address),
        address,
    );
    write_u64_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingInfo<LittleEndian>, size),
        size,
    );
    write_u64_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingInfo<LittleEndian>, file_offset),
        file_offset,
    );
    write_u32_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingInfo<LittleEndian>, max_prot),
        max_prot,
    );
    write_u32_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingInfo<LittleEndian>, init_prot),
        init_prot,
    );
}

fn write_mapping_v2(
    bytes: &mut [u8],
    mapping_offset: usize,
    address: u64,
    size: u64,
    file_offset: u64,
    max_prot: u32,
    init_prot: u32,
) {
    write_u64_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingAndSlideInfo<LittleEndian>, address),
        address,
    );
    write_u64_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingAndSlideInfo<LittleEndian>, size),
        size,
    );
    write_u64_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingAndSlideInfo<LittleEndian>, file_offset),
        file_offset,
    );
    write_u64_le(
        bytes,
        mapping_offset
            + offset_of!(
                DyldCacheMappingAndSlideInfo<LittleEndian>,
                slide_info_file_offset
            ),
        0,
    );
    write_u64_le(
        bytes,
        mapping_offset
            + offset_of!(
                DyldCacheMappingAndSlideInfo<LittleEndian>,
                slide_info_file_size
            ),
        0,
    );
    write_u64_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingAndSlideInfo<LittleEndian>, flags),
        0,
    );
    write_u32_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingAndSlideInfo<LittleEndian>, max_prot),
        max_prot,
    );
    write_u32_le(
        bytes,
        mapping_offset + offset_of!(DyldCacheMappingAndSlideInfo<LittleEndian>, init_prot),
        init_prot,
    );
}

fn write_image_info(bytes: &mut [u8], images_offset: usize, address: u64, path_offset: u32) {
    write_u64_le(
        bytes,
        images_offset + offset_of!(DyldCacheImageInfo<LittleEndian>, address),
        address,
    );
    write_u32_le(
        bytes,
        images_offset + offset_of!(DyldCacheImageInfo<LittleEndian>, path_file_offset),
        path_offset,
    );
}

fn write_subcache_v1(bytes: &mut [u8], subcache_offset: usize, uuid: [u8; 16]) {
    let uuid_offset = subcache_offset + offset_of!(DyldSubCacheEntryV1<LittleEndian>, uuid);
    bytes[uuid_offset..uuid_offset + 16].copy_from_slice(&uuid);
    write_u64_le(
        bytes,
        subcache_offset + offset_of!(DyldSubCacheEntryV1<LittleEndian>, cache_vm_offset),
        0,
    );
}

fn make_single_cache_bytes(magic: [u8; 16], uuid: [u8; 16]) -> Vec<u8> {
    let mapping_offset = 0x80usize;
    let images_offset = 0xA0usize;
    let path_offset = 0xC0usize;
    let mapping_address = 0x1800_00000u64;
    let mapping_size = 0x1000u64;
    let mapping_file_offset = 0x200u64;

    let mut bytes = vec![0u8; 0x1400];
    write_header(
        &mut bytes,
        magic,
        uuid,
        mapping_offset as u32,
        1,
        images_offset as u32,
        1,
        0,
        0,
        0,
        0,
        [0u8; 16],
    );
    write_mapping_v2(
        &mut bytes,
        mapping_offset,
        mapping_address,
        mapping_size,
        mapping_file_offset,
        5,
        5,
    );
    write_image_info(
        &mut bytes,
        images_offset,
        mapping_address + 0x200,
        path_offset as u32,
    );
    let path = b"/usr/lib/libSynthetic.dylib\0";
    bytes[path_offset..path_offset + path.len()].copy_from_slice(path);
    bytes
}

fn make_root_with_subcache(root_uuid: [u8; 16], subcache_uuid: [u8; 16]) -> Vec<u8> {
    let mapping_offset = 0x1C8usize;
    let subcache_offset = 0x1F0usize;
    let images_offset = 0x220usize;
    let path_offset = 0x260usize;
    let mapping_address = 0x1800_00000u64;
    let mapping_size = 0x2000u64;
    let mapping_file_offset = 0x400u64;

    let mut bytes = vec![0u8; 0x2000];
    write_header(
        &mut bytes,
        MAGIC_ARM64E,
        root_uuid,
        mapping_offset as u32,
        1,
        0,
        0,
        images_offset as u32,
        1,
        subcache_offset as u32,
        1,
        [0u8; 16],
    );
    write_mapping_v2(
        &mut bytes,
        mapping_offset,
        mapping_address,
        mapping_size,
        mapping_file_offset,
        5,
        5,
    );
    write_subcache_v1(&mut bytes, subcache_offset, subcache_uuid);
    write_image_info(
        &mut bytes,
        images_offset,
        mapping_address + 0x300,
        path_offset as u32,
    );
    let path = b"/usr/lib/libSplitRoot.dylib\0";
    bytes[path_offset..path_offset + path.len()].copy_from_slice(path);
    bytes
}

fn make_root_with_symbols(root_uuid: [u8; 16], symbols_uuid: [u8; 16]) -> Vec<u8> {
    let mapping_offset = 0x1C8usize;
    let images_offset = 0x220usize;
    let path_offset = 0x250usize;
    let mapping_address = 0x1801_00000u64;
    let mapping_size = 0x2000u64;
    let mapping_file_offset = 0x400u64;

    let mut bytes = vec![0u8; 0x2000];
    write_header(
        &mut bytes,
        MAGIC_ARM64,
        root_uuid,
        mapping_offset as u32,
        1,
        0,
        0,
        images_offset as u32,
        1,
        0,
        0,
        symbols_uuid,
    );
    write_mapping(
        &mut bytes,
        mapping_offset,
        mapping_address,
        mapping_size,
        mapping_file_offset,
        5,
        5,
    );
    write_image_info(
        &mut bytes,
        images_offset,
        mapping_address + 0x120,
        path_offset as u32,
    );
    let path = b"/usr/lib/libSymbolsRoot.dylib\0";
    bytes[path_offset..path_offset + path.len()].copy_from_slice(path);
    bytes
}

fn make_subcache_member_bytes(uuid: [u8; 16]) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x200];
    write_header(
        &mut bytes,
        MAGIC_ARM64E,
        uuid,
        0x80,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        [0u8; 16],
    );
    bytes
}

fn make_symbols_member_bytes(uuid: [u8; 16]) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x200];
    write_header(
        &mut bytes,
        MAGIC_ARM64,
        uuid,
        0x80,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        [0u8; 16],
    );
    bytes
}

#[test]
fn shared_cache_loader_rejects_non_cache_input() {
    let temp_dir = unique_temp_dir("shared-cache-non-cache");
    let path = temp_dir.join("plain.bin");
    fs::write(&path, b"hello world").expect("write non-cache payload");
    let error = load_shared_cache(&path).expect_err("expected non-cache rejection");
    assert!(matches!(error, MachoError::UnsupportedInputKind(_)));
}

#[test]
fn shared_cache_loader_parses_synthetic_single_cache() {
    let temp_dir = unique_temp_dir("shared-cache-single");
    let root_path = temp_dir.join("dyld_shared_cache_arm64e");
    fs::write(
        &root_path,
        make_single_cache_bytes(MAGIC_ARM64E, [0x11; 16]),
    )
    .expect("write synthetic single cache");

    let cache = load_shared_cache(&root_path).expect("load synthetic single cache");
    assert_eq!(cache.header().architecture, Architecture::Arm64e);
    assert_eq!(cache.members().len(), 1);
    assert_eq!(cache.mappings().len(), 1);
    assert_eq!(cache.images().len(), 1);
    assert_eq!(cache.images()[0].image_index, 0);
    assert_eq!(
        cache.images()[0].install_name,
        "/usr/lib/libSynthetic.dylib"
    );
    assert_eq!(
        cache.path(),
        fs::canonicalize(&root_path).expect("canonical root")
    );
}

#[test]
fn shared_cache_loader_derives_root_from_numbered_subcache_input() {
    let temp_dir = unique_temp_dir("shared-cache-subcache");
    let root_path = temp_dir.join("dyld_shared_cache_arm64e");
    let subcache_path = add_suffix(&root_path, ".1");
    fs::write(&root_path, make_root_with_subcache([0x33; 16], [0x33; 16]))
        .expect("write root cache");
    fs::write(&subcache_path, make_subcache_member_bytes([0x33; 16]))
        .expect("write subcache member");

    let cache = load_shared_cache(&subcache_path).expect("load via .1 path");
    assert_eq!(
        cache.path(),
        fs::canonicalize(&root_path).expect("canonical root")
    );
    assert_eq!(cache.members().len(), 2);
    assert_eq!(
        cache
            .members()
            .iter()
            .filter(|member| matches!(member.role, SharedCacheMemberRole::Subcache))
            .count(),
        1
    );
    assert!(
        cache
            .members()
            .iter()
            .any(|member| matches!(member.role, SharedCacheMemberRole::Subcache))
    );
}

#[test]
fn shared_cache_loader_derives_root_from_symbols_input() {
    let temp_dir = unique_temp_dir("shared-cache-symbols");
    let root_path = temp_dir.join("dyld_shared_cache_arm64");
    let symbols_path = add_suffix(&root_path, ".symbols");
    fs::write(&root_path, make_root_with_symbols([0x55; 16], [0x66; 16]))
        .expect("write root cache");
    fs::write(&symbols_path, make_symbols_member_bytes([0x66; 16])).expect("write symbols sidecar");

    let cache = load_shared_cache(&symbols_path).expect("load via .symbols path");
    assert_eq!(
        cache.path(),
        fs::canonicalize(&root_path).expect("canonical root")
    );
    assert!(cache.header().has_local_symbols);
    assert!(
        cache
            .members()
            .iter()
            .any(|member| matches!(member.role, SharedCacheMemberRole::Symbols))
    );
}

#[test]
fn shared_cache_loader_reports_missing_required_members() {
    let temp_dir = unique_temp_dir("shared-cache-missing");
    let root_path = temp_dir.join("dyld_shared_cache_arm64e");
    fs::write(&root_path, make_root_with_subcache([0x77; 16], [0x88; 16]))
        .expect("write root cache");

    let error = load_shared_cache(&root_path).expect_err("expected incomplete cache set");
    assert!(matches!(error, MachoError::IncompleteSharedCacheSet(_)));
}

#[test]
fn shared_cache_loader_rejects_unsupported_cache_architecture() {
    let temp_dir = unique_temp_dir("shared-cache-unsupported-arch");
    let root_path = temp_dir.join("dyld_shared_cache_x86_64");
    fs::write(
        &root_path,
        make_single_cache_bytes(MAGIC_X86_64, [0x99; 16]),
    )
    .expect("write x86_64 shared cache fixture");

    let error = load_shared_cache(&root_path).expect_err("expected unsupported architecture");
    assert!(matches!(
        error,
        MachoError::UnsupportedSharedCacheArchitecture(_)
    ));
}
