# Wave 6 — Decompiler MVP Implementation Spec

Status: Proposed implementation spec for Wave 6 of the
[Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md).

## Why This Wave Exists

Function lists, xrefs and strings make navigation fast, but analysts still
read assembly to understand behavior. Pseudocode is the single largest
reading-speed multiplier for proprietary binaries. Competitors ship it, with
caveats; damsel must ship it **without** those caveats becoming lies.

This wave executes the existing
[Decompiler And IR Strategy](../hopper-gap/decompiler-ir-strategy.md)
milestones D0–D5 on top of Wave 4 (functions/xrefs/facts) and Wave 5A
(semantic opcodes), extends semantic coverage to x86_64, and raises the
bar the strategy already set: every line provable back to instructions.

Decision D-4 (2026-06-12): honesty gates are non-negotiable shipping
criteria **and** decompiler quality is a first-class investment. The MVP is
a floor, not a ceiling: quality work continues as a standing workstream
with corpus-growth gates every release (§7.1), and x86_64 control
structure/pseudocode (D4–D5) is required by Wave 7 exit rather than
lagging indefinitely.

## Scope

In scope:

- `damsel-decomp` crate: semantic opcodes → low-level IR → SSA/def-use →
  stack-frame recovery → type/ABI enrichment → control-structure recovery →
  pseudocode with per-line provenance.
- arm64/arm64e full D0–D5 (strategy §Milestones).
- x86_64 coverage through D2 (SSA/def-use) and D3 where ABI facts exist;
  D4–D5 x86_64 ships preview-level (explicitly labeled) at Wave 6 exit and
  is required by Wave 7 exit per Decision D-4.
- CLI: `decompile --function <id|addr|name>` (text/JSON), `summary`.
- Workbench/web UI: pseudocode pane synchronized with disassembly selection.
- User-assisted recovery: rename/retype/signature/hint transactions land in
  the Wave 4 kernel and trigger bounded re-decompilation.

Out of scope (hard):

- Recompilable source, whole-program alias analysis, C++ template
  reconstruction, Swift source reconstruction, binary rewriting through
  decompiler edits, interprocedural SSA (function summaries only),
  decompiling entire shared caches in one pass.

## Consumed Specifications (Normative)

- [Decompiler And IR Strategy](../hopper-gap/decompiler-ir-strategy.md) —
  IR shape, value kinds, terminator kinds, SSA scope, memory staging, stack
  frames, ABI model, overlays, renderer principles, D0–D6. This spec follows
  it; deltas below.
- [Decompiler Function IR spec](../hopper-gap/decompiler-function-ir-implementation-spec.md)
  — staged implementation contracts for procedure recovery through
  pseudocode.
- [Decompiler Quality And User-Assisted Recovery spec](../hopper-gap/decompiler-quality-user-assisted-recovery-implementation-spec.md)
  — pass ledger, source-map evidence, quality reports, hint transactions,
  regression corpus, quality gates. **These quality gates are shipping
  criteria for this wave, not stretch.**
- [Decompiler Pseudocode Workbench spec](../hopper-gap/decompiler-pseudocode-workbench-implementation-spec.md)
  — pane UX, line model, synchronization.
- Wave 4 spec — function records, facts, invalidation; Wave 5 spec —
  semantic opcodes, x86_64 producer.

Deltas added by this spec:

1. Cross-ISA scheduling: which D-stages are required per ISA at exit (§7).
2. Pseudocode acceptance corpus with per-ISA expectation files (§8).
3. Bail-out taxonomy mapping into Wave 4 problem codes (§4.4).
4. MCP/batch exposure of `decompile` is defined here but enabled in Wave 7
   (§9) — the JSON contract is frozen in this wave.

## 1. Pipeline

```text
Wave 4 FunctionRecord + Wave 5A InstructionSemantics
  → 1. Lift: per-basic-block IR (strategy §Low-Level IR)
  → 2. Normalize: nop/pac-placeholder removal, PLT thunk folding, stubs →
       declared imports, branch-chain threading
  → 3. SSA + def-use (per function), memory versioning (stack stage 1)
  → 4. Stack frame recovery (prologue/epilogue, SP/FP deltas, saved regs,
       slot typing from widths and uses)
  → 5. ABI enrichment (args/returns/clobbers per strategy §Calling Convention;
       ObjC msgSend/MSGSendStret conventions; Itanium `this`)
  → 6. Type seeds (Wave 4 symbols/strings, ObjC encodings, demangled
       signatures, SDK packs later)
  → 7. Overlays (ObjC message-send classification, C++ virtual-call candidate
       tables, string annotations, fixup/PAC intrinsics)
  → 8. Control structure (if/else, loops, switches from Wave 4 jump tables,
       early returns; unreachable blocks surfaced, never hidden)
  → 9. Render (line model + provenance + uncertainty markers)
```

Each stage writes facts into the project (kernel vocabulary) and a
`pass_ledger` entry (quality spec). No stage may only emit a string.

## 2. IR And Data-Flow Contracts

Follow the strategy document's IR value/statement/terminator kinds verbatim;
two implementation notes:

- IR is serializable and snapshot-tested per function (`ir.snap`) so diffs
  of analysis changes are reviewable.
- Memory model starts at strategy stage 1 (stack, constant globals, literal
  pools, stub/import tables) and admits stage-2 items only where Wave 4
  evidence exists (selector/class refs, vtable loads, fixup targets).

Determinism: per-function IR and pseudocode are canonical given (bytes,
analyzer version); phi ordering, value ids, and statement ids follow
deterministic traversal order.

## 3. Per-ISA Semantics Gaps

- arm64: strategy's instruction table is the floor (adr/adrp/add/sub/
  ldr/str/ldp/stp/mov/movk/movz/orr/cmp/tst/ccmp/b/bl/blr/br/ret/b.cond/
  tbz/cbz/jump tables/PAC family/ObjC stubs).
- x86_64 (new): mov/movz*/lea/add/sub/and/or/xor/shl/shr/sar/imul/idiv/cmp/
  test/jcc/jmp/call/ret/endbr64/syscall/setcc/cmov/PLT thunks; `rdi/rsi/
  rdx/rcx/r8/r9` arg registers, `rax` return, `rbp/rbx/r12–r15` callee-saved,
  `rsp` frame; SysV red-zone accesses (leaf functions without frames) are
  modeled as stack memory below the recovered frame — no named slots in
  this wave, and conflicting evidence surfaces as problems rather than
  guessed slots.
- Any instruction without semantics is lifted as `Unknown` + problem code
  `decomp_unsupported_semantics`; the renderer shows `unknown_op(...)` in
  place of fabricated arithmetic.

## 4. Honesty Contract

### 4.1 Line Model

Every pseudocode line carries `source_addresses: [Addr]`, `confidence`,
`problems: [ProblemId]`, `spans` (semantic spans for routes). Clicking a
line selects its instructions; hovering shows provenance.

### 4.2 Uncertainty Markers

`unknown`, `candidate`, `ambiguous`, `unresolved`, `/* inferred */`,
`/* user asserted */` per the strategy. Renderer must preserve them through
copy/paste (they are part of the text, not styling).

### 4.3 Dead / Unreachable Code

Never silently dropped. Unreachable blocks render behind an explicit toggle
and always produce an informational problem when omitted from the default
view.

### 4.4 Bail-Out Taxonomy (maps to Wave 4 problems)

| Code | Meaning | Rendering |
| --- | --- | --- |
| `decomp_unsupported_semantics` | IR lift gap | line with `unknown_op` |
| `decomp_unresolved_indirect` | target set empty | `call unknown_target()` |
| `decomp_ambiguous_targets` | >1 plausible target | `candidate` list |
| `decomp_low_confidence` | structural recovery weak | `/* low confidence */` block |
| `decomp_irreducible` | CFG not reducible | switch-goto form with note |
| `decomp_stack_conflict` | overlapping slot evidence | slot shown as `sN /* ? */` |
| `decomp_type_conflict` | contradictory type facts | both types + `/* conflict */` |
| `decomp_reanalysis_pending` | hints invalidated output | stale banner |

### 4.5 Clobber And Effect Conservatism

Unknown calls clobber per ABI only; memory effects conservatively `may_write`
unless proven otherwise; loop-carried values that cannot be proven become
`unknown` rather than being invented.

## 5. User-Assisted Recovery

Hints (kernel transactions; quality spec §hints): function bounds, code/data/
string marking, manual xref, jump-table base/count/stride, ObjC receiver
binding, signature/type assignment, calling convention, stack variable name
and type, virtual target choice. Flow: problem → hint → transaction →
bounded invalidation (function scope first) → targeted re-decompile →
provenance update. Wave 6 ships a minimal CLI hint set (`set-signature`,
`set-calling-convention`, `mark-jump-table`, `bind-selector`) and the
Workbench panes for the same; the semantic-editing spec owns broader UX.

## 6. CLI And API Contracts

```text
damsel decompile <project> --function <fn-id|addr|name> [--format text|json]
                                           [--include-ir] [--include-ssa]
damsel decompile <project> --function <…> --show-unreachable
damsel summary  <project> --function <…>        # D0 summary without pseudocode
damsel decompile <project> --function <…> --hints <file.json>  # replay hints
```

JSON contract (frozen this wave, consumed by Wave 7 MCP):

- `function`: id, name, bounds, signature (with provenance), stack frame,
  problems
- `lines[]`: id, indent, text, spans, source_addresses, confidence,
  problems, route
- `ir` (opt-in): blocks, statements, values, terminators, SSA summary
- `provenance`: pass ledger ids, analyzer version
- `stale`: bool + reason when hints changed inputs

`GET /api/projects/{id}/functions/{fn}/decompile?include=lines,ir` mirrors
the JSON; the UI API keeps the same route/id vocabulary.

## 7. Per-ISA Exit Gates

| Stage | arm64 | x86_64 |
| --- | --- | --- |
| D0 summaries | required | required |
| D1 low-level IR | required | required |
| D2 SSA/def-use | required | required |
| D3 types/ABI | required | required (SysV) |
| D4 control structure | required | required by Wave 7 exit (D-4); preview-level with explicit labeling allowed at Wave 6 exit |
| D5 pseudocode UI | required | required by Wave 7 exit (D-4); preview-level with explicit labeling allowed at Wave 6 exit |

### 7.1 Quality Trajectory (standing workstream, D-4)

Decompiler quality does not end at wave exit — Decision D-4 makes it a
first-class, ongoing investment. Every release after Wave 6 must show,
via the decompiler corpus bench:

- no regression in the gate percentages below;
- corpus growth: ≥ 5 new fixture functions per release, prioritized from
  real-analysis bail-outs (every `decomp_*` problem observed in the wild is
  a candidate fixture);
- the bail-out reason distribution in release notes (what improved, what
  regressed, what remains).

Ambition: match honest-output best practice first (every line
provenance-carrying), then measurably close the readability gap to mature
decompilers — never by weakening uncertainty markers.

Reading-quality gate (both ISAs where D5 claimed): ≥ 80 % of Tier-1/Tier-2
fixture functions decompile with **zero** `low_confidence`/`irreducible`
bail-outs and all lines address-mapped; ≥ 95 % address-mapped overall.
No function may render fabricated types: every typed expression carries
provenance (`inferred_abi`, `objc_encoding`, `user_asserted`, …).

## 8. Fixture And Regression Corpus

Extends the corpus plan:

- Tier 1/2/3 sets per the strategy (arithmetic/branches/stack; loops/
  switches/tail calls/ObjC/vtables; PAC/chained fixups/kernel-style tables).
- New proprietary-flavored fixtures: `decomp-stripped-binary` (symbols
  hidden), `decomp-switch-obfuscated` (flattened CFG), `decomp-inlined`
  (`-O3` aggressive inlining), `decomp-error-paths` (long early-return
  chains), `decomp-variadic`, `decomp-objc-blocks`.
- Golden pseudocode snapshots per function with a review policy: any change
  to rendered text requires an explicit rationale in the PR (quality spec's
  ledger diff discipline).
- Negative tests (mandatory): unresolved indirect branch stays unresolved;
  ambiguous selector stays ambiguous; type conflict renders conflict;
  unsupported instruction emits `unknown_op`; unreachable code toggle
  preserves hidden blocks.
- Cross-ISA paired fixtures: same source built arm64/x86_64; summaries must
  agree on call/import/string sets.

## 9. Automation Contract (Frozen, Enabled In Wave 7)

The `decompile` JSON contract above is the MCP tool payload. It must not
change shape between Wave 6 exit and Wave 7 MCP landing without a versioned
contract test. Pseudocode text is never a tool's *primary* output; lines +
spans + provenance are.

## 10. Budgets

| Budget | Target |
| --- | --- |
| Typical function (≤ 200 blocks) decompile | ≤ 250 ms |
| Large function (≤ 2,000 blocks) | ≤ 3 s, cancellable |
| SSA memory per typical function | ≤ 1 MB |
| Re-decompile after hint (function scope) | ≤ 100 ms typical |
| UI pseudocode pane first paint (cached) | ≤ 50 ms |

## 11. Risks

| Risk | Mitigation |
| --- | --- |
| Overclaiming reads as a "real decompiler" | bail-out taxonomy, quality gates, uncertainty markers part of text, honest README language |
| x86_64 D4–D5 quality | preview-level with explicit labeling at Wave 6 exit; required by Wave 7 exit (D-4); fixture-gated claims |
| SSA memory blowup on obfuscated CFGs | per-function budget, cancellable jobs, `decomp_irreducible` bail-out instead of hanging |
| Hint invalidation storms | function-scoped invalidation default, batch re-decompile only on explicit command |
| Golden-snapshot churn | review policy: text changes require rationale; IR snapshots diffed mechanically |

## 12. Open Questions

1. Stack variable identification naming (`s0`, `var_18`, or offset-based) —
   must be stable across reanalysis to avoid snapshot churn; pick a scheme
   in D2 and document it.
2. Whether x86_64 D3 ABI facts reuse SysV conventions from a shared
   `damsel-core::abi` module — likely yes; decide in the x86 D3 PR.
3. Pseudocode pane rendering technology in Workbench (SwiftUI `Text` with
   attributed runs vs a custom `NSTextView`) — measure at D5 start against
   the 16 ms interaction budget.

## 13. References

- [Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md) §5 Wave 6
- [Decompiler And IR Strategy](../hopper-gap/decompiler-ir-strategy.md)
- [Decompiler Function IR spec](../hopper-gap/decompiler-function-ir-implementation-spec.md)
- [Decompiler Quality spec](../hopper-gap/decompiler-quality-user-assisted-recovery-implementation-spec.md)
- [Decompiler Pseudocode Workbench spec](../hopper-gap/decompiler-pseudocode-workbench-implementation-spec.md)
- [Semantic editing spec](../hopper-gap/semantic-editing-reanalysis-hints-implementation-spec.md)
- [Wave 5 spec](wave-5-elf-x86-spec.md), [Wave 4 spec](wave-4-analysis-kernel-spec.md)
