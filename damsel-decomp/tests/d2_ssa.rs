//! D2 data-flow contract tests over the lifted `_main` fixture.

mod support;

use damsel_core::{DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget};
use damsel_decomp::{build_ssa, lift_arm64};

fn lift_main() -> damsel_decomp::IrFunction {
    let image = damsel_macho::load(support::fixture("arm64-symbolized")).unwrap();
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_main".to_string()),
        range: None,
        limit: DisassemblyLimit::Unlimited,
        options: DisassemblyOptions {
            include_annotations: false,
            include_value_flow: false,
        },
    };
    let result = damsel_macho::disassemble_v2(&image, &request).unwrap();
    lift_arm64(&result.instructions)
}

#[test]
fn ssa_reports_defs_uses_and_cfg() {
    let function = lift_main();
    let report = build_ssa(&function);
    assert!(report.total_defs > 0, "main writes registers");
    assert!(report.total_uses > 0, "main reads registers");
    assert!(!report.defs.is_empty());
    // Deterministic serialization for snapshots.
    let a = serde_json::to_string(&report).unwrap();
    let b = serde_json::to_string(&build_ssa(&lift_main())).unwrap();
    assert_eq!(a, b);
}

#[test]
fn empty_function_is_empty_report() {
    let function = lift_arm64(&[]);
    let report = build_ssa(&function);
    assert_eq!(report.total_defs, 0);
    assert_eq!(report.total_uses, 0);
    assert!(report.phi_candidates.is_empty());
}

#[test]
fn phi_candidates_flag_multi_def_registers_at_merges() {
    // Hand-built IR: two defs of x0 merging into a multi-predecessor
    // block that reads x0.
    use damsel_decomp::*;
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![
            IrBlock {
                id: 0,
                start_address: 0,
                statements: vec![IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Const(1)),
                }],
                terminator: IrTerminator::CondBranch {
                    condition: IrCondition::Raw("b.eq".into()),
                    taken: 16,
                    fallthrough: 8,
                },
            },
            IrBlock {
                id: 1,
                start_address: 8,
                statements: vec![IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Const(2)),
                }],
                terminator: IrTerminator::Branch(16),
            },
            IrBlock {
                id: 2,
                start_address: 16,
                statements: vec![IrStatement::Assign {
                    destination: "x1".into(),
                    expression: IrExpr::Value(IrValue::Register("x0".into())),
                }],
                terminator: IrTerminator::Return,
            },
        ],
        instruction_count: 6,
        unknown_count: 0,
    };
    let report = build_ssa(&function);
    // Block 2 has two predecessors (blocks 0 and 1).
    assert_eq!(report.predecessors.get(&2).unwrap().len(), 2);
    // x0 has two defs and is read in block 2 → phi candidate.
    assert!(report
        .phi_candidates
        .iter()
        .any(|p| p.register == "x0" && p.block == 2 && p.defs == 2));
    // x1 has one def → never a candidate.
    assert!(!report.phi_candidates.iter().any(|p| p.register == "x1"));
}

#[test]
fn register_renaming_versions_per_block() {
    use damsel_decomp::rename_registers;
    use damsel_decomp::{IrBlock, IrExpr, IrFunction, IrStatement, IrTerminator, IrValue};

    let function = IrFunction {
        entry_address: 0,
        blocks: vec![IrBlock {
            id: 0,
            start_address: 0,
            statements: vec![
                IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Const(1)),
                },
                IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Const(2)),
                },
                IrStatement::Assign {
                    destination: "x1".into(),
                    expression: IrExpr::Value(IrValue::Register("x0".into())),
                },
            ],
            terminator: IrTerminator::Return,
        }],
        instruction_count: 3,
        unknown_count: 0,
    };

    let renamed = rename_registers(&function);
    let statements = &renamed.blocks[0].statements;

    // First def: x0_v1 = 1
    assert!(matches!(&statements[0],
        IrStatement::Assign { destination, expression: IrExpr::Value(IrValue::Const(1)) }
            if destination == "x0_v1"));
    // Second def: x0_v2 = 2
    assert!(matches!(&statements[1],
        IrStatement::Assign { destination, expression: IrExpr::Value(IrValue::Const(2)) }
            if destination == "x0_v2"));
    // Use of the latest version: x1_v1 = x0_v2
    assert!(matches!(&statements[2],
        IrStatement::Assign { destination, expression: IrExpr::Value(IrValue::Register(r)) }
            if destination == "x1_v1" && r == "x0_v2"));
}

#[test]
fn abi_roles_cover_argument_return_and_callee_saved() {
    use damsel_decomp::{AbiRole, arm64_abi_roles, x86_64_abi_roles};

    let arm = arm64_abi_roles();
    assert!(arm.iter().any(|r| matches!(r.role, AbiRole::Argument { index: 0 }) && r.register == "x0"));
    assert!(arm.iter().any(|r| r.role == AbiRole::Return && r.register == "x0"));
    assert!(arm.iter().any(|r| r.role == AbiRole::CalleeSaved && r.register == "x19"));
    assert!(arm.iter().any(|r| r.role == AbiRole::StackPointer && r.register == "sp"));

    let x86 = x86_64_abi_roles();
    assert!(x86.iter().any(|r| matches!(r.role, AbiRole::Argument { index: 0 }) && r.register == "rdi"));
    assert!(x86.iter().any(|r| r.role == AbiRole::Return && r.register == "rax"));
    assert!(x86.iter().any(|r| r.role == AbiRole::CalleeSaved && r.register == "rbx"));
}

#[test]
fn phi_nodes_merge_cross_block_versions() {
    use damsel_decomp::*;
    use damsel_decomp::{IrBinOp, IrBlock, IrCondition, IrExpr, IrStatement, IrTerminator, IrValue};

    // Two blocks define x0 differently; block 2 has both as preds.
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![
            IrBlock {
                id: 0,
                start_address: 0,
                statements: vec![IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Const(1)),
                }],
                terminator: IrTerminator::CondBranch {
                    condition: IrCondition::Raw("b.eq".into()),
                    taken: 0x20,
                    fallthrough: 0x10,
                },
            },
            IrBlock {
                id: 1,
                start_address: 0x10,
                statements: vec![IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Const(2)),
                }],
                terminator: IrTerminator::Branch(0x20),
            },
            IrBlock {
                id: 2,
                start_address: 0x20,
                statements: vec![],
                terminator: IrTerminator::Return,
            },
        ],
        instruction_count: 5,
        unknown_count: 0,
    };

    let renamed = rename_registers(&function);
    let phis = build_phi_nodes(&function, &renamed);
    assert_eq!(phis.len(), 1, "x0 needs a phi at block 2");
    assert_eq!(phis[0].block, 2);
    assert_eq!(phis[0].register, "x0");
    assert_eq!(phis[0].incoming.len(), 2, "merges defs from both preds");
    assert!(phis[0].merged_version.starts_with("x0_phi"));
}
