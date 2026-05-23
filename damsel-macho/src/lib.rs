mod disasm;
mod dyld;
mod errors;
mod loader;
mod objc;
mod shared_cache;
mod shared_cache_query;

pub use damsel_core::{
    CacheImageId, CacheImageRecord, CacheLookupResult, SharedCache, SharedCacheHeader,
    SharedCacheMapping, SharedCacheMember, SharedCacheMemberRole, SharedCacheSource,
    SymbolicationMatch,
};
pub use disasm::{analyze_disassembly, disassemble, disassemble_v2};
pub use errors::{MachoError, Result};
pub use loader::{load, load_bytes};
pub use shared_cache::{
    SharedCacheExportRecord, SharedCacheSession, inspect_shared_cache, load_shared_cache,
};

#[cfg(test)]
mod tests {
    use super::*;
    use damsel_core::{
        Architecture, DisassemblyLimit, DisassemblyOptions, DisassemblyRequest,
        DisassemblyRequestV2, DisassemblyTarget,
    };
    use std::path::{Path, PathBuf};

    fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/bin")
            .join(name)
    }

    #[test]
    fn loads_universal_binary_arm64_slice() {
        let image = load(fixture_path("universal-hello")).expect("load universal binary");
        assert_eq!(image.architecture(), Architecture::Arm64);
        assert!(image.selected_slice().is_universal);
    }

    #[test]
    fn stripped_binary_exposes_sections_imports_and_disassembly() {
        let image = load(fixture_path("arm64-stripped")).expect("load stripped fixture");
        assert!(!image.sections().is_empty());
        assert!(!image.imports().is_empty());

        let request = DisassemblyRequest {
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: Some(12),
            limit: None,
            include_annotations: true,
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
            limit: None,
            include_annotations: true,
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
                .objc()
                .class_names
                .iter()
                .any(|class_name| class_name.contains("Greeter"))
        );
        assert!(
            image
                .objc()
                .selector_names
                .iter()
                .any(|selector| selector.contains("greeting"))
        );
    }

    #[test]
    fn truncated_fixture_returns_error_without_panicking() {
        let error = load(fixture_path("malformed-truncated")).expect_err("expected parse failure");
        assert!(matches!(error, MachoError::MalformedFatBinary(_)));
    }

    #[test]
    fn load_bytes_matches_file_load_for_inventory() {
        let path = fixture_path("universal-hello");
        let bytes = std::fs::read(&path).expect("read universal fixture");
        let file_image = load(&path).expect("load universal fixture from file");
        let memory_image = load_bytes(Some("universal-hello".to_string()), bytes)
            .expect("load universal fixture from bytes");

        assert_eq!(file_image.architecture(), memory_image.architecture());
        assert_eq!(file_image.entry_point(), memory_image.entry_point());
        assert_eq!(file_image.selected_slice(), memory_image.selected_slice());
        assert_eq!(
            file_image.available_slices(),
            memory_image.available_slices()
        );
        assert_eq!(file_image.sections(), memory_image.sections());
        assert_eq!(file_image.symbols(), memory_image.symbols());
        assert_eq!(
            memory_image.source().memory_label(),
            Some("universal-hello")
        );
    }

    #[test]
    fn load_bytes_matches_file_load_for_disassembly() {
        let path = fixture_path("semantic-switch");
        let bytes = std::fs::read(&path).expect("read semantic-switch fixture");
        let file_image = load(&path).expect("load semantic-switch fixture from file");
        let memory_image = load_bytes(Some("semantic-switch".to_string()), bytes)
            .expect("load semantic-switch fixture from bytes");

        let request = DisassemblyRequestV2 {
            target: DisassemblyTarget::Section("__text".to_string()),
            range: None,
            limit: DisassemblyLimit::Instructions(16),
            options: DisassemblyOptions {
                include_annotations: true,
                include_value_flow: true,
            },
        };

        let file_result =
            disassemble_v2(&file_image, &request).expect("disassemble file-backed image");
        let memory_result =
            disassemble_v2(&memory_image, &request).expect("disassemble memory-backed image");

        assert_eq!(file_result, memory_result);
    }
}
