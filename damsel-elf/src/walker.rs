//! Segment-only dynamic walker (Wave 5B.3): resolves dynamic symbols and
//! dynamic entries from `PT_DYNAMIC` + `PT_LOAD` when section headers are
//! absent or zeroed — the hard-stripped case both `object` and `goblin`
//! punt on (`object`'s own source carries the TODO; Research Sources §0
//! Round 3).
//!
//! Symbol count resolution order:
//! 1. `DT_HASH` — `nchain` equals the dynamic symbol count.
//! 2. GNU hash walk — highest bucket, then chain until the end marker.
//! 3. Layout heuristic — dynsym usually directly precedes `.dynstr`, so
//!    `(DT_STRTAB - DT_SYMTAB) / DT_SYMENT` bounds the table. This is
//!    the standard toolchain practice and is what the synthetic
//!    fixtures exercise.

use crate::image::{ElfDynamicEntry, ElfSymbol};

/// A raw `Elf64_Phdr` view (little-endian, ELF64).
#[derive(Debug, Clone, Copy)]
pub struct RawSegment {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_filesz: u64,
}

/// Parse all program headers.
pub fn parse_segments(bytes: &[u8], phoff: u64, phnum: u16) -> Option<Vec<RawSegment>> {
    let mut out = Vec::new();
    for index in 0..phnum as u64 {
        let base = phoff.checked_add(index * 56)? as usize;
        if base + 56 > bytes.len() {
            return None;
        }
        let read_u16 = |off: usize| u16::from_le_bytes([bytes[off], bytes[off + 1]]);
        let read_u32 = |off: usize| {
            u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
        };
        let read_u64 = |off: usize| {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[off..off + 8]);
            u64::from_le_bytes(buf)
        };
        out.push(RawSegment {
            p_type: read_u32(base),
            p_flags: read_u32(base + 4),
            p_offset: read_u64(base + 8),
            p_vaddr: read_u64(base + 16),
            p_filesz: read_u64(base + 32),
        });
        let _ = read_u16; // reserved for future PT_NOTE walking
    }
    Some(out)
}

/// Translate a virtual address to a file offset via PT_LOAD coverage.
pub fn vaddr_to_offset(segments: &[RawSegment], vaddr: u64) -> Option<u64> {
    segments
        .iter()
        .filter(|s| s.p_type == object::elf::PT_LOAD)
        .find(|s| vaddr >= s.p_vaddr && vaddr < s.p_vaddr + s.p_filesz)
        .map(|s| s.p_offset + (vaddr - s.p_vaddr))
}

struct RawDynamic {
    tag: u64,
    value: u64,
}

fn parse_dynamic_entries(bytes: &[u8], segments: &[RawSegment]) -> Option<Vec<RawDynamic>> {
    let dynamic = segments.iter().find(|s| s.p_type == object::elf::PT_DYNAMIC)?;
    let start = dynamic.p_offset as usize;
    let end = start.checked_add(dynamic.p_filesz as usize)?;
    if end > bytes.len() {
        return None;
    }
    let mut out = Vec::new();
    for chunk in bytes[start..end].chunks_exact(16) {
        let mut tag = [0u8; 8];
        let mut value = [0u8; 8];
        tag.copy_from_slice(&chunk[0..8]);
        value.copy_from_slice(&chunk[8..16]);
        let tag = u64::from_le_bytes(tag);
        let value = u64::from_le_bytes(value);
        out.push(RawDynamic { tag, value });
        if tag == object::elf::DT_NULL as u64 {
            break;
        }
    }
    Some(out)
}

fn read_strtab_bytes<'a>(bytes: &'a [u8], segments: &[RawSegment], strtab: u64, strsz: u64) -> Option<&'a [u8]> {
    let offset = vaddr_to_offset(segments, strtab)? as usize;
    let end = offset.checked_add(strsz as usize)?;
    if end > bytes.len() {
        return None;
    }
    Some(&bytes[offset..end])
}

fn string_at(strtab: &[u8], offset: u64) -> Option<String> {
    let start = offset as usize;
    if start >= strtab.len() {
        return None;
    }
    let end = strtab[start..]
        .iter()
        .position(|b| *b == 0)
        .map(|p| start + p)?;
    Some(String::from_utf8_lossy(&strtab[start..end]).into_owned())
}

/// Resolve the dynamic symbol count (documented order in the module docs).
fn dynamic_symbol_count(
    bytes: &[u8],
    segments: &[RawSegment],
    entries: &[RawDynamic],
) -> Option<usize> {
    let get = |tag: u64| entries.iter().find(|e| e.tag == tag).map(|e| e.value);
    let symtab = get(object::elf::DT_SYMTAB as u64)?;
    let syment = get(object::elf::DT_SYMENT as u64).filter(|v| *v == 24).unwrap_or(24);

    // 1. SysV hash: nbucket, nchain, … — nchain == symbol count.
    if let Some(hash) = get(object::elf::DT_HASH as u64) {
        if let Some(offset) = vaddr_to_offset(segments, hash) {
            let base = offset as usize;
            if base + 8 <= bytes.len() {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&bytes[base + 4..base + 8]);
                return Some(u32::from_le_bytes(buf) as usize);
            }
        }
    }

    // 2. GNU hash walk.
    if let Some(gnu_hash) = get(object::elf::DT_GNU_HASH as u64) {
        if let Some(count) = gnu_hash_symbol_count(bytes, segments, gnu_hash) {
            return Some(count);
        }
    }

    // 3. Layout heuristic: dynsym directly precedes dynstr.
    let strtab = get(object::elf::DT_STRTAB as u64)?;
    if strtab > symtab {
        let span = strtab - symtab;
        if span % syment == 0 {
            return Some((span / syment) as usize);
        }
    }
    None
}

fn gnu_hash_symbol_count(bytes: &[u8], segments: &[RawSegment], gnu_hash: u64) -> Option<usize> {
    let base = vaddr_to_offset(segments, gnu_hash)? as usize;
    let read_u32 = |off: usize| -> Option<u32> {
        let mut buf = [0u8; 4];
        if off + 4 > bytes.len() {
            return None;
        }
        buf.copy_from_slice(&bytes[off..off + 4]);
        Some(u32::from_le_bytes(buf))
    };
    let nbuckets = read_u32(base)? as usize;
    let symoffset = read_u32(base + 4)? as usize;
    let bloom_size = read_u32(base + 8)? as usize;
    let bloom_shift = read_u32(base + 12)? as usize;
    let buckets_off = base + 16 + bloom_size * 8;
    let chain_off = buckets_off + nbuckets * 4;

    let mut highest = 0usize;
    for index in 0..nbuckets {
        let bucket = read_u32(buckets_off + index * 4)? as usize;
        if bucket > highest {
            highest = bucket;
        }
    }
    if highest < symoffset {
        return None;
    }
    let mut last = highest;
    loop {
        let value = read_u32(chain_off + (last - symoffset) * 4)?;
        if value & 1 == 1 {
            break;
        }
        last += 1;
    }
    let _ = bloom_shift;
    Some(last + 1)
}

/// Segment-only dynamic info: symbols + entries of interest.
pub struct SegmentOnlyDynamic {
    pub symbols: Vec<ElfSymbol>,
    pub entries: Vec<ElfDynamicEntry>,
}

/// Walk `PT_DYNAMIC` for symbols and entries. Returns `None` when the
/// table is absent (static binaries) rather than failing.
pub fn walk_dynamic(bytes: &[u8], segments: &[RawSegment]) -> Option<SegmentOnlyDynamic> {
    let entries = parse_dynamic_entries(bytes, segments)?;
    let get = |tag: u64| entries.iter().find(|e| e.tag == tag).map(|e| e.value);
    let strtab_addr = get(object::elf::DT_STRTAB as u64)?;
    let strsz = get(object::elf::DT_STRSZ as u64)?;
    let strtab = read_strtab_bytes(bytes, segments, strtab_addr, strsz)?;

    let mut out_entries = Vec::new();
    for entry in &entries {
        let Some(text) = string_at(strtab, entry.value) else { continue };
        match entry.tag {
            t if t == object::elf::DT_NEEDED as u64 => out_entries.push(ElfDynamicEntry::Needed(text)),
            t if t == object::elf::DT_SONAME as u64 => out_entries.push(ElfDynamicEntry::Soname(text)),
            t if t == object::elf::DT_RPATH as u64 => out_entries.push(ElfDynamicEntry::Rpath(text)),
            t if t == object::elf::DT_RUNPATH as u64 => out_entries.push(ElfDynamicEntry::Runpath(text)),
            _ => {}
        }
    }

    let mut symbols = Vec::new();
    if let (Some(symtab_addr), Some(count)) = (
        get(object::elf::DT_SYMTAB as u64),
        dynamic_symbol_count(bytes, segments, &entries),
    ) {
        let versions = resolve_versions(bytes, segments, &entries, strtab, count);
        if let Some(symtab_off) = vaddr_to_offset(segments, symtab_addr) {
            for index in 0..count {
                let base = symtab_off as usize + index * 24;
                if base + 24 > bytes.len() {
                    break;
                }
                let read_u32 = |off: usize| {
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(&bytes[off..off + 4]);
                    u32::from_le_bytes(buf)
                };
                let read_u16 = |off: usize| {
                    u16::from_le_bytes([bytes[off], bytes[off + 1]])
                };
                let read_u64 = |off: usize| {
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&bytes[off..off + 8]);
                    u64::from_le_bytes(buf)
                };
                let name_off = read_u32(base) as u64;
                let st_info = bytes[base + 4];
                let shndx = read_u16(base + 6);
                let value = read_u64(base + 8);
                let size = read_u64(base + 16);
                if name_off == 0 && value == 0 && size == 0 {
                    continue; // null symbol
                }
                let Some(name) = string_at(strtab, name_off) else { continue };
                let defined = shndx != 0; // SHN_UNDEF
                let _ = st_info;
                symbols.push(ElfSymbol {
                    name,
                    address: value,
                    size,
                    defined,
                    is_text: defined,
                    dynamic: true,
                    is_import: !defined,
                    version: versions.get(index).cloned().flatten(),
                });
            }
        }
    }

    Some(SegmentOnlyDynamic {
        symbols,
        entries: out_entries,
    })
}

/// Resolve per-symbol version names from DT_VERSYM + DT_VERNEED
/// (`Elf64_Verneed`/`Elf64_Vernaux` walk; verdef handled on the
/// section-ful path by `object`'s VersionTable).
fn resolve_versions(
    bytes: &[u8],
    segments: &[RawSegment],
    entries: &[RawDynamic],
    strtab: &[u8],
    symbol_count: usize,
) -> Vec<Option<String>> {
    let mut out = vec![None; symbol_count];
    let get = |tag: u64| entries.iter().find(|e| e.tag == tag).map(|e| e.value);

    // Version-index → name map from DT_VERNEED.
    let mut by_index: std::collections::BTreeMap<u16, String> = Default::default();
    if let Some(verneed_addr) = get(object::elf::DT_VERNEED as u64) {
        if let Some(base) = vaddr_to_offset(segments, verneed_addr) {
            let read_u16 = |off: usize| -> Option<u16> {
                let off = off as usize;
                if off + 2 > bytes.len() {
                    return None;
                }
                Some(u16::from_le_bytes([bytes[off], bytes[off + 1]]))
            };
            let read_u32 = |off: usize| -> Option<u32> {
                let off = off as usize;
                if off + 4 > bytes.len() {
                    return None;
                }
                Some(u32::from_le_bytes([
                    bytes[off],
                    bytes[off + 1],
                    bytes[off + 2],
                    bytes[off + 3],
                ]))
            };
            // Walk the Verneed chain, then each Vernaux chain.
            let mut need_off = base as usize;
            for _ in 0..64 {
                let Some(next) = read_u32(need_off + 12) else { break };
                let Some(aux_offset) = read_u32(need_off + 8) else { break };
                let mut aux_off = need_off + aux_offset as usize;
                for _ in 0..256 {
                    let Some(aux_next) = read_u32(aux_off + 12) else { break };
                    let (Some(other), Some(name_off)) =
                        (read_u16(aux_off + 6), read_u32(aux_off + 8))
                    else {
                        break;
                    };
                    if let Some(name) = string_at(strtab, u64::from(name_off)) {
                        by_index.insert(other, name);
                    }
                    if aux_next == 0 {
                        break;
                    }
                    aux_off += aux_next as usize;
                }
                if next == 0 {
                    break;
                }
                need_off += next as usize;
            }
        }
    }

    // DT_VERSYM array: one u16 per dynamic symbol (mask the hidden bit).
    if let Some(versym_addr) = get(object::elf::DT_VERSYM as u64) {
        if let Some(base) = vaddr_to_offset(segments, versym_addr) {
            for (index, slot) in out.iter_mut().enumerate() {
                let off = base as usize + index * 2;
                if off + 2 > bytes.len() {
                    break;
                }
                let value = u16::from_le_bytes([bytes[off], bytes[off + 1]]) & 0x7fff;
                if value > 1 {
                    *slot = by_index.get(&value).cloned();
                }
            }
        }
    }
    out
}

/// Walk dynamic relocations (Elf64_Rela) from DT_JMPREL (PLT) and
/// DT_RELA tables: `{r_offset, r_info = sym<<32|type, r_addend}`.
pub fn walk_relocations(
    bytes: &[u8],
    segments: &[RawSegment],
    dynsym: &[(String, Option<String>)],
) -> Vec<crate::image::ElfRelocation> {
    use crate::image::{ElfRelocation, ElfRelocationSymbol};
    let mut out = Vec::new();
    let Some(entries) = parse_dynamic_entries(bytes, segments) else {
        return out;
    };
    let get = |tag: u64| entries.iter().find(|e| e.tag == tag).map(|e| e.value);

    let mut tables: Vec<(u64, u64, bool)> = Vec::new(); // (addr, size, is_plt)
    if let (Some(addr), Some(size)) = (
        get(0x17), // DT_JMPREL
        get(0x2),  // DT_PLTRELSZ
    ) {
        tables.push((addr, size, true));
    }
    if let (Some(addr), Some(size)) = (
        get(0x7), // DT_RELA
        get(0x8), // DT_RELASZ
    ) {
        tables.push((addr, size, false));
    }
    for (addr, size, _is_plt) in tables {
        let Some(start) = vaddr_to_offset(segments, addr) else { continue };
        let start = start as usize;
        let end = match start.checked_add(size as usize) {
            Some(end) if end <= bytes.len() => end,
            _ => continue,
        };
        for chunk in bytes[start..end].chunks_exact(24) {
            let read_u64 = |off: usize| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&chunk[off..off + 8]);
                u64::from_le_bytes(buf)
            };
            let offset = read_u64(0);
            let info = read_u64(8);
            let addend = read_u64(16) as i64;
            let r_type = (info & 0xffff_ffff) as u32;
            let sym_index = (info >> 32) as usize;
            let kind = match r_type {
                7 => "jump_slot",
                6 => "glob_dat",
                8 => "relative",
                37 => "irelative",
                _ => "other",
            }
            .to_string();
            let symbol = if sym_index > 0 {
                dynsym.get(sym_index).map(|(name, version)| {
                    ElfRelocationSymbol {
                        name: name.clone(),
                        version: version.clone(),
                    }
                })
            } else {
                None
            };
            out.push(ElfRelocation {
                offset,
                kind,
                symbol,
                addend,
            });
        }
    }
    out
}
