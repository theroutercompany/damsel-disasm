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
