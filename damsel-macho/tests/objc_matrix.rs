use damsel_core::{ObjcNameSource, ObjcPointerKind, ObjcSelectorSource};
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
    let refs = &image.objc().pointer_refs;
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
        .objc()
        .pointer_refs
        .iter()
        .filter(|entry| matches!(entry.kind, ObjcPointerKind::SelRef))
    {
        if let Some(name) = &entry.resolved_name {
            assert!(
                image
                    .objc()
                    .selector_names
                    .iter()
                    .any(|selector| selector == name),
                "selector compatibility view missing resolved selref name: {name}"
            );
        }
    }

    for entry in image.objc().pointer_refs.iter().filter(|entry| {
        matches!(
            entry.kind,
            ObjcPointerKind::ClassRef | ObjcPointerKind::ClassList
        )
    }) {
        if let Some(name) = &entry.resolved_name {
            assert!(
                image
                    .objc()
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
    assert_sorted_deduped(&image.objc().class_names);
    assert_sorted_deduped(&image.objc().selector_names);
    assert_sorted_deduped(&image.objc().method_names);
}

#[test]
fn structured_objc_runtime_records_are_populated_and_ordered() {
    let image = load(fixture("objc-sample")).expect("load objc fixture");

    if image.objc().pointer_refs.iter().any(|entry| {
        matches!(
            entry.kind,
            ObjcPointerKind::ClassList | ObjcPointerKind::ClassRef
        )
    }) {
        assert!(
            !image.objc().classes.is_empty(),
            "class pointer tables should produce class records"
        );
    }

    let mut previous_class_pointer = 0u64;
    let mut saw_class_methods = false;
    let mut saw_properties = false;
    let mut saw_ivars = false;
    let mut saw_protocol_adoption = false;
    for (index, class_record) in image.objc().classes.iter().enumerate() {
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
                    .objc()
                    .class_names
                    .iter()
                    .any(|class_name| class_name == name),
                "class compatibility view missing structured class name: {name}"
            );
        }
        for property in &class_record.properties {
            if property.name.is_some() {
                assert!(
                    !matches!(property.name_source, ObjcNameSource::Unresolved),
                    "resolved class property name must not be marked unresolved"
                );
            }
        }
        for ivar in &class_record.ivars {
            if ivar.name.is_some() {
                assert!(
                    !matches!(ivar.name_source, ObjcNameSource::Unresolved),
                    "resolved ivar name must not be marked unresolved"
                );
            }
        }
        saw_class_methods |= !class_record.class_methods.is_empty();
        saw_properties |= !class_record.properties.is_empty();
        saw_ivars |= !class_record.ivars.is_empty();
        saw_protocol_adoption |= !class_record.adopted_protocols.is_empty();
    }
    assert!(
        saw_class_methods,
        "expected class methods in structured class records"
    );
    assert!(
        saw_properties,
        "expected property records in structured class records"
    );
    assert!(
        saw_ivars,
        "expected ivar records in structured class records"
    );
    assert!(
        saw_protocol_adoption,
        "expected adopted protocols in structured class records"
    );

    let mut previous_protocol_pointer = 0u64;
    let mut saw_protocol_methods = false;
    let mut saw_protocol_properties = false;
    let mut saw_resolved_selectors = false;
    for (index, protocol_record) in image.objc().protocols.iter().enumerate() {
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
            assert!(
                !matches!(protocol_record.name_source, ObjcNameSource::Unresolved),
                "resolved protocol name must not be marked unresolved"
            );
        }
        for property in &protocol_record.properties {
            if property.name.is_some() {
                assert!(
                    !matches!(property.name_source, ObjcNameSource::Unresolved),
                    "resolved protocol property name must not be marked unresolved"
                );
            }
        }
        saw_protocol_methods |= !protocol_record.required_instance_methods.is_empty()
            || !protocol_record.required_class_methods.is_empty()
            || !protocol_record.optional_instance_methods.is_empty()
            || !protocol_record.optional_class_methods.is_empty();
        saw_protocol_properties |= !protocol_record.properties.is_empty();
        saw_resolved_selectors |= protocol_record
            .required_instance_methods
            .iter()
            .chain(protocol_record.required_class_methods.iter())
            .chain(protocol_record.optional_instance_methods.iter())
            .chain(protocol_record.optional_class_methods.iter())
            .any(|method| {
                method.selector.is_some()
                    && matches!(
                        method.selector_source,
                        ObjcSelectorSource::Direct
                            | ObjcSelectorSource::Relative
                            | ObjcSelectorSource::LegacyPool
                    )
            });
    }
    assert!(
        saw_protocol_methods,
        "expected structured protocol methods to be decoded"
    );
    assert!(
        saw_protocol_properties,
        "expected structured protocol properties to be decoded"
    );
    assert!(
        saw_resolved_selectors,
        "expected protocol method selector recovery with non-unresolved provenance"
    );

    let mut previous_category_pointer = 0u64;
    let mut saw_category_methods = false;
    let mut saw_category_properties = false;
    let mut saw_category_with_name_source = false;
    let mut saw_category_with_class_source = false;
    let mut saw_synthetic_category = false;
    let mut runtime_category_present = false;
    for (index, category_record) in image.objc().categories.iter().enumerate() {
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
            assert!(
                !matches!(category_record.name_source, ObjcNameSource::Unresolved),
                "resolved category name must not be marked unresolved"
            );
            saw_category_with_name_source = true;
        }
        if let Some(class_name) = &category_record.class_name {
            assert!(
                image
                    .objc()
                    .class_names
                    .iter()
                    .any(|existing| existing == class_name),
                "compatibility view missing category owner class name: {class_name}"
            );
            assert!(
                !matches!(
                    category_record.class_name_source,
                    ObjcNameSource::Unresolved
                ),
                "resolved category class name must not be marked unresolved"
            );
            saw_category_with_class_source = true;
        }
        saw_synthetic_category |= matches!(
            category_record.record_source,
            damsel_core::ObjcCategoryRecordSource::SymbolSynthesis
        );
        runtime_category_present |= matches!(
            category_record.record_source,
            damsel_core::ObjcCategoryRecordSource::RuntimeList
        );
        saw_category_methods |=
            !category_record.methods.is_empty() || !category_record.class_methods.is_empty();
        saw_category_properties |= !category_record.properties.is_empty();
        for property in &category_record.properties {
            if property.name.is_some() {
                assert!(
                    !matches!(property.name_source, ObjcNameSource::Unresolved),
                    "resolved category property name must not be marked unresolved"
                );
            }
        }
        for method in category_record
            .methods
            .iter()
            .chain(category_record.class_methods.iter())
        {
            if method.selector.is_some() {
                assert!(
                    !matches!(method.selector_source, ObjcSelectorSource::Unresolved),
                    "resolved selector must not be marked unresolved"
                );
            }
        }
    }
    if !image.objc().categories.is_empty() {
        assert!(
            saw_category_methods,
            "expected structured category methods to be decoded"
        );
        if runtime_category_present {
            assert!(
                saw_category_properties,
                "expected runtime-backed category properties to be decoded"
            );
        }
        assert!(
            saw_category_with_name_source,
            "expected at least one category with resolved provenance-labeled name"
        );
        assert!(
            saw_category_with_class_source,
            "expected at least one category with resolved provenance-labeled class name"
        );
        assert!(
            saw_synthetic_category,
            "expected synthetic category recovery when the fixture lacks __objc_catlist"
        );
    }

    let mut saw_class_with_name_source = false;
    let mut saw_selector_with_source = false;
    for class_record in &image.objc().classes {
        if class_record.name.is_some() {
            assert!(
                !matches!(class_record.name_source, ObjcNameSource::Unresolved),
                "resolved class name must not be marked unresolved"
            );
            saw_class_with_name_source = true;
        }
        for method in class_record
            .methods
            .iter()
            .chain(class_record.class_methods.iter())
        {
            if method.selector.is_some() {
                assert!(
                    !matches!(method.selector_source, ObjcSelectorSource::Unresolved),
                    "resolved class selector must not be marked unresolved"
                );
                assert!(
                    image
                        .objc()
                        .method_names
                        .iter()
                        .any(|selector| selector == method.selector.as_ref().expect("selector")),
                    "method compatibility view missing structured selector"
                );
                saw_selector_with_source = true;
            }
        }
    }
    assert!(
        saw_class_with_name_source,
        "expected class records with resolved provenance-labeled names"
    );
    assert!(
        saw_selector_with_source,
        "expected resolved selectors in structured class method records"
    );
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
