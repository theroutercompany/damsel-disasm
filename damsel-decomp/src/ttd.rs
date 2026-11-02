//! Time-travel debugging (plan feature 13, slice 1): deterministic
//! re-execution traces on the IR emulator. Every step records the pc,
//! the statement executed, registers written, and memory writes —
//! producing a queryable trace: "all writes to address X", "register
//! value at step N", "reverse from any point".
//!
//! Deterministic by construction: the emulator is pure (no system calls,
//! no threads), so replaying from step 0 to any N reproduces the exact
//! state. Reverse execution = replay to the requested step. No hardware
//! counters needed on macOS — this is the software-based recorder the
//! plan identifies as the biggest 2026 gap on Apple Silicon.

use crate::emulate::{EmulationOutcome, Emulator};
use crate::ir::{IrExpr, IrFunction, IrStatement, IrValue};
use std::collections::BTreeMap;

/// One recorded step.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TraceStep {
    pub step: usize,
    pub pc: u64,
    /// The statement's rendered text (for the timeline view).
    pub statement: String,
    /// Registers written this step: name → value after.
    pub registers_written: BTreeMap<String, u64>,
    /// Memory writes this step: address → byte values written.
    pub memory_writes: BTreeMap<u64, Vec<u8>>,
}

/// A full deterministic trace.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Trace {
    pub steps: Vec<TraceStep>,
    /// Final outcome.
    pub outcome: String,
    /// Total memory writes across the trace.
    pub total_writes: usize,
}

impl Trace {
    /// All memory writes to `address` (any width), with their step.
    pub fn writes_to(&self, address: u64) -> Vec<(usize, Vec<u8>)> {
        self.steps
            .iter()
            .filter_map(|step| {
                step.memory_writes
                    .get(&address)
                    .map(|bytes| (step.step, bytes.clone()))
            })
            .collect()
    }

    /// All memory writes to any address in `[start, end)`.
    pub fn writes_to_range(&self, start: u64, end: u64) -> Vec<(usize, u64, Vec<u8>)> {
        self.steps
            .iter()
            .flat_map(|step| {
                step.memory_writes
                    .iter()
                    .filter(|(address, _)| **address >= start && **address < end)
                    .map(|(address, bytes)| (step.step, *address, bytes.clone()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Register value at a given step (the most recent write ≤ step).
    pub fn register_at(&self, register: &str, step: usize) -> Option<u64> {
        let lowered = register.to_ascii_lowercase();
        self.steps
            .iter()
            .take(step + 1)
            .rev()
            .find_map(|trace_step| trace_step.registers_written.get(&lowered).copied())
    }

    /// The step at which a register was last written before `step`.
    pub fn register_last_written_at(&self, register: &str, step: usize) -> Option<usize> {
        let lowered = register.to_ascii_lowercase();
        self.steps
            .iter()
            .take(step + 1)
            .enumerate()
            .rev()
            .find(|(_, trace_step)| trace_step.registers_written.contains_key(&lowered))
            .map(|(index, _)| index)
    }

    /// Replay from step 0 to `target_step`, returning the emulator state
    /// at that point (deterministic reverse execution).
    pub fn state_at(
        &self,
        function: &IrFunction,
        target_step: usize,
        initial: &BTreeMap<String, u64>,
    ) -> Emulator {
        let mut emulator = Emulator::new(usize::MAX);
        for (register, value) in initial {
            emulator.set_register(register, *value);
        }
        // Re-run the trace deterministically up to target_step.
        let budget = target_step + 1;
        let mut replay = Emulator::new(budget);
        for (register, value) in initial {
            replay.set_register(register, *value);
        }
        let _ = replay.run(function);
        emulator = replay;
        emulator
    }

    /// Steps where a register was written (timeline view).
    pub fn register_timeline(&self, register: &str) -> Vec<(usize, u64)> {
        let lowered = register.to_ascii_lowercase();
        self.steps
            .iter()
            .filter(|step| step.registers_written.contains_key(&lowered))
            .map(|step| (step.step, step.registers_written[&lowered]))
            .collect()
    }
}

fn render_statement(statement: &IrStatement) -> String {
    match statement {
        IrStatement::Assign {
            destination,
            expression,
        } => {
            format!("{destination} = {:?}", expression)
        }
        IrStatement::Load { destination, .. } => format!("{destination} = load(...)"),
        IrStatement::Store { address, .. } => format!("store({:?})", address),
        IrStatement::Call { .. } => "call(...)".to_string(),
        IrStatement::Unknown { mnemonic, .. } => format!("unknown({mnemonic})"),
    }
}

/// Run a function under the emulator, recording every step.
pub fn record_trace(
    function: &IrFunction,
    initial_registers: &BTreeMap<String, u64>,
    step_budget: usize,
) -> (Trace, EmulationOutcome) {
    let mut emulator = Emulator::new(step_budget);
    for (register, value) in initial_registers {
        emulator.set_register(register, *value);
    }

    let outcome = emulator.run(function);

    // Rebuild the trace by re-running with step-by-step observation.
    // Since the emulator is deterministic, we replay and snapshot.
    let mut trace = Trace::default();
    let mut replay_emulator = Emulator::new(step_budget);
    for (register, value) in initial_registers {
        replay_emulator.set_register(register, *value);
    }

    // Walk blocks and statements, recording each step.
    let mut step_count = 0usize;
    let mut block_index = 0usize;
    let mut registers_before = initial_registers.clone();

    'outer: loop {
        let Some(block) = function.blocks.get(block_index) else {
            break;
        };
        for statement in &block.statements {
            if step_count >= step_budget {
                break 'outer;
            }

            let mut step = TraceStep {
                step: step_count,
                pc: block.start_address,
                statement: render_statement(statement),
                registers_written: BTreeMap::new(),
                memory_writes: BTreeMap::new(),
            };

            // Capture the store target/value before executing (direct
            // observation, no diff ambiguity on repeated writes).
            if let IrStatement::Store { address, value } = statement {
                if let (Ok(address_value), Ok(stored_value)) = (
                    eval_simple(&replay_emulator, address),
                    eval_simple(&replay_emulator, value),
                ) {
                    step.memory_writes
                        .insert(address_value, stored_value.to_le_bytes().to_vec());
                }
            }

            // Execute one step on the replay emulator.
            match exec_one_step(&mut replay_emulator, statement) {
                Ok(()) => {}
                Err(_) => break 'outer,
            }

            // Record registers written (diff against before state).
            for (register, value) in &replay_emulator.registers {
                if registers_before.get(register) != Some(value) {
                    step.registers_written.insert(register.clone(), *value);
                }
            }
            registers_before = replay_emulator.registers.clone();

            trace.steps.push(step);
            trace.total_writes = replay_emulator.memory.writes().len();
            step_count += 1;
        }

        match &block.terminator {
            crate::ir::IrTerminator::Return => break,
            crate::ir::IrTerminator::Branch(target) => {
                match function
                    .blocks
                    .iter()
                    .position(|b| b.start_address == *target)
                {
                    Some(index) => block_index = index,
                    None => break,
                }
            }
            crate::ir::IrTerminator::WindowEnd => block_index += 1,
            _ => break,
        }
    }

    trace.outcome = match &outcome {
        EmulationOutcome::Returned { .. } => "returned".to_string(),
        EmulationOutcome::Branched { .. } => "branched".to_string(),
        EmulationOutcome::BudgetExhausted { .. } => "budget_exhausted".to_string(),
        EmulationOutcome::Unsupported { .. } => "unsupported".to_string(),
    };

    (trace, outcome)
}

fn exec_one_step(emulator: &mut Emulator, statement: &IrStatement) -> Result<(), String> {
    match statement {
        IrStatement::Assign {
            destination,
            expression,
        } => {
            let value = eval_simple(emulator, expression)?;
            emulator.set_register(destination, value);
            Ok(())
        }
        IrStatement::Load {
            destination,
            address,
        } => {
            let address_value = eval_simple(emulator, address)?;
            let bytes = emulator
                .memory
                .read_bytes(address_value, 8)
                .ok_or_else(|| format!("unmapped load at {address_value:#x}"))?;
            let mut buffer = [0u8; 8];
            buffer.copy_from_slice(&bytes);
            emulator.set_register(destination, u64::from_le_bytes(buffer));
            Ok(())
        }
        IrStatement::Store { address, value } => {
            let address_value = eval_simple(emulator, address)?;
            let value = eval_simple(emulator, value)?;
            emulator
                .memory
                .write_bytes(address_value, &value.to_le_bytes());
            Ok(())
        }
        IrStatement::Call { .. } => Err("call not modeled".to_string()),
        IrStatement::Unknown { mnemonic, .. } => Err(format!("unsupported: {mnemonic}")),
    }
}

fn eval_simple(emulator: &Emulator, expression: &IrExpr) -> Result<u64, String> {
    match expression {
        IrExpr::Value(IrValue::Const(value)) => Ok(*value),
        IrExpr::Value(IrValue::Register(register)) => emulator
            .registers
            .get(register)
            .copied()
            .ok_or_else(|| format!("uninitialized register: {register}")),
        IrExpr::Value(IrValue::Unknown(what)) => Err(format!("unknown: {what}")),
        _ => {
            // Complex expressions: evaluate through the emulator's
            // existing eval (public surface via a fresh run is not
            // available; approximate with a mini-eval for BinOp).
            if let IrExpr::BinOp { op, lhs, rhs } = expression {
                let left = eval_simple(emulator, lhs)?;
                let right = eval_simple(emulator, rhs)?;
                use crate::ir::IrBinOp;
                return Ok(match op {
                    IrBinOp::Add => left.wrapping_add(right),
                    IrBinOp::Sub => left.wrapping_sub(right),
                    IrBinOp::Mul => left.wrapping_mul(right),
                    IrBinOp::And => left & right,
                    IrBinOp::Or => left | right,
                    IrBinOp::Xor => left ^ right,
                    IrBinOp::Shl => left.wrapping_shl(right as u32),
                    IrBinOp::Shr => left.wrapping_shr(right as u32),
                    IrBinOp::Asr => ((left as i64).wrapping_shr(right as u32)) as u64,
                    IrBinOp::Div => {
                        if right == 0 {
                            return Err("div by zero".to_string());
                        }
                        left.wrapping_div(right)
                    }
                    IrBinOp::CmpEq => u64::from(left == right),
                    IrBinOp::CmpGt => u64::from(left > right),
                });
            }
            if let IrExpr::AddrOf { base, displacement } = expression {
                let base_value = emulator
                    .registers
                    .get(base)
                    .copied()
                    .ok_or_else(|| format!("uninitialized base: {base}"))?;
                return Ok(base_value.wrapping_add_signed(*displacement));
            }
            if let IrExpr::Select { condition, .. } = expression {
                return Err(format!("select with unresolved condition ({condition})"));
            }
            Err("unsupported expression".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrBinOp, IrBlock, IrTerminator};

    fn traced_function() -> IrFunction {
        IrFunction {
            entry_address: 0x1000,
            blocks: vec![IrBlock {
                id: 1,
                start_address: 0x1000,
                statements: vec![
                    IrStatement::Assign {
                        destination: "x0".into(),
                        expression: IrExpr::Value(IrValue::Const(0x42)),
                    },
                    IrStatement::Store {
                        address: IrExpr::Value(IrValue::Const(0x2000)),
                        value: IrExpr::Value(IrValue::Register("x0".into())),
                    },
                    IrStatement::Assign {
                        destination: "x1".into(),
                        expression: IrExpr::BinOp {
                            op: IrBinOp::Add,
                            lhs: Box::new(IrExpr::Value(IrValue::Register("x0".into()))),
                            rhs: Box::new(IrExpr::Value(IrValue::Const(1))),
                        },
                    },
                    IrStatement::Store {
                        address: IrExpr::Value(IrValue::Const(0x2000)),
                        value: IrExpr::Value(IrValue::Register("x1".into())),
                    },
                ],
                terminator: IrTerminator::Return,
            }],
            instruction_count: 4,
            unknown_count: 0,
        }
    }

    #[test]
    fn records_steps_registers_and_writes() {
        let (trace, outcome) = record_trace(&traced_function(), &BTreeMap::new(), 100);
        assert!(matches!(outcome, EmulationOutcome::Returned { .. }));
        assert_eq!(trace.steps.len(), 4);
        assert_eq!(trace.outcome, "returned");

        // Step 0 wrote x0 = 0x42.
        assert_eq!(trace.steps[0].registers_written.get("x0"), Some(&0x42));

        // Steps 1 and 3 stored to 0x2000.
        assert_eq!(trace.writes_to(0x2000).len(), 2);

        // Register timeline.
        let timeline = trace.register_timeline("x0");
        assert_eq!(timeline, vec![(0, 0x42)]);

        // Register value at step 2 (x0 was set at step 0).
        assert_eq!(trace.register_at("x0", 2), Some(0x42));
        // x1 was set at step 2.
        assert_eq!(trace.register_at("x1", 2), Some(0x43));
        assert_eq!(trace.register_at("x1", 1), None);

        // Writes to a range.
        let range_writes = trace.writes_to_range(0x1000, 0x3000);
        assert_eq!(range_writes.len(), 2);
    }

    #[test]
    fn deterministic_replay_reaches_same_state() {
        let (trace, _) = record_trace(&traced_function(), &BTreeMap::new(), 100);
        // Replay to step 2 and verify x0 is set.
        let state = trace.state_at(&traced_function(), 2, &BTreeMap::new());
        assert_eq!(state.registers.get("x0"), Some(&0x42));
    }

    #[test]
    fn budget_stops_trace() {
        let (trace, outcome) = record_trace(&traced_function(), &BTreeMap::new(), 2);
        assert!(matches!(outcome, EmulationOutcome::BudgetExhausted { .. }));
        assert_eq!(trace.steps.len(), 2);
        assert_eq!(trace.outcome, "budget_exhausted");
    }
}
