use assert_cmd::Command;
use predicates::prelude::*;
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

#[test]
fn disasm_unknown_symbol_returns_error() {
    let path = fixture("arm64-symbolized");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "disasm",
            path.to_str().expect("utf8 path"),
            "--symbol",
            "__missing_symbol__",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("symbol not found"));
}

#[test]
fn disasm_unknown_section_returns_error() {
    let path = fixture("arm64-symbolized");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "disasm",
            path.to_str().expect("utf8 path"),
            "--section",
            "__missing_section__",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("section not found"));
}

#[test]
fn disasm_requires_target_group() {
    let path = fixture("arm64-symbolized");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(["disasm", path.to_str().expect("utf8 path")])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn disasm_rejects_invalid_address_value() {
    let path = fixture("arm64-symbolized");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "disasm",
            path.to_str().expect("utf8 path"),
            "--addr",
            "not-an-address",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}
