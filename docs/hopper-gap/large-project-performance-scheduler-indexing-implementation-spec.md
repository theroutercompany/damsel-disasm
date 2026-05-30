# Large-Project Performance, Scheduler, And Indexing Implementation Spec

Status: implementation architecture plan. This is not an implementation claim
and it does not assert that current Damsel already meets these budgets.

Purpose: define the performance architecture Damsel needs to feel competitive
with Hopper on large Apple-system workloads: dyld shared caches, projected
cache images, driver-cache dylibs, kernel collections, KDK/dSYM-enriched
projects, Objective-C-heavy frameworks, C++/IOKit binaries, and long native
Workbench sessions.

This spec focuses on the part that cannot be solved by polishing one view:
source registration, memory mapping, background job scheduling, analysis pass
prioritization, index layout, paging, cancellation, progress, caching,
diagnostics, and Workbench responsiveness.

## Research Companions

- [Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
- [Research Sources](research-sources.md)

## Architecture Companions

- [Performance And Responsiveness Architecture](performance-responsiveness-architecture.md)
- [UX Latency And Polish Measurement Plan](ux-latency-polish-measurement-plan.md)
- [Corpus, Benchmark, And Regression Plan](corpus-benchmark-regression-plan.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [API And Data Contracts](api-data-contracts.md)
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)

## Public Hopper Evidence

Official changelog evidence:

- Hopper 6.2.7 improves loading time for large binaries and decompilation of
  large functions.
- Hopper 6.2.6 optimizes analysis time for very large binaries.
- Hopper 6.2.4 fixes a memory leak in the decompiler.
- Hopper 6.2.0 improves large database loading time, memory footprint, MCP
  reliability, and UI details.
- Hopper 6.1.0 adds quick linear segment disassembly and driver-cache dylib
  reading.
- Hopper 6.0.12 improves method navigation for files extracted from the dyld
  shared cache.
- Hopper 6.0.11 fixes an infinite loop during analysis.
- Hopper 6.0.10 fixes a switch/case analysis hang and procedure-list selection
  annoyances.
- Hopper 6.0.7, 6.0.6, and 6.0.5 fix decompiler hangs, lower decompiler
  memory footprint, improve memory efficiency, and fix UI glitches.
- Hopper 5.19.3 fixes a kext-loading crash.
- Hopper 5.17.4 improves CFG scrolling/zooming and Objective-C header export
  for dyld-cache extracted files.
- Hopper 5.15.4 fixes hangs on mangled files extracted from the dyld shared
  cache.
- Hopper 5.11.7 drastically reduces memory footprint on very large binaries
  during analysis.
- Hopper 5.11.5 improves newer dyld cache handling and works around large
  document display problems.
- Hopper 5.11.4 improves address-list responsiveness and fixes a deadlock when
  resizing the address list while the procedure list and background analysis
  are active.
- Hopper 5.11.0 fixes a potential deadlock when exporting assembly or
  pseudocode.
- Hopper 5.8.6 improves dyld shared-cache symbol export and extraction time.
- Hopper 5.8.0 fixes crashes when exporting pseudocode containing very large
  methods.
- Hopper 5.7.5 improves extraction of files from dyld shared caches.

Official SDK evidence:

- `HPDocument` exposes background-process state, wait/cancel UI, and
  background-process stop requests.
- `FileLoader` accepts progress callbacks during long loads.
- `HPDisassembledFile` exposes large document primitives that require indexes:
  names, named addresses, byte types, procedures, xrefs, tags, colors,
  problems, structured types, and transactions.
- `CPUContext` has analysis lifecycle hooks, branch/switch analysis hooks,
  padding/prolog detection, and instruction-specific analysis hooks, which
  implies pass-level cost and cancellation boundaries.

Official product/tutorial evidence:

- Hopper starts automatic analysis after loading.
- Hopper synchronizes assembly, graph, pseudocode, and hex representations.
- Hopper has large navigation lists such as symbols, strings, procedures, and
  tags.
- Hopper can inspect files extracted from dyld caches, driver caches, and
  kernel caches.

Architecture inference:

- Hopper-class performance is not one optimization. It is a document runtime
  design with background analysis, cancellable work, index-backed navigation,
  paged views, memory-efficient source access, durable project state, and UI
  projections that can stay responsive while analysis continues.

## Current Damsel Baseline

Current useful pieces:

- `damsel-core` and `damsel-macho` already use typed models and serde-friendly
  records.
- `damsel-macho` already supports standalone Mach-O, dyld metadata, ObjC
  metadata, disassembly analysis, and shared-cache projection.
- `damsel-cli` already has deterministic command output and a local UI server.
- `damsel-app` already has a native Workbench that launches the local server.
- The Workbench reads file bytes off the main actor through a detached task.
- The instruction surface uses an AppKit `NSTableView`, which is the right
  primitive for large table virtualization on macOS.
- `WorkbenchStore` uses request ids to ignore stale load/disassembly results.

Current performance bottlenecks:

- `damsel-app` uploads file contents through multipart request bodies.
- `damsel-cli ui` caps uploads at `128 * 1024 * 1024` bytes.
- The UI server stores images in a process-local `Mutex<HashMap<String,
  StoredImage>>`.
- The local UI server uses a current-thread Tokio runtime.
- Uploaded bytes are copied into Swift `Data`, multipart `Data`, Axum body
  bytes, then `Vec<u8>` for `load_bytes`.
- There is no project/source registration API by path.
- There is no durable project database.
- There is no job scheduler or persisted analysis job state.
- Long disassembly and analysis happen inside request handling.
- Search suggestions scan loaded image symbols in memory.
- Instruction filtering happens in `WorkbenchStore.visibleInstructions` over
  the current payload.
- There is no shared index for symbols, functions, strings, xrefs, runtime
  objects, problems, or routes.
- There is no progress event stream from backend analysis passes.
- There is no cancellation token connected to parser/analyzer passes.
- There are no performance counters or main-thread stall probes in the
  Workbench.

Interpretation:

- Current architecture is appropriate for alpha point-query disassembly.
- It is not an acceptable foundation for dyld caches, kernel collections, KDK
  imports, large shared-cache extracted images, large C++ frameworks, or
  Hopper-grade Workbench claims.
- The performance refactor must be first-class and cross-cutting, not a final
  tuning pass.

## Core Thesis

Damsel should treat responsiveness as a project-runtime invariant:

```text
open source quickly
show first useful metadata quickly
run deep analysis in cancellable background jobs
write facts incrementally
build indexes incrementally
serve every large UI surface through paging
preserve route and selection state across updates
measure every budget
```

The app should never need to load, analyze, index, render, or transmit the
entire artifact before the user can start working on a selected source,
function, selector, kext, or panic frame.

## Non-Goals For The First Performance Wave

Do not implement first:

- distributed project server
- remote multi-user collaboration
- cloud indexing
- whole dyld-cache decompilation before any UI is shown
- eager full KDK/DWARF import
- eager whole-cache graph layout
- eager pseudocode for all functions
- Swift-side storage of full binary bytes
- unbounded in-memory search results
- automatic analysis jobs that cannot be canceled
- a performance dashboard that lacks test-enforced budgets

The first wave should make a local, single-user, path-backed, incremental
project runtime viable.

## Target Runtime Stack

```text
Native Workbench
  ProjectSessionStore
  NavigationStore
  QueryStores
  JobStore
  MetricsStore
  Paged view models
       |
       v
Local API
  project/source endpoints
  paged query endpoints
  job control endpoints
  event stream
  metrics endpoints
       |
       v
Project Runtime
  project registry
  source registry
  source access service
  analysis database
  job scheduler
  indexer
  hot cache manager
  invalidation engine
       |
       v
Source Access
  mmap-backed files
  split cache members
  projected image windows
  kernel collection members
  sidecar debug data
```

## Required New Modules

Suggested Rust modules:

- `damsel-core::perf`
- `damsel-core::metrics`
- `damsel-core::job`
- `damsel-core::paging`
- `damsel-core::query_index`
- `damsel-core::source_access`
- `damsel-core::snapshot`
- `damsel-macho::project_loader`
- `damsel-macho::analysis_scheduler`
- `damsel-macho::analysis_passes`
- `damsel-cli::project_api`
- `damsel-cli::event_stream`

Suggested Swift groups:

- `ProjectSessionStore`
- `SourceRegistrationStore`
- `JobProgressStore`
- `PagedQueryStore`
- `RouteQueryStore`
- `PerformanceProbeStore`
- `WorkbenchEventStream`

Rules:

- model types live in Rust first
- Swift stores mirror API state; they do not own analysis truth
- performance counters are API-visible and testable
- every expensive operation has a job id or explicit query budget

## Source Registration Architecture

### Current Problem

The current upload path copies bytes through the Workbench and local server.
That is costly for large files and structurally incompatible with split dyld
caches, driver caches, kernel collections, KDK bundles, dSYMs, and persistent
projects.

### Target API

```text
POST /api/projects
POST /api/projects/{project_id}/sources/register-path
POST /api/projects/{project_id}/sources/register-sidecar
GET  /api/projects/{project_id}/sources/{source_id}
GET  /api/projects/{project_id}/sources/{source_id}/availability
```

`register-path` request:

```json
{
  "path": "/System/Library/dyld/dyld_shared_cache_arm64e",
  "kind_hint": "dyld_shared_cache",
  "privacy": "local_path",
  "analysis_profile": "inventory_first"
}
```

Response:

```json
{
  "source_id": "source:dyld-cache:1",
  "kind": "dyld_shared_cache",
  "display_name": "dyld_shared_cache_arm64e",
  "content_identity": {
    "hash": "sha256:...",
    "uuid": null,
    "size_bytes": 4294967296
  },
  "availability": "local",
  "registered_jobs": ["job:source-scan:1"]
}
```

### Security And Privacy

Rules:

- path-based registration is local-only
- diagnostics redact paths by default
- raw bytes are never written to logs
- source bookmarks or sandbox access records are explicit future work
- missing source paths become recoverable source availability problems

### Source Access Service

Responsibilities:

- open source handles lazily
- memory-map file-backed sources where practical
- expose bounded byte windows
- normalize file offsets and virtual addresses
- handle projected cache-image windows
- track source hash and UUID identity
- close mappings when projects close
- expose high-level memory counters

Important distinction:

- source registration records identity and availability
- source access reads bytes on demand
- analysis facts should not store raw bytes unless the specific fact requires a
  short encoded value

## Analysis Job Scheduler

### Why A Scheduler Is Required

Hopper's public changelog repeatedly references background-analysis issues:
large analysis time, infinite loops, hangs, deadlocks, address list
responsiveness, and large database loading. Damsel should design these failure
modes out early.

### Job Record

```json
{
  "job_id": "job:analysis:42",
  "project_id": "project:1",
  "source_id": "source:1",
  "kind": "objc_index",
  "priority": "interactive",
  "state": "running",
  "progress": {
    "phase": "method scan",
    "completed_units": 2400,
    "total_units": 10000,
    "message": "Indexing Objective-C methods"
  },
  "dependencies": ["job:load-segments:4"],
  "started_at": "2026-05-26T20:00:00Z",
  "updated_at": "2026-05-26T20:00:01Z",
  "cancelable": true
}
```

### Job States

- created
- queued
- running
- cancel_requested
- canceled
- completed
- completed_with_warnings
- failed
- blocked
- stale

### Priority Classes

- `foreground`: route jump, visible function, user-selected selector
- `interactive`: search first page, selected source inventory, visible graph
- `background`: whole-source xrefs, all functions, full string index
- `maintenance`: compaction, cache eviction, stale fact cleanup
- `opt_in_heavy`: full cache scan, KDK import, whole-project summary

Scheduling rules:

- foreground jobs can preempt background jobs at safe checkpoints
- background jobs checkpoint frequently
- opt-in heavy jobs require explicit user action
- all jobs have cancellation tokens
- jobs emit progress at bounded cadence
- jobs write partial facts with `partial` or `stale` status where appropriate
- failed jobs become problem records

### Pass Dependency Graph

Initial pass graph:

```text
source_detect
  -> segment_section_index
      -> symbol_import_export_index
      -> string_index_light
      -> objc_metadata_index
      -> function_seed_index
          -> procedure_recovery
              -> xref_index
              -> basic_block_index
              -> semantic_instruction_index
                  -> decompiler_lite_summary
                  -> ir_later
      -> dyld_cache_image_index
          -> selected_image_projection
      -> kernel_member_index
          -> selected_member_projection
```

Rules:

- source detection and segment/section indexing should finish before deep work
- symbol/string/runtime indexes can provide early navigation while procedures
  recover
- selected routes should boost related jobs
- failed optional passes should not block unrelated routes
- each pass declares input fact versions and output fact kinds

## Incremental Analysis And Invalidation

### Fact Versioning

Every derived fact records:

- analyzer id
- analyzer version
- input source identity
- input fact versions
- created project revision
- confidence
- completeness

Completeness values:

- complete
- partial
- stale
- canceled
- failed
- superseded

### Invalidation Triggers

Triggers:

- source hash changed
- sidecar attached or detached
- analysis profile changed
- analyzer version changed
- byte classification overridden
- procedure bounds changed
- type imported or edited
- annotation hint applied
- patch preview changes source revision
- architecture semantics version changed

Rules:

- invalidation marks affected facts stale before recomputing
- stale facts can still be displayed with visible warning when useful
- user annotations are never deleted by analyzer invalidation
- recomputation is job-backed and cancelable

### Partial Results

Partial results are allowed when:

- a pass is canceled
- a source is huge
- an artifact is malformed
- a sidecar is missing
- a pass is intentionally light-mode

Partial result requirements:

- APIs expose partial status
- Workbench displays partial state without panic wording
- MCP and CLI responses include completeness metadata
- validation fixtures include partial/stale cases

## Index Architecture

### Index Families

Required indexes:

- source index
- segment/section index
- byte classification index
- name/symbol index
- string index
- procedure index
- basic block index
- xref index
- call graph index
- ObjC selector/class/method index
- C++ type/vtable/RTTI index
- Swift demangled-name index
- dyld cache image/dependency/linkage index
- kernel member/kext/IOKit/MIG/syscall index
- problem index
- route index
- annotation index
- type/signature index
- search index
- job index
- metrics index

### Query Requirements

Every large index query must support:

- project id
- optional source id
- optional route scope
- filter text
- sort key
- page limit
- cursor
- result version
- completeness
- stale status

No Workbench list should require a full unpaged result for:

- symbols
- strings
- procedures
- xrefs
- selectors
- classes
- methods
- C++ types
- vtables
- cache images
- kexts
- problems
- jobs

### Search Index

Search should be multi-domain:

- names
- symbols
- procedures
- strings
- selectors
- classes
- protocols
- categories
- methods
- C++ types
- vtables
- imports
- exports
- kexts
- syscalls
- MIG routines
- panic frames
- comments
- tags
- problems

Ranking signals:

- exact match
- prefix match
- substring match
- fuzzy match
- selected source boost
- selected runtime object boost
- user annotations boost
- problem severity boost
- recent navigation boost

Search output:

```json
{
  "query": "objc_msgSend",
  "items": [
    {
      "route": "route:symbol:project-1:objc_msgSend",
      "kind": "symbol",
      "display": "_objc_msgSend",
      "subtitle": "import from libobjc.A.dylib",
      "score": 0.98,
      "provenance": []
    }
  ],
  "page": {
    "limit": 50,
    "next_cursor": "opaque",
    "result_version": "search:12"
  }
}
```

### Xref Index

The xref index must support:

- to/from address
- to/from object id
- to/from runtime object
- direct call
- tail call
- branch
- data load
- string use
- selector use
- class ref
- import/stub call
- virtual call candidate
- cache linkage
- kernel symbol use
- panic frame mapping

Rules:

- xrefs include direction, route, source, confidence, and analyzer version
- xref queries return first page fast
- reverse xrefs are materialized or indexed, not recomputed from all
  instructions on each click

## Paged API Surface

### Endpoint Shape

```text
GET /api/projects/{project_id}/query/{query_kind}
```

Common query parameters:

- `source_id`
- `route`
- `filter`
- `sort`
- `limit`
- `cursor`
- `include_stale`
- `include_provenance`

Common response:

```json
{
  "items": [],
  "page": {
    "limit": 100,
    "next_cursor": "opaque",
    "result_version": "index:42",
    "total": null,
    "total_is_exact": false
  },
  "completeness": "partial",
  "jobs": ["job:xref-index:7"],
  "warnings": []
}
```

Rules:

- `total` is optional when expensive
- cursors are opaque
- result versions detect stale UI pages
- pages should be small enough for Swift decoding to stay cheap
- UI stores request adjacent pages opportunistically

### Event Stream

Use server-sent events, WebSocket, or a simple polling fallback.

Event kinds:

- project_changed
- source_changed
- job_started
- job_progress
- job_completed
- job_failed
- facts_inserted
- facts_invalidated
- route_updated
- metrics_sample
- problem_created
- automation_activity

Event rules:

- events are coalesced for high-volume fact changes
- event payloads do not include raw bytes
- UI can recover by querying project version after reconnect
- event stream is optional for CLI batch mode

## Workbench Responsiveness Architecture

### Store Split

`WorkbenchStore` should not remain the owner of all state. Split into:

- `ProjectSessionStore`
- `SourceStore`
- `NavigationStore`
- `CommandStore`
- `JobStore`
- `ProblemStore`
- `SearchStore`
- `PagedListStore`
- `RepresentationStore`
- `MetricsStore`

Rules:

- each store has clear ownership
- expensive decoding happens off the main actor
- main actor only publishes compact state changes
- stale request ids or result versions prevent old results from overwriting
  current state

### Paged List Model

Use a generic paged list abstraction for:

- symbols
- strings
- functions
- xrefs
- selectors
- classes
- cache images
- kernel members
- problems
- jobs

Model state:

- current query
- loaded pages
- cursor map
- result version
- selection route
- loading state
- stale state
- error state

Rules:

- filters debounce and cancel old requests
- selection is route-based, not index-based
- refresh preserves selection if the route still exists
- list cells are small value types
- list pages can be dropped under memory pressure

### Main-Actor Budget

Main actor may:

- publish state
- update selections
- dispatch commands
- apply small decoded page results
- render existing view models

Main actor must not:

- read full binary bytes
- filter huge arrays
- decode huge JSON payloads
- compute graph layout
- build search indexes
- import KDK/DWARF data
- assemble whole project diagnostics

### Swift Decoding Strategy

Rules:

- decode paged responses in background tasks
- use compact DTOs for list rows
- defer heavy details until route selection
- avoid optional megastruct payloads where separate endpoints are clearer
- prefer stable ids and route strings in list rows
- use `result_version` to discard stale decoded pages

## Hot Cache Strategy

### Cache Classes

Hot caches:

- project manifest
- route resolution
- selected source section map
- selected function assembly page
- selected function graph
- selected route inspector modules
- recent search first page
- recent xref pages
- feature-map bands

Warm caches:

- string index pages
- symbol search pages
- procedure list pages
- ObjC selector pages
- cache image list pages
- kernel member pages

Cold rebuildable caches:

- graph layout caches
- decompiler-lite summaries
- large string suffix indexes
- full xref shards
- KDK type lookup tables

Rules:

- caches are keyed by project id, source id, route, query, analyzer version, and
  fact version
- cache eviction is observable in metrics
- cache misses are not errors
- stale cache entries are never served as current without marking them stale

## Memory Architecture

### Source Bytes

Rules:

- file-backed sources use memory mapping where practical
- Swift should not store full bytes for project workflows
- byte windows are bounded by API limits
- projected images refer to parent source windows rather than duplicating bytes
- diagnostics do not include raw bytes by default

### Analysis Facts

Rules:

- facts are compact and normalized
- long rendered strings are cached separately from canonical facts
- large indexes can be sharded by source or address-space range
- decompiler-lite summaries can be evicted and rebuilt
- graph layout is a derived cache, not canonical CFG truth

### Memory Metrics

Expose:

- source handle count
- mapped source bytes
- hot cache bytes
- database cache bytes
- query result bytes
- recent response size
- Swift decoded item counts
- high-water estimates where available

## Cancellation And Timeouts

### Cancellation Points

Every long pass should checkpoint:

- after source chunk
- after segment
- after procedure
- after basic block batch
- after string batch
- after selector/class batch
- after xref shard
- after type import batch
- after graph layout iteration

### Timeout Policy

Timeouts:

- foreground query soft budget: 100 ms warm, 500 ms cold first useful response
- interactive job progress budget: visible within 500 ms
- search first page budget: 150 ms warm index
- route resolution budget: 100 ms warm
- cancel acknowledgement: immediate UI state change

Rules:

- timeout does not silently kill data integrity operations
- timeout creates partial response or queued job reference where possible
- UI shows "analysis still running" rather than spinner-only state

## Diagnostics And Instrumentation

### Trace Spans

Add spans for:

- source register
- source map open
- load commands parse
- dyld cache inventory
- image projection
- ObjC metadata index
- string index
- function seed collection
- procedure recovery
- xref indexing
- graph layout
- decompiler-lite summary
- route resolution
- search query
- paged query
- JSON encode/decode
- Swift response decode
- main actor apply
- inspector update

### Metrics Endpoint

```text
GET /api/projects/{project_id}/metrics
```

Response categories:

- project size
- source count
- fact counts
- index versions
- job timings
- query timings
- response sizes
- cache sizes
- memory estimates
- cancellation stats
- recent slow queries

### Diagnostics Export

Diagnostics export should include:

- version info
- project schema version
- analyzer versions
- source identities without raw paths by default
- job history
- problem list
- slow query samples
- cache summary
- index status
- recent errors

Diagnostics export should exclude by default:

- raw binary bytes
- private strings
- comments
- local full paths
- debug sidecar contents
- stack traces with sensitive paths unless user opts in

## Implementation Phases

### Phase P0: Measurement Foundation

Deliverables:

- Rust timing/span helper
- query duration logging
- response size logging
- Workbench main-actor apply timing
- Workbench route/search/selection probes
- metrics DTOs
- benchmark output schema alignment

Validation:

- unit tests for metric aggregation
- snapshot for metrics endpoint on fixture project
- Workbench probe test for selection-to-inspector timing where feasible

### Phase P1: Path-Based Source Registration

Deliverables:

- project create endpoint
- path-based source registration endpoint
- source identity record
- local source availability check
- source byte-window service
- mmap-backed read path where practical
- compatibility fallback for upload endpoint

Validation:

- source registration fixture
- missing source fixture
- source hash mismatch fixture
- byte-window fixture
- upload compatibility test

### Phase P2: Job Scheduler

Deliverables:

- job model
- scheduler queue
- priorities
- cancellation token
- job progress API
- job events or polling
- failed job to problem record

Validation:

- job lifecycle fixture
- cancellation fixture
- failed job problem fixture
- priority ordering test

### Phase P3: First Index Shards

Deliverables:

- segment/section index
- symbol/name index
- string index light mode
- procedure seed index
- route index
- problem index
- paged query shape

Validation:

- paged symbols fixture
- paged strings fixture
- route resolution fixture
- stale cursor fixture
- first-page latency benchmark

### Phase P4: Procedure And Xref Indexing

Deliverables:

- persistent procedure index
- basic-block index
- call graph index
- xref shards
- query endpoints
- selected-function analysis priority boost

Validation:

- function list fixture
- function detail fixture
- xref to/from fixture
- selected-route priority fixture
- background cancellation fixture

### Phase P5: Workbench Store Split And Paged Views

Deliverables:

- `ProjectSessionStore`
- `JobStore`
- `SearchStore`
- generic paged list store
- route-backed selection
- paged symbol/function/string navigators
- progress drawer
- metrics debug panel behind developer mode

Validation:

- Workbench model tests
- stale response discard test
- selection preservation test
- search cancellation test
- UI smoke on fixture project

### Phase P6: Large Apple Artifact Optimization

Deliverables:

- dyld cache inventory first paint
- selected image projection job
- driver-cache source type
- kernel collection member inventory
- KDK/dSYM import as opt-in background jobs
- cache-wide search shards
- cache image route paging

Validation:

- synthetic split-cache benchmark
- projected image benchmark
- kernel/fileset fixture
- KDK mismatch fixture
- opt-in real dyld cache smoke if claims mention real caches

### Phase P7: Heavy-Derived-View Caches

Deliverables:

- graph layout cache
- feature-map band cache
- decompiler-lite summary cache
- large xref shard compaction
- cache eviction metrics

Validation:

- graph cache hit/miss fixture
- feature-map page fixture
- summary cache invalidation fixture
- cache eviction fixture

## API Contract Additions

New core objects:

- `SourceRegistrationRequest`
- `SourceIdentity`
- `SourceAvailability`
- `SourceAccessHandle`
- `ByteWindow`
- `JobRecord`
- `JobProgress`
- `JobEvent`
- `AnalysisPass`
- `AnalysisPassDependency`
- `FactCompleteness`
- `IndexStatus`
- `IndexShard`
- `PagedQueryRequest`
- `PagedQueryResponse`
- `QueryCursor`
- `ResultVersion`
- `PerformanceMetric`
- `SlowQueryRecord`
- `CacheMetric`
- `DiagnosticsExportPolicy`

Endpoint groups:

- project lifecycle
- source registration
- byte windows
- jobs
- query indexes
- metrics
- diagnostics
- event stream

## Workbench Contract Additions

New Swift DTO groups:

- `ProjectSessionDTO`
- `SourceSummaryDTO`
- `SourceAvailabilityDTO`
- `JobRecordDTO`
- `JobEventDTO`
- `PagedResultDTO<Item>`
- `QueryCursorDTO`
- `MetricSampleDTO`
- `SlowQueryDTO`

New UI surfaces:

- source registration progress
- project job drawer
- per-source analysis status
- indexed search progress
- stale fact indicator
- partial result indicator
- diagnostics export sheet
- developer metrics panel

## Benchmark Plan

### Fixture Benchmarks

Benchmarks:

- source registration small fixture
- byte-window query
- section index query
- symbol search first page
- string search first page
- function list first page
- route resolution
- xref first page
- job cancellation checkpoint
- metrics endpoint

### Synthetic Container Benchmarks

Benchmarks:

- synthetic dyld cache inventory
- split cache member list
- projected image registration
- synthetic kernel collection member list
- source missing recovery
- source hash mismatch recovery

### Opt-In Real Artifact Benchmarks

Benchmarks:

- local dyld cache inventory time
- selected cache image first useful UI
- selected cache image symbol search
- selected Objective-C-heavy framework selector search
- local kernel collection member inventory
- KDK attach status

Rules:

- real artifact benchmarks are opt-in
- output records exact artifact kind and run date
- local paths are redacted unless explicitly requested
- no public claim may cite a real artifact benchmark that was not actually run

## Acceptance Gates

Gate P0:

- metrics endpoint exists for a fixture project
- Workbench can report route/search/selection timings in a developer-only path
- response sizes are measurable

Gate P1:

- Workbench can register a local source by path without uploading full bytes
- source identity and availability are persisted
- missing source is recoverable
- upload endpoint remains available for compatibility

Gate P2:

- background job lifecycle is visible in API and Workbench
- cancel request is acknowledged immediately
- canceled job does not look successful
- failed job creates a problem record

Gate P3:

- symbols, strings, procedures, and problems have paged query contracts
- stale cursor errors are typed
- search first page is benchmarked
- Swift does not materialize full large lists

Gate P4:

- procedure and xref indexes are durable project facts
- selected function analysis can be prioritized
- xref first page does not require full reanalysis

Gate P5:

- Workbench store split is complete enough that selection, jobs, search, and
  representation state have separate owners
- route-backed selection survives refresh
- search cancellation discards stale results

Gate P6:

- dyld cache inventory can show useful metadata before deep analysis
- selected image analysis is job-backed
- kernel collection member inventory is job-backed
- KDK/dSYM attachment is opt-in and progress-visible

## Risks And Mitigations

Risk: project runtime becomes too complex before core facts stabilize.

Mitigation:

- begin with in-memory models and deterministic snapshots
- add SQLite or embedded indexes only after object identity and invalidation
  rules are stable

Risk: background jobs corrupt project facts when canceled.

Mitigation:

- write facts in batches with pass revision ids
- mark partial results explicitly
- commit only consistent fact batches

Risk: Workbench decodes huge responses on main actor.

Mitigation:

- enforce paged APIs
- measure response sizes
- decode in background tasks
- keep list rows compact

Risk: source path registration leaks private paths in diagnostics.

Mitigation:

- separate source identity from diagnostics display
- redact paths by default
- require explicit export policy for private data

Risk: dyld cache and kernel workflows require too much eager analysis.

Mitigation:

- inventory first
- selected image/member projection second
- cross-image analysis only when requested or needed by selected route

Risk: caches hide stale facts.

Mitigation:

- cache keys include fact versions and analyzer versions
- stale display is explicit
- invalidation tests cover source, sidecar, byte map, type, and analyzer
  changes

Risk: metrics become decorative.

Mitigation:

- include performance gates in CI where feasible
- include benchmark output in milestone audits
- fail release claims that lack matching measurements

## How This Fits With Existing Damsel

The implementation should preserve current commands and alpha Workbench paths
while adding project-backed alternatives.

Keep:

- current standalone CLI commands
- current cache CLI commands
- current upload/disasm UI path as compatibility mode
- current fixture and snapshot tests
- current `NSTableView` instruction table foundation

Add beside current paths:

- project create/open/register-source commands
- path-based local project API
- source access service
- job scheduler
- indexed query endpoints
- Workbench project mode
- performance metrics

Migration rule:

- no existing point-query behavior should be removed until project-backed
  equivalents have tests and Workbench coverage.

## Completion Criteria

This performance tranche is complete only when:

- source registration by path exists and avoids whole-file Swift upload for
  project workflows
- long analysis work is job-backed and cancellable
- progress is visible in Workbench
- large lists are paged and indexed
- route/search/xref queries report result versions
- stale cursors and stale facts are typed conditions
- Workbench state is split enough to avoid one all-purpose store controlling
  every expensive surface
- metrics exist for source registration, search, route resolution, disassembly
  page fetch, graph fetch, job progress, response size, and main-actor apply
- benchmark fixtures cover small binaries, synthetic dyld caches, synthetic
  kernel collections, and opt-in real artifacts where claimed
- no broad performance claim is made without a matching measured gate
