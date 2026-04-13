use assert_cmd::Command;
use serde_json::Value;
use std::process::Output;

fn run_output(args: &[&str]) -> Output {
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs")
}

fn run_output_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::cargo_bin("damsel-cli").expect("binary exists");
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("command runs")
}

fn parse_json_output(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!(
            "invalid json: {error}\nstdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn doctor_check_windows_macho_analysis_requires_degraded_threshold() {
    let strict = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "macho-analysis",
            "--require-status",
            "supported",
        ],
        &[("DAMSEL_DOCTOR_TEST_SCENARIO", "windows-x86_64")],
    );
    assert_eq!(strict.status.code(), Some(2));
    assert!(
        strict.stderr.is_empty(),
        "unexpected stderr for threshold failure: {:?}",
        strict.stderr
    );
    let strict_json = parse_json_output(&strict);
    assert_eq!(
        strict_json["data"]["capabilities"]["macho_analysis"]["status"],
        "supported-with-degraded-features"
    );

    let degraded = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "macho-analysis",
            "--require-status",
            "supported-with-degraded-features",
        ],
        &[("DAMSEL_DOCTOR_TEST_SCENARIO", "windows-x86_64")],
    );
    assert_eq!(degraded.status.code(), Some(0));
    assert!(
        degraded.stderr.is_empty(),
        "unexpected stderr for degraded threshold pass: {:?}",
        degraded.stderr
    );
}

#[test]
fn doctor_check_linux_x86_runtime_requires_degraded_threshold() {
    let strict = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "bench-runtime",
            "--require-status",
            "supported",
        ],
        &[("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-x86_64")],
    );
    assert_eq!(strict.status.code(), Some(2));
    assert!(
        strict.stderr.is_empty(),
        "unexpected stderr for threshold failure: {:?}",
        strict.stderr
    );
    let strict_json = parse_json_output(&strict);
    assert_eq!(
        strict_json["data"]["capabilities"]["bench_runtime"]["status"],
        "supported-with-degraded-features"
    );
    assert!(
        strict_json["data"]["capabilities"]["bench_runtime"]["reasons"]
            .as_array()
            .expect("bench_runtime reasons")
            .iter()
            .any(|reason| reason["code"] == "throughput_smoke_linux_arm64_only")
    );

    let degraded = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "bench-runtime",
            "--require-status",
            "supported-with-degraded-features",
        ],
        &[("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-x86_64")],
    );
    assert_eq!(degraded.status.code(), Some(0));
    assert!(
        degraded.stderr.is_empty(),
        "unexpected stderr for degraded threshold pass: {:?}",
        degraded.stderr
    );
}

#[test]
fn doctor_check_all_on_non_macos_strict_fails_without_self_derived_baseline() {
    let output = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "all",
            "--require-status",
            "supported",
        ],
        &[("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-arm64")],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr for threshold failure: {:?}",
        output.stderr
    );
    let json = parse_json_output(&output);
    assert_eq!(
        json["data"]["capabilities"]["fixture_rebuild"]["status"],
        "unsupported"
    );
    assert!(
        json["data"]["capabilities"]["fixture_rebuild"]["reasons"]
            .as_array()
            .expect("fixture rebuild reasons")
            .iter()
            .any(|reason| reason["code"] == "fixture_rebuild_macos_only")
    );
}

#[test]
fn doctor_check_requires_check_flag_for_require_status() {
    let output = run_output(&[
        "--format",
        "json",
        "doctor",
        "--require-status",
        "supported",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("the following required arguments were not provided"),
        "unexpected stderr: {stderr}"
    );
}
