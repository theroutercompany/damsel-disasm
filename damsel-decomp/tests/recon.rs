//! Recon triage contract tests.

use damsel_decomp::{Verdict, assess, detect_packers, shannon_entropy};

#[test]
fn entropy_of_uniform_data_is_zero() {
    assert_eq!(shannon_entropy(&[0u8; 64]), 0.0);
}

#[test]
fn entropy_of_random_data_is_high() {
    // Pseudo-random but deterministic bytes
    let data: Vec<u8> = (0..1024u32).map(|i| (i.wrapping_mul(2654435761) >> 16) as u8).collect();
    let entropy = shannon_entropy(&data);
    assert!(entropy > 7.0, "random data entropy {entropy:.2} should exceed 7.0");
}

#[test]
fn upx_magic_is_detected() {
    let data = b"\x00UPX! packed data here\x00".to_vec();
    let found = detect_packers(".text", &data);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "UPX");
    assert!(found[0].evidence.contains(".text"));
}

#[test]
fn clean_binary_gets_clean_verdict() {
    let sections = vec![
        (".text".to_string(), vec![0x48u8; 256]),
        (".rodata".to_string(), b"hello world".to_vec()),
    ];
    let imports = vec!["printf".to_string()];
    let strings = vec!["hello".to_string()];
    let report = assess(150, &sections, &imports, &strings);
    assert_eq!(report.verdict, Verdict::Clean);
    assert!(!report.stripped);
}

#[test]
fn ptrace_import_flags_needs_review() {
    let sections = vec![(".text".to_string(), vec![0x48u8; 64])];
    let imports = vec!["printf".to_string(), "ptrace".to_string()];
    let strings: Vec<String> = vec![];
    let report = assess(100, &sections, &imports, &strings);
    assert!(report.anti_analysis_indicators.iter().any(|i| i.name == "anti_debug_ptrace"));
    assert_eq!(report.verdict, Verdict::NeedsManualReview);
}

#[test]
fn upx_and_low_symbols_yield_likely_packed() {
    let sections = vec![(".text".to_string(), b"UPX!\x00\x01\x02".to_vec())];
    let imports: Vec<String> = vec![];
    let strings: Vec<String> = vec![];
    let report = assess(0, &sections, &imports, &strings);
    assert_eq!(report.verdict, Verdict::LikelyPacked);
    assert!(report.stripped);
}

#[test]
fn flattened_cfg_detected_by_dispatcher_indegree() {
    use damsel_decomp::detect_cfg_flattening;
    // Simulated flattened function: dispatcher at 0x100 with 8 state
    // blocks branching back to it.
    let bounds = vec![(0x100u64, 0x300u64)];
    let targets: Vec<u64> = std::iter::repeat_n(0x100u64, 8).collect();
    let found = detect_cfg_flattening(&bounds, &targets);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].dispatcher_address, 0x100);
    assert!(found[0].state_blocks >= 6);
    assert!(found[0].evidence.contains("8 incoming branches"));
}

#[test]
fn normal_cfg_not_flagged() {
    use damsel_decomp::detect_cfg_flattening;
    let bounds = vec![(0x100u64, 0x300u64)];
    let targets = vec![0x200u64, 0x100u64, 0x180u64, 0x220u64]; // spread out
    let found = detect_cfg_flattening(&bounds, &targets);
    assert!(found.is_empty(), "no dispatcher with >=6 in-degree");
}

#[test]
fn opaque_predicate_detected_from_constant_flags() {
    use damsel_decomp::{IrBlock, IrCondition, IrExpr, IrFunction, IrStatement, IrTerminator, IrValue, detect_opaque_predicates};

    // flags = (5 - 5) → constant zero → conditional branch is opaque
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![IrBlock {
            id: 0,
            start_address: 0x100,
            statements: vec![IrStatement::Assign {
                destination: "flags".into(),
                expression: IrExpr::BinOp {
                    op: damsel_decomp::IrBinOp::Sub,
                    lhs: Box::new(IrExpr::Value(IrValue::Const(5))),
                    rhs: Box::new(IrExpr::Value(IrValue::Const(5))),
                },
            }],
            terminator: IrTerminator::CondBranch {
                condition: IrCondition::Raw("b.eq".into()),
                taken: 0x200,
                fallthrough: 0x180,
            },
        }],
        instruction_count: 2,
        unknown_count: 0,
    };
    let found = detect_opaque_predicates(&function);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].branch_address, 0x100);
    assert!(found[0].evidence.contains("constant operands"));
}

#[test]
fn variable_condition_not_flagged_as_opaque() {
    use damsel_decomp::{IrBlock, IrCondition, IrExpr, IrFunction, IrStatement, IrTerminator, IrValue, detect_opaque_predicates};

    // flags = (x0 - 5) → variable → NOT opaque
    let function = IrFunction {
        entry_address: 0,
        blocks: vec![IrBlock {
            id: 0,
            start_address: 0x100,
            statements: vec![IrStatement::Assign {
                destination: "flags".into(),
                expression: IrExpr::BinOp {
                    op: damsel_decomp::IrBinOp::Sub,
                    lhs: Box::new(IrExpr::Value(IrValue::Register("x0".into()))),
                    rhs: Box::new(IrExpr::Value(IrValue::Const(5))),
                },
            }],
            terminator: IrTerminator::CondBranch {
                condition: IrCondition::Raw("b.eq".into()),
                taken: 0x200,
                fallthrough: 0x180,
            },
        }],
        instruction_count: 2,
        unknown_count: 0,
    };
    assert!(detect_opaque_predicates(&function).is_empty());
}

#[test]
fn string_encryption_detected_by_xor_and_entropy() {
    use damsel_decomp::{SectionEntropy, detect_string_encryption};
    let entropies = vec![SectionEntropy { section: ".data".into(), entropy: 7.2 }];
    let found = detect_string_encryption("decrypt_strings", 5, 3, &entropies);
    assert_eq!(found.len(), 1);
    assert!(found[0].evidence.contains("5 xor/shift ops"));
}

#[test]
fn clean_function_not_flagged_for_encryption() {
    use damsel_decomp::{SectionEntropy, detect_string_encryption};
    let entropies = vec![SectionEntropy { section: ".rodata".into(), entropy: 3.5 }];
    assert!(detect_string_encryption("normal_fn", 1, 1, &entropies).is_empty());
}
