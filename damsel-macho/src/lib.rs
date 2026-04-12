mod disasm;
mod dyld;
mod errors;
mod loader;
mod objc;

pub use disasm::disassemble;
pub use errors::{MachoError, Result};
pub use loader::load;

#[cfg(test)]
mod tests {
    use super::*;
    use damsel_core::{Architecture, DisassemblyRequest, DisassemblyTarget};
    use std::path::{Path, PathBuf};

    fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/bin")
            .join(name)
    }

    #[test]
    fn loads_universal_binary_arm64_slice() {
        let image = load(fixture_path("universal-hello")).expect("load universal binary");
        assert_eq!(image.architecture, Architecture::Arm64);
        assert!(image.slice.is_universal);
    }

    #[test]
    fn stripped_binary_exposes_sections_imports_and_disassembly() {
        let image = load(fixture_path("arm64-stripped")).expect("load stripped fixture");
        assert!(!image.sections.is_empty());
        assert!(!image.imports.is_empty());

        let request = DisassemblyRequest {
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: Some(12),
        };
        let result = disassemble(&image, &request).expect("disassemble stripped fixture");
        assert!(!result.instructions.is_empty());
    }

    #[test]
    fn symbolized_binary_resolves_symbol_annotations() {
        let image = load(fixture_path("arm64-symbolized")).expect("load symbolized fixture");
        let request = DisassemblyRequest {
            target: DisassemblyTarget::Symbol("_main".to_string()),
            max_instructions: Some(12),
        };
        let result = disassemble(&image, &request).expect("disassemble main");
        let annotations = result
            .instructions
            .iter()
            .flat_map(|instruction| instruction.annotations.iter())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert!(
            annotations
                .iter()
                .any(|annotation| annotation.contains("_main"))
        );
    }

    #[test]
    fn objc_fixture_exposes_objc_metadata() {
        let image = load(fixture_path("objc-sample")).expect("load objc fixture");
        assert!(
            image
                .objc
                .class_names
                .iter()
                .any(|class_name| class_name.contains("Greeter"))
        );
        assert!(
            image
                .objc
                .selector_names
                .iter()
                .any(|selector| selector.contains("greeting"))
        );
    }

    #[test]
    fn truncated_fixture_returns_error_without_panicking() {
        let error = load(fixture_path("malformed-truncated")).expect_err("expected parse failure");
        assert!(matches!(
            error,
            MachoError::Object(_) | MachoError::Goblin(_) | MachoError::UnsupportedFileKind(_)
        ));
    }
}
