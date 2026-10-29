# Wave 7 — Efficiency, Automation, Diff, And Workbench Power

Status: Proposed implementation spec for Wave 7 of the
[Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md).

## Why This Wave Exists

After Waves 4–6 damsel can analyze deeply; Wave 7 makes it **fast at scale
and usable by machines**. This is where damsel becomes category-unique:
headless batch sweeps, an MCP server returning evidence-backed facts, a
version-diff workflow for proprietary updates, first-touch obfuscation
triage, and a Workbench that survives thousands of functions.

## Scope

- Performance engineering: parallel whole-image analysis, incremental
  invalidation at scale, sampled-then-full shared-cache indexing, bounded
  memory for every store.
- Automation: `damsel serve` (JSON API + MCP server), `--jsonl` batch mode,
  typed tool schemas, capability grants and redaction per the automation
  spec.
- Diff: two projects in; exact/runtime/structural/fuzzy matching; findings;
  paged results; export profiles; ELF and Apple profiles.
- Recon: stripped-ness, entropy, packer signatures, CFG-flattening and
  opaque-predicate detection (using the value-flow engine to prove
  constant conditions), anti-debug/anti-VM indicators.
- Workbench: function/xref navigation, call graph, diff workspace, casebook
  sessions, keyboard-first command palette (consumes the interaction specs).

Out of scope (hard): debugger transport, plugin sandboxing beyond MCP
capability grants, patching (Wave 8), cloud/team features.

## Consumed Specifications (Normative)

- [Automation Studio, Scripting, MCP spec](../hopper-gap/automation-studio-scripting-plugin-workbench-implementation-spec.md)
  — capability model, tool principles, session binding, script runner
  boundaries, recipes.
- [Diffing, Export, Evidence spec](../hopper-gap/apple-system-diffing-export-evidence-implementation-spec.md)
  — diff pipeline, identity, fingerprints, profiles, conflict resolution,
  export profiles, Workbench diff workspace.
- [Performance architecture](../hopper-gap/performance-responsiveness-architecture.md)
  and [large-project scheduler spec](../hopper-gap/large-project-performance-scheduler-indexing-implementation-spec.md)
  — budgets, scheduler, paging.
- [Corpus benchmark regression plan](../hopper-gap/corpus-benchmark-regression-plan.md)
  — gates and tiers this wave enforces.
- [Command/navigation/undo spec](../hopper-gap/command-navigation-undo-workbench-implementation-spec.md)
  and [workbench browsers spec](../hopper-gap/apple-runtime-workbench-browsers-implementation-spec.md)
  — Workbench interaction contracts.
- [Evidence-grounded AI assistant spec](../hopper-gap/evidence-grounded-ai-assistant-workbench-implementation-spec.md)
  — evidence-first assistant principles applied to MCP tool design.
- Wave 4–6 specs — facts, semantics, decompile JSON contract.

## 1. Performance Engineering

### 1.1 Parallelism

- Whole-image pass driver fans per-function analysis across a work-stealing
  pool (`rayon`), with deterministic output regardless of scheduling:
  fact ids derive from (source, pass, address), never from completion order.
- Decode remains per-function; a batched line-level decoder is a bench
  experiment only if budgets are missed.
- Cache-wide indexing: image-parallel with a bounded worker pool; memory
  ceiling enforced by paging string/xref shards to the store (blueprint
  Stage 3 direction).

### 1.2 Incremental Invalidation

- Fact dependencies recorded (fact → inputs); a hint/byte-override
  invalidates the closure, not the world (blueprint scopes).
- Reanalysis reports invalidated ranges and re-rendered objects.
- Shared-cache sets: sampled index by default (user-visible sample manifest);
  full index is an explicit job.

### 1.3 Scheduler And UI Runtime

- Job classes: `foreground` (route), `interactive` (visible pane),
  `background`; preemption only at safe checkpoints (pass boundaries,
  per-function boundaries).
- Progress events at ≤ 250 ms cadence (existing budget); cancellation tokens
  honored inside decode loops (existing engine hooks).
- UI server switches to project-backed paged endpoints; the legacy upload
  path stays as compatibility mode.

### 1.4 Gate Set (CI-enforced)

The roadmap §6 table, plus: 1,000-function project open ≤ 1 s warm;
50k-function corpus (dyld-cache sample) search first page ≤ 150 ms; memory
≤ 2 GB for the 30 MB deep-analysis fixture; batch sweep of 100 binaries
startup overhead ≤ 5 %.

## 2. Automation

### 2.1 `damsel serve`

- Transport: local HTTP (axum) + MCP stdio and Streamable HTTP (per the
  2026-07-28 MCP specification; the legacy SSE transport is deprecated —
  verified 2026-10-28); loopback binding default;
  session token requirement when bound by the Workbench helper (trust spec).
- Objects expose the Wave 4 route/id vocabulary; every response carries
  ids, provenance, confidence, pages.
- Capability grants: read-only default; mutation/export/patch-plan grants
  opt-in with audit log entries (automation spec §Capability Model).

### 2.2 MCP Tools (first wave)

Read-only: `damsel_project_summary`, `damsel_list_sources`,
`damsel_resolve_route`, `damsel_search`, `damsel_get_function`,
`damsel_get_assembly_window`, `damsel_get_xrefs`, `damsel_get_graph`,
`damsel_get_string`, `damsel_decompile_function`, `damsel_recon_report`,
`damsel_diff_findings`, `damsel_get_problem`, `damsel_export_evidence_fragment`.

Design rules (from the spec, restated as hard contracts):

- Tool results use MCP `structuredContent` with an `outputSchema`
  (JSON Schema 2020-12) and `annotations` — verified against the 2026-07-28
  specification (Research Sources §0); our typed-JSON rule is the same
  mechanism, not an invention.
- JSON only, paged, ids/routes/provenance included, raw bytes only on
  explicit request, problems included, permission errors typed.
- Tool responses never prose summaries; text is a rendering of the same
  JSON (`damsel_decompile_function` returns lines + spans + provenance).
- Schemas versioned; contract tests freeze them per release.

Mutation tools (post-wave gate): `add_comment`, `set_label`, `add_tag`,
`apply_hint` — allowed only with grants + transactions + audit (the kernel
already supports it).

### 2.3 Batch Mode

```text
damsel batch --inputs list.txt --profile default --out results/
              [--format jsonl] [--jobs N] [--fail-fast|--continue]
damsel recon --batch list.txt --out recon/
```

- One process, one project open per input, deterministic JSONL records
  (`{"input", "status", "project", "summary", "problems"}`), partial
  failures typed and counted, exit code contract documented.
- Used by the corpus benches and by the real-ELF/real-cache evidence sweeps.

### 2.4 Recipes

Ship the automation spec's starter recipes as versioned JSON (ObjC selector
audit, dyld provider graph, C++ vtable sweep, ELF import audit, stripped
function recovery report) — executed by `damsel recipe run`.

## 3. Diff

### 3.1 Model And Pipeline

Follow the diff spec's pipeline and records verbatim (`DiffProject`,
`ObjectFingerprint`, `FunctionDiffRecord`, `RuntimeObjectDiffRecord`,
`DiffFinding`, ambiguity records). Profiles:

| Profile | Platform | Required matching |
| --- | --- | --- |
| `app_version_diff` | macOS | runtime + structural + fuzzy |
| `dyld_cache_build_diff` | macOS | image identity + runtime + fuzzy |
| `elf_version_diff` (**new**) | Linux | build-id fallback, versioned symbols, structural + fuzzy |
| `stripped_binary_diff` | both | structural + fuzzy only, strict ambiguity |

### 3.2 Signals And Scoring

Signals per the spec (normalized instruction sequence, block/edge counts,
call targets by matched object, string/selector/import sets, constants,
stack shape, CFG hash, decompile summary hash). Each signal contributes a
named, explainable score; thresholds are profile constants; no-match beats
misleading match.

### 3.3 Findings And Export

- Findings classes: function added/removed/changed, runtime object changes,
  import/linkage changes, string changes, xref movement, byte deltas at
  section granularity.
- Export profiles: evidence JSON (default, no raw bytes), Markdown/HTML
  report, `--include-strings` opt-in, redaction policy on paths/strings.
- Workbench diff workspace: side-by-side function view, finding list,
  pinned matches as annotations.

### 3.4 Fixture And Correctness Gates

- Injected-delta pairs: a single function/string/import change must be the
  exact reported diff (zero false negatives).
- False-positive budget: structural+fuzzy scores on unrelated binaries in
  the corpus must produce zero "changed" findings above threshold.
- ELF fixture pair rebuilt with one source-line change; dyld synthetic pair
  for the Apple profile.

## 4. Recon

```text
damsel recon <target> [--format text|json] [--deep]
```

Reports (each with provenance + confidence):

- Stripped-ness: symbols/debug presence, FDE-only recovery share.
- Entropy: per-section Shannon entropy, high-entropy run map.
- Packer/protector signatures: section layout anomalies, known-format
  fingerprints (UPX's documented `UPX!` ELF pack-header magic — verified
  from UPX source; common protectors), import anomalies (imports only of
  `dlopen`/`mmap`/`sysctl` families).
- Obfuscation: CFG flattening indicators (dispatch-loop shape, state
  variable patterns — the Obfuscator-LLVM `-fla` output class; also its
  `-bcf` bogus-control-flow and `-sub` substitution classes),
  opaque-predicate candidates where Wave 4 value-flow proves a constant
  condition (`mov w0, #1; cbz w0, …`), string encryption indicators
  (decrypt loop shapes feeding `__cstring`-less string use).
- Anti-analysis: `ptrace` self-attach patterns, `sysctl` debugger checks,
  timing checks, `getppid`/`argv[0]` probes, arm64e PAC misuse signals.
- Verdict block: likely packed / likely obfuscated / likely clean, with the
  evidence lines that justify it.

Gates: labeled fixture corpus (packed vs not, flattened vs not) with
precision/recall thresholds recorded; every claim must cite an evidence
line (no vibes).

## 5. Workbench

- Function list (virtualized, filterable), xref panes, call graph view
  (bounded, explorable), pseudocode pane (Wave 6), diff workspace, recon
  summary, casebook sessions/timeline (investigation-casebook spec),
  keyboard-first palette with route history (command/navigation spec).
- Latency gates from the performance spec enforced in the workbench
  measurement plan; no main-thread analysis.

## 6. Budgets Summary

| Surface | Budget |
| --- | --- |
| MCP `get_xrefs` first page | ≤ 150 ms |
| `decompile_function` (cached) | ≤ 300 ms including MCP transport/render overhead (engine budget remains ≤ 250 ms, Wave 6 §10) |
| Batch per-binary overhead | ≤ 5 % wall |
| Diff first findings page | ≤ 2 s |
| Diff full, two 30 MB | ≤ 60 s |
| Recon full pass (30 MB) | ≤ 20 s |
| Workbench selection → paint | ≤ 50 ms |

## 7. Risks

| Risk | Mitigation |
| --- | --- |
| Automation surface becomes an unstable API | versioned schemas, contract tests, capability grants, read-only default |
| Diff false confidence | explainable scores, ambiguity records, no-match bias, injected-delta gates |
| Recon heuristics overclaim | evidence-line citations, labeled corpus gates, wording rules (candidate/indicated/likely) |
| Parallel analysis nondeterminism | address-derived ids, completion-order-free output, determinism tests with shuffled scheduling |
| Workbench scope explosion | consume the existing interaction/browser specs; ship panes, not a redesign |

## 8. Open Questions

1. MCP transport default (stdio vs Streamable HTTP) and session lifecycle
   for agents — decide with the first consumer.
2. Whether `serve` embeds the UI server or runs alongside it — embed to keep
   one route vocabulary; confirm during the endpoint migration.
3. Recon "deep" mode cost ceiling on packed binaries (entropy of unpacked
   sections requires emulation) — default to static-only; emulation is a
   post-wave item.

## 9. References

- [Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md) §5 Wave 7
- [Automation/MCP spec](../hopper-gap/automation-studio-scripting-plugin-workbench-implementation-spec.md)
- [Diffing/evidence spec](../hopper-gap/apple-system-diffing-export-evidence-implementation-spec.md)
- [Performance architecture](../hopper-gap/performance-responsiveness-architecture.md)
- [Scheduler/indexing spec](../hopper-gap/large-project-performance-scheduler-indexing-implementation-spec.md)
- [Corpus benchmark regression plan](../hopper-gap/corpus-benchmark-regression-plan.md)
- [Workbench interaction specs](../hopper-gap/command-navigation-undo-workbench-implementation-spec.md)
- [Wave 4](wave-4-analysis-kernel-spec.md), [Wave 5](wave-5-elf-x86-spec.md), [Wave 6](wave-6-decompiler-spec.md)
