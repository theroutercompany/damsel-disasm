//! `.eh_frame` FDE parsing: recovers function start addresses and
//! extents from unwind info — the primary function-discovery seed for
//! stripped static binaries (each FDE describes one function's frame).



/// A function extent recovered from an FDE record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdeEntry {
    /// Function start address (pc_begin).
    pub start: u64,
    /// Function end address (start + pc_range).
    pub end: u64,
}

/// Parse `.eh_frame` section bytes into function extents.
///
/// Walks CIE/FDE records. CIEs (CIE_pointer == 0) are skipped — their
/// encoding details only matter for augmentation handling; the common
/// DW_EH_PE_sdata4 pcrel encoding covers the vast majority of binaries.
pub fn parse_eh_frame(data: &[u8], section_vaddr: u64) -> Vec<FdeEntry> {
    let mut entries = Vec::new();
    let mut offset = 0usize;

    while offset + 8 <= data.len() {
        // Record length (4 bytes; 0 = end).
        let length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;

        if length == 0 {
            break; // end-of-list marker
        }
        if length == 0xFFFF_FFFF {
            break; // 64-bit length — not handled in v1
        }

        let record_start = offset;
        let record_end = offset + 4 + length;
        if record_end > data.len() {
            break;
        }

        // CIE_pointer: 0 = CIE, non-zero = FDE.
        let cie_pointer = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);

        if cie_pointer != 0 {
            // FDE: pc_begin (4-byte PC-relative) + pc_range (4-byte).
            let pcrel_field_offset = record_start + 8;
            if pcrel_field_offset + 8 <= record_end {
                let pcrel = i32::from_le_bytes([
                    data[pcrel_field_offset],
                    data[pcrel_field_offset + 1],
                    data[pcrel_field_offset + 2],
                    data[pcrel_field_offset + 3],
                ]);
                let range = u32::from_le_bytes([
                    data[pcrel_field_offset + 4],
                    data[pcrel_field_offset + 5],
                    data[pcrel_field_offset + 6],
                    data[pcrel_field_offset + 7],
                ]);

                // pc_begin is PC-relative to the field's own address.
                let field_vaddr = section_vaddr + pcrel_field_offset as u64;
                let start = (field_vaddr as i64 + pcrel as i64) as u64;
                let end = start + range as u64;

                // Accept any nonzero-extent function; FDEs describe
                // .text functions which live at lower addresses than
                // .eh_frame itself.
                if range > 0 && start > 0 {
                    entries.push(FdeEntry { start, end });
                }
            }
        }
        // else: CIE — skip (its encoding details don't block FDE
        // parsing for the common sdata4 case).

        offset = record_end;
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32b(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }

    /// Build a minimal .eh_frame with one CIE and one FDE.
    fn build_frame(cie_len: usize, fde_pc_offset: i32, fde_range: u32) -> Vec<u8> {
        let mut data = Vec::new();

        // CIE: length + CIE_id(0) + minimal body
        let cie_body = 4 + cie_len; // CIE_id + filler
        data.extend_from_slice(&u32b(cie_body as u32));
        data.extend_from_slice(&u32b(0)); // CIE_id = 0
        data.extend(std::iter::repeat(0u8).take(cie_len));

        // FDE: length + CIE_pointer(non-zero) + pc_begin(pcrel) + pc_range
        let fde_body = 4 + 4 + 4; // CIE_ptr + pc_begin + pc_range
        data.extend_from_slice(&u32b(fde_body as u32));
        data.extend_from_slice(&u32b(4)); // CIE_pointer = 4 (points back)
        data.extend_from_slice(&u32b(fde_pc_offset as u32));
        data.extend_from_slice(&u32b(fde_range));

        // End marker
        data.extend_from_slice(&u32b(0));
        data
    }

    #[test]
    fn parses_simple_fde() {
        // CIE total = 4(len) + 4(id) + 8(body) = 16 bytes.
        // FDE pc_begin field at absolute offset 16 + 8 = 24.
        // start = section + 24 + 0x100, range = 0x50.
        let data = build_frame(8, 0x100, 0x50);
        let entries = parse_eh_frame(&data, 0x1000);
        assert_eq!(entries.len(), 1);
        let expected_start = 0x1000 + 24 + 0x100;
        assert_eq!(entries[0].start, expected_start);
        assert_eq!(entries[0].end, expected_start + 0x50);
    }

    #[test]
    fn skips_cies() {
        // A frame with only a CIE and no FDE.
        let data = build_frame(8, 0, 0);
        // The FDE with range=0 is skipped (zero-extent)
        let entries = parse_eh_frame(&data, 0x1000);
        assert!(entries.is_empty());
    }

    #[test]
    fn handles_empty_input() {
        assert!(parse_eh_frame(&[], 0x1000).is_empty());
    }

    #[test]
    fn handles_truncated() {
        // Truncated record (claims more bytes than available).
        let mut data = Vec::new();
        data.extend_from_slice(&u32b(1000)); // claims 1000 bytes
        data.extend_from_slice(&u32b(4)); // FDE
        data.extend_from_slice(&[0u8; 8]); // truncated body
        assert!(parse_eh_frame(&data, 0x1000).is_empty());
    }

    #[test]
    fn multiple_fdes() {
        let mut data = Vec::new();
        // CIE: length = CIE_id(4) + body(8) = 12
        data.extend_from_slice(&u32b(12));
        data.extend_from_slice(&u32b(0)); // CIE_id
        data.extend_from_slice(&[0u8; 8]);

        // FDE 1: start=0x100, range=0x20
        let fde1_field_addr = 0x1000 + data.len() + 8;
        data.extend_from_slice(&u32b(12));
        data.extend_from_slice(&u32b(4));
        data.extend_from_slice(&u32b((0x1000 + 0x100 - fde1_field_addr) as u32));
        data.extend_from_slice(&u32b(0x20));

        // FDE 2: start=0x200, range=0x40
        let fde2_field_addr = 0x1000 + data.len() + 8;
        data.extend_from_slice(&u32b(12));
        data.extend_from_slice(&u32b(4));
        data.extend_from_slice(&u32b((0x1000 + 0x200 - fde2_field_addr) as u32));
        data.extend_from_slice(&u32b(0x40));

        data.extend_from_slice(&u32b(0)); // end

        let entries = parse_eh_frame(&data, 0x1000);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].start, 0x1100);
        assert_eq!(entries[1].start, 0x1200);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn static_binary_fde_recovery() {
        // If a local static test binary exists, verify FDE parsing works
        // on real compiler output. Skipped when the fixture is absent.
        let path = "/tmp/lab/test_static";
        if !std::path::Path::new(path).exists() {
            return; // skip on machines without the lab fixture
        }
        let image = crate::load(path).expect("load");
        let eh = image.sections.iter().find(|s| s.name == ".eh_frame");
        let Some(section) = eh else {
            panic!("no .eh_frame section in sections list ({})", image.sections.len());
        };
        let bytes = image
            .read_at_vaddr(section.address, section.size as usize)
            .unwrap_or_else(|| panic!("read_at_vaddr failed for {:#x}", section.address));
        let fdes = parse_eh_frame(bytes, section.address);
        assert!(!fdes.is_empty(), "0 FDEs from {} bytes", bytes.len());
        assert!(fdes.len() > 100, "expected >100 FDEs, got {}", fdes.len());
    }
}
