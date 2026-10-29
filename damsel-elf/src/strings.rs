//! ELF string harvesting (`strings_v1`, Wave 5B.4b): zero-terminated
//! printable runs over read-only data sections, same fact vocabulary as
//! the Mach-O pass. Values stay out of the project (privacy design).

use crate::image::ElfImage;
use damsel_project::{
    AnalysisFact, Confidence, FactKind, FactProvenance, ObjectId, TypedStore,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ElfStringsError {
    #[error("project store error during ELF string harvest: {0}")]
    Store(#[from] damsel_project::StoreError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfStringsOutcome {
    pub strings: usize,
}

const PASS: &str = "strings_v1";
const MIN_LEN: usize = 4;

pub fn harvest_elf_strings(
    store: &mut impl TypedStore,
    image: &ElfImage,
    source_id: i64,
    analyzer_version: &str,
) -> Result<ElfStringsOutcome, ElfStringsError> {
    let provenance = FactProvenance::analyzer(PASS, analyzer_version);
    let mut count = 0usize;

    for section in &image.sections {
        let lower = section.name.to_ascii_lowercase();
        let is_rodata = lower.contains("rodata") || lower.contains("str");
        if !is_rodata || section.size == 0 {
            continue;
        }
        let Some(bytes) = image.read_at_vaddr(section.address, section.size as usize) else {
            continue;
        };
        let mut cursor = 0usize;
        while cursor < bytes.len() {
            let start = cursor;
            let end = bytes[start..]
                .iter()
                .position(|b| *b == 0)
                .map(|p| start + p)
                .unwrap_or(bytes.len());
            let run = &bytes[start..end];
            cursor = (end + 1).min(bytes.len());
            if run.len() < MIN_LEN {
                continue;
            }
            let value = String::from_utf8_lossy(run);
            let printable = value
                .chars()
                .all(|ch| ch.is_ascii_graphic() || ch == ' ' || ch == ':');
            if !printable {
                continue;
            }
            let address = section.address + start as u64;
            let object = ObjectId::new("string", format!("{address:#x}"));
            store.put_fact(AnalysisFact {
                source_id,
                object_id: Some(object),
                kind: FactKind::new("string"),
                payload: format!("utf8|len={}", value.len()),
                confidence: Confidence::Exact,
                provenance: provenance.clone(),
            })?;
            count += 1;
        }
    }
    Ok(ElfStringsOutcome { strings: count })
}

/// Read a zero-terminated printable string at a virtual address
/// (bounded); CLI display mirror of `read_c_string_at_address`.
pub fn read_elf_c_string(image: &ElfImage, vaddr: u64, max_len: usize) -> Option<String> {
    let bytes = image.read_at_vaddr(vaddr, max_len)?;
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    if end == 0 {
        return None;
    }
    let value = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let printable = value
        .chars()
        .all(|ch| ch.is_ascii_graphic() || ch == ' ' || ch == ':');
    printable.then_some(value)
}
