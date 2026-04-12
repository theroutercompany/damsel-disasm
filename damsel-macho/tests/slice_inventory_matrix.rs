use damsel_core::Architecture;
use damsel_macho::load;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn thin_fixture_reports_single_selected_slice_descriptor() {
    let image = load(fixture("arm64-symbolized")).expect("load thin arm64 fixture");
    let available = image.available_slices();
    assert_eq!(
        available.len(),
        1,
        "thin fixture should expose one available slice descriptor"
    );

    let selected = image
        .selected_slice_descriptor()
        .expect("selected slice descriptor");
    assert!(selected.selected, "selected descriptor must be marked selected");
    assert_eq!(
        selected.architecture,
        Architecture::Arm64,
        "thin arm64 fixture should report arm64 architecture"
    );
    assert!(
        !selected.is_universal,
        "thin fixture selected descriptor should not be universal"
    );
    assert_eq!(
        selected.offset,
        image.selected_slice().offset,
        "descriptor offset should mirror selected slice"
    );
    assert_eq!(
        selected.size,
        image.selected_slice().size,
        "descriptor size should mirror selected slice"
    );
}

#[test]
fn universal_fixture_selected_descriptor_is_marked_universal() {
    let image = load(fixture("universal-hello")).expect("load universal fixture");
    let available = image.available_slices();
    assert_eq!(
        available.len(),
        1,
        "current loader semantics expose selected universal slice only"
    );

    let selected = image
        .selected_slice_descriptor()
        .expect("selected slice descriptor");
    assert!(selected.selected, "selected descriptor must be selected");
    assert!(
        selected.is_universal,
        "selected descriptor should preserve universal-source metadata"
    );
    assert!(
        image.selected_slice().is_universal,
        "selected slice should preserve universal-source metadata"
    );
    assert_eq!(
        selected.architecture,
        image.architecture,
        "selected descriptor architecture should match image architecture"
    );
}

#[test]
fn arm64e_fixture_selected_descriptor_reports_arm64e_when_present() {
    let path = fixture("arm64e-sample");
    if !path.exists() {
        eprintln!("skipping: arm64e fixture missing at {}", path.display());
        return;
    }

    let image = load(&path).expect("load arm64e fixture");
    let available = image.available_slices();
    assert_eq!(
        available.len(),
        1,
        "current loader semantics expose one selected descriptor"
    );

    let selected = image
        .selected_slice_descriptor()
        .expect("selected slice descriptor");
    assert_eq!(
        selected.architecture,
        Architecture::Arm64e,
        "arm64e fixture should report arm64e descriptor architecture"
    );
    assert_eq!(
        image.architecture,
        Architecture::Arm64e,
        "arm64e fixture should report arm64e image architecture"
    );
}
