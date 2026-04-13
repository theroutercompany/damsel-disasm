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
        stdout.contains("CI guidance:"),
        "expected CI guidance section in help output, got:\n{stdout}"
    );
    assert!(
        stdout
            .contains("macOS runners: `doctor --check macho-analysis --require-status supported`"),
        "expected macOS CI target guidance in help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains(
            "linux x86_64 runners: `doctor --check macho-analysis --require-status supported`"
        ),
        "expected linux x86_64 CI target guidance in help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains(
            "linux arm64 runners: `doctor --check macho-analysis --require-status supported`"
        ),
        "expected linux arm64 CI target guidance in help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("2 => doctor threshold failure or argument parsing failure"),
        "expected exit code 2 contract in help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("threshold failures still print a doctor report to stdout"),
        "expected threshold-failure stdout behavior note in help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("argument parsing failures come from clap on stderr"),
        "expected parse-failure stderr behavior note in help output, got:\n{stdout}"
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
