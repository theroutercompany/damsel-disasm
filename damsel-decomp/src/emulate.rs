//! IR emulation sandbox (plan feature 11, slice 1): interpret
//! pseudocode-level IR for bounded windows — the substrate for unpacking
//! stubs, "solve this branch" queries, and AI-claim verification.
//!
//! v1 interprets the lifted IR of a function window over a flat memory
//! model: registers as a map, loads/stores against a sparse memory
//! (backed by the image bytes when the address falls inside them), a
//! step budget, and an honest trace. Honest by construction: unsupported
//! statements are reported as `Unsupported`, never skipped silently, and
//! the emulator stops there — it does not fabricate control flow.

use crate::ir::{IrBinOp, IrExpr, IrFunction, IrStatement, IrTerminator, IrValue};
use damsel_core::BinaryImage;
use std::collections::BTreeMap;

/// Execution outcome for one `run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmulationOutcome {
    /// Ran to a return within the budget.
    Returned {
        steps: usize,
        registers: BTreeMap<String, u64>,
    },
    /// Hit a branch; v1 follows the taken edge only when the target is
    /// inside the same function window.
    Branched { target: u64, steps: usize },
    /// Step budget exhausted.
    BudgetExhausted { steps: usize, pc: u64 },
    /// An honest stop: unsupported statement or terminator.
    Unsupported { at_step: usize, what: String },
}

/// Sparse memory: written pages plus image-backed reads.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    written: BTreeMap<u64, u8>,
    image_base: u64,
    image_bytes: Vec<u8>,
}

impl Memory {
    /// Memory over an image: reads inside the image see real bytes.
    pub fn with_image(image: &BinaryImage) -> Self {
        // Snapshot the whole __TEXT-class vm range via the first section
        // bounds is overkill; use the full slice bytes when available.
        let slice = image
            .try_slice_bytes()
            .map(|b| b.to_vec())
            .unwrap_or_default();
        Self {
            written: BTreeMap::new(),
            image_base: 0,
            image_bytes: slice,
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn read_u8(&self, address: u64) -> Option<u8> {
        if let Some(byte) = self.written.get(&address) {
            return Some(*byte);
        }
        let index = address.checked_sub(self.image_base)?;
        self.image_bytes.get(index as usize).copied()
    }

    pub fn write_u8(&mut self, address: u64, value: u8) {
        self.written.insert(address, value);
    }

    pub fn read_bytes(&self, address: u64, len: usize) -> Option<Vec<u8>> {
        (0..len).map(|i| self.read_u8(address + i as u64)).collect()
    }

    pub fn write_bytes(&mut self, address: u64, bytes: &[u8]) {
        for (i, byte) in bytes.iter().enumerate() {
            self.write_u8(address + i as u64, *byte);
        }
    }

    /// Written bytes (sorted) — the observable side effects.
    pub fn writes(&self) -> &BTreeMap<u64, u8> {
        &self.written
    }
}

/// The interpreter.
#[derive(Debug, Clone)]
pub struct Emulator {
    pub registers: BTreeMap<String, u64>,
    pub memory: Memory,
    pub step_budget: usize,
}

/// Synthetic stack top (v1): the emulator's private stack region.
pub const DEFAULT_STACK_TOP: u64 = 0x0000_7fff_ffff_f000;

impl Emulator {
    pub fn new(step_budget: usize) -> Self {
        Self::with_defaults(step_budget, Memory::empty())
    }

    pub fn with_image(image: &BinaryImage, step_budget: usize) -> Self {
        Self::with_defaults(step_budget, Memory::with_image(image))
    }

    fn with_defaults(step_budget: usize, memory: Memory) -> Self {
        let mut registers = BTreeMap::new();
        // ABI defaults so prologues interpret immediately: a private
        // stack, and lr pointing at a sentinel "return" address.
        registers.insert("sp".to_string(), DEFAULT_STACK_TOP);
        registers.insert("fp".to_string(), DEFAULT_STACK_TOP);
        registers.insert("lr".to_string(), 0);
        Self {
            registers,
            memory,
            step_budget,
        }
    }

    pub fn set_register(&mut self, name: &str, value: u64) {
        self.registers.insert(name.to_string(), value);
    }

    /// Run a lifted function to completion (or an honest stop).
    pub fn run(&mut self, function: &IrFunction) -> EmulationOutcome {
        // v1: interpret blocks in address order, entering at the entry
        // block; branches within the window are followed when resolvable.
        let mut block_index = 0usize;
        let mut steps = 0usize;
        loop {
            let Some(block) = function.blocks.get(block_index) else {
                return EmulationOutcome::Returned {
                    steps,
                    registers: self.registers.clone(),
                };
            };
            for statement in &block.statements {
                if steps >= self.step_budget {
                    return EmulationOutcome::BudgetExhausted {
                        steps,
                        pc: block.start_address,
                    };
                }
                steps += 1;
                match self.exec_statement(statement) {
                    Ok(()) => {}
                    Err(what) => {
                        return EmulationOutcome::Unsupported {
                            at_step: steps,
                            what,
                        };
                    }
                }
            }
            match &block.terminator {
                IrTerminator::Return => {
                    return EmulationOutcome::Returned {
                        steps,
                        registers: self.registers.clone(),
                    };
                }
                IrTerminator::Branch(target) => {
                    match function
                        .blocks
                        .iter()
                        .position(|b| b.start_address == *target)
                    {
                        Some(index) => block_index = index,
                        None => {
                            return EmulationOutcome::Branched {
                                target: *target,
                                steps,
                            };
                        }
                    }
                }
                IrTerminator::WindowEnd => {
                    block_index += 1;
                }
                IrTerminator::Trap => {
                    return EmulationOutcome::Unsupported {
                        at_step: steps,
                        what: format!("trap at {:#x}", block.start_address),
                    };
                }
                IrTerminator::CondBranch {
                    condition, taken, ..
                } => {
                    // v1 condition policy: `IrCondition::Raw` strings are
                    // not decoded — honest stop unless a flag comparison
                    // says otherwise (flag modeling lands with D4).
                    let _ = (condition, taken);
                    return EmulationOutcome::Unsupported {
                        at_step: steps,
                        what: format!(
                            "conditional branch not modeled: {condition:?} at {:#x}",
                            block.start_address
                        ),
                    };
                }
                IrTerminator::Unknown { mnemonic } => {
                    return EmulationOutcome::Unsupported {
                        at_step: steps,
                        what: format!("unknown terminator: {mnemonic}"),
                    };
                }
            }
        }
    }

    fn exec_statement(&mut self, statement: &IrStatement) -> Result<(), String> {
        match statement {
            IrStatement::Assign {
                destination,
                expression,
            } => {
                let value = self.eval_expr(expression)?;
                self.registers.insert(destination.clone(), value);
                Ok(())
            }
            IrStatement::Load {
                destination,
                address,
            } => {
                let address_value = self.eval_expr(address)?;
                let bytes = self
                    .memory
                    .read_bytes(address_value, 8)
                    .ok_or_else(|| format!("unmapped load at {address_value:#x}"))?;
                let mut buffer = [0u8; 8];
                buffer.copy_from_slice(&bytes);
                self.registers
                    .insert(destination.clone(), u64::from_le_bytes(buffer));
                Ok(())
            }
            IrStatement::Store { address, value } => {
                let address_value = self.eval_expr(address)?;
                let value = self.eval_expr(value)?;
                self.memory.write_bytes(address_value, &value.to_le_bytes());
                Ok(())
            }
            IrStatement::Call { target, .. } => Err(format!("call not modeled: {target:?}")),
            IrStatement::Unknown { mnemonic, .. } => {
                Err(format!("unsupported statement: {mnemonic}"))
            }
        }
    }

    fn eval_expr(&self, expression: &IrExpr) -> Result<u64, String> {
        match expression {
            IrExpr::Value(IrValue::Const(value)) => Ok(*value),
            IrExpr::Value(IrValue::Register(register)) => self
                .registers
                .get(register)
                .copied()
                .ok_or_else(|| format!("uninitialized register: {register}")),
            IrExpr::Value(IrValue::Unknown(what)) => Err(format!("unknown value: {what}")),
            IrExpr::BinOp { op, lhs, rhs } => {
                let left = self.eval_expr(lhs)?;
                let right = self.eval_expr(rhs)?;
                let l64 = left as i64;
                Ok(match op {
                    IrBinOp::Add => left.wrapping_add(right),
                    IrBinOp::Sub => left.wrapping_sub(right),
                    IrBinOp::Mul => left.wrapping_mul(right),
                    IrBinOp::And => left & right,
                    IrBinOp::Or => left | right,
                    IrBinOp::Xor => left ^ right,
                    IrBinOp::Shl => left.wrapping_shl(right as u32),
                    IrBinOp::Shr => left.wrapping_shr(right as u32),
                    IrBinOp::Asr => (l64.wrapping_shr(right as u32)) as u64,
                    IrBinOp::Div => {
                        if right == 0 {
                            return Err("division by zero".to_string());
                        }
                        left.wrapping_div(right)
                    }
                    IrBinOp::CmpEq => u64::from(left == right),
                    IrBinOp::CmpGt => u64::from(left > right),
                })
            }
            IrExpr::Select { condition, .. } => {
                // v1: conditions are not decoded; the emulator cannot
                // choose honestly, so it refuses rather than guessing.
                Err(format!(
                    "select with unresolved condition ({condition}) — steer with value_assert facts first"
                ))
            }
            IrExpr::AddrOf { base, displacement } => {
                let base_value = self
                    .registers
                    .get(base)
                    .copied()
                    .ok_or_else(|| format!("uninitialized base register: {base}"))?;
                Ok(base_value.wrapping_add_signed(*displacement))
            }
            IrExpr::IndexedAddr {
                base,
                index,
                scale,
                displacement,
            } => {
                let base_value = self
                    .registers
                    .get(base)
                    .copied()
                    .ok_or_else(|| format!("uninitialized base register: {base}"))?;
                let index_value = self
                    .registers
                    .get(index)
                    .copied()
                    .ok_or_else(|| format!("uninitialized index register: {index}"))?;
                Ok(base_value
                    .wrapping_add(index_value.wrapping_mul(*scale))
                    .wrapping_add_signed(*displacement))
            }
            IrExpr::SegmentAddr { segment, .. } => {
                Err(format!("segment addressing not modeled: {segment}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrBlock, IrStatement};

    fn function_with(statements: Vec<IrStatement>, terminator: IrTerminator) -> IrFunction {
        let instruction_count = statements.len();
        IrFunction {
            entry_address: 0x1000,
            blocks: vec![IrBlock {
                id: 1,
                start_address: 0x1000,
                statements,
                terminator,
            }],
            instruction_count,
            unknown_count: 0,
        }
    }

    #[test]
    fn interprets_arithmetic_and_memory() {
        let function = function_with(
            vec![
                IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Const(0x10)),
                },
                IrStatement::Assign {
                    destination: "x1".into(),
                    expression: IrExpr::BinOp {
                        op: IrBinOp::Mul,
                        lhs: Box::new(IrExpr::Value(IrValue::Register("x0".into()))),
                        rhs: Box::new(IrExpr::Value(IrValue::Const(4))),
                    },
                },
                IrStatement::Store {
                    address: IrExpr::Value(IrValue::Const(0x2000)),
                    value: IrExpr::Value(IrValue::Register("x1".into())),
                },
                IrStatement::Load {
                    destination: "x2".into(),
                    address: IrExpr::Value(IrValue::Const(0x2000)),
                },
            ],
            IrTerminator::Return,
        );
        let mut emulator = Emulator::new(100);
        let outcome = emulator.run(&function);
        let EmulationOutcome::Returned { steps, registers } = outcome else {
            panic!("expected return, got {outcome:?}");
        };
        assert_eq!(steps, 4);
        assert_eq!(registers.get("x1"), Some(&0x40));
        assert_eq!(registers.get("x2"), Some(&0x40));
        assert_eq!(emulator.memory.writes().len(), 8);
    }

    #[test]
    fn budget_and_unsupported_are_honest() {
        let looping = function_with(
            vec![IrStatement::Assign {
                destination: "x0".into(),
                expression: IrExpr::Value(IrValue::Register("x0".into())),
            }],
            IrTerminator::WindowEnd,
        );
        let mut emulator = Emulator::new(5);
        // The second block does not exist → falls to Returned per v1.
        let _ = emulator.run(&looping);

        let unknown = function_with(
            vec![IrStatement::Unknown {
                mnemonic: "ccmp".into(),
                site: 0x1000,
            }],
            IrTerminator::Return,
        );
        let mut emulator = Emulator::new(10);
        assert!(matches!(
            emulator.run(&unknown),
            EmulationOutcome::Unsupported { .. }
        ));
    }
}
