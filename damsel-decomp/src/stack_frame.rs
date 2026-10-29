//! Stack-frame facts (Wave 6; strategy §Stack Frame Recovery).
//!
//! v1 derives frame facts from lifted IR: frame size from explicit
//! `sub/add sp, sp, #N` pairs, saved registers from stores of fp/lr/
//! callee-saved registers to sp/fp-based slots, local slots from other
//! fp/sp-based accesses. Writes to non-callee-saved registers and
//! patterns outside the model are simply not reported — never guessed.

use crate::ir::{IrBinOp, IrExpr, IrFunction, IrStatement, IrValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackFrameFacts {
    /// `sub sp, sp, #N` in the prologue (frame allocation).
    pub frame_size: Option<u64>,
    /// Matching `add sp, sp, #N` in the epilogue when present.
    pub epilogue_restore: Option<u64>,
    /// Saved registers (fp/lr/x29/x30/x19..x28) stored to sp/fp slots.
    pub saved_registers: Vec<String>,
    /// Spilled/local slot displacements (fp-negative or sp-relative).
    pub local_slots: Vec<i64>,
}

pub fn analyze_stack_frame(function: &IrFunction) -> StackFrameFacts {
    let mut frame_size = None;
    let mut epilogue_restore = None;
    let mut saved: BTreeSet<String> = BTreeSet::new();
    let mut locals: BTreeSet<i64> = BTreeSet::new();

    for block in &function.blocks {
        for statement in &block.statements {
            match statement {
                IrStatement::Assign { destination, expression } => {
                    if destination != "sp" && destination != "x31" {
                        continue;
                    }
                    if let IrExpr::BinOp { op, lhs, rhs } = expression {
                        if matches!(
                            (&**lhs, &**rhs),
                            (IrExpr::Value(IrValue::Register(r)), IrExpr::Value(IrValue::Const(_)))
                                if r == "sp" || r == "x31"
                        ) {
                            let IrExpr::Value(IrValue::Const(amount)) = &**rhs else {
                                continue;
                            };
                            match op {
                                IrBinOp::Sub => frame_size = Some(*amount),
                                IrBinOp::Add => epilogue_restore = Some(*amount),
                                _ => {}
                            }
                        }
                    }
                }
                IrStatement::Store { address, value } => {
                    let Some(IrExpr::AddrOf { base, displacement }) = addr_of(address) else {
                        continue;
                    };
                    if !is_frame_base(base) {
                        continue;
                    }
                    if let IrExpr::Value(IrValue::Register(register)) = value {
                        if is_callee_saved(register) {
                            saved.insert(register.clone());
                            continue;
                        }
                    }
                    locals.insert(*displacement);
                }
                IrStatement::Load { address, .. } => {
                    let Some(IrExpr::AddrOf { base, displacement }) = addr_of(address) else {
                        continue;
                    };
                    if is_frame_base(base) {
                        locals.insert(*displacement);
                    }
                }
                _ => {}
            }
        }
    }

    StackFrameFacts {
        frame_size,
        epilogue_restore,
        saved_registers: saved.into_iter().collect(),
        local_slots: locals.into_iter().collect(),
    }
}

fn addr_of(expression: &IrExpr) -> Option<&IrExpr> {
    match expression {
        IrExpr::AddrOf { .. } => Some(expression),
        _ => None,
    }
}

fn is_frame_base(base: &str) -> bool {
    matches!(base, "sp" | "x31" | "fp" | "x29")
}

fn is_callee_saved(register: &str) -> bool {
    matches!(register, "fp" | "lr" | "x29" | "x30")
        || matches!(
            register,
            "x19" | "x20" | "x21" | "x22" | "x23" | "x24" | "x25" | "x26" | "x27" | "x28"
        )
}
