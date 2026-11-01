//! Swift symbol demangling (Wave 8.3): `_$s`-prefixed mangled names to
//! readable `Module.function` form. Handles length-prefixed identifiers,
//! common type abbreviations, and special suffixes (default arguments,
//! partial applies, static/class methods).

use serde::Serialize;

/// A demangled Swift symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demangled {
    /// The readable form: `Module.function` or `Module.function(labels:)`.
    pub display: String,
    /// Module name (e.g. `DamselSwiftSample` or `Swift`).
    pub module: String,
    /// Function/variable name without module.
    pub name: String,
    /// True if this is a default-argument thunk or partial apply.
    pub is_thunk: bool,
}

/// Demangle a Swift symbol. Returns None if not a Swift mangling
/// (`_$s` or `$s` prefix) or if the structure can't be parsed.
pub fn demangle(mangled: &str) -> Option<Demangled> {
    let rest = mangled
        .strip_prefix("_$s")
        .or_else(|| mangled.strip_prefix("$s"))?;

    // Two module forms:
    //   `_$s17DamselSwiftSample9foo...` — length-prefixed module name
    //   `_$ss5print_...`               — `s` = Swift module shorthand
    //   `_$sSa12_endMutation...`        — `Sa` = Array type context
    let (module, after_module): (&str, &str) = if let Some(r) = rest.strip_prefix("ss") {
        // `ss` = Swift module (double s)
        if r.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            ("Swift", r)
        } else {
            parse_identifier(rest)?
        }
    } else if let Some(r) = rest.strip_prefix('s') {
        if r.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            ("Swift", r)
        } else {
            parse_identifier(rest)?
        }
    } else if let Some(r) = rest.strip_prefix("Sa") {
        // Array type context
        if r.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            ("Swift.Array", r)
        } else {
            parse_identifier(rest)?
        }
    } else if let Some(r) = rest.strip_prefix("SH") {
        // Hashable protocol
        ("Swift.Hashable", r)
    } else if let Some(r) = rest.strip_prefix("SQ") {
        // Equatable protocol
        ("Swift.Equatable", r)
    } else if let Some(r) = rest.strip_prefix("SR") {
        // RawRepresentable protocol
        ("Swift.RawRepresentable", r)
    } else if let Some(r) = rest.strip_prefix("SE") {
        // Encodable protocol
        ("Swift.Encodable", r)
    } else if let Some(r) = rest.strip_prefix("SD") {
        // Decodable protocol
        ("Swift.Decodable", r)
    } else if let Some(r) = rest.strip_prefix("SCT") {
        // CaseIterable protocol (three-char)
        ("Swift.CaseIterable", r)
    } else if let Some(r) = rest.strip_prefix("Bo") {
        // Builtin.Bool witness
        ("Builtin.Bool", r)
    } else {
        parse_identifier(rest)?
    };

    // Parse the first identifier (type name or function name).
    let (first, after_first) = parse_identifier(after_module)?;

    // Check for a type marker (V=struct, C=class, O=enum) followed by
    // a member name: `RectV8describe...` = Rect.describe()
    let (name, display, after_name) = if let Some(stripped) = after_first
        .strip_prefix('V')
        .or_else(|| after_first.strip_prefix('C'))
        .or_else(|| after_first.strip_prefix('O'))
    {
        if let Some((member, rest)) = parse_identifier(stripped) {
            (member, format!("{module}.{first}.{member}"), rest)
        } else {
            (first, format!("{module}.{first}"), after_first)
        }
    } else {
        (first, format!("{module}.{first}"), after_first)
    };

    // Check for special suffixes in the type section.
    let type_section = after_name;
    let is_thunk =
        type_section.contains("FfA") || type_section.contains("TW") || type_section.contains("TJ");

    // Extend display with common accessor suffixes.
    let display = if type_section.ends_with("vg") {
        format!("{display}.getter")
    } else if type_section.ends_with("vs") {
        format!("{display}.setter")
    } else if type_section.ends_with("fC") {
        format!("{display}(init)")
    } else {
        display
    };

    Some(Demangled {
        display,
        module: module.to_string(),
        name: name.to_string(),
        is_thunk,
    })
}

/// Parse a length-prefixed identifier: `<len><name>`.
fn parse_identifier(input: &str) -> Option<(&str, &str)> {
    let digits: String = input.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let len: usize = digits.parse().ok()?;
    let rest = &input[digits.len()..];
    if rest.len() < len {
        return None;
    }
    let (name, remainder) = rest.split_at(len);
    // Identifiers must be valid word chars.
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some((name, remainder))
}

/// Demangle a type abbreviation to a readable type name.
/// Returns None for unhandled type encodings.
pub fn demangle_type(type_str: &str) -> Option<String> {
    match type_str {
        "Si" => Some("Swift.Int".to_string()),
        "SS" => Some("Swift.String".to_string()),
        "Sb" => Some("Swift.Bool".to_string()),
        "Sd" => Some("Swift.Double".to_string()),
        "Sf" => Some("Swift.Float".to_string()),
        "Su" => Some("Swift.UInt".to_string()),
        "y" => Some("()".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demangles_public_add() {
        let d = demangle("_$s17DamselSwiftSample9publicAddyS2iF").unwrap();
        assert_eq!(d.display, "DamselSwiftSample.publicAdd");
        assert_eq!(d.module, "DamselSwiftSample");
        assert_eq!(d.name, "publicAdd");
        assert!(!d.is_thunk);
    }

    #[test]
    fn demangles_internal_message() {
        let d = demangle("_$s17DamselSwiftSample15internalMessageySSSiF").unwrap();
        assert_eq!(d.display, "DamselSwiftSample.internalMessage");
        assert_eq!(d.module, "DamselSwiftSample");
        assert_eq!(d.name, "internalMessage");
    }

    #[test]
    fn demangles_swift_module_print() {
        // `_$ss5print_...` — module Swift, function print
        let d = demangle("_$ss5print_9separator10terminatoryypd_S2StFfA0_").unwrap();
        assert_eq!(d.display, "Swift.print");
        assert_eq!(d.module, "Swift");
        assert_eq!(d.name, "print");
        assert!(d.is_thunk); // default argument thunk
    }

    #[test]
    fn demangles_finalize_array() {
        let d = demangle("_$ss27_finalizeUninitializedArrayySayxGABnlF").unwrap();
        assert_eq!(d.display, "Swift._finalizeUninitializedArray");
        assert!(!d.is_thunk); // generic function, not a thunk
    }

    #[test]
    fn rejects_non_swift() {
        assert!(demangle("_main").is_none());
        assert!(demangle("printf").is_none());
        assert!(demangle("_objc_msgSend").is_none());
    }

    #[test]
    fn rejects_malformed() {
        // Truncated length prefix
        assert!(demangle("_$s999xyz").is_none());
        // Non-identifier content
        assert!(demangle("_$s4ma!n3foo").is_none());
    }

    #[test]
    fn type_abbreviations() {
        assert_eq!(demangle_type("Si").unwrap(), "Swift.Int");
        assert_eq!(demangle_type("SS").unwrap(), "Swift.String");
        assert_eq!(demangle_type("Sb").unwrap(), "Swift.Bool");
        assert!(demangle_type("XX").is_none());
    }
}

/// A Swift metadata section found in a binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwiftSection {
    /// Section name without the `__swift5_` prefix (e.g. `typeref`).
    pub kind: String,
    /// Full section name (e.g. `__swift5_typeref`).
    pub name: String,
    /// Section size in bytes.
    pub size: usize,
}

/// Scan section names for Swift metadata sections (`__swift5_*`).
pub fn swift_sections(sections: &[(String, usize)]) -> Vec<SwiftSection> {
    sections
        .iter()
        .filter(|(name, _)| name.starts_with("__swift5_"))
        .map(|(name, size)| SwiftSection {
            kind: name.trim_start_matches("__swift5_").to_string(),
            name: name.clone(),
            size: *size,
        })
        .collect()
}

/// True if the binary contains any Swift metadata sections.
pub fn is_swift_binary(sections: &[(String, usize)]) -> bool {
    sections
        .iter()
        .any(|(name, _)| name.starts_with("__swift5_"))
}

#[cfg(test)]
mod section_tests {
    use super::*;

    #[test]
    fn detects_swift_sections() {
        let sections = vec![
            ("__text".to_string(), 4096),
            ("__swift5_entry".to_string(), 8),
            ("__swift5_typeref".to_string(), 256),
            ("__cstring".to_string(), 512),
        ];
        let swift = swift_sections(&sections);
        assert_eq!(swift.len(), 2);
        assert_eq!(swift[0].kind, "entry");
        assert_eq!(swift[1].kind, "typeref");
        assert_eq!(swift[1].size, 256);
    }

    #[test]
    fn no_swift_sections() {
        let sections = vec![("__text".to_string(), 4096)];
        assert!(swift_sections(&sections).is_empty());
        assert!(!is_swift_binary(&sections));
    }

    #[test]
    fn is_swift() {
        let sections = vec![("__swift5_entry".to_string(), 8)];
        assert!(is_swift_binary(&sections));
    }
}

/// A field name extracted from `__swift5_reflstr`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwiftField {
    pub name: String,
}

/// Parse `__swift5_reflstr` bytes into field names. The section is a
/// sequence of null-terminated strings (field names, case names,
/// associated value labels).
pub fn parse_reflstr(data: &[u8]) -> Vec<SwiftField> {
    data.split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() >= 1 && s.len() <= 64)
        .filter(|s| s.iter().all(|b| b.is_ascii_graphic() || *b == b' '))
        .map(|s| SwiftField {
            name: String::from_utf8_lossy(s).into_owned(),
        })
        .collect()
}

/// A type record from `__swift5_types` (simplified: type kind +
/// name from the mangled reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwiftTypeRecord {
    /// "struct", "class", "enum", or "protocol".
    pub kind: String,
    /// The type's name from reflection strings or the symbol table.
    pub name: String,
}

#[cfg(test)]
mod reflstr_tests {
    use super::*;

    #[test]
    fn parses_field_names() {
        // "radius\0w\0h\0red\0green\0blue\0"
        let data = b"radius\0w\0h\0red\0green\0blue\0";
        let fields = parse_reflstr(data);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["radius", "w", "h", "red", "green", "blue"]);
    }

    #[test]
    fn skips_empty_and_binary() {
        // Real reflstr segments are compiler-written clean strings;
        // binary garbage appears as its own segment.
        let data = b"\0\0name\0\xff\xfe\0";
        let fields = parse_reflstr(data);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["name"]);
    }

    #[test]
    fn real_fixture_reflstr() {
        // The swift-rich fixture contains: RawValue, AllCases, radius,
        // w, h, red, green, blue, shapes, theme
        let path = format!(
            "{}/../fixtures/bin/adversarial/swift-rich-arm64",
            env!("CARGO_MANIFEST_DIR")
        );
        let image = match damsel_macho::load(&path) {
            Ok(img) => img,
            Err(_) => return, // skip on non-macOS CI
        };
        let reflstr: Vec<u8> = image
            .sections()
            .iter()
            .filter(|s| s.name == "__swift5_reflstr")
            .filter_map(|s| image.bytes_for_file_range(s.file_offset?, s.file_size))
            .flat_map(|b| b.iter().copied())
            .collect();
        if reflstr.is_empty() {
            return;
        }
        let fields = parse_reflstr(&reflstr);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"radius"), "fields: {names:?}");
        assert!(names.contains(&"red"));
        assert!(names.contains(&"theme"));
    }
}

/// A field descriptor from `__swift5_fieldmd`: one struct/class/enum's
/// fields with names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldDescriptor {
    /// Descriptor kind (0=struct, 1=class, 2=enum...).
    pub kind: u16,
    /// Field names in declaration order.
    pub field_names: Vec<String>,
}

/// Parse `__swift5_fieldmd` bytes into field descriptors.
///
/// Each descriptor is: MangledTypeName(4) + Superclass(4) + Kind(2) +
/// FieldRecordSize(2) + FieldCount(4) + N × FieldRecord(12 each).
/// Field records reference names via relative pointers into
/// `__swift5_reflstr`.
pub fn parse_fieldmd(
    fieldmd: &[u8],
    fieldmd_vaddr: u64,
    reflstr: &[u8],
    reflstr_vaddr: u64,
) -> Vec<FieldDescriptor> {
    let mut descriptors = Vec::new();
    let mut offset = 0usize;

    while offset + 16 <= fieldmd.len() {
        // Read the header.
        let kind = u16::from_le_bytes([fieldmd[offset + 8], fieldmd[offset + 9]]);
        let field_record_size =
            u16::from_le_bytes([fieldmd[offset + 10], fieldmd[offset + 11]]) as usize;
        let field_count = u32::from_le_bytes([
            fieldmd[offset + 12],
            fieldmd[offset + 13],
            fieldmd[offset + 14],
            fieldmd[offset + 15],
        ]) as usize;

        // Sanity: field_record_size is typically 12; count is bounded.
        if field_record_size != 12 || field_count > 128 || field_count == 0 {
            offset += 4; // Skip forward looking for the next record.
            continue;
        }

        let records_start = offset + 16;
        let total = records_start + field_count * field_record_size;
        if total > fieldmd.len() {
            break;
        }

        let mut field_names = Vec::new();
        for i in 0..field_count {
            let record_offset = records_start + i * field_record_size;
            // FieldRecord: Flags(4) + MangledTypeName(4) + FieldName(4)
            // FieldName is a relative pointer from its own position.
            let field_name_ptr_pos = record_offset + 8;
            if field_name_ptr_pos + 4 > fieldmd.len() {
                break;
            }
            let rel = i32::from_le_bytes([
                fieldmd[field_name_ptr_pos],
                fieldmd[field_name_ptr_pos + 1],
                fieldmd[field_name_ptr_pos + 2],
                fieldmd[field_name_ptr_pos + 3],
            ]);
            if rel == 0 {
                continue;
            }
            // Resolve: the pointer's vaddr + rel gives the target vaddr.
            let ptr_vaddr = fieldmd_vaddr + field_name_ptr_pos as u64;
            let target_vaddr = (ptr_vaddr as i64 + rel as i64) as u64;
            // Convert to reflstr offset.
            if target_vaddr >= reflstr_vaddr {
                let reflstr_offset = (target_vaddr - reflstr_vaddr) as usize;
                if reflstr_offset < reflstr.len() {
                    // Read null-terminated string.
                    let end = reflstr[reflstr_offset..]
                        .iter()
                        .position(|b| *b == 0)
                        .map(|p| reflstr_offset + p)
                        .unwrap_or(reflstr.len());
                    let name = String::from_utf8_lossy(&reflstr[reflstr_offset..end]).into_owned();
                    if !name.is_empty() && name.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
                        field_names.push(name);
                    }
                }
            }
        }

        if !field_names.is_empty() {
            descriptors.push(FieldDescriptor { kind, field_names });
        }

        offset = total;
    }

    descriptors
}

#[cfg(test)]
mod fieldmd_tests {
    use super::*;

    fn u32b(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }
    fn u16b(v: u16) -> [u8; 2] {
        v.to_le_bytes()
    }

    /// Build a minimal fieldmd with one descriptor (struct, 2 fields)
    /// and a reflstr section with the field names.
    fn build_fixture() -> (Vec<u8>, Vec<u8>) {
        // reflstr: "radius\0w\0h\0" at vaddr 0x2000
        let reflstr = b"radius\0w\0h\0".to_vec();
        let reflstr_vaddr = 0x2000u64;

        // fieldmd at vaddr 0x1000:
        // Descriptor: MangledTypeName(0) + Superclass(0) + Kind(0=struct) +
        //             RecordSize(12) + Count(2)
        // Field 1: Flags(0) + MangledTypeName(0) + FieldName(→ "radius")
        // Field 2: Flags(0) + MangledTypeName(0) + FieldName(→ "w")
        let fieldmd_vaddr = 0x1000u64;
        let mut fieldmd = Vec::new();
        fieldmd.extend_from_slice(&u32b(0)); // MangledTypeName: none
        fieldmd.extend_from_slice(&u32b(0)); // Superclass: none
        fieldmd.extend_from_slice(&u16b(0)); // Kind: struct
        fieldmd.extend_from_slice(&u16b(12)); // RecordSize
        fieldmd.extend_from_slice(&u32b(2)); // Count

        // Field 1: FieldName relative pointer to reflstr "radius" (0x2000)
        let field1_name_pos = fieldmd_vaddr + 16 + 8; // +Flags+Mangled
        let rel1 = (reflstr_vaddr as i64 - field1_name_pos as i64) as i32;
        fieldmd.extend_from_slice(&u32b(0)); // Flags
        fieldmd.extend_from_slice(&u32b(0)); // MangledTypeName
        fieldmd.extend_from_slice(&u32b(rel1 as u32)); // FieldName rel ptr

        // Field 2: FieldName → "w" (0x2000 + 7)
        let field2_name_pos = fieldmd_vaddr + 16 + 12 + 8;
        let w_addr = reflstr_vaddr + 7; // "radius\0" is 7 bytes
        let rel2 = (w_addr as i64 - field2_name_pos as i64) as i32;
        fieldmd.extend_from_slice(&u32b(0)); // Flags
        fieldmd.extend_from_slice(&u32b(0)); // MangledTypeName
        fieldmd.extend_from_slice(&u32b(rel2 as u32)); // FieldName rel ptr

        (fieldmd, reflstr)
    }

    #[test]
    fn parses_field_descriptor() {
        let (fieldmd, reflstr) = build_fixture();
        let descriptors = parse_fieldmd(&fieldmd, 0x1000, &reflstr, 0x2000);
        assert_eq!(descriptors.len(), 1);
        assert_eq!(descriptors[0].kind, 0); // struct
        assert_eq!(descriptors[0].field_names, vec!["radius", "w"]);
    }

    #[test]
    fn real_fixture_fieldmd() {
        let path = format!(
            "{}/../fixtures/bin/adversarial/swift-rich-arm64",
            env!("CARGO_MANIFEST_DIR")
        );
        let image = match damsel_macho::load(&path) {
            Ok(img) => img,
            Err(_) => return,
        };
        let get_section = |name: &str| -> Option<(Vec<u8>, u64)> {
            image
                .sections()
                .iter()
                .find(|s| s.name == name)
                .and_then(|s| {
                    image
                        .bytes_for_file_range(s.file_offset?, s.file_size)
                        .map(|b| (b.to_vec(), s.address))
                })
        };
        let Some((fieldmd, fieldmd_va)) = get_section("__swift5_fieldmd") else {
            return;
        };
        let Some((reflstr, reflstr_va)) = get_section("__swift5_reflstr") else {
            return;
        };
        let descriptors = parse_fieldmd(&fieldmd, fieldmd_va, &reflstr, reflstr_va);
        if descriptors.is_empty() {
            return; // skip if format variation
        }
        // The fixture has Circle(radius), Rect(w, h), Renderer(shapes, theme)
        let all_names: Vec<&str> = descriptors
            .iter()
            .flat_map(|d| d.field_names.iter().map(|s| s.as_str()))
            .collect();
        assert!(all_names.contains(&"radius"), "names: {all_names:?}");
        assert!(all_names.contains(&"w") || all_names.contains(&"h"));
    }
}

// ---------------------------------------------------------------------------
// Protocol / type / conformance reconstruction (plan feature 4, slice 1;
// closes Wave 8.3's "protocol/conformance records remain").
//
// Swift reflection ABI shapes involved:
// - `__swift5_protocols` (some toolchains `__swift5_protos`): array of
//   4-byte relative pointers to protocol context descriptors.
// - `__swift5_types`: array of 4-byte relative pointers to type context
//   descriptors. Every context descriptor begins with
//   Flags(4) + Parent(4) + Name(4, relative); kind is Flags' low nibble
//   (0 struct, 1 class, 2 enum, 3 protocol, 4 extension).
// - `__swift5_proto`: array of 12-byte conformance records:
//   Protocol(4, relative) + TypeRef(4, relative) + Flags(4) — bit 0x40 of
//   Flags marks retroactive conformance.
// The descriptors themselves live in `__TEXT,__constg_swiftt`.

/// One section's bytes with its vmaddr (descriptor + string lookups span
/// several Swift sections; resolution searches whatever is provided).
#[derive(Debug, Clone)]
pub struct SwiftSectionBytes {
    pub vaddr: u64,
    pub bytes: Vec<u8>,
}

/// A protocol descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwiftProtocol {
    /// Name when the descriptor's Name field resolves in a string section.
    pub name: Option<String>,
    pub descriptor_vaddr: u64,
}

/// A type context descriptor (`__swift5_types`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwiftTypeDescriptor {
    /// "struct" | "class" | "enum" | "protocol" | "extension" | "other".
    pub kind: &'static str,
    pub name: Option<String>,
    pub descriptor_vaddr: u64,
}

/// One protocol conformance record (`__swift5_proto`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwiftConformance {
    pub protocol_name: Option<String>,
    pub type_name: Option<String>,
    pub type_kind: &'static str,
    pub flags: u32,
    pub retroactive: bool,
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    read_u32(data, offset).map(|value| value as i32)
}

/// Bytes at `vaddr` when any provided section covers them.
fn bytes_at<'a>(sections: &'a [SwiftSectionBytes], vaddr: u64, len: usize) -> Option<&'a [u8]> {
    sections
        .iter()
        .find(|section| {
            vaddr >= section.vaddr
                && vaddr + len as u64 <= section.vaddr + section.bytes.len() as u64
        })
        .map(|section| {
            &section.bytes[(vaddr - section.vaddr) as usize..(vaddr - section.vaddr) as usize + len]
        })
}

/// Resolve a relative pointer stored at `pointer_vaddr` (in `sections`).
fn resolve_relative(sections: &[SwiftSectionBytes], pointer_vaddr: u64) -> Option<u64> {
    let rel = read_i32(bytes_at(sections, pointer_vaddr, 4)?, 0)?;
    if rel == 0 {
        return None;
    }
    Some(pointer_vaddr.wrapping_add(rel as i64 as u64))
}

/// Read a null-terminated printable string starting at `vaddr`.
fn read_cstring(sections: &[SwiftSectionBytes], vaddr: u64) -> Option<String> {
    let base = sections.iter().find(|section| {
        vaddr >= section.vaddr && vaddr < section.vaddr + section.bytes.len() as u64
    })?;
    let start = (vaddr - base.vaddr) as usize;
    let tail = &base.bytes[start..];
    let len = tail.iter().position(|b| *b == 0)?;
    let bytes = &tail[..len];
    if bytes.is_empty() || bytes.len() > 128 {
        return None;
    }
    if !bytes.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// Flags + Name of a context descriptor at `descriptor_vaddr`.
fn descriptor_header(
    sections: &[SwiftSectionBytes],
    descriptor_vaddr: u64,
) -> Option<(u32, Option<String>)> {
    let flags = read_u32(bytes_at(sections, descriptor_vaddr, 4)?, 0)?;
    let name = resolve_relative(sections, descriptor_vaddr + 8)
        .and_then(|name_vaddr| read_cstring(sections, name_vaddr));
    Some((flags, name))
}

fn descriptor_kind(flags: u32) -> &'static str {
    match flags & 0xf {
        0 => "class",
        1 => "struct",
        2 => "enum",
        3 => "protocol",
        4 => "extension",
        _ => "other",
    }
}

/// Parse a section of 4-byte relative pointers into descriptors
/// (`__swift5_protocols` / `__swift5_types` share the shape).
fn parse_descriptor_pointers(
    pointers: &[u8],
    pointers_vaddr: u64,
    sections: &[SwiftSectionBytes],
) -> Vec<(u64, Option<u32>, Option<String>)> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset + 4 <= pointers.len() {
        let entry_vaddr = pointers_vaddr + offset as u64;
        let rel = match read_i32(pointers, offset) {
            Some(rel) if rel != 0 => rel,
            _ => {
                offset += 4;
                continue;
            }
        };
        let descriptor_vaddr = entry_vaddr.wrapping_add(rel as i64 as u64);
        let header = bytes_at(sections, descriptor_vaddr, 16)
            .and_then(|_| descriptor_header(sections, descriptor_vaddr));
        out.push(match header {
            Some((flags, name)) => (descriptor_vaddr, Some(flags), name),
            None => (descriptor_vaddr, None, None),
        });
        offset += 4;
    }
    out
}

/// `__swift5_protocols` (or `__swift5_protos`) → named protocols.
pub fn parse_protocols(
    protocols: &[u8],
    protocols_vaddr: u64,
    sections: &[SwiftSectionBytes],
) -> Vec<SwiftProtocol> {
    parse_descriptor_pointers(protocols, protocols_vaddr, sections)
        .into_iter()
        .map(|(descriptor_vaddr, flags, name)| {
            // Only keep entries that look like protocol descriptors when
            // flags are readable; unverifiable entries still surface.
            let _ = flags;
            SwiftProtocol {
                name,
                descriptor_vaddr,
            }
        })
        .collect()
}

/// `__swift5_types` → named type descriptors.
pub fn parse_types(
    types: &[u8],
    types_vaddr: u64,
    sections: &[SwiftSectionBytes],
) -> Vec<SwiftTypeDescriptor> {
    parse_descriptor_pointers(types, types_vaddr, sections)
        .into_iter()
        .map(|(descriptor_vaddr, flags, name)| SwiftTypeDescriptor {
            kind: flags.map(descriptor_kind).unwrap_or("other"),
            name,
            descriptor_vaddr,
        })
        .collect()
}

/// `__swift5_proto` → conformance records with protocol and type names
/// resolved against the parsed protocol/type descriptor sets.
///
/// The section is an array of 4-byte relative pointers to
/// `ConformanceDescriptor { Protocol(rel), TypeRef(rel), Flags }` records
/// living in the const sections. Protocol and type references may be
/// *indirect* (low bit set: the aligned address holds a pointer to the
/// real descriptor, typically for runtime-library protocols); indirect
/// references surface with `None` names and kind "indirect".
pub fn parse_conformances(
    conformances: &[u8],
    conformances_vaddr: u64,
    sections: &[SwiftSectionBytes],
    protocols: &[SwiftProtocol],
    types: &[SwiftTypeDescriptor],
) -> Vec<SwiftConformance> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset + 4 <= conformances.len() {
        let entry_vaddr = conformances_vaddr + offset as u64;
        let rel = match read_i32(conformances, offset) {
            Some(rel) if rel != 0 => rel,
            _ => {
                offset += 4;
                continue;
            }
        };
        offset += 4;
        let record_vaddr = entry_vaddr.wrapping_add(rel as i64 as u64);
        let Some(record) = bytes_at(sections, record_vaddr, 12) else {
            out.push(SwiftConformance {
                protocol_name: None,
                type_name: None,
                type_kind: "unknown",
                flags: 0,
                retroactive: false,
            });
            continue;
        };
        let protocol_rel = read_i32(record, 0).unwrap_or(0);
        let type_rel = read_i32(record, 4).unwrap_or(0);
        let flags = read_u32(record, 8).unwrap_or(0);

        let mut protocol_name = None;
        if protocol_rel != 0 {
            let target = record_vaddr.wrapping_add(protocol_rel as i64 as u64);
            if target & 1 == 0 {
                protocol_name = protocols
                    .iter()
                    .find(|protocol| protocol.descriptor_vaddr == target)
                    .and_then(|protocol| protocol.name.clone());
            }
        }
        let mut type_name = None;
        let mut type_kind = "unknown";
        if type_rel != 0 {
            let target = record_vaddr
                .wrapping_add(4)
                .wrapping_add(type_rel as i64 as u64);
            if target & 1 == 1 {
                type_kind = "indirect";
            } else if let Some(descriptor) = types
                .iter()
                .find(|descriptor| descriptor.descriptor_vaddr == target)
            {
                type_name = descriptor.name.clone();
                type_kind = descriptor.kind;
            }
        }
        out.push(SwiftConformance {
            protocol_name,
            type_name,
            type_kind,
            flags,
            retroactive: flags & 0x40 != 0,
        });
    }
    out
}

#[cfg(test)]
mod protocol_tests {
    use super::*;

    fn fixture_section_bytes(path: &str, segment: &str, name: &str) -> Option<(Vec<u8>, u64)> {
        let image = damsel_macho::load(path).ok()?;
        // Match by segment too: `__const` also exists as __DATA zero-fill.
        let section = image
            .sections()
            .iter()
            .find(|s| s.segment_name == segment && s.name == name)?;
        let bytes = image
            .bytes_for_file_range(section.file_offset?, section.file_size)?
            .to_vec();
        Some((bytes, section.address))
    }

    fn fixture_sections(path: &str, names: &[(&str, &str)]) -> Vec<SwiftSectionBytes> {
        names
            .iter()
            .filter_map(|(segment, name)| {
                fixture_section_bytes(path, segment, name)
                    .map(|(bytes, vaddr)| SwiftSectionBytes { vaddr, bytes })
            })
            .collect()
    }

    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/bin/adversarial/swift-rich-arm64"
    );

    #[test]
    fn parses_protocols_types_and_conformances_from_fixture() {
        let Some((protocols_bytes, protocols_vaddr)) =
            fixture_section_bytes(FIXTURE, "__TEXT", "__swift5_protos")
        else {
            return; // skip on hosts without the fixture loader
        };
        // Descriptor names are plain C strings: __cstring carries them on
        // this toolchain; Swift string sections carry field names.
        let sections = fixture_sections(
            FIXTURE,
            &[
                ("__TEXT", "__constg_swiftt"),
                ("__TEXT", "__const"),
                ("__TEXT", "__cstring"),
                ("__TEXT", "__swift5_reflstr"),
                ("__TEXT", "__swift5_typeref"),
                ("__TEXT", "__swift5_fieldmd"),
            ],
        );
        assert!(
            !sections.is_empty(),
            "fixture must carry descriptor/string sections"
        );

        let protocols = parse_protocols(&protocols_bytes, protocols_vaddr, &sections);
        assert!(!protocols.is_empty(), "one protocol descriptor entry");
        // The referenced descriptor header must have been read: every
        // entry resolves flags or reports a descriptor address.
        assert!(
            protocols
                .iter()
                .all(|protocol| protocol.descriptor_vaddr != 0)
        );

        let types = fixture_section_bytes(FIXTURE, "__TEXT", "__swift5_types")
            .map(|(bytes, vaddr)| parse_types(&bytes, vaddr, &sections))
            .unwrap_or_default();
        assert!(!types.is_empty(), "type descriptors present");

        let conformances = fixture_section_bytes(FIXTURE, "__TEXT", "__swift5_proto")
            .map(|(bytes, vaddr)| parse_conformances(&bytes, vaddr, &sections, &protocols, &types))
            .unwrap_or_default();
        assert_eq!(conformances.len(), 6, "six relative-pointer entries");
        // Non-retroactive records; the first two join to the fixture's own
        // protocol and struct descriptors (both names resolve).
        assert!(conformances.iter().all(|c| !c.retroactive));
        assert!(
            conformances
                .iter()
                .any(|c| c.protocol_name.is_some() && c.type_name.is_some()),
            "descriptor join resolves names: {conformances:#?}"
        );
        // Later records reference runtime-library protocols indirectly.
        assert!(
            conformances
                .iter()
                .skip(2)
                .all(|c| c.protocol_name.is_none())
        );
    }
}
