//! User type declarations (plan feature 7, slice 1): structs persisted
//! through the fact store and applied to pseudocode rendering.
//!
//! v1 ships the *persistence and application contract*: `damsel type`
//! declares structs (`damsel type <project> --struct NAME
//! --field offset:type ...`), stored as annotation facts (`type_struct`
//! kind, actor-scoped like every annotation) and applied by the
//! pseudocode renderer — `x0 = &x1[0x10]` for a field offset inside a
//! typed pointer becomes `x0 = &typed_ptr->field`. DWARF/dSYM import and
//! inference arrive on this contract.

use damsel_project::{AnnotationKind, StoreHandle, put_annotation};
use std::collections::BTreeMap;

/// The user-type annotation kind (annotation facts with structured
/// payloads: `struct|name=NAME|field=0xOFF:TYPE,0xOFF:TYPE,…`).
pub const TYPE_FACT_MARKER: &str = "struct";

/// A declared struct field.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StructField {
    pub offset: u64,
    pub type_name: String,
    pub field_name: String,
}

/// A declared struct.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
}

impl StructDecl {
    /// Encode into the annotation payload format.
    pub fn payload(&self) -> String {
        let fields = self
            .fields
            .iter()
            .map(|f| format!("{:#x}:{}:{}", f.offset, f.type_name, f.field_name))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}|{}|{}", TYPE_FACT_MARKER, self.name, fields)
    }

    /// Decode from the annotation payload format.
    pub fn parse_payload(payload: &str) -> Option<Self> {
        let mut parts = payload.split('|');
        if parts.next()? != TYPE_FACT_MARKER {
            return None;
        }
        let name = parts.next()?.to_string();
        let mut fields = Vec::new();
        for field in parts.next()?.split(',').filter(|s| !s.is_empty()) {
            let mut segments = field.splitn(3, ':');
            let offset = u64::from_str_radix(segments.next()?.trim_start_matches("0x"), 16).ok()?;
            fields.push(StructField {
                offset,
                type_name: segments.next()?.to_string(),
                field_name: segments.next()?.to_string(),
            });
        }
        Some(StructDecl { name, fields })
    }

    /// Field at exact offset.
    pub fn field_at(&self, offset: u64) -> Option<&StructField> {
        self.fields.iter().find(|field| field.offset == offset)
    }
}

/// The user-type environment: declared structs plus pointer bindings
/// (base name → struct name), offset-applied.
#[derive(Debug, Clone, Default)]
pub struct TypeTable {
    pub structs: BTreeMap<String, StructDecl>,
    /// Base pointer name (lowercased) → struct name.
    pub bindings: BTreeMap<String, String>,
}

impl TypeTable {
    /// Load the table from a project's type annotations (actor-agnostic:
    /// user and model types apply equally).
    pub fn from_store(store: &StoreHandle) -> Result<Self, String> {
        let mut table = TypeTable::default();
        for annotation in damsel_project::list_annotations(store).map_err(|e| e.to_string())? {
            if annotation.kind != AnnotationKind::Tag {
                continue;
            }
            if let Some(decl) = StructDecl::parse_payload(&annotation.value) {
                table.structs.insert(decl.name.clone(), decl);
                continue;
            }
            if let Some((register, struct_name)) = parse_bind_payload(&annotation.value) {
                table
                    .bindings
                    .insert(register.to_ascii_lowercase(), struct_name);
            }
        }
        Ok(table)
    }

    /// Declare (or replace) a struct through the annotation layer.
    pub fn declare(store: &mut StoreHandle, decl: &StructDecl, actor: &str) -> Result<(), String> {
        let object = damsel_project::ObjectId::new("type", format!("struct:{}", decl.name));
        put_annotation(store, &object, AnnotationKind::Tag, &decl.payload(), actor)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Bind a base pointer name to a declared struct (annotation layer,
    /// actor-scoped, re-bind = replace).
    pub fn bind(
        store: &mut StoreHandle,
        register: &str,
        struct_name: &str,
        actor: &str,
    ) -> Result<(), String> {
        let object = damsel_project::ObjectId::new(
            "type",
            format!("bind:{}", register.to_ascii_lowercase()),
        );
        let payload = format!("bind|reg={register}|struct={struct_name}");
        put_annotation(store, &object, AnnotationKind::Tag, &payload, actor)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Apply typed field names to a base-relative access: given the
    /// pointer's base value expression text and a displacement, render
    /// `base->field` when a struct field matches the offset.
    pub fn render_field_access(&self, struct_name: &str, displacement: i64) -> Option<String> {
        let decl = self.structs.get(struct_name)?;
        let offset = u64::try_from(displacement).ok()?;
        let field = decl.field_at(offset)?;
        Some(format!("{struct_name}->{}", field.field_name))
    }
}

/// `bind|reg=fp|struct=Frame` → `("fp", "Frame")`.
fn parse_bind_payload(payload: &str) -> Option<(String, String)> {
    let mut parts = payload.split('|');
    if parts.next()? != "bind" {
        return None;
    }
    let register = parts.next()?.strip_prefix("reg=")?.to_string();
    let struct_name = parts.next()?.strip_prefix("struct=")?.to_string();
    if register.is_empty() || struct_name.is_empty() {
        return None;
    }
    Some((register, struct_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_payload_round_trips() {
        let decl = StructDecl {
            name: "DamselState".to_string(),
            fields: vec![
                StructField {
                    offset: 0x0,
                    type_name: "u64".to_string(),
                    field_name: "count".to_string(),
                },
                StructField {
                    offset: 0x10,
                    type_name: "char*".to_string(),
                    field_name: "name".to_string(),
                },
            ],
        };
        let payload = decl.payload();
        assert_eq!(StructDecl::parse_payload(&payload).as_ref(), Some(&decl));
        assert!(StructDecl::parse_payload("rename|to=x").is_none());
    }

    #[test]
    fn bind_payloads_parse() {
        assert_eq!(
            parse_bind_payload("bind|reg=fp|struct=Frame"),
            Some(("fp".to_string(), "Frame".to_string()))
        );
        assert_eq!(parse_bind_payload("struct|Frame|0x10:u64:x"), None);
        assert_eq!(parse_bind_payload("bind|reg=|struct=Frame"), None);
    }

    #[test]
    fn field_access_renders_for_declared_offsets() {
        let mut table = TypeTable::default();
        table.structs.insert(
            "DamselState".to_string(),
            StructDecl {
                name: "DamselState".to_string(),
                fields: vec![StructField {
                    offset: 0x10,
                    type_name: "char*".to_string(),
                    field_name: "name".to_string(),
                }],
            },
        );
        assert_eq!(
            table.render_field_access("DamselState", 0x10),
            Some("DamselState->name".to_string())
        );
        assert_eq!(table.render_field_access("DamselState", 0x20), None);
        assert_eq!(table.render_field_access("Unknown", 0x10), None);
    }
}
