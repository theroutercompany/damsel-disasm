use damsel_core::{
    Architecture, BinaryFormat, CacheDependentRecord, CacheImageDependencyRecord, CacheImageId,
    CacheImageRecord, CacheLookupResult, CacheMappingContext, CacheReexportRecord,
    CacheSymbolImporterRecord, CacheSymbolProviderKind, CacheSymbolProviderRecord,
    CacheSymbolSource, Endianness, ImportBindingKind, ImportBindingSource, ProjectedBinaryImage,
    ProjectedImageProvenance, SharedCache, SharedCacheHeader, SharedCacheMapping,
    SharedCacheMember, SharedCacheMemberRole, SharedCacheSource, SharedCacheValidationError,
    SliceInfo, SymbolicationMatch,
};
use std::path::PathBuf;
use std::ptr;
use std::sync::Arc;

fn sample_header() -> SharedCacheHeader {
    SharedCacheHeader {
        cache_uuid: "CACHE-UUID-1234".to_string(),
        architecture: Architecture::Arm64,
        mapping_count: 2,
        image_count: 3,
        base_address: Some(0x1800_0000_0),
        has_local_symbols: false,
    }
}

fn sample_members() -> Vec<SharedCacheMember> {
    vec![
        SharedCacheMember {
            role: SharedCacheMemberRole::Root,
            path: PathBuf::from("/tmp/dyld_shared_cache_arm64"),
            file_size: 0x10_000,
            suffix: None,
            uuid: Some("CACHE-UUID-1234".to_string()),
        },
        SharedCacheMember {
            role: SharedCacheMemberRole::Subcache,
            path: PathBuf::from("/tmp/dyld_shared_cache_arm64.1"),
            file_size: 0x8_000,
            suffix: Some(".1".to_string()),
            uuid: Some("CACHE-UUID-1234".to_string()),
        },
    ]
}

fn sample_mappings() -> Vec<SharedCacheMapping> {
    vec![
        SharedCacheMapping {
            member_index: 0,
            cache_vmaddr: 0x1800_0000_0,
            size: 0x4000,
            member_file_offset: 0,
        },
        SharedCacheMapping {
            member_index: 1,
            cache_vmaddr: 0x1800_1000_0,
            size: 0x1000,
            member_file_offset: 0x2000,
        },
    ]
}

fn sample_images(header: &SharedCacheHeader) -> Vec<CacheImageRecord> {
    vec![
        CacheImageRecord {
            id: CacheImageId::new(header.cache_uuid.clone(), 7),
            image_index: 7,
            install_name: "/usr/lib/libA.dylib".to_string(),
            basename: "libA.dylib".to_string(),
            image_base_vmaddr: 0x1800_0010_0,
            image_size: 0x200,
            member_index: 0,
        },
        CacheImageRecord {
            id: CacheImageId::new(header.cache_uuid.clone(), 2),
            image_index: 2,
            install_name: "/usr/lib/libB.dylib".to_string(),
            basename: "libB.dylib".to_string(),
            image_base_vmaddr: 0x1800_0050_0,
            image_size: 0x300,
            member_index: 0,
        },
        CacheImageRecord {
            id: CacheImageId::new(header.cache_uuid.clone(), 9),
            image_index: 9,
            install_name: "/System/Library/PrivateFrameworks/Another.framework/libA.dylib"
                .to_string(),
            basename: "libA.dylib".to_string(),
            image_base_vmaddr: 0x1800_1010_0,
            image_size: 0x180,
            member_index: 1,
        },
    ]
}

fn sample_cache() -> SharedCache {
    let header = sample_header();
    SharedCache::builder(
        SharedCacheSource::File(PathBuf::from("/tmp/dyld_shared_cache_arm64")),
        PathBuf::from("/tmp/dyld_shared_cache_arm64"),
        header.clone(),
    )
    .with_members(sample_members())
    .with_mappings(sample_mappings())
    .with_images(sample_images(&header))
    .build()
    .expect("build sample shared cache")
}

#[test]
fn cache_image_id_generation_and_parsing_is_stable() {
    let id = CacheImageId::new("ABCDEF", 42);
    assert_eq!(id.as_str(), "ABCDEF:42");
    assert_eq!(id.cache_uuid(), "ABCDEF");
    assert_eq!(id.image_index(), 42);

    let parsed = CacheImageId::parse("ABCDEF:42").expect("parse cache image id");
    assert_eq!(parsed, id);
    assert!(CacheImageId::parse("ABCDEF").is_none());
    assert!(CacheImageId::parse("ABCDEF:xyz").is_none());
}

#[test]
fn builder_rejects_invalid_inventories() {
    let header = sample_header();
    let source = SharedCacheSource::File(PathBuf::from("/tmp/dyld_shared_cache_arm64"));
    let path = PathBuf::from("/tmp/dyld_shared_cache_arm64");

    let missing_members = SharedCache::builder(source.clone(), path.clone(), header.clone())
        .with_mappings(sample_mappings())
        .with_images(sample_images(&header))
        .build();
    assert!(matches!(
        missing_members,
        Err(SharedCacheValidationError::MissingMembers)
    ));

    let missing_mappings = SharedCache::builder(source.clone(), path.clone(), header.clone())
        .with_members(sample_members())
        .with_images(sample_images(&header))
        .build();
    assert!(matches!(
        missing_mappings,
        Err(SharedCacheValidationError::MissingMappings)
    ));

    let missing_images = SharedCache::builder(source.clone(), path.clone(), header.clone())
        .with_members(sample_members())
        .with_mappings(sample_mappings())
        .build();
    assert!(matches!(
        missing_images,
        Err(SharedCacheValidationError::MissingImages)
    ));

    let mut bad_header = header.clone();
    bad_header.mapping_count = 1;
    let mapping_mismatch = SharedCache::builder(source.clone(), path.clone(), bad_header)
        .with_members(sample_members())
        .with_mappings(sample_mappings())
        .with_images(sample_images(&header))
        .build();
    assert!(matches!(
        mapping_mismatch,
        Err(SharedCacheValidationError::HeaderMappingCountMismatch)
    ));

    let mut images = sample_images(&header);
    images[0].id = CacheImageId::new("WRONG-UUID", 7);
    let id_mismatch = SharedCache::builder(source.clone(), path.clone(), header.clone())
        .with_members(sample_members())
        .with_mappings(sample_mappings())
        .with_images(images)
        .build();
    assert!(matches!(
        id_mismatch,
        Err(SharedCacheValidationError::CacheImageIdMismatch)
    ));

    let mut bad_mappings = sample_mappings();
    bad_mappings[0].member_index = 9;
    let mapping_member_out_of_range = SharedCache::builder(source, path, header.clone())
        .with_members(sample_members())
        .with_mappings(bad_mappings)
        .with_images(sample_images(&header))
        .build();
    assert!(matches!(
        mapping_member_out_of_range,
        Err(SharedCacheValidationError::MappingMemberOutOfRange)
    ));
}

#[test]
fn cached_indexes_are_memoized_and_deterministic() {
    let cache = sample_cache();

    let image_id_index_a = cache.image_id_index_cached();
    let image_id_index_b = cache.image_id_index_cached();
    assert!(ptr::eq(image_id_index_a, image_id_index_b));

    let install_name_index_a = cache.image_install_name_index_cached();
    let install_name_index_b = cache.image_install_name_index_cached();
    assert!(ptr::eq(install_name_index_a, install_name_index_b));

    let basename_index_a = cache.image_basename_index_cached();
    let basename_index_b = cache.image_basename_index_cached();
    assert!(ptr::eq(basename_index_a, basename_index_b));

    let mapping_index_a = cache.mapping_start_index_cached();
    let mapping_index_b = cache.mapping_start_index_cached();
    assert!(ptr::eq(mapping_index_a, mapping_index_b));

    let id_keys: Vec<String> = cache
        .image_id_index_cached()
        .keys()
        .map(|id| id.as_str().to_string())
        .collect();
    assert_eq!(
        id_keys,
        vec![
            "CACHE-UUID-1234:2".to_string(),
            "CACHE-UUID-1234:7".to_string(),
            "CACHE-UUID-1234:9".to_string(),
        ]
    );

    let install_name_keys: Vec<String> = cache
        .image_install_name_index_cached()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        install_name_keys,
        vec![
            "/System/Library/PrivateFrameworks/Another.framework/libA.dylib".to_string(),
            "/usr/lib/libA.dylib".to_string(),
            "/usr/lib/libB.dylib".to_string(),
        ]
    );

    let basename_keys: Vec<String> = cache
        .image_basename_index_cached()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        basename_keys,
        vec!["libA.dylib".to_string(), "libB.dylib".to_string()]
    );
}

#[test]
fn mapping_and_image_lookup_helpers_are_correct() {
    let cache = sample_cache();

    let by_id = cache
        .image_by_id_str("CACHE-UUID-1234:7")
        .expect("lookup by id");
    assert_eq!(by_id.install_name, "/usr/lib/libA.dylib");

    let by_install_name = cache
        .image_by_install_name("/usr/lib/libB.dylib")
        .expect("lookup by install name");
    assert_eq!(by_install_name.id.as_str(), "CACHE-UUID-1234:2");

    let basename_matches = cache.images_by_basename("libA.dylib");
    assert_eq!(basename_matches.len(), 2);
    assert!(cache.image_by_basename_unique("libA.dylib").is_none());
    let unique = cache
        .image_by_basename_unique("libB.dylib")
        .expect("unique basename");
    assert_eq!(unique.id.as_str(), "CACHE-UUID-1234:2");

    let mapped = cache
        .mapping_for_vm_address(0x1800_0002_0)
        .expect("mapped address");
    assert_eq!(mapped.member_index, 0);
    assert_eq!(
        mapped.member_file_offset_for_vmaddr(0x1800_0002_0),
        Some(0x20)
    );

    let mapped_second = cache
        .mapping_for_vm_address(0x1800_1000_1)
        .expect("mapped address in second member");
    assert_eq!(mapped_second.member_index, 1);

    let image = cache
        .image_for_vm_address(0x1800_0015_0)
        .expect("image for vmaddr");
    assert_eq!(image.id.as_str(), "CACHE-UUID-1234:7");
    assert_eq!(
        cache.image_offset_for_vm_address(image, 0x1800_0015_0),
        Some(0x50)
    );

    assert!(cache.mapping_for_vm_address(0x1900_0000_0).is_none());
    assert!(cache.image_for_vm_address(0x1800_0000_8).is_none());
}

#[test]
fn projected_image_and_lookup_types_are_constructible() {
    let cache = sample_cache();
    let image = cache.image_by_id_str("CACHE-UUID-1234:7").expect("image");
    let provenance = ProjectedImageProvenance {
        cache_uuid: cache.header().cache_uuid.clone(),
        image_id: image.id.clone(),
        install_name: image.install_name.clone(),
        basename: image.basename.clone(),
        image_base_vmaddr: image.image_base_vmaddr,
        member_name: "dyld_shared_cache_arm64".to_string(),
        local_symbols_available: false,
    };
    let projected = ProjectedBinaryImage {
        provenance: provenance.clone(),
        image: damsel_core::BinaryImage::from_memory_bytes(
            Some("projected".to_string()),
            BinaryFormat::MachO,
            Architecture::Arm64,
            Endianness::Little,
            Some(image.image_base_vmaddr),
            None,
            SliceInfo {
                offset: 0,
                size: 4,
                is_universal: false,
                cpu_subtype: 0,
            },
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            damsel_core::ObjcMetadata::default(),
            damsel_core::DyldMetadata::default(),
            Arc::<[u8]>::from(vec![0u8; 4]),
        ),
    };
    assert_eq!(projected.provenance.image_id.as_str(), "CACHE-UUID-1234:7");

    let mapping = CacheMappingContext {
        member_name: "dyld_shared_cache_arm64".to_string(),
        mapping_base_vmaddr: 0x1800_0000_0,
        mapping_size: 0x4000,
        member_file_offset: 0x120,
    };
    let symbol = SymbolicationMatch {
        image_id: image.id.clone(),
        image_install_name: image.install_name.clone(),
        symbol_name: "_demo".to_string(),
        symbol_vmaddr: image.image_base_vmaddr + 0x10,
        cache_vmaddr: image.image_base_vmaddr + 0x10,
        image_base_vmaddr: image.image_base_vmaddr,
        image_offset: 0x10,
        member_name: "dyld_shared_cache_arm64".to_string(),
        member_file_offset: Some(0x120),
        symbol_source: CacheSymbolSource::Export,
        exact: true,
    };
    let result = CacheLookupResult::ExactSymbol {
        cache_vmaddr: image.image_base_vmaddr + 0x10,
        mapping,
        image: provenance,
        symbol,
    };
    match result {
        CacheLookupResult::ExactSymbol {
            cache_vmaddr,
            image,
            symbol,
            ..
        } => {
            assert_eq!(cache_vmaddr, 0x1800_0011_0);
            assert_eq!(image.image_id.as_str(), "CACHE-UUID-1234:7");
            assert_eq!(symbol.symbol_source, CacheSymbolSource::Export);
        }
        other => panic!("unexpected result variant: {other:?}"),
    }
}

#[test]
fn cache_relation_record_types_are_constructible() {
    let cache = sample_cache();
    let image_a = cache.image_by_id_str("CACHE-UUID-1234:7").expect("image a");
    let image_b = cache.image_by_id_str("CACHE-UUID-1234:2").expect("image b");
    let provenance_a = ProjectedImageProvenance {
        cache_uuid: cache.header().cache_uuid.clone(),
        image_id: image_a.id.clone(),
        install_name: image_a.install_name.clone(),
        basename: image_a.basename.clone(),
        image_base_vmaddr: image_a.image_base_vmaddr,
        member_name: "dyld_shared_cache_arm64".to_string(),
        local_symbols_available: false,
    };
    let provenance_b = ProjectedImageProvenance {
        cache_uuid: cache.header().cache_uuid.clone(),
        image_id: image_b.id.clone(),
        install_name: image_b.install_name.clone(),
        basename: image_b.basename.clone(),
        image_base_vmaddr: image_b.image_base_vmaddr,
        member_name: "dyld_shared_cache_arm64".to_string(),
        local_symbols_available: false,
    };

    let dependency = CacheImageDependencyRecord {
        source_image: provenance_a.clone(),
        target_dylib_install_name: provenance_b.install_name.clone(),
        target_image: Some(provenance_b.clone()),
        within_cache: true,
        reference_count: 3,
    };
    assert_eq!(dependency.target_dylib_install_name, "/usr/lib/libB.dylib");

    let dependent = CacheDependentRecord {
        dependent_image: provenance_a.clone(),
        dependency_count: 3,
    };
    assert_eq!(
        dependent.dependent_image.image_id.as_str(),
        "CACHE-UUID-1234:7"
    );

    let reexport = CacheReexportRecord {
        source_image: provenance_a.clone(),
        export_name: "_alias".to_string(),
        target_dylib: provenance_b.install_name.clone(),
        target_symbol: Some("_target".to_string()),
        resolved_target_image: Some(provenance_b.clone()),
    };
    assert_eq!(reexport.target_symbol.as_deref(), Some("_target"));

    let provider = CacheSymbolProviderRecord {
        provider_image: provenance_a.clone(),
        symbol_name: "_alias".to_string(),
        provider_kind: CacheSymbolProviderKind::Reexport,
        target_dylib: Some(provenance_b.install_name.clone()),
        target_symbol: Some("_target".to_string()),
        resolved_target_image: Some(provenance_b.clone()),
    };
    assert_eq!(provider.provider_kind, CacheSymbolProviderKind::Reexport);

    let importer = CacheSymbolImporterRecord {
        importer_image: provenance_a,
        symbol_name: "_target".to_string(),
        dylib_name: provenance_b.install_name.clone(),
        import_binding_kind: Some(ImportBindingKind::NonLazy),
        import_binding_source: Some(ImportBindingSource::IndirectSymbol),
        resolved_provider_image: Some(provenance_b),
    };
    assert_eq!(
        importer.import_binding_source,
        Some(ImportBindingSource::IndirectSymbol)
    );
}
