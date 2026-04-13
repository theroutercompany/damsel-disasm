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
}
