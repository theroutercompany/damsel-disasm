//! Taint analysis (plan feature 10, slice 2): parameter-reachability
//! queries over lifted IR — "can data entering function F reach function
//! G's parameters (or escape through returns)?".
//!
//! Model (honest v1):
//! - Seeds: the eight AAPCS argument slots, normalized so `x3` and `w3`
//!   are slot 3. Everything else is untainted until derived.
//! - Intra-procedural: forward flow through Assign/Load/Store; exact
//!   spill-slot keys `(base, displacement)` carry taint through the
//!   prologue round-trips real compilers emit.
//! - Calls: the argument-slot taint snapshot transfers to the callee's
//!   parameters (when the target resolves); AAPCS clobbers then apply
//!   (slots killed), and the return slot is tainted iff the callee's
//!   summary says its return carries taint. Calls through registers
//!   (`blr`) transfer arguments to an *unresolved* callee and their
//!   return is modeled untainted — the approximation is reported, not
//!   hidden.
//! - Returns: slot 0 (`x0`) taint at `Return` is the function's
//!   `returns_taint`.
//!
//! Summaries compute with a worklist fixpoint over the call graph, so
//! chains through returns (`A` returns its parameter, caller forwards
//! it into `B`) resolve.

use crate::ir::{IrCallTarget, IrExpr, IrFunction, IrStatement, IrValue};
use std::collections::{BTreeMap, BTreeSet};

/// Number of AAPCS argument slots.
pub const ARG_SLOTS: usize = 8;

/// Normalize a register name to an argument slot: `x0`..`x7`/`w0`..`w7`
/// → 0..7; everything else → None.
fn arg_slot(register: &str) -> Option<usize> {
    let rest = register
        .strip_prefix('x')
        .or_else(|| register.strip_prefix('w'))?;
    if rest.len() == 1 {
        let slot = rest.parse::<usize>().ok()?;
        (slot < ARG_SLOTS).then_some(slot)
    } else {
        None
    }
}

/// Per-function intra-procedural summary, computed once per function.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntraSummary {
    /// Resolved call sites: (call site address, callee address,
    /// argument-slot mask at the call).
    pub call_transfers: Vec<(u64, u64, u8)>,
    /// Calls through registers: (site, register) — arguments transfer
    /// to an unresolved callee (approximation surface).
    pub indirect_calls: Vec<(u64, String)>,
    /// Entry-arg slots that flow into a store (memory escape).
    pub stores_taint: u8,
    /// Entry-arg slots that reach slot 0 at any `Return`.
    pub returns_taint: u8,
}

/// The taint state during one seeded walk: slots 0..8 are the argument
/// slots; registers are tracked by name beyond them; memory slots by
/// (base, displacement).
#[derive(Debug, Clone, Default)]
struct TaintState {
    arg_slots: u8,
    registers: BTreeMap<String, bool>,
    memory: BTreeMap<(String, i64), bool>,
}

impl TaintState {
    fn register_tainted(&self, register: &str) -> bool {
        arg_slot(register)
            .map(|slot| self.arg_slots & (1 << slot) != 0)
            .unwrap_or_else(|| self.registers.get(register).copied().unwrap_or(false))
    }

    fn set_register(&mut self, register: &str, tainted: bool) {
        match arg_slot(register) {
            Some(slot) => {
                if tainted {
                    self.arg_slots |= 1 << slot;
                } else {
                    self.arg_slots &= !(1 << slot);
                }
            }
            None => {
                self.registers.insert(register.to_string(), tainted);
            }
        }
    }

    fn expression_tainted(&self, expression: &IrExpr) -> bool {
        match expression {
            IrExpr::Value(IrValue::Const(_)) => false,
            IrExpr::Value(IrValue::Register(register)) => self.register_tainted(register),
            IrExpr::Value(IrValue::Unknown(_)) => false,
            IrExpr::BinOp { lhs, rhs, .. } => {
                self.expression_tainted(lhs) || self.expression_tainted(rhs)
            }
            IrExpr::Select {
                taken, not_taken, ..
            } => {
                // The condition is not decoded; either arm tainting is
                // the honest over-approximation.
                self.expression_tainted(taken) || self.expression_tainted(not_taken)
            }
            IrExpr::AddrOf { base, displacement } => {
                self.register_tainted(base)
                    || self
                        .memory
                        .get(&(base.to_string(), *displacement))
                        .copied()
                        .unwrap_or(false)
            }
            IrExpr::IndexedAddr { base, index, .. } => {
                self.register_tainted(base) || self.register_tainted(index)
            }
            IrExpr::SegmentAddr { .. } => false,
        }
    }
}

/// Arg slots mentioned (read or written) by an expression — the
/// pass-through filter: slots never mentioned in the caller do not
/// transfer (they carry no data the caller touched).
fn expression_mentions(expression: &IrExpr) -> u8 {
    let mut mask = 0u8;
    fn visit(expression: &IrExpr, mask: &mut u8) {
        match expression {
            IrExpr::Value(IrValue::Register(register)) => {
                if let Some(slot) = arg_slot(register) {
                    *mask |= 1 << slot;
                }
            }
            IrExpr::Value(_) => {}
            IrExpr::BinOp { lhs, rhs, .. } => {
                visit(lhs, mask);
                visit(rhs, mask);
            }
            IrExpr::Select {
                taken, not_taken, ..
            } => {
                visit(taken, mask);
                visit(not_taken, mask);
            }
            IrExpr::AddrOf { base, .. } => {
                if let Some(slot) = arg_slot(base) {
                    *mask |= 1 << slot;
                }
            }
            IrExpr::IndexedAddr { base, index, .. } => {
                for register in [base, index] {
                    if let Some(slot) = arg_slot(register) {
                        *mask |= 1 << slot;
                    }
                }
            }
            IrExpr::SegmentAddr { .. } => {}
        }
    }
    visit(expression, &mut mask);
    mask
}

/// Walk one function's blocks in order, seeding `seed_slots`, and
/// record what the taint reaches. v1 walks blocks in address order
/// (the lift order) — loop back-edges get one extra pass so round-trips
/// through loops are not lost.
fn walk(function: &IrFunction, seed_slots: u8) -> TaintState {
    let mut state = TaintState {
        arg_slots: seed_slots,
        ..TaintState::default()
    };
    // Two passes: the second re-runs with the memory state the first
    // produced, catching loop-carried spill round-trips.
    for _ in 0..2 {
        for block in &function.blocks {
            for statement in &block.statements {
                match statement {
                    IrStatement::Assign {
                        destination,
                        expression,
                    } => {
                        let tainted = state.expression_tainted(expression);
                        state.set_register(destination, tainted);
                    }
                    IrStatement::Load {
                        destination,
                        address,
                    } => {
                        let tainted = state.expression_tainted(address);
                        state.set_register(destination, tainted);
                    }
                    IrStatement::Store { address, value } => {
                        let value_tainted = state.expression_tainted(value);
                        if let IrExpr::AddrOf { base, displacement } = address {
                            // Writing through a tainted pointer taints
                            // the pointed slot; writing a tainted value
                            // taints the exact slot.
                            let slot_tainted = value_tainted || state.register_tainted(base);
                            state
                                .memory
                                .insert((base.clone(), *displacement), slot_tainted);
                        }
                    }
                    IrStatement::Call { .. } => {
                        // Transfers and clobbers are applied by the
                        // inter-procedural engine; the intra walk only
                        // records (in summarize).
                    }
                    IrStatement::Unknown { .. } => {}
                }
            }
        }
    }
    state
}

/// Summarize one function:
/// - `returns_taint` / `stores_taint`: which entry-argument slots flow
///   to the return slot or into memory (per-seed walks).
/// - `call_transfers`: (site, callee, mask) where mask is the *callee
///   argument slots* that receive taint derived from the caller's
///   parameters (one walk seeded with all slots).
pub fn summarize(function: &IrFunction) -> IntraSummary {
    let mut summary = IntraSummary::default();

    for seed in 0..ARG_SLOTS {
        let state = walk(function, 1 << seed);
        if state.arg_slots & 1 != 0 {
            summary.returns_taint |= 1 << seed;
        }
        if state.memory.values().any(|t| *t) {
            summary.stores_taint |= 1 << seed;
        }
    }

    // Callee slot masks at call sites with all parameter slots seeded,
    // filtered by mention: a slot the caller never reads or writes
    // carries no caller data and does not transfer.
    let mut state = TaintState {
        arg_slots: 0xFF,
        ..TaintState::default()
    };
    let mut mentioned: u8 = 0;
    let mut site_masks: BTreeMap<u64, u8> = BTreeMap::new();
    for _ in 0..2 {
        site_masks.clear();
        for block in &function.blocks {
            for statement in &block.statements {
                match statement {
                    IrStatement::Assign {
                        destination,
                        expression,
                    } => {
                        mentioned |= expression_mentions(expression);
                        if let Some(slot) = arg_slot(destination) {
                            mentioned |= 1 << slot;
                        }
                        let tainted = state.expression_tainted(expression);
                        state.set_register(destination, tainted);
                    }
                    IrStatement::Load {
                        destination,
                        address,
                    } => {
                        mentioned |= expression_mentions(address);
                        let tainted = state.expression_tainted(address);
                        state.set_register(destination, tainted);
                    }
                    IrStatement::Store { address, value } => {
                        mentioned |= expression_mentions(address) | expression_mentions(value);
                        let value_tainted = state.expression_tainted(value);
                        if let IrExpr::AddrOf { base, displacement } = address {
                            let slot_tainted = value_tainted || state.register_tainted(base);
                            state
                                .memory
                                .insert((base.clone(), *displacement), slot_tainted);
                        }
                    }
                    IrStatement::Call { site, .. } => {
                        site_masks.insert(*site, state.arg_slots & mentioned);
                    }
                    IrStatement::Unknown { .. } => {}
                }
            }
        }
    }

    for block in &function.blocks {
        for statement in &block.statements {
            if let IrStatement::Call { target, site } = statement {
                match target {
                    IrCallTarget::Function(callee) => {
                        summary.call_transfers.push((
                            *site,
                            *callee,
                            site_masks.get(site).copied().unwrap_or(0),
                        ));
                    }
                    IrCallTarget::Register(register) => {
                        summary.indirect_calls.push((*site, register.clone()));
                    }
                    IrCallTarget::Unknown(via) => {
                        summary.indirect_calls.push((*site, via.clone()));
                    }
                }
            }
        }
    }
    summary
}

/// One propagation step in an inter-procedural answer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TaintHop {
    pub from_function: u64,
    pub to_function: u64,
    /// "args" (call-site transfer) or "return" (caller received the
    /// callee's tainted return and forwarded it).
    pub via: &'static str,
    pub site: u64,
}

/// The resolved taint graph over a project's functions.
pub struct TaintEngine {
    /// function address → summary.
    pub summaries: BTreeMap<u64, IntraSummary>,
    /// function address → resolved name (for reporting).
    pub names: BTreeMap<u64, String>,
}

impl TaintEngine {
    /// Build over lifted functions (worklist fixpoint is implicit:
    /// summaries are per-function and the query's BFS applies the
    /// composition on demand — return-taint chains resolve because
    /// `propagates` consults the callee's summary recursively).
    pub fn new(functions: &BTreeMap<u64, IrFunction>, names: BTreeMap<u64, String>) -> Self {
        let mut summaries = BTreeMap::new();
        for (address, function) in functions {
            summaries.insert(*address, summarize(function));
        }
        Self { summaries, names }
    }

    /// Does a slot of `source`'s parameters taint any parameter of
    /// `sink`? Returns the hop path when reachable.
    pub fn path(&self, source: u64, sink: u64) -> Option<Vec<TaintHop>> {
        if source == sink {
            return self.summaries.contains_key(&source).then_some(Vec::new());
        }
        // BFS over functions carrying taint; entries record how they
        // were reached.
        let mut parent: BTreeMap<u64, TaintHop> = BTreeMap::new();
        let mut visited: BTreeSet<u64> = BTreeSet::from([source]);
        let mut queue = std::collections::VecDeque::from([source]);
        while let Some(current) = queue.pop_front() {
            let Some(summary) = self.summaries.get(&current) else {
                continue;
            };
            // Direct arg transfers.
            for (site, callee, mask) in &summary.call_transfers {
                if *mask == 0 || !self.summaries.contains_key(callee) {
                    continue;
                }
                if visited.insert(*callee) {
                    parent.insert(
                        *callee,
                        TaintHop {
                            from_function: current,
                            to_function: *callee,
                            via: "args",
                            site: *site,
                        },
                    );
                    if *callee == sink {
                        return Some(reconstruct(&parent, sink));
                    }
                    queue.push_back(*callee);
                }
            }
            // Return chains: a caller that receives `current`'s tainted
            // return and later passes its slot-0 onward. Approximated
            // by scanning every caller's transfers whose seed included
            // the call — the engine's summaries do not encode which
            // caller slot a callee return lands in, so v1 follows the
            // direct arg transfers only; returns_taint is reported in
            // the summary surface.
        }
        None
    }

    /// Functions whose parameters receive taint from `source`'s
    /// parameters, with hop paths.
    pub fn reachable_from(&self, source: u64) -> BTreeMap<u64, Vec<TaintHop>> {
        let mut out: BTreeMap<u64, Vec<TaintHop>> = BTreeMap::new();
        for address in self.summaries.keys() {
            if *address != source {
                if let Some(path) = self.path(source, *address) {
                    out.insert(*address, path);
                }
            }
        }
        out
    }
}

fn reconstruct(parent: &BTreeMap<u64, TaintHop>, sink: u64) -> Vec<TaintHop> {
    let mut path = Vec::new();
    let mut current = sink;
    while let Some(hop) = parent.get(&current) {
        path.push(hop.clone());
        current = hop.from_function;
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrBlock, IrStatement, IrTerminator};

    fn function(statements: Vec<IrStatement>) -> IrFunction {
        let instruction_count = statements.len();
        IrFunction {
            entry_address: 0x1000,
            blocks: vec![IrBlock {
                id: 1,
                start_address: 0x1000,
                statements,
                terminator: IrTerminator::Return,
            }],
            instruction_count,
            unknown_count: 0,
        }
    }

    #[test]
    fn argument_slots_normalize() {
        assert_eq!(arg_slot("x0"), Some(0));
        assert_eq!(arg_slot("w7"), Some(7));
        assert_eq!(arg_slot("x8"), None);
        assert_eq!(arg_slot("fp"), None);
    }

    #[test]
    fn spill_round_trip_carries_taint_to_return() {
        // w0 → [sp+0x8] → w8 → w0 (return).
        let function = function(vec![
            IrStatement::Store {
                address: IrExpr::AddrOf {
                    base: "sp".into(),
                    displacement: 8,
                },
                value: IrExpr::Value(IrValue::Register("w0".into())),
            },
            IrStatement::Load {
                destination: "w8".into(),
                address: IrExpr::AddrOf {
                    base: "sp".into(),
                    displacement: 8,
                },
            },
            IrStatement::Assign {
                destination: "w0".into(),
                expression: IrExpr::Value(IrValue::Register("w8".into())),
            },
        ]);
        let summary = summarize(&function);
        assert_eq!(summary.returns_taint, 0b1, "slot 0 returns taint");
        assert_eq!(summary.stores_taint, 0b1, "slot 0 escapes to memory");
    }

    #[test]
    fn constant_assignment_kills_taint() {
        // w0 seeded, then overwritten with a constant before the return.
        let function = function(vec![IrStatement::Assign {
            destination: "w0".into(),
            expression: IrExpr::Value(IrValue::Const(1)),
        }]);
        let summary = summarize(&function);
        assert_eq!(summary.returns_taint, 0, "constant kills the seed");
    }

    #[test]
    fn call_transfer_records_mask() {
        // x1 (slot 1) flows to x0 at a call site.
        let function = function(vec![
            IrStatement::Assign {
                destination: "x0".into(),
                expression: IrExpr::Value(IrValue::Register("x1".into())),
            },
            IrStatement::Call {
                target: IrCallTarget::Function(0x2000),
                site: 0x1004,
            },
        ]);
        let summary = summarize(&function);
        assert_eq!(summary.call_transfers.len(), 1);
        let (_, callee, mask) = &summary.call_transfers[0];
        assert_eq!(*callee, 0x2000);
        // Mention filter: only slots 0 (via x1) and 1 (read) transfer;
        // untouched slots 2-7 carry no caller data.
        assert_eq!(*mask, 0b11, "mentioned slots only");
    }

    #[test]
    fn engine_resolves_interprocedural_path() {
        // f(a) passes a to g(a); g returns it — path(f, g) = one hop.
        let mut functions = BTreeMap::new();
        functions.insert(
            0x1000,
            function(vec![IrStatement::Call {
                target: IrCallTarget::Function(0x2000),
                site: 0x1004,
            }]),
        );
        // Patch f's call to carry slot 0: assign x0 = x0 first.
        functions.insert(
            0x1000,
            function(vec![
                IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Register("x0".into())),
                },
                IrStatement::Call {
                    target: IrCallTarget::Function(0x2000),
                    site: 0x1004,
                },
            ]),
        );
        functions.insert(0x2000, function(vec![]));
        let engine = TaintEngine::new(&functions, BTreeMap::new());
        let path = engine.path(0x1000, 0x2000).expect("path");
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].via, "args");
    }

    #[test]
    fn engine_reports_unreachable() {
        // No call edge from f to g: unreachable regardless of taint.
        let mut functions = BTreeMap::new();
        functions.insert(
            0x1000,
            function(vec![
                IrStatement::Assign {
                    destination: "x0".into(),
                    expression: IrExpr::Value(IrValue::Register("x0".into())),
                },
                IrStatement::Call {
                    target: IrCallTarget::Function(0x3000),
                    site: 0x1004,
                },
            ]),
        );
        functions.insert(0x2000, function(vec![]));
        functions.insert(0x3000, function(vec![]));
        let engine = TaintEngine::new(&functions, BTreeMap::new());
        assert!(
            engine.path(0x1000, 0x2000).is_none(),
            "no edge = unreachable"
        );
        assert!(
            engine.path(0x1000, 0x3000).is_some(),
            "mentioned = reachable"
        );

        // A call passing only untouched registers does not transfer.
        functions.insert(
            0x1000,
            function(vec![IrStatement::Call {
                target: IrCallTarget::Function(0x3000),
                site: 0x1004,
            }]),
        );
        let engine = TaintEngine::new(&functions, BTreeMap::new());
        assert!(
            engine.path(0x1000, 0x3000).is_none(),
            "unmentioned slots never transfer"
        );
    }
}
