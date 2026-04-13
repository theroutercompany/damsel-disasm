use assert_cmd::Command;
use std::process::Output;

fn run_output(args: &[&str]) -> Output {
    Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn doctor_help_explains_all_semantics_and_exit_code_two() {
    let output = run_output(&["doctor", "--help"]);
    assert_eq!(output.status.code(), Some(0));

    let stdout = stdout_text(&output);
    assert!(
        stdout.contains("`--check all` validates the CI capability set"),
        "expected all-target explanation in help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("`fixture-rebuild` is macOS-only"),
        "expected macOS-only fixture-rebuild note in help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("2 => doctor threshold failure or argument parsing failure"),
        "expected exit code 2 contract in help output, got:\n{stdout}"
    );
}

#[test]
fn doctor_check_text_mode_failure_keeps_report_on_stdout_and_stderr_empty() {
    let output = run_output(&["doctor", "--check", "all", "--require-status", "supported"]);
    assert_eq!(output.status.code(), Some(2));

    let stdout = stdout_text(&output);
    assert!(
        stdout.contains("host_os:"),
        "expected doctor report on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("capabilities:"),
        "expected capability section on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("tools:"),
        "expected tools section on stdout, got:\n{stdout}"
    );

    let stderr = stderr_text(&output);
    assert!(
        stderr.trim().is_empty(),
        "expected empty stderr on threshold failure, got:\n{stderr}"
    );
}

#[test]
fn doctor_check_text_mode_pass_prints_report() {
    let output = run_output(&[
        "doctor",
        "--check",
        "macho-analysis",
        "--require-status",
        "supported-with-degraded-features",
    ]);
    assert_eq!(output.status.code(), Some(0));

    let stdout = stdout_text(&output);
    assert!(
        stdout.contains("host_os:"),
        "expected doctor report on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("capabilities:"),
        "expected capability section on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("tools:"),
        "expected tools section on stdout, got:\n{stdout}"
    );
}

#[test]
fn doctor_check_without_require_status_defaults_to_supported() {
    let defaulted = run_output(&["doctor", "--check", "all"]);
    let explicit_supported =
        run_output(&["doctor", "--check", "all", "--require-status", "supported"]);

    assert_eq!(
        defaulted.status.code(),
        explicit_supported.status.code(),
        "defaulted and explicit strict checks must behave identically"
    );
    assert_eq!(
        defaulted.status.code(),
        Some(2),
        "strict all-check should fail because it includes non-portable capabilities"
    );

    let stdout = stdout_text(&defaulted);
    assert!(
        stdout.contains("host_os:"),
        "expected doctor report on stdout, got:\n{stdout}"
    );

    let stderr = stderr_text(&defaulted);
    assert!(
        stderr.trim().is_empty(),
        "expected empty stderr when check threshold fails, got:\n{stderr}"
    );
}

#[test]
fn doctor_parse_failure_exit_two_uses_clap_stderr_without_report_stdout() {
    let output = run_output(&["doctor", "--require-status", "supported"]);
    assert_eq!(output.status.code(), Some(2));

    let stdout = stdout_text(&output);
    assert!(
        stdout.trim().is_empty(),
        "expected no doctor report on stdout for parse failure, got:\n{stdout}"
    );

    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("the following required arguments were not provided"),
        "expected clap parse error in stderr, got:\n{stderr}"
    );
}
