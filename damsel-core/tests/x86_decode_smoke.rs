//! Wave 5A.3 in-workspace proof: with the `arch_x86` feature enabled, the
//! capstone x86 backend decodes and exposes detail (implicit registers
//! including `rflags`, RIP-relative base) — replicating the research
//! spike (Research Sources §0 Round 3) inside the workspace.

#[test]
fn capstone_x86_decodes_with_detail_in_workspace() {
    use capstone::arch::ArchDetail;
    use capstone::arch::BuildsCapstone;
    use capstone::prelude::*;

    // push rbp; mov rbp,rsp; lea rax,[rip+0x1234]; call rax; add rsp,0x20;
    // cmp eax,0x10; jne -0x1c; ret
    let code: Vec<u8> = vec![
        0x55,
        0x48, 0x89, 0xE5,
        0x48, 0x8D, 0x05, 0x34, 0x12, 0x00, 0x00,
        0xFF, 0xD0,
        0x48, 0x83, 0xC4, 0x20,
        0x83, 0xF8, 0x10,
        0x75, 0xE4,
        0xC3,
    ];
    let cs = Capstone::new()
        .x86()
        .mode(capstone::arch::x86::ArchMode::Mode64)
        .detail(true)
        .build()
        .expect("capstone x86 backend available (arch_x86 feature)");
    let insns = cs.disasm_all(&code, 0x1000).expect("decode spike window");

    let mut saw_rflags_write = false;
    let mut saw_rip_read = false;
    for insn in insns.iter() {
        let detail = cs.insn_detail(&insn).unwrap();
        let regs_write: Vec<_> = detail
            .regs_write()
            .iter()
            .filter_map(|r| cs.reg_name(*r))
            .collect();
        let regs_read: Vec<_> = detail
            .regs_read()
            .iter()
            .filter_map(|r| cs.reg_name(*r))
            .collect();
        if regs_write.iter().any(|r| r == "rflags") {
            saw_rflags_write = true;
        }
        if regs_read.iter().any(|r| r == "rip") {
            saw_rip_read = true;
        }
        if let ArchDetail::X86Detail(x86) = detail.arch_detail() {
            // Detail parses for every instruction.
            let _ = x86.operands().count();
        }
    }
    assert!(saw_rflags_write, "add/cmp must report implicit rflags write");
    assert!(saw_rip_read, "lea [rip+…] must report implicit rip read");
}
