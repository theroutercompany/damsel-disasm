//! Adversarial fixture tests (Wave 8.4): labeled binaries that each
//! contain a known defense, scanned end-to-end through the recon
//! path. Each fixture asserts the expected detection fires and that
//! clean binaries produce no detections (no false positives).

use damsel_macho::load;
use damsel_recon_catalog::{
    Detection, Severity, scan_code_bytes_for_xor_loop, scan_imports,
    scan_strings, swift,
};

/// Load a Mach-O fixture's imports and code bytes.
fn load_fixture(name: &str) -> (Vec<String>, Vec<u8>, Vec<String>) {
    let path = format!(
        "{}/../fixtures/bin/adversarial/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let image = load(&path)
        .unwrap_or_else(|e| panic!("load {name}: {e}"));
    let imports: Vec<String> = image.imports().iter().map(|i| i.name.clone()).collect();
    let code: Vec<u8> = image
        .sections()
        .iter()
        .filter(|s| s.name == "__text")
        .filter_map(|s| image.bytes_for_file_range(s.file_offset?, s.file_size))
        .flat_map(|b| b.iter().copied())
        .collect();
    let strings = vec![]; // strings not needed for these fixtures
    (imports, code, strings)
}

fn has_code(dets: &[Detection], code: &str) -> bool {
    dets.iter().any(|d| d.code == code)
}

// -------------------------------------------------------------------------
// anti-debug fixture: ptrace + sysctl — both must fire
// -------------------------------------------------------------------------

#[test]
fn anti_debug_fixture_detects_ptrace() {
    let (imports, _, _) = load_fixture("anti-debug-arm64");
    let dets = scan_imports(&imports);
    assert!(
        has_code(&dets, "ANTI_DEBUG_PTRACE"),
        "expected ANTI_DEBUG_PTRACE in {dets:?}"
    );
    let ptrace_det = dets.iter().find(|d| d.code == "ANTI_DEBUG_PTRACE").unwrap();
    assert_eq!(ptrace_det.severity, Severity::Medium);
}

#[test]
fn anti_debug_fixture_detects_sysctl() {
    let (imports, _, _) = load_fixture("anti-debug-arm64");
    let dets = scan_imports(&imports);
    assert!(
        has_code(&dets, "ANTI_DEBUG_SYSCTL"),
        "expected ANTI_DEBUG_SYSCTL in {dets:?}"
    );
}

// -------------------------------------------------------------------------
// timing fixture: mach_absolute_time — must fire
// -------------------------------------------------------------------------

#[test]
fn timing_fixture_detects_mach_absolute_time() {
    let (imports, _, _) = load_fixture("timing-arm64");
    let dets = scan_imports(&imports);
    assert!(
        has_code(&dets, "ANTI_DEBUG_TIMING"),
        "expected ANTI_DEBUG_TIMING from mach_absolute_time in {dets:?}"
    );
}

// -------------------------------------------------------------------------
// string-encryption fixture: XOR decode loop — must fire
// -------------------------------------------------------------------------

#[test]
fn strcrypt_fixture_detects_xor_loop() {
    let (_, code, _) = load_fixture("strcrypt-arm64");
    assert!(!code.is_empty(), "fixture has no __text bytes");
    let dets = scan_code_bytes_for_xor_loop(&code);
    assert!(
        has_code(&dets, "STRING_ENCRYPTION"),
        "expected STRING_ENCRYPTION in {} bytes of __text: {dets:?}",
        code.len()
    );
}

// -------------------------------------------------------------------------
// Swift fixture: demangling + section inventory
// -------------------------------------------------------------------------

#[test]
fn swift_rich_fixture_demangles() {
    let path = format!(
        "{}/../fixtures/bin/adversarial/swift-rich-arm64",
        env!("CARGO_MANIFEST_DIR")
    );
    let image = load(&path)
        .unwrap_or_else(|e| panic!("load swift-rich: {e}"));
    // Count Swift symbols
    let swift_syms: Vec<&str> = image
        .symbols()
        .iter()
        .map(|s| s.name.as_str())
        .filter(|n| n.starts_with("_$s"))
        .collect();
    assert!(!swift_syms.is_empty(), "no Swift symbols found");

    let demangled = swift_syms
        .iter()
        .filter_map(|s| swift::demangle(s))
        .collect::<Vec<_>>();
    let rate = demangled.len() as f64 / swift_syms.len() as f64;
    
// Report failure patterns for diagnosis
    let failures: Vec<&&str> = swift_syms.iter()
        .filter(|s| damsel_recon_catalog::swift::demangle(s).is_none())
        .collect();
    eprintln!("demangle failures ({}):", failures.len());
    for f in failures.iter().take(8) { eprintln!("  {f}"); }
    // All-symbol rate includes data symbols (metadata records,
    // witness tables, generic descriptors) which use deeper
    // mangling forms. Function-symbol rate is higher.
    assert!(
        rate >= 0.80,
        "demangling rate {rate:.2} < 0.80 ({} of {})",
        demangled.len(),
        swift_syms.len()
    );
}

#[test]
fn swift_rich_fixture_has_swift_sections() {
    let path = format!(
        "{}/../fixtures/bin/adversarial/swift-rich-arm64",
        env!("CARGO_MANIFEST_DIR")
    );
    let image = load(&path)
        .unwrap_or_else(|e| panic!("load swift-rich: {e}"));
    let sections: Vec<(String, usize)> = image
        .sections()
        .iter()
        .map(|s| (s.name.clone(), s.file_size as usize))
        .collect();
    let swift_secs = swift::swift_sections(&sections);
    assert!(swift_secs.len() >= 6, "expected ≥6 swift5 sections, got {}", swift_secs.len());
    assert!(swift::is_swift_binary(&sections));
}

// -------------------------------------------------------------------------
// Negative control: clean binaries must produce no detections
// -------------------------------------------------------------------------

#[test]
fn clean_fixture_no_false_positives() {
    // objc-sample: a normal ObjC program — no anti-analysis imports,
    // no XOR loops.
    let path = format!(
        "{}/../fixtures/bin/objc-sample",
        env!("CARGO_MANIFEST_DIR")
    );
    let image = load(&path)
        .unwrap_or_else(|e| panic!("load objc-sample: {e}"));
    let imports: Vec<String> = image.imports().iter().map(|i| i.name.clone()).collect();
    let code: Vec<u8> = image
        .sections()
        .iter()
        .filter(|s| s.name == "__text")
        .filter_map(|s| image.bytes_for_file_range(s.file_offset?, s.file_size))
        .flat_map(|b| b.iter().copied())
        .collect();

    let import_dets = scan_imports(&imports);
    let xor_dets = scan_code_bytes_for_xor_loop(&code);
    assert!(
        import_dets.is_empty(),
        "false positive on clean binary: {import_dets:?}"
    );
    assert!(
        xor_dets.is_empty(),
        "false positive XOR on clean binary: {xor_dets:?}"
    );
}
