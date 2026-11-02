//! DWARF import (plan feature 7, slice 3): struct declarations
//! extracted from a binary's DWARF debug info and landed in the type
//! system through the annotation layer — `damsel type --import-dwarf`
//! closes the loop from compiler debug info to typed pseudocode.
//!
//! Uses `object` for `__debug_*` sections and `gimli` for DIE
//! parsing: every `DW_TAG_structure_type` with a name yields its
//! members (offset from `DW_AT_data_member_location`, field name from
//! `DW_AT_name`, type name resolved through `DW_AT_type` references —
//! basic types render as C-ish names, pointers suffix `*`, nested
//! structs by their DWARF name). Anonymous structs and unions are
//! skipped v1 (honest surface, not a fabricating one).

use crate::errors::MachoError;
use gimli::{AttributeValue, DW_TAG_structure_type, Unit};
use object::read::macho::MachOFile64;
use object::{Endianness, Object, ObjectSection};
use std::collections::BTreeMap;
use std::path::Path;

/// One DWARF-extracted struct field.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DwarfField {
    pub offset: u64,
    pub type_name: String,
    pub field_name: String,
}

/// One DWARF-extracted struct declaration.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DwarfStruct {
    pub name: String,
    pub byte_size: Option<u64>,
    pub fields: Vec<DwarfField>,
}

/// Extract named struct declarations from a Mach-O file's DWARF.
pub fn import_structs(path: &Path) -> Result<Vec<DwarfStruct>, MachoError> {
    let data = std::fs::read(path).map_err(|error| {
        MachoError::MalformedSharedCache(format!("dwarf read {}: {error}", path.display()))
    })?;
    let file = MachOFile64::<Endianness, &[u8]>::parse(&data).map_err(|error| {
        MachoError::MalformedSharedCache(format!("dwarf parse {}: {error}", path.display()))
    })?;
    import_structs_from(&file)
}

/// Extract from an already-parsed Mach-O (test surface).
pub fn import_structs_from<'data>(
    file: &MachOFile64<'data, Endianness, &'data [u8]>,
) -> Result<Vec<DwarfStruct>, MachoError> {
    // Load every __debug section by its gimli SectionId.
    let dwarf = gimli::Dwarf::load(|id| {
        let bytes: std::borrow::Cow<[u8]> = file
            .section_by_name(id.name())
            .and_then(|section| section.uncompressed_data().ok())
            .unwrap_or_default();
        Ok::<_, gimli::Error>(gimli::EndianSlice::new(
            match bytes {
                std::borrow::Cow::Borrowed(slice) => slice,
                std::borrow::Cow::Owned(owned) => owned.leak(),
            },
            gimli::LittleEndian,
        ))
    })
    .map_err(|error| MachoError::MalformedSharedCache(format!("dwarf sections: {error}")))?;

    let mut structs: BTreeMap<String, DwarfStruct> = BTreeMap::new();
    let mut units = dwarf.units();
    while let Some(header) = units.next().map_err(dwarf_error)? {
        let unit = dwarf.unit(header).map_err(dwarf_error)?;
        let abbrevs = dwarf.abbreviations(&unit.header).map_err(dwarf_error)?;
        let mut tree = unit
            .header
            .entries_tree(&abbrevs, None)
            .map_err(dwarf_error)?;
        let root = tree.root().map_err(dwarf_error)?;
        walk_entries(&dwarf, &unit, root, &mut structs)?;
    }
    Ok(structs.into_values().collect())
}

fn dwarf_error(error: gimli::Error) -> MachoError {
    MachoError::MalformedSharedCache(format!("dwarf traversal: {error}"))
}

fn walk_entries(
    dwarf: &gimli::Dwarf<gimli::EndianSlice<gimli::LittleEndian>>,
    unit: &Unit<gimli::EndianSlice<gimli::LittleEndian>>,
    tree: gimli::EntriesTreeNode<gimli::EndianSlice<gimli::LittleEndian>>,
    structs: &mut BTreeMap<String, DwarfStruct>,
) -> Result<(), MachoError> {
    let entry = tree.entry();
    if entry.tag() == DW_TAG_structure_type {
        return walk_struct_tree(dwarf, unit, tree, structs);
    }
    let mut child = tree.children();
    while let Some(child_node) = child.next().map_err(dwarf_error)? {
        walk_entries(dwarf, unit, child_node, structs)?;
    }
    Ok(())
}

/// A structure_type node: extract its members (consuming the node),
/// then stop — struct children are members, not nested scopes.
fn walk_struct_tree(
    dwarf: &gimli::Dwarf<gimli::EndianSlice<gimli::LittleEndian>>,
    unit: &Unit<gimli::EndianSlice<gimli::LittleEndian>>,
    tree: gimli::EntriesTreeNode<gimli::EndianSlice<gimli::LittleEndian>>,
    structs: &mut BTreeMap<String, DwarfStruct>,
) -> Result<(), MachoError> {
    let entry = tree.entry();
    let Some(name) = name_of(dwarf, entry, unit) else {
        return Ok(());
    };
    let byte_size = match entry.attr_value(gimli::DW_AT_byte_size) {
        Ok(Some(AttributeValue::Udata(size))) => Some(size),
        _ => None,
    };
    let mut fields = Vec::new();
    let mut child = tree.children();
    while let Some(child_node) = child.next().map_err(dwarf_error)? {
        let child_entry = child_node.entry();
        if child_entry.tag() != gimli::DW_TAG_member {
            continue;
        }
        let Some(field_name) = name_of(dwarf, child_entry, unit) else {
            continue;
        };
        let offset = match child_entry.attr_value(gimli::DW_AT_data_member_location) {
            Ok(Some(AttributeValue::Udata(offset))) => offset,
            Ok(Some(AttributeValue::LocationListsRef(_))) => continue,
            _ => 0,
        };
        let type_name = match child_entry.attr_value(gimli::DW_AT_type) {
            Ok(Some(reference)) => {
                type_name_of(dwarf, unit, reference).unwrap_or_else(|| "?".to_string())
            }
            _ => "?".to_string(),
        };
        fields.push(DwarfField {
            offset,
            type_name,
            field_name,
        });
    }
    structs.insert(
        name.clone(),
        DwarfStruct {
            name,
            byte_size,
            fields,
        },
    );
    Ok(())
}

fn name_of<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    entry: &gimli::DebuggingInformationEntry<R>,
    unit: &Unit<R>,
) -> Option<String> {
    let value = entry.attr_value(gimli::DW_AT_name).ok()??;
    let name = dwarf.attr_string(unit, value).ok()?;
    name.to_string_lossy().ok().map(|s| s.into_owned())
}

/// Resolve a DW_AT_type reference to a C-ish name: basic integer
/// classes (u8..u64, char), pointers (`T *`), and named composites by
/// their DWARF name.
fn type_name_of(
    dwarf: &gimli::Dwarf<gimli::EndianSlice<gimli::LittleEndian>>,
    unit: &Unit<gimli::EndianSlice<gimli::LittleEndian>>,
    reference: AttributeValue<gimli::EndianSlice<gimli::LittleEndian>>,
) -> Option<String> {
    // Reference offset within the unit (v1: unit-relative refs only,
    // the form clang emits for these fixtures).
    let offset = match reference {
        AttributeValue::UnitRef(offset) => offset,
        AttributeValue::DebugInfoRef(offset) => offset.to_unit_offset(&unit.header)?,
        _ => return None,
    };
    let abbrevs = dwarf.abbreviations(&unit.header).ok()?;
    let mut entries = unit.header.entries_at_offset(&abbrevs, offset).ok()?;
    let entry = entries.next_dfs().ok()?.map(|(_, entry)| entry)?;
    match entry.tag() {
        gimli::DW_TAG_base_type => {
            let byte_size = match entry.attr_value(gimli::DW_AT_byte_size) {
                Ok(Some(AttributeValue::Udata(size))) => size,
                _ => return Some("int".to_string()),
            };
            let name = name_of(dwarf, entry, unit).unwrap_or_default();
            Some(base_type_name(&name, byte_size))
        }
        gimli::DW_TAG_pointer_type => {
            let inner = entry.attr_value(gimli::DW_AT_type).ok()?;
            let inner_name = inner
                .and_then(|reference| type_name_of(dwarf, unit, reference))
                .unwrap_or_else(|| "void".to_string());
            Some(format!("{inner_name} *"))
        }
        gimli::DW_TAG_structure_type
        | gimli::DW_TAG_union_type
        | gimli::DW_TAG_enumeration_type => name_of(dwarf, entry, unit),
        gimli::DW_TAG_typedef => name_of(dwarf, entry, unit),
        _ => None,
    }
}

fn base_type_name(dwarf_name: &str, byte_size: u64) -> String {
    let lowered = dwarf_name.to_ascii_lowercase();
    if lowered.contains("char") {
        return "char".to_string();
    }
    let unsigned = lowered.starts_with('u') || lowered.contains("unsigned");
    match (byte_size, unsigned) {
        (1, true) => "uint8".to_string(),
        (1, false) => "int8".to_string(),
        (2, true) => "uint16".to_string(),
        (2, false) => "int16".to_string(),
        (4, true) => "uint32".to_string(),
        (4, false) => "int32".to_string(),
        (8, true) => "uint64".to_string(),
        (8, false) => "int64".to_string(),
        _ => dwarf_name.to_string(),
    }
}
