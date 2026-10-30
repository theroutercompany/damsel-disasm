//! Defense-delta diff tests (Wave 8.4): when a vendor adds anti-analysis
//! checks between versions, `defense_added` surfaces them with evidence.

use damsel_decomp::{DefenseFinding, diff_defenses};
use damsel_recon_catalog::{Confidence, Detection, Severity, scan_imports, scan_strings};

fn ptrace_import() -> Detection {
    Detection {
        code: "ANTI_DEBUG_PTRACE",
        location: "import:_ptrace".into(),
        evidence: "imports _ptrace".into(),
        severity: Severity::Medium,
        confidence: Confidence::Low,
    }
}

fn sysctl_import() -> Detection {
    Detection {
        code: "ANTI_DEBUG_SYSCTL",
        location: "import:_sysctl".into(),
        evidence: "imports _sysctl".into(),
        severity: Severity::Medium,
        confidence: Confidence::Low,
    }
}

#[test]
fn defense_added_when_new_check_appears() {
    let v1: Vec<Detection> = vec![];
    let v2 = vec![ptrace_import()];

    let (added, removed) = diff_defenses(&v1, &v2);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].code, "ANTI_DEBUG_PTRACE");
    assert_eq!(added[0].evidence, "imports _ptrace");
    assert!(removed.is_empty());
}

#[test]
fn defense_removed_when_check_disappears() {
    let v1 = vec![ptrace_import(), sysctl_import()];
    let v2 = vec![sysctl_import()];

    let (added, removed) = diff_defenses(&v1, &v2);
    assert!(added.is_empty());
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].code, "ANTI_DEBUG_PTRACE");
}

#[test]
fn no_delta_when_same_detections() {
    let v1 = vec![ptrace_import()];
    let v2 = vec![ptrace_import()];

    let (added, removed) = diff_defenses(&v1, &v2);
    assert!(added.is_empty());
    assert!(removed.is_empty());
}

#[test]
fn defense_finding_serializes() {
    let finding = DefenseFinding {
        code: "ANTI_VM_STRING".into(),
        location: "string:VMware Tools".into(),
        evidence: "contains VM indicator".into(),
    };
    let json = serde_json::to_string(&finding).unwrap();
    assert!(json.contains("ANTI_VM_STRING"));
    let back: DefenseFinding = serde_json::from_str(&json).unwrap();
    assert_eq!(back, finding);
}

#[test]
fn catalog_scanners_produce_diffable_detections() {
    // Integration: the catalog scanners produce detections that
    // diff_defenses consumes directly.
    let v1_imports: Vec<String> = vec!["printf".into()];
    let v2_imports: Vec<String> = vec!["printf".into(), "_ptrace".into()];

    let d1 = scan_imports(&v1_imports);
    let d2 = scan_imports(&v2_imports);

    let (added, _) = diff_defenses(&d1, &d2);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].code, "ANTI_DEBUG_PTRACE");
}

#[test]
fn strings_scanner_feeds_diff() {
    let v1: Vec<String> = vec!["normal string".into()];
    let v2: Vec<String> = vec!["normal string".into(), "/proc/vmware".into()];

    let d1 = scan_strings(&v1);
    let d2 = scan_strings(&v2);

    let (added, _) = diff_defenses(&d1, &d2);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].code, "ANTI_VM_STRING");
}
