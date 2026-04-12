use damsel_core::{CapabilityStatus, CompatibilityIssue, HostArchitecture, HostPlatform};

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
