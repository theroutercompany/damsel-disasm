use damsel_core::{
    Annotation, Architecture, BinaryFormat, BinaryImage, DisassemblyRequest, DisassemblyTarget,
    Endianness, ImportBindingRecord, ImportBindingSource, Reference, Relocation, SliceDescriptor,
    SliceInfo,
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
        source: ImportBindingSource::ChainedFixup,
        is_weak: false,
    };
    let stub = damsel_core::StubEntry {
        stub_address: 0x3000,
        pointer_address: Some(0x3010),
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
