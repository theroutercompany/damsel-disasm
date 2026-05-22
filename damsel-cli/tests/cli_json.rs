use assert_cmd::Command;
use damsel_core::{
    CompatibilityCapability, CompatibilityPolicy, CompatibilityToolRequirement,
    CompatibilityVerificationScenario,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
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

fn cache_fixture(name: &str) -> PathBuf {
    repo_root().join("fixtures/shared-cache-corpus").join(name)
}

fn write_temp_input(bytes: &[u8]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let counter = TEMP_INPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "damsel-cli-json-test-{}-{nanos}-{counter}.bin",
        std::process::id()
    ));
    fs::write(&path, bytes).expect("write temp input");
    path
}

fn run_json_ok(args: &[&str]) -> String {
    let output = run_json_output(args);
    assert!(
        output.status.success(),
        "stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn run_json_err(args: &[&str]) -> String {
    let output = run_json_output(args);
    assert!(
        !output.status.success(),
        "expected failure, stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

fn run_json_output(args: &[&str]) -> Output {
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs")
}

fn run_json_output_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::cargo_bin("damsel-cli").expect("binary exists");
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("command runs")
}

fn run_json_ok_with_env(args: &[&str], envs: &[(&str, &str)]) -> String {
    let output = run_json_output_with_env(args, envs);
    assert!(
        output.status.success(),
        "stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn parse_json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("invalid json: {error}\n{text}"))
}

fn sorted_object_keys(value: &Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .unwrap_or_else(|| panic!("expected object: {value}"))
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn status_severity(status: &str) -> u8 {
    match status {
        "supported" => 0,
        "supported-with-degraded-features" => 1,
        "unsupported" => 2,
        other => panic!("unexpected status: {other}"),
    }
}

fn max_status<'a>(statuses: impl IntoIterator<Item = &'a str>) -> &'a str {
    statuses
        .into_iter()
        .max_by_key(|status| status_severity(status))
        .expect("at least one status")
}

fn assert_exact_object_keys(value: &Value, expected: &[&str]) {
    let mut expected_keys = expected
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    expected_keys.sort();
    assert_eq!(sorted_object_keys(value), expected_keys);
}

fn assert_raw_key_order(raw: &str, keys: &[&str]) {
    let mut offset = 0usize;
    for key in keys {
        let needle = format!("\"{key}\":");
        let found = raw[offset..]
            .find(&needle)
            .unwrap_or_else(|| panic!("missing key token in output: {needle}"));
        offset += found + needle.len();
    }
}

fn raw_object_source<'a>(raw: &'a str, key: &str) -> &'a str {
    let needle = format!("\"{key}\":");
    let key_offset = raw
        .find(&needle)
        .unwrap_or_else(|| panic!("missing object key token in output: {needle}"));
    let object_start = raw[key_offset + needle.len()..]
        .find('{')
        .map(|offset| key_offset + needle.len() + offset)
        .unwrap_or_else(|| panic!("missing object body for key: {key}"));
    let mut depth = 0usize;
    for (relative_idx, ch) in raw[object_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &raw[object_start..=object_start + relative_idx];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated object body for key: {key}");
}

fn assert_raw_object_key_order(raw: &str, key: &str, keys: &[&str]) {
    assert_raw_key_order(raw_object_source(raw, key), keys);
}

fn issue_set(value: &Value) -> BTreeSet<(String, String)> {
    value
        .as_array()
        .expect("issues array")
        .iter()
        .map(|issue| {
            (
                issue["code"].as_str().expect("issue code").to_string(),
                issue["message"]
                    .as_str()
                    .expect("issue message")
                    .to_string(),
            )
        })
        .collect()
}

fn reason_code_set(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .expect("reasons array")
        .iter()
        .map(|reason| reason["code"].as_str().expect("reason code").to_string())
        .collect()
}

#[cfg(unix)]
fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let counter = TEMP_INPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "damsel-cli-{prefix}-{}-{nanos}-{counter}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[cfg(unix)]
fn write_exec_script(path: &Path, body: &str) {
    fs::write(path, body).expect("write script");
    let mut perms = fs::metadata(path).expect("script metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod script");
}

#[cfg(unix)]
#[derive(Debug)]
struct FakeDoctorHarness {
    root: PathBuf,
}

#[cfg(unix)]
impl FakeDoctorHarness {
    fn all_usable() -> Self {
        Self::new(HarnessProfile {
            xcrun_success: true,
            clang_success: true,
            strip_success: true,
            sdk_probe_success: true,
            sha256sum_success: true,
            shasum_success: true,
            openssl_success: true,
            python3_success: true,
            nm_success: true,
            swiftc_success: true,
        })
    }

    fn broken_hash_and_sdk() -> Self {
        Self::new(HarnessProfile {
            xcrun_success: true,
            clang_success: true,
            strip_success: true,
            sdk_probe_success: false,
            sha256sum_success: false,
            shasum_success: false,
            openssl_success: false,
            python3_success: true,
            nm_success: true,
            swiftc_success: true,
        })
    }

    fn python_unusable() -> Self {
        Self::new(HarnessProfile {
            xcrun_success: true,
            clang_success: true,
            strip_success: true,
            sdk_probe_success: true,
            sha256sum_success: true,
            shasum_success: false,
            openssl_success: false,
            python3_success: false,
            nm_success: true,
            swiftc_success: true,
        })
    }

    fn from_verification_scenario(scenario: &CompatibilityVerificationScenario) -> Self {
        let profile = HarnessProfile {
            xcrun_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Xcrun),
            clang_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Clang),
            strip_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Strip),
            sdk_probe_success: scenario_tool_usable(
                scenario,
                CompatibilityToolRequirement::XcrunSdkPathProbe,
            ),
            sha256sum_success: scenario_tool_usable(
                scenario,
                CompatibilityToolRequirement::Sha256sum,
            ),
            shasum_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Shasum),
            openssl_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Openssl),
            python3_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Python3),
            nm_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Nm),
            swiftc_success: scenario_tool_usable(scenario, CompatibilityToolRequirement::Swiftc),
        };
        Self::new(profile)
    }

    fn new(profile: HarnessProfile) -> Self {
        let root = unique_temp_dir("doctor-harness");
        let sdk_dir = root.join("sdk");
        fs::create_dir_all(&sdk_dir).expect("create fake sdk");

        let xcrun_script = format!(
            "#!/bin/sh\ncase \"$1\" in\n  --version)\n    if [ \"{xcrun_success}\" = \"1\" ]; then exit 0; fi\n    exit 1 ;;\n  --find)\n    if [ \"{xcrun_success}\" != \"1\" ]; then exit 1; fi\n    case \"$2\" in\n      clang)\n        if [ \"{clang_success}\" = \"1\" ]; then echo \"{root}/clang\"; exit 0; fi\n        exit 1 ;;\n      strip)\n        if [ \"{strip_success}\" = \"1\" ]; then echo \"{root}/strip\"; exit 0; fi\n        exit 1 ;;\n      swiftc)\n        if [ \"{swiftc_success}\" = \"1\" ]; then echo \"{root}/swiftc\"; exit 0; fi\n        exit 1 ;;\n      *) exit 1 ;;\n    esac ;;\n  --show-sdk-path)\n    if [ \"{xcrun_success}\" = \"1\" ] && [ \"{sdk_probe_success}\" = \"1\" ]; then\n      echo \"{sdk}\"; exit 0\n    fi\n    exit 1 ;;\n  *) exit 1 ;;\nesac\n",
            root = root.display(),
            sdk = sdk_dir.display(),
            xcrun_success = if profile.xcrun_success { "1" } else { "0" },
            clang_success = if profile.clang_success { "1" } else { "0" },
            strip_success = if profile.strip_success { "1" } else { "0" },
            swiftc_success = if profile.swiftc_success { "1" } else { "0" },
            sdk_probe_success = if profile.sdk_probe_success { "1" } else { "0" },
        );
        write_exec_script(&root.join("xcrun"), &xcrun_script);
        write_exec_script(
            &root.join("clang"),
            if profile.clang_success {
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 0; fi\nexit 1\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );
        write_exec_script(
            &root.join("strip"),
            if profile.strip_success {
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 0; fi\nexit 1\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );
        write_exec_script(
            &root.join("python3"),
            if profile.python3_success {
                "#!/bin/sh\nexit 0\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );
        write_exec_script(
            &root.join("nm"),
            if profile.nm_success {
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 0; fi\nexit 1\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );
        write_exec_script(
            &root.join("swiftc"),
            if profile.swiftc_success {
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 0; fi\nexit 1\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );
        write_exec_script(
            &root.join("sha256sum"),
            if profile.sha256sum_success {
                "#!/bin/sh\nexit 0\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );
        write_exec_script(
            &root.join("shasum"),
            if profile.shasum_success {
                "#!/bin/sh\nexit 0\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );
        write_exec_script(
            &root.join("openssl"),
            if profile.openssl_success {
                "#!/bin/sh\nexit 0\n"
            } else {
                "#!/bin/sh\nexit 1\n"
            },
        );

        Self { root }
    }

    fn path(&self) -> &str {
        self.root.to_str().expect("utf8 harness path")
    }
}

#[cfg(unix)]
impl Drop for FakeDoctorHarness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
struct HarnessProfile {
    xcrun_success: bool,
    clang_success: bool,
    strip_success: bool,
    sdk_probe_success: bool,
    sha256sum_success: bool,
    shasum_success: bool,
    openssl_success: bool,
    python3_success: bool,
    nm_success: bool,
    swiftc_success: bool,
}

#[cfg(unix)]
fn doctor_harness_env<'a>(
    harness: &'a FakeDoctorHarness,
    scenario: &'a str,
    target_triple: &'a str,
) -> [(&'a str, &'a str); 3] {
    [
        ("PATH", harness.path()),
        ("DAMSEL_DOCTOR_TEST_SCENARIO", scenario),
        ("DAMSEL_DOCTOR_TEST_TARGET_TRIPLE", target_triple),
    ]
}

#[cfg(unix)]
fn scenario_tool_usable(
    scenario: &CompatibilityVerificationScenario,
    requirement: CompatibilityToolRequirement,
) -> bool {
    scenario
        .tools_usable
        .iter()
        .find_map(|(tool, usable)| (*tool == requirement).then_some(*usable))
        .unwrap_or(false)
}

#[cfg(unix)]
fn scenario_selected_hash_tool(
    scenario: &CompatibilityVerificationScenario,
) -> Option<&'static str> {
    for requirement in [
        CompatibilityToolRequirement::Sha256sum,
        CompatibilityToolRequirement::Shasum,
        CompatibilityToolRequirement::Openssl,
    ] {
        if scenario_tool_usable(scenario, requirement) {
            return Some(requirement.key());
        }
    }
    None
}

#[cfg(unix)]
fn doctor_test_scenario_for_verification_case(
    scenario: &CompatibilityVerificationScenario,
) -> String {
    match (scenario.host_platform, scenario.host_architecture) {
        ("linux", "arm64" | "aarch64") => "linux-arm64".to_string(),
        ("linux", "x86_64") => "linux-x86_64".to_string(),
        ("macos", "arm64") => "macos-arm64".to_string(),
        ("macos", "x86_64") => "macos-x86_64".to_string(),
        ("windows", "x86_64") => "windows-x86_64".to_string(),
        (platform, architecture) => format!("unknown:{platform}:{architecture}"),
    }
}

#[test]
fn doctor_json_contract_exposes_host_capabilities_and_tools() {
    let out = run_json_ok(&["--format", "json", "doctor"]);
    assert_raw_key_order(&out, &["schema_version", "command", "data"]);
    assert_raw_object_key_order(
        &out,
        "data",
        &["host", "overall_status", "capabilities", "tools", "issues"],
    );
    assert_raw_object_key_order(
        &out,
        "capabilities",
        &[
            "macho_analysis",
            "fixture_rebuild",
            "fixture_drift_check",
            "bench_compile",
            "bench_runtime",
            "benchmark",
        ],
    );
    assert_raw_object_key_order(
        &out,
        "tools",
        &[
            "xcrun",
            "strip",
            "clang",
            "python3",
            "nm",
            "swiftc",
            "sdk_path_probe",
            "hash_tools",
            "selected_hash_tool",
        ],
    );
    let json = parse_json(&out);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "doctor");
    assert_exact_object_keys(&json, &["schema_version", "command", "data"]);
    assert_exact_object_keys(
        &json["data"],
        &["host", "overall_status", "capabilities", "tools", "issues"],
    );
    assert_exact_object_keys(
        &json["data"]["host"],
        &["os", "architecture", "target_triple"],
    );
    assert!(json["data"]["host"]["os"].is_string());
    assert!(json["data"]["host"]["architecture"].is_string());
    assert!(
        json["data"]["host"]["target_triple"].is_string()
            || json["data"]["host"]["target_triple"].is_null()
    );
    assert_exact_object_keys(
        &json["data"]["capabilities"],
        &[
            "macho_analysis",
            "fixture_rebuild",
            "fixture_drift_check",
            "bench_compile",
            "bench_runtime",
            "benchmark",
        ],
    );
    for key in [
        "macho_analysis",
        "fixture_rebuild",
        "fixture_drift_check",
        "bench_compile",
        "bench_runtime",
        "benchmark",
    ] {
        assert_exact_object_keys(&json["data"]["capabilities"][key], &["status", "reasons"]);
        let capability_status = json["data"]["capabilities"][key]["status"]
            .as_str()
            .expect("capability status string");
        assert!(matches!(
            capability_status,
            "supported" | "supported-with-degraded-features" | "unsupported"
        ));
        let reasons = json["data"]["capabilities"][key]["reasons"]
            .as_array()
            .expect("reasons array");
        for reason in reasons {
            assert_exact_object_keys(reason, &["code", "message"]);
            assert!(reason["code"].is_string());
            assert!(reason["message"].is_string());
        }
    }
    assert_exact_object_keys(
        &json["data"]["tools"],
        &[
            "xcrun",
            "strip",
            "clang",
            "python3",
            "nm",
            "swiftc",
            "sdk_path_probe",
            "hash_tools",
            "selected_hash_tool",
        ],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["xcrun"],
        &["detected", "usable", "path"],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["strip"],
        &["detected", "usable", "path"],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["clang"],
        &["detected", "usable", "path"],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["python3"],
        &["detected", "usable", "path"],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["nm"],
        &["detected", "usable", "path"],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["swiftc"],
        &["detected", "usable", "path"],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["sdk_path_probe"],
        &["detected", "usable", "path"],
    );
    assert_exact_object_keys(
        &json["data"]["tools"]["hash_tools"],
        &["sha256sum", "shasum", "openssl"],
    );
    for key in ["sha256sum", "shasum", "openssl"] {
        assert_exact_object_keys(
            &json["data"]["tools"]["hash_tools"][key],
            &["detected", "usable", "path"],
        );
    }
    assert!(json["data"]["tools"]["xcrun"]["detected"].is_boolean());
    assert!(json["data"]["tools"]["xcrun"]["usable"].is_boolean());
    assert!(
        json["data"]["tools"]["xcrun"]["path"].is_string()
            || json["data"]["tools"]["xcrun"]["path"].is_null()
    );
    assert!(json["data"]["tools"]["strip"]["detected"].is_boolean());
    assert!(json["data"]["tools"]["strip"]["usable"].is_boolean());
    assert!(
        json["data"]["tools"]["strip"]["path"].is_string()
            || json["data"]["tools"]["strip"]["path"].is_null()
    );
    assert!(json["data"]["tools"]["clang"]["detected"].is_boolean());
    assert!(json["data"]["tools"]["clang"]["usable"].is_boolean());
    assert!(
        json["data"]["tools"]["clang"]["path"].is_string()
            || json["data"]["tools"]["clang"]["path"].is_null()
    );
    assert!(json["data"]["tools"]["python3"]["detected"].is_boolean());
    assert!(json["data"]["tools"]["python3"]["usable"].is_boolean());
    assert!(
        json["data"]["tools"]["python3"]["path"].is_string()
            || json["data"]["tools"]["python3"]["path"].is_null()
    );
    assert!(json["data"]["tools"]["nm"]["detected"].is_boolean());
    assert!(json["data"]["tools"]["nm"]["usable"].is_boolean());
    assert!(
        json["data"]["tools"]["nm"]["path"].is_string()
            || json["data"]["tools"]["nm"]["path"].is_null()
    );
    assert!(json["data"]["tools"]["swiftc"]["detected"].is_boolean());
    assert!(json["data"]["tools"]["swiftc"]["usable"].is_boolean());
    assert!(
        json["data"]["tools"]["swiftc"]["path"].is_string()
            || json["data"]["tools"]["swiftc"]["path"].is_null()
    );
    assert!(json["data"]["tools"]["sdk_path_probe"]["detected"].is_boolean());
    assert!(json["data"]["tools"]["sdk_path_probe"]["usable"].is_boolean());
    assert!(
        json["data"]["tools"]["sdk_path_probe"]["path"].is_string()
            || json["data"]["tools"]["sdk_path_probe"]["path"].is_null()
    );
    let selected_hash_tool = json["data"]["tools"]["selected_hash_tool"].as_str();
    let has_sha256sum = json["data"]["tools"]["hash_tools"]["sha256sum"]["detected"] == true;
    let has_shasum = json["data"]["tools"]["hash_tools"]["shasum"]["detected"] == true;
    let has_openssl = json["data"]["tools"]["hash_tools"]["openssl"]["detected"] == true;
    let has_usable_sha256sum = json["data"]["tools"]["hash_tools"]["sha256sum"]["usable"] == true;
    let has_usable_shasum = json["data"]["tools"]["hash_tools"]["shasum"]["usable"] == true;
    let has_usable_openssl = json["data"]["tools"]["hash_tools"]["openssl"]["usable"] == true;
    if let Some(selected) = selected_hash_tool {
        assert!(matches!(selected, "sha256sum" | "shasum" | "openssl"));
        assert!(json["data"]["tools"]["hash_tools"][selected]["usable"] == true);
    } else {
        assert!(
            !has_usable_sha256sum && !has_usable_shasum && !has_usable_openssl,
            "selected_hash_tool should be present when a usable hash tool is detected"
        );
    }
    if !has_sha256sum && !has_shasum && !has_openssl {
        assert!(selected_hash_tool.is_none());
    }
    let status = json["data"]["overall_status"]
        .as_str()
        .expect("overall_status string");
    assert!(matches!(
        status,
        "supported" | "supported-with-degraded-features" | "unsupported"
    ));
    let issues = json["data"]["issues"].as_array().expect("issues array");
    for issue in issues {
        assert_exact_object_keys(issue, &["code", "message"]);
        assert!(issue["code"].is_string());
        assert!(issue["message"].is_string());
    }
    let bench_compile_status = json["data"]["capabilities"]["bench_compile"]["status"]
        .as_str()
        .expect("bench_compile status");
    let bench_runtime_status = json["data"]["capabilities"]["bench_runtime"]["status"]
        .as_str()
        .expect("bench_runtime status");
    let benchmark_status = json["data"]["capabilities"]["benchmark"]["status"]
        .as_str()
        .expect("benchmark status");
    assert_eq!(
        benchmark_status,
        max_status([bench_compile_status, bench_runtime_status]),
        "benchmark should summarize split bench capabilities"
    );
    let overall_expected = max_status([
        json["data"]["capabilities"]["macho_analysis"]["status"]
            .as_str()
            .expect("macho status"),
        json["data"]["capabilities"]["fixture_rebuild"]["status"]
            .as_str()
            .expect("fixture rebuild status"),
        json["data"]["capabilities"]["fixture_drift_check"]["status"]
            .as_str()
            .expect("fixture drift status"),
        bench_compile_status,
        bench_runtime_status,
    ]);
    assert_eq!(
        status, overall_expected,
        "overall_status should summarize the non-summary capability set"
    );

    let benchmark_reasons = json["data"]["capabilities"]["benchmark"]["reasons"]
        .as_array()
        .expect("benchmark reasons");
    let mut expected_benchmark_reasons = Vec::new();
    for key in ["bench_compile", "bench_runtime"] {
        for reason in json["data"]["capabilities"][key]["reasons"]
            .as_array()
            .expect("summary input reasons")
        {
            if !expected_benchmark_reasons.contains(reason) {
                expected_benchmark_reasons.push(reason.clone());
            }
        }
    }
    assert_eq!(
        benchmark_reasons, &expected_benchmark_reasons,
        "benchmark reasons should be the deduped union of split bench reasons"
    );

    let mut expected_issue_union = Vec::new();
    for key in [
        "macho_analysis",
        "fixture_rebuild",
        "fixture_drift_check",
        "bench_compile",
        "bench_runtime",
    ] {
        for reason in json["data"]["capabilities"][key]["reasons"]
            .as_array()
            .expect("capability reasons")
        {
            if !expected_issue_union.contains(reason) {
                expected_issue_union.push(reason.clone());
            }
        }
    }
    assert_eq!(
        issues, &expected_issue_union,
        "issues should be the deduped union of non-summary capability reasons"
    );
    for key in [
        "macho_analysis",
        "fixture_rebuild",
        "fixture_drift_check",
        "bench_compile",
        "bench_runtime",
        "benchmark",
    ] {
        let capability = &json["data"]["capabilities"][key];
        if capability["status"] != "supported" {
            assert!(
                capability["reasons"]
                    .as_array()
                    .expect("reasons array")
                    .len()
                    > 0,
                "{key} should include reasons when status is not supported"
            );
        }
    }
}

#[test]
fn doctor_json_semantic_invariants_hold() {
    let out = run_json_ok(&["--format", "json", "doctor"]);
    let json = parse_json(&out);
    let capabilities = &json["data"]["capabilities"];
    let bench_compile = capabilities["bench_compile"]["status"]
        .as_str()
        .expect("bench_compile status");
    let bench_runtime = capabilities["bench_runtime"]["status"]
        .as_str()
        .expect("bench_runtime status");
    let benchmark = capabilities["benchmark"]["status"]
        .as_str()
        .expect("benchmark status");
    let split_max = if status_severity(bench_compile) >= status_severity(bench_runtime) {
        bench_compile
    } else {
        bench_runtime
    };
    assert_eq!(benchmark, split_max);

    let non_summary_statuses = [
        capabilities["macho_analysis"]["status"]
            .as_str()
            .expect("macho_analysis status"),
        capabilities["fixture_rebuild"]["status"]
            .as_str()
            .expect("fixture_rebuild status"),
        capabilities["fixture_drift_check"]["status"]
            .as_str()
            .expect("fixture_drift_check status"),
        bench_compile,
        bench_runtime,
    ];
    let expected_overall = non_summary_statuses
        .iter()
        .copied()
        .max_by_key(|status| status_severity(status))
        .expect("non-summary statuses");
    let overall = json["data"]["overall_status"]
        .as_str()
        .expect("overall_status string");
    assert_eq!(overall, expected_overall);

    let expected_issues = issue_set(&Value::Array(
        [
            "macho_analysis",
            "fixture_rebuild",
            "fixture_drift_check",
            "bench_compile",
            "bench_runtime",
        ]
        .iter()
        .flat_map(|key| {
            capabilities[*key]["reasons"]
                .as_array()
                .expect("reasons array")
                .iter()
                .cloned()
        })
        .collect(),
    ));
    assert_eq!(issue_set(&json["data"]["issues"]), expected_issues);

    let tools = &json["data"]["tools"]["hash_tools"];
    let selected_hash_tool = json["data"]["tools"]["selected_hash_tool"].as_str();
    let usable_sha256 = tools["sha256sum"]["usable"] == true;
    let usable_shasum = tools["shasum"]["usable"] == true;
    let usable_openssl = tools["openssl"]["usable"] == true;
    let expected_selected = if usable_sha256 {
        Some("sha256sum")
    } else if usable_shasum {
        Some("shasum")
    } else if usable_openssl {
        Some("openssl")
    } else {
        None
    };
    assert_eq!(selected_hash_tool, expected_selected);
}

#[cfg(unix)]
#[test]
fn doctor_json_harness_macos_usable_tools_has_deterministic_statuses() {
    let harness = FakeDoctorHarness::all_usable();
    let envs = doctor_harness_env(&harness, "macos-arm64", "aarch64-apple-darwin");
    let out = run_json_ok_with_env(&["--format", "json", "doctor"], &envs);
    let json = parse_json(&out);
    assert_eq!(json["command"], "doctor");
    assert_eq!(json["data"]["host"]["os"], "macos");
    assert_eq!(json["data"]["host"]["architecture"], "arm64");
    assert_eq!(
        json["data"]["host"]["target_triple"],
        "aarch64-apple-darwin"
    );
    assert_eq!(
        json["data"]["capabilities"]["macho_analysis"]["status"],
        "supported"
    );
    assert_eq!(
        json["data"]["capabilities"]["fixture_rebuild"]["status"],
        "supported"
    );
    assert_eq!(
        json["data"]["capabilities"]["fixture_drift_check"]["status"],
        "supported"
    );
    assert_eq!(
        json["data"]["capabilities"]["bench_runtime"]["status"],
        "supported-with-degraded-features"
    );
    assert_eq!(
        json["data"]["capabilities"]["benchmark"]["status"],
        "supported-with-degraded-features"
    );
    assert_eq!(
        json["data"]["overall_status"],
        "supported-with-degraded-features"
    );
    assert_eq!(json["data"]["tools"]["selected_hash_tool"], "sha256sum");
    assert_eq!(
        json["data"]["tools"]["xcrun"]["path"],
        format!("{}/xcrun", harness.path())
    );
    assert_eq!(
        json["data"]["tools"]["clang"]["path"],
        format!("{}/clang", harness.path())
    );
    assert_eq!(
        json["data"]["tools"]["strip"]["path"],
        format!("{}/strip", harness.path())
    );
    assert_eq!(
        json["data"]["tools"]["swiftc"]["path"],
        format!("{}/swiftc", harness.path())
    );
    assert_eq!(
        reason_code_set(&json["data"]["issues"]),
        BTreeSet::from([String::from("throughput_smoke_linux_arm64_only")])
    );
    assert!(
        json["data"]["capabilities"]["bench_runtime"]["reasons"]
            .as_array()
            .expect("bench_runtime reasons")
            .iter()
            .any(|reason| reason["code"] == "throughput_smoke_linux_arm64_only")
    );
}

#[cfg(unix)]
#[test]
fn doctor_json_harness_broken_hash_and_sdk_reports_detected_unusable_tools() {
    let harness = FakeDoctorHarness::broken_hash_and_sdk();
    let envs = doctor_harness_env(&harness, "macos-arm64", "aarch64-apple-darwin");
    let out = run_json_ok_with_env(&["--format", "json", "doctor"], &envs);
    let json = parse_json(&out);
    assert_eq!(json["command"], "doctor");
    assert_eq!(json["data"]["overall_status"], "unsupported");
    assert_eq!(
        json["data"]["capabilities"]["fixture_rebuild"]["status"],
        "unsupported"
    );
    assert_eq!(
        json["data"]["capabilities"]["fixture_drift_check"]["status"],
        "unsupported"
    );
    assert!(json["data"]["tools"]["sdk_path_probe"]["detected"] == true);
    assert!(json["data"]["tools"]["sdk_path_probe"]["usable"] == false);
    assert!(json["data"]["tools"]["hash_tools"]["sha256sum"]["detected"] == true);
    assert!(json["data"]["tools"]["hash_tools"]["sha256sum"]["usable"] == false);
    assert!(json["data"]["tools"]["hash_tools"]["shasum"]["detected"] == true);
    assert!(json["data"]["tools"]["hash_tools"]["shasum"]["usable"] == false);
    assert!(json["data"]["tools"]["hash_tools"]["openssl"]["detected"] == true);
    assert!(json["data"]["tools"]["hash_tools"]["openssl"]["usable"] == false);
    assert!(json["data"]["tools"]["selected_hash_tool"].is_null());

    let fixture_rebuild_codes =
        reason_code_set(&json["data"]["capabilities"]["fixture_rebuild"]["reasons"]);
    assert!(fixture_rebuild_codes.contains("xcrun_sdk_path_probe_failed"));
    let fixture_drift_codes =
        reason_code_set(&json["data"]["capabilities"]["fixture_drift_check"]["reasons"]);
    assert!(fixture_drift_codes.contains("missing_usable_hash_tool"));
}

#[cfg(unix)]
#[test]
fn doctor_check_fixture_rebuild_uses_controlled_harness_and_default_strict_threshold() {
    let harness = FakeDoctorHarness::python_unusable();
    let envs = doctor_harness_env(&harness, "macos-arm64", "aarch64-apple-darwin");
    let strict_output = run_json_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "fixture-rebuild",
            "--require-status",
            "supported",
        ],
        &envs,
    );
    assert_eq!(strict_output.status.code(), Some(2));
    assert!(
        strict_output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        strict_output.stderr
    );
    let strict_stdout = String::from_utf8_lossy(&strict_output.stdout)
        .trim()
        .to_string();
    let strict_json = parse_json(&strict_stdout);
    assert_eq!(strict_json["schema_version"], 1);
    assert_eq!(strict_json["command"], "doctor");
    assert_eq!(
        strict_json["data"]["capabilities"]["fixture_rebuild"]["status"],
        "unsupported"
    );
    assert!(
        strict_json["data"]["capabilities"]["fixture_rebuild"]["reasons"]
            .as_array()
            .expect("fixture_rebuild reasons")
            .iter()
            .any(|reason| reason["code"] == "unusable_python3")
    );

    let default_output = run_json_output_with_env(
        &["--format", "json", "doctor", "--check", "fixture-rebuild"],
        &envs,
    );
    assert_eq!(
        default_output.status.code(),
        Some(2),
        "default check threshold should be strict supported"
    );
    assert!(default_output.stderr.is_empty());
}

#[cfg(unix)]
#[test]
fn doctor_json_verification_corpus_is_enforced_under_controlled_harnesses() {
    for scenario in CompatibilityPolicy::verification_corpus() {
        let harness = FakeDoctorHarness::from_verification_scenario(scenario);
        let doctor_scenario = doctor_test_scenario_for_verification_case(scenario);
        let envs = doctor_harness_env(&harness, &doctor_scenario, "oracle-target");
        let out = run_json_ok_with_env(&["--format", "json", "doctor"], &envs);
        let json = parse_json(&out);

        assert_eq!(json["data"]["host"]["os"], scenario.host_platform);
        assert_eq!(
            json["data"]["host"]["architecture"],
            scenario.host_architecture
        );
        assert_eq!(json["data"]["host"]["target_triple"], "oracle-target");
        assert_eq!(
            json["data"]["overall_status"],
            scenario.expected_overall_status.to_string()
        );
        assert_eq!(
            json["data"]["tools"]["selected_hash_tool"].as_str(),
            scenario_selected_hash_tool(scenario)
        );

        for expectation in scenario.expected_capabilities {
            let capability = &json["data"]["capabilities"][expectation.capability.key()];
            assert_eq!(capability["status"], expectation.status.to_string());
            let actual_reason_codes = capability["reasons"]
                .as_array()
                .expect("capability reasons")
                .iter()
                .map(|reason| reason["code"].as_str().expect("reason code"))
                .collect::<Vec<_>>();
            assert_eq!(actual_reason_codes, expectation.reason_codes);
        }

        let actual_issue_codes = json["data"]["issues"]
            .as_array()
            .expect("issues array")
            .iter()
            .map(|issue| issue["code"].as_str().expect("issue code"))
            .collect::<Vec<_>>();
        assert_eq!(actual_issue_codes, scenario.expected_issue_codes);
    }
}

#[cfg(unix)]
#[test]
fn doctor_check_strict_threshold_follows_verification_corpus() {
    for scenario in CompatibilityPolicy::verification_corpus() {
        let harness = FakeDoctorHarness::from_verification_scenario(scenario);
        let doctor_scenario = doctor_test_scenario_for_verification_case(scenario);
        let envs = doctor_harness_env(&harness, &doctor_scenario, "oracle-target");
        let output = run_json_output_with_env(
            &[
                "--format",
                "json",
                "doctor",
                "--check",
                "all",
                "--require-status",
                "supported",
            ],
            &envs,
        );
        let strict_success = CompatibilityCapability::DOCTOR_CHECK_ALL
            .iter()
            .all(|capability| {
                scenario
                    .capability_expectation(*capability)
                    .is_some_and(|expectation| expectation.status.to_string() == "supported")
            });
        assert_eq!(
            output.status.code(),
            Some(if strict_success { 0 } else { 2 })
        );
        assert!(
            output.stderr.is_empty(),
            "unexpected stderr for scenario {}: {:?}",
            scenario.id,
            output.stderr
        );
        let json = parse_json(&String::from_utf8_lossy(&output.stdout));
        assert_eq!(json["command"], "doctor");
    }
}

#[test]
fn doctor_check_can_succeed_with_degraded_threshold() {
    let output = run_json_output(&[
        "--format",
        "json",
        "doctor",
        "--check",
        "macho-analysis",
        "--require-status",
        "supported-with-degraded-features",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let json = parse_json(&stdout);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "doctor");
}

#[test]
fn error_json_envelope_for_unsupported_input() {
    let path = write_temp_input(b"not a macho file");
    let path_string = path.to_string_lossy().to_string();
    let err = run_json_err(&["--format", "json", "info", path_string.as_str()]);
    let _ = fs::remove_file(path);
    let json = parse_json(&err);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "error");
    assert_eq!(json["data"]["code"], "unsupported_input");
    assert!(json["data"]["message"].is_string());
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
    let exports = json["data"]["exports"].as_array().expect("exports array");
    if let Some(export) = exports.first() {
        assert_exact_object_keys(
            export,
            &[
                "name",
                "address",
                "raw_flags",
                "flags",
                "kind",
                "reexport_target",
                "resolver_target",
            ],
        );
        assert_exact_object_keys(
            &export["flags"],
            &[
                "raw_bits",
                "kind_bits",
                "is_weak_definition",
                "is_reexport",
                "is_stub_and_resolver",
                "is_thread_local",
                "is_absolute",
                "unknown_bits",
                "flag_names",
            ],
        );
    }
    assert_exact_object_keys(
        &json["data"],
        &[
            "has_rebases",
            "has_binds",
            "has_chained_fixups",
            "included_sections",
            "name_filter",
            "dylib_filter",
            "source_filter",
            "binding_kind_filter",
            "stub_kind_filter",
            "export_kind_filter",
            "export_flag_filter",
            "ordinal_filter",
            "dylibs",
            "rpaths",
            "exports",
            "function_starts",
            "import_bindings",
            "stubs",
            "stub_helpers",
        ],
    );
    let stubs = json["data"]["stubs"].as_array().expect("stub array");
    assert!(!stubs.is_empty(), "{out}");
    assert_eq!(stubs[0]["section"], "__TEXT:__stubs");
    assert!(stubs[0]["stub_kind"].is_string());
    assert_exact_object_keys(
        &stubs[0],
        &[
            "stub_address",
            "section",
            "pointer_address",
            "pointer_section",
            "helper_address",
            "binding_ordinal",
            "stub_kind",
            "dylib",
            "name",
            "source",
        ],
    );
    let bindings = json["data"]["import_bindings"]
        .as_array()
        .expect("binding array");
    assert!(!bindings.is_empty(), "{out}");
    assert_exact_object_keys(
        &bindings[0],
        &[
            "dylib",
            "name",
            "address",
            "offset",
            "addend",
            "source",
            "ordinal",
            "symbol_index",
            "binding_kind",
            "is_weak",
        ],
    );
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
fn dyld_json_filters_by_kind_and_ordinal() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
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
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["binding_kind_filter"], "Lazy");
    assert_eq!(json["data"]["stub_kind_filter"], "Lazy");
    assert_eq!(json["data"]["ordinal_filter"], 1);
    let bindings = json["data"]["import_bindings"]
        .as_array()
        .expect("binding array");
    assert!(!bindings.is_empty(), "{out}");
    assert!(
        bindings
            .iter()
            .all(|binding| binding["binding_kind"] == "Lazy" && binding["ordinal"] == 1)
    );
    let stubs = json["data"]["stubs"].as_array().expect("stub array");
    assert!(!stubs.is_empty(), "{out}");
    assert!(
        stubs
            .iter()
            .all(|stub| stub["stub_kind"] == "Lazy" && stub["binding_ordinal"] == 1)
    );
    let helpers = json["data"]["stub_helpers"]
        .as_array()
        .expect("helper array");
    assert!(!helpers.is_empty(), "{out}");
    assert!(helpers.iter().all(|helper| helper["binding_ordinal"] == 1));
}

#[test]
fn dyld_json_filters_exports_by_kind() {
    let path = fixture("export-kinds");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--exports",
        "--export-kind",
        "regular",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["export_kind_filter"], "Regular");
    let exports = json["data"]["exports"].as_array().expect("exports array");
    assert!(!exports.is_empty(), "{out}");
    assert!(
        exports
            .iter()
            .all(|export| export["kind"]["type"] == "regular")
    );
}

#[test]
fn dyld_json_filters_exports_by_flag() {
    let path = fixture("export-kinds");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--exports",
        "--export-flag",
        "absolute",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["export_flag_filter"], "Absolute");
    let exports = json["data"]["exports"].as_array().expect("exports array");
    assert!(!exports.is_empty(), "{out}");
    assert!(
        exports
            .iter()
            .all(|export| export["flags"]["is_absolute"] == true)
    );
    assert!(exports.iter().all(|export| {
        export["flags"]["flag_names"]
            .as_array()
            .is_some_and(|names| names.iter().any(|name| name == "Absolute"))
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
    assert!(helpers[0]["stub_section"].is_string() || helpers[0]["stub_section"].is_null());
    assert!(helpers[0]["pointer_section"].is_string() || helpers[0]["pointer_section"].is_null());
    assert!(helpers[0]["pointer_address"].is_number() || helpers[0]["pointer_address"].is_null());
    assert_exact_object_keys(
        &helpers[0],
        &[
            "helper_address",
            "target_stub",
            "binding_ordinal",
            "stub_section",
            "pointer_address",
            "pointer_section",
            "dylib",
            "name",
        ],
    );
}

#[test]
fn dyld_json_section_toggles_keep_key_stability() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "dyld");
    assert_eq!(
        json["data"]["included_sections"],
        Value::Array(vec![Value::String("import_bindings".to_string())])
    );
    assert!(
        json["data"]["import_bindings"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
    );
    assert_eq!(json["data"]["dylibs"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["rpaths"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["exports"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["function_starts"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["stubs"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["stub_helpers"], Value::Array(Vec::new()));
    assert_exact_object_keys(
        &json["data"],
        &[
            "has_rebases",
            "has_binds",
            "has_chained_fixups",
            "included_sections",
            "name_filter",
            "dylib_filter",
            "source_filter",
            "binding_kind_filter",
            "stub_kind_filter",
            "export_kind_filter",
            "export_flag_filter",
            "ordinal_filter",
            "dylibs",
            "rpaths",
            "exports",
            "function_starts",
            "import_bindings",
            "stubs",
            "stub_helpers",
        ],
    );
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
    assert_exact_object_keys(
        &json["data"],
        &[
            "requested_detail",
            "owner_filter",
            "name_source_filter",
            "selector_source_filter",
            "category_source_filter",
            "image_info_flags",
            "class_names",
            "selector_names",
            "method_names",
            "pointer_refs",
            "classes",
            "protocols",
            "categories",
            "methods",
            "properties",
            "ivars",
        ],
    );
    let classes = json["data"]["classes"].as_array().expect("class array");
    if let Some(class) = classes.first() {
        assert!(class["name_source"].is_string());
    }
    let categories = json["data"]["categories"]
        .as_array()
        .expect("category array");
    if let Some(category) = categories.first() {
        assert!(category["record_source"].is_string());
        assert!(
            category["property_list_pointer"].is_null()
                || category["property_list_pointer"].is_number()
        );
        assert!(
            category["protocol_list_pointer"].is_null()
                || category["protocol_list_pointer"].is_number()
        );
        assert!(category["properties_source"].is_string());
        assert!(category["protocols_source"].is_string());
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
fn objc_json_provenance_filters_are_applied() {
    let path = fixture("objc-sample");
    let baseline = parse_json(&run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
    ]));
    let class_source = baseline["data"]["classes"]
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry["name_source"].as_str())
        .unwrap_or("Unresolved");
    let selector_source = baseline["data"]["methods"]
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry["selector_source"].as_str())
        .unwrap_or("Unresolved");
    let class_source_arg = match class_source {
        "Runtime" => "runtime",
        "PointerTable" => "pointer-table",
        "LegacyPool" => "legacy-pool",
        "Unresolved" => "unresolved",
        value => panic!("unexpected name_source: {value}"),
    };
    let selector_source_arg = match selector_source {
        "Direct" => "direct",
        "Relative" => "relative",
        "LegacyPool" => "legacy-pool",
        "Unresolved" => "unresolved",
        value => panic!("unexpected selector_source: {value}"),
    };

    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
        "--name-source",
        class_source_arg,
        "--selector-source",
        selector_source_arg,
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["name_source_filter"], class_source);
    assert_eq!(json["data"]["selector_source_filter"], selector_source);
    let classes = json["data"]["classes"].as_array().expect("classes array");
    if !classes.is_empty() {
        assert!(
            classes
                .iter()
                .all(|entry| entry["name_source"] == class_source)
        );
    }
    let methods = json["data"]["methods"].as_array().expect("methods array");
    if !methods.is_empty() {
        assert!(
            methods
                .iter()
                .all(|entry| entry["selector_source"] == selector_source)
        );
    }
}

#[test]
fn objc_json_category_source_filter_is_applied() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
        "--category-source",
        "symbol-synthesis",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["category_source_filter"], "SymbolSynthesis");
    let categories = json["data"]["categories"]
        .as_array()
        .expect("categories array");
    assert!(!categories.is_empty(), "{out}");
    assert!(
        categories
            .iter()
            .all(|category| category["record_source"] == "SymbolSynthesis")
    );
    assert!(categories.iter().any(|category| {
        category["protocol_list_pointer"].is_number()
            || category["property_list_pointer"].is_number()
    }));
}

#[test]
fn objc_json_category_source_with_lists_filter_is_applied() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
        "--category-source",
        "symbol-synthesis-with-lists",
    ]);
    let json = parse_json(&out);
    assert_eq!(
        json["data"]["category_source_filter"],
        "SymbolSynthesisWithLists"
    );
    let categories = json["data"]["categories"]
        .as_array()
        .expect("categories array");
    assert!(!categories.is_empty(), "{out}");
    assert!(categories.iter().all(|category| {
        category["record_source"] == "SymbolSynthesis"
            && (category["protocol_list_pointer"].is_number()
                || category["property_list_pointer"].is_number())
    }));
}

#[test]
fn objc_json_detail_toggle_keeps_empty_sections_and_stable_keys() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["requested_detail"], "methods");
    assert!(
        json["data"]["methods"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
    );
    assert_eq!(json["data"]["classes"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["protocols"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["categories"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["properties"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["ivars"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["pointer_refs"], Value::Array(Vec::new()));
    assert_exact_object_keys(
        &json["data"],
        &[
            "requested_detail",
            "owner_filter",
            "name_source_filter",
            "selector_source_filter",
            "category_source_filter",
            "image_info_flags",
            "class_names",
            "selector_names",
            "method_names",
            "pointer_refs",
            "classes",
            "protocols",
            "categories",
            "methods",
            "properties",
            "ivars",
        ],
    );
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
    assert!(json["data"]["window_end"].is_null() || json["data"]["window_end"].is_number());
    assert!(json["data"]["decoded_bytes"].is_number());
    assert!(json["data"]["stop_reason"].is_string());
    assert!(json["data"]["instruction_count"].is_number());
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(!instructions.is_empty(), "{out}");
    assert!(instructions[0]["references"].is_array());
    assert!(instructions[0]["annotations"].is_array());
    assert!(instructions[0]["recovered_values"].is_array());
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations
                    .iter()
                    .any(|annotation| annotation["type"] == "jump_table_candidate")
            })
    }));
    for annotation in instructions
        .iter()
        .flat_map(|instruction| instruction["annotations"].as_array().into_iter().flatten())
        .filter(|annotation| annotation["type"] == "table_slot_resolved")
    {
        assert!(annotation["table_base"].is_number());
        assert!(annotation["slot_address"].is_number());
        assert!(annotation["index_register"].is_string());
        assert!(annotation["element_size"].is_number());
        assert!(annotation["encoding"].is_string());
        assert!(annotation["target"].is_number());
        assert!(annotation["encoding"] == "absolute64" || annotation["encoding"] == "relative32");
        assert_exact_object_keys(
            annotation,
            &[
                "type",
                "table_base",
                "slot_address",
                "index_register",
                "element_size",
                "encoding",
                "target",
            ],
        );
    }
    let helper_refs = instructions
        .iter()
        .flat_map(|instruction| instruction["references"].as_array().into_iter().flatten())
        .filter(|reference| reference["type"] == "stub_helper")
        .collect::<Vec<_>>();
    if let Some(reference) = helper_refs.first() {
        assert!(reference["helper_address"].is_number());
        assert!(reference["target_stub"].is_number() || reference["target_stub"].is_null());
        assert!(reference["pointer_address"].is_number() || reference["pointer_address"].is_null());
        assert!(reference["stub_section"].is_string() || reference["stub_section"].is_null());
        assert!(reference["pointer_section"].is_string() || reference["pointer_section"].is_null());
        assert_exact_object_keys(
            reference,
            &[
                "type",
                "helper_address",
                "target_stub",
                "stub_section",
                "pointer_address",
                "pointer_section",
                "binding_ordinal",
                "dylib",
                "name",
            ],
        );
    }
    assert_exact_object_keys(
        &json["data"],
        &[
            "target",
            "start_address",
            "end_address",
            "window_end",
            "bytes_len",
            "decoded_bytes",
            "instruction_count",
            "stop_reason",
            "instructions",
        ],
    );
    assert_exact_object_keys(
        &instructions[0],
        &[
            "address",
            "size",
            "opcode",
            "mnemonic",
            "operands",
            "rendered",
            "references",
            "annotations",
            "recovered_values",
        ],
    );
}

#[test]
fn disasm_json_analysis_contract_is_opt_in() {
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
        "--analysis",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "disasm");
    assert_exact_object_keys(
        &json["data"],
        &[
            "target",
            "start_address",
            "end_address",
            "window_end",
            "bytes_len",
            "decoded_bytes",
            "instruction_count",
            "stop_reason",
            "instructions",
            "analysis",
        ],
    );
    let analysis = &json["data"]["analysis"];
    assert_exact_object_keys(
        analysis,
        &[
            "target",
            "start_address",
            "end_address",
            "instruction_count",
            "summary",
            "basic_blocks",
            "edges",
            "direct_calls",
            "branch_targets",
            "data_references",
            "indirect_controls",
            "imports",
            "cache_links",
            "recovered_values",
            "jump_tables",
        ],
    );
    assert_exact_object_keys(
        &analysis["summary"],
        &[
            "basic_block_count",
            "edge_count",
            "direct_call_count",
            "indirect_call_count",
            "branch_count",
            "return_count",
            "data_reference_count",
            "import_count",
            "cache_link_count",
            "recovered_value_count",
            "jump_table_count",
            "unresolved_indirect_count",
        ],
    );
    assert!(analysis["summary"]["basic_block_count"].as_u64().unwrap() > 1);
    assert!(analysis["summary"]["edge_count"].as_u64().unwrap() > 0);
    assert!(
        analysis["summary"]["data_reference_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(analysis["basic_blocks"].as_array().is_some_and(|blocks| {
        blocks.iter().all(|block| {
            block["id"].is_number()
                && block["start_address"].is_number()
                && block["end_address"].is_number()
                && block["instruction_count"].is_number()
        })
    }));
    assert!(analysis["edges"].as_array().is_some_and(|edges| {
        edges
            .iter()
            .any(|edge| edge["kind"] == "conditional-branch" || edge["kind"] == "branch")
    }));
    assert!(
        analysis["jump_tables"]
            .as_array()
            .is_some_and(|tables| !tables.is_empty())
    );
}

#[test]
fn disasm_json_objc_method_target_resolves_implementation() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--objc-owner",
        "Greeter",
        "--objc-selector",
        "greeting",
        "--objc-method-kind",
        "instance",
        "--limit",
        "8",
        "--analysis",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "disasm");
    assert_eq!(json["data"]["target"], "objc:-[Greeter greeting]");
    assert_eq!(
        json["data"]["analysis"]["target"],
        "objc:-[Greeter greeting]"
    );
    assert!(json["data"]["start_address"].as_u64().unwrap() > 0);
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(!instructions.is_empty(), "{out}");
    assert!(
        instructions[0]["annotations"]
            .as_array()
            .expect("annotations")
            .iter()
            .any(|annotation| annotation["type"] == "symbol"
                && annotation["name"] == "-[Greeter greeting]")
    );
}

#[test]
fn disasm_json_swift_symbol_target_resolves_checked_in_fixture() {
    let path = fixture("swift-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--swift-symbol",
        "_$s17DamselSwiftSample9publicAddyS2iF",
        "--limit",
        "4",
        "--analysis",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "disasm");
    assert_eq!(
        json["data"]["target"],
        "swift:_$s17DamselSwiftSample9publicAddyS2iF"
    );
    assert_eq!(
        json["data"]["analysis"]["target"],
        "swift:_$s17DamselSwiftSample9publicAddyS2iF"
    );
    assert!(json["data"]["start_address"].as_u64().unwrap() > 0);
    assert_eq!(json["data"]["instruction_count"], 4);
    assert!(
        json["data"]["instructions"]
            .as_array()
            .expect("instructions")
            .iter()
            .flat_map(|instruction| instruction["annotations"].as_array().into_iter().flatten())
            .any(|annotation| annotation["type"] == "symbol"
                && annotation["name"] == "_$s17DamselSwiftSample9publicAddyS2iF")
    );
    assert_eq!(json["data"]["analysis"]["summary"]["basic_block_count"], 1);
}

#[test]
fn cache_disasm_json_analysis_links_imports_to_cache_providers() {
    let path = cache_fixture("internal-linkage-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "disasm",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libdispatch.dylib",
        "--section",
        "__text",
        "--show-references",
        "--show-values",
        "--analysis",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_disasm");
    let analysis = &json["data"]["disassembly"]["analysis"];
    assert!(analysis["summary"]["import_count"].as_u64().unwrap() > 0);
    assert!(analysis["summary"]["cache_link_count"].as_u64().unwrap() > 0);
    assert_eq!(
        analysis["summary"]["cache_link_count"].as_u64(),
        analysis["cache_links"]
            .as_array()
            .map(|links| links.len() as u64)
    );
    assert!(
        analysis["cache_links"]
            .as_array()
            .expect("cache links")
            .iter()
            .any(|link| {
                link["symbol_name"] == "_puts"
                    && link["dylib"] == "/usr/lib/libSystem.B.dylib"
                    && link["provider_install_name"] == "/usr/lib/libSystem.B.dylib"
                    && link["provider_kind"] == "export"
            })
    );
    for entry in analysis["cache_links"].as_array().expect("cache links") {
        assert_exact_object_keys(
            entry,
            &[
                "instruction_address",
                "symbol_name",
                "dylib",
                "provider_image_id",
                "provider_install_name",
                "provider_member_name",
                "provider_kind",
                "target_dylib",
                "target_symbol",
                "resolved_target_image_id",
                "resolved_target_install_name",
                "resolved_target_member_name",
            ],
        );
    }
}

#[test]
fn disasm_json_exposes_typed_indirect_target_reasons() {
    let path = fixture("indirect-dispatch");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_dispatch_second_slot",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations.iter().any(|annotation| {
                    annotation["type"] == "indirect_target_resolved"
                        && annotation["reason"] == "function-pointer"
                })
            })
    }));
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations.iter().any(|annotation| {
                    annotation["type"] == "table_slot_resolved"
                        && annotation["encoding"].is_string()
                        && annotation["target"].is_number()
                })
            })
    }));
}

#[test]
fn disasm_json_exposes_export_address_recovered_values() {
    let path = fixture("indirect-dispatch");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_load_export_target",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(instructions.iter().any(|instruction| {
        instruction["recovered_values"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value["kind"] == "ExportAddress"))
    }));
}

#[test]
fn disasm_json_exposes_table_load_provenance_sources() {
    let path = fixture("indirect-dispatch");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_dispatch_second_slot",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    let recovered_values = instructions
        .iter()
        .flat_map(|instruction| {
            instruction["recovered_values"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .collect::<Vec<_>>();
    assert!(!recovered_values.is_empty(), "{out}");
    assert!(
        recovered_values
            .iter()
            .any(|value| value["source"] == "TableLoad"),
        "{out}"
    );
    for value in recovered_values {
        assert_exact_object_keys(value, &["register", "value", "kind", "source"]);
        assert!(value["source"].is_string());
        if value["source"] == "RelativeTableLoad" {
            assert!(value["kind"].is_string());
        }
    }
}

#[test]
fn disasm_json_exposes_relative_table_load_metadata_when_present() {
    let path = fixture("relative-dispatch");
    if !path.exists() {
        eprintln!("relative-dispatch fixture not present; skipping");
        return;
    }
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_relative_dispatch_second_slot",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations.iter().any(|annotation| {
                    annotation["type"] == "table_slot_resolved"
                        && annotation["encoding"] == "relative32"
                })
            })
    }));
    assert!(instructions.iter().any(|instruction| {
        instruction["recovered_values"]
            .as_array()
            .is_some_and(|values| {
                values.iter().any(|value| {
                    value["source"] == "RelativeTableLoad" && value["kind"] == "FunctionPointer"
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

fn cache_first_image_id(cache_path: &Path) -> String {
    let raw = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "images",
        cache_path.to_str().expect("utf8 path"),
    ]);
    let json = parse_json(&raw);
    json["data"]["images"]
        .as_array()
        .expect("images array")
        .first()
        .expect("first image")
        .get("id")
        .and_then(Value::as_str)
        .expect("image id")
        .to_string()
}

#[test]
fn cache_info_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "info",
        path.to_str().expect("utf8 path"),
    ]);
    assert_raw_key_order(&out, &["schema_version", "command", "data"]);
    assert_raw_object_key_order(&out, "data", &["header", "members"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_info");
    assert_exact_object_keys(&json["data"], &["header", "members"]);
    assert_exact_object_keys(
        &json["data"]["header"],
        &[
            "cache_path",
            "cache_uuid",
            "architecture",
            "member_count",
            "image_count",
            "has_local_symbols",
        ],
    );
    assert!(json["data"]["header"]["cache_path"].is_string());
    assert!(json["data"]["header"]["cache_uuid"].is_string());
    assert!(json["data"]["header"]["architecture"].is_string());
    assert!(json["data"]["header"]["member_count"].is_number());
    assert!(json["data"]["header"]["image_count"].is_number());
    assert!(json["data"]["header"]["has_local_symbols"].is_boolean());
    let members = json["data"]["members"].as_array().expect("members array");
    assert!(!members.is_empty());
    for member in members {
        assert_exact_object_keys(member, &["name", "role", "path"]);
        assert!(member["name"].is_string());
        assert!(member["role"].is_string());
        assert!(member["path"].is_string());
    }
}

#[test]
fn cache_images_json_contract_and_order() {
    let path = cache_fixture("valid-single-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "images",
        path.to_str().expect("utf8 path"),
        "--limit",
        "1",
    ]);
    assert_raw_key_order(&out, &["schema_version", "command", "data"]);
    assert_raw_object_key_order(&out, "data", &["metadata", "images"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_images");
    assert_exact_object_keys(&json["data"], &["metadata", "images"]);
    assert_exact_object_keys(
        &json["data"]["metadata"],
        &["total", "returned", "truncated"],
    );
    assert!(json["data"]["metadata"]["total"].is_number());
    assert!(json["data"]["metadata"]["returned"].is_number());
    assert!(json["data"]["metadata"]["truncated"].is_boolean());
    let images = json["data"]["images"].as_array().expect("images array");
    assert_eq!(images.len(), 1);
    for image in images {
        assert_exact_object_keys(
            image,
            &[
                "id",
                "image_index",
                "install_name",
                "basename",
                "image_base_vmaddr",
                "member_name",
            ],
        );
    }
}

#[test]
fn cache_image_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let image_id = cache_first_image_id(&path);
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "image",
        path.to_str().expect("utf8 path"),
        image_id.as_str(),
    ]);
    assert_raw_object_key_order(&out, "data", &["image"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_image");
    assert_exact_object_keys(&json["data"], &["image"]);
    assert_exact_object_keys(
        &json["data"]["image"],
        &[
            "id",
            "image_index",
            "install_name",
            "basename",
            "image_base_vmaddr",
            "member_name",
        ],
    );
}

#[test]
fn cache_exports_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let image_id = cache_first_image_id(&path);
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "exports",
        path.to_str().expect("utf8 path"),
        image_id.as_str(),
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "metadata", "exports"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_exports");
    assert_exact_object_keys(&json["data"], &["image", "metadata", "exports"]);
    assert_exact_object_keys(
        &json["data"]["metadata"],
        &["total", "returned", "truncated"],
    );
    let exports = json["data"]["exports"].as_array().expect("exports array");
    for export in exports {
        assert_exact_object_keys(export, &["name", "cache_vmaddr", "kind", "flags"]);
        assert!(export["name"].is_string());
        assert!(export["kind"].is_string());
        assert!(export["flags"].is_string());
        assert!(export["cache_vmaddr"].is_number() || export["cache_vmaddr"].is_null());
    }
}

#[test]
fn cache_lookup_address_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let images_out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "images",
        path.to_str().expect("utf8 path"),
    ]);
    let images_json = parse_json(&images_out);
    let vmaddr = images_json["data"]["images"][0]["image_base_vmaddr"]
        .as_u64()
        .expect("image_base_vmaddr");
    let vmarg = format!("{vmaddr:#x}");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "lookup-address",
        path.to_str().expect("utf8 path"),
        vmarg.as_str(),
    ]);
    assert_raw_object_key_order(&out, "data", &["result"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_lookup_address");
    assert_exact_object_keys(&json["data"], &["result"]);
    assert_exact_object_keys(
        &json["data"]["result"],
        &[
            "kind",
            "cache_vmaddr",
            "member_name",
            "mapping_base_vmaddr",
            "mapping_size",
            "member_file_offset",
            "image_id",
            "install_name",
            "image_base_vmaddr",
            "image_offset",
            "symbol",
            "symbol_source",
            "symbol_address",
            "offset_from_symbol",
        ],
    );
    assert!(matches!(
        json["data"]["result"]["kind"].as_str(),
        Some("exact_symbol" | "nearest_symbol" | "mapping_only")
    ));
}

#[test]
fn cache_resolve_symbol_json_contract_and_metadata() {
    let path = cache_fixture("valid-single-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "resolve-symbol",
        path.to_str().expect("utf8 path"),
        "_main",
        "--limit",
        "1",
    ]);
    assert_raw_object_key_order(&out, "data", &["metadata", "matches"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_resolve_symbol");
    assert_exact_object_keys(&json["data"], &["metadata", "matches"]);
    assert_exact_object_keys(
        &json["data"]["metadata"],
        &["total", "returned", "truncated"],
    );
    let matches = json["data"]["matches"].as_array().expect("matches array");
    assert_eq!(matches.len(), 1);
    for entry in matches {
        assert_exact_object_keys(
            entry,
            &[
                "name",
                "image_id",
                "install_name",
                "member_name",
                "cache_vmaddr",
                "image_base_vmaddr",
                "image_offset",
                "member_file_offset",
                "source",
            ],
        );
        assert!(entry["name"].is_string());
        assert!(entry["image_id"].is_string());
        assert!(entry["install_name"].is_string());
        assert!(entry["cache_vmaddr"].is_number());
        assert!(entry["image_base_vmaddr"].is_number());
        assert!(entry["image_offset"].is_number());
        assert!(entry["member_file_offset"].is_number() || entry["member_file_offset"].is_null());
        assert!(entry["source"].is_string());
    }
}

#[test]
fn cache_image_not_found_json_error_envelope_is_typed() {
    let path = cache_fixture("valid-single-arm64.cache");
    let err = run_json_err(&[
        "--format",
        "json",
        "cache",
        "image",
        path.to_str().expect("utf8 path"),
        "missing-image",
    ]);
    let json = parse_json(&err);
    assert_eq!(json["command"], "error");
    assert_eq!(json["data"]["code"], "cache_image_not_found");
    assert!(json["data"]["message"].is_string());
    assert!(json["data"]["details"].is_null());
}

#[test]
fn cache_sections_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "sections",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libobjc.A.dylib",
        "--exec",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "sections"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_sections");
    assert_exact_object_keys(&json["data"], &["image", "sections"]);
    assert!(json["data"]["sections"].is_array());
}

#[test]
fn cache_symbols_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "symbols",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libobjc.A.dylib",
        "--global",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "symbols"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_symbols");
    assert_exact_object_keys(&json["data"], &["image", "symbols"]);
    assert!(json["data"]["symbols"].is_array());
}

#[test]
fn cache_imports_json_contract() {
    let path = cache_fixture("valid-split-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "imports",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libdispatch.dylib",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "imports"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_imports");
    assert_exact_object_keys(&json["data"], &["image", "imports"]);
    assert!(json["data"]["imports"].is_array());
}

#[test]
fn cache_dyld_json_contract() {
    let path = cache_fixture("valid-split-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "dyld",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libdispatch.dylib",
        "--exports",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "dyld"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_dyld");
    assert_exact_object_keys(&json["data"], &["image", "dyld"]);
    assert!(json["data"]["dyld"].is_object());
}

#[test]
fn cache_objc_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "objc",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libobjc.A.dylib",
        "--detail",
        "summary",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "objc"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_objc");
    assert_exact_object_keys(&json["data"], &["image", "objc"]);
    assert!(json["data"]["objc"].is_object());
}

#[test]
fn cache_disasm_json_contract() {
    let path = cache_fixture("valid-single-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "disasm",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libobjc.A.dylib",
        "--symbol",
        "_main",
        "--limit",
        "4",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "disassembly"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_disasm");
    assert_exact_object_keys(&json["data"], &["image", "disassembly"]);
    assert!(json["data"]["disassembly"].is_object());
}

#[test]
fn cache_image_deps_json_contract() {
    let path = cache_fixture("internal-linkage-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "image-deps",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libdispatch.dylib",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "metadata", "dependencies"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_image_deps");
    assert_exact_object_keys(&json["data"], &["image", "metadata", "dependencies"]);
    let dependencies = json["data"]["dependencies"]
        .as_array()
        .expect("dependencies array");
    assert!(!dependencies.is_empty());
    for entry in dependencies {
        assert_exact_object_keys(
            entry,
            &[
                "target_dylib_install_name",
                "target_image_id",
                "target_install_name",
                "target_member_name",
                "within_cache",
                "reference_count",
            ],
        );
    }
}

#[test]
fn cache_dependents_json_contract() {
    let path = cache_fixture("internal-linkage-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "dependents",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libSystem.B.dylib",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "metadata", "dependents"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_dependents");
    assert_exact_object_keys(&json["data"], &["image", "metadata", "dependents"]);
    let dependents = json["data"]["dependents"].as_array().expect("dependents");
    assert!(!dependents.is_empty());
    for entry in dependents {
        assert_exact_object_keys(
            entry,
            &[
                "image_id",
                "install_name",
                "member_name",
                "dependency_count",
            ],
        );
    }
}

#[test]
fn cache_symbol_providers_json_contract() {
    let path = cache_fixture("reexport-linkage-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "symbol-providers",
        path.to_str().expect("utf8 path"),
        "_exported_regular",
    ]);
    assert_raw_object_key_order(&out, "data", &["metadata", "providers"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_symbol_providers");
    assert_exact_object_keys(&json["data"], &["metadata", "providers"]);
    let providers = json["data"]["providers"].as_array().expect("providers");
    assert!(!providers.is_empty());
    for entry in providers {
        assert_exact_object_keys(
            entry,
            &[
                "image_id",
                "install_name",
                "member_name",
                "symbol_name",
                "provider_kind",
                "target_dylib",
                "target_symbol",
                "resolved_target_image_id",
                "resolved_target_install_name",
                "resolved_target_member_name",
            ],
        );
    }
}

#[test]
fn cache_symbol_importers_json_contract() {
    let path = cache_fixture("internal-linkage-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "symbol-importers",
        path.to_str().expect("utf8 path"),
        "_puts",
    ]);
    assert_raw_object_key_order(&out, "data", &["metadata", "importers"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_symbol_importers");
    assert_exact_object_keys(&json["data"], &["metadata", "importers"]);
    let importers = json["data"]["importers"].as_array().expect("importers");
    assert!(!importers.is_empty());
    for entry in importers {
        assert_exact_object_keys(
            entry,
            &[
                "image_id",
                "install_name",
                "member_name",
                "symbol_name",
                "dylib_name",
                "import_binding_kind",
                "import_binding_source",
                "resolved_provider_image_id",
                "resolved_provider_install_name",
                "resolved_provider_member_name",
            ],
        );
    }
}

#[test]
fn cache_reexports_json_contract() {
    let path = cache_fixture("reexport-linkage-arm64.cache");
    let out = run_json_ok(&[
        "--format",
        "json",
        "cache",
        "reexports",
        path.to_str().expect("utf8 path"),
        "/usr/lib/libreexporter.dylib",
    ]);
    assert_raw_object_key_order(&out, "data", &["image", "metadata", "reexports"]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "cache_reexports");
    assert_exact_object_keys(&json["data"], &["image", "metadata", "reexports"]);
    let reexports = json["data"]["reexports"].as_array().expect("reexports");
    assert!(!reexports.is_empty());
    for entry in reexports {
        assert_exact_object_keys(
            entry,
            &[
                "export_name",
                "target_dylib",
                "target_symbol",
                "resolved_target_image_id",
                "resolved_target_install_name",
                "resolved_target_member_name",
            ],
        );
    }
}
