//! x86-64 lift + render contract tests on the synthetic ELF blob
//! (`damsel_elf::synth::text_two_functions` bytes at 0xE8).

use damsel_decomp::{IrCallTarget, IrStatement, IrTerminator, lift_x86_64, render_pseudocode};

#[test]
fn x86_blob_lifts_call_and_return() {
    let (code, _main, helper) = damsel_elf::synth::text_two_functions();
    let instructions = damsel_core::decode_x86_64(&code, 0x40, None).unwrap();
    let function = lift_x86_64(&instructions);

    // Two blocks: main (ret-terminated) and helper (ret-terminated).
    assert!(function
        .blocks
        .iter()
        .filter(|b| b.terminator == IrTerminator::Return)
        .count()
        >= 2);

    // The call lifts resolved to the helper address.
    let calls: Vec<&IrStatement> = function
        .blocks
        .iter()
        .flat_map(|b| b.statements.iter())
        .filter(|s| matches!(s, IrStatement::Call { .. }))
        .collect();
    assert!(calls.iter().any(|s| matches!(s,
        IrStatement::Call { target: IrCallTarget::Function(t), .. } if *t == 0x40 + helper)));

    // Register moves lift as assignments (push rbp → unknown v1, visible).
    assert!(function.blocks.iter().any(|b| b
        .statements
        .iter()
        .any(|s| matches!(s, IrStatement::Assign { destination, .. } if destination == "rbp"))));
    assert!(function.unknown_count > 0, "push/pop are visible unknowns in v1");

    // Renders with provenance and the call target name.
    let pseudocode = render_pseudocode(&function, Some("exported_func"));
    let texts: Vec<&str> = pseudocode.lines.iter().map(|l| l.text.as_str()).collect();
    assert!(texts.iter().any(|t| t.contains(&format!("sub_{:x}", 0x40 + helper))));
    assert!(texts.iter().any(|t| *t == "return;"));
    assert!(texts.iter().any(|t| t.contains("/* unknown: push */")));
}

#[test]
fn x86_empty_window_is_empty() {
    let function = lift_x86_64(&[]);
    assert!(function.blocks.is_empty());
    assert_eq!(function.instruction_count, 0);
}
