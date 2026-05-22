use damsel_core::{
    CapabilityStatus, CompatibilityCapability, CompatibilityCapabilityRole, CompatibilityHostClass,
    CompatibilityHostRule, CompatibilityIssue, CompatibilityPolicy, CompatibilityToolRequirement,
    HostArchitecture, HostPlatform,
};
use std::collections::BTreeSet;

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
fn compatibility_capability_doctor_check_portable_is_stable() {
    assert_eq!(
        CompatibilityCapability::DOCTOR_CHECK_PORTABLE,
        [
            CompatibilityCapability::MachoAnalysis,
            CompatibilityCapability::FixtureDriftCheck,
            CompatibilityCapability::BenchCompile,
        ]
    );
    assert!(
        !CompatibilityCapability::DOCTOR_CHECK_PORTABLE
            .contains(&CompatibilityCapability::FixtureRebuild)
    );
    assert!(
        !CompatibilityCapability::DOCTOR_CHECK_PORTABLE
            .contains(&CompatibilityCapability::BenchRuntime)
    );
    assert!(
        !CompatibilityCapability::DOCTOR_CHECK_PORTABLE
            .contains(&CompatibilityCapability::Benchmark)
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
fn compatibility_policy_target_sets_are_stable() {
    assert_eq!(
        CompatibilityPolicy::DOCTOR_CHECK_ALL,
        CompatibilityCapability::DOCTOR_CHECK_ALL
    );
    assert_eq!(
        CompatibilityPolicy::DOCTOR_CHECK_PORTABLE,
        CompatibilityCapability::DOCTOR_CHECK_PORTABLE
    );
}

#[test]
fn compatibility_policy_primary_supported_hosts_are_stable() {
    assert_eq!(
        CompatibilityPolicy::PRIMARY_SUPPORTED_HOSTS,
        [
            (HostPlatform::MacOS, HostArchitecture::Arm64),
            (HostPlatform::Linux, HostArchitecture::X86_64),
            (HostPlatform::Linux, HostArchitecture::Arm64),
        ]
    );
    assert!(CompatibilityPolicy::is_primary_supported_host(
        &HostPlatform::MacOS,
        &HostArchitecture::X86_64
    ));
    assert!(CompatibilityPolicy::is_primary_supported_host(
        &HostPlatform::Linux,
        &HostArchitecture::X86_64
    ));
    assert!(CompatibilityPolicy::is_primary_supported_host(
        &HostPlatform::Linux,
        &HostArchitecture::Arm64
    ));
    assert!(!CompatibilityPolicy::is_primary_supported_host(
        &HostPlatform::Windows,
        &HostArchitecture::X86_64
    ));
}

#[test]
fn compatibility_policy_matches_capability_role_and_summary_contracts() {
    for capability in CompatibilityCapability::ALL {
        let policy = CompatibilityPolicy::capability(capability);
        assert_eq!(policy.capability, capability);
        assert_eq!(policy.role, capability.role());
        assert_eq!(policy.summary_inputs, capability.summary_inputs());
    }
}

#[test]
fn compatibility_policy_host_rules_are_stable() {
    assert_eq!(
        CompatibilityPolicy::capability(CompatibilityCapability::MachoAnalysis).host_rule,
        CompatibilityHostRule::PrimarySupportedHosts
    );
    assert_eq!(
        CompatibilityPolicy::capability(CompatibilityCapability::FixtureRebuild).host_rule,
        CompatibilityHostRule::MacOSOnly
    );
    assert_eq!(
        CompatibilityPolicy::capability(CompatibilityCapability::FixtureDriftCheck).host_rule,
        CompatibilityHostRule::AnyHost
    );
    assert_eq!(
        CompatibilityPolicy::capability(CompatibilityCapability::Benchmark).host_rule,
        CompatibilityHostRule::DerivedFromInputs
    );
    assert_eq!(
        CompatibilityPolicy::capability(CompatibilityCapability::BenchCompile).host_rule,
        CompatibilityHostRule::PrimarySupportedHosts
    );
    assert_eq!(
        CompatibilityPolicy::capability(CompatibilityCapability::BenchRuntime).host_rule,
        CompatibilityHostRule::LinuxArm64Only
    );
}

#[test]
fn compatibility_policy_tool_requirements_are_stable() {
    let rebuild = CompatibilityPolicy::capability(CompatibilityCapability::FixtureRebuild);
    assert_eq!(rebuild.required_tools_any, &[]);
    assert_eq!(
        rebuild.required_tools_all,
        &[
            CompatibilityToolRequirement::Xcrun,
            CompatibilityToolRequirement::XcrunSdkPathProbe,
            CompatibilityToolRequirement::Clang,
            CompatibilityToolRequirement::Strip,
            CompatibilityToolRequirement::Python3,
            CompatibilityToolRequirement::Nm,
            CompatibilityToolRequirement::Swiftc,
        ]
    );

    let drift = CompatibilityPolicy::capability(CompatibilityCapability::FixtureDriftCheck);
    assert_eq!(drift.required_tools_all, &[]);
    assert_eq!(
        drift.required_tools_any,
        &[
            CompatibilityToolRequirement::Sha256sum,
            CompatibilityToolRequirement::Shasum,
            CompatibilityToolRequirement::Openssl,
        ]
    );
}

#[test]
fn compatibility_policy_is_exhaustive_and_ordered() {
    let policies = CompatibilityPolicy::policies();
    assert_eq!(policies.len(), CompatibilityCapability::ALL.len());
    let listed: Vec<_> = policies.iter().map(|policy| policy.capability).collect();
    assert_eq!(listed.as_slice(), &CompatibilityCapability::ALL);
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

#[test]
fn compatibility_policy_host_classification_is_stable() {
    assert_eq!(
        CompatibilityPolicy::host_class(&HostPlatform::MacOS, &HostArchitecture::Arm64),
        CompatibilityHostClass::PrimarySupported
    );
    assert_eq!(
        CompatibilityPolicy::host_class(&HostPlatform::MacOS, &HostArchitecture::X86_64),
        CompatibilityHostClass::PrimarySupported
    );
    assert_eq!(
        CompatibilityPolicy::host_class(&HostPlatform::Linux, &HostArchitecture::X86_64),
        CompatibilityHostClass::PrimarySupported
    );
    assert_eq!(
        CompatibilityPolicy::host_class(&HostPlatform::Linux, &HostArchitecture::Arm64),
        CompatibilityHostClass::PrimarySupported
    );
    assert_eq!(
        CompatibilityPolicy::host_class(&HostPlatform::Windows, &HostArchitecture::X86_64),
        CompatibilityHostClass::OutsidePrimaryMatrix
    );
    assert_eq!(
        CompatibilityPolicy::host_class(
            &HostPlatform::Unknown("solaris".to_string()),
            &HostArchitecture::Unknown("sparc64".to_string()),
        ),
        CompatibilityHostClass::OutsidePrimaryMatrix
    );
}

#[test]
fn compatibility_policy_expected_host_rule_statuses_are_stable() {
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::PrimarySupportedHosts,
            &HostPlatform::Linux,
            &HostArchitecture::Arm64,
        ),
        CapabilityStatus::Supported
    );
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::PrimarySupportedHosts,
            &HostPlatform::Windows,
            &HostArchitecture::X86_64,
        ),
        CapabilityStatus::SupportedWithDegradedFeatures
    );
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::MacOSOnly,
            &HostPlatform::Linux,
            &HostArchitecture::Arm64,
        ),
        CapabilityStatus::Unsupported
    );
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::MacOSOnly,
            &HostPlatform::MacOS,
            &HostArchitecture::X86_64,
        ),
        CapabilityStatus::Supported
    );
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::LinuxArm64Only,
            &HostPlatform::Linux,
            &HostArchitecture::X86_64,
        ),
        CapabilityStatus::SupportedWithDegradedFeatures
    );
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::LinuxArm64Only,
            &HostPlatform::Linux,
            &HostArchitecture::Arm64,
        ),
        CapabilityStatus::Supported
    );
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::AnyHost,
            &HostPlatform::Windows,
            &HostArchitecture::X86_64,
        ),
        CapabilityStatus::Supported
    );
    assert_eq!(
        CompatibilityPolicy::expected_status_for_host_rule(
            CompatibilityHostRule::DerivedFromInputs,
            &HostPlatform::Windows,
            &HostArchitecture::X86_64,
        ),
        CapabilityStatus::Supported
    );
}

#[test]
fn compatibility_verification_corpus_is_stable_and_complete() {
    let corpus = CompatibilityPolicy::verification_corpus();
    assert_eq!(corpus.len(), 7);

    let ids: BTreeSet<_> = corpus.iter().map(|scenario| scenario.id).collect();
    assert_eq!(ids.len(), corpus.len(), "scenario ids must be unique");

    for scenario in corpus {
        assert_eq!(
            CompatibilityPolicy::verification_scenario(scenario.id),
            Some(scenario),
            "scenario lookup should be stable for {}",
            scenario.id
        );
        assert_eq!(
            CompatibilityPolicy::host_class(
                &scenario.parsed_host_platform(),
                &scenario.parsed_host_architecture(),
            ),
            scenario.expected_host_class,
            "scenario {} host class drift",
            scenario.id
        );

        let capability_set: BTreeSet<_> = scenario
            .expected_capabilities
            .iter()
            .map(|expectation| expectation.capability)
            .collect();
        assert_eq!(
            capability_set,
            CompatibilityCapability::ALL.into_iter().collect(),
            "scenario {} must provide expectations for every capability",
            scenario.id
        );
    }
}

fn severity(status: CapabilityStatus) -> u8 {
    match status {
        CapabilityStatus::Supported => 0,
        CapabilityStatus::SupportedWithDegradedFeatures => 1,
        CapabilityStatus::Unsupported => 2,
    }
}

#[test]
fn compatibility_verification_corpus_semantics_are_stable() {
    for scenario in CompatibilityPolicy::verification_corpus() {
        let benchmark = scenario
            .capability_expectation(CompatibilityCapability::Benchmark)
            .expect("benchmark expectation missing");
        let bench_compile = scenario
            .capability_expectation(CompatibilityCapability::BenchCompile)
            .expect("bench_compile expectation missing");
        let bench_runtime = scenario
            .capability_expectation(CompatibilityCapability::BenchRuntime)
            .expect("bench_runtime expectation missing");

        let split_max = if severity(bench_compile.status) >= severity(bench_runtime.status) {
            bench_compile.status
        } else {
            bench_runtime.status
        };
        assert_eq!(
            benchmark.status, split_max,
            "scenario {} benchmark summary drift",
            scenario.id
        );

        let non_summary_max = scenario
            .expected_capabilities
            .iter()
            .filter(|expectation| expectation.capability.is_primary_input())
            .map(|expectation| expectation.status)
            .max_by_key(|status| severity(*status))
            .expect("non-summary capability status expected");
        assert_eq!(
            scenario.expected_overall_status, non_summary_max,
            "scenario {} overall status drift",
            scenario.id
        );

        let issue_codes: BTreeSet<_> = scenario.expected_issue_codes.iter().copied().collect();
        let deduped_reason_union: BTreeSet<_> = scenario
            .expected_capabilities
            .iter()
            .filter(|expectation| expectation.status != CapabilityStatus::Supported)
            .flat_map(|expectation| expectation.reason_codes.iter().copied())
            .collect();
        assert_eq!(
            issue_codes, deduped_reason_union,
            "scenario {} issue-code union drift",
            scenario.id
        );

        for expectation in scenario
            .expected_capabilities
            .iter()
            .filter(|expectation| expectation.status != CapabilityStatus::Supported)
        {
            assert!(
                !expectation.reason_codes.is_empty(),
                "scenario {} capability {} needs at least one reason code",
                scenario.id,
                expectation.capability
            );
        }

        assert_eq!(
            scenario
                .tools_usable
                .iter()
                .map(|(requirement, _)| requirement)
                .collect::<BTreeSet<_>>()
                .len(),
            scenario.tools_usable.len(),
            "scenario {} tool requirements should not duplicate keys",
            scenario.id
        );
    }
}
