# Architecture Decision And Risk Register

Status: Decision and risk planning document. This register records major
choices that should be made deliberately while pursuing Hopper-level
competitiveness.

Purpose: make the hidden hard calls visible. The Hopper-gap plan contains
features that can easily produce large rewrites, performance problems, or
misleading product claims if architectural decisions stay implicit.

## Decision Status Legend

- Proposed: recommended by this documentation pack, not yet accepted.
- Deferred: intentionally not decided yet.
- Accepted: should be treated as active direction after review.
- Rejected: considered and not chosen.

## Major Architecture Decisions

### D1: Rust Remains The Analysis Source Of Truth

Status: Proposed

Decision:

- binary parsing, analysis passes, project database, and fact generation remain
  in Rust
- Swift Workbench consumes stable APIs and owns native UI state

Rationale:

- current parser/analysis code already lives in Rust
- CLI and UI server can share model contracts
- Swift should not duplicate decoder logic
- Rust is the better place for memory mapping, large-file IO, and deterministic
  fixture snapshots

Risks:

- API design must be good enough for native UX
- Workbench iteration may feel slower if every UI concept waits for Rust

Mitigation:

- define typed project APIs
- keep Swift models as mirrors of API contracts
- use small compatibility shims during migration

### D2: Add Project APIs Beside Existing Image APIs

Status: Proposed

Decision:

- keep `/api/images` upload/disasm routes during migration
- add `/api/projects` routes beside them
- migrate Workbench when project APIs are stable

Rationale:

- preserves current browser UI and Workbench behavior
- lowers risk of large app rewrite
- allows CLI/UI API tests to prove model first

Risks:

- duplicate API surface
- compatibility paths linger too long

Mitigation:

- mark image APIs as compatibility once Workbench migrates
- share internals between image and project paths where possible

### D3: In-Memory Model Before SQLite

Status: Proposed

Decision:

- implement pure in-memory project/fact/byte-map models first
- use deterministic JSON snapshots for early validation
- defer SQLite or embedded persistence until schema churn slows

Rationale:

- model semantics are not stable yet
- snapshots are easier to review
- early SQL schema churn would waste time

Risks:

- large artifacts need persistence and paging
- temporary in-memory shape may influence long-term model poorly

Mitigation:

- design object ids and facts as persistence-ready
- keep large-artifact constraints visible
- add append-only log before final store

### D4: Object Ids And Routes Are Mandatory

Status: Proposed

Decision:

- every navigable object gets an opaque object id
- UI, CLI, scripting, MCP, and debugger surfaces use routes rather than raw
  addresses alone

Rationale:

- Hopper-like navigation spans addresses, functions, blocks, selectors, cache
  images, kexts, types, problems, and annotations
- raw addresses are ambiguous across caches, projected images, filesets, and
  runtime slides

Risks:

- route model feels heavy for simple commands

Mitigation:

- preserve simple point-query commands
- let simple commands internally produce routes without exposing complexity in
  text output

### D5: Byte Classification Is Core Infrastructure

Status: Proposed

Decision:

- build a byte classification table early
- feature map, hex sync, make-code/data/string, and patching all depend on it

Rationale:

- Hopper public docs make byte typing central
- section names alone cannot support user retyping or feature-map navigation

Risks:

- classification overlap and invalidation are subtle

Mitigation:

- start with inferred classifications only
- add user overrides and transaction invalidation later
- test overlap precedence explicitly

### D6: Project Facts Are Provenance-Aware

Status: Proposed

Decision:

- every inferred/imported fact includes source, analyzer, confidence, and target
- contradictory facts can coexist

Rationale:

- Apple system binaries combine symbols, debug info, ObjC metadata, dyld
  fixups, KDK data, and heuristics
- wrong certainty is worse than visible uncertainty

Risks:

- fact model becomes verbose
- UI may need simplification layers

Mitigation:

- store rich facts
- render compact summaries
- expose detail in inspector/problems

### D7: Read-Only Before Mutation

Status: Proposed

Decision:

- build read-only analysis, navigation, scripting, and MCP before binary
  mutation or process memory writes

Rationale:

- patching and debugger mutation require undo, audit, permissions, source hash
  preconditions, and user trust

Risks:

- feature parity with Hopper patching/debugging takes longer

Mitigation:

- document patching/debugger as later
- ship excellent static workflows first

### D8: Real Apple Artifacts Are Opt-In Evidence

Status: Proposed

Decision:

- ordinary CI uses checked-in fixtures and synthetic corpora
- real dyld caches, KDKs, and kernel collections are opt-in local/CI evidence

Rationale:

- real artifacts are OS-dependent, large, and often private
- CI should remain portable and reliable

Risks:

- synthetic fixtures miss real-world issues

Mitigation:

- maintain real-evidence harnesses
- record exact artifact/build context
- require real evidence for claims that mention real artifacts

### D9: Decompiler Starts With IR And Summaries

Status: Proposed

Decision:

- do not start with C-like pseudocode
- build function summaries, low-level IR, SSA/data-flow, and source mapping
  first

Rationale:

- pseudocode without provenance misleads users
- Damsel can provide value through truthful summaries before full decompiler

Risks:

- users may expect Hopper-style pseudocode sooner

Mitigation:

- communicate decompiler-lite scope clearly
- make summaries excellent and Apple-aware

### D10: Plugin/SDK Boundary Is Deferred

Status: Proposed

Decision:

- do not expose raw Rust internals as plugin API early
- wait until project objects and facts stabilize

Rationale:

- early SDKs freeze bad abstractions
- Hopper-like plugins need stable document objects, not parser internals

Risks:

- extension ecosystem delayed

Mitigation:

- expose CLI/API automation first
- design capability model now

## Risk Register

### R1: Scope Explosion

Severity: High

Risk:

- trying to implement project DB, Workbench overhaul, ObjC browser, C++,
  kernel, decompiler, scripting, and debugger in one branch

Mitigation:

- follow Wave 1 project foundation spec
- keep first branch model/CLI focused
- require epic-level completion gates

### R2: Current Workbench Becomes A Bottleneck

Severity: High

Risk:

- `WorkbenchStore` grows into a monolith as project/jobs/search/routes are added

Mitigation:

- split stores after project API appears
- keep route/navigation/job/search state separate
- add store transition tests

### R3: Large Files Are Copied Too Often

Severity: High

Risk:

- local project open continues using multipart upload and Swift `Data`,
  doubling memory for dyld caches/kernel collections

Mitigation:

- add local path registration for project APIs
- memory-map sources in Rust
- retain upload only for browser/small-fixture compatibility

### R4: Cache Context Is Lost

Severity: High

Risk:

- projected cache images become standalone images and lose provider/importer,
  reexport, and cache-address context

Mitigation:

- model cache source, projected image source, and address-space conversions
- preserve shared-cache session provenance in project facts

### R5: Overconfident Analysis

Severity: High

Risk:

- Damsel renders guessed ObjC/C++/switch/decompiler facts as exact

Mitigation:

- provenance/confidence required on facts
- problem list for ambiguous cases
- UI labels uncertainty
- negative tests for unresolved cases

### R6: Performance Claims Lack Measurements

Severity: Medium

Risk:

- docs or release notes claim responsiveness without benchmark evidence

Mitigation:

- require metric names and benchmark classes
- use corpus/benchmark plan
- record Workbench probes for app claims

### R7: Real-Artifact Claims Are Overstated

Severity: High

Risk:

- one local real-cache smoke is presented as broad real-world support

Mitigation:

- evidence matrix distinguishes fixture, synthetic, opt-in real, and private
  corpus evidence
- final reports state exact artifacts and dates

### R8: SQLite Schema Freezes Too Early

Severity: Medium

Risk:

- premature store schema makes model evolution expensive

Mitigation:

- in-memory model first
- deterministic snapshots
- append-only log before final indexed store

### R9: Plugin API Freezes Internals

Severity: Medium

Risk:

- early plugin API exposes unstable Rust structs and blocks refactors

Mitigation:

- defer plugin API
- expose stable CLI/API automation first
- version plugin capabilities explicitly

### R10: Debugger Distracts From Static Core

Severity: Medium

Risk:

- debugger UI arrives before static/runtime address mapping and project model
  are trustworthy

Mitigation:

- require route/object/source/slide mapping first
- local LLDB bridge only after project shell and symbolication profile exist

### R11: Patching Creates Data-Loss Risk

Severity: High

Risk:

- binary mutation appears before transaction, preview, source hash precondition,
  and write-copy-only design

Mitigation:

- patching is design-only until persistence/undo are mature
- never patch shared caches or kernel collections in place

### R12: Documentation Becomes Stale

Severity: Medium

Risk:

- the Hopper-gap pack keeps saying features are planned after code lands, or
  claims implemented features without evidence

Mitigation:

- update evidence matrix as milestones land
- run completion audits
- link docs updates to implementation PRs

## Decision Review Cadence

Before each implementation wave:

- review relevant decisions
- mark accepted/deferred/rejected
- update risk severity
- add newly discovered risks
- check evidence requirements

After each wave:

- update current-code integration map
- update evidence matrix
- update roadmap status
- preserve limitations and non-goals

## Open Decisions

These should be resolved before or during Wave 1:

- exact object id string scheme
- whether project snapshots include content hash by default
- path redaction defaults
- serde payload strategy for early fact kinds
- CLI project snapshot output mode
- whether to create a `damsel-project` module or keep initial models in
  `damsel-core`

These should stay deferred:

- SQLite table schema
- plugin runtime
- scripting language
- MCP server process model
- LLDB transport crate
- binary patch writer

