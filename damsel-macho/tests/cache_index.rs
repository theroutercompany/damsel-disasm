//! Cache inventory index tests (plan feature 2, slice 1): encode/decode
//! round trip and freshness rules against a fabricated real-kind cache.

use damsel_core::{
    Architecture, CacheImageId, CacheImageRecord, SharedCache, SharedCacheHeader,
    SharedCacheMapping, SharedCacheMember, SharedCacheMemberRole,
};
use damsel_macho::{build_index, try_load_fresh};
use std::path::PathBuf;

fn fixture_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("damsel-cache-index-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fabricate_real_cache(dir: &std::path::Path) -> (PathBuf, SharedCache) {
    let root = dir.join("dyld_shared_cache_arm64e");
    let sub = dir.join("dyld_shared_cache_arm64e.1");
    std::fs::write(&root, vec![0u8; 128]).unwrap();
    std::fs::write(&sub, vec![0u8; 64]).unwrap();

    let cache = SharedCache::from_file_inventory(
        root.clone(),
        SharedCacheHeader {
            cache_uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            architecture: Architecture::Arm64e,
            mapping_count: 1,
            image_count: 1,
            base_address: Some(0x180000000),
            has_local_symbols: false,
        },
        vec![
            SharedCacheMember {
                role: SharedCacheMemberRole::Root,
                path: root.clone(),
                file_size: 128,
                suffix: None,
                uuid: Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string()),
            },
            SharedCacheMember {
                role: SharedCacheMemberRole::Subcache,
                path: sub.clone(),
                file_size: 64,
                suffix: Some(".1".to_string()),
                uuid: Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string()),
            },
        ],
        vec![SharedCacheMapping {
            member_index: 1,
            cache_vmaddr: 0x180000000,
            size: 64,
            member_file_offset: 0,
        }],
        vec![CacheImageRecord {
            id: CacheImageId::new("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", 0),
            image_index: 0,
            install_name: "/usr/lib/libSystem.B.dylib".to_string(),
            basename: "libSystem.B.dylib".to_string(),
            image_base_vmaddr: 0x180000000,
            image_size: 0x10000,
            member_index: 1,
        }],
    );
    (root, cache)
}

#[test]
fn index_round_trips_and_stays_fresh_until_members_change() {
    let dir = fixture_dir("roundtrip");
    let (root, cache) = fabricate_real_cache(&dir);
    // The tempdir is writable, so the index lands beside the cache
    // (no fallback root involved).
    assert!(try_load_fresh(&root).is_none(), "no index yet");
    let stats = build_index(&cache, &root).expect("build index");
    assert_eq!(stats.images, 1);
    assert_eq!(stats.members, 2);
    assert_eq!(stats.mappings, 1);

    let loaded = try_load_fresh(&root).expect("fresh index loads");
    assert_eq!(loaded.header().cache_uuid, cache.header().cache_uuid);
    assert_eq!(loaded.images(), cache.images());
    assert_eq!(loaded.mappings(), cache.mappings());
    assert_eq!(loaded.members().len(), cache.members().len());
    assert_eq!(loaded.header().image_count, 1);

    // Member growth invalidates freshness (stat mismatch → full parse).
    let sub = dir.join("dyld_shared_cache_arm64e.1");
    std::fs::write(&sub, vec![0u8; 96]).unwrap();
    assert!(try_load_fresh(&root).is_none(), "stale index refused");
}

#[test]
fn index_refuses_a_different_root_path() {
    let dir = fixture_dir("rootpath");
    let (root, cache) = fabricate_real_cache(&dir);
    build_index(&cache, &root).expect("build index");

    // Same bytes, different path: the index is root-path scoped.
    let copy = dir.join("renamed_cache");
    std::fs::copy(&root, &copy).unwrap();
    assert!(try_load_fresh(&copy).is_none(), "foreign root refused");
}
