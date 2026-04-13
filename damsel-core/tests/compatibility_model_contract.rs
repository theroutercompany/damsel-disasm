use damsel_core::{
    CapabilityStatus, CompatibilityCapability, CompatibilityCapabilityRole, CompatibilityIssue,
    HostArchitecture, HostPlatform,
};

#[test]
fn host_platform_parsing_and_display_is_stable() {
    assert_eq!(HostPlatform::parse("darwin"), HostPlatform::MacOS);
    assert_eq!(HostPlatform::parse("linux"), HostPlatform::Linux);
    assert_eq!(HostPlatform::parse("win32"), HostPlatform::Windows);
    assert_eq!(HostPlatform::MacOS.to_string(), "macos");
    assert_eq!(HostPlatform::Linux.to_string(), "linux");
    assert_eq!(HostPlatform::Windows.to_string(), "windows");

    let unknown = HostPlatform::parse("Plan9");
    assert_eq!(unknown, HostPlatform::Unknown("Plan9".to_string()));
    assert_eq!(unknown.raw_identifier(), Some("Plan9"));
}

#[test]
fn host_platform_current_uses_runtime_identifier() {
    assert_eq!(
        HostPlatform::current(),
        HostPlatform::parse(std::env::consts::OS)
    );
}

#[test]
fn host_architecture_parsing_and_display_is_stable() {
    assert_eq!(HostArchitecture::parse("aarch64"), HostArchitecture::Arm64);
    assert_eq!(HostArchitecture::parse("arm64"), HostArchitecture::Arm64);
    assert_eq!(HostArchitecture::parse("amd64"), HostArchitecture::X86_64);
    assert_eq!(HostArchitecture::parse("x86_64"), HostArchitecture::X86_64);
    assert_eq!(HostArchitecture::Arm64.to_string(), "arm64");
    assert_eq!(HostArchitecture::X86_64.to_string(), "x86_64");

    let unknown = HostArchitecture::parse("mips64");
    assert_eq!(unknown, HostArchitecture::Unknown("mips64".to_string()));
    assert_eq!(unknown.raw_identifier(), Some("mips64"));
}

#[test]
fn host_architecture_current_uses_runtime_identifier() {
    assert_eq!(
        HostArchitecture::current(),
        HostArchitecture::parse(std::env::consts::ARCH)
    );
}

#[test]
fn capability_status_display_and_helpers_are_stable() {
    assert_eq!(CapabilityStatus::Supported.to_string(), "supported");
    assert_eq!(
        CapabilityStatus::SupportedWithDegradedFeatures.to_string(),
        "supported-with-degraded-features"
    );
    assert_eq!(CapabilityStatus::Unsupported.to_string(), "unsupported");

    assert!(CapabilityStatus::Supported.is_supported());
    assert!(CapabilityStatus::SupportedWithDegradedFeatures.is_supported());
    assert!(!CapabilityStatus::Unsupported.is_supported());
    assert!(!CapabilityStatus::Supported.is_degraded());
    assert!(CapabilityStatus::SupportedWithDegradedFeatures.is_degraded());
    assert!(!CapabilityStatus::Unsupported.is_degraded());
}

#[test]
fn compatibility_issue_constructor_and_display_are_stable() {
    let issue = CompatibilityIssue::new("missing-tool", "xcrun was not found");
    assert_eq!(issue.code, "missing-tool");
    assert_eq!(issue.message, "xcrun was not found");
    assert_eq!(issue.to_string(), "missing-tool: xcrun was not found");
}

#[test]
fn compatibility_capability_key_and_cli_name_are_stable() {
    assert_eq!(
        CompatibilityCapability::MachoAnalysis.key(),
        "macho_analysis"
    );
    assert_eq!(
        CompatibilityCapability::FixtureRebuild.key(),
        "fixture_rebuild"
    );
    assert_eq!(
        CompatibilityCapability::FixtureDriftCheck.key(),
        "fixture_drift_check"
    );
    assert_eq!(CompatibilityCapability::Benchmark.key(), "benchmark");
    assert_eq!(CompatibilityCapability::BenchCompile.key(), "bench_compile");
    assert_eq!(CompatibilityCapability::BenchRuntime.key(), "bench_runtime");

    assert_eq!(
        CompatibilityCapability::MachoAnalysis.cli_name(),
        "macho-analysis"
    );
    assert_eq!(
        CompatibilityCapability::FixtureRebuild.cli_name(),
        "fixture-rebuild"
    );
    assert_eq!(
        CompatibilityCapability::FixtureDriftCheck.cli_name(),
        "fixture-drift-check"
    );
    assert_eq!(CompatibilityCapability::Benchmark.cli_name(), "benchmark");
    assert_eq!(
        CompatibilityCapability::BenchCompile.cli_name(),
        "bench-compile"
    );
    assert_eq!(
        CompatibilityCapability::BenchRuntime.cli_name(),
        "bench-runtime"
    );
}

#[test]
fn compatibility_capability_parse_supports_json_and_cli_forms() {
    assert_eq!(
        CompatibilityCapability::parse("macho_analysis"),
        Some(CompatibilityCapability::MachoAnalysis)
    );
    assert_eq!(
        CompatibilityCapability::parse("macho-analysis"),
        Some(CompatibilityCapability::MachoAnalysis)
    );
    assert_eq!(
        CompatibilityCapability::parse("fixture_rebuild"),
        Some(CompatibilityCapability::FixtureRebuild)
    );
    assert_eq!(
        CompatibilityCapability::parse("fixture-drift-check"),
        Some(CompatibilityCapability::FixtureDriftCheck)
    );
    assert_eq!(
        CompatibilityCapability::parse("bench_compile"),
        Some(CompatibilityCapability::BenchCompile)
    );
    assert_eq!(
        CompatibilityCapability::parse("bench-runtime"),
        Some(CompatibilityCapability::BenchRuntime)
    );
    assert_eq!(CompatibilityCapability::parse("unknown"), None);
}

#[test]
fn compatibility_capability_all_is_deterministic() {
    assert_eq!(
        CompatibilityCapability::ALL,
        [
            CompatibilityCapability::MachoAnalysis,
            CompatibilityCapability::FixtureRebuild,
            CompatibilityCapability::FixtureDriftCheck,
            CompatibilityCapability::Benchmark,
            CompatibilityCapability::BenchCompile,
            CompatibilityCapability::BenchRuntime,
        ]
    );
}

#[test]
fn compatibility_capability_doctor_check_all_is_stable() {
    assert_eq!(
        CompatibilityCapability::DOCTOR_CHECK_ALL,
        [
            CompatibilityCapability::MachoAnalysis,
            CompatibilityCapability::FixtureRebuild,
            CompatibilityCapability::FixtureDriftCheck,
            CompatibilityCapability::BenchCompile,
            CompatibilityCapability::BenchRuntime,
        ]
    );
    assert!(
        !CompatibilityCapability::DOCTOR_CHECK_ALL.contains(&CompatibilityCapability::Benchmark)
    );
}

#[test]
fn compatibility_capability_roles_distinguish_primary_and_derived() {
    assert_eq!(
        CompatibilityCapability::Benchmark.role(),
        CompatibilityCapabilityRole::DerivedSummary
    );
    assert!(CompatibilityCapability::Benchmark.is_derived_summary());
    assert!(!CompatibilityCapability::Benchmark.is_primary_input());

    for capability in [
        CompatibilityCapability::MachoAnalysis,
        CompatibilityCapability::FixtureRebuild,
        CompatibilityCapability::FixtureDriftCheck,
        CompatibilityCapability::BenchCompile,
        CompatibilityCapability::BenchRuntime,
    ] {
        assert_eq!(capability.role(), CompatibilityCapabilityRole::PrimaryInput);
        assert!(capability.is_primary_input());
        assert!(!capability.is_derived_summary());
    }
}

#[test]
fn compatibility_capability_benchmark_summary_inputs_are_stable() {
    assert_eq!(
        CompatibilityCapability::Benchmark.summary_inputs(),
        &[
            CompatibilityCapability::BenchCompile,
            CompatibilityCapability::BenchRuntime,
        ]
    );
    assert!(
        CompatibilityCapability::MachoAnalysis
            .summary_inputs()
            .is_empty()
    );
    assert!(
        CompatibilityCapability::FixtureRebuild
            .summary_inputs()
            .is_empty()
    );
    assert!(
        CompatibilityCapability::FixtureDriftCheck
            .summary_inputs()
            .is_empty()
    );
    assert!(
        CompatibilityCapability::BenchCompile
            .summary_inputs()
            .is_empty()
    );
    assert!(
        CompatibilityCapability::BenchRuntime
            .summary_inputs()
            .is_empty()
    );
}
