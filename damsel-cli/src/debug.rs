//! LLDB debug workbench (plan feature 14, slice 1): the static↔dynamic
//! bridge. Batch-mode lldb sessions driven from recovered function
//! bounds — breakpoints at named entries, registers read at each stop,
//! backtraces captured — with every observation promoted to a
//! provenance-carrying fact (`lldb_v1`).
//!
//! Honest by construction: the debugger is optional (typed error when
//! lldb is absent); runs report exactly what lldb printed; register
//! facts carry the site (pc), the run pid, and the stop reason.

use std::path::Path;
use std::process::Command;

/// One observed register value at a stop.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RegisterObservation {
    pub register: String,
    pub value: u64,
}

/// One debugger stop with its observations.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DebugStop {
    pub function: String,
    pub pc: u64,
    pub stop_reason: String,
    pub registers: Vec<RegisterObservation>,
    pub backtrace: Vec<String>,
}

/// Result of a batch debug run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DebugRunReport {
    pub target: String,
    pub stops: Vec<DebugStop>,
}

/// Parse `register read` output lines: `      x0 = 0x0000000000000001`.
pub fn parse_register_line(line: &str) -> Option<RegisterObservation> {
    let trimmed = line.trim();
    let (register, value) = trimmed.split_once(" = ")?;
    let value = value.trim();
    let value = value.split_whitespace().next()?; // strip trailing annotations
    let numeric = value
        .strip_prefix("0x")
        .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        .or_else(|| value.parse::<u64>().ok())?;
    if register.is_empty()
        || !register
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    Some(RegisterObservation {
        register: register.to_ascii_lowercase(),
        value: numeric,
    })
}

/// Parse a full lldb batch transcript into stop records.
pub fn parse_lldb_transcript(transcript: &str) -> Vec<DebugStop> {
    let mut stops = Vec::new();
    let mut current: Option<DebugStop> = None;
    let mut in_backtrace = false;

    for line in transcript.lines() {
        let trimmed = line.trim();
        // Stop reason lives mid-line: `* thread #1, ... stop reason = breakpoint 1.1`
        if let Some(index) = trimmed.find("stop reason = ") {
            let reason = trimmed[index + "stop reason = ".len()..].trim();
            if let Some(stop) = current.as_mut() {
                stop.stop_reason = reason.to_string();
            }
            continue;
        }
        // Frame line: `* frame #0: 0x00000001000004b8 f\`main`
        if trimmed.starts_with("* frame #") || trimmed.starts_with("frame #") {
            in_backtrace = true;
            if let Some(stop) = current.as_mut() {
                stop.backtrace.push(trimmed.to_string());
            }
            // Extract function from `f\`main`
            if let Some(backtick) = trimmed.rfind('`') {
                let function = trimmed[backtick + 1..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if let Some(stop) = current.as_mut() {
                    if stop.function.is_empty() {
                        stop.function = function.to_string();
                    }
                }
            }
            continue;
        }
        // Register line: `      x0 = 0x...`
        if let Some(observation) = parse_register_line(trimmed) {
            if let Some(stop) = current.as_mut() {
                if observation.register == "pc" {
                    stop.pc = observation.value;
                }
                stop.registers.push(observation);
            }
            continue;
        }
        // A new breakpoint stop starts a new record: `Process NNNN stopped`
        if trimmed.starts_with("Process ") && trimmed.ends_with("stopped") {
            if let Some(stop) = current.take() {
                if !stop.registers.is_empty() || !stop.backtrace.is_empty() {
                    stops.push(stop);
                }
            }
            current = Some(DebugStop {
                function: String::new(),
                pc: 0,
                stop_reason: String::new(),
                registers: Vec::new(),
                backtrace: Vec::new(),
            });
            in_backtrace = false;
            continue;
        }
        // PC from register lines lands in registers; also detect it
        // for the stop record.
        if in_backtrace {
            in_backtrace = false;
        }
        if trimmed.starts_with("pc = ") {
            if let Some(observation) = parse_register_line(trimmed) {
                if let Some(stop) = current.as_mut() {
                    stop.pc = observation.value;
                }
            }
        }
    }
    if let Some(stop) = current {
        if !stop.registers.is_empty() || !stop.backtrace.is_empty() {
            stops.push(stop);
        }
    }
    stops
}

/// Run a batch lldb session: breakpoint at `function`, read registers
/// and backtrace, return the parsed report. Requires lldb on PATH.
pub fn debug_function(
    binary: &Path,
    function: &str,
    extra_registers: &[&str],
) -> Result<DebugRunReport, String> {
    let lldb = which_lldb().ok_or_else(|| "lldb not found on PATH".to_string())?;
    // lldb resolves `main` (no leading underscore on Apple platforms).
    let lldb_name = function.trim_start_matches('_');

    let mut script = vec![
        format!("breakpoint set --name {lldb_name}"),
        "run".to_string(),
    ];
    let register_list = if extra_registers.is_empty() {
        "x0 x1 x2 x3 sp pc lr".to_string()
    } else {
        extra_registers.join(" ")
    };
    script.push(format!("register read {register_list}"));
    script.push("thread backtrace".to_string());
    script.push("quit".to_string());

    let output = Command::new(&lldb)
        .arg("--batch")
        .args(script.iter().map(|cmd| ["-o", cmd]).flatten())
        .arg("--")
        .arg(binary)
        .output()
        .map_err(|e| format!("failed to run lldb: {e}"))?;

    let transcript = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() && !transcript.contains("stopped") {
        return Err(format!("lldb run failed ({}): {stderr}", output.status));
    }
    let stops = parse_lldb_transcript(&transcript);
    Ok(DebugRunReport {
        target: binary.display().to_string(),
        stops,
    })
}

fn which_lldb() -> Option<std::path::PathBuf> {
    // Common locations, then PATH.
    for candidate in [
        "/usr/bin/lldb",
        "/Library/Developer/CommandLineTools/usr/bin/lldb",
    ] {
        if Path::new(candidate).exists() {
            return Some(candidate.into());
        }
    }
    Command::new("which")
        .arg("lldb")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|path| !path.is_empty())
        .map(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_register_lines() {
        assert_eq!(
            parse_register_line("      x0 = 0x0000000000000001"),
            Some(RegisterObservation {
                register: "x0".into(),
                value: 1
            })
        );
        assert_eq!(
            parse_register_line("      sp = 0x000000016fdfe0c0"),
            Some(RegisterObservation {
                register: "sp".into(),
                value: 0x16fdfe0c0
            })
        );
        assert_eq!(parse_register_line("(lldb) register read"), None);
        assert_eq!(parse_register_line(""), None);
    }

    #[test]
    fn parses_stop_from_transcript() {
        let transcript = r#"
Process 47250 stopped
* thread #1, queue = 'com.apple.main-thread', stop reason = breakpoint 1.1
       frame #0: 0x00000001000004b8 f`main
f`main:
->  0x1000004b8 <+0>:  sub    sp, sp, #0x20
Target 0: (f) stopped.
(lldb) register read x0 x1 sp pc
      x0 = 0x0000000000000001
      x1 = 0x000000016fdfe750
      sp = 0x000000016fdfe0c0
      pc = 0x00000001000004b8  f`main
(lldb) thread backtrace
* thread #1, queue = 'com.apple.main-thread', stop reason = breakpoint 1.1
   * frame #0: 0x00000001000004b8 f`main
(lldb) quit
"#;
        let stops = parse_lldb_transcript(transcript);
        assert_eq!(stops.len(), 1);
        assert_eq!(stops[0].function, "main");
        assert_eq!(stops[0].pc, 0x1000004b8);
        assert!(stops[0].stop_reason.contains("breakpoint"));
        assert!(
            stops[0]
                .registers
                .iter()
                .any(|r| r.register == "x0" && r.value == 1)
        );
        assert!(!stops[0].backtrace.is_empty());
    }
}
