//! Swift symbol demangling (Wave 8.3): `_$s`-prefixed mangled names to
//! readable `Module.function` form. Handles length-prefixed identifiers,
//! common type abbreviations, and special suffixes (default arguments,
//! partial applies, static/class methods).

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
    let (module, after_module): (&str, &str) =
        if let Some(r) = rest.strip_prefix("ss") {
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
            (
                member,
                format!("{module}.{first}.{member}"),
                rest,
            )
        } else {
            (first, format!("{module}.{first}"), after_first)
        }
    } else {
        (first, format!("{module}.{first}"), after_first)
    };

    // Check for special suffixes in the type section.
    let type_section = after_name;
    let is_thunk = type_section.contains("FfA")
        || type_section.contains("TW")
        || type_section.contains("TJ");

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    sections.iter().any(|(name, _)| name.starts_with("__swift5_"))
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
