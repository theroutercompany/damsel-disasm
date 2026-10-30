//! Anti-analysis detection catalog (Wave 8.4): named detections with
//! severity, confidence, evidence, and remediation hints. Every
//! detection is grounded in an address/instruction/constant; wording
//! is "indicated", never "confirmed".

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Typed detection model
// ---------------------------------------------------------------------------

/// Severity of an indicated defense/obfuscation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Informational — presence alone is not defensive.
    Info,
    /// Likely defense; may be dual-use (debugger APIs used legitimately).
    Medium,
    /// Strong defense indication; multiple independent signals.
    High,
}

/// Confidence in the detection (how specific the evidence is).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// Single weak signal (import name, string match).
    Low,
    /// Behavioral pattern (instruction sequence, constant combo).
    Medium,
    /// Multiple independent signals agree.
    High,
}

/// One detection produced by a scanner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    /// Catalog code, e.g. `ANTI_DEBUG_PTRACE`.
    pub code: &'static str,
    /// Where: address (hex), symbol, or section.
    pub location: String,
    /// Human-readable evidence: what was seen.
    pub evidence: String,
    pub severity: Severity,
    pub confidence: Confidence,
}

/// Catalog metadata for a detection code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub code: &'static str,
    pub description: &'static str,
    pub remediation: &'static str,
}

/// The static catalog: every detection code damsel knows.
pub fn catalog() -> &'static [CatalogEntry] {
    &[
        CatalogEntry {
            code: "ANTI_DEBUG_PTRACE",
            description: "ptrace(PT_DENY_ATTACH) or PTRACE_TRACEME — debugger denial",
            remediation: "Patch the ptrace call to NOP or intercept ptrace in a debugger script",
        },
        CatalogEntry {
            code: "ANTI_DEBUG_SYSCTL",
            description: "sysctl kern.proc or P_TRACED check — debugger detection via kernel info",
            remediation: "Hook sysctl return or patch the flag comparison",
        },
        CatalogEntry {
            code: "ANTI_DEBUG_EXCEPTION_PORTS",
            description: "task_get_exception_ports — checks for attached debuggers/exception handlers",
            remediation: "Patch the call or fake the return value",
        },
        CatalogEntry {
            code: "ANTI_DEBUG_TIMING",
            description: "Timing check (rdtsc/mach_absolute_time) — detects single-stepping",
            remediation: "Freeze the clock source or patch the comparison",
        },
        CatalogEntry {
            code: "ANTI_VM_STRING",
            description: "VM indicator string (vmware/vbox/qemu/xen) in the binary",
            remediation: "Patch the string or the comparison using it",
        },
        CatalogEntry {
            code: "ANTI_VM_CPUID",
            description: "cpuid hypervisor-leaf check (leaf 1 ECX bit 31 or explicit leaf 0x40000000)",
            remediation: "Patch the bit test or intercept cpuid",
        },
        CatalogEntry {
            code: "PACKER_UPX",
            description: "UPX packed section (magic string or section entropy)",
            remediation: "Unpack with upx -d, or dump after decompression stub",
        },
        CatalogEntry {
            code: "STRING_ENCRYPTION",
            description: "XOR/add decode loop feeding API calls — strings decrypted at runtime",
            remediation: "Locate the decode function; dump the plaintext after it runs",
        },
        CatalogEntry {
            code: "CFG_FLATTENING",
            description: "Control-flow flattening — dispatcher loop with state variable",
            remediation: "Trace the state variable; symbolic execution recovers the original CFG",
        },
        CatalogEntry {
            code: "OPAQUE_PREDICATE",
            description: "Constant-condition branch — always taken or never taken",
            remediation: "The branch target is dead code; fold the condition manually",
        },
        CatalogEntry {
            code: "JUNK_CODE",
            description: "Junk/dead code insertion — instructions with no side effect",
            remediation: "Ignore; the instructions do not affect program state",
        },
        CatalogEntry {
            code: "SYSCALL_STUB",
            description: "Raw syscall instruction (not via libc) — direct kernel interface",
            remediation: "Check the syscall number against the platform table",
        },
        CatalogEntry {
            code: "DYLD_CACHE_ABUSE",
            description: "dyld shared-cache address referenced as code cave or trampoline",
            remediation: "The system image is read-only; treat as a call-through",
        },
    ]
}

/// Look up a catalog entry by code.
pub fn lookup(code: &str) -> Option<&'static CatalogEntry> {
    catalog().iter().find(|e| e.code == code)
}

// ---------------------------------------------------------------------------
// Import-based scanners (anti-debug, anti-VM)
// ---------------------------------------------------------------------------

/// Scan imports for anti-debug and anti-VM API usage.
pub fn scan_imports(imports: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();

    for import in imports {
        let lower = import.to_ascii_lowercase();

        if lower.contains("ptrace") {
            detections.push(Detection {
                code: "ANTI_DEBUG_PTRACE",
                location: format!("import:{import}"),
                evidence: format!("imports {import}"),
                severity: Severity::Medium,
                confidence: Confidence::Low,
            });
        }
        if lower.contains("sysctl") {
            detections.push(Detection {
                code: "ANTI_DEBUG_SYSCTL",
                location: format!("import:{import}"),
                evidence: format!("imports {import}"),
                severity: Severity::Medium,
                confidence: Confidence::Low,
            });
        }
        if lower.contains("task_get_exception_ports") {
            detections.push(Detection {
                code: "ANTI_DEBUG_EXCEPTION_PORTS",
                location: format!("import:{import}"),
                evidence: format!("imports {import}"),
                severity: Severity::Medium,
                confidence: Confidence::Medium,
            });
        }
        // Timing-check APIs: high-resolution clocks used to detect
        // single-stepping (elapsed time between two reads).
        if lower.contains("mach_absolute_time") || lower.contains("clock_gettime") {
            detections.push(Detection {
                code: "ANTI_DEBUG_TIMING",
                location: format!("import:{import}"),
                evidence: format!("imports {import} (high-resolution clock)"),
                severity: Severity::Info,
                confidence: Confidence::Low,
            });
        }
    }

    detections
}

/// Scan strings for VM indicators.
pub fn scan_strings(strings: &[String]) -> Vec<Detection> {
    let vm_indicators = ["vmware", "vbox", "virtualbox", "qemu", "xen", "hyperv"];
    let mut detections = Vec::new();

    for string in strings {
        let lower = string.to_ascii_lowercase();
        if vm_indicators.iter().any(|v| lower.contains(v)) {
            detections.push(Detection {
                code: "ANTI_VM_STRING",
                location: format!("string:{string}"),
                evidence: format!("contains VM indicator in {string:?}"),
                severity: Severity::Info,
                confidence: Confidence::Low,
            });
        }
    }

    detections
}

// ---------------------------------------------------------------------------
// Instruction-level scanners (need decoded instructions)
// ---------------------------------------------------------------------------

/// Evidence extracted from an instruction window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstructionWindow {
    /// Address of the anchor instruction.
    pub address: u64,
    /// Disassembly text of the surrounding window (3–5 instructions).
    pub text: Vec<String>,
}

/// Scan decoded instruction mnemonics for syscall stubs and timing checks.
///
/// `mnemonics` is (address, mnemonic) pairs in program order.
pub fn scan_instructions(mnemonics: &[(u64, String)]) -> Vec<Detection> {
    let mut detections = Vec::new();

    for &(address, ref mnemonic) in mnemonics {
        let lower = mnemonic.to_ascii_lowercase();

        if lower == "syscall" {
            detections.push(Detection {
                code: "SYSCALL_STUB",
                location: format!("{address:#x}"),
                evidence: format!("raw {mnemonic} instruction at {address:#x}"),
                severity: Severity::Info,
                confidence: Confidence::Medium,
            });
        }
        if lower == "rdtsc" || lower == "rdtscp" {
            detections.push(Detection {
                code: "ANTI_DEBUG_TIMING",
                location: format!("{address:#x}"),
                evidence: format!("timestamp read ({mnemonic}) at {address:#x}"),
                severity: Severity::Info,
                confidence: Confidence::Low,
            });
        }
        if lower == "cpuid" {
            // Check if the preceding instructions suggest a hypervisor check
            // (leaf 0x40000000 in eax before cpuid).
            detections.push(Detection {
                code: "ANTI_VM_CPUID",
                location: format!("{address:#x}"),
                evidence: format!("cpuid at {address:#x} (check for hypervisor leaf)"),
                severity: Severity::Info,
                confidence: Confidence::Low,
            });
        }
    }

    detections
}

// ---------------------------------------------------------------------------
// Structural scanners (decompiler IR level)
// ---------------------------------------------------------------------------

/// Scan IR-level indicators (re-exported predicates for the catalog).
/// Callers that have decompiler IR should run the structural detectors
/// and convert their findings into catalog detections.
pub fn structural_detection(
    code: &'static str,
    location: String,
    evidence: String,
    severity: Severity,
    confidence: Confidence,
) -> Detection {
    Detection { code, location, evidence, severity, confidence }
}


/// Scan arm64 code bytes for the XOR-decode-loop pattern:
/// ldrb → eor → strb within a small window (byte-wise string
/// decryption). This is a raw-byte heuristic — the IR-level detector
/// (`detect_string_encryption`) provides richer evidence when a
/// project exists.
pub fn scan_code_bytes_for_xor_loop(code: &[u8]) -> Vec<Detection> {
    let mut detections = Vec::new();
    if code.len() < 12 {
        return detections;
    }

    // arm64 32-bit little-endian instruction words.
    let words: Vec<(usize, u32)> = code
        .chunks_exact(4)
        .enumerate()
        .map(|(i, c)| (i * 4, u32::from_le_bytes([c[0], c[1], c[2], c[3]])))
        .collect();

    for window in words.windows(7) {
        let has_ldrb = window.iter().any(|(_, w)| (w & 0xFFC00000) == 0x39400000);
        let has_eor = window.iter().any(|(_, w)| (w & 0xFF200000) == 0x4A000000);
        let has_strb = window.iter().any(|(_, w)| (w & 0xFFC00000) == 0x39000000);
        let has_loop_back = window.iter().any(|(_, w)| {
            // b (unconditional) with negative offset (loop back)
            (w & 0xFC000000) == 0x14000000 && (w & 0x03FFFFFF) >= 0x03000000
        });

        if has_ldrb && has_eor && has_strb {
            let addr = window[0].0;
            // Only report once per pattern (skip adjacent duplicates)
            let is_duplicate = detections.last().is_some_and(|d: &Detection| {
                usize::from_str_radix(d.location.trim_start_matches("0x"), 16)
                    .is_ok_and(|prev| addr <= prev + 24)
            });
            if !is_duplicate {
                detections.push(Detection {
                    code: "STRING_ENCRYPTION",
                    location: format!("{addr:#x}"),
                    evidence: format!(
                        "ldrb/eor/strb XOR-decode loop at byte offset {addr:#x}{}",
                        if has_loop_back { " (with back-edge)" } else { "" }
                    ),
                    severity: Severity::Medium,
                    confidence: Confidence::Medium,
                });
            }
        }
    }

    detections
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

/// Render a detection set as a Markdown report.
pub fn render_markdown(detections: &[Detection]) -> String {
    if detections.is_empty() {
        return "# Anti-Analysis Scan\n\nNo defense indicators detected.\n".to_string();
    }

    let mut out = String::from("# Anti-Analysis Scan\n\n");
    out.push_str(&format!("{} detection(s) indicated:\n\n", detections.len()));
    out.push_str("| Code | Severity | Confidence | Location | Evidence |\n");
    out.push_str("|---|---|---|---|---|\n");
    for d in detections {
        let sev = format!("{:?}", d.severity).to_lowercase();
        let conf = format!("{:?}", d.confidence).to_lowercase();
        let entry = lookup(d.code);
        out.push_str(&format!(
            "| `{}` | {} | {} | `{}` | {} |\n",
            d.code, sev, conf, d.location, d.evidence
        ));
        if let Some(e) = entry {
            out.push_str(&format!(
                "|  | | | | → {} |\n",
                e.remediation
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_entries() {
        assert!(catalog().len() >= 13);
        assert!(lookup("ANTI_DEBUG_PTRACE").is_some());
        assert!(lookup("NONEXISTENT").is_none());
    }

    #[test]
    fn scan_imports_ptrace() {
        let dets = scan_imports(&["_ptrace".to_string(), "printf".to_string()]);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].code, "ANTI_DEBUG_PTRACE");
        assert_eq!(dets[0].severity, Severity::Medium);
    }

    #[test]
    fn scan_imports_exception_ports() {
        let dets = scan_imports(&["_task_get_exception_ports".to_string()]);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].code, "ANTI_DEBUG_EXCEPTION_PORTS");
        assert_eq!(dets[0].confidence, Confidence::Medium);
    }

    #[test]
    fn scan_strings_vm() {
        let dets = scan_strings(&["VMware Tools".to_string(), "hello".to_string()]);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].code, "ANTI_VM_STRING");
    }

    #[test]
    fn scan_instructions_syscall() {
        let dets = scan_instructions(&[
            (0x1000, "mov".to_string()),
            (0x1004, "syscall".to_string()),
        ]);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].code, "SYSCALL_STUB");
        assert_eq!(dets[0].location, "0x1004");
    }

    #[test]
    fn scan_instructions_rdtsc() {
        let dets = scan_instructions(&[(0x2000, "rdtsc".to_string())]);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].code, "ANTI_DEBUG_TIMING");
    }

    #[test]
    fn markdown_renders() {
        let dets = scan_imports(&["_ptrace".to_string()]);
        let md = render_markdown(&dets);
        assert!(md.contains("ANTI_DEBUG_PTRACE"));
        assert!(md.contains("remediation") || md.contains("Patch"));
    }

    #[test]
    fn xor_loop_detected_in_arm64_bytes() {
        // ldrb w8,[x9]; eor w8,w8,w10; strb w8,[x9]; nop; nop; b back
        let code: Vec<u8> = []
            .into_iter()
            .chain(0x39400128u32.to_le_bytes()) // ldrb
            .chain(0x4a0a0108u32.to_le_bytes()) // eor
            .chain(0x39000128u32.to_le_bytes()) // strb
            .chain(0xd503201fu32.to_le_bytes()) // nop
            .chain(0xd503201fu32.to_le_bytes()) // nop
            .chain(0xd503201fu32.to_le_bytes()) // nop
            .chain(0x17fffff0u32.to_le_bytes()) // b (back)
            .collect();
        let dets = scan_code_bytes_for_xor_loop(&code);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].code, "STRING_ENCRYPTION");
        assert_eq!(dets[0].severity, Severity::Medium);
    }

    #[test]
    fn no_xor_loop_in_plain_code() {
        // Simple mov/ret sequence — no decode loop
        let code: Vec<u8> = []
            .into_iter()
            .chain(0x52800000u32.to_le_bytes()) // mov w0, #0
            .chain(0xd65f03c0u32.to_le_bytes()) // ret
            .collect();
        let dets = scan_code_bytes_for_xor_loop(&code);
        assert!(dets.is_empty());
    }

    #[test]
    fn markdown_empty() {
        let md = render_markdown(&[]);
        assert!(md.contains("No defense indicators"));
    }
}
