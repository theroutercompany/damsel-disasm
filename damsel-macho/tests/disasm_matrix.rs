use damsel_core::{DisassemblyRequest, DisassemblyTarget};
use damsel_macho::{MachoError, disassemble, load};
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
