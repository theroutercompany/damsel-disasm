//! `damsel-decomp` — the decompiler pipeline (Wave 6 of
//! `docs/roadmaps/re-power-roadmap.md`).
//!
//! Stage D0 ships first: function summaries derived entirely from the
//! format-neutral project fact store — calls, imports, strings,
//! unresolved indirect transfers, bounds, problems. Later stages (IR,
//! SSA, structuring, pseudocode — strategy D1–D5) consume the same
//! facts plus Wave 5A semantics; no stage may emit only a string.

mod ir;
mod ssa;
mod summary;

pub use ir::{
    IrBinOp, IrBlock, IrCallTarget, IrCondition, IrExpr, IrFunction, IrStatement,
    IrTerminator, IrValue, lift_arm64,
};
pub use ssa::{PhiCandidate, SsaReport, build_ssa};
pub use summary::{
    CallSite, FunctionSummaryReport, SummaryError, summarize_function, summarize_named,
};
