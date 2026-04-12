use damsel_core::{
    DisassemblyLimit, DisassemblyOptions, DisassemblyRequest, DisassemblyRequestV2,
    DisassemblyStopReason, DisassemblyTarget, Reference,
};
use damsel_macho::{disassemble, disassemble_v2, load, MachoError};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn disasm_unknown_symbol_errors() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Symbol("__missing_symbol__".to_string()),
        max_instructions: Some(8),
        limit: None,
        include_annotations: true,
    };
    let error = disassemble(&image, &request).expect_err("unknown symbol should fail");
    assert!(matches!(error, MachoError::SymbolNotFound(_)));
}

#[test]
fn disasm_unknown_section_errors() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__missing_section__".to_string()),
        max_instructions: Some(8),
        limit: None,
        include_annotations: true,
    };
    let error = disassemble(&image, &request).expect_err("unknown section should fail");
    assert!(matches!(error, MachoError::SectionNotFound(_)));
}

#[test]
fn disasm_unmapped_address_errors() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Address(0x1),
        max_instructions: Some(8),
        limit: None,
        include_annotations: true,
    };
    let error = disassemble(&image, &request).expect_err("unmapped address should fail");
    assert!(matches!(error, MachoError::AddressNotMapped(0x1)));
}

#[test]
fn disasm_by_address_smoke() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let main_symbol = image.symbol_by_name("_main").expect("main symbol exists");

    let request = DisassemblyRequest {
        target: DisassemblyTarget::Address(main_symbol.address),
        max_instructions: Some(6),
        limit: None,
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble by address");
    assert_eq!(result.start_address, main_symbol.address);
    assert!(!result.instructions.is_empty());
}

#[test]
fn disasm_by_section_smoke() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__text".to_string()),
        max_instructions: Some(6),
        limit: None,
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble by section");
    assert!(result.target.contains("__text"));
    assert!(!result.instructions.is_empty());
}

#[test]
fn disasm_honors_include_annotations_flag() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Symbol("_main".to_string()),
        max_instructions: Some(8),
        limit: None,
        include_annotations: false,
    };
    let result = disassemble(&image, &request).expect("disassemble main without annotations");
    assert!(!result.instructions.is_empty());
    assert!(result
        .instructions
        .iter()
        .all(|instruction| instruction.annotations.is_empty()));
}

#[test]
fn disasm_reports_limit_reached_stop_reason() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__text".to_string()),
        max_instructions: None,
        limit: Some(DisassemblyLimit::Instructions(1)),
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble with hard instruction cap");
    assert_eq!(result.stop_reason, DisassemblyStopReason::LimitReached);
}

#[test]
fn disasm_reports_decode_halt_for_too_short_byte_window() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__text".to_string()),
        max_instructions: None,
        limit: Some(DisassemblyLimit::Bytes(3)),
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble tiny byte window");
    assert_eq!(result.stop_reason, DisassemblyStopReason::DecodeHalt);
}

#[test]
fn disasm_v2_range_clamps_target_window() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let main_symbol = image.symbol_by_name("_main").expect("main symbol exists");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Address(main_symbol.address),
        range: Some(main_symbol.address..main_symbol.address.saturating_add(8)),
        limit: DisassemblyLimit::Unlimited,
        options: DisassemblyOptions {
            include_annotations: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble v2 with range");
    assert_eq!(result.start_address, main_symbol.address);
    assert!(result.decoded_bytes <= 8, "range should clamp decoded bytes");
    assert!(matches!(
        result.stop_reason,
        DisassemblyStopReason::LimitReached | DisassemblyStopReason::TargetRangeEnd
    ));
}

#[test]
fn disasm_emits_import_references_when_targets_match_import_sites() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let target = if let Some(stub) = image.dyld.stubs.first() {
        DisassemblyTarget::Address(stub.stub_address)
    } else {
        DisassemblyTarget::Section("__text".to_string())
    };

    if image.dyld.stubs.is_empty()
        && image.dyld.import_bindings.is_empty()
        && image.imports().iter().all(|import| import.address.is_none())
    {
        return;
    }

    let request = DisassemblyRequest {
        target,
        max_instructions: Some(64),
        limit: None,
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble target window");

    let has_import_reference = result.instructions.iter().any(|instruction| {
        instruction.references.iter().any(|reference| {
            matches!(
                reference,
                Reference::Import { .. } | Reference::ImportBinding { .. } | Reference::Stub { .. }
            )
        })
    });
    assert!(
        has_import_reference,
        "expected at least one import-related reference in selected target disassembly"
    );
}
