use assert_cmd::Command;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("fixtures/bin").join(name)
}

fn run_json_ok(args: &[&str]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(
        output.status.success(),
        "stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn run_json_err(args: &[&str]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(
        !output.status.success(),
        "expected failure, stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

#[test]
fn info_json_contract() {
    let path = fixture("arm64-symbolized");
    let out = run_json_ok(&[
        "--format",
        "json",
        "info",
        path.to_str().expect("utf8 path"),
    ]);
    assert!(out.contains("\"schema_version\":1"), "{out}");
    assert!(out.contains("\"command\":\"info\""), "{out}");
    assert!(out.contains("\"data\""), "{out}");
    assert!(out.contains("\"architecture\""), "{out}");
    assert!(out.contains("\"dyld\""), "{out}");
}

#[test]
fn dyld_json_contract_exposes_bindings_and_stubs() {
    let path = fixture("import-rich");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
    ]);
    assert!(out.contains("\"command\":\"dyld\""), "{out}");
    assert!(out.contains("\"import_bindings\""), "{out}");
    assert!(out.contains("\"stubs\""), "{out}");
    assert!(out.contains("\"section\":\"__TEXT:__stubs\""), "{out}");
}

#[test]
fn objc_json_contract_exposes_structured_runtime_records() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
    ]);
    assert!(out.contains("\"command\":\"objc\""), "{out}");
    assert!(out.contains("\"class_names\""), "{out}");
    assert!(out.contains("\"pointer_refs\""), "{out}");
    assert!(out.contains("\"classes\""), "{out}");
    assert!(out.contains("\"protocols\""), "{out}");
    assert!(out.contains("\"categories\""), "{out}");
}

#[test]
fn slices_json_contract_exposes_full_inventory() {
    let path = fixture("universal-hello");
    let out = run_json_ok(&[
        "--format",
        "json",
        "slices",
        path.to_str().expect("utf8 path"),
    ]);
    assert!(out.contains("\"command\":\"slices\""), "{out}");
    assert!(out.contains("\"selected\":true"), "{out}");
    assert!(out.contains("\"architecture\":\"arm64\""), "{out}");
    assert!(out.contains("\"architecture\":\"x86_64\""), "{out}");
}

#[test]
fn disasm_json_contract_has_window_and_analysis_fields() {
    let path = fixture("arm64-symbolized");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_main",
        "--from",
        "0x1000004c8",
        "--to",
        "0x1000004d8",
        "--show-references",
    ]);
    assert!(out.contains("\"command\":\"disasm\""), "{out}");
    assert!(out.contains("\"window_end\":"), "{out}");
    assert!(out.contains("\"decoded_bytes\""), "{out}");
    assert!(out.contains("\"stop_reason\""), "{out}");
    assert!(out.contains("\"instruction_count\""), "{out}");
    assert!(out.contains("\"references\""), "{out}");
}

#[test]
fn error_json_envelope_for_invalid_args() {
    let path = fixture("arm64-symbolized");
    let err = run_json_err(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_main",
        "--count",
        "8",
        "--limit",
        "4",
    ]);
    assert!(err.contains("\"schema_version\":1"), "{err}");
    assert!(err.contains("\"command\":\"error\""), "{err}");
    assert!(err.contains("\"code\":\"invalid_args\""), "{err}");
    assert!(
        err.contains("\"message\":\"`--count` and `--limit` cannot differ"),
        "{err}"
    );
}
