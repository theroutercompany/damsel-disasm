use damsel_core::{
    ObjcCategoryRecord, ObjcClassRecord, ObjcIvarRecord, ObjcMetadata, ObjcMethodOwnerKind,
    ObjcMethodRecord, ObjcPointerKind, ObjcPointerRef, ObjcPropertyRecord, ObjcProtocolRecord,
    Section,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn collect_objc_metadata(bytes: &[u8], sections: &[Section]) -> ObjcMetadata {
    let mut metadata = ObjcMetadata::default();
    let section_slices = collect_section_slices(bytes, sections);
    let image_base = infer_image_base(sections);
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
    let pointer_tables = collect_pointer_tables(&section_slices, image_base);
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
    metadata.classes = collect_class_records(&pointer_tables, &section_slices, image_base);
    metadata.protocols = collect_protocol_records(&section_slices, image_base);
    metadata.categories = collect_category_records(&section_slices, image_base, &metadata.classes);

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
) -> ObjcPointerTables {
    ObjcPointerTables {
        selrefs: collect_pointer_table(
            slices,
            image_base,
            "__objc_selrefs",
            ObjcPointerKind::SelRef,
        ),
        classrefs: collect_pointer_table(
            slices,
            image_base,
            "__objc_classrefs",
            ObjcPointerKind::ClassRef,
        ),
        classlist: collect_pointer_table(
            slices,
            image_base,
            "__objc_classlist",
            ObjcPointerKind::ClassList,
        ),
    }
}

fn collect_pointer_table(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
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
                    resolve_class_name_from_pointer(slices, address, image_base)
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
) -> Vec<ObjcClassRecord> {
    let mut pointers = BTreeSet::new();
    for entry in tables.classrefs.iter().chain(tables.classlist.iter()) {
        if let Some(pointer) = entry.resolved_address {
            pointers.insert(pointer);
        }
    }

    let mut records = pointers
        .into_iter()
        .map(|class_pointer| build_class_record(slices, class_pointer, image_base))
        .collect::<Vec<_>>();
    enrich_class_records(&mut records, slices, image_base);
    records.sort_by_key(|record| record.class_pointer);
    records
}

fn build_class_record(
    slices: &[SectionSlice<'_>],
    class_pointer: u64,
    image_base: Option<u64>,
) -> ObjcClassRecord {
    let metaclass_pointer = read_u64_at_va(slices, class_pointer)
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let superclass_pointer = read_u64_at_va(slices, class_pointer.saturating_add(8))
        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
    let superclass_name = superclass_pointer
        .and_then(|pointer| resolve_class_name_from_pointer(slices, pointer, image_base))
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

    ObjcClassRecord {
        class_pointer,
        name: resolve_class_name_from_pointer(slices, class_pointer, image_base)
            .filter(|name| is_plausible_objc_type_name(name)),
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
        .map(|pointer| build_protocol_record(slices, pointer, image_base))
        .collect::<Vec<_>>();
    records.sort_by_key(|record| record.pointer);
    records
}

fn collect_category_records(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    classes: &[ObjcClassRecord],
) -> Vec<ObjcCategoryRecord> {
    let class_names = classes
        .iter()
        .filter_map(|record| {
            record
                .name
                .as_ref()
                .map(|name| (record.class_pointer, name.clone()))
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
            let class_pointer = read_u64_at_va(slices, pointer.saturating_add(8))
                .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base));
            let class_name = class_pointer
                .and_then(|value| class_names.get(&value).cloned())
                .or_else(|| {
                    class_pointer.and_then(|value| {
                        resolve_class_name_from_pointer(slices, value, image_base)
                            .filter(|name| is_plausible_objc_type_name(name))
                    })
                });

            records.push(ObjcCategoryRecord {
                pointer,
                name: resolve_category_name_from_pointer(slices, pointer, image_base)
                    .filter(|name| is_plausible_objc_type_name(name)),
                class_pointer,
                class_name,
                methods: read_method_list_at_pointer(
                    slices,
                    read_u64_at_va(slices, pointer.saturating_add(16))
                        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base)),
                    pointer,
                    ObjcMethodOwnerKind::Category,
                    false,
                    image_base,
                ),
                class_methods: read_method_list_at_pointer(
                    slices,
                    read_u64_at_va(slices, pointer.saturating_add(24))
                        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base)),
                    pointer,
                    ObjcMethodOwnerKind::Category,
                    true,
                    image_base,
                ),
                properties: read_property_list_at_pointer(
                    slices,
                    read_u64_at_va(slices, pointer.saturating_add(40))
                        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base)),
                    pointer,
                    image_base,
                ),
                adopted_protocols: read_protocol_name_list(
                    slices,
                    read_u64_at_va(slices, pointer.saturating_add(32))
                        .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base)),
                    image_base,
                ),
            });
        }
    }

    records.sort_by_key(|record| record.pointer);
    records.dedup_by(|left, right| left.pointer == right.pointer);
    records
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
        );
        if let Some(metaclass_pointer) = record.metaclass_pointer {
            let metaclass_methods = extract_class_list_pointers(slices, metaclass_pointer, image_base)
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

    ObjcProtocolRecord {
        pointer,
        name: resolve_protocol_name_from_pointer(slices, pointer, image_base)
            .filter(|name| is_plausible_objc_type_name(name)),
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
    let entry_size = usize::try_from((entsize_flags & 0x0000_FFFF).max(if is_small { 12 } else { 24 }))
        .unwrap_or(if is_small { 12 } else { 24 });
    let count = count.min(256);
    for index in 0..count {
        let entry = list_pointer.saturating_add(8).saturating_add((index as u64) * (entry_size as u64));
        let mut record = if is_small {
            let name_rel = read_i32_at_va(slices, entry);
            let types_rel = read_i32_at_va(slices, entry.saturating_add(4));
            let imp_rel = read_i32_at_va(slices, entry.saturating_add(8));
            ObjcMethodRecord {
                owner_pointer,
                owner_kind,
                is_class_method,
                selector: name_rel
                    .and_then(|rel| add_relative_address(entry, rel))
                    .and_then(|ptr| resolve_pointer_to_mapped_va(slices, ptr, image_base).or(Some(ptr)))
                    .and_then(|ptr| read_c_string_at_va(slices, ptr)),
                implementation: imp_rel.and_then(|rel| add_relative_address(entry.saturating_add(8), rel)),
                type_encoding: types_rel
                    .and_then(|rel| add_relative_address(entry.saturating_add(4), rel))
                    .and_then(|ptr| resolve_pointer_to_mapped_va(slices, ptr, image_base).or(Some(ptr)))
                    .and_then(|ptr| read_c_string_at_va(slices, ptr)),
            }
        } else {
            ObjcMethodRecord {
                owner_pointer,
                owner_kind,
                is_class_method,
                selector: read_u64_at_va(slices, entry)
                    .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
                    .and_then(|ptr| read_c_string_at_va(slices, ptr)),
                implementation: read_u64_at_va(slices, entry.saturating_add(16)),
                type_encoding: read_u64_at_va(slices, entry.saturating_add(8))
                    .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
                    .and_then(|ptr| read_c_string_at_va(slices, ptr)),
            }
        };
        record.selector = record
            .selector
            .filter(|selector| is_plausible_selector_name(selector));
        if record.selector.is_some() || record.implementation.is_some() || record.type_encoding.is_some() {
            methods.push(record);
        }
    }
    methods
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
        let entry = list_pointer.saturating_add(8).saturating_add((index as u64) * (entry_size as u64));
        let name = read_u64_at_va(slices, entry)
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
            .and_then(|ptr| read_c_string_at_va(slices, ptr));
        let attributes = read_u64_at_va(slices, entry.saturating_add(8))
            .and_then(|raw| resolve_pointer_to_mapped_va(slices, raw, image_base).or(Some(raw)))
            .and_then(|ptr| read_c_string_at_va(slices, ptr));
        if name.is_some() || attributes.is_some() {
            properties.push(ObjcPropertyRecord {
                owner_pointer,
                name,
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
        let entry = list_pointer.saturating_add(8).saturating_add((index as u64) * (entry_size as u64));
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
            ivars.push(ObjcIvarRecord {
                owner_pointer,
                name,
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
        let Some(name) = resolve_protocol_name_from_pointer(slices, pointer, image_base) else {
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
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
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
