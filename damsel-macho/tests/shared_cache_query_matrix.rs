#[path = "../src/shared_cache_query.rs"]
mod shared_cache_query;

use std::sync::Arc;

use damsel_core::{
    Architecture, BinaryFormat, BinaryImage, DyldMetadata, Endianness, ExportFlags, ExportKind,
    ExportRecord, ObjcMetadata, Section, Segment, SliceInfo, Symbol, SymbolKind,
};
use shared_cache_query::{
    CacheLookupResult, CacheMappingRecord, CacheSymbolSource, ProjectedCacheImage,
    SharedCacheQueryEngine, SharedCacheQueryError,
};

fn synthetic_binary(
    label: &str,
    image_base: u64,
    image_size: u64,
    exports: &[(&str, u64)],
    locals: &[(&str, u64)],
) -> BinaryImage {
    let section = Section {
        segment_name: "__TEXT".to_string(),
        name: "__text".to_string(),
        address: image_base,
        size: image_size,
        file_offset: Some(0),
        file_size: image_size,
        kind: "text".to_string(),
        executable: true,
    };
    let segment = Segment {
        name: "__TEXT".to_string(),
        address: image_base,
        size: image_size,
        file_offset: 0,
        file_size: image_size,
        readable: true,
        writable: false,
        executable: true,
    };
    let symbols = locals
        .iter()
        .map(|(name, address)| Symbol {
            name: (*name).to_string(),
            address: *address,
            size: 0,
            kind: SymbolKind::Text,
            defined: true,
            global: true,
            weak: false,
            section: Some("__text".to_string()),
        })
        .collect::<Vec<_>>();
    let exported_symbols = exports
        .iter()
        .map(|(name, address)| ExportRecord {
            name: (*name).to_string(),
            address: Some(*address),
            raw_flags: format!("offset={address:#x}"),
            flags: ExportFlags::from_bits(0),
            kind: ExportKind::Regular,
            reexport_target: None,
            resolver_target: None,
        })
        .collect::<Vec<_>>();
    let dyld = DyldMetadata {
        exported_symbols,
        ..DyldMetadata::default()
    };
    let data = Arc::<[u8]>::from(vec![0u8; image_size as usize]);
    BinaryImage::from_memory_bytes(
        Some(label.to_string()),
        BinaryFormat::MachO,
        Architecture::Arm64,
        Endianness::Little,
        Some(image_base),
        None,
        SliceInfo {
            offset: 0,
            size: image_size,
            is_universal: false,
            cpu_subtype: 0,
        },
        vec![segment],
        vec![section],
        symbols,
        Vec::new(),
        Vec::new(),
        ObjcMetadata::default(),
        dyld,
        data,
    )
}

fn image(
    id: &str,
    index: usize,
    install_name: &str,
    base: u64,
    size: u64,
    exports: &[(&str, u64)],
    locals: &[(&str, u64)],
    local_symbols_available: bool,
) -> ProjectedCacheImage {
    ProjectedCacheImage::new(
        id.to_string(),
        index,
        install_name.to_string(),
        base,
        size,
        Some("dyld_shared_cache_arm64".to_string()),
        synthetic_binary(install_name, base, size, exports, locals),
        local_symbols_available,
    )
}

fn engine_fixture() -> SharedCacheQueryEngine {
    let mappings = vec![
        CacheMappingRecord {
            member_label: "dyld_shared_cache_arm64".to_string(),
            mapping_base_vmaddr: 0x1000_0000,
            mapping_size: 0x10000,
            mapping_file_offset: 0,
        },
        CacheMappingRecord {
            member_label: "dyld_shared_cache_arm64.1".to_string(),
            mapping_base_vmaddr: 0x2000_0000,
            mapping_size: 0x10000,
            mapping_file_offset: 0x20000,
        },
    ];
    let images = vec![
        image(
            "CACHEUUID:0",
            0,
            "/usr/lib/libalpha.dylib",
            0x1000_1000,
            0x1000,
            &[
                ("_alpha_export", 0x1000_1050),
                ("_shared_name", 0x1000_1080),
            ],
            &[("_alpha_local", 0x1000_1060), ("_local_only", 0x1000_1068)],
            true,
        ),
        image(
            "CACHEUUID:1",
            1,
            "/System/Library/Frameworks/Foo.framework/Foo",
            0x1000_3000,
            0x1000,
            &[("_foo_export", 0x1000_3040), ("_shared_name", 0x1000_30B0)],
            &[("_foo_local", 0x1000_3060)],
            true,
        ),
        image(
            "CACHEUUID:2",
            2,
            "/System/iOSSupport/usr/lib/libalpha.dylib",
            0x1000_5000,
            0x1000,
            &[("_ios_alpha_export", 0x1000_5030)],
            &[("_local_only", 0x1000_5040)],
            false,
        ),
    ];
    SharedCacheQueryEngine::new("CACHEUUID", mappings, images)
}

#[test]
fn project_image_prefers_id_then_install_name_and_reports_ambiguity() {
    let engine = engine_fixture();

    let by_id = engine.project_image("CACHEUUID:1").expect("select by id");
    assert_eq!(
        by_id.install_name,
        "/System/Library/Frameworks/Foo.framework/Foo"
    );

    let by_install = engine
        .project_image("/usr/lib/libalpha.dylib")
        .expect("select by install-name");
    assert_eq!(by_install.image_id, "CACHEUUID:0");

    let ambiguous = engine
        .project_image("libalpha.dylib")
        .expect_err("basename should be ambiguous");
    match ambiguous {
        SharedCacheQueryError::CacheImageAmbiguous {
            selector,
            candidates,
        } => {
            assert_eq!(selector, "libalpha.dylib");
            assert_eq!(candidates.len(), 2);
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    let missing = engine
        .project_image("missing-image")
        .expect_err("missing image selector");
    assert!(matches!(
        missing,
        SharedCacheQueryError::CacheImageNotFound(_)
    ));
}

#[test]
fn lookup_cache_vmaddr_returns_exact_nearest_and_mapping_only() {
    let engine = engine_fixture();

    let exact = engine
        .lookup_cache_vmaddr(0x1000_1050)
        .expect("exact export lookup");
    assert_eq!(exact.kind(), "exact_symbol");
    match exact {
        CacheLookupResult::ExactSymbol {
            mapping,
            image,
            symbol,
        } => {
            assert_eq!(mapping.member_file_offset, 0x1050);
            assert_eq!(image.image_id, "CACHEUUID:0");
            assert_eq!(symbol.symbol_name, "_alpha_export");
            assert_eq!(symbol.source, CacheSymbolSource::Export);
        }
        other => panic!("expected exact symbol result, got {other:?}"),
    }

    let nearest = engine
        .lookup_cache_vmaddr(0x1000_1074)
        .expect("nearest symbol lookup");
    assert_eq!(nearest.kind(), "nearest_symbol");
    match nearest {
        CacheLookupResult::NearestSymbol {
            symbol, distance, ..
        } => {
            assert_eq!(symbol.symbol_name, "_local_only");
            assert_eq!(symbol.source, CacheSymbolSource::Local);
            assert_eq!(distance, 0x0c);
        }
        other => panic!("expected nearest symbol result, got {other:?}"),
    }

    let mapping_only = engine
        .lookup_cache_vmaddr(0x1000_8000)
        .expect("mapped but outside image inventory");
    assert_eq!(mapping_only.kind(), "mapping_only");
    match mapping_only {
        CacheLookupResult::MappingOnly { image, .. } => {
            assert!(image.is_none());
        }
        other => panic!("expected mapping-only result, got {other:?}"),
    }
}

#[test]
fn lookup_cache_vmaddr_rejects_unmapped_addresses() {
    let engine = engine_fixture();
    let error = engine
        .lookup_cache_vmaddr(0x3000_0000)
        .expect_err("address should be unmapped");
    assert!(matches!(error, SharedCacheQueryError::AddressNotMapped(_)));
}

#[test]
fn resolve_exact_symbol_searches_exports_and_local_symbols_opportunistically() {
    let engine = engine_fixture();

    let shared = engine
        .resolve_exact_symbol("_shared_name", None, None)
        .expect("shared export across cache");
    assert_eq!(shared.len(), 2);
    assert!(
        shared
            .iter()
            .all(|entry| entry.source == CacheSymbolSource::Export)
    );

    let local = engine
        .resolve_exact_symbol("_local_only", None, None)
        .expect("local symbol should resolve only where available");
    assert_eq!(local.len(), 1);
    assert_eq!(local[0].image_id, "CACHEUUID:0");
    assert_eq!(local[0].source, CacheSymbolSource::Local);
}

#[test]
fn resolve_exact_symbol_honors_case_insensitive_image_filter_and_limit() {
    let engine = engine_fixture();
    let filtered = engine
        .resolve_exact_symbol("_shared_name", Some("foo.framework"), Some(1))
        .expect("filtered symbol resolution");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].image_id, "CACHEUUID:1");
    assert_eq!(filtered[0].symbol_name, "_shared_name");
}

#[test]
fn exports_for_image_filters_case_insensitive_substring() {
    let engine = engine_fixture();
    let exports = engine
        .exports_for_image("CACHEUUID:0", Some("ALPHA"))
        .expect("export listing");
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].symbol_name, "_alpha_export");
    assert_eq!(exports[0].source, CacheSymbolSource::Export);
}
