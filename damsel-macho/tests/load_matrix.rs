use damsel_core::Architecture;
use damsel_macho::{MachoError, load};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

fn write_temp_fixture(bytes: &[u8]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("damsel-test-{nanos}.bin"));
    fs::write(&path, bytes).expect("write temp fixture");
    path
}

#[test]
fn loads_universal_fixture_with_arm64_slice() {
    let image = load(fixture("universal-hello")).expect("load universal fixture");
    assert_eq!(image.architecture, Architecture::Arm64);
    assert!(image.slice.is_universal);
}

#[test]
fn loads_objc_fixture_metadata() {
    let image = load(fixture("objc-sample")).expect("load objc fixture");
    assert!(!image.objc.class_names.is_empty());
    assert!(!image.objc.selector_names.is_empty());
}

#[test]
fn rejects_non_macho_fixture_without_panicking() {
    let path = write_temp_fixture(b"this is not a macho file");
    let error = load(&path).expect_err("non-mach-o should fail");
    let _ = fs::remove_file(path);
    assert!(matches!(
        error,
        MachoError::UnsupportedFileKind(_) | MachoError::Object(_) | MachoError::Goblin(_)
    ));
}

#[test]
fn rejects_truncated_fixture_without_panicking() {
    let error = load(fixture("malformed-truncated")).expect_err("expected parse failure");
    assert!(matches!(
        error,
        MachoError::Object(_) | MachoError::Goblin(_) | MachoError::UnsupportedFileKind(_)
    ));
}
