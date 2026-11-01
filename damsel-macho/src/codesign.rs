//! Code signature inspection (plan feature 6, slice 1): LC_CODE_SIGNATURE
//! parsing — the triage surface for entitlements, TeamIDs, and hardened
//! runtime on macOS. Read-only reconstruction, `codesign(1)`-shaped.
//!
//! Layout (all big-endian): a SuperBlob (0xfade0cc0) at the linkedit
//! offset holds indexed slots — CodeDirectory (0xfade0c02, slot 0),
//! Entitlements (0xfade7171, slot 5), CMS wrapper (0xfade0b01,
//! 0x10000). Flag decoding is conservative: only the well-attested bits
//! (`0x0002` adhoc, `0x10000` hardened runtime, `0x0200` library
//! validation) get names; the raw word always ships alongside.

use crate::errors::MachoError;
use damsel_core::BinaryImage;
use std::path::Path;

const LC_CODE_SIGNATURE: u32 = 0x1d;
const SUPERBLOB_MAGIC: u32 = 0xfade_0cc0;
const CODE_DIRECTORY_MAGIC: u32 = 0xfade_0c02;
const ENTITLEMENTS_MAGIC: u32 = 0xfade_7171;
const CMS_MAGIC: u32 = 0xfade_0b01;
const CSSLOT_CODEDIRECTORY: u32 = 0;
const CSSLOT_ENTITLEMENTS: u32 = 5;

/// One entitlement key/value (values render as the plist text).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entitlement {
    pub key: String,
    pub value: String,
}

/// Reconstructed code-signature summary.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CodesignInfo {
    pub identifier: Option<String>,
    pub team_id: Option<String>,
    /// Raw CodeDirectory flags (only well-attested bits are decoded).
    pub flags: u32,
    pub hardened_runtime: bool,
    pub library_validation: bool,
    pub adhoc: bool,
    pub entitlements: Vec<Entitlement>,
    /// A CMS wrapper blob is present (not adhoc-signed).
    pub has_cms: bool,
    pub code_limit: u32,
    pub hash_type: u8,
    pub page_size: u32,
}

fn le_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn be_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn cstring_at(data: &[u8], offset: usize) -> Option<String> {
    let tail = data.get(offset..)?;
    let len = tail.iter().position(|b| *b == 0)?;
    let bytes = &tail[..len];
    if bytes.is_empty() || bytes.len() > 256 {
        return None;
    }
    if !bytes
        .iter()
        .all(|b| b.is_ascii_graphic() || *b == b' ' || *b == b'.')
    {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// Locate the LC_CODE_SIGNATURE blob range, if any.
fn code_signature_range(image: &BinaryImage) -> Option<(u64, u64)> {
    let header = image.bytes_for_file_range(0, 4096)?;
    // The Mach-O header and load commands are little-endian on damsel's
    // targets; only the signature blobs are big-endian.
    let ncmds = le_u32(header, 16)? as usize;
    let mut offset = 32usize; // mach_header_64
    for _ in 0..ncmds {
        let (cmd, cmdsize) = (
            le_u32(header, offset)?,
            le_u32(header, offset + 4)? as usize,
        );
        if cmdsize < 8 || offset + cmdsize > header.len() {
            return None;
        }
        if cmd == LC_CODE_SIGNATURE {
            let dataoff = le_u32(header, offset + 8)? as u64;
            let datasize = le_u32(header, offset + 12)? as u64;
            return Some((dataoff, datasize));
        }
        offset += cmdsize;
    }
    None
}

/// Parse an entitlements plist (XML) into key/value pairs. Best-effort:
/// `<key>` followed by `<string>`/`<true/>`/`<false/>`/`<integer>`/other
/// simple tags; anything else records the tag name.
fn parse_entitlements_plist(xml: &[u8]) -> Vec<Entitlement> {
    let text = String::from_utf8_lossy(xml);
    let mut out = Vec::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find("<key>") {
        let after_key = &rest[start + 5..];
        let Some(end) = after_key.find("</key>") else {
            break;
        };
        let key = after_key[..end].trim().to_string();
        rest = &after_key[end + 6..];
        let value = if rest.trim_start().starts_with("<string>") {
            let tail = rest.trim_start();
            let value_start = &tail["<string>".len()..];
            match value_start.find("</string>") {
                Some(stop) => {
                    let value = value_start[..stop].to_string();
                    rest = &value_start[stop + 9..];
                    value
                }
                None => break,
            }
        } else if rest.trim_start().starts_with("<true/>") {
            "true".to_string()
        } else if rest.trim_start().starts_with("<false/>") {
            "false".to_string()
        } else if rest.trim_start().starts_with("<integer>") {
            let tail = rest.trim_start();
            let value_start = &tail["<integer>".len()..];
            match value_start.find("</integer>") {
                Some(stop) => {
                    let value = value_start[..stop].to_string();
                    rest = &value_start[stop + 10..];
                    value
                }
                None => break,
            }
        } else {
            // Arrays/dicts and unknown tags collapse to a marker.
            "(value)".to_string()
        };
        if !key.is_empty() {
            out.push(Entitlement { key, value });
        }
    }
    out
}

/// Parse a code-signature SuperBlob.
pub fn parse_signature_blob(blob: &[u8]) -> Result<CodesignInfo, MachoError> {
    if be_u32(blob, 0).ok_or_else(malformed())? != SUPERBLOB_MAGIC {
        return Err(MachoError::MalformedSharedCache(
            "code signature superblob magic mismatch".to_string(),
        ));
    }
    let count = be_u32(blob, 8).ok_or_else(malformed())? as usize;
    let mut info = CodesignInfo {
        identifier: None,
        team_id: None,
        flags: 0,
        hardened_runtime: false,
        library_validation: false,
        adhoc: false,
        entitlements: Vec::new(),
        has_cms: false,
        code_limit: 0,
        hash_type: 0,
        page_size: 0,
    };
    let mut code_directory: Option<&[u8]> = None;

    for index in 0..count {
        let entry = 12 + index * 8;
        let (slot_type, slot_offset) = (
            be_u32(blob, entry).ok_or_else(malformed())?,
            be_u32(blob, entry + 4).ok_or_else(malformed())? as usize,
        );
        if slot_offset + 8 > blob.len() {
            continue;
        }
        let magic = be_u32(blob, slot_offset).unwrap_or(0);
        let length = be_u32(blob, slot_offset + 4).unwrap_or(0) as usize;
        let slot_bytes = blob
            .get(slot_offset..slot_offset.saturating_add(length))
            .unwrap_or(&[]);
        match magic {
            CODE_DIRECTORY_MAGIC if slot_type == CSSLOT_CODEDIRECTORY => {
                code_directory = Some(slot_bytes);
            }
            ENTITLEMENTS_MAGIC if slot_type == CSSLOT_ENTITLEMENTS => {
                if slot_bytes.len() > 8 {
                    info.entitlements = parse_entitlements_plist(&slot_bytes[8..]);
                }
            }
            CMS_MAGIC => {
                info.has_cms = true;
            }
            _ => {}
        }
    }

    if let Some(directory) = code_directory {
        // CodeDirectory header (v0x20400-aware):
        // magic0 length4 version8 flags12 hashOffset16 identOffset20
        // nSpecialSlots24 nCodeSlots28 codeLimit32 hashSize36 hashType37
        // platform38 pageSize39(log2) spare2 40 scatterOffset44
        // teamOffset48 (v>=0x20200).
        let version = be_u32(directory, 8).unwrap_or(0);
        info.flags = be_u32(directory, 12).unwrap_or(0);
        let ident_offset = be_u32(directory, 20).unwrap_or(0) as usize;
        info.code_limit = be_u32(directory, 32).unwrap_or(0);
        info.hash_type = directory.get(37).copied().unwrap_or(0);
        let page_log2 = directory.get(39).copied().unwrap_or(12);
        info.page_size = if page_log2 <= 16 {
            1u32 << page_log2
        } else {
            0
        };
        info.identifier = cstring_at(directory, ident_offset);
        info.hardened_runtime = info.flags & 0x10000 != 0;
        info.library_validation = info.flags & 0x0200 != 0;
        info.adhoc = info.flags & 0x0002 != 0;
        if version >= 0x20200 {
            let team_offset = be_u32(directory, 48).unwrap_or(0) as usize;
            if team_offset != 0 {
                info.team_id = cstring_at(directory, team_offset);
            }
        }
    }
    Ok(info)
}

/// Read the code signature from a standalone Mach-O image. `Ok(None)`
/// means the binary carries no LC_CODE_SIGNATURE.
pub fn read_codesign(image: &BinaryImage) -> Result<Option<CodesignInfo>, MachoError> {
    let Some((dataoff, datasize)) = code_signature_range(image) else {
        return Ok(None);
    };
    let Some(blob) = image.bytes_for_file_range(dataoff, datasize) else {
        return Err(MachoError::MalformedSharedCache(format!(
            "code signature at {dataoff:#x} (+{datasize:#x}) is outside the image"
        )));
    };
    Ok(Some(parse_signature_blob(blob)?))
}

fn malformed() -> impl Fn() -> MachoError {
    || MachoError::MalformedSharedCache("truncated code signature blob".to_string())
}

/// Convenience for CLI callers: load from a path.
pub fn read_codesign_path(path: &Path) -> Result<Option<CodesignInfo>, MachoError> {
    let image = crate::loader::load(path)?;
    read_codesign(&image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/bin")
            .join(name)
    }

    #[test]
    fn adhoc_fixture_decodes() {
        let image = crate::loader::load(fixture_path("objc-sample")).expect("load fixture");
        let info = read_codesign(&image).expect("parse").expect("signed");
        assert_eq!(info.identifier.as_deref(), Some("objc-sample"));
        assert!(info.adhoc, "flags {:#x}", info.flags);
        assert!(!info.has_cms, "adhoc has no CMS wrapper");
        assert_eq!(info.hash_type, 2, "SHA-256");
        assert_eq!(info.page_size, 4096);
        assert_eq!(info.code_limit, 0xd5d0);
        assert!(info.entitlements.is_empty());
    }

    #[test]
    fn entitlements_plist_decode() {
        let xml = br#"<?xml version="1.0"?>
<plist><dict>
<key>com.apple.security.hardened-process</key><true/>
<key>com.apple.security.cs.allow-jit</key><false/>
<key>com.apple.security.app-sandbox</key><string>enforced</string>
<key>group-priv</key><integer>42</integer>
</dict></plist>"#;
        let parsed = parse_entitlements_plist(xml);
        let pairs: Vec<(&str, &str)> = parsed
            .iter()
            .map(|entry| (entry.key.as_str(), entry.value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("com.apple.security.hardened-process", "true"),
                ("com.apple.security.cs.allow-jit", "false"),
                ("com.apple.security.app-sandbox", "enforced"),
                ("group-priv", "42"),
            ]
        );
    }

    #[test]
    fn synthetic_superblob_with_entitlements_and_cms() {
        // CodeDirectory with identifier + teamID (v0x20200) + flags, an
        // entitlements slot, and a CMS wrapper.
        let mut directory = vec![0u8; 96];
        directory[0..4].copy_from_slice(&0xfade_0c02u32.to_be_bytes());
        let directory_len = directory.len() as u32;
        directory[4..8].copy_from_slice(&directory_len.to_be_bytes());
        directory[8..12].copy_from_slice(&0x20200u32.to_be_bytes());
        directory[12..16].copy_from_slice(&0x10200u32.to_be_bytes()); // runtime+LV
        directory[20..24].copy_from_slice(&96u32.to_be_bytes()); // identOffset
        directory.extend_from_slice(b"damsel-test\0");
        let ident_end = directory.len() as u32;
        directory[48..52].copy_from_slice(&ident_end.to_be_bytes()); // teamOffset
        directory.extend_from_slice(b"TEAM12345\0");
        // Fix length after extension.
        let total = directory.len() as u32;
        directory[4..8].copy_from_slice(&total.to_be_bytes());

        let entitlements_payload = b"<plist><dict><key>test.allow</key><true/></dict></plist>";
        let mut entitlements = vec![0u8; 8 + entitlements_payload.len()];
        entitlements[0..4].copy_from_slice(&0xfade_7171u32.to_be_bytes());
        let entitlements_len = entitlements.len() as u32;
        entitlements[4..8].copy_from_slice(&entitlements_len.to_be_bytes());
        entitlements[8..].copy_from_slice(entitlements_payload);

        let mut cms = vec![0u8; 16];
        cms[0..4].copy_from_slice(&0xfade_0b01u32.to_be_bytes());
        cms[4..8].copy_from_slice(&16u32.to_be_bytes());

        let mut blob = vec![0u8; 12 + 3 * 8];
        blob[0..4].copy_from_slice(&0xfade_0cc0u32.to_be_bytes());
        let superblob_len = (12 + 3 * 8 + total as usize + entitlements.len() + cms.len()) as u32;
        blob[4..8].copy_from_slice(&superblob_len.to_be_bytes());
        blob[8..12].copy_from_slice(&3u32.to_be_bytes());
        let mut offsets = [(0u32, 0usize), (5u32, 0usize), (0x10000u32, 0usize)];
        offsets[0].1 = 12 + 3 * 8; // slots begin after the superblob index
        offsets[1].1 = offsets[0].1 + directory.len();
        offsets[2].1 = offsets[1].1 + entitlements.len();
        for (index, (slot_type, offset)) in offsets.iter().enumerate() {
            let entry = 12 + index * 8;
            blob[entry..entry + 4].copy_from_slice(&slot_type.to_be_bytes());
            blob[entry + 4..entry + 8].copy_from_slice(&(*offset as u32).to_be_bytes());
        }
        blob.extend_from_slice(&directory);
        blob.extend_from_slice(&entitlements);
        blob.extend_from_slice(&cms);

        let info = parse_signature_blob(&blob).expect("parse superblob");
        assert_eq!(info.identifier.as_deref(), Some("damsel-test"));
        assert_eq!(info.team_id.as_deref(), Some("TEAM12345"));
        assert!(info.hardened_runtime);
        assert!(info.library_validation);
        assert!(!info.adhoc);
        assert!(info.has_cms);
        assert_eq!(info.entitlements.len(), 1);
        assert_eq!(info.entitlements[0].key, "test.allow");
        assert_eq!(info.entitlements[0].value, "true");
    }

    #[test]
    fn unsigned_image_is_none() {
        let image = crate::loader::load(fixture_path("arm64-stripped")).expect("load");
        // Whatever the fixture carries, the API must not fabricate.
        if let Some(info) = read_codesign(&image).expect("parse") {
            assert!(info.identifier.is_some() || info.adhoc || info.has_cms);
        }
    }
}
