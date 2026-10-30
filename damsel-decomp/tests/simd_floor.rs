//! SIMD floor-set tests (Wave 6 D-4 quality): SSE2 instructions used
//! by glibc strlen/strcmp/memcpy fast paths lift to readable IR
//! instead of Unknown.

use damsel_core::{DecodedInstruction, Operand};
use damsel_decomp::lift_x86_64;

fn simd_instruction(address: u64, mnemonic: &str, operands: Vec<Operand>) -> DecodedInstruction {
    DecodedInstruction {
        address,
        size: 4,
        opcode: 0,
        mnemonic: mnemonic.to_string(),
        operands,
        recovered_values: Vec::new(),
        references: Vec::new(),
        annotations: Vec::new(),
    }
}

fn reg(name: &str) -> Operand {
    Operand::Register(name.to_string())
}

fn mem(base: &str, displacement: i64) -> Operand {
    Operand::Memory {
        base: base.to_string(),
        index: None,
        index_modifier: None,
        displacement,
        writeback: None,
    }
}

#[test]
fn movdqa_reg_to_reg_is_assign() {
    let inst = simd_instruction(0x1000, "movdqa", vec![reg("xmm0"), reg("xmm1")]);
    let func = lift_x86_64(&[inst]);
    assert_eq!(func.unknown_count, 0);
    let text = format!("{:?}", func.blocks[0].statements[0]);
    assert!(text.contains("xmm0"), "statement: {text}");
}

#[test]
fn movdqa_memory_load_is_load() {
    let inst = simd_instruction(0x1000, "movdqu", vec![reg("xmm0"), mem("rdi", 0)]);
    let func = lift_x86_64(&[inst]);
    assert_eq!(func.unknown_count, 0, "movdqu from memory should not be unknown");
}

#[test]
fn pxor_self_is_zero() {
    let inst = simd_instruction(0x1000, "pxor", vec![reg("xmm0"), reg("xmm0")]);
    let func = lift_x86_64(&[inst]);
    assert_eq!(func.unknown_count, 0);
    let text = format!("{:?}", func.blocks[0].statements[0]);
    assert!(text.contains("Const(0)"), "expected zero, got: {text}");
}

#[test]
fn pcmpeqb_is_compare_not_unknown() {
    let inst = simd_instruction(0x1000, "pcmpeqb", vec![reg("xmm0"), reg("xmm1")]);
    let func = lift_x86_64(&[inst]);
    assert_eq!(func.unknown_count, 0, "pcmpeqb should not be unknown");
    let text = format!("{:?}", func.blocks[0].statements[0]);
    assert!(text.contains("CmpEq"), "expected CmpEq, got: {text}");
}

#[test]
fn pcmpgtb_is_greater_than() {
    let inst = simd_instruction(0x1000, "pcmpgtb", vec![reg("xmm0"), reg("xmm1")]);
    let func = lift_x86_64(&[inst]);
    assert_eq!(func.unknown_count, 0);
    let text = format!("{:?}", func.blocks[0].statements[0]);
    assert!(text.contains("CmpGt"), "expected CmpGt, got: {text}");
}

#[test]
fn pmovmskb_is_load_not_unknown() {
    let inst = simd_instruction(0x1000, "pmovmskb", vec![reg("eax"), reg("xmm0")]);
    let func = lift_x86_64(&[inst]);
    assert_eq!(func.unknown_count, 0, "pmovmskb should not be unknown");
}

#[test]
fn ptest_sets_flags_via_and() {
    let inst = simd_instruction(0x1000, "ptest", vec![reg("xmm0"), reg("xmm1")]);
    let func = lift_x86_64(&[inst]);
    assert_eq!(func.unknown_count, 0);
    let text = format!("{:?}", func.blocks[0].statements[0]);
    assert!(text.contains("flags") && text.contains("And"), "got: {text}");
}

#[test]
fn strlen_sse2_pattern_decompiles() {
    // Core loop pattern from __strlen_sse2:
    //   movdqa xmm1, [rdi]      ; load 16 bytes
    //   pxor   xmm2, xmm2       ; zero
    //   pcmpeqb xmm1, xmm2      ; compare against zero
    //   pmovmskb eax, xmm1      ; extract mask
    //   test   eax, eax         ; check for zero byte
    let insts = vec![
        simd_instruction(0x1000, "movdqa", vec![reg("xmm1"), mem("rdi", 0)]),
        simd_instruction(0x1004, "pxor", vec![reg("xmm2"), reg("xmm2")]),
        simd_instruction(0x1008, "pcmpeqb", vec![reg("xmm1"), reg("xmm2")]),
        simd_instruction(0x100c, "pmovmskb", vec![reg("eax"), reg("xmm1")]),
        simd_instruction(0x1010, "test", vec![reg("eax"), reg("eax")]),
    ];
    let func = lift_x86_64(&insts);
    assert_eq!(
        func.unknown_count, 0,
        "the entire SSE2 strlen pattern should have zero unknowns"
    );
}
