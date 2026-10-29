//! Project CLI contract tests (Wave 4 Unit 4.3; evidence rows 4.1/4.3 via
//! the CLI surface): analyze → show → export, typed `project_exists`
//! refusal, `--refresh` rebuild.

use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn smoke_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("damsel-project-cli-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create smoke dir");
    dir
}

fn damsel() -> Command {
    Command::cargo_bin("damsel-cli").expect("damsel-cli binary")
}

#[test]
fn analyze_show_export_round_trip() {
    let dir = smoke_dir("round-trip");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    let project = dir.join("arm64-symbolized.damsel");

    let analyze = damsel()
        .arg("analyze")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value =
        serde_json::from_slice(&analyze.get_output().stdout).expect("analyze json");
    assert_eq!(body["command"], "analyze");
    assert_eq!(body["sources"], 1);
    assert!(body["facts"].as_u64().unwrap() > 0);
    assert!(body["objects"].as_u64().unwrap() > 0);
    assert!(body["project"]["project_id"].is_string());

    // project show resolves both the .damsel dir and the adjacent form.
    for spec in [&project, &target] {
        let show = damsel()
            .arg("project")
            .arg("show")
            .arg(spec)
            .arg("--format")
            .arg("json")
            .assert()
            .success();
        let body: Value =
            serde_json::from_slice(&show.get_output().stdout).expect("show json");
        assert_eq!(body["command"], "project_show");
        assert_eq!(body["project"]["name"], "arm64-symbolized");
        assert_eq!(body["sources"][0]["kind"], "macho_image");
        assert_eq!(body["sources"][0]["address_spaces"][0], "image_vmaddr");
    }

    // Export is canonical JSON with the documented top-level keys and the
    // project-level ingest marker sorting first among facts.
    let export = damsel()
        .arg("project")
        .arg("export")
        .arg(&project)
        .output()
        .unwrap();
    assert!(export.status.success());
    let text = String::from_utf8(export.stdout).unwrap();
    assert!(text.starts_with("{\"facts\":[{\"confidence\":\"exact\",\"kind\":\"project_note\""));
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert!(value["sources"].as_array().unwrap().len() == 1);
    assert!(value["facts"].as_array().unwrap().len() > 5);
}

#[test]
fn functions_command_lists_seeded_functions() {
    let dir = smoke_dir("functions");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    let out = damsel()
        .arg("functions")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "functions");
    let functions = body["functions"].as_array().unwrap();
    assert!(functions.len() >= 4);
    let main = functions
        .iter()
        .find(|f| f["name"] == "_main")
        .expect("_main listed");
    assert!(main["seeds"].as_array().unwrap().iter().any(|s| s == "symbol_table"));

    // Name filter narrows deterministically.
    let out = damsel()
        .arg("functions")
        .arg(&target)
        .arg("--name")
        .arg("main")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    let functions = body["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0]["name"], "_main");
}

#[test]
fn xrefs_command_queries_by_direction_and_kind() {
    let dir = smoke_dir("xrefs");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // --to a known function address: exactly the call site, attributed.
    let out = damsel()
        .arg("xrefs").arg(&target)
        .arg("--to").arg("0x100000460")
        .arg("--format").arg("json")
        .output().unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "xrefs");
    assert_eq!(body["count"], 1);
    let row = &body["xrefs"][0];
    assert_eq!(row["kind"], "call");
    assert_eq!(row["to"], "function:0x100000460");
    assert_eq!(row["from_function"], "_main");

    // kind filter narrows; a kind with no matches returns count 0.
    let out = damsel()
        .arg("xrefs").arg(&target)
        .arg("--to").arg("0x100000460")
        .arg("--kind").arg("data")
        .arg("--format").arg("json")
        .output().unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 0);

    // missing direction is a typed argument error (exit 1, code invalid_args)
    let out = damsel().arg("xrefs").arg(&target).arg("--format").arg("json").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err_text = String::from_utf8(out.stderr).unwrap_or_default();
    assert!(err_text.contains("invalid_args"), "got: {err_text}");
}

#[test]
fn strings_command_lists_values_and_filters() {
    let dir = smoke_dir("strings");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    let out = damsel()
        .arg("strings").arg(&target)
        .arg("--format").arg("json")
        .output().unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "strings");
    assert_eq!(body["values_available"], true);
    let strings = body["strings"].as_array().unwrap();
    assert!(!strings.is_empty());
    let value = strings[0]["value"].as_str().unwrap();
    assert!(value.contains("hello from damsel"));

    // name filter narrows to the match; a miss returns zero.
    let out = damsel()
        .arg("strings").arg(&target)
        .arg("--name").arg("hello")
        .arg("--format").arg("json")
        .output().unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 1);
    let out = damsel()
        .arg("strings").arg(&target)
        .arg("--name").arg("no-such-string")
        .arg("--format").arg("json")
        .output().unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn search_ranks_across_kinds_with_filters() {
    let dir = smoke_dir("search");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // Cross-kind search: `main` finds the function; ranking puts exact
    // matches first.
    let out = damsel()
        .arg("search").arg(&target).arg("main")
        .arg("--format").arg("json")
        .output().unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "search");
    assert_eq!(body["count"], 1);
    assert_eq!(body["results"][0]["kind"], "function");
    assert_eq!(body["results"][0]["name"], "_main");

    // Kind filter restricts; string search requires the target binary.
    let out = damsel()
        .arg("search").arg(&target).arg("hello")
        .arg("--kind").arg("string")
        .arg("--format").arg("json")
        .output().unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["strings_searchable"], true);
    assert_eq!(body["count"], 1);
    assert_eq!(body["results"][0]["name"], "hello from damsel");

    // Miss returns zero deterministically.
    let out = damsel()
        .arg("search").arg(&target).arg("zzz-not-present")
        .arg("--format").arg("json")
        .output().unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn analyze_elf_end_to_end_functions_and_search() {
    let dir = smoke_dir("elf");
    let target = dir.join("libfake.so.1");
    std::fs::write(&target, damsel_elf::synth::shared_object()).unwrap();

    let out = damsel()
        .arg("analyze").arg(&target)
        .arg("--format").arg("json")
        .output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "analyze");
    assert!(body["facts"].as_u64().unwrap() > 0);
    assert_eq!(body["engine_passes"], "ingest_only (x86_64 engine passes pending)");

    // Wave 4 queries work unchanged on the ELF project.
    let out = damsel()
        .arg("functions").arg(&target)
        .arg("--format").arg("json")
        .output().unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(body["functions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["name"] == "exported_func"));

    let out = damsel()
        .arg("search").arg(&target).arg("exported")
        .arg("--format").arg("json")
        .output().unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["results"][0]["kind"], "function");
}

#[test]
fn analyze_refuses_existing_project_and_refresh_rebuilds() {
    let dir = smoke_dir("refresh");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();

    damsel().arg("analyze").arg(&target).arg("--format").arg("json").assert().success();

    let second = damsel()
        .arg("analyze")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(second.status.code(), Some(1));
    let err_text = String::from_utf8(
        second.stderr.is_empty().then(|| second.stdout.clone()).unwrap_or(second.stderr.clone()),
    )
    .unwrap();
    assert!(err_text.contains("project_exists"), "got: {err_text}");

    damsel()
        .arg("analyze")
        .arg(&target)
        .arg("--refresh")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
}

#[test]
fn project_show_unknown_is_typed_error() {
    let dir = smoke_dir("missing");
    let out = damsel()
        .arg("project")
        .arg("show")
        .arg(dir.join("does-not-exist"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err_text = String::from_utf8(out.stderr).unwrap_or_default();
    assert!(err_text.contains("project_not_found"), "got: {err_text}");
}
