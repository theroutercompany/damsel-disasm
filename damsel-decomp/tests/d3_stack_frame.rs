//! Stack-frame facts contract tests over `_main` (real prologue shape).

mod support;

use damsel_core::{DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget};
use damsel_decomp::{analyze_stack_frame, lift_arm64};

#[test]
fn main_frame_facts_match_real_prologue() {
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
    let function = lift_arm64(&result.instructions);
    let facts = analyze_stack_frame(&function);

    // Real shape: sub sp,sp,#0x20 … stp fp,lr,[sp,#0x10] … stur wzr,
    // [fp,#-4] … ldp … add sp,sp,#0x20.
    assert_eq!(facts.frame_size, Some(0x20));
    assert_eq!(facts.epilogue_restore, Some(0x20));
    assert!(facts.saved_registers.contains(&"fp".to_string()), "fp saved: {:?}", facts.saved_registers);
    assert!(facts.saved_registers.contains(&"lr".to_string()), "lr saved: {:?}", facts.saved_registers);
    assert!(facts.local_slots.contains(&-4), "local at fp-4: {:?}", facts.local_slots);
    // Deterministic serialization.
    let a = serde_json::to_string(&facts).unwrap();
    let b = serde_json::to_string(&analyze_stack_frame(&function)).unwrap();
    assert_eq!(a, b);
}

#[test]
fn empty_function_has_no_frame_facts() {
    let facts = analyze_stack_frame(&lift_arm64(&[]));
    assert_eq!(facts.frame_size, None);
    assert!(facts.saved_registers.is_empty());
    assert!(facts.local_slots.is_empty());
}
