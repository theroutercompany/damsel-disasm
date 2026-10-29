//! x86-64 decoder mapper tests (Wave 5A.5): operands, references, and
//! RIP-relative data targets on authored byte sequences (the same bytes
//! the synthetic ELF fixtures embed).

use damsel_core::{DecodedInstruction, Operand, Reference, decode_x86_64};

fn find<'a>(instructions: &'a [DecodedInstruction], mnemonic: &str) -> &'a DecodedInstruction {
    instructions
        .iter()
        .find(|i| i.mnemonic == mnemonic)
        .unwrap_or_else(|| panic!("expected a `{mnemonic}` instruction"))
}

#[test]
fn decodes_two_function_blob_with_call_reference() {
    // The synthetic .text blob at base 0x40 (see damsel_elf::synth):
    // push rbp; mov rbp,rsp; call helper(+15); pop rbp; ret; nops; ret.
    let code: Vec<u8> = vec![
        0x55,
        0x48, 0x89, 0xE5,
        0xE8, 0x06, 0x00, 0x00, 0x00,
        0x5D,
        0xC3,
        0x90, 0x90, 0x90, 0x90,
        0xC3,
    ];
    let instructions = decode_x86_64(&code, 0x40, None).expect("decode blob");
    assert_eq!(instructions.len(), 10, "every byte decodes");

    // Direct call maps to a Call reference at the helper address.
    let call = find(&instructions, "call");
    assert_eq!(call.address, 0x44);
    assert_eq!(call.size, 5);
    assert!(call
        .references
        .iter()
        .any(|r| matches!(r, Reference::Call { target: 0x4F })));
    // The immediate operand renders as a Label (same vocabulary as arm64).
    assert!(call
        .operands
        .iter()
        .any(|o| matches!(o, Operand::Label(target) if *target == 0x4F)));

    // ret carries no references; push/pop map registers.
    let ret = find(&instructions, "ret");
    assert!(ret.references.is_empty());
    assert!(find(&instructions, "push")
        .operands
        .iter()
        .any(|o| matches!(o, Operand::Register(r) if r == "rbp")));
}

#[test]
fn rip_relative_lea_resolves_to_data_reference() {
    // lea rax, [rip+0x1234] at 0x1000 → target = next-ip 0x1007 + 0x1234 = 0x223B.
    let code: Vec<u8> = vec![0x48, 0x8D, 0x05, 0x34, 0x12, 0x00, 0x00];
    let instructions = decode_x86_64(&code, 0x1000, None).expect("decode lea");
    assert_eq!(instructions.len(), 1);
    let lea = &instructions[0];
    assert_eq!(lea.mnemonic, "lea");
    assert!(lea
        .references
        .iter()
        .any(|r| matches!(r, Reference::Data { target: 0x223B })));
    // Operand is memory with rip base.
    assert!(lea.operands.iter().any(|o| matches!(
        o,
        Operand::Memory { base, displacement: 0x1234, .. } if base == "rip"
    )));
}

#[test]
fn indirect_call_maps_via_register() {
    // call rax = FF D0
    let code: Vec<u8> = vec![0xFF, 0xD0];
    let instructions = decode_x86_64(&code, 0x2000, None).expect("decode call rax");
    let call = &instructions[0];
    assert!(call
        .references
        .iter()
        .any(|r| matches!(r, Reference::IndirectCall { via } if via == "rax")));
}

#[test]
fn limit_and_garbage_are_typed() {
    let code: Vec<u8> = vec![0x55, 0x48, 0x89, 0xE5];
    let limited = decode_x86_64(&code, 0, Some(1)).expect("decode with limit");
    assert_eq!(limited.len(), 1);
    // Undecodable stream: capstone's disasm_all skips invalid bytes, so
    // the contract is an empty result, not an error and not a panic.
    let bad = vec![0x06u8; 8];
    let empty = decode_x86_64(&bad, 0, None).expect("no panic on garbage");
    assert!(empty.is_empty());
}
