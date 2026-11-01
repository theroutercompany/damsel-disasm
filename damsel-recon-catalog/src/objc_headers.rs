//! Objective-C header reconstruction (plan feature 4, slice 2): the
//! class-dump successor. Emits `@interface` / `@protocol` declarations
//! from the runtime metadata damsel already extracts — classes with
//! superclasses and adopted protocols, method signatures decoded from
//! type encodings, properties, ivars, and categories.
//!
//! Honesty rules (matching the decompiler's provenance philosophy):
//! - types decode from Objective-C type encodings; anything unmapped or
//!   struct/array-complex renders as `void *` carrying the raw encoding in
//!   a comment — never a guessed layout
//! - method argument names derive from selector segments (classic
//!   class-dump behavior) with `argN` fallbacks
//! - `imp 0x…` comments are provenance addresses, not source claims

use damsel_core::{
    ObjcCategoryRecord, ObjcClassRecord, ObjcMetadata, ObjcMethodRecord, ObjcProtocolRecord,
};

/// Decode the first type in an Objective-C type encoding, returning
/// `(c_type, consumed, fully_decoded)`.
fn decode_one(encoding: &[u8]) -> Option<(String, usize, bool)> {
    let first = *encoding.first()?;
    let (ty, len, clean) = match first {
        b'v' => ("void".to_string(), 1, true),
        b'V' => ("void".to_string(), 1, true),
        b'c' => ("char".to_string(), 1, true),
        b'C' => ("unsigned char".to_string(), 1, true),
        b's' => ("short".to_string(), 1, true),
        b'S' => ("unsigned short".to_string(), 1, true),
        b'i' => ("int".to_string(), 1, true),
        b'I' => ("unsigned int".to_string(), 1, true),
        b'l' => ("long".to_string(), 1, true),
        b'L' => ("unsigned long".to_string(), 1, true),
        b'q' => ("long long".to_string(), 1, true),
        b'Q' => ("unsigned long long".to_string(), 1, true),
        b'f' => ("float".to_string(), 1, true),
        b'd' => ("double".to_string(), 1, true),
        b'B' => ("BOOL".to_string(), 1, true),
        b'@' => {
            if encoding.len() > 1 && encoding[1] == b'?' {
                ("id".to_string(), 2, true) // block
            } else {
                ("id".to_string(), 1, true)
            }
        }
        b':' => ("SEL".to_string(), 1, true),
        b'#' => ("Class".to_string(), 1, true),
        b'*' => ("const char *".to_string(), 1, true),
        b'^' => {
            let (pointee, len, clean) = decode_one(&encoding[1..])?;
            (format!("{pointee} *"), len + 1, clean)
        }
        // Structs/arrays/unions render opaquely with the raw encoding.
        b'{' | b'[' | b'(' => {
            let (open, close) = match first {
                b'{' => (b'{', b'}'),
                b'[' => (b'[', b']'),
                _ => (b'(', b')'),
            };
            let mut depth = 0usize;
            let mut index = 0usize;
            while index < encoding.len() {
                if encoding[index] == open {
                    depth += 1;
                } else if encoding[index] == close {
                    depth -= 1;
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                index += 1;
            }
            let raw = String::from_utf8_lossy(&encoding[..index]).into_owned();
            (format!("void * /* {raw} */"), index, false)
        }
        _ => return None,
    };
    Some((ty, len, clean))
}

/// Decode a full method encoding into `(return_type, arg_types)`.
/// Argument list skips the implicit `self`/`_cmd` pair.
fn decode_method_encoding(encoding: &str) -> (String, Vec<String>, bool) {
    let bytes = encoding.as_bytes();
    let mut offset = 0usize;
    let mut clean = true;
    let mut next_type = |offset: &mut usize| -> Option<String> {
        let (ty, len, ok) = decode_one(&bytes[*offset..])?;
        *offset += len;
        // Skip per-argument size/offset digits.
        while *offset < bytes.len() && bytes[*offset].is_ascii_digit() {
            *offset += 1;
        }
        if !ok {
            clean = false;
        }
        Some(ty)
    };
    let Some(return_type) = next_type(&mut offset) else {
        return (format!("void * /* {encoding} */"), Vec::new(), false);
    };
    let mut args = Vec::new();
    while offset < bytes.len() {
        match next_type(&mut offset) {
            Some(ty) => args.push(ty),
            None => {
                clean = false;
                break;
            }
        }
    }
    // self, _cmd
    if args.len() >= 2 {
        args.drain(0..2);
    }
    (return_type, args, clean)
}

/// Derive argument names from selector segments (`doThing:withStuff:` →
/// `doThing`, `withStuff`), falling back to `argN`.
fn argument_names(selector: &str, count: usize) -> Vec<String> {
    let segments: Vec<String> = selector
        .split(':')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect();
    let mut names = Vec::with_capacity(count);
    for index in 0..count {
        let name = segments
            .get(index + 1)
            .cloned()
            .unwrap_or_else(|| format!("arg{index}"));
        let mut chars = name.chars();
        match chars.next() {
            Some(first) if first.is_ascii_uppercase() => {
                let lowered = first.to_ascii_lowercase();
                let mut renamed = String::with_capacity(name.len());
                renamed.push(lowered);
                renamed.extend(chars);
                names.push(renamed);
            }
            _ => names.push(name),
        }
    }
    names
}

fn render_method(method: &ObjcMethodRecord, indent: &str) -> String {
    let selector = method.selector.clone().unwrap_or_default();
    let (return_type, arg_types, clean) = method
        .type_encoding
        .as_deref()
        .map(decode_method_encoding)
        .unwrap_or_else(|| ("id".to_string(), Vec::new(), false));
    let names = argument_names(&selector, arg_types.len());
    let mut parts = selector.split(':').filter(|p| !p.is_empty());
    let first = parts.next().unwrap_or("?");
    let mut line = format!(
        "{indent}{} ({return_type}){first}",
        if method.is_class_method { "+" } else { "-" }
    );
    for (index, arg_type) in arg_types.iter().enumerate() {
        let label = parts.next();
        let name = names
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("arg{index}"));
        match label {
            Some(label) if index > 0 => line.push_str(&format!(" {label}:({arg_type}){name}")),
            _ => line.push_str(&format!(":({arg_type}){name}")),
        }
    }
    line.push(';');
    if !clean || method.type_encoding.is_none() {
        line.push_str(&format!(
            " // enc: {}",
            method.type_encoding.as_deref().unwrap_or("?")
        ));
    }
    if let Some(imp) = method.implementation {
        line.push_str(&format!(" // imp {imp:#x}"));
    }
    line
}

fn property_type_and_flags(attributes: &str) -> (String, String, bool) {
    // Attribute format: T<encoding>,<flag>,...,V<ivar>
    let mut property_type = "id".to_string();
    let mut clean = true;
    if let Some(rest) = attributes.strip_prefix('T') {
        let end = rest.find(',').unwrap_or(rest.len());
        let encoding = &rest[..end];
        match decode_one(encoding.as_bytes()) {
            Some((ty, len, ok)) if len == encoding.len() => {
                property_type = ty;
                clean = ok;
            }
            Some((ty, _, ok)) => {
                property_type = ty;
                clean = ok;
            }
            None => {
                property_type = format!("id /* T{encoding} */");
                clean = false;
            }
        }
    }
    let mut flags: Vec<&str> = Vec::new();
    if attributes.contains(",R") {
        flags.push("readonly");
    }
    if attributes.contains(",N") {
        flags.push("nonatomic");
    }
    if attributes.contains(",C") {
        flags.push("copy");
    }
    (property_type, flags.join(", "), clean)
}

fn render_class(class: &ObjcClassRecord) -> String {
    let name = class.name.clone().unwrap_or_else(|| "Unknown".to_string());
    let mut out = String::new();
    let supers = class
        .superclass_name
        .clone()
        .unwrap_or_else(|| "NSObject".to_string());
    if class.adopted_protocols.is_empty() {
        out.push_str(&format!("@interface {name} : {supers}\n"));
    } else {
        out.push_str(&format!(
            "@interface {name} : {supers} <{}>\n",
            class.adopted_protocols.join(", ")
        ));
    }
    if !class.ivars.is_empty() {
        out.push_str("{\n");
        for ivar in &class.ivars {
            let ty = ivar
                .type_encoding
                .as_deref()
                .and_then(|enc| decode_one(enc.as_bytes()))
                .map(|(ty, _, _)| ty)
                .unwrap_or_else(|| "id".to_string());
            out.push_str(&format!(
                "    {} {};\n",
                ty,
                ivar.name.clone().unwrap_or_else(|| "_".to_string())
            ));
        }
        out.push_str("}\n");
    }
    for property in &class.properties {
        let attributes = property.attributes.as_deref().unwrap_or("");
        let (ty, flags, _) = property_type_and_flags(attributes);
        if flags.is_empty() {
            out.push_str(&format!(
                "@property {} {};\n",
                ty,
                property.name.clone().unwrap_or_default()
            ));
        } else {
            out.push_str(&format!(
                "@property ({}) {} {};\n",
                flags,
                ty,
                property.name.clone().unwrap_or_default()
            ));
        }
    }
    for method in &class.class_methods {
        out.push_str(&render_method(method, ""));
        out.push('\n');
    }
    for method in &class.methods {
        out.push_str(&render_method(method, ""));
        out.push('\n');
    }
    out.push_str("@end\n");
    out
}

fn render_protocol(protocol: &ObjcProtocolRecord) -> String {
    let name = protocol
        .name
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let mut out = format!("@protocol {name}\n");
    let mut optional = false;
    for method in protocol
        .required_class_methods
        .iter()
        .chain(protocol.required_instance_methods.iter())
    {
        out.push_str(&render_method(method, ""));
        out.push('\n');
    }
    if !(protocol.optional_class_methods.is_empty()
        && protocol.optional_instance_methods.is_empty())
    {
        optional = true;
        out.push_str("@optional\n");
        for method in protocol
            .optional_class_methods
            .iter()
            .chain(protocol.optional_instance_methods.iter())
        {
            out.push_str(&render_method(method, ""));
            out.push('\n');
        }
    }
    let _ = optional;
    for property in &protocol.properties {
        let attributes = property.attributes.as_deref().unwrap_or("");
        let (ty, _, _) = property_type_and_flags(attributes);
        out.push_str(&format!(
            "@property {} {};\n",
            ty,
            property.name.clone().unwrap_or_default()
        ));
    }
    out.push_str("@end\n");
    out
}

fn render_category(category: &ObjcCategoryRecord) -> String {
    let host = category
        .class_name
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let name = category.name.clone().unwrap_or_default();
    let mut out = if category.adopted_protocols.is_empty() {
        format!("@interface {host} ({name})\n")
    } else {
        format!(
            "@interface {host} ({name}) <{}>\n",
            category.adopted_protocols.join(", ")
        )
    };
    for property in &category.properties {
        let attributes = property.attributes.as_deref().unwrap_or("");
        let (ty, flags, _) = property_type_and_flags(attributes);
        if flags.is_empty() {
            out.push_str(&format!(
                "@property {} {};\n",
                ty,
                property.name.clone().unwrap_or_default()
            ));
        } else {
            out.push_str(&format!(
                "@property ({}) {} {};\n",
                flags,
                ty,
                property.name.clone().unwrap_or_default()
            ));
        }
    }
    for method in &category.class_methods {
        out.push_str(&render_method(method, ""));
        out.push('\n');
    }
    for method in &category.methods {
        out.push_str(&render_method(method, ""));
        out.push('\n');
    }
    out.push_str("@end\n");
    out
}

/// Render the full header file for a binary's Objective-C metadata.
pub fn render_headers(metadata: &ObjcMetadata, damsel_version: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "// Generated by damsel {damsel_version} from Objective-C runtime metadata.\n\
         // Types decode from type encodings; undecodable encodings carry the raw\n\
         // string in comments (never a guessed layout). `imp` comments are provenance\n\
         // addresses, not claims about original source.\n\n"
    ));
    let mut classes: Vec<&ObjcClassRecord> = metadata.classes.iter().collect();
    classes.sort_by_key(|class| class.name.clone().unwrap_or_default());
    let mut protocols: Vec<&ObjcProtocolRecord> = metadata.protocols.iter().collect();
    protocols.sort_by_key(|protocol| protocol.name.clone().unwrap_or_default());
    let mut categories: Vec<&ObjcCategoryRecord> = metadata.categories.iter().collect();
    categories.sort_by_key(|category| {
        (
            category.class_name.clone().unwrap_or_default(),
            category.name.clone().unwrap_or_default(),
        )
    });

    if !protocols.is_empty() {
        out.push_str("// Protocols\n\n");
        for protocol in &protocols {
            out.push_str(&render_protocol(protocol));
            out.push('\n');
        }
    }
    if !classes.is_empty() {
        out.push_str("// Classes\n\n");
        for class in &classes {
            out.push_str(&render_class(class));
            out.push('\n');
        }
    }
    if !categories.is_empty() {
        out.push_str("// Categories\n\n");
        for category in &categories {
            out.push_str(&render_category(category));
            out.push('\n');
        }
    }
    out
}
