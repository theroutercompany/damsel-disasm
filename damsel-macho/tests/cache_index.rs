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

/// Unit I3: symbol rows are served from the index — exact and nearest
/// address queries, name queries, and the inventory-rebuild invalidation
/// contract.
#[test]
fn symbol_index_serves_and_invalidates() {
    use damsel_macho::{build_symbol_index, query_symbol_at, query_symbols_named, symbols_present};

    let dir = fixture_dir("symbols");
    let (root, cache) = fabricate_real_cache(&dir);
    build_index(&cache, &root).expect("inventory index");

    let index_path = damsel_macho::index_path_for(&root);
    assert!(!symbols_present(&index_path), "no symbols yet");

    // Fabricated caches carry no extractable symbols; seed rows directly
    // through the same builder contract (rows come from the session).
    let stats = build_symbol_index(
        &root,
        |image_index| {
            Ok(if image_index == 0 {
                vec![
                    (
                        "_objc_msgSend".to_string(),
                        0x180405000,
                        "export".to_string(),
                    ),
                    ("_malloc".to_string(), 0x180406000, "export".to_string()),
                ]
            } else {
                Vec::new()
            })
        },
        1,
    )
    .expect("build symbol index");
    assert_eq!(stats.symbols, 2);
    assert_eq!(stats.images_indexed, 1);
    assert!(symbols_present(&index_path));

    // Exact hit.
    let hit = query_symbol_at(&index_path, 0x180405000)
        .expect("query")
        .expect("exact row");
    assert_eq!(hit.name, "_objc_msgSend");
    assert!(hit.is_exact(0x180405000));

    // Nearest symbol at or below an interior address.
    let nearest = query_symbol_at(&index_path, 0x180405fff)
        .expect("query")
        .expect("nearest row");
    assert_eq!(nearest.name, "_objc_msgSend");
    assert!(!nearest.is_exact(0x180405fff));

    // Below every row → None.
    assert!(
        query_symbol_at(&index_path, 0x1000)
            .expect("query")
            .is_none()
    );

    // Name query across images.
    let hits = query_symbols_named(&index_path, "_malloc", None).expect("named");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].cache_vmaddr, 0x180406000);
    assert!(
        query_symbols_named(&index_path, "_absent", None)
            .expect("named")
            .is_empty()
    );

    // Inventory rebuild drops symbol rows with it.
    build_index(&cache, &root).expect("rebuild inventory");
    assert!(!symbols_present(&index_path), "symbols dropped on rebuild");
}

/// Unit I4: projection store round trip, digest staleness, and the
/// env bypass.
#[test]
fn projection_store_round_trip_and_staleness() {
    let dir = fixture_dir("projections");
    let (root, cache) = fabricate_real_cache(&dir);
    let projection_dir = damsel_macho::projection_dir_for(&root);
    let digest = damsel_macho::projection_digest(&cache);

    // Nothing cached yet.
    assert!(damsel_macho::read_projection_cache_for_test(&projection_dir, digest, 0).is_none());

    // Write + read back.
    let bytes = vec![0xceu8, 1, 2, 0xfe, 0xed, 0xfa, 0xce, 0xfa];
    damsel_macho::write_projection(&projection_dir, digest, 0, &bytes);
    assert_eq!(
        damsel_macho::read_projection_cache_for_test(&projection_dir, digest, 0),
        Some(bytes)
    );

    // A different digest (member growth) refuses the same files.
    let other_digest = digest ^ 0x1;
    assert!(
        damsel_macho::read_projection_cache_for_test(&projection_dir, other_digest, 0).is_none()
    );

    // The env bypass skips warm reads.
    unsafe {
        std::env::set_var("DAMSEL_NO_PROJECTION_CACHE", "1");
    }
    let none = damsel_macho::read_projection_cache_for_test(&projection_dir, digest, 0).is_none();
    unsafe {
        std::env::remove_var("DAMSEL_NO_PROJECTION_CACHE");
    }
    assert!(none, "env bypass active");
}
