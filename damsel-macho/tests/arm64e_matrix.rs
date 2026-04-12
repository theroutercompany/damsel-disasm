use damsel_core::Architecture;
use damsel_macho::load;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn loads_arm64e_fixture_when_present() {
    let path = fixture("arm64e-sample");
    if !path.exists() {
        eprintln!("skipping: arm64e fixture missing at {}", path.display());
        return;
    }

    let image = load(&path).expect("load arm64e fixture");
    assert_eq!(image.architecture, Architecture::Arm64e);
    assert_eq!(image.slice.is_universal, false);
    assert!(
        !image.sections().is_empty(),
        "expected non-empty section table"
    );
    assert!(
        image.entry_point.is_some(),
        "expected entry point in arm64e fixture"
    );
}
