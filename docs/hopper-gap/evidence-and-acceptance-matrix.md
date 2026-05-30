# Evidence And Acceptance Matrix

Status: Validation and completion-audit planning. This document maps the broad
Hopper-gap goal to concrete evidence requirements.

Purpose: prevent a future "done" claim from resting on word count or good
intent. Every requirement needs evidence that matches its scope.

## Evidence Levels

Use these labels during audits:

- Proven: current-state evidence directly satisfies the requirement.
- Partially proven: some required evidence exists, but scope is incomplete.
- Planned: docs describe the requirement, but implementation evidence is absent.
- Not verified: evidence may exist, but has not been inspected.
- Contradicted: current-state evidence shows the requirement is not satisfied.
- Not applicable yet: explicitly out of scope for the current phase.

## Documentation-Pack Acceptance

| Requirement | Evidence Needed | Current Evidence | Status |
| --- | --- | --- | --- |
| Pack has an entrypoint | README links every major doc | `docs/hopper-gap/README.md` | Proven for current docs |
| Feature gaps are inventoried | missing features and current Damsel baseline | `feature-gap-inventory.md` | Proven as planning doc |
| Hopper public surface is researched | official and public-community sources | `hopper-public-research.md`, `hopper-public-web-feature-architecture-audit-2026.md`, `research-sources.md` | Proven as docs, not exhaustive forever |
| Product requirements exist | user personas, priorities, non-goals | `product-requirements.md` | Proven as docs |
| UX/app polish is specified | menus, keyboard, command palette, errors, accessibility | `competitive-ux-app-plan.md`, `native-app-polish-plan.md` | Proven as docs |
| Command/navigation/undo is specified | typed command registry, menu validation, palette, focus, route selection, navigation stack, transactions, activity log | `command-navigation-undo-workbench-implementation-spec.md` | Proven as docs |
| Apple runtime Workbench browsers are specified | ObjC, dispatch, C++/ObjC++, IOKit, cache, kernel, KDK/dSYM, panic, relationship graph, problem browser pages and APIs | `apple-runtime-workbench-browsers-implementation-spec.md` | Proven as docs |
| Performance responsiveness is specified | latency budgets, jobs, paging, metrics | `performance-responsiveness-architecture.md`, `large-project-performance-scheduler-indexing-implementation-spec.md` | Proven as docs |
| Native app trust is specified | helper auth, source grants, signing/update policy, diagnostics redaction | `native-app-trust-distribution-diagnostics-implementation-spec.md` | Proven as docs |
| Artifact intake is specified | source registration plans, source graphs, sidecar matching, artifact overview | `apple-artifact-intake-source-registration-implementation-spec.md` | Proven as docs |
| Architecture is specified | layers, source registry, analysis DB, Workbench integration | `target-architecture.md` | Proven as docs |
| Existing-code integration is mapped | current modules and migration slices | `current-code-integration-map.md` | Proven as docs |
| API/data contracts are specified | project, source, job, route, function, xref, annotation APIs | `api-data-contracts.md` | Proven as docs |
| Implementation roadmap exists | milestones, dependencies, gates | `integration-roadmap.md`, `implementation-epic-breakdown.md` | Proven as docs |
| Validation strategy exists | gates, traceability, evidence levels | `quality-validation-plan.md`, this file | Proven as docs |

## Hopper-Competitive Feature Acceptance

| Feature Family | Minimum Evidence For Implementation Completion | Current State |
| --- | --- | --- |
| Project database | project create/open/reopen tests, schema version tests, source hash mismatch tests | Planned |
| Source registry | standalone/cache/kernel/sidecar sources with stable ids | Planned |
| Address spaces | standalone, projected cache, cache vmaddr, runtime slide, fileset mapping tests | Planned |
| Source access and scheduler | path registration, byte windows, jobs, cancellation, event/progress tests | Planned |
| Paged indexes | source/symbol/string/procedure/problem/xref pages with stale cursor tests | Planned |
| Byte typing | byte classification snapshots and user override tests | Planned |
| Feature map | compact range API and Workbench route sync test | Planned |
| Function index | whole-function fixture snapshots and overlap/conflict tests | Planned |
| Xref index | from/to xref queries across calls/data/import/runtime refs | Planned |
| Graph view | CFG JSON, layout smoke, selection sync | Planned |
| Hex/data view | byte-range fetches and assembly/hex selection sync | Planned |
| Representation sync | graph/assembly/hex/feature-map route sync, list fallback, export snapshots | Planned |
| Command registry | descriptor snapshots, menu validation tests, palette discovery, disabled reasons | Planned |
| Navigation stack | route push/back/forward/session restore tests across route kinds | Planned |
| Command palette | command/object search, disabled reason display, keyboard-only execution tests | Planned |
| Annotations | label/comment/tag/bookmark/manual-ref transaction tests | Planned |
| Undo/redo | model-level transaction replay tests | Planned |
| Problems/hints | problem records, suppress/apply hint, reanalysis trigger tests | Planned |
| ObjC browser | class/selector/method implementer/caller fixtures | Planned |
| ObjC dispatch browser | message-send evidence, confidence buckets, manual binding undo tests | Planned |
| C++ browser | demangle/vtable/RTTI/virtual-call fixtures | Planned |
| IOKit browser | user-client/external-method candidate fixtures with weak-evidence labeling | Planned |
| Runtime relationship graph | ObjC/C++/cache/kernel/panic neighborhood graph snapshots and list fallback | Planned |
| Kernel browser | fileset/kext/KDK/panic fixtures | Planned |
| Panic browser | parsed panic log, slide mapping, frame route, unresolved-frame explanations | Planned |
| Decompiler-lite | function summary snapshots with unresolved facts | Planned |
| IR | IR snapshots and source instruction mapping tests | Planned |
| Decompiler Workbench | pseudocode route, line/token/source-map snapshots, edit transactions, cancellation, stale-cache, export, and overlay tests | Planned |
| Scripting/MCP | read-only query tools, mutation permission/audit tests | Planned |
| Plugin/SDK boundary | manifest, capability, provenance, cancellation, and isolation tests | Planned |
| Diff/export/evidence | snapshots, fingerprints, findings, report export, and redaction tests | Planned |
| Native app trust | helper token auth, source grants, diagnostics redaction, crash/hang recovery, signing gates | Planned |
| Live debugger Workbench | debug target/session/module/breakpoint/stop-event fixtures, helper isolation tests, route-backed PC overlays, permission-denial problems, and redacted exports | Planned |
| Patch Studio | patchability, byte edit, assembler, NOP, copy-on-write revision, reanalysis, export, code-signing warning, and evidence-package tests | Planned |
| Artifact intake | intake sessions, candidate plans, source graph, sidecar matching, projection recipes, artifact overview | Planned |
| DWARF/dSYM/KDK debug info | debug-source registry, exact/mismatch candidate explanations, line/function/variable/type fixtures, privacy tests | Planned |
| Debugger bridge | LLDB local smoke and static/runtime route mapping | Planned |
| Patch Studio | native patchability, preview, impact, export, and immutable-original UX proof | Planned |
| Patching | design, source-hash precondition, write-copy-only tests | Not applicable yet |

## Existing Current-State Evidence

Current code evidence that can be used as foundation:

- `damsel-core/src/lib.rs` exports typed binary, cache, ObjC, dyld, and
  disassembly-analysis models.
- `damsel-macho/src/loader.rs` owns Mach-O loading.
- `damsel-macho/src/disasm.rs` owns current disassembly analysis.
- `damsel-macho/src/objc.rs` owns ObjC metadata extraction.
- `damsel-macho/src/shared_cache.rs` and
  `damsel-macho/src/shared_cache_query.rs` own shared-cache loading/querying.
- `damsel-cli/src/main.rs` exposes standalone and cache commands.
- `damsel-cli/src/ui.rs` exposes upload/symbol/disassembly API routes.
- `damsel-app/Sources/DamselAppCore` owns Workbench server, API, store, models,
  and views.
- `damsel-macho/tests/shared_cache_real_env.rs` contains opt-in real-cache
  testing.

Current contradictions to Hopper-complete implementation:

- no persistent project database
- no saved project open/reopen flow
- no path-based project source registration
- no job scheduler or progress/event stream
- no paged index runtime
- no durable byte classification table
- no Workbench feature map
- no Workbench graph view
- no annotation persistence/undo
- no project-wide xref API
- no ObjC/C++/kernel browsers in Workbench
- no decompiler IR
- no scripting/MCP implementation
- no plugin/SDK extension boundary implementation
- no diff/export/evidence package implementation
- no debugger bridge
- no helper authentication, source grant, diagnostics redaction, update state,
  plugin trust, or signing/notarization validation implementation
- no artifact intake session, source registration plan, source graph, sidecar
  matching, panic attachment, or artifact overview implementation

These contradictions are expected. The goal of the pack is planning, not
claiming implementation.

## Verification Commands For Documentation

Run after documentation edits:

```sh
find docs/hopper-gap -maxdepth 1 -type f -print
wc -l docs/hopper-gap/*.md
wc -w docs/hopper-gap/*.md
rg -n "work marker pattern" docs/hopper-gap README.md docs/roadmaps/apple-runtime-analysis-roadmap.md
rg -n "[^[:ascii:]]|[ \t]+$" docs/hopper-gap README.md docs/roadmaps/apple-runtime-analysis-roadmap.md
git diff --check -- README.md docs/roadmaps/apple-runtime-analysis-roadmap.md
```

Interpretation:

- clean commands prove formatting hygiene, not content completeness
- word/line count proves scope, not quality
- source links prove research traceability, not implementation

## Verification Commands For Future Implementation

Baseline gates:

```sh
cargo test --workspace
cargo test -p damsel-cli --test ui_api
cargo test -p damsel-cli --test cli_snapshots
swift test --package-path damsel-app
./fixtures/build-fixtures.sh --check
./fixtures/tests/build-fixtures-parity.sh
cargo bench -p damsel-macho --bench decode_bench --no-run
```

Optional gates:

```sh
DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT=/path/to/cache-root cargo test -p damsel-macho --test shared_cache_real_env
./script/build_and_run.sh --verify-fixture
```

New gates must be added for:

- project model snapshots
- project reopen
- schema migration
- byte map
- annotation transaction
- function/xref index
- graph API
- feature map
- graph layout cache and export snapshots
- graph/assembly/hex/feature-map selection sync fixtures
- graph accessibility list fallback tests
- Workbench project open
- paging/cancellation
- diagnostics redaction
- local helper token/auth enforcement
- source access grants and bookmark repair
- helper crash/hang recovery
- signing/notarization validation if release claims are made
- artifact intake and source graph fixtures
- KDK/dSYM/panic sidecar matching fixtures
- type/signature/debug-info store snapshots
- dSYM/KDK debug-source matching and explanation fixtures
- source-line, inline-frame, variable-location, and source-root privacy fixtures
- Workbench type navigator and signature editor store tests
- symbolication profile mismatch and profile-switch tests
- Workbench artifact overview smoke

## Requirement Traceability Template

Use this table in issues and milestone audits:

| Requirement | Source Doc | Implementation Files | API/CLI Surface | UI Surface | Fixture Evidence | Real Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Example: byte classification | feature specs, DB blueprint | `byte_map.rs` | `project byte-map` | feature map | byte-map fixture | large Mach-O smoke | Planned |

Rules:

- every row needs a source doc
- every implemented row needs test evidence
- real-system evidence is optional only when explicitly marked opt-in
- broad requirements cannot be proven by narrow smoke tests
- incomplete evidence keeps the row below Proven

## Release Readiness Checklist

Before claiming a Hopper-gap implementation wave is complete:

- all intended docs are linked from the pack README
- feature requirements are traced to issues or branches
- current code status is inspected, not assumed
- dirty unrelated files are identified
- tests cover touched behavior
- performance claims have measurements
- real-cache/kernel claims are opt-in or actually run
- Workbench claims are backed by app tests or smoke proof
- limitations and non-goals are stated
- no unsupported Hopper parity claims remain
