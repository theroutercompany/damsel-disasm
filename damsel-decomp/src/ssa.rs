//! D2 register data-flow (Wave 6; decompiler-ir-strategy §SSA And Def-Use).
//!
//! v1 is deliberately honest about scope: linear register versioning
//! over the lifted blocks (every write bumps a per-function version),
//! a block-level CFG from terminators, and **phi candidates** —
//! registers with multiple defs whose uses sit in multi-predecessor
//! blocks. Full dominance-frontier phi construction and per-block
//! renumbering arrive in the next D2 slice; the v1 report already
//! answers "which registers carry merging values", which is what
//! structuring and stack recovery need first.

use crate::ir::{IrExpr, IrFunction, IrStatement, IrTerminator, IrValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsaReport {
    /// Block id -> incoming block ids (CFG predecessors).
    pub predecessors: BTreeMap<usize, Vec<usize>>,
    /// register -> number of defs (writes) in the function.
    pub defs: BTreeMap<String, usize>,
    /// register -> number of uses (reads) in the function.
    pub uses: BTreeMap<String, usize>,
    /// Registers that likely need phi nodes: multiple defs and at
    /// least one use in a block with multiple predecessors.
    pub phi_candidates: Vec<PhiCandidate>,
    /// Total statement-level writes/reads for quick gating.
    pub total_defs: usize,
    pub total_uses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhiCandidate {
    pub block: usize,
    pub register: String,
    pub defs: usize,
}

pub fn build_ssa(function: &IrFunction) -> SsaReport {
    // Block address -> id map for CFG edges.
    let block_at: BTreeMap<u64, usize> = function
        .blocks
        .iter()
        .map(|b| (b.start_address, b.id))
        .collect();

    let mut predecessors: BTreeMap<usize, Vec<usize>> = BTreeMap::new();

    let mut defs: BTreeMap<String, usize> = BTreeMap::new();
    let mut uses: BTreeMap<String, usize> = BTreeMap::new();
    // register -> blocks that read it (for phi candidacy).
    let mut use_blocks: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for block in &function.blocks {
        // CFG edges from the terminator.
        let mut targets: Vec<u64> = Vec::new();
        match &block.terminator {
            IrTerminator::Branch(target) => targets.push(*target),
            IrTerminator::CondBranch { taken, fallthrough, .. } => {
                targets.push(*taken);
                targets.push(*fallthrough);
            }
            // Return/Trap/WindowEnd/Unknown end the path; WindowEnd
            // falls through to the next block by layout when present.
            IrTerminator::WindowEnd => {
                if let Some(next) = function
                    .blocks
                    .iter()
                    .find(|b| b.start_address > block.start_address)
                {
                    targets.push(next.start_address);
                }
            }
            _ => {}
        }
        for target in targets {
            if let Some(id) = block_at.get(&target) {
                predecessors.entry(*id).or_default().push(block.id);
            }
        }

        for statement in &block.statements {
            match statement {
                IrStatement::Assign { destination, expression } => {
                    *defs.entry(destination.clone()).or_insert(0) += 1;
                    read_registers(expression, &mut |r| {
                        *uses.entry(r.to_string()).or_insert(0) += 1;
                        use_blocks.entry(r.to_string()).or_default().push(block.id);
                    });
                }
                IrStatement::Load { destination, address } => {
                    *defs.entry(destination.clone()).or_insert(0) += 1;
                    read_registers(address, &mut |r| {
                        *uses.entry(r.to_string()).or_insert(0) += 1;
                        use_blocks.entry(r.to_string()).or_default().push(block.id);
                    });
                }
                IrStatement::Store { address, value } => {
                    for expr in [address, value] {
                        read_registers(expr, &mut |r| {
                            *uses.entry(r.to_string()).or_insert(0) += 1;
                            use_blocks.entry(r.to_string()).or_default().push(block.id);
                        });
                    }
                }
                IrStatement::Call { .. } | IrStatement::Unknown { .. } => {}
            }
        }
    }

    // Phi candidates: multi-def registers read in multi-predecessor
    // blocks (v1 heuristic — no dominance frontiers yet).
    let mut phi_candidates = Vec::new();
    for (register, def_count) in &defs {
        if *def_count < 2 {
            continue;
        }
        for block in &use_blocks[register] {
            if predecessors.get(block).is_some_and(|v| v.len() > 1) {
                phi_candidates.push(PhiCandidate {
                    block: *block,
                    register: register.clone(),
                    defs: *def_count,
                });
            }
        }
    }
    phi_candidates.sort();
    phi_candidates.dedup();

    SsaReport {
        predecessors,
        total_defs: defs.values().sum(),
        total_uses: uses.values().sum(),
        phi_candidates,
        defs,
        uses,
    }
}

fn read_registers(expression: &IrExpr, visit: &mut impl FnMut(&str)) {
    match expression {
        IrExpr::Value(IrValue::Register(register)) => visit(register),
        IrExpr::Value(_) => {}
        IrExpr::BinOp { lhs, rhs, .. } => {
            read_registers(lhs, visit);
            read_registers(rhs, visit);
        }
        IrExpr::AddrOf { base, .. } => visit(base),
    }
}
