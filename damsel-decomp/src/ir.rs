//! D1 low-level IR (Wave 6; decompiler-ir-strategy §Low-Level IR).
//!
//! v1 lift covers the strategy's floor instruction set for arm64
//! (data movement, arithmetic, loads/stores, direct control flow);
//! anything else lifts as `Unknown` — visible, never guessed. Values
//! carry widths where the operand names encode them (`x0`/`w0`).

use damsel_core::{DecodedInstruction, Operand};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrValue {
    Const(u64),
    Register(String),
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrBinOp {
    Add,
    Sub,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrExpr {
    Value(IrValue),
    BinOp {
        op: IrBinOp,
        lhs: Box<IrExpr>,
        rhs: Box<IrExpr>,
    },
    /// Memory address: base register + displacement.
    AddrOf {
        base: String,
        displacement: i64,
    },
    /// Indexed address: base + index * scale + displacement
    /// (scale 1 = plain index; models x86 [base+idx*scale+disp] and
    /// arm64 [base, idx, lsl #n] once the lift forwards shifts).
    IndexedAddr {
        base: String,
        index: String,
        scale: u64,
        displacement: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrCallTarget {
    Function(u64),
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrStatement {
    Assign {
        destination: String,
        expression: IrExpr,
    },
    Load {
        destination: String,
        address: IrExpr,
    },
    Store {
        address: IrExpr,
        value: IrExpr,
    },
    Call {
        target: IrCallTarget,
        site: u64,
    },
    /// Instructions without v1 semantics — surfaced honestly.
    Unknown {
        mnemonic: String,
        site: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrCondition {
    /// Flag condition not yet modeled (e.g. `b.eq` v1) — kept as the
    /// mnemonic so renderers can show it instead of inventing one.
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrTerminator {
    Return,
    Branch(u64),
    CondBranch {
        condition: IrCondition,
        taken: u64,
        fallthrough: u64,
    },
    Trap,
    /// End of the decoded window without a hard terminator.
    WindowEnd,
    Unknown {
        mnemonic: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrBlock {
    pub id: usize,
    pub start_address: u64,
    pub statements: Vec<IrStatement>,
    pub terminator: IrTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrFunction {
    pub entry_address: u64,
    pub blocks: Vec<IrBlock>,
    pub instruction_count: usize,
    pub unknown_count: usize,
}

/// Lift decoded arm64 instructions into D1 IR. Pure function of the
/// instruction stream (no image context): address resolution beyond
/// direct targets is deliberately out of scope for v1.
pub fn lift_arm64(instructions: &[DecodedInstruction]) -> IrFunction {
    let mut blocks: Vec<IrBlock> = Vec::new();
    let mut current = IrBlock {
        id: 0,
        start_address: instructions.first().map(|i| i.address).unwrap_or(0),
        statements: Vec::new(),
        terminator: IrTerminator::WindowEnd,
    };
    let mut unknown_count = 0usize;

    let mut statement_for = |instruction: &DecodedInstruction,
                             statements: &mut Vec<IrStatement>,
                             unknowns: &mut usize| {
        let lower = instruction.mnemonic.to_ascii_lowercase();
        let site = instruction.address;
        // Data movement / arithmetic floor set.
        let binary_op = match lower.as_str() {
            "add" | "adds" => Some(IrBinOp::Add),
            "sub" | "subs" => Some(IrBinOp::Sub),
            "and" | "ands" => Some(IrBinOp::And),
            "orr" => Some(IrBinOp::Or),
            "eor" => Some(IrBinOp::Xor),
            "lsl" => Some(IrBinOp::Shl),
            "lsr" => Some(IrBinOp::Shr),
            _ => None,
        };
        // Flag-setting compares (arm64: cmp/cmn/tst) mirror the x86
        // cmp/test model — the shared substrate for condition resolution.
        let flag_op = match lower.as_str() {
            "cmp" | "subs" => Some(IrBinOp::Sub),
            "cmn" | "adds" => Some(IrBinOp::Add),
            "tst" | "ands" => Some(IrBinOp::And),
            _ => None,
        };
        let destination = instruction
            .operands
            .first()
            .and_then(register_of)
            .map(|d| d.clone());
        if let Some(op) = flag_op {
            // Register-destination variants (subs/adds/ands) compare
            // operands [1],[2]; compare-only forms (cmp/cmn/tst) use
            // [0],[1].
            let has_destination = destination.is_some() && binary_op.is_some();
            let (flag_lhs, flag_rhs) = if has_destination { (1, 2) } else { (0, 1) };
            statements.push(IrStatement::Assign {
                destination: "flags".to_string(),
                expression: IrExpr::BinOp {
                    op,
                    lhs: Box::new(operand_expr(instruction.operands.get(flag_lhs))),
                    rhs: Box::new(operand_expr(instruction.operands.get(flag_rhs))),
                },
            });
            if has_destination {
                let dst = destination.clone().unwrap();
                statements.push(IrStatement::Assign {
                    destination: dst,
                    expression: IrExpr::BinOp {
                        op,
                        lhs: Box::new(operand_expr(instruction.operands.get(1))),
                        rhs: Box::new(operand_expr(instruction.operands.get(2))),
                    },
                });
            }
            return;
        }
        if let (Some(op), Some(dst)) = (binary_op, destination.clone()) {
            statements.push(IrStatement::Assign {
                destination: dst,
                expression: IrExpr::BinOp {
                    op,
                    lhs: Box::new(operand_expr(instruction.operands.get(1))),
                    rhs: Box::new(operand_expr(instruction.operands.get(2))),
                },
            });
            return;
        }
        match lower.as_str() {
            "mov" | "movz" | "movk" => {
                if let Some(dst) = destination.clone() {
                    statements.push(IrStatement::Assign {
                        destination: dst,
                        expression: operand_expr(instruction.operands.get(1)),
                    });
                }
            }
            "stp" | "stp.nu" => {
                // Pair store: reg1 at [base+d], reg2 at [base+d+8].
                // The pre-index writeback's sp adjustment is tracked by
                // the stack-frame analyzer via the explicit sub/add sp.
                if let Some(IrExpr::AddrOf { base, displacement }) = memory_expr(instruction) {
                    for (index, operand) in instruction.operands.iter().take(2).enumerate() {
                        if let Some(register) = register_of(operand) {
                            statements.push(IrStatement::Store {
                                address: IrExpr::AddrOf {
                                    base: base.clone(),
                                    displacement: displacement + 8 * index as i64,
                                },
                                value: IrExpr::Value(IrValue::Register(register)),
                            });
                        }
                    }
                } else {
                    *unknowns += 1;
                    statements.push(IrStatement::Unknown { mnemonic: lower, site });
                }
            }
            "ldp" | "ldp.nu" => {
                // Pair load: mirrors stp.
                if let Some(IrExpr::AddrOf { base, displacement }) = memory_expr(instruction) {
                    for (index, operand) in instruction.operands.iter().take(2).enumerate() {
                        if let Some(register) = register_of(operand) {
                            statements.push(IrStatement::Load {
                                destination: register,
                                address: IrExpr::AddrOf {
                                    base: base.clone(),
                                    displacement: displacement + 8 * index as i64,
                                },
                            });
                        }
                    }
                } else {
                    *unknowns += 1;
                    statements.push(IrStatement::Unknown { mnemonic: lower, site });
                }
            }
            "cset" | "cinc" => {
                // Conditional set: dest = (flags condition) ? 1 : 0.
                // v1 renders the condition from the mnemonic suffix —
                // resolved precisely in the D4 pass.
                if let Some(dst) = destination.clone() {
                    statements.push(IrStatement::Assign {
                        destination: dst,
                        expression: IrExpr::Value(IrValue::Unknown(format!(
                            "cond:{}",
                            lower.trim_start_matches("cset").trim_start_matches("cinc")
                        ))),
                    });
                } else {
                    *unknowns += 1;
                    statements.push(IrStatement::Unknown { mnemonic: lower, site });
                }
            }
            "adrp" | "adr" => {
                // Page/label materialization: the decoder already
                // resolved the absolute target into a Page/Data
                // reference — lift as a constant assignment.
                let target = instruction.references.iter().find_map(|r| match r {
                    damsel_core::Reference::Page { target }
                    | damsel_core::Reference::Data { target } => Some(*target),
                    _ => None,
                });
                if let (Some(dst), Some(target)) = (destination.clone(), target) {
                    statements.push(IrStatement::Assign {
                        destination: dst,
                        expression: IrExpr::Value(IrValue::Const(target)),
                    });
                } else {
                    *unknowns += 1;
                    statements.push(IrStatement::Unknown { mnemonic: lower, site });
                }
            }
            "ldr" | "ldur" => {
                if let Some(dst) = destination.clone() {
                    if let Some(address) = memory_expr(instruction) {
                        statements.push(IrStatement::Load {
                            destination: dst,
                            address,
                        });
                    } else {
                        *unknowns += 1;
                        statements.push(IrStatement::Unknown { mnemonic: lower, site });
                    }
                }
            }
            "str" | "stur" => {
                if let Some(address) = memory_expr(instruction) {
                    let value = operand_expr(instruction.operands.first());
                    statements.push(IrStatement::Store { address, value });
                } else {
                    *unknowns += 1;
                    statements.push(IrStatement::Unknown { mnemonic: lower, site });
                }
            }
            "bl" => {
                let call_target = instruction.references.iter().find_map(|r| match r {
                    damsel_core::Reference::Call { target } => Some(*target),
                    _ => None,
                });
                if let Some(target) = call_target {
                    statements.push(IrStatement::Call {
                        target: IrCallTarget::Function(target),
                        site,
                    });
                } else {
                    *unknowns += 1;
                    statements.push(IrStatement::Unknown { mnemonic: lower, site });
                }
            }
            _ if damsel_core::semantics_arm64::is_cfg_boundary(&lower) => {
                // Handled as terminators below; no statement.
            }
            _ => {
                *unknowns += 1;
                statements.push(IrStatement::Unknown { mnemonic: lower, site });
            }
        }
    };

    for instruction in instructions {
        let lower = instruction.mnemonic.to_ascii_lowercase();
        let is_boundary = damsel_core::semantics_arm64::is_cfg_boundary(&lower);
        if !is_boundary {
            statement_for(instruction, &mut current.statements, &mut unknown_count);
            continue;
        }
        // Boundary instruction: close the block with a terminator.
        let next = instruction.address + instruction.size as u64;
        let terminator = if damsel_core::semantics_arm64::is_return(&lower) {
            IrTerminator::Return
        } else if damsel_core::semantics_arm64::is_trap(&lower) {
            IrTerminator::Trap
        } else if let Some(target) = branch_target(instruction) {
            let target = *target;
            if damsel_core::semantics_arm64::is_conditional_branch(&lower) {
                IrTerminator::CondBranch {
                    condition: IrCondition::Raw(lower),
                    taken: target,
                    fallthrough: next,
                }
            } else {
                IrTerminator::Branch(target)
            }
        } else {
            IrTerminator::Unknown {
                mnemonic: lower.clone(),
            }
        };
        current.terminator = terminator;
        blocks.push(std::mem::replace(
            &mut current,
            IrBlock {
                id: blocks.len() + 1,
                start_address: next,
                statements: Vec::new(),
                terminator: IrTerminator::WindowEnd,
            },
        ));
        unknown_count += usize::from(matches!(blocks.last().unwrap().terminator, IrTerminator::Unknown { .. }));
    }
    // Final open block (e.g. window ended after a return's fallthrough
    // padding or the last instruction was not a boundary).
    if !current.statements.is_empty() || (blocks.is_empty() && !instructions.is_empty()) {
        blocks.push(current);
    }

    IrFunction {
        entry_address: instructions.first().map(|i| i.address).unwrap_or(0),
        blocks,
        instruction_count: instructions.len(),
        unknown_count,
    }
}

fn register_of(operand: &Operand) -> Option<String> {
    match operand {
        Operand::Register(register) => Some(register.clone()),
        Operand::QualifiedRegister { register, .. } => Some(register.clone()),
        _ => None,
    }
}

fn operand_expr(operand: Option<&Operand>) -> IrExpr {
    match operand {
        Some(Operand::Register(register)) | Some(Operand::QualifiedRegister { register, .. }) => {
            IrExpr::Value(IrValue::Register(register.clone()))
        }
        Some(Operand::ImmediateSigned(value)) => IrExpr::Value(IrValue::Const(*value as u64)),
        Some(Operand::ImmediateUnsigned(value)) => IrExpr::Value(IrValue::Const(*value)),
        Some(Operand::Label(target)) => IrExpr::Value(IrValue::Const(*target)),
        _ => IrExpr::Value(IrValue::Unknown("operand".to_string())),
    }
}

fn memory_expr(instruction: &DecodedInstruction) -> Option<IrExpr> {
    instruction.operands.iter().find_map(|operand| match operand {
        Operand::Memory { base, displacement, .. } => Some(IrExpr::AddrOf {
            base: base.clone(),
            displacement: *displacement,
        }),
        _ => None,
    })
}

fn branch_target(instruction: &DecodedInstruction) -> Option<&u64> {
    instruction.references.iter().find_map(|r| match r {
        damsel_core::Reference::Branch { target } => Some(target),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// x86-64 lift (Wave 6; shares the D1 vocabulary). Floor set: mov/movabs
// (reg↔imm/reg/mem), lea, add/sub/and/or/xor/shl/shr, call (direct via
// references, indirect → Unknown target), ret/jmp/jcc/trap terminators.
// Indexed memory (base+index*scale) stays `Unknown` until D2 models
// scale — never guessed.
// ---------------------------------------------------------------------------

pub fn lift_x86_64(instructions: &[DecodedInstruction]) -> IrFunction {
    use damsel_core::semantics_x86_64 as sem;

    let mut blocks: Vec<IrBlock> = Vec::new();
    let mut current = IrBlock {
        id: 0,
        start_address: instructions.first().map(|i| i.address).unwrap_or(0),
        statements: Vec::new(),
        terminator: IrTerminator::WindowEnd,
    };
    let mut unknown_count = 0usize;

    for instruction in instructions {
        let lower = instruction.mnemonic.to_ascii_lowercase();
        let site = instruction.address;

        // Terminator first (x86 terminators emit no statements).
        if sem::is_return(&lower) || matches!(lower.as_str(), "jmp" | "ljmp")
            || sem::is_conditional_branch(&lower) || sem::is_trap(&lower)
        {
            let next = instruction.address + instruction.size as u64;
            let terminator = if sem::is_return(&lower) {
                IrTerminator::Return
            } else if sem::is_trap(&lower) {
                IrTerminator::Trap
            } else if let Some(target) = instruction.references.iter().find_map(|r| match r {
                damsel_core::Reference::Branch { target } => Some(*target),
                _ => None,
            }) {
                if sem::is_conditional_branch(&lower) {
                    IrTerminator::CondBranch {
                        condition: IrCondition::Raw(lower.clone()),
                        taken: target,
                        fallthrough: next,
                    }
                } else {
                    IrTerminator::Branch(target)
                }
            } else {
                IrTerminator::Unknown { mnemonic: lower.clone() }
            };
            current.terminator = terminator;
            blocks.push(std::mem::replace(
                &mut current,
                IrBlock {
                    id: blocks.len() + 1,
                    start_address: next,
                    statements: Vec::new(),
                    terminator: IrTerminator::WindowEnd,
                },
            ));
            unknown_count += usize::from(matches!(
                blocks.last().unwrap().terminator,
                IrTerminator::Unknown { .. }
            ));
            continue;
        }

        let binary_op = match lower.as_str() {
            "add" => Some(IrBinOp::Add),
            "sub" => Some(IrBinOp::Sub),
            "and" => Some(IrBinOp::And),
            "or" => Some(IrBinOp::Or),
            "xor" => Some(IrBinOp::Xor),
            "shl" => Some(IrBinOp::Shl),
            "shr" => Some(IrBinOp::Shr),
            _ => None,
        };
        let destination = instruction.operands.first().and_then(register_of);

        // cmp/test write flags without a register destination — model
        // as an assignment to the synthetic `flags` register so D4 can
        // resolve jcc conditions later.
        let flag_op = match lower.as_str() {
            "cmp" => Some(IrBinOp::Sub),
            "test" => Some(IrBinOp::And),
            _ => None,
        };
        let statement = if let Some(op) = flag_op {
            Some(IrStatement::Assign {
                destination: "flags".to_string(),
                expression: IrExpr::BinOp {
                    op,
                    lhs: Box::new(operand_expr(instruction.operands.first())),
                    rhs: Box::new(operand_expr(instruction.operands.get(1))),
                },
            })
        } else if let (Some(op), Some(dst)) = (binary_op, destination.clone()) {
            Some(IrStatement::Assign {
                destination: dst,
                expression: IrExpr::BinOp {
                    op,
                    lhs: Box::new(operand_expr(instruction.operands.first())),
                    rhs: Box::new(operand_expr(instruction.operands.get(1))),
                },
            })
        } else {
            match lower.as_str() {
                "mov" | "movabs" | "movq" | "movl" => {
                    let first = instruction.operands.first();
                    let second = instruction.operands.get(1);
                    match (first, second) {
                        (Some(dst_op), Some(src_op)) => {
                            if let Some(dst) = register_of(dst_op) {
                                if matches!(src_op, Operand::Memory { .. }) {
                                    memory_expr_from(src_op).map(|address| IrStatement::Load {
                                        destination: dst,
                                        address,
                                    })
                                } else {
                                    Some(IrStatement::Assign {
                                        destination: dst,
                                        expression: operand_expr(Some(src_op)),
                                    })
                                }
                            } else if let (Some(address), Some(value)) =
                                (memory_expr_from(dst_op), operand_expr_opt(Some(src_op)))
                            {
                                Some(IrStatement::Store { address, value })
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                }
                "lea" => {
                    let destination = destination.clone();
                    instruction
                        .operands
                        .get(1)
                        .and_then(memory_expr_from)
                        .map(|address| IrStatement::Assign {
                            destination: destination.unwrap_or_default(),
                            expression: address,
                        })
                }
                "call" | "lcall" => {
                    let target = instruction.references.iter().find_map(|r| match r {
                        damsel_core::Reference::Call { target } => {
                            Some(IrCallTarget::Function(*target))
                        }
                        damsel_core::Reference::IndirectCall { via } => {
                            Some(IrCallTarget::Unknown(via.clone()))
                        }
                        _ => None,
                    });
                    target.map(|target| IrStatement::Call { target, site })
                }
                _ => None,
            }
        };

        match statement {
            Some(statement) => current.statements.push(statement),
            None => {
                unknown_count += 1;
                current.statements.push(IrStatement::Unknown { mnemonic: lower, site });
            }
        }
    }
    if !current.statements.is_empty() || (blocks.is_empty() && !instructions.is_empty()) {
        blocks.push(current);
    }

    IrFunction {
        entry_address: instructions.first().map(|i| i.address).unwrap_or(0),
        blocks,
        instruction_count: instructions.len(),
        unknown_count,
    }
}

fn memory_expr_from(operand: &Operand) -> Option<IrExpr> {
    match operand {
        Operand::Memory { base, index, index_modifier, displacement, .. } => {
            match (index, index_modifier) {
                (Some(index), Some(modifier)) => {
                    let scale = modifier
                        .strip_prefix('*')
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(1);
                    Some(IrExpr::IndexedAddr {
                        base: base.clone(),
                        index: index.clone(),
                        scale,
                        displacement: *displacement,
                    })
                }
                (Some(index), None) => Some(IrExpr::IndexedAddr {
                    base: base.clone(),
                    index: index.clone(),
                    scale: 1,
                    displacement: *displacement,
                }),
                _ => Some(IrExpr::AddrOf {
                    base: base.clone(),
                    displacement: *displacement,
                }),
            }
        }
        _ => None,
    }
}

fn operand_expr_opt(operand: Option<&Operand>) -> Option<IrExpr> {
    operand.map(|o| operand_expr(Some(o)))
}
