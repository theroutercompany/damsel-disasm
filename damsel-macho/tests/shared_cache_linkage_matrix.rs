use damsel_macho::inspect_shared_cache;
use std::path::{Path, PathBuf};

fn cache_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/shared-cache-corpus")
        .join(name)
}

#[test]
fn internal_linkage_fixture_reports_dependencies_dependents_and_importers() {
    let session =
        inspect_shared_cache(cache_fixture("internal-linkage-arm64.cache")).expect("load cache");

    let dependencies = session
        .image_dependencies("/usr/lib/libdispatch.dylib")
        .expect("dispatch dependencies");
    let libsystem_dependency = dependencies
        .iter()
        .find(|record| record.target_dylib_install_name == "/usr/lib/libSystem.B.dylib")
        .expect("libSystem dependency");
    assert!(libsystem_dependency.within_cache);
    assert_eq!(
        libsystem_dependency
            .target_image
            .as_ref()
            .expect("resolved target image")
            .install_name,
        "/usr/lib/libSystem.B.dylib"
    );

    let dependents = session
        .dependents("/usr/lib/libSystem.B.dylib")
        .expect("libSystem dependents");
    assert!(dependents.iter().any(|record| {
        record.dependent_image.install_name == "/usr/lib/libdispatch.dylib"
            && record.dependency_count > 0
    }));

    let importers = session.symbol_importers("_puts").expect("symbol importers");
    assert!(importers.iter().any(|record| {
        record.importer_image.install_name == "/usr/lib/libdispatch.dylib"
            && record.dylib_name == "/usr/lib/libSystem.B.dylib"
            && record
                .resolved_provider_image
                .as_ref()
                .is_some_and(|image| image.install_name == "/usr/lib/libSystem.B.dylib")
    }));
}

#[test]
fn reexport_fixture_reports_export_and_reexport_providers() {
    let session =
        inspect_shared_cache(cache_fixture("reexport-linkage-arm64.cache")).expect("load cache");

    let providers = session
        .symbol_providers("_exported_regular")
        .expect("provider lookup");
    assert!(providers.iter().any(|record| {
        record.provider_image.install_name == "/usr/lib/libprovider.dylib"
            && matches!(
                record.provider_kind,
                damsel_core::CacheSymbolProviderKind::Export
            )
    }));
    assert!(providers.iter().any(|record| {
        record.provider_image.install_name == "/usr/lib/libreexporter.dylib"
            && matches!(
                record.provider_kind,
                damsel_core::CacheSymbolProviderKind::Reexport
            )
            && record.target_dylib.as_deref() == Some("/usr/lib/libprovider.dylib")
            && record
                .resolved_target_image
                .as_ref()
                .is_some_and(|image| image.install_name == "/usr/lib/libprovider.dylib")
    }));
}

#[test]
fn reexport_fixture_reports_image_reexports() {
    let session =
        inspect_shared_cache(cache_fixture("reexport-linkage-arm64.cache")).expect("load cache");

    let reexports = session
        .reexports("/usr/lib/libreexporter.dylib")
        .expect("reexports");
    assert!(reexports.iter().any(|record| {
        record.export_name == "_exported_regular"
            && record.target_dylib == "/usr/lib/libprovider.dylib"
            && record.target_symbol.as_deref() == Some("_exported_regular")
            && record
                .resolved_target_image
                .as_ref()
                .is_some_and(|image| image.install_name == "/usr/lib/libprovider.dylib")
    }));
}
