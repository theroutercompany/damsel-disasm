use assert_cmd::Command;
use damsel_macho::load;
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

fn run_snapshot_owned(args: &[String]) -> String {
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
fn sections_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["sections", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("sections_snapshot", stdout);
}

#[test]
fn imports_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["imports", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("imports_snapshot", stdout);
}

#[test]
fn dyld_snapshot() {
    let path = fixture("import-rich");
    let stdout = run_snapshot(&["dyld", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("dyld_snapshot", stdout);
}

#[test]
fn slices_snapshot() {
    let path = fixture("universal-hello");
    let stdout = run_snapshot(&["slices", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("slices_snapshot", stdout);
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

#[test]
fn disasm_addr_snapshot() {
    let path = fixture("arm64-symbolized");
    let image = load(&path).expect("load fixture");
    let main = image.symbol_by_name("_main").expect("main symbol");
    let stdout = run_snapshot_owned(&[
        "disasm".to_string(),
        path.to_string_lossy().to_string(),
        "--addr".to_string(),
        format!("{:#x}", main.address),
        "--limit".to_string(),
        "8".to_string(),
    ]);
    insta::assert_snapshot!("disasm_addr_snapshot", stdout);
}

#[test]
fn disasm_section_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&[
        "disasm",
        path.to_str().expect("utf8 path"),
        "--section",
        "__text",
        "--limit",
        "8",
    ]);
    insta::assert_snapshot!("disasm_section_snapshot", stdout);
}
