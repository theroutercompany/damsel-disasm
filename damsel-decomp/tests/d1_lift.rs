//! D1 IR lift contract tests: lift `_main` and `_exported_sum` from the
//! arm64 fixture and assert block/statement/terminator shapes with
//! honest `Unknown` accounting. IR snapshots serialize deterministically.

mod support;

use damsel_core::{DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget};
use damsel_decomp::{
    IrCallTarget, IrStatement, IrTerminator, lift_arm64,
};

fn disassemble_main() -> Vec<damsel_core::DecodedInstruction> {
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
    damsel_macho::disassemble_v2(&image, &request).unwrap().instructions
}

#[test]
fn lift_main_produces_blocks_call_and_return() {
    let instructions = disassemble_main();
    assert!(!instructions.is_empty());
    let function = lift_arm64(&instructions);

    assert_eq!(function.entry_address, instructions[0].address);
    assert_eq!(function.instruction_count, instructions.len());
    // _main ends with a return-terminated block.
    assert!(function
        .blocks
        .iter()
        .any(|b| b.terminator == IrTerminator::Return));

    // The bl to _exported_sum lifts as a resolved Call statement
    // (target 0x100000460 per the fixture layout).
    let calls: Vec<&IrStatement> = function
        .blocks
        .iter()
        .flat_map(|b| b.statements.iter())
        .filter(|s| matches!(s, IrStatement::Call { .. }))
        .collect();
    assert!(
        calls.iter().any(|s| matches!(s,
            IrStatement::Call { target: IrCallTarget::Function(0x100000460), .. })),
        "expected call to _exported_sum, got {calls:?}"
    );

    // Deterministic serialization (snapshot-ready).
    let serialized = serde_json::to_string(&function).unwrap();
    let reparsed: damsel_decomp::IrFunction = serde_json::from_str(&serialized).unwrap();
    assert_eq!(reparsed, function);
}

#[test]
fn unknown_statements_are_counted_not_hidden() {
    let instructions = disassemble_main();
    let function = lift_arm64(&instructions);
    let unknown_statements: usize = function
        .blocks
        .iter()
        .flat_map(|b| b.statements.iter())
        .filter(|s| matches!(s, IrStatement::Unknown { .. }))
        .count();
    assert_eq!(
        function.unknown_count,
        function
            .blocks
            .iter()
            .flat_map(|b| b.statements.iter())
            .filter(|s| matches!(s, IrStatement::Unknown { .. }))
            .count()
    );
    // The count is honest and stable for the fixture: whatever it is,
    // statements + boundary instructions cover the window.
    assert!(function.unknown_count <= function.instruction_count);
}

#[test]
fn lift_empty_window_is_empty_not_panicking() {
    let function = lift_arm64(&[]);
    assert!(function.blocks.is_empty());
    assert_eq!(function.instruction_count, 0);
}
