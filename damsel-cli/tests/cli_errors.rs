use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_INPUT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("fixtures/bin").join(name)
}

fn write_temp_input(bytes: &[u8]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let counter = TEMP_INPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "damsel-cli-test-{}-{nanos}-{counter}.bin",
        std::process::id()
    ));
    fs::write(&path, bytes).expect("write temp input");
    path
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
fn info_non_macho_returns_typed_unsupported_input_error() {
    let path = write_temp_input(b"not a macho file");
    let path_string = path.to_string_lossy().to_string();
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(["info", path_string.as_str()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error [unsupported_input]"));
    let _ = fs::remove_file(path);
}

#[test]
fn doctor_require_status_without_check_is_rejected() {
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(["doctor", "--require-status", "supported"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "the following required arguments were not provided",
        ));
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

#[test]
fn dyld_rejects_invalid_binding_kind_value() {
    let path = fixture("import-lazy");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "dyld",
            path.to_str().expect("utf8 path"),
            "--binding-kind",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn dyld_rejects_invalid_stub_kind_value() {
    let path = fixture("import-lazy");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "dyld",
            path.to_str().expect("utf8 path"),
            "--stub-kind",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn dyld_rejects_invalid_export_kind_value() {
    let path = fixture("import-lazy");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "dyld",
            path.to_str().expect("utf8 path"),
            "--export-kind",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn dyld_rejects_invalid_export_flag_value() {
    let path = fixture("import-lazy");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "dyld",
            path.to_str().expect("utf8 path"),
            "--export-flag",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn dyld_rejects_invalid_source_value() {
    let path = fixture("import-lazy");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "dyld",
            path.to_str().expect("utf8 path"),
            "--source",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn dyld_rejects_invalid_ordinal_value() {
    let path = fixture("import-lazy");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "dyld",
            path.to_str().expect("utf8 path"),
            "--ordinal",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn objc_rejects_invalid_category_source_value() {
    let path = fixture("objc-sample");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "objc",
            path.to_str().expect("utf8 path"),
            "--category-source",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn objc_rejects_invalid_name_source_value() {
    let path = fixture("objc-sample");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "objc",
            path.to_str().expect("utf8 path"),
            "--name-source",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn objc_rejects_invalid_selector_source_value() {
    let path = fixture("objc-sample");
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args([
            "objc",
            path.to_str().expect("utf8 path"),
            "--selector-source",
            "bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}
