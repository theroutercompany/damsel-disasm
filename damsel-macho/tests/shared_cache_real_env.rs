use damsel_core::{DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget};
use damsel_macho::{disassemble_v2, inspect_shared_cache};
use std::path::PathBuf;

const REAL_CACHE_ROOT_ENV: &str = "DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT";

#[test]
fn real_shared_cache_env_projected_image_smoke() {
    let Some(root) = std::env::var_os(REAL_CACHE_ROOT_ENV) else {
        return;
    };
    let root = PathBuf::from(root);
    let session = inspect_shared_cache(&root).expect("inspect real shared cache");
    let first_image = session
        .cache()
        .images()
        .first()
        .expect("real cache exposes at least one image");
    let projected = session
        .project_image(first_image.id.as_str())
        .expect("project first real cache image");

    assert!(!projected.image.sections().is_empty());
    assert!(!projected.image.symbols().is_empty() || !projected.image.imports().is_empty());

    let executable_section = projected
        .image
        .sections()
        .iter()
        .find(|section| section.executable)
        .expect("projected image has an executable section");

    let result = disassemble_v2(
        &projected.image,
        &DisassemblyRequestV2 {
            target: DisassemblyTarget::Section(executable_section.name.clone()),
            range: None,
            limit: DisassemblyLimit::Instructions(4),
            options: DisassemblyOptions {
                include_annotations: true,
                include_value_flow: true,
            },
        },
    )
    .expect("disassemble projected real cache image");

    assert!(!result.instructions.is_empty());

    let exports = session
        .exports_for_image(first_image.id.as_str())
        .expect("exports for projected real cache image");
    let export = exports
        .iter()
        .find(|record| record.cache_vmaddr.is_some())
        .expect("at least one export with an address");
    let lookup = session
        .lookup_cache_vmaddr(export.cache_vmaddr.expect("export address"))
        .expect("lookup exported address");
    match lookup {
        damsel_core::CacheLookupResult::ExactSymbol { .. }
        | damsel_core::CacheLookupResult::NearestSymbol { .. }
        | damsel_core::CacheLookupResult::MappingOnly { .. } => {}
    }

    let _dependencies = session
        .image_dependencies(first_image.id.as_str())
        .expect("targeted image dependency query");

    let _ = session
        .reexports(first_image.id.as_str())
        .expect("targeted reexports query");
}
