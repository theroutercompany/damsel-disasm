use damsel_core::ImportBindingSource;
use damsel_macho::load;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

fn optional_fixture(names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| fixture(name))
        .find(|path| path.exists())
}

fn assert_import_bindings_and_stubs(path: &Path) {
    let image =
        load(path).unwrap_or_else(|error| panic!("failed to load fixture {path:?}: {error}"));
    let dyld = &image.dyld;

    assert!(
        dyld.has_chained_fixups,
        "expected chained fixups for fixture {path:?}"
    );
    assert!(dyld.has_binds, "expected bind fixups for fixture {path:?}");
    assert!(
        !dyld.import_bindings.is_empty(),
        "expected typed import bindings for fixture {path:?}"
    );

    assert!(
        dyld.import_bindings
            .iter()
            .any(|binding| binding.source == ImportBindingSource::ChainedFixup),
        "expected at least one chained-fixup binding source"
    );
    assert!(
        dyld.import_bindings
            .iter()
            .any(|binding| binding.address.is_some()),
        "expected at least one resolved binding address"
    );
    assert!(
        dyld.import_bindings
            .iter()
            .any(|binding| !binding.name.is_empty() && !binding.dylib.is_empty()),
        "expected binding records to include dylib/name pairs"
    );

    let mut binding_keys = HashSet::new();
    for binding in &dyld.import_bindings {
        let key = format!(
            "{}|{}|{}|{}|{}",
            binding.address.unwrap_or_default(),
            binding.offset.unwrap_or_default(),
            binding.dylib,
            binding.name,
            binding.addend
        );
        assert!(
            binding_keys.insert(key),
            "duplicate import binding record found"
        );
    }

    assert!(
        !dyld.stubs.is_empty(),
        "expected stub entries to be materialized for fixture {path:?}"
    );
    assert!(
        dyld.stubs
            .iter()
            .any(|stub| stub.pointer_address.is_some()
                && (stub.name.is_some() || stub.dylib.is_some())),
        "expected at least one resolved stub pointer/name mapping"
    );
}

#[test]
fn import_bindings_and_stubs_symbolized_fixture() {
    assert_import_bindings_and_stubs(&fixture("arm64-symbolized"));
}

#[test]
fn import_bindings_and_stubs_objc_fixture() {
    assert_import_bindings_and_stubs(&fixture("objc-sample"));
}

#[test]
fn import_bindings_and_stubs_import_rich_fixture_if_present() {
    let Some(path) = optional_fixture(&["import-rich", "import_rich", "arm64-import-rich"]) else {
        eprintln!("import-rich fixture not present; skipping");
        return;
    };
    assert_import_bindings_and_stubs(&path);
}
