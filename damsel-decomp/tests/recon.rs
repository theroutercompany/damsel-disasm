//! Recon triage contract tests.

use damsel_decomp::{Verdict, assess, detect_packers, shannon_entropy};

#[test]
fn entropy_of_uniform_data_is_zero() {
    assert_eq!(shannon_entropy(&[0u8; 64]), 0.0);
}

#[test]
fn entropy_of_random_data_is_high() {
    // Pseudo-random but deterministic bytes
    let data: Vec<u8> = (0..1024u32).map(|i| (i.wrapping_mul(2654435761) >> 16) as u8).collect();
    let entropy = shannon_entropy(&data);
    assert!(entropy > 7.0, "random data entropy {entropy:.2} should exceed 7.0");
}

#[test]
fn upx_magic_is_detected() {
    let data = b"\x00UPX! packed data here\x00".to_vec();
    let found = detect_packers(".text", &data);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "UPX");
    assert!(found[0].evidence.contains(".text"));
}

#[test]
fn clean_binary_gets_clean_verdict() {
    let sections = vec![
        (".text".to_string(), vec![0x48u8; 256]),
        (".rodata".to_string(), b"hello world".to_vec()),
    ];
    let imports = vec!["printf".to_string()];
    let strings = vec!["hello".to_string()];
    let report = assess(150, &sections, &imports, &strings);
    assert_eq!(report.verdict, Verdict::Clean);
    assert!(!report.stripped);
}

#[test]
fn ptrace_import_flags_needs_review() {
    let sections = vec![(".text".to_string(), vec![0x48u8; 64])];
    let imports = vec!["printf".to_string(), "ptrace".to_string()];
    let strings: Vec<String> = vec![];
    let report = assess(100, &sections, &imports, &strings);
    assert!(report.anti_analysis_indicators.iter().any(|i| i.name == "anti_debug_ptrace"));
    assert_eq!(report.verdict, Verdict::NeedsManualReview);
}

#[test]
fn upx_and_low_symbols_yield_likely_packed() {
    let sections = vec![(".text".to_string(), b"UPX!\x00\x01\x02".to_vec())];
    let imports: Vec<String> = vec![];
    let strings: Vec<String> = vec![];
    let report = assess(0, &sections, &imports, &strings);
    assert_eq!(report.verdict, Verdict::LikelyPacked);
    assert!(report.stripped);
}
