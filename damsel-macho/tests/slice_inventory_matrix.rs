use damsel_core::{Architecture, Platform};
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
    assert!(
        selected.selected,
        "selected descriptor must be marked selected"
    );
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
    assert!(
        available.len() >= 2,
        "universal fixture should expose every supported slice descriptor"
    );
    assert!(
        available.iter().any(|descriptor| descriptor.selected),
        "one slice descriptor must be marked selected"
    );
    assert!(
        available
            .iter()
            .any(|descriptor| descriptor.architecture == Architecture::X86_64),
        "universal fixture should surface the x86_64 slice inventory too"
    );
    assert!(
        available
            .iter()
            .any(|descriptor| descriptor.architecture == Architecture::Arm64),
        "universal fixture should surface the arm64 slice inventory too"
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
        image.architecture(),
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
        image.architecture(),
        Architecture::Arm64e,
        "arm64e fixture should report arm64e image architecture"
    );
}

#[test]
fn platform_metadata_is_fixture_derived_and_host_agnostic() {
    let thin = load(fixture("arm64-symbolized")).expect("load thin arm64 fixture");
    let universal = load(fixture("universal-hello")).expect("load universal fixture");

    assert_eq!(thin.platform(), Some(&Platform::MacOS));
    assert_eq!(universal.platform(), Some(&Platform::MacOS));
    assert_eq!(
        thin.platform(),
        universal.platform(),
        "platform metadata should come from Mach-O load commands, not host environment"
    );
}

#[test]
fn universal_slice_selection_is_deterministic_across_reloads() {
    let first = load(fixture("universal-hello")).expect("first universal load");
    let second = load(fixture("universal-hello")).expect("second universal load");

    let first_selected = first
        .selected_slice_descriptor()
        .expect("first selected descriptor");
    let second_selected = second
        .selected_slice_descriptor()
        .expect("second selected descriptor");

    assert_eq!(first_selected.architecture, second_selected.architecture);
    assert_eq!(first_selected.cpu_subtype, second_selected.cpu_subtype);
    assert_eq!(first_selected.offset, second_selected.offset);
    assert_eq!(first_selected.size, second_selected.size);
}
