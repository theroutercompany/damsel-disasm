use damsel_core::{ObjcMetadata, ObjcPointerKind, ObjcPointerRef, Section};
use std::collections::BTreeSet;

pub(crate) fn collect_objc_metadata(bytes: &[u8], sections: &[Section]) -> ObjcMetadata {
    let mut metadata = ObjcMetadata::default();
    let section_slices = collect_section_slices(bytes, sections);
    let image_base = infer_image_base(sections);

    for section in &section_slices {
        match section.section.name.as_str() {
            "__objc_classname" => metadata
                .class_names
                .extend(read_c_strings(section.contents)),
            "__objc_methname" => {
                let names = read_c_strings(section.contents);
                metadata.method_names.extend(
                    names
                        .iter()
                        .filter(|name| is_plausible_selector_name(name))
                        .cloned(),
                );
                metadata.selector_names.extend(
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
    // 2) derive compatibility lists from resolved pointer names
    // 3) add bounded protocol/category names from their top-level pointer lists
    let pointer_tables = collect_pointer_tables(&section_slices, image_base);
    let mut pointer_refs = Vec::new();
    pointer_refs.extend(pointer_tables.selrefs.iter().cloned());
    pointer_refs.extend(pointer_tables.classrefs.iter().cloned());
    pointer_refs.extend(pointer_tables.classlist.iter().cloned());
    pointer_refs.sort_by_key(|entry| entry.table_address);
    metadata.pointer_refs = pointer_refs;

    metadata.selector_names.extend(
        pointer_tables
            .selrefs
            .iter()
            .filter_map(|entry| entry.resolved_name.clone())
            .filter(|name| is_plausible_selector_name(name)),
    );
    metadata.class_names.extend(
        pointer_tables
            .classrefs
            .iter()
            .chain(pointer_tables.classlist.iter())
            .filter_map(|entry| entry.resolved_name.clone())
            .filter(|name| is_plausible_objc_type_name(name)),
    );
    metadata
        .class_names
        .extend(collect_protocol_names(&section_slices, image_base));
    metadata
        .class_names
        .extend(collect_category_names(&section_slices, image_base));

    metadata.class_names.sort();
    metadata.class_names.dedup();
    metadata.method_names.sort();
    metadata.method_names.dedup();
    metadata.selector_names.sort();
    metadata.selector_names.dedup();
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

fn collect_protocol_names(slices: &[SectionSlice<'_>], image_base: Option<u64>) -> Vec<String> {
    let mut names = collect_named_entries_from_pointer_table(
        slices,
        image_base,
        "__objc_protolist",
        resolve_protocol_name_from_pointer,
    );
    names.extend(collect_named_entries_from_pointer_table(
        slices,
        image_base,
        "__objc_protorefs",
        resolve_protocol_name_from_pointer,
    ));
    names.sort();
    names.dedup();
    names
}

fn collect_category_names(slices: &[SectionSlice<'_>], image_base: Option<u64>) -> Vec<String> {
    collect_named_entries_from_pointer_table(
        slices,
        image_base,
        "__objc_catlist",
        resolve_category_name_from_pointer,
    )
}

fn collect_named_entries_from_pointer_table(
    slices: &[SectionSlice<'_>],
    image_base: Option<u64>,
    table_section_name: &str,
    resolver: fn(&[SectionSlice<'_>], u64, Option<u64>) -> Option<String>,
) -> Vec<String> {
    let mut names = Vec::new();
    for section in slices
        .iter()
        .filter(|section| section.section.name == table_section_name)
    {
        for chunk in section.contents.chunks_exact(8) {
            let raw_pointer = u64::from_le_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
            if raw_pointer == 0 {
                continue;
            }
            let Some(resolved_address) =
                resolve_pointer_to_mapped_va(slices, raw_pointer, image_base)
            else {
                continue;
            };
            let Some(name) = resolver(slices, resolved_address, image_base) else {
                continue;
            };
            if is_plausible_objc_type_name(&name) {
                names.push(name);
            }
        }
    }
    names.sort();
    names.dedup();
    names
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
