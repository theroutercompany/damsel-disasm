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
                    version: None,
                });
            }
        }
    }

    Some(SegmentOnlyDynamic {
        symbols,
        entries: out_entries,
    })
}
