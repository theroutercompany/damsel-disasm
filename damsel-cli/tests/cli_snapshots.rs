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

fn normalize(output: &[u8]) -> String {
    let root = repo_root();
    String::from_utf8_lossy(output)
        .replace(root.to_string_lossy().as_ref(), "$REPO")
        .trim()
        .to_string()
}

fn run_snapshot(args: &[&str]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(output.status.success(), "command failed: {:?}", output);
    normalize(&output.stdout)
}

#[test]
fn info_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["info", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("info_snapshot", stdout);
}

#[test]
fn symbols_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["symbols", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("symbols_snapshot", stdout);
}

#[test]
fn objc_snapshot() {
    let path = fixture("objc-sample");
    let stdout = run_snapshot(&["objc", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("objc_snapshot", stdout);
}

#[test]
fn disasm_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&[
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_main",
        "--limit",
        "8",
    ]);
    insta::assert_snapshot!("disasm_snapshot", stdout);
}
