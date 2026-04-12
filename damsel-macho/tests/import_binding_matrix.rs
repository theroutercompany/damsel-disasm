use damsel_core::{ImportBindingKind, ImportBindingSource, StubKind};
use damsel_macho::load;
use std::collections::{BTreeMap, BTreeSet, HashSet};
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

fn assert_import_bindings_and_stubs(path: &Path, expect_chained_fixups: bool) {
    let image =
        load(path).unwrap_or_else(|error| panic!("failed to load fixture {path:?}: {error}"));
    let dyld = image.dyld();

    assert_eq!(
        dyld.has_chained_fixups, expect_chained_fixups,
        "unexpected chained-fixups state for fixture {path:?}"
    );
    assert!(dyld.has_binds, "expected bind fixups for fixture {path:?}");
    assert!(
        !dyld.import_bindings.is_empty(),
        "expected typed import bindings for fixture {path:?}"
    );

    assert!(
        dyld.import_bindings.iter().any(|binding| {
            matches!(
                binding.source,
                ImportBindingSource::ChainedFixup | ImportBindingSource::IndirectSymbol
            )
        }),
        "expected at least one concrete binding source"
    );
    assert!(
        dyld.import_bindings.iter().any(|binding| matches!(
            binding.binding_kind,
            ImportBindingKind::ChainedFixup | ImportBindingKind::Lazy | ImportBindingKind::NonLazy
        )),
        "expected typed binding kinds for fixture {path:?}"
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
            "{}|{}|{}|{}|{}|{:?}|{:?}",
            binding.address.unwrap_or_default(),
            binding.offset.unwrap_or_default(),
            binding.dylib,
            binding.name,
            binding.addend,
            binding.binding_kind,
            binding.source
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
        dyld.stubs.iter().any(|stub| {
            stub.section.is_some()
                && stub.pointer_address.is_some()
                && matches!(stub.stub_kind, StubKind::Lazy | StubKind::NonLazy)
                && (stub.name.is_some() || stub.dylib.is_some())
        }),
        "expected at least one resolved stub pointer/name mapping"
    );
}

#[test]
fn import_bindings_and_stubs_symbolized_fixture() {
    assert_import_bindings_and_stubs(&fixture("arm64-symbolized"), true);
}

#[test]
fn import_bindings_and_stubs_objc_fixture() {
    assert_import_bindings_and_stubs(&fixture("objc-sample"), true);
}

#[test]
fn import_bindings_and_stubs_import_rich_fixture_if_present() {
    let Some(path) = optional_fixture(&["import-rich", "import_rich", "arm64-import-rich"]) else {
        eprintln!("import-rich fixture not present; skipping");
        return;
    };
    assert_import_bindings_and_stubs(&path, true);
}

#[test]
fn import_bindings_and_stubs_lazy_fixture_if_present() {
    let Some(path) = optional_fixture(&["import-lazy"]) else {
        eprintln!("import-lazy fixture not present; skipping");
        return;
    };
    assert_import_bindings_and_stubs(&path, false);
}

#[test]
fn lazy_fixture_stub_helpers_link_back_to_stub_and_pointer_metadata() {
    let Some(path) = optional_fixture(&["import-lazy"]) else {
        eprintln!("import-lazy fixture not present; skipping");
        return;
    };
    let image = load(&path).expect("load lazy fixture");
    let dyld = image.dyld();
    assert!(
        !dyld.stub_helpers.is_empty(),
        "expected decoded helper entries in lazy fixture"
    );
    assert!(
        dyld.import_bindings
            .iter()
            .any(|binding| binding.binding_kind == ImportBindingKind::Lazy),
        "expected at least one lazy binding"
    );
    for helper in &dyld.stub_helpers {
        assert!(
            helper.target_stub.is_some(),
            "helper entry should link to a stub address"
        );
        assert!(
            helper.pointer_address.is_some(),
            "helper entry should link to a lazy pointer slot"
        );
        assert!(
            helper.stub_section.as_deref().is_some(),
            "helper entry should carry its stub section"
        );
        assert!(
            helper.pointer_section.as_deref().is_some(),
            "helper entry should carry its pointer section"
        );
        let by_address = dyld
            .helper_for_address(helper.helper_address)
            .expect("helper lookup by address should succeed");
        assert_eq!(by_address.target_stub, helper.target_stub);
        if let Some(stub_address) = helper.target_stub {
            let linked = dyld
                .stubs
                .iter()
                .find(|stub| stub.stub_address == stub_address)
                .expect("linked stub should exist");
            assert_eq!(linked.helper_address, Some(helper.helper_address));
            assert_eq!(linked.pointer_address, helper.pointer_address);
        }
    }
}

#[test]
fn duplicate_symbol_fixture_prefers_ordinal_backed_attribution_when_present() {
    let Some(path) = optional_fixture(&["duplicate-symbol-ordinal"]) else {
        eprintln!("duplicate-symbol-ordinal fixture not present; skipping");
        return;
    };
    let image = load(&path).expect("load duplicate symbol fixture");
    let mut grouped = BTreeMap::<String, (BTreeSet<String>, BTreeSet<u32>)>::new();
    for binding in image.dyld().import_bindings.iter().filter(|binding| {
        binding.source == ImportBindingSource::IndirectSymbol && binding.ordinal.is_some()
    }) {
        let entry = grouped
            .entry(binding.name.clone())
            .or_insert_with(|| (BTreeSet::new(), BTreeSet::new()));
        entry.0.insert(binding.dylib.clone());
        entry.1.insert(binding.ordinal.expect("checked is_some"));
    }

    let has_ordinal_disambiguation = grouped
        .values()
        .any(|(dylibs, ordinals)| dylibs.len() > 1 && ordinals.len() > 1);
    assert!(
        has_ordinal_disambiguation,
        "expected at least one symbol name to resolve across multiple dylibs with distinct ordinals"
    );
}
