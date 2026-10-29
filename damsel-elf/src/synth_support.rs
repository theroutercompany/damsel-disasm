// Synthetic ELF64 builders (inlined test-support module; layout: vaddr == file offset).

/// x86-64 code: `main_func` (16 bytes) calls `helper_func` (ret).
/// Returns (code, main_offset_in_code, helper_offset_in_code).
pub fn text_two_functions() -> (Vec<u8>, u64, u64) {
    // Layout (16 bytes total): main = [0,15): push rbp; mov rbp,rsp;
    // call helper; pop rbp; ret; 4 nops. helper = byte 15: ret.
    // call at 4..9, next-ip 9, target 15 → rel32 = 6.
    let code = vec![
        0x55, // push rbp
        0x48, 0x89, 0xE5, // mov rbp,rsp
        0xE8, 0x06, 0x00, 0x00, 0x00, // call helper (rel32 = 6)
        0x5D, // pop rbp
        0xC3, // ret
        0x90, 0x90, 0x90, 0x90, // nop padding (indices 11..15)
        0xC3, // helper (index 15): ret
    ];
    (code, 0, 15)
}

pub fn build_id_note() -> Vec<u8> {
    let mut note = Vec::new();
    note.extend((4u32).to_le_bytes());
    note.extend((8u32).to_le_bytes());
    note.extend((3u32).to_le_bytes());
    note.extend_from_slice(b"GNU\0");
    note.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x11, 0x22, 0x33]);
    note
}

pub fn sym_defined(name_off: u32, value: u64, size: u64, shndx: u16) -> [u8; 24] {
    let mut out = [0u8; 24];
    out[0..4].copy_from_slice(&name_off.to_le_bytes());
    out[4] = 0x12; // STB_GLOBAL | STT_FUNC
    out[5] = 0;
    out[6..8].copy_from_slice(&shndx.to_le_bytes());
    out[8..16].copy_from_slice(&value.to_le_bytes());
    out[16..24].copy_from_slice(&size.to_le_bytes());
    out
}

pub fn sym_undefined(name_off: u32) -> [u8; 24] {
    sym_defined(name_off, 0, 0, 0)
}

/// strtab with a leading NUL; returns (data, offsets in order).
pub fn strtab_of(strings: &[&str]) -> (Vec<u8>, Vec<u32>) {
    let mut data = vec![0u8];
    let mut offsets = Vec::new();
    for s in strings {
        offsets.push(data.len() as u32);
        data.extend_from_slice(s.as_bytes());
        data.push(0);
    }
    (data, offsets)
}

fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

fn ehdr(e_type: u16, phoff: u64, phnum: u16, shoff: u64, shnum: u16, shstrndx: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    let mut ident = [0u8; 16];
    ident[..4].copy_from_slice(b"\x7fELF");
    ident[4] = 2;
    ident[5] = 1;
    ident[6] = 1;
    out.extend_from_slice(&ident);
    out.extend(e_type.to_le_bytes());
    out.extend((62u16).to_le_bytes()); // EM_X86_64
    out.extend((1u32).to_le_bytes());
    out.extend((0u64).to_le_bytes()); // entry
    out.extend(phoff.to_le_bytes());
    out.extend(shoff.to_le_bytes());
    out.extend((0u32).to_le_bytes());
    out.extend((64u16).to_le_bytes());
    out.extend((56u16).to_le_bytes());
    out.extend(phnum.to_le_bytes());
    out.extend((64u16).to_le_bytes());
    out.extend(shnum.to_le_bytes());
    out.extend(shstrndx.to_le_bytes());
    out
}

fn phdr(p_type: u32, p_flags: u32, offset: u64, size: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(56);
    out.extend(p_type.to_le_bytes());
    out.extend(p_flags.to_le_bytes());
    out.extend(offset.to_le_bytes());
    out.extend(offset.to_le_bytes()); // vaddr == offset
    out.extend(offset.to_le_bytes()); // paddr
    out.extend(size.to_le_bytes()); // filesz
    out.extend(size.to_le_bytes()); // memsz
    out.extend((8u64).to_le_bytes());
    out
}

fn shdr(
    name: u32,
    sh_type: u32,
    flags: u64,
    offset: u64,
    size: u64,
    link: u32,
    entsize: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend(name.to_le_bytes());
    out.extend(sh_type.to_le_bytes());
    out.extend(flags.to_le_bytes());
    out.extend(offset.to_le_bytes()); // addr == offset
    out.extend(offset.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(link.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend((1u64).to_le_bytes());
    out.extend(entsize.to_le_bytes());
    out
}

/// Minimal ET_EXEC static binary: `.text` (two functions),
/// `.note.gnu.build-id`, `.symtab`/`.strtab` (`main_func`, `helper_func`).
/// No program headers (phnum 0) — sections carry the truth for this
/// fixture; segment coverage is exercised by `shared_object`.
pub fn static_exec() -> Vec<u8> {
    let (text, _m, _h) = text_two_functions();
    let (strtab, offs) = strtab_of(&["main_func", "helper_func"]);
    let note = build_id_note();

    // Layout (vaddr == file offset).
    let text_off = 0x40;
    let note_off = text_off + text.len();
    let note_off = align_up(note_off, 4);
    let strtab_off = note_off + note.len();
    let symtab_off = strtab_off + strtab.len();
    let symtab_len = 24 * 3;
    let shstrtab_off = symtab_off + symtab_len;

    let (shstrtab, names) = strtab_of(&[".text", ".note.gnu.build-id", ".symtab", ".strtab", ".shstrtab"]);
    let shoff = shstrtab_off + shstrtab.len();

    let mut file = ehdr(2, 0, 0, shoff as u64, 6, 5);
    debug_assert_eq!(file.len(), 0x40);
    file.resize(text_off, 0);
    file.extend_from_slice(&text);
    file.resize(note_off, 0);
    file.extend_from_slice(&note);
    file.resize(strtab_off, 0);
    file.extend_from_slice(&strtab);
    file.resize(symtab_off, 0);
    file.extend_from_slice(&[0u8; 24]); // null symbol
    file.extend_from_slice(&sym_defined(offs[0], text_off as u64, 15, 1));
    file.extend_from_slice(&sym_defined(offs[1], (text_off + 15) as u64, 1, 1));
    file.resize(shstrtab_off, 0);
    file.extend_from_slice(&shstrtab);
    file.resize(shoff, 0);
    file.extend_from_slice(&[0u8; 64]); // null section
    file.extend_from_slice(&shdr(names[0], 1, 0x6, text_off as u64, text.len() as u64, 0, 0));
    file.extend_from_slice(&shdr(names[1], 7, 0x2, note_off as u64, note.len() as u64, 0, 0));
    file.extend_from_slice(&shdr(names[2], 2, 0, symtab_off as u64, symtab_len as u64, 4, 24)); // link=.strtab
    file.extend_from_slice(&shdr(names[3], 3, 0, strtab_off as u64, strtab.len() as u64, 0, 0));
    file.extend_from_slice(&shdr(names[4], 3, 0, shstrtab_off as u64, shstrtab.len() as u64, 0, 0));
    file
}

/// Hard-stripped variant of `shared_object`: section headers zeroed
/// (e_shoff = e_shnum = e_shstrndx = 0) — the `elf-nosections` case only
/// the segment-only walker can parse.
pub fn elf_nosections() -> Vec<u8> {
    let mut bytes = shared_object();
    // ehdr layout: … e_shoff at 0x28, e_flags 0x30, … e_shnum at 0x3C,
    // e_shstrndx at 0x3E (ELF64 little-endian).
    bytes[0x28..0x30].fill(0); // e_shoff = 0
    bytes[0x3C..0x3E].fill(0); // e_shnum = 0
    bytes[0x3E..0x40].fill(0); // e_shstrndx = 0
    bytes
}

/// Minimal ET_DYN shared object: PT_LOAD + PT_DYNAMIC, `.text`,
/// `.dynstr`/`.dynsym` (`exported_func` defined, `malloc` import),
/// `.dynamic` with DT_SONAME/DT_NEEDED/DT_STRTAB/DT_STRSZ/DT_SYMTAB.
pub fn shared_object() -> Vec<u8> {
    let (text, _m, _h) = text_two_functions();
    let (dynstr, offs) = strtab_of(&[
        "exported_func", "malloc", "libfake.so.1", "fake.so.1",
        "GLIBC_2.2.5", "FAKE_1.0",
    ]);

    // Layout (standard toolchain order: dynsym directly precedes dynstr
    // so the segment-only walker's strtab-adjacency heuristic resolves).
    let phoff = 0x40usize;
    let phnum = 3usize; // PT_LOAD, PT_DYNAMIC, PT_NOTE
    let text_off = phoff + phnum * 56;
    let rodata_off = text_off + text.len();
    let rodata = b"hello from damsel elf\0".to_vec();
    let note_off = rodata_off + rodata.len();
    let note = build_id_note();
    let dynsym_off = note_off + note.len();
    let dynsym_len = 24 * 3;
    let dynstr_off = dynsym_off + dynsym_len;
    // .gnu.version: [0 (null), 2 (exported_func -> FAKE_1.0), 1 (malloc -> GLIBC_2.2.5)]
    let versym_off = dynstr_off + dynstr.len();
    let versym: Vec<u8> = [0u16, 3, 2]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    // .gnu.version_r: one Verneed (file libfake.so.1, 2 aux: idx2 FAKE_1.0, idx1 GLIBC_2.2.5)
    let verneed_off = versym_off + versym.len();
    let mut verneed: Vec<u8> = Vec::new();
    let push_u16 = |v: u16, out: &mut Vec<u8>| out.extend(v.to_le_bytes());
    let push_u32 = |v: u32, out: &mut Vec<u8>| out.extend(v.to_le_bytes());
    // Elf64_Verneed @0: version=1, cnt=2, file=offs[2], aux=16, next=0
    push_u16(1, &mut verneed);
    push_u16(2, &mut verneed);
    push_u32(offs[2] as u32, &mut verneed);
    push_u32(16, &mut verneed);
    push_u32(0, &mut verneed);
    // Elf64_Vernaux @16: hash=0, flags=0, other=3, name=offs[5], next=16
    push_u32(0, &mut verneed);
    push_u16(0, &mut verneed);
    push_u16(3, &mut verneed);
    push_u32(offs[5] as u32, &mut verneed);
    push_u32(16, &mut verneed);
    // Elf64_Vernaux @32: hash=0, flags=0, other=2, name=offs[4], next=0
    push_u32(0, &mut verneed);
    push_u16(0, &mut verneed);
    push_u16(2, &mut verneed);
    push_u32(offs[4] as u32, &mut verneed);
    push_u32(0, &mut verneed);
    // .rela.plt: one R_X86_64_JUMP_SLOT for malloc at GOT slot 0x3000.
    // sym index 2 (malloc), type 7: r_info = (2 << 32) | 7.
    let rela_off = verneed_off + verneed.len();
    let rela: Vec<u8> = [0x3000u64, (2u64 << 32) | 7, 0i64 as u64]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let dynamic_off = rela_off + rela.len();
    let mut dynamic: Vec<u8> = Vec::new();
    // dynamic entries gain DT_VERSYM/DT_VERNEED so the walker path resolves.
    let mut dt = |tag: u64, val: u64, out: &mut Vec<u8>| {
        out.extend(tag.to_le_bytes());
        out.extend(val.to_le_bytes());
    };
    dt(1, u64::from(offs[2]), &mut dynamic); // DT_NEEDED
    dt(14, u64::from(offs[3]), &mut dynamic); // DT_SONAME
    dt(5, dynstr_off as u64, &mut dynamic); // DT_STRTAB
    dt(10, dynstr.len() as u64, &mut dynamic); // DT_STRSZ
    dt(6, dynsym_off as u64, &mut dynamic); // DT_SYMTAB
    dt(11, 24, &mut dynamic); // DT_SYMENT
    dt(23, rela_off as u64, &mut dynamic); // DT_JMPREL
    dt(2, rela.len() as u64, &mut dynamic); // DT_PLTRELSZ
    dt(20, 7, &mut dynamic); // DT_PLTREL = RELA
    dt(0x6ffffff0, versym_off as u64, &mut dynamic); // DT_VERSYM
    dt(0x6ffffffe, verneed_off as u64, &mut dynamic); // DT_VERNEED
    dt(0, 0, &mut dynamic); // DT_NULL
    let shstrtab_off = dynamic_off + dynamic.len();
    let (shstrtab, names) = strtab_of(&[
        ".text", ".rodata", ".dynsym", ".dynstr", ".gnu.version",
        ".gnu.version_r", ".rela.plt", ".dynamic", ".shstrtab",
    ]);
    let shoff = shstrtab_off + shstrtab.len();

    let mut file = ehdr(3, phoff as u64, phnum as u16, shoff as u64, 10, 9);
    debug_assert_eq!(file.len(), phoff);
    file.extend(phdr(1, 5, 0, shoff as u64)); // PT_LOAD R|X covering headers+data
    file.extend(phdr(2, 6, dynamic_off as u64, dynamic.len() as u64)); // PT_DYNAMIC RW
    file.extend(phdr(4, 4, note_off as u64, note.len() as u64)); // PT_NOTE R
    file.resize(text_off, 0);
    file.extend_from_slice(&text);
    file.resize(rodata_off, 0);
    file.extend_from_slice(&rodata);
    file.resize(note_off, 0);
    file.extend_from_slice(&note);
    file.resize(dynsym_off, 0);
    file.extend_from_slice(&[0u8; 24]);
    file.extend_from_slice(&sym_defined(offs[0], text_off as u64, 15, 1));
    file.extend_from_slice(&sym_undefined(offs[1]));
    file.resize(dynstr_off, 0);
    file.extend_from_slice(&dynstr);
    file.resize(versym_off, 0);
    file.extend_from_slice(&versym);
    file.resize(verneed_off, 0);
    file.extend_from_slice(&verneed);
    file.resize(rela_off, 0);
    file.extend_from_slice(&rela);
    file.resize(dynamic_off, 0);
    file.extend_from_slice(&dynamic);
    file.resize(shstrtab_off, 0);
    file.extend_from_slice(&shstrtab);
    file.resize(shoff, 0);
    file.extend_from_slice(&[0u8; 64]);
    file.extend_from_slice(&shdr(names[0], 1, 0x6, text_off as u64, text.len() as u64, 0, 0)); // .text
    file.extend_from_slice(&shdr(names[1], 1, 0x2, rodata_off as u64, rodata.len() as u64, 0, 0)); // .rodata
    file.extend_from_slice(&shdr(names[2], 11, 0x2, dynsym_off as u64, dynsym_len as u64, 4, 24)); // .dynsym, link=.dynstr (idx 4)
    file.extend_from_slice(&shdr(names[3], 3, 0x2, dynstr_off as u64, dynstr.len() as u64, 0, 0)); // .dynstr
    file.extend_from_slice(&shdr(names[4], 0x6fffffff, 0x2, versym_off as u64, versym.len() as u64, 3, 2)); // .gnu.version, link=.dynsym (idx 3)
    file.extend_from_slice(&shdr(names[5], 0x6ffffffe, 0x2, verneed_off as u64, verneed.len() as u64, 3, 0)); // .gnu.version_r, link=.dynsym (idx 3)
    file.extend_from_slice(&shdr(
        names[6],
        4, // SHT_RELA
        0x2,
        rela_off as u64,
        rela.len() as u64,
        4, // link=.dynstr (idx 4)
        24,
    )); // .rela.plt
    file.extend_from_slice(&shdr(
        names[7],
        6,
        0x3,
        dynamic_off as u64,
        dynamic.len() as u64,
        4, // link=.dynstr (idx 4)
        16,
    )); // .dynamic
    file.extend_from_slice(&shdr(names[8], 3, 0, shstrtab_off as u64, shstrtab.len() as u64, 0, 0)); // .shstrtab
    file
}
