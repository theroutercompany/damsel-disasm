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

    /// True for any instruction that ends a basic block (conditional
    /// branch, unconditional transfer, return, or trap).
    pub fn is_cfg_boundary(lower: &str) -> bool {
        is_conditional_branch(lower)
            || is_unconditional_transfer(lower)
            || is_return(lower)
            || is_trap(lower)
    }

    /// Indirect-control display kind used by the engine's summaries:
    /// authenticated call/branch (PAC), plain call, or branch.
    pub fn indirect_control_kind(lower: &str) -> &'static str {
        if matches!(lower, "blraa" | "blraaz" | "blrab" | "blrabz") {
            "authenticated-call"
        } else if matches!(lower, "braa" | "braaz" | "brab" | "brabz") {
            "authenticated-branch"
        } else if is_call(lower) {
            "call"
        } else {
            "branch"
        }
    }

    /// Destination-register analysis (ported verbatim from the engine;
    /// Wave 5 spec §5A.2 step c). Operands come from the decoded
    /// instruction, so these live here in damsel-core where both the
    /// engine and future producers share them.

    use crate::model::{DecodedInstruction, Operand};

    pub fn register_operand_name(operand: &Operand) -> Option<String> {
        match operand {
            Operand::Register(register) => Some(register.clone()),
            Operand::QualifiedRegister { register, .. } => Some(register.clone()),
            Operand::SystemRegister(register) => Some(register.clone()),
            _ => None,
        }
    }

    pub fn destination_register(instruction: &DecodedInstruction) -> Option<String> {
        register_operand_name(instruction.operands.first()?)
    }

    pub fn store_exclusive_writes_status(lower: &str) -> bool {
        matches!(
            lower,
            "stxr" | "stxrb" | "stxrh" | "stlxr" | "stlxrb" | "stlxrh" | "stxp" | "stlxp"
        )
    }

    pub fn atomic_read_modify_write_writes_second_operand(lower: &str) -> bool {
        [
            "swp", "ldadd", "ldclr", "ldeor", "ldset", "ldsmax", "ldsmin", "ldumax", "ldumin",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    }

    pub fn atomic_pair_compare_swap_writes_first_two_operands(lower: &str) -> bool {
        matches!(lower, "casp" | "caspa" | "caspl" | "caspal")
    }

    pub fn load_pair_writes_two_destinations(lower: &str) -> bool {
        lower.starts_with("ldp")
            || lower.starts_with("ldnp")
            || matches!(lower, "ldxp" | "ldaxp")
    }

    /// True when the instruction writes its destination register (the
    /// engine's `instruction_writes_destination`, verbatim).
    pub fn instruction_writes_destination(lower: &str) -> bool {
        if store_exclusive_writes_status(lower) {
            return true;
        }
        !(lower == "cmp"
            || lower == "cmn"
            || lower == "ccmp"
            || lower == "ccmn"
            || lower == "tst"
            || is_conditional_branch(lower)
            || is_trap(lower)
            || lower.starts_with("st")
            || is_unconditional_transfer(lower)
            || is_call(lower)
            || is_return(lower))
    }

    /// Registers written by the instruction, in operand order (the
    /// engine's `written_destination_registers`, verbatim).
    pub fn written_destination_registers(
        instruction: &DecodedInstruction,
        lower: &str,
    ) -> Vec<String> {
        if !instruction_writes_destination(lower) {
            return Vec::new();
        }
        if atomic_read_modify_write_writes_second_operand(lower) {
            return instruction
                .operands
                .iter()
                .skip(1)
                .take(1)
                .filter_map(register_operand_name)
                .collect();
        }
        if atomic_pair_compare_swap_writes_first_two_operands(lower)
            || load_pair_writes_two_destinations(lower)
        {
            return instruction
                .operands
                .iter()
                .take(2)
                .filter_map(register_operand_name)
                .collect();
        }
        destination_register(instruction).into_iter().collect()
    }

    /// Multiply-family mnemonics (v1 set for the value-flow dispatch
    /// port; full MRS/SIMD families land with their own slices).
    pub fn is_multiply(lower: &str) -> bool {
        matches!(lower, "mul" | "madd" | "msub" | "smull" | "umull" | "smulh" | "umulh")
            || lower.starts_with("mul")
    }

    /// Divide mnemonics.
    pub fn is_divide(lower: &str) -> bool {
        matches!(lower, "sdiv" | "udiv")
    }

    /// Extend mnemonics (sign/zero extend from narrow registers).
    pub fn is_extend(lower: &str) -> bool {
        matches!(
            lower,
            "sxtb" | "sxth" | "sxtw" | "uxtb" | "uxth" | "uxtw"
        )
    }

    /// Shift mnemonics beyond lsl/lsr (explicit register variants).
    pub fn is_shift(lower: &str) -> bool {
        matches!(lower, "asr" | "lsl" | "lsr" | "ror" | "asl")
    }

    /// Reverse bit/byte mnemonics.
    pub fn is_reverse(lower: &str) -> bool {
        matches!(lower, "rev" | "rev16" | "rev32" | "rev64" | "rbit")
    }

    /// Count-zeroes / count-leading-signs family.
    pub fn is_count(lower: &str) -> bool {
        matches!(lower, "clz" | "cls" | "ctz")
    }

    /// Prefetch mnemonics.
    pub fn is_prefetch(lower: &str) -> bool {
        matches!(lower, "prfm" | "prfum")
    }

    /// Atomic compare-and-swap mnemonics.
    pub fn is_atomic_compare_swap(lower: &str) -> bool {
        matches!(
            lower,
            "cas" | "casa" | "casl" | "casal" | "casb" | "casab" | "caslb" | "casalb"
                | "cash" | "casah" | "caslh" | "casalh"
        ) || is_atomic_pair_compare_swap(lower)
    }

    /// Pair compare-and-swap (casp family — writes two destinations).
    pub fn is_atomic_pair_compare_swap(lower: &str) -> bool {
        matches!(lower, "casp" | "caspa" | "caspl" | "caspal")
    }

    /// Store-pair mnemonics (write two memory slots).
    pub fn is_store_pair(lower: &str) -> bool {
        lower.starts_with("stp")
            || lower.starts_with("stnp")
            || matches!(lower, "stxp" | "stlxp")
    }

    /// Load mnemonics that read the destination register's current value
    /// (for value-flow: ldr/ldur/ldtr/ldar family).
    pub fn load_reads_destination(lower: &str) -> bool {
        lower.starts_with("ldr")
            || lower.starts_with("ldur")
            || lower.starts_with("ldtr")
            || lower.starts_with("ldar")
    }

    /// Load mnemonics that read a relative table offset (literal loads).
    pub fn load_reads_relative_table(lower: &str) -> bool {
        matches!(lower, "ldr" | "ldur") || lower.starts_with("ldr")
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

pub mod x86_64 {
    //! x86-64 (SysV) control-flow classification (RE Power Wave 5A.3).
    //!
    //! Mnemonic-only v1: x86 directness (immediate vs register/memory
    //! target) is an operand property, so callers pass `indirect` from
    //! decoder detail — exactly the split the spike validated (capstone
    //! x86 exposes RIP-relative bases and implicit regs).
    use super::ControlFlowClass;

    /// Indirectness of the transfer target, from decoder operand detail.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Transfer {
        /// Immediate/PC-relative target.
        Direct,
        /// Register or memory target.
        Indirect,
    }

    pub fn is_return(lower: &str) -> bool {
        matches!(lower, "ret" | "retf" | "iretd" | "iretq" | "iretw" | "ud1")
            || lower.starts_with("ret")
    }

    pub fn is_trap(lower: &str) -> bool {
        matches!(lower, "int3" | "int1" | "into" | "ud2" | "hlt" | "bnd")
            || lower.starts_with("int ")
    }

    pub fn is_conditional_branch(lower: &str) -> bool {
        const SET: &[&str] = &[
            "jo", "jno", "js", "jns", "je", "jz", "jne", "jnz", "jb", "jc", "jnae",
            "jnb", "jnc", "jae", "jbe", "jna", "ja", "jnbe", "jl", "jnge", "jge",
            "jnl", "jle", "jng", "jg", "jnle", "jp", "jpe", "jnp", "jpo", "jcxz",
            "jecxz", "jrcxz", "loop", "loope", "loopne", "loopz", "loopnz",
        ];
        SET.contains(&lower)
    }

    pub fn is_unconditional_transfer(lower: &str) -> bool {
        matches!(lower, "jmp" | "ljmp")
    }

    /// Classify with target transfer kind. `syscall` is not a terminator
    /// (it returns to the caller) — v1 keeps it `Other`.
    pub fn classify(lower: &str, transfer: Transfer) -> ControlFlowClass {
        if is_return(lower) {
            ControlFlowClass::Return
        } else if is_trap(lower) {
            ControlFlowClass::Trap
        } else if matches!(lower, "call" | "lcall") {
            match transfer {
                Transfer::Direct => ControlFlowClass::DirectCall,
                Transfer::Indirect => ControlFlowClass::IndirectCall,
            }
        } else if is_unconditional_transfer(lower) {
            match transfer {
                Transfer::Direct => ControlFlowClass::DirectBranch,
                Transfer::Indirect => ControlFlowClass::IndirectBranch,
            }
        } else if is_conditional_branch(lower) {
            ControlFlowClass::ConditionalBranch
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

    mod x86 {
        use crate::semantics::x86_64::*;
        use crate::semantics::ControlFlowClass::*;

        #[test]
        fn classifies_control_flow_families() {
            assert_eq!(classify("call", Transfer::Direct), DirectCall);
            assert_eq!(classify("call", Transfer::Indirect), IndirectCall);
            assert_eq!(classify("jmp", Transfer::Direct), DirectBranch);
            assert_eq!(classify("jmp", Transfer::Indirect), IndirectBranch);
            assert_eq!(classify("jne", Transfer::Direct), ConditionalBranch);
            assert_eq!(classify("ret", Transfer::Direct), Return);
            assert_eq!(classify("retf", Transfer::Direct), Return);
            assert_eq!(classify("int3", Transfer::Direct), Trap);
            assert_eq!(classify("ud2", Transfer::Direct), Trap);
            assert_eq!(classify("syscall", Transfer::Direct), Other);
            assert_eq!(classify("endbr64", Transfer::Direct), Other);
            assert_eq!(classify("add", Transfer::Direct), Other);
            // Spike window instructions round-trip.
            assert_eq!(classify("push", Transfer::Direct), Other);
            assert_eq!(classify("lea", Transfer::Direct), Other);
        }

        #[test]
        fn conditional_family_is_complete_for_common_forms() {
            for m in ["je", "jnz", "jge", "ja", "jbe", "jrcxz", "loopne"] {
                assert!(is_conditional_branch(m), "{m}");
            }
            for m in ["jmp", "call", "ret", "add", "nop"] {
                assert!(!is_conditional_branch(m), "{m}");
            }
        }
    }

    #[test]
    fn register_write_analysis_matches_engine_tables() {
        use crate::model::{DecodedInstruction, Operand};
        use super::arm64;

        // cmp/tst/st*/branches write nothing.
        assert!(!arm64::instruction_writes_destination("cmp"));
        assert!(!arm64::instruction_writes_destination("tst"));
        assert!(!arm64::instruction_writes_destination("str"));
        assert!(!arm64::instruction_writes_destination("stp"));
        assert!(!arm64::instruction_writes_destination("bl"));
        assert!(!arm64::instruction_writes_destination("b.eq"));
        // store-exclusive writes its status register despite `st` prefix.
        assert!(arm64::instruction_writes_destination("stxr"));
        assert!(arm64::instruction_writes_destination("stxp"));
        assert!(arm64::instruction_writes_destination("add"));
        assert!(arm64::instruction_writes_destination("ldp"));

        let insn = |mnemonic: &str, operands: Vec<Operand>| DecodedInstruction {
            address: 0,
            size: 4,
            opcode: 0,
            mnemonic: mnemonic.to_string(),
            operands,
            recovered_values: Vec::new(),
            references: Vec::new(),
            annotations: Vec::new(),
        };
        // Single destination: add x0, x1, x2 → [x0].
        assert_eq!(
            arm64::written_destination_registers(
                &insn("add", vec![Operand::Register("x0".into()), Operand::Register("x1".into())]),
                "add"
            ),
            vec!["x0".to_string()]
        );
        // Pair load writes two destinations.
        assert_eq!(
            arm64::written_destination_registers(
                &insn("ldp", vec![
                    Operand::Register("x0".into()),
                    Operand::Register("x1".into()),
                    Operand::Memory { base: "sp".into(), index: None, index_modifier: None, displacement: 0, writeback: None },
                ]),
                "ldp"
            ),
            vec!["x0".to_string(), "x1".to_string()]
        );
        // Atomic RMW writes the second operand.
        assert_eq!(
            arm64::written_destination_registers(
                &insn("ldadd", vec![
                    Operand::Register("x0".into()),
                    Operand::Register("x1".into()),
                    Operand::Memory { base: "x2".into(), index: None, index_modifier: None, displacement: 0, writeback: None },
                ]),
                "ldadd"
            ),
            vec!["x1".to_string()]
        );
    }

    #[test]
    fn value_flow_families_classified() {
        assert!(arm64::is_multiply("mul"));
        assert!(arm64::is_multiply("smull"));
        assert!(!arm64::is_multiply("add"));
        assert!(arm64::is_divide("sdiv"));
        assert!(arm64::is_divide("udiv"));
        assert!(!arm64::is_divide("mul"));
        assert!(arm64::is_extend("sxtw"));
        assert!(arm64::is_extend("uxtb"));
        assert!(!arm64::is_extend("mov"));
        assert!(arm64::is_shift("asr"));
        assert!(arm64::is_shift("ror"));
        assert!(!arm64::is_shift("add"));
        assert!(arm64::is_reverse("rev"));
        assert!(arm64::is_reverse("rbit"));
        assert!(arm64::is_count("clz"));
        assert!(arm64::is_count("cls"));
        assert!(!arm64::is_count("mul"));
    }

    #[test]
    fn cfg_boundary_and_indirect_kind_match_engine_behavior() {
        for m in ["b", "b.eq", "br", "ret", "brk", "cbz", "eret"] {
            assert!(arm64::is_cfg_boundary(m), "{m} is a boundary");
        }
        for m in ["bl", "blr", "add", "nop", "str"] {
            assert!(!arm64::is_cfg_boundary(m), "{m} is not a boundary");
        }
        assert_eq!(arm64::indirect_control_kind("blraaz"), "authenticated-call");
        assert_eq!(arm64::indirect_control_kind("brab"), "authenticated-branch");
        assert_eq!(arm64::indirect_control_kind("blr"), "call");
        assert_eq!(arm64::indirect_control_kind("br"), "branch");
    }
}
