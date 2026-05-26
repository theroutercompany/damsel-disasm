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
fn decode_labels_direct_call_targets() {
    let bytes = [0x02, 0x00, 0x00, 0x94]; // bl #0x1008
    let instructions = decode(&bytes, 0x1000, None);

    assert_eq!(instructions.len(), 1);
    assert!(
        instructions[0]
            .references
            .iter()
            .any(|reference| matches!(reference, Reference::Call { target } if *target == 0x1008)),
        "direct calls should recover their target reference, got {:?}",
        instructions[0].references
    );
    assert!(
        matches!(
            instructions[0].operands.first(),
            Some(Operand::Label(0x1008))
        ),
        "direct call targets should render as label operands, got {:?}",
        instructions[0].operands
    );
}

#[test]
fn decode_preserves_high_half_address_targets() {
    let start = 0xffff_ffff_ffff_f000;
    let bytes = [
        0x00, 0x00, 0x00, 0x14, // b #0xfffffffffffff000
        0x00, 0x00, 0x00, 0x94, // bl #0xfffffffffffff004
        0x00, 0x00, 0x00, 0x90, // adrp x0, 0xfffffffffffff000
    ];
    let instructions = decode(&bytes, start, None);

    assert_eq!(instructions.len(), 3);
    assert!(
        instructions[0]
            .references
            .contains(&Reference::Branch { target: start }),
        "high-half direct branch target should be preserved, got {:?}",
        instructions[0].references
    );
    assert!(
        matches!(instructions[0].operands.first(), Some(Operand::Label(target)) if *target == start),
        "high-half branch target should render as a label, got {:?}",
        instructions[0].operands
    );

    assert!(
        instructions[1]
            .references
            .contains(&Reference::Call { target: start + 4 }),
        "high-half direct call target should be preserved, got {:?}",
        instructions[1].references
    );
    assert!(
        matches!(instructions[1].operands.first(), Some(Operand::Label(target)) if *target == start + 4),
        "high-half call target should render as a label, got {:?}",
        instructions[1].operands
    );

    assert!(
        instructions[2]
            .references
            .contains(&Reference::Page { target: start }),
        "high-half page materialization target should be preserved, got {:?}",
        instructions[2].references
    );
    assert!(
        matches!(instructions[2].operands.get(1), Some(Operand::Label(target)) if *target == start),
        "high-half adrp target should render as a label, got {:?}",
        instructions[2].operands
    );
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
fn decode_labels_bc_conditional_branch_targets() {
    let bytes = [0x50, 0x00, 0x00, 0x54]; // bc.eq #0x1008
    let instructions = decode(&bytes, 0x1000, None);

    assert_eq!(instructions.len(), 1);
    assert_eq!(instructions[0].mnemonic, "bc.eq");
    assert!(
        instructions[0].references.iter().any(
            |reference| matches!(reference, Reference::Branch { target } if *target == 0x1008)
        ),
        "bc.cond should recover its branch target, got {:?}",
        instructions[0].references
    );
    assert!(
        matches!(
            instructions[0].operands.first(),
            Some(Operand::Label(0x1008))
        ),
        "bc.cond branch destination should render as a label operand, got {:?}",
        instructions[0].operands
    );
}

#[test]
fn decode_treats_authenticated_exception_returns_as_terminal() {
    let bytes = [
        0xff, 0x0b, 0x9f, 0xd6, // eretaa
        0xff, 0x0f, 0x9f, 0xd6, // eretab
    ];
    let instructions = decode(&bytes, 0x1000, None);

    assert_eq!(instructions.len(), 2);
    assert_eq!(instructions[0].mnemonic, "eretaa");
    assert_eq!(instructions[1].mnemonic, "eretab");
    assert!(instructions.iter().all(|instruction| {
        instruction.references.is_empty()
            && !instruction.annotations.iter().any(|annotation| {
                matches!(
                    annotation,
                    Annotation::Note(note) if note.starts_with("control-flow target unresolved")
                )
            })
    }));
}

#[test]
fn decode_treats_terminal_traps_as_targetless_boundaries() {
    let bytes = [
        0x20, 0x00, 0x20, 0xd4, // brk #1
        0x00, 0x00, 0x40, 0xd4, // hlt #0
        0x00, 0x00, 0x00, 0x00, // udf #0
    ];
    let instructions = decode(&bytes, 0x1000, None);

    assert_eq!(instructions.len(), 3);
    assert_eq!(instructions[0].mnemonic, "brk");
    assert_eq!(instructions[1].mnemonic, "hlt");
    assert_eq!(instructions[2].mnemonic, "udf");
    assert!(instructions.iter().all(|instruction| {
        instruction.references.is_empty()
            && !instruction.annotations.iter().any(|annotation| {
                matches!(
                    annotation,
                    Annotation::Note(note) if note.starts_with("control-flow target unresolved")
                )
            })
    }));
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
fn decode_uses_only_target_register_for_authenticated_indirect_controls() {
    let bytes = [
        0x01, 0x08, 0x3f, 0xd7, // blraa x0, x1
        0x85, 0x08, 0x1f, 0xd7, // braa x4, x5
        0x1f, 0x09, 0x3f, 0xd6, // blraaz x8
        0x3f, 0x0d, 0x1f, 0xd6, // brabz x9
    ];
    let instructions = decode(&bytes, 0x1000, None);

    assert_eq!(instructions.len(), 4);
    assert_eq!(instructions[0].mnemonic, "blraa");
    assert!(
        instructions[0]
            .references
            .contains(&Reference::IndirectCall {
                via: "x0".to_string()
            }),
        "authenticated indirect call should use the target register, got {:?}",
        instructions[0].references
    );
    assert!(
        !instructions[0]
            .references
            .contains(&Reference::IndirectCall {
                via: "x1".to_string()
            }),
        "authenticated call modifier register must not become a call target: {:?}",
        instructions[0].references
    );

    assert_eq!(instructions[1].mnemonic, "braa");
    assert!(
        instructions[1]
            .references
            .contains(&Reference::IndirectBranch {
                via: "x4".to_string()
            }),
        "authenticated indirect branch should use the target register, got {:?}",
        instructions[1].references
    );
    assert!(
        !instructions[1]
            .references
            .contains(&Reference::IndirectBranch {
                via: "x5".to_string()
            }),
        "authenticated branch modifier register must not become a branch target: {:?}",
        instructions[1].references
    );

    assert!(
        instructions[2]
            .references
            .contains(&Reference::IndirectCall {
                via: "x8".to_string()
            })
    );
    assert!(
        instructions[3]
            .references
            .contains(&Reference::IndirectBranch {
                via: "x9".to_string()
            })
    );
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
fn decode_preserves_extended_register_operands() {
    let bytes = [0x20, 0x4c, 0x22, 0x8b]; // add x0, x1, w2, uxtw #3
    let instructions = decode(&bytes, 0x1000, None);
    assert_eq!(instructions.len(), 1);
    assert_eq!(
        instructions[0].operands,
        vec![
            Operand::Register("x0".to_string()),
            Operand::Register("x1".to_string()),
            Operand::ShiftedRegister {
                register: "w2".to_string(),
                shift: "uxtw #3".to_string(),
            },
        ],
        "extended register operands must not collapse to plain registers"
    );
    assert_eq!(instructions[0].render(), "add x0, x1, w2, uxtw #3");
}

#[test]
fn decode_preserves_memory_index_modifiers() {
    let shifted = decode(&[0x20, 0x78, 0x62, 0xf8], 0x1000, None); // ldr x0, [x1, x2, lsl #3]
    assert_eq!(shifted.len(), 1);
    assert_eq!(
        shifted[0].operands,
        vec![
            Operand::Register("x0".to_string()),
            Operand::Memory {
                base: "x1".to_string(),
                index: Some("x2".to_string()),
                index_modifier: Some("lsl #3".to_string()),
                displacement: 0,
                writeback: None,
            },
        ],
        "memory index shifts must stay separate from the raw index register"
    );
    assert_eq!(shifted[0].render(), "ldr x0, [x1, x2, lsl #3]");

    let extended = decode(&[0x20, 0x58, 0x62, 0xf8], 0x1000, None); // ldr x0, [x1, w2, uxtw #3]
    assert_eq!(extended.len(), 1);
    assert_eq!(
        extended[0].operands,
        vec![
            Operand::Register("x0".to_string()),
            Operand::Memory {
                base: "x1".to_string(),
                index: Some("w2".to_string()),
                index_modifier: Some("uxtw #3".to_string()),
                displacement: 0,
                writeback: None,
            },
        ],
        "memory index extenders must stay separate from the raw index register"
    );
    assert_eq!(extended[0].render(), "ldr x0, [x1, w2, uxtw #3]");
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
