//! String harvesting pass (`strings_v1`, Wave 4 spec §6): extract
//! zero-terminated strings from string-classified sections into
//! `string:<0xADDR>` objects with canonical facts.
//!
//! Payload format (`strings_v1`): `utf8|len=<n>` or `utf16le|len=<n>`
//! (length in bytes, excluding terminators). Strings are facts, not
//! payloads: the string bytes stay in the source binary; the project
//! stores address, encoding, length, and classification (`class` fact:
//! `url|path|format|selector_like|literal` — cheap heuristics,
//! provenance-tagged inferred).

use crate::MachoError;
use damsel_project::{
    AnalysisFact, Confidence, FactKind, FactProvenance, ObjectId, TypedStore,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StringsError {
    #[error("project store error during string harvest: {0}")]
    Store(#[from] damsel_project::StoreError),
    #[error("analysis error during string harvest: {0}")]
    Macho(#[from] MachoError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringsOutcome {
    pub strings: usize,
    pub classified: usize,
}

const PASS: &str = "strings_v1";
const MIN_LEN: usize = 4;

pub fn harvest_strings(
    store: &mut impl TypedStore,
    image: &damsel_core::BinaryImage,
    source_id: i64,
    analyzer_version: &str,
) -> Result<StringsOutcome, StringsError> {
    let provenance = FactProvenance::analyzer(PASS, analyzer_version);
    let mut outcome = StringsOutcome {
        strings: 0,
        classified: 0,
    };

    for section in image.sections() {
        let lower = section.name.to_ascii_lowercase();
        let is_utf16 = lower.contains("ustring");
        let is_string_section = lower.contains("string") || lower.contains("os_log");
        if !is_string_section || section.size == 0 {
            continue;
        }
        let start = section.address;
        let end = section.address.saturating_add(section.size);
        let mut cursor = start;
        while cursor < end {
            let address = cursor;
            let Some((encoding, value, consumed)) =
                read_string(image, cursor, end, is_utf16)
            else {
                break;
            };
            if consumed == 0 {
                break;
            }
            cursor = cursor.saturating_add(consumed);
            if value.chars().count() < MIN_LEN {
                continue;
            }
            let object = ObjectId::new("string", format!("{address:#x}"));
            store.put_fact(AnalysisFact {
                source_id,
                object_id: Some(object.clone()),
                kind: FactKind::new("string"),
                payload: format!("{encoding}|len={}", value.len()),
                confidence: Confidence::Exact,
                provenance: provenance.clone(),
            })?;
            outcome.strings += 1;
            if let Some(class) = classify(&value) {
                store.put_fact(AnalysisFact {
                    source_id,
                    object_id: Some(object),
                    kind: FactKind::new("string_class"),
                    payload: class.to_string(),
                    confidence: Confidence::Inferred,
                    provenance: provenance.clone(),
                })?;
                outcome.classified += 1;
            }
        }
    }
    Ok(outcome)
}

fn read_string(
    image: &damsel_core::BinaryImage,
    address: u64,
    end: u64,
    utf16: bool,
) -> Option<(&'static str, String, u64)> {
    if !utf16 {
        let value = image.read_c_string_at_address(address, (end - address) as usize)?;
        let consumed = value.len() as u64 + 1;
        Some(("utf8", value, consumed))
    } else {
        // UTF-16LE: zero-terminated 16-bit units (Mach-O __ustring).
        let section = image
            .sections()
            .iter()
            .find(|s| address >= s.address && address < s.address.saturating_add(s.size))?;
        let file_offset = section.file_offset? + (address - section.address);
        let bytes = image.bytes_for_file_range(file_offset, (end - address) as u64)?;
        let mut units: Vec<u16> = Vec::new();
        let mut index = 0usize;
        while index + 1 < bytes.len() {
            let unit = u16::from_le_bytes([bytes[index], bytes[index + 1]]);
            index += 2;
            if unit == 0 {
                let value = String::from_utf16_lossy(&units);
                let printable = value
                    .chars()
                    .all(|ch| ch.is_ascii_graphic() || ch == ' ' || ch == ':');
                if printable && !units.is_empty() {
                    return Some(("utf16le", value, index as u64));
                }
                return None;
            }
            units.push(unit);
        }
        None
    }
}

/// Cheap first-pass classification (display/filter metadata only; never
/// renames anything). Values mirror the spec's `StringClass` set.
fn classify(value: &str) -> Option<&'static str> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Some("url");
    }
    if value.contains('/') && !value.contains(' ') {
        return Some("path");
    }
    if value.contains("%s") || value.contains("%d") || value.contains("%@") {
        return Some("format");
    }
    if value.ends_with(':') && value.chars().all(|c| c.is_ascii_lowercase() || c == ':') {
        return Some("selector_like");
    }
    None
}
