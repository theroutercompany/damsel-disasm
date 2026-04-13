use assert_cmd::Command;
use damsel_macho::load;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(unix)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn normalize(output: &[u8]) -> String {
    let root = repo_root();
    String::from_utf8_lossy(output)
        .replace(root.to_string_lossy().as_ref(), "$REPO")
        .trim()
        .to_string()
}

fn normalize_doctor_snapshot(output: &str) -> String {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if line.starts_with("host_os: ") {
                Some("host_os: <host_os>".to_string())
            } else if line.starts_with("host_architecture: ") {
                Some("host_architecture: <host_architecture>".to_string())
            } else if line.starts_with("target_triple: ") {
                Some("target_triple: <target_triple>".to_string())
            } else if line.starts_with("overall_status: ") {
                Some(line.to_string())
            } else if line == "capabilities:" {
                Some("capabilities:".to_string())
            } else if trimmed.starts_with("macho_analysis: ") {
                Some(line.to_string())
            } else if trimmed.starts_with("fixture_rebuild: ") {
                Some(line.to_string())
            } else if trimmed.starts_with("fixture_drift_check: ") {
                Some(line.to_string())
            } else if trimmed.starts_with("bench_compile: ") {
                Some(line.to_string())
            } else if trimmed.starts_with("bench_runtime: ") {
                Some(line.to_string())
            } else if trimmed.starts_with("benchmark: ") {
                Some(line.to_string())
            } else if line.starts_with("    - ") {
                let code = trimmed
                    .trim_start_matches("- ")
                    .split(':')
                    .next()
                    .unwrap_or("<code>");
                Some(format!("    - {}: <message>", code))
            } else if line == "tools:" {
                Some("tools:".to_string())
            } else if trimmed.starts_with("xcrun: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("strip: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("clang: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("python3: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("nm: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("sdk_path_probe: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("selected_hash_tool: ") {
                Some(line.to_string())
            } else if trimmed.starts_with("sha256sum: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("shasum: ") {
                Some(normalize_tool_line(line))
            } else if trimmed.starts_with("openssl: ") {
                Some(normalize_tool_line(line))
            } else if line.starts_with("issues:") {
                Some(line.to_string())
            } else if line.starts_with("  - ") {
                let code = trimmed
                    .trim_start_matches("- ")
                    .split(':')
                    .next()
                    .unwrap_or("<code>");
                Some(format!("  - {}: <message>", code))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_tool_line(line: &str) -> String {
    if let Some((prefix, _)) = line.split_once(" path=") {
        format!("{prefix} path=<path>")
    } else {
        line.to_string()
    }
}

fn normalize_cache_snapshot(output: &str) -> String {
    output
        .lines()
        .map(|line| {
            if line.starts_with("cache_uuid: ") {
                "cache_uuid: <cache_uuid>".to_string()
            } else if line.starts_with("  - id=") {
                "  - id=<id> index=<index> base=<addr> install_name=<install_name> basename=<basename> member=<member>".to_string()
            } else if line.starts_with("lookup_address: kind=") {
                "lookup_address: kind=<kind> cache_vmaddr=<addr>".to_string()
            } else if line.starts_with("  image_id=") {
                "  image_id=<image_id>".to_string()
            } else if line.starts_with("  install_name=") {
                "  install_name=<install_name>".to_string()
            } else if line.starts_with("  image_base_vmaddr=") {
                "  image_base_vmaddr=<addr>".to_string()
            } else if line.starts_with("  image_offset=") {
                "  image_offset=<offset>".to_string()
            } else if line.starts_with("  member_file_offset=") {
                "  member_file_offset=<offset>".to_string()
            } else if line.starts_with("  symbol=") {
                "  symbol=<symbol>".to_string()
            } else if line.starts_with("  symbol_address=") {
                "  symbol_address=<addr>".to_string()
            } else if line.starts_with("  offset_from_symbol=") {
                "  offset_from_symbol=<offset>".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_snapshot(args: &[&str]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(output.status.success(), "command failed: {:?}", output);
    normalize(&output.stdout)
}

fn run_snapshot_with_env(args: &[&str], envs: &[(&str, &str)]) -> String {
    let mut command = Command::cargo_bin("damsel-cli").expect("binary exists");
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command.output().expect("command runs");
    assert!(output.status.success(), "command failed: {:?}", output);
    normalize(&output.stdout)
}

fn run_snapshot_owned(args: &[String]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(output.status.success(), "command failed: {:?}", output);
    normalize(&output.stdout)
}

fn run_snapshot_err(args: &[&str]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded: {:?}",
        output
    );
    normalize(&output.stderr)
}

#[cfg(unix)]
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

#[cfg(unix)]
fn write_exec_script(path: &Path, body: &str) {
    fs::write(path, body).expect("write script");
    let mut perms = fs::metadata(path).expect("script metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod script");
}

#[cfg(unix)]
#[derive(Debug)]
struct DoctorSnapshotHarness {
    root: PathBuf,
}

#[cfg(unix)]
impl DoctorSnapshotHarness {
    fn new() -> Self {
        let root = unique_temp_dir("doctor-snapshot");
        let sdk_dir = root.join("sdk");
        fs::create_dir_all(&sdk_dir).expect("create fake sdk");
        let xcrun_script = format!(
            "#!/bin/sh\ncase \"$1\" in\n  --version) exit 0 ;;\n  --find)\n    case \"$2\" in\n      clang) echo \"{root}/clang\"; exit 0 ;;\n      strip) echo \"{root}/strip\"; exit 0 ;;\n      *) exit 1 ;;\n    esac ;;\n  --show-sdk-path)\n    echo \"{sdk}\"; exit 0 ;;\n  *) exit 1 ;;\nesac\n",
            root = root.display(),
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
        write_exec_script(&root.join("python3"), "#!/bin/sh\nexit 0\n");
        write_exec_script(
            &root.join("nm"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 0; fi\nexit 1\n",
        );
        write_exec_script(&root.join("sha256sum"), "#!/bin/sh\nexit 0\n");
        write_exec_script(&root.join("shasum"), "#!/bin/sh\nexit 0\n");
        write_exec_script(&root.join("openssl"), "#!/bin/sh\nexit 0\n");
        Self { root }
    }

    fn path(&self) -> &str {
        self.root.to_str().expect("utf8 harness path")
    }
}

#[cfg(unix)]
impl Drop for DoctorSnapshotHarness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn info_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["info", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("info_snapshot", stdout);
}

#[test]
fn symbols_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["symbols", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("symbols_snapshot", stdout);
}

#[test]
fn sections_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["sections", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("sections_snapshot", stdout);
}

#[test]
fn imports_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&["imports", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("imports_snapshot", stdout);
}

#[test]
fn dyld_snapshot() {
    let path = fixture("import-rich");
    let stdout = run_snapshot(&["dyld", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("dyld_snapshot", stdout);
}

#[test]
fn dyld_filtered_snapshot() {
    let path = fixture("import-rich");
    let stdout = run_snapshot(&[
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
        "--stubs",
        "--name",
        "puts",
        "--sort",
        "name",
    ]);
    insta::assert_snapshot!("dyld_filtered_snapshot", stdout);
}

#[test]
fn dyld_kind_ordinal_snapshot() {
    let path = fixture("import-lazy");
    let stdout = run_snapshot(&[
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
        "--sort",
        "address",
    ]);
    insta::assert_snapshot!("dyld_kind_ordinal_snapshot", stdout);
}

#[test]
fn slices_snapshot() {
    let path = fixture("universal-hello");
    let stdout = run_snapshot(&["slices", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("slices_snapshot", stdout);
}

#[test]
fn objc_snapshot() {
    let path = fixture("objc-sample");
    let stdout = run_snapshot(&["objc", path.to_str().expect("utf8 path")]);
    insta::assert_snapshot!("objc_snapshot", stdout);
}

#[test]
fn objc_methods_snapshot() {
    let path = fixture("objc-sample");
    let stdout = run_snapshot(&[
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
        "--owner",
        "GreetingProviding",
    ]);
    insta::assert_snapshot!("objc_methods_snapshot", stdout);
}

#[test]
fn objc_provenance_filtered_snapshot() {
    let path = fixture("objc-sample");
    let stdout = run_snapshot(&[
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
        "--name-source",
        "pointer-table",
        "--selector-source",
        "unresolved",
    ]);
    insta::assert_snapshot!("objc_provenance_filtered_snapshot", stdout);
}

#[test]
fn disasm_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&[
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_main",
        "--limit",
        "8",
    ]);
    insta::assert_snapshot!("disasm_snapshot", stdout);
}

#[test]
fn disasm_addr_snapshot() {
    let path = fixture("arm64-symbolized");
    let image = load(&path).expect("load fixture");
    let main = image.symbol_by_name("_main").expect("main symbol");
    let stdout = run_snapshot_owned(&[
        "disasm".to_string(),
        path.to_string_lossy().to_string(),
        "--addr".to_string(),
        format!("{:#x}", main.address),
        "--limit".to_string(),
        "8".to_string(),
    ]);
    insta::assert_snapshot!("disasm_addr_snapshot", stdout);
}

#[test]
fn disasm_section_snapshot() {
    let path = fixture("arm64-symbolized");
    let stdout = run_snapshot(&[
        "disasm",
        path.to_str().expect("utf8 path"),
        "--section",
        "__text",
        "--limit",
        "8",
    ]);
    insta::assert_snapshot!("disasm_section_snapshot", stdout);
}

#[test]
fn disasm_show_values_snapshot() {
    let path = fixture("semantic-switch");
    let stdout = run_snapshot(&[
        "disasm",
        path.to_str().expect("utf8 path"),
        "--section",
        "__text",
        "--limit",
        "16",
        "--show-references",
        "--show-values",
    ]);
    insta::assert_snapshot!("disasm_show_values_snapshot", stdout);
}

#[test]
fn doctor_snapshot() {
    #[cfg(unix)]
    let stdout = {
        let harness = DoctorSnapshotHarness::new();
        run_snapshot_with_env(
            &["doctor"],
            &[
                ("PATH", harness.path()),
                ("DAMSEL_DOCTOR_TEST_SCENARIO", "macos-arm64"),
                ("DAMSEL_DOCTOR_TEST_TARGET_TRIPLE", "aarch64-apple-darwin"),
            ],
        )
    };
    #[cfg(not(unix))]
    let stdout = run_snapshot(&["doctor"]);
    let normalized = normalize_doctor_snapshot(&stdout);
    insta::assert_snapshot!("doctor_snapshot", normalized);
}

#[test]
fn cache_info_snapshot() {
    let path = cache_fixture("valid-single-arm64.cache");
    let stdout = run_snapshot(&["cache", "info", path.to_str().expect("utf8 path")]);
    let normalized = normalize_cache_snapshot(&stdout);
    insta::assert_snapshot!(
        normalized,
        @r###"
        cache: $REPO/fixtures/shared-cache-corpus/valid-single-arm64.cache
        cache_uuid: <cache_uuid>
        architecture: arm64
        members: 1
        images: 3
        has_local_symbols: false
        members:
          - name=valid-single-arm64.cache role=root path=$REPO/fixtures/shared-cache-corpus/valid-single-arm64.cache
        "###
    );
}

#[test]
fn cache_images_snapshot() {
    let path = cache_fixture("valid-single-arm64.cache");
    let stdout = run_snapshot(&[
        "cache",
        "images",
        path.to_str().expect("utf8 path"),
        "--limit",
        "1",
    ]);
    let normalized = normalize_cache_snapshot(&stdout);
    insta::assert_snapshot!(
        normalized,
        @r###"
        images: returned=1 total=3 truncated=true
          - id=<id> index=<index> base=<addr> install_name=<install_name> basename=<basename> member=<member>
        "###
    );
}

#[test]
fn cache_lookup_address_snapshot() {
    let path = cache_fixture("valid-single-arm64.cache");
    let mapped = 0x1800_00638u64;
    let mapped_arg = format!("{mapped:#x}");
    let stdout = run_snapshot(&[
        "cache",
        "lookup-address",
        path.to_str().expect("utf8 path"),
        mapped_arg.as_str(),
    ]);
    let normalized = normalize_cache_snapshot(&stdout);
    insta::assert_snapshot!(
        normalized,
        @r###"
        lookup_address: kind=<kind> cache_vmaddr=<addr>
          image_id=<image_id>
          install_name=<install_name>
          image_base_vmaddr=<addr>
          image_offset=<offset>
          member_file_offset=<offset>
          symbol=<symbol>
          symbol_address=<addr>
          offset_from_symbol=<offset>
        "###
    );
}

#[test]
fn cache_image_not_found_error_snapshot() {
    let path = cache_fixture("valid-single-arm64.cache");
    let stderr = run_snapshot_err(&[
        "cache",
        "image",
        path.to_str().expect("utf8 path"),
        "missing-image",
    ]);
    insta::assert_snapshot!(
        stderr,
        @r###"error [cache_image_not_found]: cache image not found: missing-image"###
    );
}
