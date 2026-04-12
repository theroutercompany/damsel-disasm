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
        image.imports().iter().any(|import| import.address.is_some()),
        "expected at least one import with a resolved bind-site address"
    );
}

#[test]
fn chained_fixups_materialize_resolved_import_addresses_objc_fixture() {
    let image = load(fixture("objc-sample")).expect("load fixture");
    assert!(image.dyld().has_chained_fixups);
    assert!(image.dyld().has_binds);
    assert!(
        image.imports().iter().any(|import| import.address.is_some()),
        "expected at least one import with a resolved bind-site address"
    );
}
