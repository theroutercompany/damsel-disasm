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

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn parse_json_stdout(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!(
            "invalid json: {error}\nstdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[cfg(unix)]
mod harness {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "damsel-cli-{prefix}-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn write_exec_script(path: &Path, body: &str) {
        fs::write(path, body).expect("write script");
        let mut perms = fs::metadata(path).expect("script metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("chmod script");
    }

    #[derive(Debug, Clone, Copy)]
    pub(crate) struct HarnessProfile {
        pub(crate) xcrun_success: bool,
        pub(crate) sdk_probe_success: bool,
        pub(crate) sha256sum_success: bool,
        pub(crate) shasum_success: bool,
        pub(crate) openssl_success: bool,
        pub(crate) python3_success: bool,
        pub(crate) nm_success: bool,
        pub(crate) swiftc_success: bool,
    }

    #[derive(Debug)]
    pub(crate) struct FakeDoctorHarness {
        root: PathBuf,
    }

    impl FakeDoctorHarness {
        pub(crate) fn all_usable() -> Self {
            Self::new(HarnessProfile {
                xcrun_success: true,
                sdk_probe_success: true,
                sha256sum_success: true,
                shasum_success: true,
                openssl_success: true,
                python3_success: true,
                nm_success: true,
                swiftc_success: true,
            })
        }

        pub(crate) fn broken_hash_and_sdk() -> Self {
            Self::new(HarnessProfile {
                xcrun_success: true,
                sdk_probe_success: false,
                sha256sum_success: false,
                shasum_success: false,
                openssl_success: false,
                python3_success: true,
                nm_success: true,
                swiftc_success: true,
            })
        }

        pub(crate) fn shasum_only() -> Self {
            Self::new(HarnessProfile {
                xcrun_success: true,
                sdk_probe_success: true,
                sha256sum_success: false,
                shasum_success: true,
                openssl_success: true,
                python3_success: true,
                nm_success: true,
                swiftc_success: true,
            })
        }

        pub(crate) fn python_unusable() -> Self {
            Self::new(HarnessProfile {
                xcrun_success: true,
                sdk_probe_success: true,
                sha256sum_success: true,
                shasum_success: false,
                openssl_success: false,
                python3_success: false,
                nm_success: true,
                swiftc_success: true,
            })
        }

        pub(crate) fn path(&self) -> &str {
            self.root.to_str().expect("utf8 harness path")
        }

        fn new(profile: HarnessProfile) -> Self {
            let root = unique_temp_dir("doctor-process-harness");
            let sdk_dir = root.join("sdk");
            fs::create_dir_all(&sdk_dir).expect("create fake sdk");

            let xcrun_script = format!(
                "#!/bin/sh\nif [ \"{xcrun_success}\" != \"1\" ]; then exit 1; fi\ncase \"$1\" in\n  --version) exit 0 ;;\n  --find)\n    case \"$2\" in\n      clang) echo \"{root}/clang\"; exit 0 ;;\n      strip) echo \"{root}/strip\"; exit 0 ;;\n      swiftc)\n        if [ \"{swiftc_success}\" = \"1\" ]; then echo \"{root}/swiftc\"; exit 0; fi\n        exit 1 ;;\n      *) exit 1 ;;\n    esac ;;\n  --show-sdk-path)\n    if [ \"{sdk_probe_success}\" = \"1\" ]; then\n      echo \"{sdk}\"; exit 0\n    fi\n    exit 1 ;;\n  *) exit 1 ;;\nesac\n",
                xcrun_success = if profile.xcrun_success { "1" } else { "0" },
                root = root.display(),
                swiftc_success = if profile.swiftc_success { "1" } else { "0" },
                sdk_probe_success = if profile.sdk_probe_success { "1" } else { "0" },
                sdk = sdk_dir.display(),
            );
            write_exec_script(&root.join("xcrun"), &xcrun_script);
            write_exec_script(
                &root.join("clang"),
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 0; fi\nexit 1\n",
            );
            write_exec_script(
                &root.join("strip"),
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 0; fi\nexit 1\n",
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
    }

    impl Drop for FakeDoctorHarness {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

#[test]
fn doctor_parse_failure_text_mode_returns_exit_two_with_clap_stderr() {
    let output = run_output(&["doctor", "--require-status", "supported"]);
    assert_eq!(output.status.code(), Some(2));

    let stdout = stdout_text(&output);
    assert!(
        stdout.trim().is_empty(),
        "expected no report on stdout for parse failure, got:\n{stdout}"
    );

    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("the following required arguments were not provided"),
        "expected clap parse error in stderr, got:\n{stderr}"
    );
}

#[test]
fn doctor_parse_failure_json_mode_returns_exit_two_without_json_stdout() {
    let output = run_output(&[
        "--format",
        "json",
        "doctor",
        "--require-status",
        "supported",
    ]);
    assert_eq!(output.status.code(), Some(2));

    let stdout = stdout_text(&output);
    assert!(
        stdout.trim().is_empty(),
        "expected no JSON report on parse failure, got:\n{stdout}"
    );

    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("the following required arguments were not provided"),
        "expected clap parse error in stderr, got:\n{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn doctor_check_threshold_failure_text_mode_returns_exit_two_with_report_stdout() {
    let harness = harness::FakeDoctorHarness::all_usable();
    let output = run_output_with_env(
        &["doctor", "--check", "all", "--require-status", "supported"],
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-arm64"),
            (
                "DAMSEL_DOCTOR_TEST_TARGET_TRIPLE",
                "aarch64-unknown-linux-gnu",
            ),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(output.status.code(), Some(2));

    let stdout = stdout_text(&output);
    assert!(
        stdout.contains("host_os: linux"),
        "expected doctor report on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("capabilities:"),
        "expected capabilities section on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("tools:"),
        "expected tools section on stdout, got:\n{stdout}"
    );

    let stderr = stderr_text(&output);
    assert!(
        stderr.trim().is_empty(),
        "expected empty stderr on doctor threshold failure, got:\n{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn doctor_check_threshold_failure_json_mode_returns_exit_two_with_json_stdout() {
    let harness = harness::FakeDoctorHarness::all_usable();
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
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-arm64"),
            (
                "DAMSEL_DOCTOR_TEST_TARGET_TRIPLE",
                "aarch64-unknown-linux-gnu",
            ),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "expected empty stderr on threshold failure, got={:?}",
        output.stderr
    );

    let json = parse_json_stdout(&output);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "doctor");
    assert_eq!(json["data"]["host"]["os"], "linux");
    assert_eq!(
        json["data"]["host"]["target_triple"],
        "aarch64-unknown-linux-gnu"
    );
}

#[cfg(unix)]
#[test]
fn doctor_bench_runtime_thresholds_are_deterministic_under_simulated_linux_x86() {
    let harness = harness::FakeDoctorHarness::all_usable();
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
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-x86_64"),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(strict.status.code(), Some(2));
    let strict_json = parse_json_stdout(&strict);
    assert_eq!(
        strict_json["data"]["capabilities"]["bench_runtime"]["status"],
        "supported-with-degraded-features"
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
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-x86_64"),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(degraded.status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn doctor_fixture_drift_check_fails_without_usable_hash_backend() {
    let harness = harness::FakeDoctorHarness::broken_hash_and_sdk();
    let output = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "fixture-drift-check",
            "--require-status",
            "supported",
        ],
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-arm64"),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let json = parse_json_stdout(&output);
    assert_eq!(
        json["data"]["capabilities"]["fixture_drift_check"]["status"],
        "unsupported"
    );
    assert!(
        json["data"]["capabilities"]["fixture_drift_check"]["reasons"]
            .as_array()
            .expect("fixture drift reasons")
            .iter()
            .any(|reason| reason["code"] == "missing_usable_hash_tool")
    );
}

#[cfg(unix)]
#[test]
fn doctor_fixture_rebuild_reports_sdk_probe_failure_in_simulated_macos() {
    let harness = harness::FakeDoctorHarness::broken_hash_and_sdk();
    let output = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "fixture-rebuild",
            "--require-status",
            "supported",
        ],
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "macos-arm64"),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let json = parse_json_stdout(&output);
    assert_eq!(
        json["data"]["capabilities"]["fixture_rebuild"]["status"],
        "unsupported"
    );
    assert!(
        json["data"]["capabilities"]["fixture_rebuild"]["reasons"]
            .as_array()
            .expect("fixture rebuild reasons")
            .iter()
            .any(|reason| reason["code"] == "xcrun_sdk_path_probe_failed")
    );
}

#[cfg(unix)]
#[test]
fn doctor_hash_tool_selection_prefers_first_usable_backend_under_harness() {
    let harness = harness::FakeDoctorHarness::shasum_only();
    let output = run_output_with_env(
        &["--format", "json", "doctor"],
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "linux-arm64"),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    let json = parse_json_stdout(&output);
    assert_eq!(json["data"]["tools"]["selected_hash_tool"], "shasum");
    assert_eq!(
        json["data"]["tools"]["hash_tools"]["sha256sum"]["usable"],
        false
    );
    assert_eq!(
        json["data"]["tools"]["hash_tools"]["shasum"]["usable"],
        true
    );
}

#[cfg(unix)]
#[test]
fn doctor_fixture_rebuild_reports_unusable_python3_under_harness() {
    let harness = harness::FakeDoctorHarness::python_unusable();
    let output = run_output_with_env(
        &[
            "--format",
            "json",
            "doctor",
            "--check",
            "fixture-rebuild",
            "--require-status",
            "supported",
        ],
        &[
            ("DAMSEL_DOCTOR_TEST_SCENARIO", "macos-arm64"),
            ("PATH", harness.path()),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let json = parse_json_stdout(&output);
    assert!(
        json["data"]["capabilities"]["fixture_rebuild"]["reasons"]
            .as_array()
            .expect("fixture rebuild reasons")
            .iter()
            .any(|reason| reason["code"] == "unusable_python3")
    );
}
