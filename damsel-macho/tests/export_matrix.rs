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
