use damsel_macho::{MachoError, load};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn rejects_thin_x86_64_macho_fixture() {
    let path = fixture("x86_64-only-hello");
    if !path.exists() {
        panic!("required fixture missing: {}", path.display());
    }

    let error = load(&path).expect_err("thin x86_64 macho must be rejected");
    assert!(
        matches!(error, MachoError::UnsupportedArchitecture(_)),
        "expected UnsupportedArchitecture, got: {error:?}"
    );
}
