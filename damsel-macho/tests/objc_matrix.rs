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
    for (index, entry) in refs.iter().enumerate() {
        if index > 0 {
            assert!(
                previous <= entry.table_address,
                "pointer refs must be sorted by table address"
            );
        }
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

fn assert_sorted_deduped(values: &[String]) {
    for pair in values.windows(2) {
        assert!(pair[0] <= pair[1], "values must be sorted");
        assert!(pair[0] != pair[1], "values must be deduplicated");
    }
}
