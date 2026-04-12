use damsel_core::{
    ObjcCategoryRecord, ObjcCategoryRecordSource, ObjcClassRecord, ObjcIvarRecord, ObjcMetadata,
    ObjcMethodOwnerKind, ObjcMethodRecord, ObjcNameSource, ObjcPointerKind, ObjcPointerRef,
    ObjcPropertyRecord, ObjcProtocolRecord, ObjcSelectorSource, Section, Symbol,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn collect_objc_metadata(
    bytes: &[u8],
    sections: &[Section],
    symbols: &[Symbol],
) -> ObjcMetadata {
    let mut metadata = ObjcMetadata::default();
    let section_slices = collect_section_slices(bytes, sections);
    let image_base = infer_image_base(sections);
    let symbol_maps = collect_objc_symbol_maps(symbols);
    let mut legacy_class_names = Vec::new();
    let mut legacy_selector_names = Vec::new();
    let mut legacy_method_names = Vec::new();

    for section in &section_slices {
        match section.section.name.as_str() {
            "__objc_classname" => legacy_class_names.extend(
                read_c_strings(section.contents)
                    .into_iter()
                    .filter(|name| is_plausible_objc_type_name(name)),
            ),
            "__objc_methname" => {
                let names = read_c_strings(section.contents);
                legacy_method_names.extend(
                    names
                        .iter()
                        .filter(|name| is_plausible_selector_name(name))
                        .cloned(),
                );
                legacy_selector_names.extend(
                    names
                        .into_iter()
                        .filter(|name| is_plausible_selector_name(name)),
                );
            }
            "__objc_imageinfo" if section.contents.len() >= 8 => {
                metadata.image_info_flags = Some(u32::from_le_bytes([
                    section.contents[4],
                    section.contents[5],
                    section.contents[6],
                    section.contents[7],
                ]));
            }
            _ => {}
        }
    }

    // Bounded structured extraction pass:
    // 1) collect typed ObjC pointer refs from canonical selref/class tables
    // 2) populate class/protocol/category runtime records
    // 3) derive compatibility flat views from structured records + legacy pools
    let pointer_tables = collect_pointer_tables(&section_slices, image_base, &symbol_maps);
    let mut pointer_refs = Vec::new();
    pointer_refs.extend(pointer_tables.selrefs.iter().cloned());
    pointer_refs.extend(pointer_tables.classrefs.iter().cloned());
    pointer_refs.extend(pointer_tables.classlist.iter().cloned());
    pointer_refs.sort_by_key(|entry| (entry.table_address, pointer_kind_sort_key(entry.kind)));
    pointer_refs.dedup_by(|left, right| {
        left.table_address == right.table_address
            && left.kind == right.kind
            && left.raw_pointer == right.raw_pointer
    });
    metadata.pointer_refs = pointer_refs;
    metadata.classes =
        collect_class_records(&pointer_tables, &section_slices, image_base, &symbol_maps);
    metadata.protocols = collect_protocol_records(&section_slices, image_base, &symbol_maps);
    metadata.categories =
        collect_category_records(&section_slices, image_base, &metadata.classes, &symbol_maps);

    derive_compatibility_views(
        &mut metadata,
        &legacy_class_names,
        &legacy_selector_names,
        &legacy_method_names,
    );
    metadata
}

fn read_c_strings(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|slice| !slice.is_empty())
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
        .collect()
}

fn is_plausible_selector_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | ':'))
}

fn is_plausible_objc_type_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(ch, '_' | '$' | ':' | '(' | ')' | '.' | '+' | '-')
        })
}

#[derive(Debug, Default)]
struct ObjcSymbolMaps {
    class_names: BTreeMap<u64, String>,
    metaclass_names: BTreeMap<u64, String>,
    protocol_names: BTreeMap<u64, String>,
    category_hints_by_list_pointer: BTreeMap<u64, ObjcCategorySymbolHint>,
    category_method_symbols: Vec<ObjcCategoryMethodSymbol>,
    category_list_symbols: Vec<ObjcCategoryListSymbol>,
}

#[derive(Debug, Clone)]
struct ObjcCategorySymbolHint {
    class_name: String,
    category_name: String,
}

#[derive(Debug, Clone)]
struct ObjcCategoryMethodSymbol {
    class_name: String,
    category_name: String,
    selector: String,
    implementation: u64,
    is_class_method: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjcCategoryListKind {
    Properties,
    Protocols,
}

#[derive(Debug, Clone)]
struct ObjcCategoryListSymbol {
    class_name: String,
    category_name: String,
    pointer: u64,
    kind: ObjcCategoryListKind,
}

fn collect_objc_symbol_maps(symbols: &[Symbol]) -> ObjcSymbolMaps {
    let mut maps = ObjcSymbolMaps::default();
    for symbol in symbols {
        if let Some(name) = symbol.name.strip_prefix("_OBJC_CLASS_$_") {
            maps.class_names.insert(symbol.address, name.to_string());
        } else if let Some(name) = symbol.name.strip_prefix("_OBJC_METACLASS_$_") {
            maps.metaclass_names
                .insert(symbol.address, name.to_string());
        } else if let Some(name) = symbol.name.strip_prefix("__OBJC_PROTOCOL_$_") {
            maps.protocol_names.insert(symbol.address, name.to_string());
        } else if let Some(name) = symbol.name.strip_prefix("_OBJC_PROTOCOL_$_") {
            maps.protocol_names.insert(symbol.address, name.to_string());
        } else if let Some(hints) = parse_category_list_symbol(&symbol.name, symbol.address) {
            maps.category_list_symbols.extend(hints);
            if let Some(hint) = parse_category_hint_symbol(&symbol.name) {
                maps.category_hints_by_list_pointer
                    .insert(symbol.address, hint);
            }
        } else if let Some(hint) = parse_category_hint_symbol(&symbol.name) {
            maps.category_hints_by_list_pointer
                .insert(symbol.address, hint);
        } else if let Some(hints) = parse_category_method_symbol(&symbol.name, symbol.address) {
            maps.category_method_symbols.extend(hints);
        }
    }
    maps
}

fn parse_category_hint_symbol(symbol_name: &str) -> Option<ObjcCategorySymbolHint> {
    const PREFIXES: [&str; 4] = [
        "__OBJC_$_INSTANCE_METHODS_",
        "__OBJC_$_CLASS_METHODS_",
        "__OBJC_CLASS_PROTOCOLS_$_",
        "__OBJC_$_PROP_LIST_",
    ];
    for prefix in PREFIXES {
        if let Some(rest) = symbol_name.strip_prefix(prefix) {
            return parse_category_hint_rest(rest).map(|(class_name, category_name)| {
                ObjcCategorySymbolHint {
                    class_name,
                    category_name,
                }
            });
        }
    }
    None
}

fn parse_category_hint_rest(value: &str) -> Option<(String, String)> {
    let open_index = value.find('(')?;
    let close_index = value.rfind(')')?;
    if close_index <= open_index + 1 || close_index + 1 != value.len() {
        return None;
    }
    let class_name = value[..open_index].trim();
    let category_name = value[open_index + 1..close_index].trim();
    if !is_plausible_objc_type_name(class_name) || !is_plausible_objc_type_name(category_name) {
        return None;
    }
    Some((class_name.to_string(), category_name.to_string()))
}

fn parse_category_list_symbol(
    symbol_name: &str,
    pointer: u64,
) -> Option<Vec<ObjcCategoryListSymbol>> {
    let (kind, rest) = if let Some(rest) = symbol_name.strip_prefix("__OBJC_$_PROP_LIST_") {
        (ObjcCategoryListKind::Properties, rest)
    } else if let Some(rest) = symbol_name.strip_prefix("__OBJC_CLASS_PROTOCOLS_$_") {
        (ObjcCategoryListKind::Protocols, rest)
    } else {
        return None;
    };
    let (class_name, category_names) = parse_category_hint_rest(rest)?;
    let category_names = category_names
        .split('|')
        .map(str::trim)
        .filter(|name| is_plausible_objc_type_name(name))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if category_names.is_empty() {
        return None;
    }
    Some(
        category_names
            .into_iter()
            .map(|category_name| ObjcCategoryListSymbol {
                class_name: class_name.clone(),
                category_name,
                pointer,
                kind,
            })
            .collect(),
    )
}

fn parse_category_method_symbol(
    symbol_name: &str,
    implementation: u64,
) -> Option<Vec<ObjcCategoryMethodSymbol>> {
    let (is_class_method, rest) = if let Some(rest) = symbol_name.strip_prefix("+[") {
        (true, rest)
    } else if let Some(rest) = symbol_name.strip_prefix("-[") {
        (false, rest)
    } else {
        return None;
    };
    let body = rest.strip_suffix(']')?;
    let space_index = body.rfind(' ')?;
    let owner = body[..space_index].trim();
    let selector = body[space_index + 1..].trim();
    if !is_plausible_selector_name(selector) {
        return None;
    }
    let (class_name, category_name) = parse_category_hint_rest(owner)?;
    let category_names = category_name
        .split('|')
        .map(str::trim)
        .filter(|name| is_plausible_objc_type_name(name))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if category_names.is_empty() {
        return None;
    }
    Some(
        category_names
            .into_iter()
            .map(|category_name| ObjcCategoryMethodSymbol {
                class_name: class_name.clone(),
                category_name,
                selector: selector.to_string(),
                implementation,
                is_class_method,
            })
            .collect(),
    )
}

fn pointer_kind_sort_key(kind: ObjcPointerKind) -> u8 {
    match kind {
        ObjcPointerKind::SelRef => 0,
        ObjcPointerKind::ClassRef => 1,
        ObjcPointerKind::ClassList => 2,
    }
}

fn derive_compatibility_views(
    metadata: &mut ObjcMetadata,
    legacy_class_names: &[String],
    legacy_selector_names: &[String],
    legacy_method_names: &[String],
) {
    let mut class_names = Vec::new();
    class_names.extend(
        legacy_class_names
            .iter()
            .filter(|name| is_plausible_objc_type_name(name))
            .cloned(),
    );
    class_names.extend(
        metadata
            .pointer_refs
            .iter()
            .filter(|entry| {
                matches!(
                    entry.kind,
                    ObjcPointerKind::ClassRef | ObjcPointerKind::ClassList
                )
            })
            .filter_map(|entry| entry.resolved_name.clone())
            .filter(|name| is_plausible_objc_type_name(name)),
    );
    class_names.extend(
        metadata
            .classes
            .iter()
            .filter_map(|record| record.name.clone())
            .filter(|name| is_plausible_objc_type_name(name)),
    );
    class_names.extend(
        metadata
            .categories
            .iter()
            .filter_map(|record| record.class_name.clone())
            .filter(|name| is_plausible_objc_type_name(name)),
    );
    class_names.extend(
        metadata
            .protocols
            .iter()
            .filter_map(|record| record.name.clone())
            .filter(|name| is_plausible_objc_type_name(name)),
    );
    class_names.extend(
        metadata
            .categories
            .iter()
            .filter_map(|record| record.name.clone())
            .filter(|name| is_plausible_objc_type_name(name)),
    );
    class_names.sort();
    class_names.dedup();
    metadata.class_names = class_names;

    let mut selector_names = Vec::new();
    selector_names.extend(
        legacy_selector_names
            .iter()
            .filter(|name| is_plausible_selector_name(name))
            .cloned(),
    );
    selector_names.extend(
        metadata
            .pointer_refs
            .iter()
            .filter(|entry| matches!(entry.kind, ObjcPointerKind::SelRef))
            .filter_map(|entry| entry.resolved_name.clone())
            .filter(|name| is_plausible_selector_name(name)),
    );
    selector_names.sort();
    selector_names.dedup();
    metadata.selector_names = selector_names;

    let mut method_names = legacy_method_names
        .iter()
        .filter(|name| is_plausible_selector_name(name))
        .cloned()
        .collect::<Vec<_>>();
    method_names.extend(
        metadata
            .classes
            .iter()
            .flat_map(|record| record.methods.iter().chain(record.class_methods.iter()))
            .chain(
                metadata.protocols.iter().flat_map(|record| {
                    record
                        .required_instance_methods
                        .iter()
                        .chain(record.required_class_methods.iter())
                        .chain(record.optional_instance_methods.iter())
                        .chain(record.optional_class_methods.iter())
                }),
            )
            .chain(
                metadata
                    .categories
                    .iter()
                    .flat_map(|record| record.methods.iter().chain(record.class_methods.iter())),
            )
            .filter_map(|method| method.selector.clone())
            .filter(|name| is_plausible_selector_name(name)),
    );
    method_names.sort();
    method_names.dedup();
    metadata.method_names = method_names;
}

#[derive(Debug, Default)]
struct ObjcPointerTables {
    selrefs: Vec<ObjcPointerRef>,
    classrefs: Vec<ObjcPointerRef>,
    classlist: Vec<ObjcPointerRef>,
}

#[derive(Debug, Clone, Copy)]
struct SectionSlice<'a> {
    section: &'a Section,
    contents: &'a [u8],
}

fn collect_section_slices<'a>(bytes: &'a [u8], sections: &'a [Section]) -> Vec<SectionSlice<'a>> {
    let mut slices = Vec::new();
    for section in sections {
        let Some(offset) = section.file_offset else {
            continue;
        };
        let Ok(start) = usize::try_from(offset) else {
            continue;
        };
        let Ok(file_size) = usize::try_from(section.file_size) else {
            continue;
        };
        let Some(end) = start.checked_add(file_size) else {
            continue;
        };
        let Some(contents) = bytes.get(start..end) else {
            continue;
        };
        slices.push(SectionSlice { section, contents });
    }
    slices
}

fn collect_pointer_tables(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> ObjcPointerTables {
    ObjcPointerTables {
        selrefs: collect_pointer_table(
            slices,
            image_base,
            symbol_maps,
            "__objc_selrefs",
            ObjcPointerKind::SelRef,
        ),
        classrefs: collect_pointer_table(
            slices,
            image_base,
            symbol_maps,
            "__objc_classrefs",
            ObjcPointerKind::ClassRef,
        ),
        classlist: collect_pointer_table(
            slices,
            image_base,
            symbol_maps,
            "__objc_classlist",
            ObjcPointerKind::ClassList,
        ),
    }
}

fn collect_pointer_table(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
    table_section_name: &str,
    kind: ObjcPointerKind,
) -> Vec<ObjcPointerRef> {
    let mut refs = Vec::new();

    for section in slices
        .iter()
        .filter(|section| section.section.name == table_section_name)
    {
        for (entry_index, chunk) in section.contents.chunks_exact(8).enumerate() {
            let raw_pointer = u64::from_le_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
            if raw_pointer == 0 {
                continue;
            }

            let resolved_address = resolve_pointer_to_mapped_va(slices, raw_pointer, image_base);
            let resolved_name = resolved_address.and_then(|address| match kind {
                ObjcPointerKind::SelRef => read_c_string_at_va(slices, address)
                    .filter(|name| is_plausible_selector_name(name)),
                ObjcPointerKind::ClassRef | ObjcPointerKind::ClassList => {
                    resolve_class_name_from_pointer(slices, address, image_base, symbol_maps)
                        .filter(|name| is_plausible_objc_type_name(name))
                }
            });

            refs.push(ObjcPointerRef {
                kind,
                table_address: section
                    .section
                    .address
                    .saturating_add((entry_index as u64).saturating_mul(8)),
                raw_pointer,
                resolved_address,
                resolved_name,
            });
        }
    }

    refs.sort_by_key(|entry| entry.table_address);
    refs
}

fn collect_class_records(
    tables: &ObjcPointerTables,
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> Vec<ObjcClassRecord> {
    let mut pointers = BTreeSet::new();
    for entry in tables.classrefs.iter().chain(tables.classlist.iter()) {
        if let Some(pointer) = entry.resolved_address {
            pointers.insert(pointer);
        }
    }

    let mut records = pointers
        .into_iter()
        .map(|class_pointer| build_class_record(slices, class_pointer, image_base, symbol_maps))
        .collect::<Vec<_>>();
    enrich_class_records(&mut records, slices, image_base, symbol_maps);
    records.sort_by_key(|record| record.class_pointer);
    records
}

fn build_class_record(
    slices: &[SectionSlice<'_>],
    class_pointer: u64,
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> ObjcClassRecord {
    let metaclass_pointer = read_u64_at_va(slices, class_pointer)
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let superclass_pointer = read_u64_at_va(slices, class_pointer.saturating_add(8))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let superclass_name = superclass_pointer
        .and_then(|pointer| {
            resolve_class_name_from_pointer(slices, pointer, image_base, symbol_maps)
        })
        .filter(|name| is_plausible_objc_type_name(name));

    let class_data_bits = read_u64_at_va(slices, class_pointer.saturating_add(32));
    let ro_pointer = class_data_bits
        .and_then(|bits| resolve_pointer_to_mapped_va(slices, bits & !0x7, image_base));
    let method_list_pointer = ro_pointer
        .and_then(|pointer| read_u64_at_va(slices, pointer.saturating_add(32)))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let protocol_list_pointer = ro_pointer
        .and_then(|pointer| read_u64_at_va(slices, pointer.saturating_add(40)))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let ivar_list_pointer = ro_pointer
        .and_then(|pointer| read_u64_at_va(slices, pointer.saturating_add(48)))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let property_list_pointer = ro_pointer
        .and_then(|pointer| read_u64_at_va(slices, pointer.saturating_add(64)))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));

    let runtime_name = resolve_runtime_class_name_from_pointer(slices, class_pointer, image_base)
        .filter(|name| is_plausible_objc_type_name(name));
    let pointer_table_name = symbol_maps
        .class_names
        .get(&class_pointer)
        .cloned()
        .or_else(|| symbol_maps.metaclass_names.get(&class_pointer).cloned())
        .filter(|name| is_plausible_objc_type_name(name));

    ObjcClassRecord {
        class_pointer,
        name: runtime_name.clone().or(pointer_table_name.clone()),
        name_source: if runtime_name.is_some() {
            ObjcNameSource::Runtime
        } else if pointer_table_name.is_some() {
            ObjcNameSource::PointerTable
        } else {
            ObjcNameSource::Unresolved
        },
        superclass_pointer,
        superclass_name,
        metaclass_pointer,
        ro_pointer,
        method_list_pointer,
        property_list_pointer,
        protocol_list_pointer,
        ivar_list_pointer,
        methods: Vec::new(),
        class_methods: Vec::new(),
        properties: Vec::new(),
        ivars: Vec::new(),
        adopted_protocols: Vec::new(),
    }
}

fn collect_protocol_records(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> Vec<ObjcProtocolRecord> {
    let mut pointers = BTreeSet::new();
    for table_name in ["__objc_protolist", "__objc_protorefs"] {
        for section in slices
            .iter()
            .filter(|section| section.section.name == table_name)
        {
            for chunk in section.contents.chunks_exact(8) {
                let raw_pointer = u64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]);
                if let Some(pointer) = resolve_pointer_to_mapped_va(slices, raw_pointer, image_base)
                {
                    pointers.insert(pointer);
                }
            }
        }
    }

    let mut records = pointers
        .into_iter()
        .map(|pointer| build_protocol_record(slices, pointer, image_base, symbol_maps))
        .collect::<Vec<_>>();
    records.sort_by_key(|record| record.pointer);
    records
}

fn collect_category_records(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    classes: &[ObjcClassRecord],
    symbol_maps: &ObjcSymbolMaps,
) -> Vec<ObjcCategoryRecord> {
    let class_names = classes
        .iter()
        .filter_map(|record| {
            record
                .name
                .as_ref()
                .map(|name| (record.class_pointer, (name.clone(), record.name_source)))
        })
        .collect::<BTreeMap<_, _>>();
    let mut records = Vec::new();

    for section in slices
        .iter()
        .filter(|section| section.section.name == "__objc_catlist")
    {
        for chunk in section.contents.chunks_exact(8) {
            let raw_pointer = u64::from_le_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
            let Some(pointer) = resolve_pointer_to_mapped_va(slices, raw_pointer, image_base)
            else {
                continue;
            };
            let raw_class_pointer = read_u64_at_va(slices, pointer.saturating_add(8));
            let class_pointer = raw_class_pointer
                .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
            let method_list_pointer = read_u64_at_va(slices, pointer.saturating_add(16))
                .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
            let class_method_list_pointer = read_u64_at_va(slices, pointer.saturating_add(24))
                .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
            let protocol_list_pointer = read_u64_at_va(slices, pointer.saturating_add(32))
                .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
            let property_list_pointer = read_u64_at_va(slices, pointer.saturating_add(40))
                .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));

            let category_symbol_hint = [
                method_list_pointer,
                class_method_list_pointer,
                property_list_pointer,
                protocol_list_pointer,
            ]
            .into_iter()
            .flatten()
            .find_map(|list_pointer| {
                symbol_maps
                    .category_hints_by_list_pointer
                    .get(&list_pointer)
                    .cloned()
            });

            let runtime_category_name =
                resolve_category_name_from_pointer(slices, pointer, image_base)
                    .filter(|name| is_plausible_objc_type_name(name));
            let symbol_category_name = category_symbol_hint
                .as_ref()
                .map(|hint| hint.category_name.clone());
            let category_name = runtime_category_name
                .clone()
                .or(symbol_category_name.clone());
            let category_name_source = if runtime_category_name.is_some() {
                ObjcNameSource::Runtime
            } else if symbol_category_name.is_some() {
                ObjcNameSource::LegacyPool
            } else {
                ObjcNameSource::Unresolved
            };

            let mut class_name_source = ObjcNameSource::Unresolved;
            let mut class_name = None;
            if let Some(value) = class_pointer {
                if let Some((name, source)) = class_names.get(&value).cloned() {
                    class_name = Some(name);
                    class_name_source = source;
                } else if let Some(name) =
                    resolve_runtime_class_name_from_pointer(slices, value, image_base)
                        .filter(|name| is_plausible_objc_type_name(name))
                {
                    class_name = Some(name);
                    class_name_source = ObjcNameSource::Runtime;
                } else if let Some(name) = symbol_maps
                    .class_names
                    .get(&value)
                    .cloned()
                    .or_else(|| symbol_maps.metaclass_names.get(&value).cloned())
                    .filter(|name| is_plausible_objc_type_name(name))
                {
                    class_name = Some(name);
                    class_name_source = ObjcNameSource::PointerTable;
                }
            }
            if class_name.is_none() {
                class_name = raw_class_pointer
                    .and_then(|value| {
                        symbol_maps
                            .class_names
                            .get(&value)
                            .cloned()
                            .or_else(|| symbol_maps.metaclass_names.get(&value).cloned())
                    })
                    .filter(|name| is_plausible_objc_type_name(name));
                if class_name.is_some() {
                    class_name_source = ObjcNameSource::PointerTable;
                }
            }
            if class_name.is_none() {
                class_name = category_symbol_hint
                    .as_ref()
                    .map(|hint| hint.class_name.clone())
                    .filter(|name| is_plausible_objc_type_name(name));
                if class_name.is_some() {
                    class_name_source = ObjcNameSource::LegacyPool;
                }
            }

            records.push(ObjcCategoryRecord {
                properties_source: if property_list_pointer.is_some() {
                    ObjcNameSource::Runtime
                } else {
                    ObjcNameSource::Unresolved
                },
                protocols_source: if protocol_list_pointer.is_some() {
                    ObjcNameSource::Runtime
                } else {
                    ObjcNameSource::Unresolved
                },
                pointer,
                name: category_name,
                name_source: category_name_source,
                record_source: ObjcCategoryRecordSource::RuntimeList,
                class_pointer,
                class_name,
                class_name_source,
                property_list_pointer,
                protocol_list_pointer,
                methods: read_method_list_at_pointer(
                    slices,
                    method_list_pointer,
                    pointer,
                    ObjcMethodOwnerKind::Category,
                    false,
                    image_base,
                ),
                class_methods: read_method_list_at_pointer(
                    slices,
                    class_method_list_pointer,
                    pointer,
                    ObjcMethodOwnerKind::Category,
                    true,
                    image_base,
                ),
                properties: read_property_list_at_pointer(
                    slices,
                    property_list_pointer,
                    pointer,
                    image_base,
                ),
                adopted_protocols: read_protocol_name_list(
                    slices,
                    protocol_list_pointer,
                    image_base,
                    symbol_maps,
                ),
            });
        }
    }

    if records.is_empty() {
        records.extend(collect_synthetic_category_records(
            slices,
            image_base,
            classes,
            symbol_maps,
        ));
    }

    records.sort_by_key(|record| record.pointer);
    records.dedup_by(|left, right| left.pointer == right.pointer);
    records
}

fn collect_synthetic_category_records(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    classes: &[ObjcClassRecord],
    symbol_maps: &ObjcSymbolMaps,
) -> Vec<ObjcCategoryRecord> {
    let class_lookup = classes
        .iter()
        .filter_map(|record| {
            record.name.as_ref().map(|name| {
                (
                    name.clone(),
                    (Some(record.class_pointer), record.name_source),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let mut list_lookup =
        BTreeMap::<(String, String), (Option<u64>, Option<u64>)>::new();
    for list_symbol in &symbol_maps.category_list_symbols {
        let entry = list_lookup
            .entry((
                list_symbol.class_name.clone(),
                list_symbol.category_name.clone(),
            ))
            .or_insert((None, None));
        match list_symbol.kind {
            ObjcCategoryListKind::Properties => entry.0 = Some(list_symbol.pointer),
            ObjcCategoryListKind::Protocols => entry.1 = Some(list_symbol.pointer),
        }
    }
    let mut grouped = BTreeMap::<(String, String), Vec<&ObjcCategoryMethodSymbol>>::new();
    for symbol in &symbol_maps.category_method_symbols {
        grouped
            .entry((symbol.class_name.clone(), symbol.category_name.clone()))
            .or_default()
            .push(symbol);
    }

    grouped
        .into_iter()
        .filter_map(|((class_name, category_name), mut symbols)| {
            symbols.sort_by_key(|symbol| (symbol.implementation, symbol.selector.clone()));
            let pointer = symbols.first()?.implementation;
            let (class_pointer, class_name_source) = class_lookup
                .get(&class_name)
                .cloned()
                .unwrap_or((None, ObjcNameSource::LegacyPool));
            let (property_list_pointer, protocol_list_pointer) = list_lookup
                .get(&(class_name.clone(), category_name.clone()))
                .copied()
                .unwrap_or((None, None));
            let mut methods = Vec::new();
            let mut class_methods = Vec::new();
            for symbol in symbols {
                let method = ObjcMethodRecord {
                    owner_pointer: pointer,
                    owner_kind: ObjcMethodOwnerKind::Category,
                    is_class_method: symbol.is_class_method,
                    selector: Some(symbol.selector.clone()),
                    selector_source: ObjcSelectorSource::LegacyPool,
                    implementation: Some(symbol.implementation),
                    type_encoding: None,
                };
                if symbol.is_class_method {
                    class_methods.push(method);
                } else {
                    methods.push(method);
                }
            }
            Some(ObjcCategoryRecord {
                properties_source: if property_list_pointer.is_some() {
                    ObjcNameSource::LegacyPool
                } else {
                    ObjcNameSource::Unresolved
                },
                protocols_source: if protocol_list_pointer.is_some() {
                    ObjcNameSource::LegacyPool
                } else {
                    ObjcNameSource::Unresolved
                },
                pointer,
                name: Some(category_name),
                name_source: ObjcNameSource::LegacyPool,
                record_source: ObjcCategoryRecordSource::SymbolSynthesis,
                class_pointer,
                class_name: Some(class_name),
                class_name_source,
                property_list_pointer,
                protocol_list_pointer,
                methods,
                class_methods,
                properties: read_property_list_at_pointer(
                    slices,
                    property_list_pointer,
                    pointer,
                    image_base,
                ),
                adopted_protocols: read_protocol_name_list(
                    slices,
                    protocol_list_pointer,
                    image_base,
                    symbol_maps,
                ),
            })
        })
        .collect()
}

fn infer_image_base(sections: &[Section]) -> Option<u64> {
    sections
        .iter()
        .filter_map(|section| {
            section
                .file_offset
                .and_then(|file_offset| section.address.checked_sub(file_offset))
        })
        .min()
}

fn resolve_class_name_from_pointer(
    slices: &[SectionSlice<'_>],
    class_pointer: u64,
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> Option<String> {
    if let Some(name) = symbol_maps
        .class_names
        .get(&class_pointer)
        .cloned()
        .or_else(|| symbol_maps.metaclass_names.get(&class_pointer).cloned())
    {
        return Some(name);
    }

    resolve_runtime_class_name_from_pointer(slices, class_pointer, image_base)
}

fn resolve_runtime_class_name_from_pointer(
    slices: &[SectionSlice<'_>],
    class_pointer: u64,
    image_base: Option<u64>,
) -> Option<String> {
    // Some entries can already point at a C string.
    if let Some(name) = read_c_string_at_va(slices, class_pointer) {
        return Some(name);
    }

    // Objective-C runtime class_t layout (64-bit): isa, superclass, cache, vtable, data.
    // `data` is at +32 and points at class_ro_t with low-bit flags.
    let class_data_bits = read_u64_at_va(slices, class_pointer.checked_add(32)?)?;
    let class_ro = resolve_pointer_to_mapped_va(slices, class_data_bits & !0x7, image_base)?;

    // class_ro_t starts with 16 bytes of scalar fields, then ivarLayout pointer, then name pointer.
    let name_pointer_raw = read_u64_at_va(slices, class_ro.checked_add(24)?)?;
    let name_pointer = resolve_pointer_to_mapped_va(slices, name_pointer_raw, image_base)?;
    read_c_string_at_va(slices, name_pointer)
}

fn resolve_protocol_name_from_pointer(
    slices: &[SectionSlice<'_>],
    protocol_pointer: u64,
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> Option<String> {
    if let Some(name) = symbol_maps.protocol_names.get(&protocol_pointer).cloned() {
        return Some(name);
    }
    resolve_runtime_protocol_name_from_pointer(slices, protocol_pointer, image_base)
}

fn resolve_runtime_protocol_name_from_pointer(
    slices: &[SectionSlice<'_>],
    protocol_pointer: u64,
    image_base: Option<u64>,
) -> Option<String> {
    // protocol_t layout starts with `isa`, then `name`.
    let name_pointer_raw = read_u64_at_va(slices, protocol_pointer.checked_add(8)?)?;
    let name_pointer = resolve_pointer_to_mapped_va(slices, name_pointer_raw, image_base)?;
    read_c_string_at_va(slices, name_pointer)
        .or_else(|| read_c_string_at_va(slices, protocol_pointer))
}

fn resolve_category_name_from_pointer(
    slices: &[SectionSlice<'_>],
    category_pointer: u64,
    image_base: Option<u64>,
) -> Option<String> {
    // category_t layout starts with `name`.
    let name_pointer_raw = read_u64_at_va(slices, category_pointer)?;
    let name_pointer = resolve_pointer_to_mapped_va(slices, name_pointer_raw, image_base)?;
    read_c_string_at_va(slices, name_pointer)
        .or_else(|| read_c_string_at_va(slices, category_pointer))
}

fn resolve_pointer_to_mapped_va(
    slices: &[SectionSlice<'_>],
    raw_pointer: u64,
    image_base: Option<u64>,
) -> Option<u64> {
    pointer_candidates(raw_pointer, image_base)
        .into_iter()
        .find(|candidate| has_file_backed_va(slices, *candidate))
}

fn enrich_class_records(
    records: &mut [ObjcClassRecord],
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) {
    for record in records {
        record.methods = read_method_list_at_pointer(
            slices,
            record.method_list_pointer,
            record.class_pointer,
            ObjcMethodOwnerKind::Class,
            false,
            image_base,
        );
        record.properties = read_property_list_at_pointer(
            slices,
            record.property_list_pointer,
            record.class_pointer,
            image_base,
        );
        record.ivars = read_ivar_list_at_pointer(
            slices,
            record.ivar_list_pointer,
            record.class_pointer,
            image_base,
        );
        record.adopted_protocols = read_protocol_name_list(
            slices,
            record.protocol_list_pointer,
            image_base,
            symbol_maps,
        );
        if let Some(metaclass_pointer) = record.metaclass_pointer {
            let metaclass_methods =
                extract_class_list_pointers(slices, metaclass_pointer, image_base)
                    .and_then(|lists| {
                        Some(read_method_list_at_pointer(
                            slices,
                            lists.method_list_pointer,
                            metaclass_pointer,
                            ObjcMethodOwnerKind::Metaclass,
                            true,
                            image_base,
                        ))
                    })
                    .unwrap_or_default();
            record.class_methods = metaclass_methods;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ClassRuntimeLists {
    method_list_pointer: Option<u64>,
}

fn extract_class_list_pointers(
    slices: &[SectionSlice<'_>],
    class_pointer: u64,
    image_base: Option<u64>,
) -> Option<ClassRuntimeLists> {
    let class_data_bits = read_u64_at_va(slices, class_pointer.saturating_add(32))?;
    let ro_pointer = resolve_pointer_to_mapped_va(slices, class_data_bits & !0x7, image_base)?;
    Some(ClassRuntimeLists {
        method_list_pointer: read_u64_at_va(slices, ro_pointer.saturating_add(32))
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base)),
    })
}

fn build_protocol_record(
    slices: &[SectionSlice<'_>],
    pointer: u64,
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> ObjcProtocolRecord {
    let required_instance = read_u64_at_va(slices, pointer.saturating_add(24))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let required_class = read_u64_at_va(slices, pointer.saturating_add(32))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let optional_instance = read_u64_at_va(slices, pointer.saturating_add(40))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let optional_class = read_u64_at_va(slices, pointer.saturating_add(48))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let properties = read_u64_at_va(slices, pointer.saturating_add(56))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let runtime_name = resolve_runtime_protocol_name_from_pointer(slices, pointer, image_base)
        .filter(|name| is_plausible_objc_type_name(name));
    let pointer_table_name = symbol_maps
        .protocol_names
        .get(&pointer)
        .cloned()
        .filter(|name| is_plausible_objc_type_name(name));

    ObjcProtocolRecord {
        pointer,
        name: runtime_name.clone().or(pointer_table_name.clone()),
        name_source: if runtime_name.is_some() {
            ObjcNameSource::Runtime
        } else if pointer_table_name.is_some() {
            ObjcNameSource::PointerTable
        } else {
            ObjcNameSource::Unresolved
        },
        required_instance_methods: read_method_list_at_pointer(
            slices,
            required_instance,
            pointer,
            ObjcMethodOwnerKind::Protocol,
            false,
            image_base,
        ),
        required_class_methods: read_method_list_at_pointer(
            slices,
            required_class,
            pointer,
            ObjcMethodOwnerKind::Protocol,
            true,
            image_base,
        ),
        optional_instance_methods: read_method_list_at_pointer(
            slices,
            optional_instance,
            pointer,
            ObjcMethodOwnerKind::Protocol,
            false,
            image_base,
        ),
        optional_class_methods: read_method_list_at_pointer(
            slices,
            optional_class,
            pointer,
            ObjcMethodOwnerKind::Protocol,
            true,
            image_base,
        ),
        properties: read_property_list_at_pointer(slices, properties, pointer, image_base),
    }
}

fn read_method_list_at_pointer(
    slices: &[SectionSlice<'_>],
    list_pointer: Option<u64>,
    owner_pointer: u64,
    owner_kind: ObjcMethodOwnerKind,
    is_class_method: bool,
    image_base: Option<u64>,
) -> Vec<ObjcMethodRecord> {
    let Some(list_pointer) = list_pointer else {
        return Vec::new();
    };
    let Some(entsize_flags) = read_u32_at_va(slices, list_pointer) else {
        return Vec::new();
    };
    let Some(count) = read_u32_at_va(slices, list_pointer.saturating_add(4)) else {
        return Vec::new();
    };
    let mut methods = Vec::new();
    let is_small = (entsize_flags & 0x8000_0000) != 0;
    let entry_size =
        usize::try_from((entsize_flags & 0x0000_FFFF).max(if is_small { 12 } else { 24 }))
            .unwrap_or(if is_small { 12 } else { 24 });
    let count = count.min(256);
    for index in 0..count {
        let entry = list_pointer
            .saturating_add(8)
            .saturating_add((index as u64) * (entry_size as u64));
        let mut record = if is_small {
            let name_rel = read_i32_at_va(slices, entry);
            let types_rel = read_i32_at_va(slices, entry.saturating_add(4));
            let imp_rel = read_i32_at_va(slices, entry.saturating_add(8));
            let selector = name_rel
                .and_then(|rel| add_relative_address(entry, rel))
                .and_then(|ptr| resolve_selector_from_location(slices, ptr, image_base));
            ObjcMethodRecord {
                owner_pointer,
                owner_kind,
                is_class_method,
                selector,
                selector_source: if name_rel.is_some() {
                    ObjcSelectorSource::Relative
                } else {
                    ObjcSelectorSource::Unresolved
                },
                implementation: imp_rel
                    .and_then(|rel| add_relative_address(entry.saturating_add(8), rel)),
                type_encoding: types_rel
                    .and_then(|rel| add_relative_address(entry.saturating_add(4), rel))
                    .and_then(|ptr| {
                        resolve_pointer_to_mapped_va(slices, ptr, image_base).or(Some(ptr))
                    })
                    .and_then(|ptr| read_c_string_at_va(slices, ptr)),
            }
        } else {
            let selector = read_u64_at_va(slices, entry)
                .and_then(|raw| resolve_selector_from_raw_pointer(slices, raw, image_base));
            ObjcMethodRecord {
                owner_pointer,
                owner_kind,
                is_class_method,
                selector,
                selector_source: ObjcSelectorSource::Direct,
                implementation: read_u64_at_va(slices, entry.saturating_add(16)),
                type_encoding: read_u64_at_va(slices, entry.saturating_add(8))
                    .and_then(|raw| {
                        resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw))
                    })
                    .and_then(|ptr| read_c_string_at_va(slices, ptr)),
            }
        };
        record.selector = record
            .selector
            .filter(|selector| is_plausible_selector_name(selector));
        if record.selector.is_none() {
            record.selector_source = ObjcSelectorSource::Unresolved;
        }
        if record.selector.is_some()
            || record.implementation.is_some()
            || record.type_encoding.is_some()
        {
            methods.push(record);
        }
    }
    methods
}

fn resolve_selector_from_raw_pointer(
    slices: &[SectionSlice<'_>],
    raw_pointer: u64,
    image_base: Option<u64>,
) -> Option<String> {
    for candidate in pointer_candidates(raw_pointer, image_base) {
        if let Some(selector) = resolve_selector_from_location(slices, candidate, image_base) {
            return Some(selector);
        }
    }
    None
}

fn resolve_selector_from_location(
    slices: &[SectionSlice<'_>],
    selector_location: u64,
    image_base: Option<u64>,
) -> Option<String> {
    if let Some(selector) = read_c_string_at_va(slices, selector_location)
        .filter(|value| is_plausible_selector_name(value))
    {
        return Some(selector);
    }

    // If this points at a selref slot, dereference it once and retry.
    let selector_pointer_raw = read_u64_at_va(slices, selector_location)?;
    let selector_pointer = resolve_pointer_to_mapped_va(slices, selector_pointer_raw, image_base)
        .or(Some(selector_pointer_raw))?;
    read_c_string_at_va(slices, selector_pointer).filter(|value| is_plausible_selector_name(value))
}

fn read_property_list_at_pointer(
    slices: &[SectionSlice<'_>],
    list_pointer: Option<u64>,
    owner_pointer: u64,
    image_base: Option<u64>,
) -> Vec<ObjcPropertyRecord> {
    let Some(list_pointer) = list_pointer else {
        return Vec::new();
    };
    let Some(entsize) = read_u32_at_va(slices, list_pointer) else {
        return Vec::new();
    };
    let Some(count) = read_u32_at_va(slices, list_pointer.saturating_add(4)) else {
        return Vec::new();
    };
    let entry_size = usize::try_from(entsize.max(16)).unwrap_or(16);
    let mut properties = Vec::new();
    for index in 0..count.min(128) {
        let entry = list_pointer
            .saturating_add(8)
            .saturating_add((index as u64) * (entry_size as u64));
        let name = read_u64_at_va(slices, entry)
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
            .and_then(|ptr| read_c_string_at_va(slices, ptr));
        let attributes = read_u64_at_va(slices, entry.saturating_add(8))
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
            .and_then(|ptr| read_c_string_at_va(slices, ptr));
        if name.is_some() || attributes.is_some() {
            let name_source = if name.is_some() {
                ObjcNameSource::Runtime
            } else {
                ObjcNameSource::Unresolved
            };
            properties.push(ObjcPropertyRecord {
                owner_pointer,
                name,
                name_source,
                attributes,
            });
        }
    }
    properties
}

fn read_ivar_list_at_pointer(
    slices: &[SectionSlice<'_>],
    list_pointer: Option<u64>,
    owner_pointer: u64,
    image_base: Option<u64>,
) -> Vec<ObjcIvarRecord> {
    let Some(list_pointer) = list_pointer else {
        return Vec::new();
    };
    let Some(entsize) = read_u32_at_va(slices, list_pointer) else {
        return Vec::new();
    };
    let Some(count) = read_u32_at_va(slices, list_pointer.saturating_add(4)) else {
        return Vec::new();
    };
    let entry_size = usize::try_from(entsize.max(32)).unwrap_or(32);
    let mut ivars = Vec::new();
    for index in 0..count.min(128) {
        let entry = list_pointer
            .saturating_add(8)
            .saturating_add((index as u64) * (entry_size as u64));
        let offset = read_u64_at_va(slices, entry)
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
            .and_then(|ptr| read_u32_at_va(slices, ptr).map(u64::from));
        let name = read_u64_at_va(slices, entry.saturating_add(8))
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
            .and_then(|ptr| read_c_string_at_va(slices, ptr));
        let type_encoding = read_u64_at_va(slices, entry.saturating_add(16))
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
            .and_then(|ptr| read_c_string_at_va(slices, ptr));
        if name.is_some() || type_encoding.is_some() || offset.is_some() {
            let name_source = if name.is_some() {
                ObjcNameSource::Runtime
            } else {
                ObjcNameSource::Unresolved
            };
            ivars.push(ObjcIvarRecord {
                owner_pointer,
                name,
                name_source,
                type_encoding,
                offset,
            });
        }
    }
    ivars
}

fn read_protocol_name_list(
    slices: &[SectionSlice<'_>],
    list_pointer: Option<u64>,
    image_base: Option<u64>,
    symbol_maps: &ObjcSymbolMaps,
) -> Vec<String> {
    let Some(list_pointer) = list_pointer else {
        return Vec::new();
    };
    let Some(count) = read_u64_at_va(slices, list_pointer) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for index in 0..count.min(128) {
        let entry_ptr = list_pointer.saturating_add(8).saturating_add(index * 8);
        let Some(raw_pointer) = read_u64_at_va(slices, entry_ptr) else {
            continue;
        };
        let Some(pointer) = resolve_pointer_to_mapped_va(slices, raw_pointer, image_base) else {
            continue;
        };
        let Some(name) =
            resolve_protocol_name_from_pointer(slices, pointer, image_base, symbol_maps)
        else {
            continue;
        };
        if is_plausible_objc_type_name(&name) {
            names.push(name);
        }
    }
    names.sort();
    names.dedup();
    names
}

fn pointer_candidates(raw_pointer: u64, image_base: Option<u64>) -> Vec<u64> {
    const LOW_48_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
    let masked_48 = raw_pointer & LOW_48_MASK;
    let stripped_low_flags = raw_pointer & !0x7;
    let stripped_48 = stripped_low_flags & LOW_48_MASK;

    let mut seen = BTreeSet::new();
    let mut candidates = Vec::with_capacity(8);
    let mut push = |value: u64| {
        if value != 0 && seen.insert(value) {
            candidates.push(value);
        }
    };

    push(raw_pointer);
    push(stripped_low_flags);
    push(masked_48);
    push(stripped_48);

    if let Some(base) = image_base {
        if let Some(value) = base.checked_add(masked_48) {
            push(value);
        }
        if let Some(value) = base.checked_add(stripped_48) {
            push(value);
        }
    }

    candidates
}

fn has_file_backed_va(slices: &[SectionSlice<'_>], va: u64) -> bool {
    find_slice_for_va(slices, va).is_some()
}

fn read_u64_at_va(slices: &[SectionSlice<'_>], va: u64) -> Option<u64> {
    let (slice, offset) = find_slice_for_va(slices, va)?;
    let bytes = slice.contents.get(offset..offset.checked_add(8)?)?;
    Some(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn read_u32_at_va(slices: &[SectionSlice<'_>], va: u64) -> Option<u32> {
    let (slice, offset) = find_slice_for_va(slices, va)?;
    let bytes = slice.contents.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_i32_at_va(slices: &[SectionSlice<'_>], va: u64) -> Option<i32> {
    read_u32_at_va(slices, va).map(|value| i32::from_le_bytes(value.to_le_bytes()))
}

fn add_relative_address(base: u64, displacement: i32) -> Option<u64> {
    if displacement >= 0 {
        base.checked_add(displacement as u64)
    } else {
        base.checked_sub(displacement.unsigned_abs() as u64)
    }
}

fn read_c_string_at_va(slices: &[SectionSlice<'_>], va: u64) -> Option<String> {
    let (slice, offset) = find_slice_for_va(slices, va)?;
    let tail = slice.contents.get(offset..)?;
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(tail.len());
    if length == 0 {
        return None;
    }
    let value = String::from_utf8_lossy(&tail[..length]).into_owned();
    if value.is_empty() { None } else { Some(value) }
}

fn find_slice_for_va<'a>(
    slices: &'a [SectionSlice<'a>],
    va: u64,
) -> Option<(&'a SectionSlice<'a>, usize)> {
    slices.iter().find_map(|slice| {
        let relative = va.checked_sub(slice.section.address)?;
        if relative >= slice.section.file_size {
            return None;
        }
        let offset = usize::try_from(relative).ok()?;
        if offset >= slice.contents.len() {
            return None;
        }
        Some((slice, offset))
    })
}
