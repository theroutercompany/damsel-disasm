use damsel_core::ExportKind;
use damsel_macho::load;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn export_kinds_fixture_exposes_typed_export_kinds_when_present() {
    let path = fixture("export-kinds");
    if !path.exists() {
        eprintln!("export-kinds fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load export-kinds fixture");
    let exports = &image.dyld().exported_symbols;
    assert!(!exports.is_empty(), "expected exported symbols");
    assert!(exports.iter().any(|export| matches!(export.kind, ExportKind::Regular)));
    assert!(exports
        .iter()
        .any(|export| matches!(export.kind, ExportKind::WeakDefinition)));
    assert!(exports
        .iter()
        .any(|export| matches!(export.kind, ExportKind::ThreadLocal)));
    assert!(exports
        .iter()
        .any(|export| matches!(export.kind, ExportKind::Absolute)));

    let absolute = exports
        .iter()
        .find(|export| export.name == "_exported_absolute")
        .expect("absolute export present");
    assert_eq!(absolute.address, Some(0x1234));
    assert!(absolute.flags.is_absolute);

    let weak = exports
        .iter()
        .find(|export| export.name == "_exported_weak")
        .expect("weak export present");
    assert!(weak.flags.is_weak_definition);

    let thread_local = exports
        .iter()
        .find(|export| export.name == "_exported_tls")
        .expect("thread-local export present");
    assert!(thread_local.flags.is_thread_local);

    for export in exports {
        if let Some(address) = export.address {
            assert_eq!(image.dyld().export_by_address(address), Some(export));
        }
        assert!(image.dyld().export_by_name(&export.name).is_some());
        assert_eq!(
            image.dyld().exports_named(&export.name).next().map(|value| value.name.as_str()),
            Some(export.name.as_str())
        );
    }
}
