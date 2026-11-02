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
    let body: Value = serde_json::from_slice(&analyze.get_output().stdout).expect("analyze json");
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
        let body: Value = serde_json::from_slice(&show.get_output().stdout).expect("show json");
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
    assert!(
        main["seeds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s == "symbol_table")
    );

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
        .arg("xrefs")
        .arg(&target)
        .arg("--to")
        .arg("0x100000460")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
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
        .arg("xrefs")
        .arg(&target)
        .arg("--to")
        .arg("0x100000460")
        .arg("--kind")
        .arg("data")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 0);

    // missing direction is a typed argument error (exit 1, code invalid_args)
    let out = damsel()
        .arg("xrefs")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
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
        .arg("strings")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
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
        .arg("strings")
        .arg(&target)
        .arg("--name")
        .arg("hello")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 1);
    let out = damsel()
        .arg("strings")
        .arg(&target)
        .arg("--name")
        .arg("no-such-string")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
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
        .arg("search")
        .arg(&target)
        .arg("main")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "search");
    assert_eq!(body["count"], 1);
    assert_eq!(body["results"][0]["kind"], "function");
    assert_eq!(body["results"][0]["name"], "_main");

    // Kind filter restricts; string search requires the target binary.
    let out = damsel()
        .arg("search")
        .arg(&target)
        .arg("hello")
        .arg("--kind")
        .arg("string")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["strings_searchable"], true);
    assert_eq!(body["count"], 1);
    assert_eq!(body["results"][0]["name"], "hello from damsel");

    // Miss returns zero deterministically.
    let out = damsel()
        .arg("search")
        .arg(&target)
        .arg("zzz-not-present")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 0);
}

#[test]
fn analyze_elf_end_to_end_functions_and_search() {
    let dir = smoke_dir("elf");
    let target = dir.join("libfake.so.1");
    std::fs::write(&target, damsel_elf::synth::shared_object()).unwrap();

    let out = damsel()
        .arg("analyze")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "analyze");
    assert!(body["facts"].as_u64().unwrap() > 0);
    // Engine passes run: the blob's call exported_func<-main produces at
    // least one call xref and bounds for both functions.
    assert!(body["xrefs"].as_u64().unwrap() >= 1);
    assert!(body["function_bounds"].as_u64().unwrap() >= 1);

    // Wave 4 queries work unchanged on the ELF project — with bounds.
    let out = damsel()
        .arg("functions")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    let exported = body["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "exported_func")
        .expect("exported_func listed");
    assert!(
        exported["bound"].is_string(),
        "bounds present: {:?}",
        exported["bound"]
    );

    // xrefs --to the helper resolves the call site. Shared-fixture
    // layout: 3 phdrs (LOAD/DYNAMIC/NOTE) put .text at 0xE8; helper
    // (ret) at 0xF7, call at 0xEC.
    let out = damsel()
        .arg("xrefs")
        .arg(&target)
        .arg("--to")
        .arg("0xf7")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["xrefs"][0]["kind"], "call");

    // Strings parity: the fixture's .rodata message is harvested and
    // readable through the same command surface.
    let out = damsel()
        .arg("strings")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "strings");
    assert!(body["count"].as_u64().unwrap() >= 1);
    let value = body["strings"][0]["value"].as_str().unwrap();
    assert!(value.contains("hello from damsel elf"));

    let out = damsel()
        .arg("search")
        .arg(&target)
        .arg("exported")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["results"][0]["kind"], "function");
}

#[test]
fn summary_command_reports_d0_function_summary() {
    let dir = smoke_dir("summary");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    let out = damsel()
        .arg("summary")
        .arg(&target)
        .arg("--function")
        .arg("_main")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "summary");
    let summary = &body["summary"];
    assert_eq!(summary["name"], "_main");
    assert!(summary["bound"].as_str().unwrap().contains("stop=return"));
    let calls = summary["calls"].as_array().unwrap();
    assert!(calls.iter().any(|c| c["target"] == "import:_puts"));
    assert!(
        summary["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i == "_puts")
    );
    assert!(!summary["strings"].as_array().unwrap().is_empty());

    // Unknown selector is a typed error.
    let out = damsel()
        .arg("summary")
        .arg(&target)
        .arg("--function")
        .arg("nope")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err_text = String::from_utf8(out.stderr).unwrap_or_default();
    assert!(err_text.contains("function_not_found"), "got: {err_text}");
}

#[test]
fn decompile_command_renders_pseudocode_with_provenance() {
    let dir = smoke_dir("decompile");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    let out = damsel()
        .arg("decompile")
        .arg(&target)
        .arg("--function")
        .arg("_main")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "decompile");
    assert_eq!(body["pseudocode"]["name"], "_main");
    let lines = body["pseudocode"]["lines"].as_array().unwrap();
    let texts: Vec<&str> = lines.iter().map(|l| l["text"].as_str().unwrap()).collect();
    assert!(texts.iter().any(|t| t.contains("sub_100000460(")));
    assert!(texts.iter().any(|t| *t == "return;"));
    // cset now models as a conditional-set assignment (6.14) — the
    // adrp unknown from earlier versions is also gone.
    assert!(texts.iter().any(|t| t.contains("cond:")));
    assert!(
        texts.iter().any(|t| t.contains("[-0x4]")),
        "negative slot renders signed"
    );
    // Frame facts ride along.
    assert_eq!(body["frame"]["frame_size"], 32);

    // Unknown function is typed.
    let out = damsel()
        .arg("decompile")
        .arg(&target)
        .arg("--function")
        .arg("nope")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn decompile_elf_function_via_x86_lift() {
    let dir = smoke_dir("decompile-elf");
    let target = dir.join("libfake.so.1");
    std::fs::write(&target, damsel_elf::synth::shared_object()).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    let out = damsel()
        .arg("decompile")
        .arg(&target)
        .arg("--function")
        .arg("exported_func")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    let lines = body["pseudocode"]["lines"].as_array().unwrap();
    let texts: Vec<&str> = lines.iter().map(|l| l["text"].as_str().unwrap()).collect();
    // .text at 0xE8 → helper at 0xF7; the call renders resolved.
    assert!(
        texts.iter().any(|t| t.contains("sub_f7(")),
        "got: {texts:?}"
    );
    assert!(texts.iter().any(|t| *t == "return;"));
    // push/pop model as stack stores (6.16).
    assert!(texts.iter().any(|t| t.contains("&sp")));
}

#[test]
fn recon_reports_entropy_and_verdict() {
    let out = damsel()
        .arg("recon")
        .arg(repo_root().join("fixtures/bin/arm64-symbolized"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["command"], "recon");
    let report = &body["report"];
    assert!(report["symbol_count"].as_u64().unwrap() > 0);
    assert!(!report["section_entropies"].as_array().unwrap().is_empty());
    assert_eq!(report["verdict"], "Clean");
}

#[test]
fn batch_processes_list_with_typed_failures() {
    let dir = smoke_dir("batch");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    let missing = dir.join("does-not-exist");
    let list = dir.join("batch.txt");
    std::fs::write(
        &list,
        format!("{}\n{}", target.display(), missing.display()),
    )
    .unwrap();

    let out = damsel()
        .arg("batch")
        .arg("--inputs")
        .arg(&list)
        .arg("--out")
        .arg(dir.join("batch-results"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "one failure → exit 1");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("1 passed, 1 failed"));

    let jsonl = std::fs::read_to_string(dir.join("batch-results/batch.jsonl")).unwrap();
    let lines: Vec<&str> = jsonl.lines().collect();
    assert_eq!(lines.len(), 2);
    let ok: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(ok["status"], "ok");
    assert_eq!(ok["format"], "macho");
    assert!(ok["facts"].as_u64().unwrap() > 0);
    let err: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(err["status"], "error");
}

#[test]
fn diff_reports_symbol_and_address_matches() {
    let dir = smoke_dir("diff");
    let left = dir.join("arm64-symbolized");
    let right = dir.join("arm64-stripped");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &left).unwrap();
    std::fs::copy(repo_root().join("fixtures/bin/arm64-stripped"), &right).unwrap();
    damsel().arg("analyze").arg(&left).assert().success();
    damsel().arg("analyze").arg(&right).assert().success();

    let out = damsel()
        .arg("diff")
        .arg(&left)
        .arg(&right)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    let report = &body["report"];
    let matched = report["matched"].as_array().unwrap();
    assert!(matched.len() >= 3);
    assert!(matched.iter().any(|m| m["strategy"] == "ExactSymbol"));
    assert!(matched.iter().any(|m| m["name"] == "_main"));
}

#[test]
fn analyze_refuses_existing_project_and_refresh_rebuilds() {
    let dir = smoke_dir("refresh");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();

    damsel()
        .arg("analyze")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let second = damsel()
        .arg("analyze")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(second.status.code(), Some(1));
    let err_text = String::from_utf8(
        second
            .stderr
            .is_empty()
            .then(|| second.stdout.clone())
            .unwrap_or(second.stderr.clone()),
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

/// Wave 1 cache ingest mapping: analyze one projected shared-cache image,
/// then run every project surface against it without extracting the cache.
#[test]
fn analyze_shared_cache_image_end_to_end() {
    let dir = smoke_dir("cache-image");
    let cache = dir.join("valid-single-arm64.cache");
    std::fs::copy(
        repo_root().join("fixtures/shared-cache-corpus/valid-single-arm64.cache"),
        &cache,
    )
    .expect("copy synthetic cache");
    let project = dir.join("valid-single-arm64.cache.Foundation.damsel");

    let analyze = damsel()
        .arg("analyze")
        .arg(&cache)
        .arg("--image")
        .arg("Foundation")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&analyze.get_output().stdout).expect("analyze json");
    assert_eq!(body["command"], "analyze");
    assert_eq!(body["source_kind"], "shared_cache_projected_image");
    assert_eq!(
        body["image"]["install_name"],
        "/System/Library/Frameworks/Foundation.framework/Foundation"
    );
    assert_eq!(
        body["image"]["cache_uuid"],
        "11111111-2222-3333-4444-555555555555"
    );
    assert_eq!(body["image"]["image_index"], 2);
    assert!(body["facts"].as_u64().unwrap() > 0);
    assert!(body["xrefs"].as_u64().unwrap() > 0);
    assert!(project.join("project.db").is_file());
    assert!(project.join("target.json").is_file());

    // Source provenance is the projected-image kind, in the cache space.
    let show = damsel()
        .arg("project")
        .arg("show")
        .arg(&project)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&show.get_output().stdout).expect("show json");
    assert_eq!(body["sources"][0]["kind"], "shared_cache_projected_image");
    assert_eq!(body["sources"][0]["address_spaces"][0], "cache_vmaddr");

    // Function recovery runs on cache-vmaddr-relative addresses.
    let functions = damsel()
        .arg("functions")
        .arg(&project)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value =
        serde_json::from_slice(&functions.get_output().stdout).expect("functions json");
    let rows = body["functions"].as_array().expect("functions array");
    assert!(!rows.is_empty());
    assert!(
        rows.iter().any(|row| row["name"]
            .as_str()
            .is_some_and(|name| name.contains("Greeter"))),
        "expected Objective-C names in a projected Foundation image"
    );

    // Decompile resolves the image through target.json, not the cache path.
    let decompile = damsel()
        .arg("decompile")
        .arg(&project)
        .arg("--function")
        .arg("0x18020001c")
        .assert()
        .success();
    let text = String::from_utf8(decompile.get_output().stdout.clone()).unwrap();
    assert!(text.contains("fn 0x18020001c()"), "got: {text}");

    // Re-analyzing the same image refuses without --refresh.
    let second = damsel()
        .arg("analyze")
        .arg(&cache)
        .arg("--image")
        .arg("Foundation")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(second.status.code(), Some(1));
    let err_text = String::from_utf8(second.stderr).unwrap();
    assert!(err_text.contains("project_exists"), "got: {err_text}");

    // --refresh rebuilds in place.
    damsel()
        .arg("analyze")
        .arg(&cache)
        .arg("--image")
        .arg("Foundation")
        .arg("--refresh")
        .assert()
        .success();
}

/// A cache root passed without `--image` gets an actionable error instead
/// of a Mach-O parse failure.
#[test]
fn analyze_shared_cache_without_image_is_typed_error() {
    let dir = smoke_dir("cache-no-image");
    let cache = dir.join("valid-single-arm64.cache");
    std::fs::copy(
        repo_root().join("fixtures/shared-cache-corpus/valid-single-arm64.cache"),
        &cache,
    )
    .expect("copy synthetic cache");

    let out = damsel()
        .arg("analyze")
        .arg(&cache)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    // Synthetic corpus starts with `SYNTHETIC_DYLD_CACHE`, not the real
    // `dyld_v1` magic, so this asserts the filter did not misfire; the
    // real-cache path is covered by the real-cache evidence harness.
    assert!(!out.status.success());
    let err_text = String::from_utf8(out.stderr).unwrap();
    assert!(err_text.contains("unsupported_input") || err_text.contains("malformed"));
}

/// Annotations (plan feature 18, slice 1): rename → visible in functions →
/// undo → reverted; canonical export carries annotations as facts.
#[test]
fn annotate_rename_undo_round_trip() {
    let dir = smoke_dir("annotate");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();
    let project = dir.join("arm64-symbolized.damsel");

    // Rename by symbol name; the read model shows the new name.
    damsel()
        .arg("annotate")
        .arg(&project)
        .arg("--function")
        .arg("_main")
        .arg("--rename")
        .arg("main_entry")
        .arg("--tag")
        .arg("entry")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let functions = damsel()
        .arg("functions")
        .arg(&project)
        .arg("--name")
        .arg("main_entry")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value =
        serde_json::from_slice(&functions.get_output().stdout).expect("functions json");
    assert_eq!(
        body["count"], 1,
        "renamed function is findable by its new name"
    );

    // Undo pops newest first: tag, then rename.
    damsel()
        .arg("annotate")
        .arg(&project)
        .arg("--undo")
        .assert()
        .success();
    let list = damsel()
        .arg("annotate")
        .arg(&project)
        .arg("--list")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&list.get_output().stdout).expect("list json");
    assert_eq!(body["count"], 1);
    assert_eq!(body["annotations"][0]["kind"], "rename");

    damsel()
        .arg("annotate")
        .arg(&project)
        .arg("--undo")
        .assert()
        .success();
    let functions = damsel()
        .arg("functions")
        .arg(&project)
        .arg("--name")
        .arg("_main")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value =
        serde_json::from_slice(&functions.get_output().stdout).expect("functions json");
    assert_eq!(body["count"], 1, "seed name returns after undo");

    // Annotations are analysis state: exported canonically.
    damsel()
        .arg("annotate")
        .arg(&project)
        .arg("--function")
        .arg("_main")
        .arg("--comment")
        .arg("exported comment")
        .assert()
        .success();
    let export = damsel()
        .arg("project")
        .arg("export")
        .arg(&project)
        .output()
        .unwrap();
    let text = String::from_utf8(export.stdout).unwrap();
    assert!(text.contains("\"annotation\""));

    // Unknown targets are typed errors.
    let missing = damsel()
        .arg("annotate")
        .arg(&project)
        .arg("--function")
        .arg("does_not_exist")
        .arg("--tag")
        .arg("x")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(
        String::from_utf8(missing.stderr)
            .unwrap()
            .contains("function_not_found")
    );
}

/// Function matching + annotation transfer (plan feature 15, slice 1):
/// shape-exact matching, rename transfer, negative control.
#[test]
fn match_transfers_renames_across_projects() {
    let dir = smoke_dir("match");
    let left = dir.join("left");
    let right = dir.join("right");
    let other = dir.join("other");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &left).unwrap();
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &right).unwrap();
    std::fs::copy(repo_root().join("fixtures/bin/import-rich"), &other).unwrap();
    for target in [&left, &right, &other] {
        damsel().arg("analyze").arg(target).assert().success();
    }
    damsel()
        .arg("annotate")
        .arg(&left)
        .arg("--function")
        .arg("_main")
        .arg("--rename")
        .arg("main_v2")
        .assert()
        .success();

    // Shape-exact match: identical binaries match; the unrelated binary
    // is a negative control.
    let out = damsel()
        .arg("match")
        .arg(&left)
        .arg(&right)
        .arg("--transfer")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("match json");
    assert_eq!(body["command"], "match");
    assert!(body["matches"].as_u64().unwrap_or(0) >= 3);
    assert_eq!(body["transferred"], 1);

    // The rename landed in the right project via the annotation layer.
    let functions = damsel()
        .arg("functions")
        .arg(&right)
        .arg("--name")
        .arg("main_v2")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value =
        serde_json::from_slice(&functions.get_output().stdout).expect("functions json");
    assert_eq!(body["count"], 1, "rename transferred");

    let negative = damsel()
        .arg("match")
        .arg(&left)
        .arg(&other)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&negative.get_output().stdout).expect("negative json");
    assert_eq!(
        body["matches"], 0,
        "unrelated binaries must not shape-match"
    );
}

/// Call-graph queries (plan feature 10, slice 1): path, unreachability,
/// callers, rename-aware resolution — over stored facts only.
#[test]
fn query_paths_callers_and_rename_resolution() {
    let dir = smoke_dir("query");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // Direct path main -> exported_sum.
    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--from")
        .arg("_main")
        .arg("--to")
        .arg("_exported_sum")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("path json");
    assert_eq!(body["action"], "path");
    assert_eq!(body["hops"], 1);
    assert_eq!(body["path"][0]["name"], "_main");
    assert_eq!(body["path"][1]["name"], "_exported_sum");

    // Unreachability is a success with reachable=false.
    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--from")
        .arg("_exported_sum")
        .arg("--to")
        .arg("_main")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("unreachable json");
    assert_eq!(body["reachable"], false);

    // Callers.
    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--callers-of")
        .arg("_exported_sum")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("callers json");
    assert_eq!(body["callers"][0]["name"], "_main");

    // Renames resolve in queries.
    damsel()
        .arg("annotate")
        .arg(&target)
        .arg("--function")
        .arg("_exported_sum")
        .arg("--rename")
        .arg("summer")
        .assert()
        .success();
    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--from")
        .arg("_main")
        .arg("--to")
        .arg("summer")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("renamed json");
    assert_eq!(body["hops"], 1, "renamed target resolves");

    // Unknown names stay typed errors.
    let missing = damsel()
        .arg("query")
        .arg(&target)
        .arg("--from")
        .arg("_main")
        .arg("--to")
        .arg("nope")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(
        String::from_utf8(missing.stderr)
            .unwrap()
            .contains("function_not_found")
    );
}

/// Diff v2 --port (plan feature 16): renames ride matched pairs into the
/// right project through the annotation layer.
#[test]
fn diff_ports_renames_to_matched_functions() {
    let dir = smoke_dir("diff-port");
    let left = dir.join("left");
    let right = dir.join("right");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &left).unwrap();
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &right).unwrap();
    damsel().arg("analyze").arg(&left).assert().success();
    damsel().arg("analyze").arg(&right).assert().success();
    damsel()
        .arg("annotate")
        .arg(&left)
        .arg("--function")
        .arg("_main")
        .arg("--rename")
        .arg("main_renamed")
        .assert()
        .success();

    damsel()
        .arg("diff")
        .arg(&left)
        .arg(&right)
        .arg("--port")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let functions = damsel()
        .arg("functions")
        .arg(&right)
        .arg("--name")
        .arg("main_renamed")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value =
        serde_json::from_slice(&functions.get_output().stdout).expect("functions json");
    assert_eq!(body["count"], 1, "rename ported through diff");
}

/// Model refinement (plan feature 17, slice 1): proposals are
/// provenance-tagged annotations — dry-run default, apply writes,
/// actor-scoped review + undo.
#[test]
fn refine_applies_model_annotations_with_provenance() {
    let dir = smoke_dir("refine");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // Strip the name of _exported_sum by renaming to a sub_ name via the
    // annotation layer, then refine must propose for it (string refs in
    // _main's window are attributed by containment).
    damsel()
        .arg("annotate")
        .arg(&target)
        .arg("--function")
        .arg("_main")
        .arg("--rename")
        .arg("sub_1000004b8")
        .assert()
        .success();

    // Dry-run: no annotations written.
    let out = damsel()
        .arg("refine")
        .arg(&target)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("refine json");
    assert_eq!(body["command"], "refine");
    assert_eq!(body["applied"], false);
    assert!(body["proposals"].as_u64().unwrap_or(0) >= 1, "{body:#?}");
    let list = damsel()
        .arg("annotate")
        .arg(&target)
        .arg("--list")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let list_body: Value = serde_json::from_slice(&list.get_output().stdout).expect("list");
    assert_eq!(list_body["count"], 1, "dry-run wrote nothing");

    // Apply: model-actor annotations with evidence.
    let out = damsel()
        .arg("refine")
        .arg(&target)
        .arg("--apply")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("apply json");
    assert_eq!(body["applied"], true);
    assert_eq!(body["actor"], "model:heuristic-v1");
    assert!(body["applied_count"].as_u64().unwrap_or(0) >= 1);

    // The proposed name is findable and attributed to the model actor.
    let list = damsel()
        .arg("annotate")
        .arg(&target)
        .arg("--list")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let list_body: Value = serde_json::from_slice(&list.get_output().stdout).expect("list");
    let model_annotations: Vec<&Value> = list_body["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["actor"].as_str().unwrap_or("").starts_with("model:"))
        .collect();
    assert!(!model_annotations.is_empty(), "model actor attribution");
}

/// User struct types (plan feature 7, slice 1): declare, persist through
/// the annotation layer, list back — and the payload round-trips.
#[test]
fn type_declares_and_lists_structs() {
    let dir = smoke_dir("types");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    damsel()
        .arg("type")
        .arg(&target)
        .arg("--struct")
        .arg("Frame")
        .arg("--field")
        .arg("0x0:u64:count")
        .arg("--field")
        .arg("0x10:char*:name")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Listed back with both fields, in declaration order.
    let out = damsel()
        .arg("type")
        .arg(&target)
        .arg("--list")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("list json");
    assert_eq!(body["count"], 1);
    let decl = &body["structs"][0];
    assert_eq!(decl["name"], "Frame");
    assert_eq!(decl["fields"][0]["field_name"], "count");
    assert_eq!(decl["fields"][1]["field_name"], "name");
    assert_eq!(decl["fields"][1]["type_name"], "char*");

    // The declaration is an annotation (provenance + undo surface).
    let list = damsel()
        .arg("annotate")
        .arg(&target)
        .arg("--list")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&list.get_output().stdout).expect("ann json");
    assert_eq!(body["annotations"][0]["object"], "type:struct:Frame");

    // Re-declare replaces (rename replace-semantics, no duplicates).
    damsel()
        .arg("type")
        .arg(&target)
        .arg("--struct")
        .arg("Frame")
        .arg("--field")
        .arg("0x8:u64:only")
        .assert()
        .success();
    let out = damsel()
        .arg("type")
        .arg(&target)
        .arg("--list")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("redeclared");
    assert_eq!(body["count"], 1);
    assert_eq!(body["structs"][0]["fields"].as_array().unwrap().len(), 1);

    // Bad fields are typed errors.
    let bad = damsel()
        .arg("type")
        .arg(&target)
        .arg("--struct")
        .arg("Bad")
        .arg("--field")
        .arg("nope")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
}

/// Interactive steering (plan feature 9, slice 1): value assertions
/// persist as user_asserted facts with steer_v1 provenance, surface in
/// decompile output, and reject malformed input.
#[test]
fn decompile_assert_persists_user_asserted_facts() {
    let dir = smoke_dir("steer");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // Site-less (function entry) + site-pinned assertions.
    let out = damsel()
        .arg("decompile")
        .arg(&target)
        .arg("--function")
        .arg("_main")
        .arg("--assert")
        .arg("x8=0x1000")
        .arg("--assert")
        .arg("x9@0x1000004d0=0x2000")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("decompile json");
    assert_eq!(body["steered"], true, "{body:#?}");
    assert!(
        body["steer_note"]
            .as_str()
            .unwrap_or("")
            .contains("steer_v1")
    );

    // The facts are user_asserted with the steer_v1 pass, at the right
    // sites, and flow into canonical exports.
    let export = damsel()
        .arg("project")
        .arg("export")
        .arg(&target)
        .output()
        .unwrap();
    let text = String::from_utf8(export.stdout).unwrap();
    assert!(text.contains("\"value_assert\""), "fact kind in export");
    assert!(text.contains("x8=0x1000|function=0x1000004b8"));
    assert!(text.contains("\"user_asserted\""));
    assert!(text.contains("steer_v1"));

    // Malformed assertions are typed errors and write nothing.
    let before = damsel()
        .arg("project")
        .arg("export")
        .arg(&target)
        .output()
        .unwrap();
    let bad = damsel()
        .arg("decompile")
        .arg(&target)
        .arg("--function")
        .arg("_main")
        .arg("--assert")
        .arg("nope")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(
        String::from_utf8(bad.stderr)
            .unwrap()
            .contains("invalid_args")
    );
    let after = damsel()
        .arg("project")
        .arg("export")
        .arg(&target)
        .output()
        .unwrap();
    assert_eq!(
        before.stdout.len(),
        after.stdout.len() + 0, // failed run writes nothing; note: export has no timestamps
        "no writes on failure"
    );
}

/// IR emulation sandbox (plan feature 11, slice 1): seeded registers,
/// honest stops, budget guard — verified against a leaf function whose
/// arithmetic the test can predict.
#[test]
fn emulate_runs_leaf_function_to_completion() {
    let dir = smoke_dir("emulate");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // _helper(w0) = w0 + 5 → w0 = 0xa for w0=5.
    let out = damsel()
        .arg("emulate")
        .arg(&target)
        .arg("--function")
        .arg("_helper")
        .arg("--set")
        .arg("w0=5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("emulate json");
    assert_eq!(body["command"], "emulate");
    assert_eq!(body["outcome"]["kind"], "returned");
    assert_eq!(body["outcome"]["steps"], 5, "{body:#?}");
    let registers: &Vec<Value> = body["outcome"]["registers"].as_array().unwrap();
    let w0 = registers
        .iter()
        .find(|r| r["register"] == "w0")
        .expect("w0 present");
    assert_eq!(w0["value"], "0xa", "helper(5) = 10");
    assert!(body["memory_writes"].as_u64().unwrap_or(0) > 0);

    // Budget guard: 3 steps cannot finish 5.
    let out = damsel()
        .arg("emulate")
        .arg(&target)
        .arg("--function")
        .arg("_helper")
        .arg("--set")
        .arg("w0=5")
        .arg("--budget")
        .arg("3")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("budget json");
    assert_eq!(body["outcome"]["kind"], "budget_exhausted");

    // Calls stop honestly (_exported_sum calls _helper).
    let out = damsel()
        .arg("emulate")
        .arg(&target)
        .arg("--function")
        .arg("_exported_sum")
        .arg("--set")
        .arg("w0=1")
        .arg("--set")
        .arg("w1=2")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("call json");
    assert_eq!(body["outcome"]["kind"], "unsupported");
    assert!(
        body["outcome"]["what"]
            .as_str()
            .unwrap_or("")
            .contains("call not modeled")
    );

    // Bad --set is a typed error.
    let bad = damsel()
        .arg("emulate")
        .arg(&target)
        .arg("--function")
        .arg("_helper")
        .arg("--set")
        .arg("garbage")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
}

/// Type application (plan feature 7, slice 2): --bind persists pointer
/// bindings; decompile renders declared field names for bound bases at
/// declared offsets — and nothing else changes.
#[test]
fn decompile_applies_type_bindings() {
    let dir = smoke_dir("types-apply");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // Baseline: raw displacement forms present.
    let baseline = damsel()
        .arg("decompile")
        .arg(&target)
        .arg("--function")
        .arg("_main")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&baseline.get_output().stdout).expect("baseline json");
    assert_eq!(body["typed"], false);
    let text = body["pseudocode"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["text"].as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("&sp[0x10]"), "raw form in baseline:\n{text}");
    assert!(
        !text.contains("sp->"),
        "no typed forms in baseline:\n{text}"
    );

    // Declare + bind.
    damsel()
        .arg("type")
        .arg(&target)
        .arg("--struct")
        .arg("Frame")
        .arg("--field")
        .arg("0x10:u64:saved_fp")
        .arg("--field")
        .arg("0x18:u64:saved_lr")
        .assert()
        .success();
    damsel()
        .arg("type")
        .arg(&target)
        .arg("--bind")
        .arg("sp=Frame")
        .assert()
        .success();

    // Typed decompile: field names replace the declared offsets.
    let typed = damsel()
        .arg("decompile")
        .arg(&target)
        .arg("--function")
        .arg("_main")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&typed.get_output().stdout).expect("typed json");
    assert_eq!(body["typed"], true);
    let text = body["pseudocode"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["text"].as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("sp->saved_fp"), "field named:\n{text}");
    assert!(text.contains("sp->saved_lr"), "field named:\n{text}");
    assert!(!text.contains("&sp[0x10]"), "raw form replaced:\n{text}");
    assert!(!text.contains("&sp[0x18]"), "raw form replaced:\n{text}");

    // Binding an undeclared struct is a typed error.
    let bad = damsel()
        .arg("type")
        .arg(&target)
        .arg("--bind")
        .arg("x0=Missing")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(
        String::from_utf8(bad.stderr)
            .unwrap()
            .contains("undeclared struct")
    );
}

/// Taint queries (plan feature 10, slice 2): parameter-reachability
/// with precision — constant kills honored, real transfers resolve.
#[test]
fn query_taint_reachability_with_precision() {
    let dir = smoke_dir("taint");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();
    damsel().arg("analyze").arg(&target).assert().success();

    // Summary: _exported_sum(w0, w1) spills both, reloads, returns the
    // sum — both params reach the return and memory.
    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--taint-from")
        .arg("_exported_sum")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("summary json");
    assert_eq!(body["action"], "taint");
    assert_eq!(body["returns_taint_slots"], 2, "{body:#?}");
    assert_eq!(body["stores_taint_slots"], 2, "{body:#?}");
    assert!(
        body["reachable"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["function"] == "_helper"),
        "_exported_sum's w1 passes through to _helper: {body:#?}"
    );

    // Precision: _main passes constants (w0=0x2, w1=0x3) to
    // _exported_sum and 0x1 to _helper — its parameters reach neither.
    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--taint-from")
        .arg("_main")
        .arg("--taint-to")
        .arg("_helper")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("kill json");
    assert_eq!(body["reachable"], false, "constants kill the taint");

    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--taint-from")
        .arg("_main")
        .arg("--taint-to")
        .arg("_exported_sum")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("kill2 json");
    assert_eq!(body["reachable"], false);

    // Positive chain: _exported_sum -> _helper with the call site.
    let out = damsel()
        .arg("query")
        .arg(&target)
        .arg("--taint-from")
        .arg("_exported_sum")
        .arg("--taint-to")
        .arg("_helper")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("chain json");
    assert_eq!(body["reachable"], true);
    assert_eq!(body["hops"][0]["via"], "args");
    assert_eq!(body["hops"][0]["from"], "_exported_sum");
    assert_eq!(body["hops"][0]["to"], "_helper");
    assert_eq!(body["hops"][0]["site"], "0x100000488");
}

/// Pseudocode diff (plan feature 16, slice 2): matched pairs diff at
/// the line level — identical versions are clean, the v2 fixture's
/// real changes surface as remove/add pairs.
#[test]
fn diff_pseudocode_surfaces_real_changes() {
    let dir = smoke_dir("pseudodiff");
    let v1 = dir.join("v1");
    let v2 = dir.join("v2");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &v1).unwrap();
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized-v2"), &v2).unwrap();
    damsel().arg("analyze").arg(&v1).assert().success();
    damsel().arg("analyze").arg(&v2).assert().success();

    // Identical binaries: every pair clean.
    let out = damsel()
        .arg("diff")
        .arg(&v1)
        .arg(&v1)
        .arg("--pseudocode")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("self diff json");
    for diff in body["pseudocode_diffs"].as_array().unwrap() {
        assert_eq!(diff["added"], 0, "{diff:#?}");
        assert_eq!(diff["removed"], 0, "{diff:#?}");
    }

    // v1 -> v2: the three changed functions carry the real deltas.
    let out = damsel()
        .arg("diff")
        .arg(&v1)
        .arg(&v2)
        .arg("--pseudocode")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("v2 diff json");
    let diffs = body["pseudocode_diffs"].as_array().unwrap();
    let by_name = |name: &str| {
        diffs
            .iter()
            .find(|d| d["function"] == name)
            .unwrap_or_else(|| panic!("missing {name}: {diffs:#?}"))
    };
    // helper: +5 became +7.
    let helper = by_name("_helper");
    assert_eq!(helper["added"], 1);
    assert_eq!(helper["removed"], 1);
    let texts: Vec<&str> = helper["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["text"].as_str().unwrap())
        .collect();
    assert!(texts.iter().any(|t| t.contains("w8 + 0x5")), "{texts:#?}");
    assert!(texts.iter().any(|t| t.contains("w8 + 0x7")), "{texts:#?}");
    // exported_sum: helper(1) became helper(2).
    let sum = by_name("_exported_sum");
    assert_eq!(sum["added"], 1);
    assert_eq!(sum["removed"], 1);
    // main: the 11-compare became 12.
    let main = by_name("_main");
    assert_eq!(main["added"], 2);
    assert_eq!(main["removed"], 2);
}

/// Fuzzy matching tier (plan feature 15, tier 2): a restructured
/// function (shape-exact fails) is found by Dice token-similarity;
/// unrelated programs stay below the threshold.
#[test]
fn match_fuzzy_finds_restructured_function() {
    let dir = smoke_dir("fuzzy");
    let v1 = dir.join("v1");
    let v3 = dir.join("v3");
    let other = dir.join("other");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &v1).unwrap();
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized-v3"), &v3).unwrap();
    std::fs::copy(repo_root().join("fixtures/bin/import-rich"), &other).unwrap();
    for target in [&v1, &v3, &other] {
        damsel().arg("analyze").arg(target).assert().success();
    }

    // v1 vs v3: _exported_sum and _main match shape-exact; _helper
    // (restructured: multiply added) is found only by the fuzzy tier.
    let out = damsel()
        .arg("match")
        .arg(&v1)
        .arg(&v3)
        .arg("--fuzzy")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("match json");
    assert_eq!(body["matches"], 2, "shape-exact pair count: {body:#?}");
    let fuzzy = body["fuzzy_matches"].as_array().unwrap();
    assert_eq!(fuzzy.len(), 1, "the restructured helper: {fuzzy:#?}");
    assert_eq!(fuzzy[0]["right_name"], "_helper");
    assert_eq!(fuzzy[0]["left"], "0x1000004a0");
    assert!(
        fuzzy[0]["score"].as_f64().unwrap() >= 0.7,
        "score {}: {fuzzy:#?}",
        fuzzy[0]["score"]
    );

    // Without --fuzzy the tier is off.
    let out = damsel()
        .arg("match")
        .arg(&v1)
        .arg(&v3)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("plain json");
    assert_eq!(body["fuzzy_matches"].as_array().unwrap().len(), 0);

    // Cross-program: nothing fuzzy-matches above the threshold.
    let out = damsel()
        .arg("match")
        .arg(&v1)
        .arg(&other)
        .arg("--fuzzy")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let body: Value = serde_json::from_slice(&out.get_output().stdout).expect("cross json");
    assert_eq!(body["fuzzy_matches"].as_array().unwrap().len(), 0);
    assert_eq!(body["matches"], 0);
}
