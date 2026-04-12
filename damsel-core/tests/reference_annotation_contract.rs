use damsel_core::{
    Annotation, ImportBindingKind, ImportBindingRecord, ImportBindingSource, Reference, Relocation,
    StubEntry, StubKind,
};

#[test]
fn constructs_indirect_control_flow_reference_variants() {
    let call = Reference::IndirectCall {
        via: "x16".to_string(),
    };
    let branch = Reference::IndirectBranch {
        via: "x17".to_string(),
    };

    assert!(matches!(call, Reference::IndirectCall { ref via } if via == "x16"));
    assert!(matches!(branch, Reference::IndirectBranch { ref via } if via == "x17"));
}

#[test]
fn reference_from_binding_preserves_fields() {
    let binding = ImportBindingRecord {
        dylib: "/usr/lib/libSystem.B.dylib".to_string(),
        name: "_puts".to_string(),
        address: Some(0x1010),
        offset: Some(0x24),
        addend: -4,
        ordinal: Some(1),
        symbol_index: Some(3),
        binding_kind: ImportBindingKind::ChainedFixup,
        source: ImportBindingSource::ChainedFixup,
        is_weak: true,
    };

    let reference = Reference::from_binding(&binding);
    assert!(matches!(
        reference,
        Reference::ImportBinding {
            ref dylib,
            ref name,
            address: Some(0x1010),
            offset: Some(0x24),
            addend: -4,
            binding_kind: ImportBindingKind::ChainedFixup,
            source: ImportBindingSource::ChainedFixup,
            is_weak: true,
        } if dylib == "/usr/lib/libSystem.B.dylib" && name == "_puts"
    ));
}

#[test]
fn reference_from_stub_preserves_fields() {
    let stub = StubEntry {
        stub_address: 0x2000,
        section: Some("__TEXT:__stubs".to_string()),
        pointer_section: Some("__DATA_CONST:__got".to_string()),
        pointer_address: Some(0x2010),
        helper_address: None,
        binding_ordinal: Some(1),
        stub_kind: StubKind::NonLazy,
        dylib: Some("/usr/lib/libSystem.B.dylib".to_string()),
        name: Some("_puts".to_string()),
        source: ImportBindingSource::Stub,
    };

    let reference = Reference::from_stub(&stub);
    assert!(matches!(
        reference,
        Reference::Stub {
            stub_address: 0x2000,
            section: Some(ref section),
            pointer_address: Some(0x2010),
            ref dylib,
            ref name,
            source: ImportBindingSource::Stub,
            ..
        } if section == "__TEXT:__stubs"
            && dylib.as_deref() == Some("/usr/lib/libSystem.B.dylib")
            && name.as_deref() == Some("_puts")
    ));
}

#[test]
fn reference_from_stub_helper_preserves_fields() {
    let helper = damsel_core::StubHelperEntry {
        helper_address: 0x2100,
        target_stub: Some(0x2000),
        stub_section: Some("__TEXT:__stubs".to_string()),
        pointer_address: Some(0x2010),
        pointer_section: Some("__DATA_CONST:__la_symbol_ptr".to_string()),
        binding_ordinal: Some(1),
        dylib: Some("/usr/lib/libSystem.B.dylib".to_string()),
        name: Some("_puts".to_string()),
    };

    let reference = Reference::from_stub_helper(&helper);
    assert!(matches!(
        reference,
        Reference::StubHelper {
            helper_address: 0x2100,
            target_stub: Some(0x2000),
            pointer_address: Some(0x2010),
            ref dylib,
            ref name,
            ..
        } if dylib.as_deref() == Some("/usr/lib/libSystem.B.dylib")
            && name.as_deref() == Some("_puts")
    ));
}

#[test]
fn constructs_annotation_evidence_from_helpers() {
    let binding = ImportBindingRecord {
        dylib: "/usr/lib/libSystem.B.dylib".to_string(),
        name: "_puts".to_string(),
        address: Some(0x1010),
        offset: Some(0x24),
        addend: 7,
        ordinal: Some(1),
        symbol_index: Some(3),
        binding_kind: ImportBindingKind::NonLazy,
        source: ImportBindingSource::IndirectSymbol,
        is_weak: false,
    };
    let relocation = Relocation {
        section: "__TEXT:__text".to_string(),
        address: 0x3004,
        size: 8,
        kind: "Absolute".to_string(),
        encoding: "Generic".to_string(),
        target: "absolute".to_string(),
        addend: 1,
    };

    let binding_evidence = Annotation::import_binding_evidence(&binding);
    let relocation_evidence = Annotation::relocation_evidence(&relocation);

    assert!(matches!(
        binding_evidence,
        Annotation::ImportBindingEvidence {
            ref dylib,
            ref name,
            address: Some(0x1010),
            offset: Some(0x24),
            addend: 7,
            binding_kind: ImportBindingKind::NonLazy,
            source: ImportBindingSource::IndirectSymbol,
        } if dylib == "/usr/lib/libSystem.B.dylib" && name == "_puts"
    ));
    assert!(matches!(
        relocation_evidence,
        Annotation::RelocationEvidence {
            address: 0x3004,
            ref kind,
            ref encoding,
            ref target,
            addend: 1,
        } if kind == "Absolute" && encoding == "Generic" && target == "absolute"
    ));
}

#[test]
fn display_for_indirect_control_flow_annotation() {
    let annotation = Annotation::IndirectControlFlow {
        kind: "call".to_string(),
        via: "x16".to_string(),
    };
    assert_eq!(annotation.to_string(), "indirect call via x16");
}

#[test]
fn display_for_import_binding_annotation_includes_expected_fields() {
    let annotation = Annotation::ImportBinding {
        dylib: "/usr/lib/libSystem.B.dylib".to_string(),
        name: "_puts".to_string(),
        address: Some(0x1010),
        offset: Some(0x24),
        addend: -2,
        binding_kind: ImportBindingKind::ChainedFixup,
        source: ImportBindingSource::ChainedFixup,
    };
    assert_eq!(
        annotation.to_string(),
        "binding /usr/lib/libSystem.B.dylib:_puts addr=0x1010 off=0x24 addend=-2 kind=ChainedFixup source=ChainedFixup"
    );
}

#[test]
fn display_for_import_binding_evidence_annotation_uses_evidence_prefix() {
    let annotation = Annotation::ImportBindingEvidence {
        dylib: "/usr/lib/libSystem.B.dylib".to_string(),
        name: "_puts".to_string(),
        address: None,
        offset: None,
        addend: 0,
        binding_kind: ImportBindingKind::NonLazy,
        source: ImportBindingSource::Other,
    };
    assert_eq!(
        annotation.to_string(),
        "binding-evidence /usr/lib/libSystem.B.dylib:_puts addr=- off=- addend=0 kind=NonLazy source=Other"
    );
}

#[test]
fn display_for_relocation_evidence_annotation_uses_reloc_evidence_prefix() {
    let annotation = Annotation::RelocationEvidence {
        address: 0x3004,
        kind: "Absolute".to_string(),
        encoding: "Generic".to_string(),
        target: "absolute".to_string(),
        addend: 11,
    };
    assert_eq!(
        annotation.to_string(),
        "reloc-evidence kind=Absolute encoding=Generic target=absolute addend=11 addr=0x3004"
    );
}
