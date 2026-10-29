//! D4 structuring contract tests: condition resolution + diamonds.

use damsel_decomp::*;
use damsel_decomp::render_pseudocode;
use damsel_decomp::{IrBinOp, IrBlock, IrCondition, IrExpr, IrStatement, IrTerminator, IrValue};

fn flags_block(id: usize, start: u64, op: IrBinOp, lhs: &str, rhs: u64, taken: u64, fall: u64, mnemonic: &str) -> IrBlock {
    IrBlock {
        id,
        start_address: start,
        statements: vec![IrStatement::Assign {
            destination: "flags".into(),
            expression: IrExpr::BinOp {
                op,
                lhs: Box::new(IrExpr::Value(IrValue::Register(lhs.into()))),
                rhs: Box::new(IrExpr::Value(IrValue::Const(rhs))),
            },
        }],
        terminator: IrTerminator::CondBranch {
            condition: IrCondition::Raw(mnemonic.into()),
            taken,
            fallthrough: fall,
        },
    }
}

#[test]
fn conditions_resolve_from_flag_assignments() {
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![flags_block(0, 0, IrBinOp::Sub, "eax", 0x10, 0x20, 0x10, "jge")],
        instruction_count: 2,
        unknown_count: 0,
    };
    match resolve_condition(&function, 0) {
        ResolvedCondition::Relational { lhs, op, rhs } => {
            assert_eq!(lhs, "eax");
            assert_eq!(op, ">=");
            assert_eq!(rhs, "0x10");
        }
        other => panic!("expected relational, got {other:?}"),
    }
}

#[test]
fn test_mask_conditions_resolve() {
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![flags_block(0, 0, IrBinOp::And, "rax", 0, 0x20, 0x10, "jne")],
        instruction_count: 2,
        unknown_count: 0,
    };
    match resolve_condition(&function, 0) {
        ResolvedCondition::Mask { lhs, rhs, nonzero } => {
            assert_eq!(lhs, "rax");
            assert!(nonzero);
            let _ = rhs;
        }
        other => panic!("expected mask, got {other:?}"),
    }
}

#[test]
fn missing_flags_stay_unresolved() {
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![IrBlock {
            id: 0,
            start_address: 0,
            statements: vec![],
            terminator: IrTerminator::CondBranch {
                condition: IrCondition::Raw("b.eq".into()),
                taken: 0x20,
                fallthrough: 0x10,
            },
        }],
        instruction_count: 1,
        unknown_count: 0,
    };
    assert!(matches!(
        resolve_condition(&function, 0),
        ResolvedCondition::Unresolved(_)
    ));
}

#[test]
fn diamonds_render_structured_without_gotos() {
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![
            flags_block(0, 0x00, IrBinOp::Sub, "eax", 5, 0x10, 0x20, "jne"),
            IrBlock {
                id: 1,
                start_address: 0x10,
                statements: vec![IrStatement::Assign {
                    destination: "eax".into(),
                    expression: IrExpr::Value(IrValue::Const(1)),
                }],
                terminator: IrTerminator::Branch(0x30),
            },
            IrBlock {
                id: 2,
                start_address: 0x20,
                statements: vec![IrStatement::Assign {
                    destination: "eax".into(),
                    expression: IrExpr::Value(IrValue::Const(2)),
                }],
                terminator: IrTerminator::Branch(0x30),
            },
            IrBlock {
                id: 3,
                start_address: 0x30,
                statements: vec![],
                terminator: IrTerminator::Return,
            },
        ],
        instruction_count: 8,
        unknown_count: 0,
    };
    let pseudocode = render_pseudocode(&function, Some("diamond"));
    let texts: Vec<&str> = pseudocode.lines.iter().map(|l| l.text.as_str()).collect();
    assert!(texts.iter().any(|t| *t == "if (eax != 0x5) {"), "got: {texts:?}");
    assert!(texts.iter().any(|t| *t == "} else {"));
    assert!(texts.iter().any(|t| *t == "}"));
    // No gotos inside the diamond.
    assert!(!texts.iter().any(|t| t.contains("goto sub_10") || t.contains("goto sub_20") || t.contains("goto sub_30")));
    // The structured condition carries provenance.
    let cond_line = pseudocode.lines.iter().find(|l| l.text.starts_with("if (")).unwrap();
    assert_eq!(cond_line.source_addresses, vec![0]);
}

#[test]
fn diamonds_are_detected() {
    // cond(0) → then(1) / else(2); both branch to merge 0x30.
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![
            flags_block(0, 0x00, IrBinOp::Sub, "eax", 5, 0x10, 0x20, "jne"),
            IrBlock {
                id: 1,
                start_address: 0x10,
                statements: vec![IrStatement::Assign {
                    destination: "eax".into(),
                    expression: IrExpr::Value(IrValue::Const(1)),
                }],
                terminator: IrTerminator::Branch(0x30),
            },
            IrBlock {
                id: 2,
                start_address: 0x20,
                statements: vec![IrStatement::Assign {
                    destination: "eax".into(),
                    expression: IrExpr::Value(IrValue::Const(2)),
                }],
                terminator: IrTerminator::Branch(0x30),
            },
            IrBlock {
                id: 3,
                start_address: 0x30,
                statements: vec![],
                terminator: IrTerminator::Return,
            },
        ],
        instruction_count: 8,
        unknown_count: 0,
    };
    let diamonds = find_diamonds(&function);
    assert_eq!(diamonds.len(), 1);
    let diamond = &diamonds[0];
    assert_eq!(diamond.then_block, 1);
    assert_eq!(diamond.else_block, 2);
    assert_eq!(diamond.merge_address, 0x30);
    assert!(matches!(
        diamond.condition,
        ResolvedCondition::Relational { ref op, .. } if op == "!="
    ));
}

#[test]
fn back_edges_are_detected_and_annotated() {
    // Entry → body; body loops back conditionally to itself-ish.
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![
            IrBlock {
                id: 0,
                start_address: 0x00,
                statements: vec![],
                terminator: IrTerminator::Branch(0x10),
            },
            IrBlock {
                id: 1,
                start_address: 0x10,
                statements: vec![IrStatement::Assign {
                    destination: "flags".into(),
                    expression: IrExpr::BinOp {
                        op: IrBinOp::Sub,
                        lhs: Box::new(IrExpr::Value(IrValue::Register("eax".into()))),
                        rhs: Box::new(IrExpr::Value(IrValue::Const(0))),
                    },
                }],
                terminator: IrTerminator::CondBranch {
                    condition: IrCondition::Raw("jne".into()),
                    taken: 0x10,
                    fallthrough: 0x20,
                },
            },
            IrBlock {
                id: 2,
                start_address: 0x20,
                statements: vec![],
                terminator: IrTerminator::Return,
            },
        ],
        instruction_count: 4,
        unknown_count: 0,
    };
    let loops = find_loops(&function);
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].from_block, 1);
    assert_eq!(loops[0].to_address, 0x10);

    // Rendering annotates the back-edge goto.
    let pseudocode = render_pseudocode(&function, Some("loop"));
    let texts: Vec<&str> = pseudocode.lines.iter().map(|l| l.text.as_str()).collect();
    assert!(texts.iter().any(|t| t.contains("goto sub_10; /* loop back */")), "got: {texts:?}");
}

#[test]
fn arm64_cmp_and_b_cond_resolve() {
    // cmp w0, #5; b.ge elsewhere  → flags = (w0 - 5); w0 >= 0x5.
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![IrBlock {
            id: 0,
            start_address: 0,
            statements: vec![IrStatement::Assign {
                destination: "flags".into(),
                expression: IrExpr::BinOp {
                    op: IrBinOp::Sub,
                    lhs: Box::new(IrExpr::Value(IrValue::Register("w0".into()))),
                    rhs: Box::new(IrExpr::Value(IrValue::Const(5))),
                },
            }],
            terminator: IrTerminator::CondBranch {
                condition: IrCondition::Raw("b.ge".into()),
                taken: 0x20,
                fallthrough: 0x10,
            },
        }],
        instruction_count: 2,
        unknown_count: 0,
    };
    match resolve_condition(&function, 0) {
        ResolvedCondition::Relational { lhs, op, rhs } => {
            assert_eq!((lhs.as_str(), op.as_str(), rhs.as_str()), ("w0", ">=", "0x5"));
        }
        other => panic!("expected relational, got {other:?}"),
    }
}

#[test]
fn arm64_lift_emits_flags_for_cmp() {
    // Real decode: cmp w0, #0x2a then b.ne — craft via decode of a tiny
    // authored stream using the fixture engine is heavy; use lift on a
    // synthetic instruction through the public API instead.
    use damsel_core::{DecodedInstruction, Operand};
    let instruction = DecodedInstruction {
        address: 0,
        size: 4,
        opcode: 0,
        mnemonic: "cmp".to_string(),
        operands: vec![
            Operand::Register("w0".to_string()),
            Operand::ImmediateUnsigned(0x2a),
        ],
        recovered_values: Vec::new(),
        references: Vec::new(),
        annotations: Vec::new(),
    };
    let function = lift_arm64(&[instruction]);
    let statements = &function.blocks[0].statements;
    assert!(statements.iter().any(|s| matches!(s,
        IrStatement::Assign { destination, expression }
            if destination == "flags"
                && matches!(expression, IrExpr::BinOp { op: IrBinOp::Sub, .. }))));
    assert_eq!(function.unknown_count, 0, "cmp is modeled, not unknown");
}
