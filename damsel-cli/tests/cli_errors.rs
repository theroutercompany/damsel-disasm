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
        .stderr(predicate::str::contains("error [symbol_not_found]"));
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
        .stderr(predicate::str::contains("error [section_not_found]"));
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

#[test]
fn disasm_rejects_conflicting_count_and_limit() {
    let path = fixture("arm64-symbolized");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "disasm",
            path.to_str().expect("utf8 path"),
            "--symbol",
            "_main",
            "--count",
            "8",
            "--limit",
            "4",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "`--count` and `--limit` cannot differ",
        ));
}

#[test]
fn disasm_rejects_bytes_with_to() {
    let path = fixture("arm64-symbolized");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "disasm",
            path.to_str().expect("utf8 path"),
            "--section",
            "__text",
            "--bytes",
            "32",
            "--to",
            "0x100000480",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "`--to` cannot be combined with `--bytes`",
        ));
}

#[test]
fn disasm_rejects_invalid_range() {
    let path = fixture("arm64-symbolized");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "disasm",
            path.to_str().expect("utf8 path"),
            "--addr",
            "0x1000004d0",
            "--from",
            "0x1000004d0",
            "--to",
            "0x1000004c0",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "`--to` must be greater than the decode start",
        ));
}
