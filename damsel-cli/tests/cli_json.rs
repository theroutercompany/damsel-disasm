use assert_cmd::Command;
use serde_json::Value;
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

fn parse_json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("invalid json: {error}\n{text}"))
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
    let json = parse_json(&out);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "info");
    assert_eq!(json["data"]["architecture"], "arm64");
    assert!(json["data"]["available_slices"].is_array());
    assert!(json["data"]["dyld"]["stub_helpers"].is_number());
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
    let json = parse_json(&out);
    assert_eq!(json["command"], "dyld");
    assert!(json["data"]["included_sections"].is_array());
    assert!(json["data"]["import_bindings"].is_array());
    assert!(json["data"]["stubs"].is_array());
    assert!(json["data"]["stub_helpers"].is_array());
    let stubs = json["data"]["stubs"].as_array().expect("stub array");
    assert!(!stubs.is_empty(), "{out}");
    assert_eq!(stubs[0]["section"], "__TEXT:__stubs");
    assert!(stubs[0]["stub_kind"].is_string());
}

#[test]
fn dyld_json_filters_bindings_by_name_and_source() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
        "--name",
        "puts",
        "--source",
        "indirect-symbol",
    ]);
    let json = parse_json(&out);
    let bindings = json["data"]["import_bindings"]
        .as_array()
        .expect("binding array");
    assert!(!bindings.is_empty(), "{out}");
    assert!(bindings.iter().all(|binding| {
        binding["name"]
            .as_str()
            .is_some_and(|name| name.to_ascii_lowercase().contains("puts"))
            && binding["source"] == "IndirectSymbol"
            && binding["binding_kind"] == "Lazy"
    }));
}

#[test]
fn dyld_json_exposes_stub_helpers_for_lazy_fixture() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
    ]);
    let json = parse_json(&out);
    let helpers = json["data"]["stub_helpers"]
        .as_array()
        .expect("helper array");
    assert!(!helpers.is_empty(), "{out}");
    assert!(helpers[0]["target_stub"].is_number());
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
    let json = parse_json(&out);
    assert_eq!(json["command"], "objc");
    assert!(json["data"]["class_names"].is_array());
    assert!(json["data"]["pointer_refs"].is_array());
    assert!(json["data"]["classes"].is_array());
    assert!(json["data"]["protocols"].is_array());
    assert!(json["data"]["categories"].is_array());
    let classes = json["data"]["classes"].as_array().expect("class array");
    if let Some(class) = classes.first() {
        assert!(class["name_source"].is_string());
    }
}

#[test]
fn objc_json_detail_and_owner_filters_are_structured() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
        "--owner",
        "GreetingProviding",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["requested_detail"], "methods");
    assert_eq!(json["data"]["owner_filter"], "GreetingProviding");
    let methods = json["data"]["methods"].as_array().expect("methods array");
    assert!(!methods.is_empty(), "{out}");
    assert!(methods.iter().all(|entry| {
        entry["owner_name"]
            .as_str()
            .is_some_and(|name| name.contains("GreetingProviding"))
    }));
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
    let json = parse_json(&out);
    assert_eq!(json["command"], "slices");
    let slices = json["data"].as_array().expect("slice array");
    assert!(slices.iter().any(|entry| entry["selected"] == true));
    assert!(slices.iter().any(|entry| entry["architecture"] == "arm64"));
    assert!(slices.iter().any(|entry| entry["architecture"] == "x86_64"));
}

#[test]
fn disasm_json_contract_has_window_and_analysis_fields() {
    let path = fixture("semantic-switch");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--section",
        "__text",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "disasm");
    assert!(
        json["data"]["window_end"].is_null() || json["data"]["window_end"].is_number()
    );
    assert!(json["data"]["decoded_bytes"].is_number());
    assert!(json["data"]["stop_reason"].is_string());
    assert!(json["data"]["instruction_count"].is_number());
    let instructions = json["data"]["instructions"].as_array().expect("instruction array");
    assert!(!instructions.is_empty(), "{out}");
    assert!(instructions[0]["references"].is_array());
    assert!(instructions[0]["annotations"].is_array());
    assert!(instructions[0]["recovered_values"].is_array());
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations.iter().any(|annotation| {
                    annotation["type"] == "jump_table_candidate"
                })
            })
    }));
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
    let json = parse_json(&err);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "error");
    assert_eq!(json["data"]["code"], "invalid_args");
    assert_eq!(
        json["data"]["message"],
        "`--count` and `--limit` cannot differ when both are provided"
    );
}
