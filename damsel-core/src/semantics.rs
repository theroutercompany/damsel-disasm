//! ISA-neutral instruction semantics (RE Power Wave 5A step 1).
//!
//! v1 ports the engine's arm64 control-flow classifier predicates from
//! `damsel-macho/src/disasm.rs` verbatim into the shared core so both the
//! engine and the future x86_64 producer speak one vocabulary. The engine
//! delegates to these functions; behavior is identical and the existing
//! test matrices are the differential oracle (Wave 5 spec §5A.2).
//!
//! Precedence is part of the contract (mirrors `terminator_kind`):
//! Return → Trap → conditional branch → indirect branch (`br` family) →
//! unconditional transfer (`b`, `eret`) → indirect call (`blr` family) →
//! call (`bl`).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlFlowClass {
    /// `bl` — direct call.
    DirectCall,
    /// `blr`/`blraa`/… — indirect call through a register.
    IndirectCall,
    /// `b`, `eret` — unconditional direct transfer.
    DirectBranch,
    /// `b.cond`, `bc.*`, `cbz`/`cbnz`, `tbz`/`tbnz` — conditional.
    ConditionalBranch,
    /// `br`/`braa`/… — indirect branch through a register.
    IndirectBranch,
    /// `ret`/`retaa`/`retab`/`eret*`/`drps`.
    Return,
    /// `brk`/`hlt`/`udf`.
    Trap,
    /// No control-flow effect (v1 scope: control flow only).
    Other,
}

pub mod arm64 {
    //! Arm64 (arm64/arm64e) control-flow classification.
    //!
    //! Inputs are expected pre-lowercased, matching engine call sites
    //! (`mnemonic.to_ascii_lowercase()`).

    use super::ControlFlowClass;

    pub fn is_return(lower: &str) -> bool {
        matches!(
            lower,
            "ret" | "retaa" | "retab" | "eret" | "eretaa" | "eretab" | "drps"
        )
    }

    pub fn is_trap(lower: &str) -> bool {
        matches!(lower, "brk" | "hlt" | "udf")
    }

    pub fn is_conditional_branch(lower: &str) -> bool {
        lower.starts_with("b.")
            || lower.starts_with("bc.")
            || matches!(lower, "cbz" | "cbnz" | "tbz" | "tbnz")
    }

    pub fn is_indirect_branch(lower: &str) -> bool {
        matches!(lower, "br" | "braa" | "braaz" | "brab" | "brabz")
    }

    pub fn is_unconditional_transfer(lower: &str) -> bool {
        matches!(
            lower,
            "b" | "br" | "braa" | "braaz" | "brab" | "brabz" | "eret"
        )
    }

    pub fn is_indirect_call(lower: &str) -> bool {
        matches!(
            lower,
            "blr" | "blraa" | "blraaz" | "blrab" | "blrabz"
        )
    }

    pub fn is_call(lower: &str) -> bool {
        matches!(lower, "bl") || is_indirect_call(lower)
    }

    /// Classify with the documented precedence.
    pub fn classify(lower: &str) -> ControlFlowClass {
        if is_return(lower) {
            ControlFlowClass::Return
        } else if is_trap(lower) {
            ControlFlowClass::Trap
        } else if is_conditional_branch(lower) {
            ControlFlowClass::ConditionalBranch
        } else if is_indirect_branch(lower) {
            ControlFlowClass::IndirectBranch
        } else if is_unconditional_transfer(lower) {
            ControlFlowClass::DirectBranch
        } else if is_indirect_call(lower) {
            ControlFlowClass::IndirectCall
        } else if matches!(lower, "bl") {
            ControlFlowClass::DirectCall
        } else {
            ControlFlowClass::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::arm64;
    use super::ControlFlowClass::*;

    #[test]
    fn classifies_each_family() {
        assert_eq!(arm64::classify("bl"), DirectCall);
        assert_eq!(arm64::classify("blr"), IndirectCall);
        assert_eq!(arm64::classify("blraaz"), IndirectCall);
        assert_eq!(arm64::classify("b"), DirectBranch);
        assert_eq!(arm64::classify("b.eq"), ConditionalBranch);
        assert_eq!(arm64::classify("bc.eq"), ConditionalBranch);
        assert_eq!(arm64::classify("cbz"), ConditionalBranch);
        assert_eq!(arm64::classify("tbnz"), ConditionalBranch);
        assert_eq!(arm64::classify("br"), IndirectBranch);
        assert_eq!(arm64::classify("brabz"), IndirectBranch);
        assert_eq!(arm64::classify("ret"), Return);
        assert_eq!(arm64::classify("retab"), Return);
        assert_eq!(arm64::classify("brk"), Trap);
        assert_eq!(arm64::classify("udf"), Trap);
        assert_eq!(arm64::classify("add"), Other);
        assert_eq!(arm64::classify("stp"), Other);
    }

    #[test]
    fn precedence_matches_engine_terminator_order() {
        // `eret` is both a return and an unconditional transfer: Return
        // wins, exactly like the legacy terminator_kind ordering.
        assert_eq!(arm64::classify("eret"), Return);
        // `br` is both an indirect branch and an unconditional transfer:
        // IndirectBranch wins.
        assert_eq!(arm64::classify("br"), IndirectBranch);
        // `blr` is both an indirect call and (historically) in the call
        // family: IndirectCall wins.
        assert_eq!(arm64::classify("blr"), IndirectCall);
        // `b.ne` must not be misread as unconditional `b`.
        assert_eq!(arm64::classify("b.ne"), ConditionalBranch);
    }

    #[test]
    fn helper_predicates_agree_with_legacy_sets() {
        for m in ["ret", "retaa", "retab", "eret", "eretaa", "eretab", "drps"] {
            assert!(arm64::is_return(m), "{m} must be return");
        }
        for m in ["b", "br", "braa", "braaz", "brab", "brabz", "eret"] {
            assert!(arm64::is_unconditional_transfer(m), "{m} must be transfer");
        }
        for m in ["bl", "blr", "blraa", "blraaz", "blrab", "blrabz"] {
            assert!(arm64::is_call(m), "{m} must be call");
        }
        for m in ["cbz", "cbnz", "tbz", "tbnz", "b.lt", "bc.ge"] {
            assert!(arm64::is_conditional_branch(m), "{m} must be conditional");
        }
        // Negative spot checks.
        assert!(!arm64::is_conditional_branch("bl"));
        assert!(!arm64::is_conditional_branch("b"));
        assert!(!arm64::is_return("brk"));
    }
}
