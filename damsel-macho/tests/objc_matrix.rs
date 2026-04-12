use damsel_core::ObjcPointerKind;
use damsel_macho::load;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn objc_pointer_refs_are_populated_and_sorted() {
    let image = load(fixture("objc-sample")).expect("load objc fixture");
    let refs = &image.objc.pointer_refs;
    assert!(!refs.is_empty(), "expected pointer refs to be populated");
    assert!(
        refs.iter()
            .any(|entry| matches!(entry.kind, ObjcPointerKind::SelRef)),
        "expected selrefs in pointer refs"
    );
    assert!(
        refs.iter()
            .any(|entry| matches!(entry.kind, ObjcPointerKind::ClassRef)),
        "expected classrefs in pointer refs"
    );
    assert!(
        refs.iter()
            .any(|entry| matches!(entry.kind, ObjcPointerKind::ClassList)),
        "expected classlist in pointer refs"
    );

    let mut previous = 0u64;
    let mut keys = std::collections::BTreeSet::new();
    for (index, entry) in refs.iter().enumerate() {
        if index > 0 {
            assert!(
                previous <= entry.table_address,
                "pointer refs must be sorted by table address"
            );
        }
        assert!(
            keys.insert((
                entry.table_address,
                pointer_kind_sort_key(entry.kind),
                entry.raw_pointer
            )),
            "pointer refs should be canonicalized without exact duplicates"
        );
        previous = entry.table_address;
    }
}

#[test]
fn compatibility_views_include_resolved_pointer_names() {
    let image = load(fixture("objc-sample")).expect("load objc fixture");

    for entry in image
        .objc
        .pointer_refs
        .iter()
        .filter(|entry| matches!(entry.kind, ObjcPointerKind::SelRef))
    {
        if let Some(name) = &entry.resolved_name {
            assert!(
                image
                    .objc
                    .selector_names
                    .iter()
                    .any(|selector| selector == name),
                "selector compatibility view missing resolved selref name: {name}"
            );
        }
    }

    for entry in image.objc.pointer_refs.iter().filter(|entry| {
        matches!(
            entry.kind,
            ObjcPointerKind::ClassRef | ObjcPointerKind::ClassList
        )
    }) {
        if let Some(name) = &entry.resolved_name {
            assert!(
                image
                    .objc
                    .class_names
                    .iter()
                    .any(|class_name| class_name == name),
                "class compatibility view missing resolved class-like name: {name}"
            );
        }
    }
}

#[test]
fn compatibility_views_remain_sorted_and_deduplicated() {
    let image = load(fixture("objc-sample")).expect("load objc fixture");
    assert_sorted_deduped(&image.objc.class_names);
    assert_sorted_deduped(&image.objc.selector_names);
    assert_sorted_deduped(&image.objc.method_names);
}

#[test]
fn structured_objc_runtime_records_are_populated_and_ordered() {
    let image = load(fixture("objc-sample")).expect("load objc fixture");

    if image.objc.pointer_refs.iter().any(|entry| {
        matches!(
            entry.kind,
            ObjcPointerKind::ClassList | ObjcPointerKind::ClassRef
        )
    }) {
        assert!(
            !image.objc.classes.is_empty(),
            "class pointer tables should produce class records"
        );
    }

    let mut previous_class_pointer = 0u64;
    for (index, class_record) in image.objc.classes.iter().enumerate() {
        assert!(
            class_record.class_pointer != 0,
            "class pointer must be non-zero"
        );
        if index > 0 {
            assert!(
                previous_class_pointer <= class_record.class_pointer,
                "class records must be sorted by class pointer"
            );
        }
        previous_class_pointer = class_record.class_pointer;

        if let Some(name) = &class_record.name {
            assert!(
                image
                    .objc
                    .class_names
                    .iter()
                    .any(|class_name| class_name == name),
                "class compatibility view missing structured class name: {name}"
            );
        }
    }

    let mut previous_protocol_pointer = 0u64;
    for (index, protocol_record) in image.objc.protocols.iter().enumerate() {
        assert!(
            protocol_record.pointer != 0,
            "protocol pointer must be non-zero when present"
        );
        if index > 0 {
            assert!(
                previous_protocol_pointer <= protocol_record.pointer,
                "protocol records must be sorted by pointer"
            );
        }
        previous_protocol_pointer = protocol_record.pointer;
        if let Some(name) = &protocol_record.name {
            assert!(
                !name.is_empty(),
                "structured protocol names should be non-empty"
            );
        }
    }

    let mut previous_category_pointer = 0u64;
    for (index, category_record) in image.objc.categories.iter().enumerate() {
        assert!(
            category_record.pointer != 0,
            "category pointer must be non-zero when present"
        );
        if index > 0 {
            assert!(
                previous_category_pointer <= category_record.pointer,
                "category records must be sorted by pointer"
            );
        }
        previous_category_pointer = category_record.pointer;
        if let Some(name) = &category_record.name {
            assert!(
                !name.is_empty(),
                "structured category names should be non-empty"
            );
        }
        if let Some(class_name) = &category_record.class_name {
            assert!(
                image
                    .objc
                    .class_names
                    .iter()
                    .any(|existing| existing == class_name),
                "compatibility view missing category owner class name: {class_name}"
            );
        }
    }
}

fn assert_sorted_deduped(values: &[String]) {
    for pair in values.windows(2) {
        assert!(pair[0] <= pair[1], "values must be sorted");
        assert!(pair[0] != pair[1], "values must be deduplicated");
    }
}

fn pointer_kind_sort_key(kind: ObjcPointerKind) -> u8 {
    match kind {
        ObjcPointerKind::SelRef => 0,
        ObjcPointerKind::ClassRef => 1,
        ObjcPointerKind::ClassList => 2,
    }
}
