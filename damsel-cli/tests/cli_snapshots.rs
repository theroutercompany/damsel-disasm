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

fn normalize_doctor_snapshot(output: &str) -> String {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if line.starts_with("host_os: ") {
                Some("host_os: <host_os>".to_string())
            } else if line.starts_with("host_architecture: ") {
                Some("host_architecture: <host_architecture>".to_string())
            } else if line.starts_with("target_triple: ") {
                Some("target_triple: <target_triple>".to_string())
            } else if line.starts_with("overall_status: ") {
                Some("overall_status: <status>".to_string())
            } else if line == "capabilities:" {
                Some("capabilities:".to_string())
            } else if trimmed.starts_with("macho_analysis: ") {
                Some("  macho_analysis: <status>".to_string())
            } else if trimmed.starts_with("fixture_rebuild: ") {
                Some("  fixture_rebuild: <status>".to_string())
            } else if trimmed.starts_with("fixture_drift_check: ") {
                Some("  fixture_drift_check: <status>".to_string())
            } else if trimmed.starts_with("bench_compile: ") {
                Some("  bench_compile: <status>".to_string())
            } else if trimmed.starts_with("bench_runtime: ") {
                Some("  bench_runtime: <status>".to_string())
            } else if trimmed.starts_with("benchmark: ") {
                Some("  benchmark: <status>".to_string())
            } else if line == "tools:" {
                Some("tools:".to_string())
            } else if trimmed.starts_with("xcrun: ") {
                Some("  xcrun: <tool_status>".to_string())
            } else if trimmed.starts_with("strip: ") {
                Some("  strip: <tool_status>".to_string())
            } else if trimmed.starts_with("clang: ") {
                Some("  clang: <tool_status>".to_string())
            } else if trimmed.starts_with("python3: ") {
                Some("  python3: <tool_status>".to_string())
            } else if trimmed.starts_with("nm: ") {
                Some("  nm: <tool_status>".to_string())
            } else if trimmed.starts_with("sdk_path_probe: ") {
                Some("  sdk_path_probe: <tool_status>".to_string())
            } else if trimmed.starts_with("selected_hash_tool: ") {
                Some("  selected_hash_tool: <tool>".to_string())
            } else if trimmed.starts_with("sha256sum: ") {
                Some("  sha256sum: <tool_status>".to_string())
            } else if trimmed.starts_with("shasum: ") {
                Some("  shasum: <tool_status>".to_string())
            } else if trimmed.starts_with("openssl: ") {
                Some("  openssl: <tool_status>".to_string())
            } else if line.starts_with("issues:") {
                Some("issues: <summary>".to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
fn dyld_filtered_snapshot() {
    let path = fixture("import-rich");
    let stdout = run_snapshot(&[
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
        "--stubs",
        "--name",
        "puts",
        "--sort",
        "name",
    ]);
    insta::assert_snapshot!("dyld_filtered_snapshot", stdout);
}

#[test]
fn dyld_kind_ordinal_snapshot() {
    let path = fixture("import-lazy");
    let stdout = run_snapshot(&[
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
        "--stubs",
        "--helpers",
        "--binding-kind",
        "lazy",
        "--stub-kind",
        "lazy",
        "--ordinal",
        "1",
        "--sort",
        "address",
    ]);
    insta::assert_snapshot!("dyld_kind_ordinal_snapshot", stdout);
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
fn objc_methods_snapshot() {
    let path = fixture("objc-sample");
    let stdout = run_snapshot(&[
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
        "--owner",
        "GreetingProviding",
    ]);
    insta::assert_snapshot!("objc_methods_snapshot", stdout);
}

#[test]
fn objc_provenance_filtered_snapshot() {
    let path = fixture("objc-sample");
    let stdout = run_snapshot(&[
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
        "--name-source",
        "pointer-table",
        "--selector-source",
        "unresolved",
    ]);
    insta::assert_snapshot!("objc_provenance_filtered_snapshot", stdout);
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

#[test]
fn disasm_show_values_snapshot() {
    let path = fixture("semantic-switch");
    let stdout = run_snapshot(&[
        "disasm",
        path.to_str().expect("utf8 path"),
        "--section",
        "__text",
        "--limit",
        "16",
        "--show-references",
        "--show-values",
    ]);
    insta::assert_snapshot!("disasm_show_values_snapshot", stdout);
}

#[test]
fn doctor_snapshot() {
    let stdout = run_snapshot(&["doctor"]);
    let normalized = normalize_doctor_snapshot(&stdout);
    insta::assert_snapshot!("doctor_snapshot", normalized);
}
