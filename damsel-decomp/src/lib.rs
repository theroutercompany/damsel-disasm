//! `damsel-decomp` — the decompiler pipeline (Wave 6 of
//! `docs/roadmaps/re-power-roadmap.md`).
//!
//! Stage D0 ships first: function summaries derived entirely from the
//! format-neutral project fact store — calls, imports, strings,
//! unresolved indirect transfers, bounds, problems. Later stages (IR,
//! SSA, structuring, pseudocode — strategy D1–D5) consume the same
//! facts plus Wave 5A semantics; no stage may emit only a string.

mod diff;
mod emulate;
mod ir;
mod pseudodiff;
mod recon;
mod render;
mod ssa;
mod stack_frame;
mod structure;
mod summary;
mod taint;
mod ttd;

pub use diff::{
    DefenseFinding, DiffError, DiffFunction, DiffReport, FunctionHistory, IntervalInput,
    MatchStrategy, MatchedFunction, ShapeMap, build_timeline, diff_defenses, diff_functions,
    diff_functions_with_shapes, render_diff_html, render_diff_markdown,
};
pub use emulate::{EmulationOutcome, Emulator, Memory};
pub use ir::{
    IrBinOp, IrBlock, IrCallTarget, IrCondition, IrExpr, IrFunction, IrStatement, IrTerminator,
    IrValue, lift_arm64, lift_x86_64,
};
pub use pseudodiff::{PseudocodeDiff, PseudocodeDiffLine, diff_lines, diff_pseudocode};
pub use recon::{
    AntiAnalysisIndicator, FlatteningIndicator, PackerSignature, ReconReport, SectionEntropy,
    Verdict, assess, detect_anti_analysis, detect_cfg_flattening, detect_opaque_predicates,
    detect_packers, detect_string_encryption, render_recon_markdown, shannon_entropy,
};
pub use render::{
    Pseudocode, PseudocodeLine, StructFieldHint, TypeHints, render_pseudocode,
    render_pseudocode_with_types,
};
pub use ssa::{
    AbiRegisterRole, AbiRole, PhiCandidate, PhiNode, RenamedBlock, RenamedFunction, SsaReport,
    arm64_abi_roles, build_phi_nodes, build_ssa, rename_registers, x86_64_abi_roles,
};
pub use stack_frame::{StackFrameFacts, analyze_stack_frame};
pub use structure::{
    Diamond, LoopBack, NaturalLoop, ResolvedCondition, find_diamonds, find_loops,
    find_natural_loops, resolve_condition,
};
pub use summary::{
    CallSite, FunctionSummaryReport, SummaryError, render_summary_markdown, summarize_function,
    summarize_named,
};
pub use taint::{IntraSummary, TaintEngine, TaintHop, summarize};
pub use ttd::{Trace, TraceStep, record_trace};
