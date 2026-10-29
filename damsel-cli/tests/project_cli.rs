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
