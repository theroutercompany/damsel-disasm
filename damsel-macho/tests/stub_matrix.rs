use damsel_macho::load;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn chained_fixups_materialize_resolved_import_addresses_symbolized_fixture() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    assert!(image.dyld().has_chained_fixups);
    assert!(image.dyld().has_binds);
    assert!(
        image
            .imports()
            .iter()
            .any(|import| import.address.is_some()),
        "expected at least one import with a resolved bind-site address"
    );
}

#[test]
fn chained_fixups_materialize_resolved_import_addresses_objc_fixture() {
    let image = load(fixture("objc-sample")).expect("load fixture");
    assert!(image.dyld().has_chained_fixups);
    assert!(image.dyld().has_binds);
    assert!(
        image
            .imports()
            .iter()
            .any(|import| import.address.is_some()),
        "expected at least one import with a resolved bind-site address"
    );
}

#[test]
fn lazy_stub_fixture_materializes_stub_helpers_when_present() {
    let path = fixture("import-lazy");
    if !path.exists() {
        eprintln!("import-lazy fixture not present; skipping");
        return;
    }
    let image = load(&path).expect("load lazy helper fixture");
    assert!(!image.dyld().has_chained_fixups);
    assert!(
        !image.dyld().stub_helpers.is_empty(),
        "expected stub helper entries for lazy fixture"
    );
    assert!(
        image
            .dyld()
            .stubs
            .iter()
            .any(|stub| stub.helper_address.is_some()),
        "expected at least one stub to link to a helper entry"
    );
    assert!(
        image.dyld().stub_helpers.iter().all(|helper| {
            helper.target_stub.is_some()
                && helper.pointer_address.is_some()
                && helper.stub_section.as_deref().is_some()
                && helper.pointer_section.as_deref().is_some()
        }),
        "expected helper entries to carry stub/pointer linkage metadata"
    );
    for helper in &image.dyld().stub_helpers {
        let by_stub = image
            .dyld()
            .helper_for_stub_address(helper.target_stub.expect("helper target_stub missing"))
            .expect("helper lookup by stub should succeed");
        assert_eq!(by_stub.helper_address, helper.helper_address);
        let by_pointer = image
            .dyld()
            .helper_for_pointer_address(helper.pointer_address.expect("helper pointer missing"))
            .expect("helper lookup by pointer should succeed");
        assert_eq!(by_pointer.helper_address, helper.helper_address);
    }
}
