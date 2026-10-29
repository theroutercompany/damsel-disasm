//! D4 control-structure recovery v1 (Wave 6; strategy §Control
//! Structure Recovery).
//!
//! Scope (honest): resolve jcc conditions from the immediately
//! preceding synthetic `flags` assignment (`cmp` → relational, `test`
//! → zero/nonzero), and recognize **diamonds** — cond blocks whose two
//! arms reconverge — rendering them as if/else without gotos. Other
//! shapes stay in goto form (visible, awaiting later D4 slices).

use crate::ir::{IrBinOp, IrCondition, IrExpr, IrFunction, IrStatement, IrTerminator, IrValue};
use serde::{Deserialize, Serialize};

/// A resolved branch condition (or the raw mnemonic when the flags
/// assignment is missing — never invented).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolvedCondition {
    /// `a == b` / `a != b` / signed+unsigned relations after Sub.
    Relational {
        lhs: String,
        op: String,
        rhs: String,
    },
    /// `a & b` zero / nonzero after And (test).
    Mask {
        lhs: String,
        rhs: String,
        nonzero: bool,
    },
    /// Flags assignment not found in this block.
    Unresolved(String),
}

pub fn resolve_condition(function: &IrFunction, block_id: usize) -> ResolvedCondition {
    let Some(block) = function.blocks.iter().find(|b| b.id == block_id) else {
        return ResolvedCondition::Unresolved("block".to_string());
    };
    let IrTerminator::CondBranch { condition, .. } = &block.terminator else {
        return ResolvedCondition::Unresolved("not-cond".to_string());
    };
    // Find the last flags assignment in the block.
    let last_flags = block.statements.iter().rev().find(|s| {
        matches!(s, IrStatement::Assign { destination, .. } if destination == "flags")
    });
    let Some(IrStatement::Assign { expression, .. }) = last_flags else {
        return ResolvedCondition::Unresolved(condition_raw(condition));
    };
    let IrExpr::BinOp { op, lhs, rhs } = expression else {
        return ResolvedCondition::Unresolved(condition_raw(condition));
    };
    let (lhs, rhs) = (render_operand(lhs), render_operand(rhs));
    let mnemonic = condition_raw(condition);
    match (op, mnemonic.as_str()) {
        (IrBinOp::Sub, "je") => rel(lhs, "==", rhs),
        (IrBinOp::Sub, "jne") => rel(lhs, "!=", rhs),
        (IrBinOp::Sub, "jl") => rel(lhs, "<", rhs),
        (IrBinOp::Sub, "jle") => rel(lhs, "<=", rhs),
        (IrBinOp::Sub, "jg") => rel(lhs, ">", rhs),
        (IrBinOp::Sub, "jge") => rel(lhs, ">=", rhs),
        (IrBinOp::Sub, "jb") => rel(lhs, "<u", rhs),
        (IrBinOp::Sub, "jbe") => rel(lhs, "<=u", rhs),
        (IrBinOp::Sub, "ja") => rel(lhs, ">u", rhs),
        (IrBinOp::Sub, "jae") => rel(lhs, ">=u", rhs),
        (IrBinOp::And, "je") => ResolvedCondition::Mask { lhs, rhs, nonzero: false },
        (IrBinOp::And, "jne") => ResolvedCondition::Mask { lhs, rhs, nonzero: true },
        _ => ResolvedCondition::Unresolved(mnemonic),
    }
}

fn rel(lhs: String, op: &str, rhs: String) -> ResolvedCondition {
    ResolvedCondition::Relational { lhs, op: op.to_string(), rhs }
}

fn condition_raw(condition: &IrCondition) -> String {
    match condition {
        IrCondition::Raw(raw) => raw.clone(),
    }
}

fn render_operand(expr: &IrExpr) -> String {
    match expr {
        IrExpr::Value(IrValue::Register(register)) => register.clone(),
        IrExpr::Value(IrValue::Const(value)) => format!("{value:#x}"),
        _ => "?".to_string(),
    }
}

/// A recognized diamond: cond block, then-arm, else-arm, merge address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diamond {
    pub cond_block: usize,
    pub condition: ResolvedCondition,
    pub then_block: usize,
    pub else_block: usize,
    pub merge_address: u64,
}

/// Detect diamonds: cond → (taken, fallthrough); both arms end in
/// Branch(M) with the same M (or WindowEnd falling to M).
pub fn find_diamonds(function: &IrFunction) -> Vec<Diamond> {
    let block_at: std::collections::BTreeMap<u64, usize> = function
        .blocks
        .iter()
        .map(|b| (b.start_address, b.id))
        .collect();
    let block = |id: usize| function.blocks.iter().find(|b| b.id == id);

    let mut out = Vec::new();
    for cond in &function.blocks {
        let IrTerminator::CondBranch { taken, fallthrough, .. } = cond.terminator else {
            continue;
        };
        let (Some(then_id), Some(else_id)) = (block_at.get(&taken), block_at.get(&fallthrough))
        else {
            continue;
        };
        let (Some(then_block), Some(else_block)) = (block(*then_id), block(*else_id)) else {
            continue;
        };
        // Diamond: both arms jump to one common merge address.
        let merge_of = |b: &crate::ir::IrBlock| -> Option<u64> {
            match &b.terminator {
                IrTerminator::Branch(target) => Some(*target),
                // An arm ending in return is an open diamond: merge is
                // the other arm's target only if it equals after-return
                // padding — too speculative; require explicit branches.
                _ => None,
            }
        };
        let (Some(merge_then), Some(merge_else)) = (merge_of(then_block), merge_of(else_block))
        else {
            continue;
        };
        if merge_then != merge_else || merge_then == taken || merge_then == fallthrough {
            continue;
        }
        out.push(Diamond {
            cond_block: cond.id,
            condition: resolve_condition(function, cond.id),
            then_block: *then_id,
            else_block: *else_id,
            merge_address: merge_then,
        });
    }
    out
}

/// Render a resolved condition as pseudocode text.
pub fn render_condition_text(condition: &ResolvedCondition) -> String {
    match condition {
        ResolvedCondition::Relational { lhs, op, rhs } => format!("{lhs} {op} {rhs}"),
        ResolvedCondition::Mask { lhs, rhs, nonzero } => {
            if *nonzero {
                format!("({lhs} & {rhs}) != 0")
            } else {
                format!("({lhs} & {rhs}) == 0")
            }
        }
        ResolvedCondition::Unresolved(raw) => format!("flags:{raw}"),
    }
}

/// A detected loop back edge (target earlier than source layout).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopBack {
    pub from_block: usize,
    pub to_address: u64,
    pub condition: Option<ResolvedCondition>,
}

/// Detect back edges: Branch/CondBranch targets that are earlier in
/// layout than the branching block. Honest v1: detection + annotation;
/// while/do-while reconstruction needs dominance (later slice).
pub fn find_loops(function: &IrFunction) -> Vec<LoopBack> {
    let mut out = Vec::new();
    for block in &function.blocks {
        match &block.terminator {
            IrTerminator::Branch(target) if *target <= block.start_address => {
                out.push(LoopBack {
                    from_block: block.id,
                    to_address: *target,
                    condition: None,
                });
            }
            IrTerminator::CondBranch { taken, fallthrough, .. }
                if *taken <= block.start_address && *fallthrough > block.start_address =>
            {
                out.push(LoopBack {
                    from_block: block.id,
                    to_address: *taken,
                    condition: Some(resolve_condition(function, block.id)),
                });
            }
            _ => {}
        }
    }
    out
}
