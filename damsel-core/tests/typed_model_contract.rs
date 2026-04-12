use damsel_core::{
    Annotation, Architecture, BinaryFormat, BinaryImage, BinaryImageValidationError,
    DisassemblyLimit, DisassemblyOptions, DisassemblyRequest, DisassemblyRequestV2,
    DisassemblyTarget, Endianness, ImportBindingKind, ImportBindingRecord, ImportBindingSource,
    ObjcNameSource, ObjcSelectorSource, Reference, Relocation, SliceDescriptor, SliceInfo,
    StubKind,
};
use std::ptr;
use std::sync::Arc;

fn sample_image() -> BinaryImage {
    let bytes: Arc<[u8]> = vec![0u8; 64].into();
    BinaryImage::from_memory_bytes(
        Some("typed-contract".to_string()),
        BinaryFormat::MachO,
        Architecture::Arm64,
        Endianness::Little,
        None,
        Some(damsel_core::Platform::unknown("xros")),
        SliceInfo {
            offset: 0,
            size: 64,
            is_universal: true,
            cpu_subtype: 2,
        },
        vec![],
        vec![damsel_core::Section {
            segment_name: "__TEXT".to_string(),
            name: "__text".to_string(),
            address: 0x1000,
            size: 32,
            file_offset: Some(0),
            file_size: 32,
            kind: "Text".to_string(),
            executable: true,
        }],
        vec![damsel_core::Symbol {
            name: "_main".to_string(),
            address: 0x1000,
            size: 12,
            kind: damsel_core::SymbolKind::Text,
            defined: true,
            global: true,
            weak: false,
            section: Some("__text".to_string()),
        }],
        vec![damsel_core::Import {
            name: "_puts".to_string(),
            dylib: "/usr/lib/libSystem.B.dylib".to_string(),
            address: Some(0x2000),
            offset: Some(0),
            addend: 0,
            is_lazy: false,
            is_weak: false,
        }],
        vec![Relocation {
            section: "__TEXT:__text".to_string(),
            address: 0x1004,
            size: 8,
            kind: "Absolute".to_string(),
            encoding: "Generic".to_string(),
            target: "absolute".to_string(),
            addend: 0,
        }],
        damsel_core::ObjcMetadata::default(),
        damsel_core::DyldMetadata::default(),
        bytes,
    )
}

#[test]
fn preserves_unknown_platform_payload() {
    let image = sample_image();
    assert_eq!(image.platform_raw_identifier(), Some("xros"));
}

#[test]
fn exposes_slice_descriptors_and_selection() {
    let image = sample_image();
    assert_eq!(image.available_slices().len(), 1);
    let selected = image.selected_slice_descriptor().expect("selected slice");
    assert!(selected.selected);
    assert!(selected.contains_file_offset(10));
    assert!(!selected.contains_file_offset(128));

    let replacement = SliceDescriptor {
        offset: 0,
        size: 64,
        is_universal: true,
        cpu_subtype: 2,
        architecture: Architecture::Arm64e,
        selected: true,
    };
    let image = image.with_available_slices(vec![replacement.clone()]);
    assert_eq!(image.available_slices(), &[replacement]);
}

#[test]
fn caches_lookup_indexes() {
    let image = sample_image();
    let section_a = image.section_name_index_cached();
    let section_b = image.section_name_index_cached();
    assert!(ptr::eq(section_a, section_b));

    let symbol_a = image.symbol_name_index_cached();
    let symbol_b = image.symbol_name_index_cached();
    assert!(ptr::eq(symbol_a, symbol_b));

    let import_a = image.import_name_index_cached();
    let import_b = image.import_name_index_cached();
    assert!(ptr::eq(import_a, import_b));

    let reloc_a = image.relocation_address_index_cached();
    let reloc_b = image.relocation_address_index_cached();
    assert!(ptr::eq(reloc_a, reloc_b));
}

#[test]
fn disassembly_v2_range_helpers_work() {
    let legacy = DisassemblyRequest::legacy(DisassemblyTarget::Address(0x1000), Some(8));
    let ranged = legacy.to_v2().with_range(Some(0x1000..0x1010));
    assert!(ranged.has_valid_range());
    assert!(ranged.range_contains(0x1008));
    assert!(!ranged.range_contains(0x2000));

    let invalid = ranged.with_range(Some(0x1010..0x1010));
    assert!(!invalid.has_valid_range());
}

#[test]
fn evidence_variants_are_constructible() {
    let binding = ImportBindingRecord {
        dylib: "/usr/lib/libSystem.B.dylib".to_string(),
        name: "_puts".to_string(),
        address: Some(0x2000),
        offset: Some(0x10),
        addend: 0,
        ordinal: Some(1),
        symbol_index: Some(2),
        binding_kind: ImportBindingKind::ChainedFixup,
        source: ImportBindingSource::ChainedFixup,
        is_weak: false,
    };
    let stub = damsel_core::StubEntry {
        stub_address: 0x3000,
        section: Some("__TEXT:__stubs".to_string()),
        pointer_section: Some("__DATA_CONST:__got".to_string()),
        pointer_address: Some(0x3010),
        helper_address: None,
        binding_ordinal: Some(1),
        stub_kind: StubKind::NonLazy,
        dylib: Some(binding.dylib.clone()),
        name: Some(binding.name.clone()),
        source: ImportBindingSource::Stub,
    };
    let relocation = Relocation {
        section: "__TEXT:__text".to_string(),
        address: 0x1004,
        size: 8,
        kind: "Absolute".to_string(),
        encoding: "Generic".to_string(),
        target: "absolute".to_string(),
        addend: 0,
    };

    let reference_binding = Reference::from_binding(&binding);
    let reference_stub = Reference::from_stub(&stub);
    let annotation_binding = Annotation::import_binding_evidence(&binding);
    let annotation_reloc = Annotation::relocation_evidence(&relocation);

    assert!(matches!(reference_binding, Reference::ImportBinding { .. }));
    assert!(matches!(reference_stub, Reference::Stub { .. }));
    assert!(matches!(
        annotation_binding,
        Annotation::ImportBindingEvidence { .. }
    ));
    assert!(matches!(
        annotation_reloc,
        Annotation::RelocationEvidence { .. }
    ));
}

#[test]
fn helper_indexes_are_queryable() {
    let binding = ImportBindingRecord {
        dylib: "/usr/lib/libSystem.B.dylib".to_string(),
        name: "_puts".to_string(),
        address: Some(0x3010),
        offset: Some(0x10),
        addend: 0,
        ordinal: Some(7),
        symbol_index: Some(2),
        binding_kind: ImportBindingKind::Lazy,
        source: ImportBindingSource::IndirectSymbol,
        is_weak: false,
    };
    let stub = damsel_core::StubEntry {
        stub_address: 0x3000,
        section: Some("__TEXT:__stubs".to_string()),
        pointer_section: Some("__DATA_CONST:__la_symbol_ptr".to_string()),
        pointer_address: Some(0x3010),
        helper_address: Some(0x3330),
        binding_ordinal: Some(7),
        stub_kind: StubKind::Lazy,
        dylib: Some("/usr/lib/libSystem.B.dylib".to_string()),
        name: Some("_puts".to_string()),
        source: ImportBindingSource::Stub,
    };
    let helper = damsel_core::StubHelperEntry {
        helper_address: 0x3330,
        target_stub: Some(0x3000),
        stub_section: Some("__TEXT:__stubs".to_string()),
        pointer_address: Some(0x3010),
        pointer_section: Some("__DATA_CONST:__la_symbol_ptr".to_string()),
        binding_ordinal: Some(7),
        dylib: Some("/usr/lib/libSystem.B.dylib".to_string()),
        name: Some("_puts".to_string()),
    };
    let helper_other = damsel_core::StubHelperEntry {
        helper_address: 0x3340,
        target_stub: Some(0x3010),
        stub_section: Some("__TEXT:__stubs".to_string()),
        pointer_address: Some(0x3020),
        pointer_section: Some("__DATA_CONST:__la_symbol_ptr".to_string()),
        binding_ordinal: Some(9),
        dylib: Some("/usr/lib/libobjc.A.dylib".to_string()),
        name: Some("_puts".to_string()),
    };
    let base = sample_image();
    let bytes: Arc<[u8]> = base.slice_bytes().to_vec().into();
    let dyld = damsel_core::DyldMetadata {
        import_bindings: vec![binding.clone()],
        stubs: vec![stub.clone()],
        stub_helpers: vec![helper.clone(), helper_other.clone()],
        ..base.dyld().clone()
    };
    let image = BinaryImage::builder(
        base.source().clone(),
        base.path().to_path_buf(),
        base.format(),
        base.architecture(),
        base.endianness(),
        base.entry_point(),
        base.platform().cloned(),
        base.selected_slice().clone(),
        base.segments().to_vec(),
        base.sections().to_vec(),
        base.symbols().to_vec(),
        base.imports().to_vec(),
        base.relocations().to_vec(),
        base.objc().clone(),
        dyld,
        bytes,
    )
    .with_available_slices(base.available_slices().to_vec())
    .build()
    .expect("build helper image");

    assert_eq!(image.dyld().helper_for_address(0x3330), Some(&helper));
    assert_eq!(image.dyld().helper_for_stub_address(0x3000), Some(&helper));
    assert_eq!(
        image.dyld().helper_for_pointer_address(0x3010),
        Some(&helper)
    );
    assert_eq!(image.dyld().helper_for_binding_ordinal(7), Some(&helper));
    assert_eq!(image.dyld().binding_for_ordinal(7), Some(&binding));
    assert_eq!(image.dyld().binding_for_symbol_index(2), Some(&binding));
    assert_eq!(image.dyld().stub_for_pointer_address(0x3010), Some(&stub));
    assert_eq!(image.dyld().stub_for_helper_address(0x3330), Some(&stub));
    let symbol_helpers = image
        .dyld()
        .helpers_for_symbol("/usr/lib/libSystem.B.dylib", "_puts")
        .collect::<Vec<_>>();
    assert_eq!(symbol_helpers, vec![&helper]);
}

#[test]
fn objc_category_provenance_is_constructible() {
    let category = damsel_core::ObjcCategoryRecord {
        pointer: 0x5000,
        name: Some("Excited".to_string()),
        name_source: ObjcNameSource::Runtime,
        class_pointer: Some(0x5100),
        class_name: Some("Greeter".to_string()),
        class_name_source: ObjcNameSource::PointerTable,
        methods: Vec::new(),
        class_methods: Vec::new(),
        properties: Vec::new(),
        adopted_protocols: Vec::new(),
    };

    assert_eq!(category.name_source, ObjcNameSource::Runtime);
    assert_eq!(category.class_name_source, ObjcNameSource::PointerTable);
}

#[test]
fn objc_records_are_queryable_by_pointer_and_selector_source() {
    let method = damsel_core::ObjcMethodRecord {
        owner_pointer: 0x4000,
        owner_kind: damsel_core::ObjcMethodOwnerKind::Protocol,
        is_class_method: false,
        selector: Some("greeting".to_string()),
        selector_source: ObjcSelectorSource::Relative,
        implementation: Some(0x2000),
        type_encoding: Some("@16@0:8".to_string()),
    };
    let metadata = damsel_core::ObjcMetadata {
        protocols: vec![damsel_core::ObjcProtocolRecord {
            pointer: 0x4000,
            name: Some("GreetingProviding".to_string()),
            name_source: ObjcNameSource::Runtime,
            required_instance_methods: vec![method.clone()],
            required_class_methods: Vec::new(),
            optional_instance_methods: Vec::new(),
            optional_class_methods: Vec::new(),
            properties: Vec::new(),
        }],
        categories: vec![damsel_core::ObjcCategoryRecord {
            pointer: 0x5000,
            name: Some("Excited".to_string()),
            name_source: ObjcNameSource::LegacyPool,
            class_pointer: Some(0x6000),
            class_name: Some("Greeter".to_string()),
            class_name_source: ObjcNameSource::PointerTable,
            methods: Vec::new(),
            class_methods: Vec::new(),
            properties: Vec::new(),
            adopted_protocols: Vec::new(),
        }],
        ..damsel_core::ObjcMetadata::default()
    };

    assert_eq!(
        metadata
            .protocol_by_pointer(0x4000)
            .and_then(|record| record.name.as_deref()),
        Some("GreetingProviding")
    );
    assert_eq!(
        metadata
            .category_by_pointer(0x5000)
            .and_then(|record| record.name.as_deref()),
        Some("Excited")
    );
    assert_eq!(
        metadata
            .methods_with_selector_source(ObjcSelectorSource::Relative)
            .count(),
        1
    );
}

#[test]
fn v2_instruction_limit_helper_matches_legacy_adapter() {
    let image = sample_image();
    let legacy = DisassemblyRequest::legacy(DisassemblyTarget::Address(0x1000), Some(8));
    let v2 = DisassemblyRequestV2 {
        target: DisassemblyTarget::Address(0x1000),
        range: None,
        limit: DisassemblyLimit::Instructions(8),
        options: DisassemblyOptions::default(),
    };

    assert_eq!(
        image.effective_instruction_limit(&legacy),
        image.effective_instruction_limit_v2(&v2)
    );
}

#[test]
fn v2_instruction_limit_helper_handles_byte_and_unlimited_limits() {
    let image = sample_image();
    let byte_limited = DisassemblyRequestV2 {
        target: DisassemblyTarget::Address(0x1000),
        range: None,
        limit: DisassemblyLimit::Bytes(7),
        options: DisassemblyOptions::default(),
    };
    let unlimited = DisassemblyRequestV2 {
        target: DisassemblyTarget::Address(0x1000),
        range: None,
        limit: DisassemblyLimit::Unlimited,
        options: DisassemblyOptions::default(),
    };

    assert_eq!(image.effective_instruction_limit_v2(&byte_limited), Some(2));
    assert_eq!(image.effective_instruction_limit_v2(&unlimited), None);
}

#[test]
fn objc_property_and_ivar_provenance_is_constructible() {
    let property = damsel_core::ObjcPropertyRecord {
        owner_pointer: 0x6000,
        name: Some("greeting".to_string()),
        name_source: ObjcNameSource::Runtime,
        attributes: Some("T@\"NSString\",&,N".to_string()),
    };
    let ivar = damsel_core::ObjcIvarRecord {
        owner_pointer: 0x6000,
        name: Some("_greeting".to_string()),
        name_source: ObjcNameSource::PointerTable,
        type_encoding: Some("@\"NSString\"".to_string()),
        offset: Some(0x18),
    };

    assert_eq!(property.name_source, ObjcNameSource::Runtime);
    assert_eq!(ivar.name_source, ObjcNameSource::PointerTable);
}

#[test]
fn recovered_value_and_export_kind_variants_are_constructible() {
    let recovered_export = damsel_core::RecoveredValue {
        register: "x0".to_string(),
        value: 0x1000,
        kind: damsel_core::RecoveredValueKind::ExportAddress,
        source: damsel_core::RecoveredValueSource::Other,
    };
    let recovered_fn = damsel_core::RecoveredValue {
        register: "x1".to_string(),
        value: 0x2000,
        kind: damsel_core::RecoveredValueKind::FunctionPointer,
        source: damsel_core::RecoveredValueSource::Other,
    };
    let export = damsel_core::ExportRecord {
        name: "_main".to_string(),
        address: Some(0x1000),
        flags: "regular".to_string(),
        kind: damsel_core::ExportKind::Regular,
    };

    assert!(matches!(
        recovered_export.kind,
        damsel_core::RecoveredValueKind::ExportAddress
    ));
    assert!(matches!(
        recovered_fn.kind,
        damsel_core::RecoveredValueKind::FunctionPointer
    ));
    assert!(matches!(export.kind, damsel_core::ExportKind::Regular));
}

#[test]
fn builder_rejects_missing_selected_slice() {
    let bytes: Arc<[u8]> = vec![0u8; 64].into();
    let builder = BinaryImage::builder(
        damsel_core::BinarySource::Memory {
            label: Some("builder-test".to_string()),
        },
        std::path::PathBuf::from("builder-test"),
        BinaryFormat::MachO,
        Architecture::Arm64,
        Endianness::Little,
        None,
        None,
        SliceInfo {
            offset: 0,
            size: 64,
            is_universal: false,
            cpu_subtype: 0,
        },
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        damsel_core::ObjcMetadata::default(),
        damsel_core::DyldMetadata::default(),
        bytes,
    )
    .with_available_slices(vec![SliceDescriptor {
        offset: 0,
        size: 64,
        is_universal: false,
        cpu_subtype: 0,
        architecture: Architecture::Arm64,
        selected: false,
    }]);

    let error = builder
        .build()
        .expect_err("builder should reject missing selected slice");
    assert_eq!(error, BinaryImageValidationError::MissingSelectedSlice);
}

#[test]
fn builder_rejects_selected_slice_mismatch() {
    let bytes: Arc<[u8]> = vec![0u8; 64].into();
    let builder = BinaryImage::builder(
        damsel_core::BinarySource::Memory {
            label: Some("builder-test".to_string()),
        },
        std::path::PathBuf::from("builder-test"),
        BinaryFormat::MachO,
        Architecture::Arm64,
        Endianness::Little,
        None,
        None,
        SliceInfo {
            offset: 0,
            size: 64,
            is_universal: false,
            cpu_subtype: 0,
        },
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        damsel_core::ObjcMetadata::default(),
        damsel_core::DyldMetadata::default(),
        bytes,
    )
    .with_available_slices(vec![SliceDescriptor {
        offset: 0,
        size: 64,
        is_universal: false,
        cpu_subtype: 0,
        architecture: Architecture::Arm64e,
        selected: true,
    }]);

    let error = builder
        .build()
        .expect_err("builder should reject mismatched selected slice");
    assert_eq!(error, BinaryImageValidationError::SelectedSliceMismatch);
}
