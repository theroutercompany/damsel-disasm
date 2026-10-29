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
        IrExpr::IndexedAddr { base, index, .. } => {
            visit(base);
            visit(index);
        }
    }
}

/// Per-block versioned renaming: `x0` becomes `x0_v1`, `x0_v2` on each
/// redefinition within a block. Cross-block renaming (phi-true SSA)
/// awaits the next D2 slice; v1 gives the renderer stable, unique
/// variable names per block so consecutive writes read correctly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamedFunction {
    pub blocks: Vec<RenamedBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamedBlock {
    pub id: usize,
    pub start_address: u64,
    pub statements: Vec<crate::ir::IrStatement>,
}

pub fn rename_registers(function: &crate::ir::IrFunction) -> RenamedFunction {
    let mut blocks = Vec::new();
    for block in &function.blocks {
        let mut versions: std::collections::BTreeMap<String, usize> = Default::default();
        let mut statements = Vec::new();
        for statement in &block.statements {
            let renamed = rename_statement(statement, &mut versions);
            statements.push(renamed);
        }
        blocks.push(RenamedBlock {
            id: block.id,
            start_address: block.start_address,
            statements,
        });
    }
    RenamedFunction { blocks }
}

fn rename_statement(
    statement: &crate::ir::IrStatement,
    versions: &mut std::collections::BTreeMap<String, usize>,
) -> crate::ir::IrStatement {
    use crate::ir::{IrExpr, IrStatement, IrValue};
    match statement {
        IrStatement::Assign { destination, expression } => {
            let version = versions.entry(destination.clone()).or_insert(0);
            *version += 1;
            let renamed_dest = format!("{destination}_v{version}");
            IrStatement::Assign {
                destination: renamed_dest,
                expression: rename_expr(expression, versions, false),
            }
        }
        IrStatement::Load { destination, address } => {
            let version = versions.entry(destination.clone()).or_insert(0);
            *version += 1;
            let renamed_dest = format!("{destination}_v{version}");
            IrStatement::Load {
                destination: renamed_dest,
                address: rename_expr(address, versions, false),
            }
        }
        IrStatement::Store { address, value } => IrStatement::Store {
            address: rename_expr(address, versions, false),
            value: rename_expr(value, versions, false),
        },
        other => other.clone(),
    }
}

fn rename_expr(
    expression: &crate::ir::IrExpr,
    versions: &std::collections::BTreeMap<String, usize>,
    _write: bool,
) -> crate::ir::IrExpr {
    use crate::ir::{IrExpr, IrValue};
    match expression {
        IrExpr::Value(IrValue::Register(register)) => {
            let version = versions.get(register).copied().unwrap_or(0);
            if version > 0 {
                IrExpr::Value(IrValue::Register(format!("{register}_v{version}")))
            } else {
                expression.clone()
            }
        }
        IrExpr::BinOp { op, lhs, rhs } => IrExpr::BinOp {
            op: *op,
            lhs: Box::new(rename_expr(lhs, versions, false)),
            rhs: Box::new(rename_expr(rhs, versions, false)),
        },
        IrExpr::AddrOf { base, displacement } => IrExpr::AddrOf {
            base: rename_register(base, versions),
            displacement: *displacement,
        },
        IrExpr::IndexedAddr { base, index, scale, displacement } => IrExpr::IndexedAddr {
            base: rename_register(base, versions),
            index: rename_register(index, versions),
            scale: *scale,
            displacement: *displacement,
        },
        _ => expression.clone(),
    }
}

fn rename_register(
    register: &str,
    versions: &std::collections::BTreeMap<String, usize>,
) -> String {
    let version = versions.get(register).copied().unwrap_or(0);
    if version > 0 {
        format!("{register}_v{version}")
    } else {
        register.to_string()
    }
}

/// ABI type seeds for arm64 (AAPCS64) and x86-64 (SysV): which
/// registers carry arguments / returns, their bit widths, and
/// callee-saved status. These seed the type system (Wave 6 spec §3)
/// from ABI facts rather than guessing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiRegisterRole {
    pub register: String,
    pub role: AbiRole,
    pub width_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiRole {
    Argument { index: usize },
    Return,
    CalleeSaved,
    CallerSaved,
    StackPointer,
    FramePointer,
    LinkRegister,
    Flags,
}

pub fn arm64_abi_roles() -> Vec<AbiRegisterRole> {
    let mut roles = Vec::new();
    // Arguments x0–x7
    for index in 0..8 {
        roles.push(AbiRegisterRole {
            register: format!("x{index}"),
            role: AbiRole::Argument { index },
            width_bits: 64,
        });
    }
    // Return x0
    roles.push(AbiRegisterRole { register: "x0".into(), role: AbiRole::Return, width_bits: 64 });
    // SP / FP / LR
    roles.push(AbiRegisterRole { register: "sp".into(), role: AbiRole::StackPointer, width_bits: 64 });
    roles.push(AbiRegisterRole { register: "fp".into(), role: AbiRole::FramePointer, width_bits: 64 });
    roles.push(AbiRegisterRole { register: "lr".into(), role: AbiRole::LinkRegister, width_bits: 64 });
    // Callee-saved x19–x28
    for index in 19..29 {
        roles.push(AbiRegisterRole {
            register: format!("x{index}"),
            role: AbiRole::CalleeSaved,
            width_bits: 64,
        });
    }
    // Caller-saved x8–x18 (scratch, indirect result at x8)
    for index in 8..19 {
        roles.push(AbiRegisterRole {
            register: format!("x{index}"),
            role: AbiRole::CallerSaved,
            width_bits: 64,
        });
    }
    roles
}

pub fn x86_64_abi_roles() -> Vec<AbiRegisterRole> {
    let mut roles = Vec::new();
    let args = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
    for (index, register) in args.iter().enumerate() {
        roles.push(AbiRegisterRole {
            register: register.to_string(),
            role: AbiRole::Argument { index },
            width_bits: 64,
        });
    }
    roles.push(AbiRegisterRole { register: "rax".into(), role: AbiRole::Return, width_bits: 64 });
    roles.push(AbiRegisterRole { register: "rsp".into(), role: AbiRole::StackPointer, width_bits: 64 });
    roles.push(AbiRegisterRole { register: "rbp".into(), role: AbiRole::FramePointer, width_bits: 64 });
    for register in ["rbx", "r12", "r13", "r14", "r15"] {
        roles.push(AbiRegisterRole { register: register.into(), role: AbiRole::CalleeSaved, width_bits: 64 });
    }
    roles
}
