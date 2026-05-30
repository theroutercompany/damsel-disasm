# Performance And Responsiveness Architecture

Status: Architecture and quality-bar proposal.

Purpose: define the system performance, app responsiveness, and scalability
architecture needed for Damsel to feel competitive against Hopper while serving
Damsel's Apple systems niche.

Surface implementation companion:
[Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
sets concrete latency, paging, main-actor, lazy-inspector, graph-layout, search,
and feature-map rules for the Workbench surfaces.

Runtime implementation companion:
[Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
turns these performance goals into concrete source-registration, mmap, job
scheduler, priority, indexing, paging, event stream, cache, metric, diagnostic,
Workbench store, and benchmark-gate requirements.

Workbench implementation companion:
[Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
turns these performance requirements into app-specific ownership rules, main
thread constraints, paging strategy, job events, representation stores, and
latency budgets.

Companion measurement docs:

- [Corpus, Benchmark, And Regression Plan](corpus-benchmark-regression-plan.md)
  defines fixture tiers, real-artifact evidence, benchmark classes, regression
  gates, and benchmark output expectations.
- [UX Latency And Polish Measurement Plan](ux-latency-polish-measurement-plan.md)
  defines Workbench-specific latency probes, keyboard completeness, cancellation
  behavior, error recovery, and interaction stability checks.

## Performance Thesis

Damsel's target workloads are harder than small app binaries:

- multi-hundred-megabyte macOS frameworks
- multi-gigabyte dyld shared caches
- split cache member sets
- iOS caches extracted from IPSWs
- kernel collections
- KDK/dSYM debug info
- Objective-C-heavy frameworks with huge selector/class sets
- C++/IOKit-heavy kernel binaries
- cross-version Apple OS comparison

The app must therefore be architected around:

- lazy loading
- memory mapping
- project indexes
- incremental analysis
- cancellable background jobs
- paged APIs
- virtualized UI lists
- streaming progress
- cached graph layouts
- measured performance gates

The current UI already takes one good step by using an `NSTableView` for
instruction rows. But the backend still exposes mostly point-query payloads and
in-memory uploaded images. Competitive responsiveness requires a deeper
pipeline.

## Performance Goals

These are target budgets, not current claims.

### App Interaction Budgets

- app launch to empty window: under 1 second on a typical Apple Silicon Mac
- server bootstrap visible status: under 500 ms when binary is already staged
- toolbar/menu interaction: under 50 ms
- row selection to inspector update: under 50 ms for cached facts
- back/forward navigation: under 75 ms for cached targets
- local indexed search first results: under 100 ms
- command palette open/filter: under 50 ms
- instruction list scroll: no visible hitching at normal trackpad speed
- graph pan/zoom: no visible hitching for medium functions
- progress update cadence: at least every 250 ms during long analysis

### Analysis Budgets

Budgets should be tiered by input size:

- tiny fixture: milliseconds to low hundreds of milliseconds
- normal app binary: useful metadata in under 1 second, first function shortly
  after
- large framework: metadata quickly, full function/xref analysis in background
- dyld cache: image inventory quickly, selected image analysis lazily
- kernel collection: member inventory quickly, deep C++/kernel analysis in
  queued passes

The app should never require full-cache analysis before the user can inspect a
selected image or search already indexed names.

### Memory Budgets

Targets:

- do not duplicate full binary bytes in Swift and Rust when avoidable
- avoid copying entire large files into multipart memory bodies for project
  workflows
- memory-map file-backed sources in Rust
- keep UI row/view models paged
- do not materialize all graph nodes for cache-wide graphs in one response
- allow project close to release source mappings

The current Workbench upload path reads the file into `Data` and posts it to the
local server. That is acceptable for small fixture-oriented UI, but not for
dyld caches, kernel collections, or KDK-backed projects. Competitive app
workflows need path-based local project registration, with careful permission
and sandbox design.

## Current Bottleneck Classes

### Upload-Based Loading

Current shape:

- Workbench reads file into memory.
- Workbench builds multipart body.
- Server reads uploaded field bytes.
- Server loads bytes into `BinaryImage`.
- Server stores image in an in-memory map.

Problems for large artifacts:

- double memory pressure
- slow copies
- upload body limit
- no durable source identity
- no reopening without re-upload
- poor fit for split dyld caches and KDK bundles

Target:

- project/source registration by path
- security-scoped access if sandboxed later
- content hash and UUID recorded
- Rust loader memory-maps file directly
- project stores source identity, not entire bytes by default

### Single Image Store

Current server state stores images in a `Mutex<HashMap<String, StoredImage>>`.

Problems:

- no project lifecycle
- no eviction policy
- no persisted analysis
- no multi-source relationships
- no background analysis state

Target:

- project registry
- source registry
- analysis database
- job scheduler
- cache eviction for derived views
- explicit project close

### Whole Payload Responses

Current disassembly endpoint returns a payload for the requested window.

Problems:

- large instruction windows inflate JSON
- UI updates depend on full response
- selection/xrefs require recomputation or payload-local facts
- impossible to page long functions naturally

Target:

- project-backed paged instruction endpoint
- function summary endpoint
- xref endpoint
- graph endpoint
- lazy inspector facts endpoint
- stable ids for selected rows

### Main-Actor Filtering

Current Workbench filters visible instructions in computed state. For small
windows this is fine. For large project-backed lists, filtering must move to an
index-backed query or background task.

Target:

- backend search indexes for project-wide search
- local UI filtering only for already small visible pages
- debounced search with cancellation
- stable result pages

### Synchronous Heavy Graph Layout

Graph rendering can become expensive quickly.

Target:

- graph facts and layout separated
- graph layout job can run in background
- graph endpoint returns partial layout state
- large graph list-mode fallback
- cached graph layout keyed by function and graph version

## Target Performance Architecture

```text
Workbench
  view state
  paged list models
  command palette cache
  selection/navigation state
        |
        v
Local API
  project endpoints
  paged queries
  background job control
  progress events
  route/object ids
        |
        v
Analysis Runtime
  project registry
  job scheduler
  in-memory hot caches
  persistent analysis database
  source registry
        |
        v
Source Access
  memory-mapped files
  split cache members
  KDK/dSYM paths
  extracted artifacts
```

## Analysis Runtime

### Job Scheduler

Add a scheduler with:

- job id
- pass id
- project id
- priority
- dependencies
- state
- progress
- cancellation token
- output fact kinds
- error/warning list
- start/end timestamps

Job states:

- queued
- running
- paused
- cancelled
- failed
- completed
- superseded

Priority classes:

- foreground navigation
- selected function analysis
- selected image analysis
- user-requested export/report
- background project indexing
- low-priority broad cache analysis

### Cancellation

Every long pass must periodically check cancellation:

- recursive descent across many functions
- shared-cache relation building
- C++ vtable sweep
- kernel recognizer sweep
- DWARF import
- graph layout
- project comparison

Cancelled passes should leave completed facts with analysis-run provenance, but
the project must record that the pass did not complete.

### Incremental Analysis

Do not analyze everything before anything is useful.

Recommended order:

1. source identity
2. coarse metadata
3. sections and load commands
4. symbols/imports/exports
5. strings
6. ObjC metadata
7. function seeds
8. selected function decode
9. selected image xrefs
10. broad function index
11. runtime deep passes
12. graph layouts
13. debug-info enrichment

Foreground user navigation can promote a function/image/member to high
priority.

### Cache Invalidation

Facts should be invalidated when:

- source hash changes
- analyzer version changes
- project schema changes
- analysis settings change
- debug sidecar changes
- user edits conflict with derived facts

Invalidation should be granular where possible. Updating graph layout should
not invalidate ObjC class extraction.

## Persistence Performance

### Stage 1: Deterministic JSON

Useful for:

- model stabilization
- test snapshots
- small projects

Not enough for:

- full dyld cache projects
- kernel collection plus KDK type info
- large xref indexes

### Stage 2: SQLite Or Embedded Index Store

Recommended indexes:

- source id
- address space id
- address range
- fact kind
- function id
- function start
- reference source
- reference target
- symbol/name text
- string text trigram/fuzzy index
- runtime object kind/name
- graph id
- annotation target

Storage rules:

- store large raw blobs only when necessary
- prefer source paths plus hashes over copying input bytes
- keep derived facts separate from user annotations
- allow vacuum/compact

### Stage 3: Sharded Fact Store

For large Apple OS corpora:

- shard by project/source/image/member
- lazily load shard metadata
- keep hot function/xref shards in memory
- evict cold graph layouts
- content-addressed derived caches

## API Responsiveness

### Paged Endpoints

Every potentially large collection needs pagination:

- functions
- symbols
- strings
- references
- instructions
- ObjC selectors
- ObjC methods
- C++ types
- C++ vtables
- kernel symbols
- cache images
- problems

Pagination requirements:

- stable cursor or offset with version token
- total count when cheap
- "count unknown" state when expensive
- sort key in response
- filter echo in response

### Streaming Or Polling Progress

Minimum:

- polling endpoint for analysis status

Better:

- server-sent events or WebSocket for progress

Progress payload:

- project id
- job id
- pass id
- state
- current item
- completed/total
- elapsed
- warnings
- cancellable

### Route/Object IDs

The UI should not use bare integers or names as navigation truth.

Use stable ids for:

- source
- address space
- function
- instruction
- basic block
- reference
- runtime object
- graph node
- kernel object
- annotation

This keeps navigation responsive because the UI can request exactly the object
needed instead of searching by name/address every time.

## Workbench Responsiveness

### View Model Rules

- Keep `@MainActor` state small.
- Do not store entire huge project lists in Swift.
- Store pages and selection ids.
- Cancel stale network requests.
- Debounce search.
- Use background decoding for large JSON if needed.
- Keep row models compact.

### List Virtualization

Use native virtualized components for:

- instructions
- functions
- symbols
- strings
- xrefs
- selectors
- methods
- C++ types
- kernel symbols
- problems

Avoid `LazyVStack` for enormous lists when `NSTableView`/`NSOutlineView` is more
appropriate.

### Inspector Loading

Inspector should request detailed facts lazily:

- show cached summary immediately
- fetch raw facts/provenance only when expanded
- fetch xrefs page-by-page
- cancel previous inspector request when selection changes

### Graph Rendering

Graph renderer requirements:

- separate model from layout
- progressive layout
- visible node culling
- minimap uses simplified geometry
- text measurement cache
- selection sync does not recompute layout
- large graph warning and list fallback

### Search UX

Search should provide:

- immediate local suggestions from cached recent items
- backend indexed results as they arrive
- cancellation of stale queries
- grouped results by kind
- keyboard navigation
- result count/progress

## Performance Instrumentation

### Metrics To Capture

Backend:

- source load time
- parse time
- section classification time
- symbol/import/export time
- ObjC parse time
- function seed time
- function analysis time
- xref build time
- graph build/layout time
- C++ analysis time
- kernel analysis time
- DWARF/KDK import time
- peak resident memory where available
- fact counts
- warning/error counts

Frontend:

- app launch time
- server bootstrap time
- upload/source registration time
- first metadata visible
- first instruction visible
- row selection to inspector visible
- search first result time
- graph first paint
- scroll hitch count if measurable

### Where Metrics Live

- CLI `--json` validation output
- project analysis status
- Workbench diagnostics panel
- test artifacts
- benchmark outputs
- optional local performance log

Avoid collecting private binary contents.

## Benchmarks

### Fixture Benchmarks

- tiny function decode
- branch-heavy function analysis
- import-rich analysis
- ObjC metadata parse
- C++ vtable parse
- synthetic cache image projection
- synthetic kernel fileset parse

### Real-System Benchmarks

Opt-in only:

- current macOS dyld shared cache inventory
- selected framework projected image analysis
- libobjc/libdispatch/Foundation/CoreFoundation selected-image analysis
- KDK-matched kernel collection inventory
- kernel selected member function analysis

### UI Benchmarks

- load fixture and render first instruction
- scroll 10,000 instruction rows
- filter/search symbol list
- open graph for branch-heavy fixture
- select 100 instructions rapidly and ensure stale inspector updates are
  cancelled

## Responsiveness Failure Modes

### UI Freeze During Analysis

Cause:

- synchronous analysis on main actor
- huge JSON decoding on main actor
- graph layout in view update

Mitigation:

- background jobs
- paged endpoints
- cancellation
- incremental rendering

### Memory Spike Opening Large Cache

Cause:

- full file copied into Swift `Data`
- multipart body copy
- server stores full bytes plus parsed image

Mitigation:

- path-based project registration
- memory mapping
- project database
- source registry

### Search Feels Slow

Cause:

- filtering huge arrays in Swift
- no backend index
- stale queries not cancelled

Mitigation:

- indexed search service
- result pagination
- debounced/cancellable requests

### Graph Freezes

Cause:

- layout computed synchronously
- all nodes rendered at once
- text measurement repeated

Mitigation:

- layout cache
- progressive layout
- visible culling
- list fallback

### Analysis Progress Is Untrustworthy

Cause:

- monolithic analysis function
- no pass-level progress
- no partial completion state

Mitigation:

- scheduler
- pass status model
- analysis run records
- warnings/problems panel

## Performance Acceptance Gates

### Gate 1: Small Binary Smoothness

Evidence:

- project open fixture timing
- first metadata visible timing
- first disassembly visible timing
- no main-thread stalls in local app smoke

### Gate 2: Long Function Smoothness

Evidence:

- virtualized instruction list handles large fixture smoothly
- row selection and inspector update stay under budget
- filter/search does not block for representative windows

### Gate 3: Shared Cache Triage

Evidence:

- cache inventory visible without full-cache disassembly
- selected image analysis can run independently
- cache image list is paged/filterable
- memory usage does not scale with entire cache copied into Swift

### Gate 4: Kernel Project Triage

Evidence:

- fileset/member list visible before deep analysis
- KDK status visible
- selected member/function analysis can run independently

### Gate 5: Background Analysis

Evidence:

- user can navigate while analysis runs
- cancellation works
- project records partial completion truthfully

## Required Refactors

### Replace Upload-Only App Loading

Add:

- source registration endpoint
- path-backed project creation
- sandbox/security design
- memory-mapped Rust loading

Keep:

- upload endpoint for web UI and small fixtures

### Replace Image-Only Server Store

Add:

- project registry
- job scheduler
- analysis database
- source registry
- cache eviction

Keep:

- existing image store as compatibility path until project APIs are ready

### Replace Whole-Result UI Contract

Add:

- paged instruction endpoints
- paged symbol/function/string/xref endpoints
- object detail endpoints
- graph endpoints

Keep:

- direct disassembly endpoint for simple workflows and regression tests

### Add Performance Test Harness

Add:

- CLI benchmark commands or scripts
- Workbench smoke scripts
- output artifacts with timings
- opt-in real-system benchmark gates

## Implementation Order

1. Define performance budgets and metric names.
2. Add source registration/project creation.
3. Add project analysis job model.
4. Add paged list endpoint for functions/symbols.
5. Migrate Workbench to project-backed source registration for local files.
6. Add progress polling.
7. Add instruction paging.
8. Add xref paging.
9. Add graph layout cache.
10. Add UI performance smoke tests.

This order makes the app feel better before every deep analyzer is complete.
