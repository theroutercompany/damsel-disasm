use damsel_core::{
    Annotation, DisassemblyLimit, DisassemblyOptions, Operand, Reference, decode_aarch64,
    decode_aarch64_v2, decode_aarch64_with_limit,
};

fn decode(bytes: &[u8], start: u64, max: Option<usize>) -> Vec<damsel_core::DecodedInstruction> {
    decode_aarch64(bytes, start, max).expect("decode succeeds")
}

#[test]
fn decode_applies_instruction_limit() {
    let bytes = [
        0xc0, 0x03, 0x5f, 0xd6, // ret
        0xc0, 0x03, 0x5f, 0xd6, // ret
    ];
    let instructions = decode(&bytes, 0x1000, Some(1));
    assert_eq!(instructions.len(), 1);
}

#[test]
fn decode_marks_range_start_annotation() {
    let bytes = [0xc0, 0x03, 0x5f, 0xd6];
    let instructions = decode(&bytes, 0x1000, Some(1));
    assert!(
        instructions[0]
            .annotations
            .contains(&Annotation::Note("range start".to_string()))
    );
}

#[test]
fn decode_classifies_branch_call_and_page_references() {
    let bytes = [
        0x00, 0x00, 0x00, 0x94, // bl
        0x00, 0x00, 0x00, 0x14, // b
        0x00, 0x00, 0x00, 0x90, // adrp
    ];
    let instructions = decode(&bytes, 0x1000, None);

    assert!(matches!(
        instructions[0].references.first(),
        Some(Reference::Call { .. })
    ));
    assert!(matches!(
        instructions[1].references.first(),
        Some(Reference::Branch { .. })
    ));
    assert!(matches!(
        instructions[2].references.first(),
        Some(Reference::Page { .. })
    ));
}

#[test]
fn decode_uses_final_immediate_for_test_bit_branch_targets() {
    let bytes = [0x28, 0x01, 0x00, 0x37]; // tbnz w8, #0, 0x10000053c
    let instructions = decode(&bytes, 0x100000518, None);

    assert_eq!(instructions.len(), 1);
    assert!(
        instructions[0].references.iter().any(
            |reference| matches!(reference, Reference::Branch { target } if *target == 0x10000053c)
        ),
        "expected tbnz to reference the branch destination, got {:?}",
        instructions[0].references
    );
    assert!(
        !instructions[0]
            .references
            .iter()
            .any(|reference| matches!(reference, Reference::Branch { target } if *target == 0)),
        "bit index must not be recorded as a branch target"
    );
    assert!(
        matches!(
            instructions[0].operands.get(1),
            Some(Operand::ImmediateUnsigned(0))
        ),
        "tbnz bit index should stay an immediate operand, got {:?}",
        instructions[0].operands
    );
    assert!(
        matches!(
            instructions[0].operands.get(2),
            Some(Operand::Label(0x10000053c))
        ),
        "tbnz branch destination should be the only label operand, got {:?}",
        instructions[0].operands
    );
}

#[test]
fn decode_emits_first_class_indirect_control_references() {
    let bytes = [
        0x00, 0x00, 0x3f, 0xd6, // blr x0
        0x00, 0x00, 0x1f, 0xd6, // br x0
    ];
    let instructions = decode(&bytes, 0x1000, None);

    assert!(instructions.iter().any(|instruction| {
        instruction
            .references
            .iter()
            .any(|reference| matches!(reference, Reference::IndirectCall { via } if via == "x0"))
    }));
    assert!(instructions.iter().any(|instruction| {
        instruction
            .references
            .iter()
            .any(|reference| matches!(reference, Reference::IndirectBranch { via } if via == "x0"))
    }));
}

#[test]
fn decode_does_not_misclassify_add_immediate_as_branch() {
    let bytes = [0x00, 0x04, 0x00, 0x91]; // add x0, x0, #1
    let instructions = decode(&bytes, 0x1000, None);
    assert_eq!(instructions.len(), 1);
    assert!(
        !instructions[0]
            .references
            .iter()
            .any(|reference| matches!(reference, Reference::Branch { .. }))
    );
    assert!(
        instructions[0].references.is_empty(),
        "ordinary arithmetic immediates must not become data references"
    );
}

#[test]
fn decode_preserves_shifted_register_operands() {
    let bytes = [0x20, 0x0c, 0x02, 0x8b]; // add x0, x1, x2, lsl #3
    let instructions = decode(&bytes, 0x1000, None);
    assert_eq!(instructions.len(), 1);
    assert_eq!(
        instructions[0].operands,
        vec![
            Operand::Register("x0".to_string()),
            Operand::Register("x1".to_string()),
            Operand::ShiftedRegister {
                register: "x2".to_string(),
                shift: "lsl #3".to_string(),
            },
        ],
        "shifted register operands must not collapse to plain registers"
    );
    assert_eq!(instructions[0].render(), "add x0, x1, x2, lsl #3");
}

#[test]
fn decode_rejects_truncated_instruction_stream() {
    let bytes = [0x00, 0x00, 0x00];
    match decode_aarch64(&bytes, 0x1000, None) {
        Ok(instructions) => {
            assert!(
                instructions.is_empty(),
                "expected no decodable instruction from truncated input"
            );
        }
        Err(error) => {
            let text = error.to_string().to_ascii_lowercase();
            assert!(
                text.contains("truncated") || text.contains("invalid instruction stream"),
                "unexpected decode error: {error}"
            );
        }
    }
}

#[test]
fn decode_with_limit_respects_byte_limit_and_annotation_flag() {
    let bytes = [
        0xc0, 0x03, 0x5f, 0xd6, // ret
        0xc0, 0x03, 0x5f, 0xd6, // ret
    ];
    let instructions = decode_aarch64_with_limit(
        &bytes,
        0x1000,
        DisassemblyLimit::Bytes(4),
        DisassemblyOptions {
            include_annotations: false,
            include_value_flow: false,
        },
    )
    .expect("decode succeeds");
    assert_eq!(instructions.len(), 1);
    assert!(instructions[0].annotations.is_empty());
}

#[test]
fn decode_v2_entrypoint_is_equivalent_to_with_limit() {
    let bytes = [
        0xc0, 0x03, 0x5f, 0xd6, // ret
        0xc0, 0x03, 0x5f, 0xd6, // ret
    ];
    let limit = DisassemblyLimit::Instructions(1);
    let options = DisassemblyOptions {
        include_annotations: false,
        include_value_flow: false,
    };

    let via_limit = decode_aarch64_with_limit(&bytes, 0x1000, limit, options).expect("decode");
    let via_v2 = decode_aarch64_v2(&bytes, 0x1000, limit, options).expect("decode");
    assert_eq!(via_limit, via_v2);
}

#[test]
fn decode_legacy_adapter_matches_with_limit_defaults() {
    let bytes = [
        0xc0, 0x03, 0x5f, 0xd6, // ret
        0xc0, 0x03, 0x5f, 0xd6, // ret
    ];
    let via_legacy = decode_aarch64(&bytes, 0x1000, Some(1)).expect("decode");
    let via_limit = decode_aarch64_with_limit(
        &bytes,
        0x1000,
        DisassemblyLimit::Instructions(1),
        DisassemblyOptions::default(),
    )
    .expect("decode");
    assert_eq!(via_legacy, via_limit);
}
