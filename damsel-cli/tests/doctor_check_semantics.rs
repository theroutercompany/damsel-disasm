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

fn parse_json_output(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!(
            "invalid json: {error}\nstdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn status_severity(status: &str) -> u8 {
    match status {
        "supported" => 0,
        "supported-with-degraded-features" => 1,
        "unsupported" => 2,
        other => panic!("unexpected status: {other}"),
    }
}

fn capability_severity(json: &Value, key: &str) -> u8 {
    let status = json["data"]["capabilities"][key]["status"]
        .as_str()
        .unwrap_or_else(|| panic!("missing capability status for key {key}"));
    status_severity(status)
}

#[test]
fn doctor_check_thresholds_match_reported_capability_statuses() {
    let baseline_output = run_output(&["--format", "json", "doctor"]);
    assert!(
        baseline_output.status.success(),
        "baseline doctor failed: stdout={:?} stderr={:?}",
        baseline_output.stdout,
        baseline_output.stderr
    );
    let baseline = parse_json_output(&baseline_output);
    assert_eq!(baseline["schema_version"], 1);
    assert_eq!(baseline["command"], "doctor");

    let all_severity = [
        capability_severity(&baseline, "macho_analysis"),
        capability_severity(&baseline, "fixture_rebuild"),
        capability_severity(&baseline, "fixture_drift_check"),
        capability_severity(&baseline, "bench_compile"),
        capability_severity(&baseline, "bench_runtime"),
    ]
    .into_iter()
    .max()
    .expect("at least one capability");

    let targets = [
        ("all", all_severity),
        (
            "macho-analysis",
            capability_severity(&baseline, "macho_analysis"),
        ),
        (
            "fixture-rebuild",
            capability_severity(&baseline, "fixture_rebuild"),
        ),
        (
            "fixture-drift-check",
            capability_severity(&baseline, "fixture_drift_check"),
        ),
        (
            "bench-compile",
            capability_severity(&baseline, "bench_compile"),
        ),
        (
            "bench-runtime",
            capability_severity(&baseline, "bench_runtime"),
        ),
    ];

    for (target, severity) in targets {
        for (required_label, required_severity) in [
            ("supported", 0_u8),
            ("supported-with-degraded-features", 1_u8),
        ] {
            let output = run_output(&[
                "--format",
                "json",
                "doctor",
                "--check",
                target,
                "--require-status",
                required_label,
            ]);
            let expected_code = if severity <= required_severity { 0 } else { 2 };
            assert_eq!(
                output.status.code(),
                Some(expected_code),
                "unexpected exit code for target={target}, require={required_label}"
            );
            assert!(
                output.stderr.is_empty(),
                "unexpected stderr for target={target}, require={required_label}: {:?}",
                output.stderr
            );

            let json = parse_json_output(&output);
            assert_eq!(json["schema_version"], 1);
            assert_eq!(json["command"], "doctor");
            assert!(json["data"]["host"].is_object());
            assert!(json["data"]["capabilities"].is_object());
            assert!(json["data"]["tools"].is_object());
            assert!(json["data"]["issues"].is_array());
        }
    }
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
