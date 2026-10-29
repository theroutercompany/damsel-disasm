# Reverse-Engineering Power Roadmap (macOS + Linux)

Status: Accepted 2026-06-12 (implementation not yet started). This document
defines the next product phase, Waves 4–8,
whose goal is to make `damsel` genuinely powerful at reverse-engineering
proprietary macOS and Linux binaries efficiently.

Waves 1–3 of the [Apple Runtime Analysis Roadmap](apple-runtime-analysis-roadmap.md)
are implemented. This phase continues the wave numbering and amends the
Apple-only scope lock of [ADR-0001](../adr/ADR-0001-apple-runtime-focus.md)
through [ADR-0003: Cross-Platform RE Scope](../adr/ADR-0003-cross-platform-re-scope.md).

Implementation companions live in the [RE Power documentation pack](../re-power/README.md):

- [Current State Audit](../re-power/current-state-audit.md)
- [Competitive Landscape](../re-power/competitive-landscape.md)
- [Wave 4 Spec](../re-power/wave-4-analysis-kernel-spec.md)
- [Wave 5 Spec](../re-power/wave-5-elf-x86-spec.md)
- [Wave 6 Spec](../re-power/wave-6-decompiler-spec.md)
- [Wave 7 Spec](../re-power/wave-7-efficiency-automation-spec.md)
- [Wave 8 Spec](../re-power/wave-8-proprietary-depth-spec.md)
- [Evidence And Acceptance Matrix](../re-power/evidence-and-acceptance-matrix.md)
- [Research Sources](../re-power/research-sources.md)

## 1. Product Thesis

Proprietary-binary reverse engineering is slow because analysts drown in
point queries: no symbols, no cross-references, no types, and no memory
between sessions. The tools that feel fast — Hopper, Ghidra, IDA, Binary
Ninja — win on three axes:

1. **Whole-image indexing** that makes navigation instant.
2. **Recovered semantics** — functions, xrefs, types, pseudocode — instead of
   raw bytes.
3. **Knowledge reuse** — demanglers, SDKs, symbol corpora, diffs, scripts.

`damsel` already owns the hardest Apple-specific foundations: a typed core
model, a dyld shared-cache container with cross-image linkage, an ObjC
metadata layer, and a window-local arm64/arm64e analysis engine with value
recovery, jump tables, and stub resolution — all under a deterministic typed
CLI/JSON discipline with evidence-backed validation infrastructure.

This phase converts those point queries into an **indexed, persistent,
cross-platform, scriptable RE workbench** without abandoning that baseline:

- Every new fact is typed, deterministic, provenance-tagged, and testable.
- Infrastructure is format- and ISA-neutral before the second format lands.
- Every efficiency claim is a measured budget with a CI gate, not prose.
- Proprietary-binary pain (stripped, packed, obfuscated, version-churned) is
  attacked directly with dedicated surfaces.

## 2. Users And Jobs To Be Done

| Persona | Job to be done | Primary waves |
| --- | --- | --- |
| Security researcher, macOS | Audit a closed-source app/daemon: entry points, ObjC surface, strings, network/privacy-relevant behavior | 4, 6, 7 |
| Malware / DRM / anti-cheat analyst | Triage a packed or obfuscated binary fast; find the real payload; track version churn | 4, 7 (recon, diff) |
| Vulnerability researcher | Diff two versions of a proprietary library to isolate the patched bug | 7 (diff), 4 (xrefs), 6 |
| Interop / compatibility engineer | Understand proprietary `.so`/framework linkage: versioned imports, reexports, ABI drift | 5, 7 (diff) |
| Tooling / AI engineer | Drive binary analysis headlessly from agents and pipelines with evidence, not prose | 7 (MCP, batch) |

Non-users this phase explicitly does not chase: firmware/IoT image packers
beyond recon, game-modding communities needing interactive debuggers, and
full-source recompilation use cases.

## 3. Current State Summary

Grounded in the [Current State Audit](../re-power/current-state-audit.md);
key facts only:

- **Formats/ISAs:** Mach-O only, arm64/arm64e. `BinaryFormat` has a single
  variant; thin x86_64 Mach-O is a typed rejection; ELF does not exist.
- **Analysis:** window-local — basic blocks, CFG edges, value-flow-lite
  recovery, jump tables, import/stub/ObjC references. No whole-image index,
  no function model, no xref table, no string inventory.
- **Persistence:** none. Every command re-opens the image; the UI server is
  an in-memory store keyed by upload id.
- **Decoders:** Capstone (arm64 detail mode) is the only decoder path.
  Live-verified 2026-10-28: the workspace enables only the `arch_arm64`
  Capstone feature (capstone-rs `full` means non-Diet mode, not
  all-architectures), so x86 decode needs the `arch_x86` feature added —
  a one-line change to the existing dependency, no new native dependency.
- **CLI/UI:** 25+ point-query commands with deterministic text/JSON output;
  axum UI server; native SwiftUI Workbench wrapping that server.
- **Verification:** fixture matrices with drift manifests, insta snapshots,
  JSON contract tests, criterion benches, LLVM external comparison,
  real-cache evidence packaging/auditing, Nix-pinned toolchain.

What does not exist anywhere in the toolchain today — and is therefore the
phase's net-new surface: project persistence, functions, xrefs, strings,
search, a second format, a second ISA, decompilation, diffing, automation
servers, obfuscation triage, and patching.

## 4. Relationship To Existing Programs

The [Hopper Gap pack](../hopper-gap/README.md) is an Apple-only program of
twelve milestones with detailed implementation specs. The RE Power program
does not replace it; it **sequences, generalizes, and funds** its
format-neutral core while the Apple-specific depth continues in parallel:

| RE Power wave | Consumes / generalizes (hopper-gap milestones) | Adds that is net-new |
| --- | --- | --- |
| 4 — Analysis kernel | M1 project foundation, M1A document primitives, M2 function/xref index ([analysis database blueprint](../hopper-gap/analysis-database-blueprint.md), [project kernel spec](../hopper-gap/project-kernel-byte-map-implementation-spec.md), [function recovery plan](../hopper-gap/function-recovery-and-procedure-database-plan.md)) | strings inventory, unified search, corpus-scale budgets, format/ISA neutrality as a hard requirement |
| 5 — ELF + x86_64 | M5 C++ explorer (generalized to both platforms) | `damsel-elf` crate, x86_64 decode, semantic-opcode layer, ELF dynamic-linkage model, Linux fixture/evidence lane |
| 6 — Decompiler | M9 summaries, [decompiler IR strategy](../hopper-gap/decompiler-ir-strategy.md) D0–D6, IR + quality specs | x86_64 semantics lane, cross-ISA scheduling, pseudocode acceptance corpus |
| 7 — Efficiency/automation | M10 read-only automation, M10B diffing ([automation/MCP spec](../hopper-gap/automation-studio-scripting-plugin-workbench-implementation-spec.md), [diffing spec](../hopper-gap/apple-system-diffing-export-evidence-implementation-spec.md)) | batch/JSONL mode, `recon` obfuscation triage, ELF version-diff profile, measured perf gates |
| 8 — Proprietary depth | M12 patching, M8 debug-info subset, [SDK semantic packs](../hopper-gap/apple-sdk-semantic-packs-implementation-spec.md), [Swift metadata spec](../hopper-gap/swift-runtime-metadata-implementation-spec.md) | bounded patch encoder (no GPL assembler), Linux symbol-reuse corpora, anti-analysis catalog |

Apple-only milestones (M4 ObjC explorer, M6 kernel, M7 PAC, M8 DWARF, M11
debugger bridge) proceed on their own track and are consumed by RE Power
waves where they add cross-platform value (ObjC seeding feeds Wave 4
functions; PAC facts feed Wave 6 intrinsics). The debugger bridge remains
explicitly deferred in this phase.

## 5. Wave Definitions

### Wave 4 — Analysis Kernel: Project Database, Functions, Xrefs, Strings, Search

Goal: give damsel memory. A persistent, versioned project database per
target; whole-image automatic analysis; a function model with provenance and
confidence; bidirectional cross-references; a string inventory joined to
xrefs; unified ranked search.

Scope highlights (details in the [Wave 4 spec](../re-power/wave-4-analysis-kernel-spec.md)):

- `damsel-project` foundation per the analysis-database blueprint: project,
  source, address space, object ids, facts with provenance/confidence, byte
  classifications, problems, jobs. Storage (Decision D-3): the real
  embedded SQLite store ships from the first implementation PR — WAL,
  versioned migrations, event-log table, paged indexes. Canonical exports
  (not the database file) are the determinism artifact.
- Function discovery as evidence fusion: exact seeds (symbol table, function
  starts, cache exports, ObjC method IMPs, ELF FDEs once Wave 5 lands) before
  inferred seeds (prologue templates, direct call targets, unreferenced code
  sweeps), with merge policy, conflict records, and per-bound confidence.
- Xref harvesting by generalizing the existing reference synthesis
  (`disasm.rs` annotations/`DisassemblyAnalysis`) into stored facts: direct
  calls/branches, indirect targets, data references, string references,
  import/stub/GOT slots, ObjC selector/class refs.
- Strings: typed per-section extraction (UTF-8/UTF-16, length floor), each
  candidate joined to referencing instructions.
- CLI: `damsel analyze`, `functions`, `xrefs --from/--to`, `strings`,
  `search` — all with the existing text/JSON duality and stable key order.

Exit criteria (abbreviated): end-to-end `analyze → xrefs` on a stripped
30 MB binary inside the efficiency budgets; existing point-query commands
unchanged; Workbench and web UI can list functions and navigate xrefs.

### Wave 5 — ELF Lane And x86_64

Goal: Linux targets as peers. Proprietary ELF executables, shared libraries,
and daemons on x86_64 and aarch64; one analysis pipeline for two ISAs.

Scope highlights (details in the [Wave 5 spec](../re-power/wave-5-elf-x86-spec.md)):

- **Wave 5A (prerequisite refactor):** a typed semantic-opcode layer. Per-ISA
  decoders emit `InstructionSemantics` (call/branch/load/store/address
  materialization/auth-branch classes with typed operands); the analysis
  engine queries semantics instead of matching mnemonic strings. Landed
  incrementally behind the existing test matrices, zero behavior change
  allowed at each step.
- **Wave 5B:** `damsel-elf` crate mirroring `damsel-macho`: ELF64 loading
  (goblin/object are already dependencies), sections/segments/symbols,
  dynamic linkage (DT_NEEDED/SONAME/RPATH, versioned symbols
  `.gnu.version`/`.gnu.version_r`, PLT/GOT layouts including `.plt.sec`
  full-RELRO form, IFUNC/IRELATIVE), relocations mapped into the existing
  `ImportBindingRecord` model, init/fini arrays, `.eh_frame` FDE function
  seeds (gimli), static stripped binaries, `.note.gnu.build-id` identity.
- x86_64 decode via the Capstone x86 backend — one prerequisite: add the
  `arch_x86` feature to the existing `capstone` dependency (capstone-rs
  `full` means non-Diet mode, not all-architectures; verified 2026-10-28);
  then teach the
  analysis engine variable-length windows, RIP-relative addressing, x86 jump
  table idioms, `endbr64` CET prologues.
- Cross-platform C++ ABI recovery (consumes hopper-gap M5): Itanium
  demangler, RTTI/typeinfo, vtable discovery and virtual-call candidate
  tables, Rust v0 symbol demangling for the growing proprietary-Rust surface.
- `doctor` capability rows (`elf-analysis`, `elf-fixture-rebuild`) and an
  honest README support-matrix update.

Exit criteria (abbreviated): `info/sections/symbols/imports/disasm/xrefs`
work on ELF64 x86_64 and aarch64 including static stripped binaries; the
same pipeline emits function/xref/string facts for both ISAs.

### Wave 6 — Decompiler MVP

Goal: function-scoped pseudocode as the primary reading surface for
proprietary code — honest, provenance-carrying, correctable.

Scope highlights (details in the [Wave 6 spec](../re-power/wave-6-decompiler-spec.md)):

- Execute the existing [decompiler IR strategy](../hopper-gap/decompiler-ir-strategy.md)
  milestones D0–D5 for arm64/arm64e (summaries → low-level IR → SSA/dataflow
  → types/ABI → structuring → pseudocode), with D1–D2 extended to x86_64
  semantics during the wave.
- Every pseudocode line maps to the instructions that produced it; unresolved
  facts stay visible; typed bail-outs (`unsupported`, `low-confidence`,
  `irreducible`) instead of silently wrong output.
- User-assisted recovery: renames/retype/signature edits recorded as project
  facts (Wave 4) that trigger bounded re-decompilation.

Exit criteria (abbreviated): a majority of fixture-corpus functions decompile
to readable pseudocode on arm64; analysts can correct names/types from CLI
and Workbench and see the effect; adversarial fixtures prove no overclaiming.

### Wave 7 — Efficiency, Automation, Diff, And Workbench Power

Goal: make the fast loops measurably fast, and open the tool to agents and
pipelines.

Scope highlights (details in the [Wave 7 spec](../re-power/wave-7-efficiency-automation-spec.md)):

- Performance engineering: parallel decode/analysis, incremental
  invalidation over project facts, budgeted background scheduler, sampled
  then full dyld-cache indexing.
- Headless surfaces: `--jsonl` batch mode for corpus sweeps; `damsel serve`
  with an MCP server exposing typed, paged, provenance-carrying tools
  (`functions`, `xrefs`, `decompile`, `strings`, `search`, `diff`, `recon`)
  per the automation/MCP spec's privacy and evidence principles.
- Diff mode: two projects in, function-level match (exact → runtime identity
  → structural → fuzzy with explainable scores), byte/section deltas,
  semantic deltas (imports, strings, xref movement), paged findings; adds the
  `elf_version_diff` profile next to the Apple diff profiles.
- `damsel recon`: first-touch triage for proprietary binaries — stripped-ness,
  section entropy profile, packer signatures, CFG-flattening and
  opaque-predicate heuristics (leveraging the existing value-flow engine to
  *prove* constant-condition branches), anti-debug/anti-VM indicators.
- Workbench: function/xref navigation, call graph, pseudocode pane, diff
  view, casebook sessions — keyboard-first per the interaction specs.

Exit criteria (abbreviated): an analyst can index, search, xref-navigate,
decompile, and diff two versions of a proprietary binary without leaving
damsel; an agent can do the same headlessly; all budgets enforced in CI.

### Wave 8 — Proprietary-Target Depth (Gated)

Goal: the specialized surfaces that only matter once the core loop is fast.
Strictly post-Wave-7; each item has its own gate (details in the
[Wave 8 spec](../re-power/wave-8-proprietary-depth-spec.md)):

- Patch studio MVP: transactional byte patches with preview, undo, checksums,
  copy-on-write export; a **bounded built-in encoder vocabulary** (arm64
  branch/NOP/immediate rewrites; x86_64 via the iced-x86 encoder) instead of
  a GPL-licensed general assembler.
- Symbol reuse at scale: match stripped proprietary targets against macOS SDK
  semantic packs, glibc/libstdc++ versioned-symbol corpora, and distro
  debuginfo-derived name sets to auto-name imports, stubs, and library code.
- Swift runtime metadata recovery and arm64e/PAC navigation polish (consumes
  the Apple specs).
- Anti-analysis catalog: tested detections for common proprietary defenses.

## 6. Efficiency Budgets

Harmonized with the existing
[performance architecture](../hopper-gap/performance-responsiveness-architecture.md)
and [database blueprint](../hopper-gap/analysis-database-blueprint.md)
budgets; RE Power adds the corpus-scale rows. All budgets are CI-enforced via
the corpus benchmark regression plan; hardware class is recorded with each
gate.

| Budget | Target | Gate |
| --- | --- | --- |
| Whole-image index build, 30 MB stripped binary | ≤ 30 s | corpus bench, per-commit |
| Indexed point query (function detail, xref lookup) | ≤ 50 ms p95 | corpus bench |
| Global search first page (warm) | ≤ 150 ms p95 | corpus bench |
| Project open (warm, indexed 30 MB) | ≤ 1 s | corpus bench |
| Project open (sampled dyld-cache set) | ≤ 5 s | corpus bench |
| Decode throughput, multithreaded | ≥ 100 MB/s sustained | decode bench |
| Batch corpus sweep startup overhead | ≤ 5 % of wall time | batch bench |
| UI row selection → inspector | ≤ 50 ms | workbench measurement plan |
| Decompilation, typical function (≤ 200 blocks) | ≤ 250 ms | decompiler corpus |
| Diff, two 30 MB projects | ≤ 60 s with paged first results ≤ 2 s | diff bench |

## 7. Sequencing And Dependencies

```text
ADR-0003 (decision)
  |
  v
Wave 4 (kernel: projects/functions/xrefs/strings/search)   Wave 5A (semantic opcodes)
  |                                |                          |
  |                                v                          |
  |                        Wave 5B (ELF + x86_64 decode) <----+
  |                                |
  +----------------+---------------+
                   |
                   v
             Wave 6 (decompiler: arm64 full, x86_64 D1–D2)
                   |
                   v
             Wave 7 (efficiency, MCP, diff, recon, workbench)
                   |
                   v
             Wave 8 (gated: patching, symbol reuse, Swift/PAC depth)
```

Rules:

- Wave 4 and Wave 5A run in parallel from day one; neither blocks the other.
- Wave 5B depends on ADR-0003 only; it can start while Wave 4 lands.
- Wave 6 requires Wave 4 (function records, xrefs) and 5A (semantics).
- Wave 7 requires Wave 4; benefits progressively from 5B and 6.
- Wave 8 requires Wave 7 exit plus its own per-item safety gates.
- Apple-track hopper-gap milestones interleave freely; the shared rule is
  that no workstream invents its own model — everything lands in the common
  fact/provenance vocabulary.

## 8. Verification Strategy

The phase inherits the repo's verification discipline and extends it:

- Typed failure classes, deterministic JSON key ordering, insta snapshots for
  every new command and object shape.
- Fixture matrices with drift manifests per format: the macOS lane keeps
  Xcode-rebuilt fixtures; the Linux lane adds a clang/gcc-rebuilt ELF corpus
  (stripped/unstripped, C/C++/Rust, PIE, static, versioned symbols, IFUNC)
  rebuilt in CI on Linux runners with the same hash-manifest discipline.
- External comparison: the existing `bin/disasm-external-compare.sh` pattern
  extended to `llvm-objdump -d`/`readelf` on ELF; the LLVM comparison becomes
  a drift gate for both ISAs.
- Evidence packaging: a real-ELF evidence sweep sibling to the real-cache
  evidence scripts — distro-built binaries with known ground truth, packaged,
  verifiable, auditable.
- Corpus benchmark regression plan becomes the CI performance gate for the
  efficiency budgets above.
- Every wave updates README/doctor honestly about implemented versus
  planned surfaces (the repo's standing rule).

## 9. Risks And Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| x86_64 variable-length decode complicates windowing and analysis | Wave 5 slips | semantic-opcode prerequisite; x86_64 decompiler coverage is preview-level at Wave 6 exit and required by Wave 7 exit (D-4); x86 fixture matrix mirrors the arm64 one |
| ELF breadth creep (Android, 32-bit, kernels) | focus loss | explicit non-goals; `.ko` behind a gate; doctor rows record the honest boundary |
| Decompiler overclaiming | trust damage | bail-out classes, provenance lines, adversarial fixtures, quality-gate spec consumed as-is |
| Persistence format churn | broken projects | real embedded store from day one with versioned migrations (D-3); canonical exports as the determinism artifact; schema changes ship with N-1 fixtures |
| Performance regressions during refactor | user-visible slowdowns | budgets enforced per-commit via corpus benches, not anecdotes |
| Scope collision with hopper-gap pack | duplicated models | consumption map in §4 is normative; shared fact vocabulary is the only landing zone |
| GPL contamination from assembler dependencies | licensing | bounded built-in encoder + iced-x86 (MIT) decision recorded in the Wave 8 spec |

## 10. Decision Log

All five program decisions were accepted by the maintainer on 2026-06-12.
They bind the wave specs; changes require a logged supersession.

| Id | Decision | Consequence |
| --- | --- | --- |
| D-1 | Expand scope to Linux ELF + x86_64 (ADR-0003 accepted) | Wave 5 unblocked; Apple depth continues unchanged |
| D-2 | Pay for the semantic-opcode refactor before x86_64 analysis | Wave 5A is a hard prerequisite; engine stays single |
| D-3 | Build the real embedded project database from day one — not the staged snapshot-first rollout | Wave 4 storage section rewritten: SQLite-backed store, versioned migrations, and WAL from the first implementation PR; canonical exports remain the determinism artifact |
| D-4 | Decompiler: honesty gates are non-negotiable shipping criteria **and** quality is a first-class investment — "work hard to make it good" | x86_64 D4–D5 required by Wave 7 exit (no indefinite lag); per-release corpus growth gates; quality is a standing workstream, not an MVP-and-move-on |
| D-5 | Read-only for now; patching stays gated and requires an explicit re-decision | Wave 8 item 8.1 stays behind its safety gate plus maintainer re-decision |

## 11. Glossary

- **Fact** — a typed, provenance-carrying assertion about a project object or
  range (analysis-database blueprint).
- **Provenance/confidence** — where a fact came from (analyzer pass, user,
  import) and how strong it is (`exact` … `weak`, `user_asserted`).
- **Xref** — a stored reference between two objects (instruction → function,
  instruction → string, function → import slot), queryable both directions.
- **Semantic opcode** — the ISA-neutral instruction classification emitted by
  per-ISA decoders (`DirectCall`, `Load`, `AddressMaterialization`, …).
- **Projected image** — a dyld-cache image materialized into per-image
  analysis semantics (existing Wave 2 capability).
- **Recon** — first-touch triage of an unknown proprietary binary: packing,
  obfuscation, stripped-ness, defensive indicators.
- **Evidence package** — a self-verifying, portable record of what analysis
  actually ran, used by the existing beta gates and extended to real-ELF
  corpora.

## 12. Phase Exit Definition

The phase is complete only when all of the following are true — not when the
code merges:

1. Every row in the
   [Evidence And Acceptance Matrix](../re-power/evidence-and-acceptance-matrix.md)
   for Waves 4–7 passes in CI, or is recorded as a deferred row with an owner
   and a date.
2. All efficiency budgets in §6 are enforced by CI benches on the recorded
   hardware classes; no budget is documented as "aspirational".
3. An analyst can complete the full loop on **both** platforms — index,
   search, xref-navigate, decompile, diff two versions, recon-triage — in the
   CLI, the Workbench, and via MCP.
4. README, `doctor`, and the support matrix describe exactly what exists;
   nothing implemented is unlisted, nothing planned is claimed.
5. Wave 8 items each pass their own gates or remain explicitly unstarted.
6. The Apple track (hopper-gap milestones) has consumed the shared kernel
   without forking its model — proven by cross-format schema equality tests.

## 13. Cross References

- [ADR-0001: Apple Runtime Focus](../adr/ADR-0001-apple-runtime-focus.md)
- [ADR-0002: Shared-Cache Container Model](../adr/ADR-0002-shared-cache-container-model.md)
- [ADR-0003: Cross-Platform RE Scope (accepted)](../adr/ADR-0003-cross-platform-re-scope.md)
- [Apple Runtime Analysis Roadmap](apple-runtime-analysis-roadmap.md)
  (Waves 1–3, implemented)
- [Disassembler V2 Alpha Plan](disassembler-v2-alpha-plan.md)
- [Hopper Gap Documentation Pack](../hopper-gap/README.md)
- [RE Power Documentation Pack](../re-power/README.md) (this phase's specs)
