# Refactor And Subsystem Plan

Status: Deep engineering plan. This document names the large refactors that are
allowed, and often required, to make Damsel competitive.

Purpose: avoid under-scoping the Hopper-gap work as a collection of small
feature patches. Some features are small additions; others require new
subsystems and deep integration with existing crates.

## Refactor Thesis

Damsel's current architecture is good for a precise static Mach-O and
shared-cache analyzer. Hopper-level UX and Damsel's Apple systems niche require
new long-lived concepts:

- project
- source registry
- address-space graph
- analysis database
- background job scheduler
- fact/provenance system
- procedure database
- architecture semantics layer
- xref index
- graph service
- normalized runtime object store
- kernel collection object model
- type/debug-info store
- decompiler IR store
- patch preview and source revision store
- debugger session and runtime mapping store
- UI route/navigation model
- typed command registry and dispatcher
- annotation and undo system
- automation capability and activity model
- extension manifest and helper-process boundary
- source access service
- artifact intake and source registration service
- performance scheduler and index runtime
- diff/export/evidence package service
- native app trust, permissions, and diagnostics runtime
- performance telemetry

These are not optional polish layers. They are the difference between "runs a
disassembly command" and "is an interactive reverse-engineering app."

Command/navigation implementation companion:
[Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
details the refactor from one-off AppKit selectors and notification names into
a typed command registry, dispatcher, keymap store, focus store, route-backed
selection, navigation history, activity log, and project transaction bridge.

## Subsystem 1: Project Runtime

### Problem

Current UI/server state is image-oriented and process-local. A user can upload
an image, disassemble a target, and inspect the returned payload. That does not
support durable analysis, autosave, navigation stack, annotations, background
analysis, cross-image cache routing, or large source registration.

### New Module

Suggested modules:

- `damsel-core::project`
- `damsel-macho::project_runtime`
- `damsel-cli::project_commands`
- `damsel-cli::project_api`

### Responsibilities

- create project
- open project
- close project
- register source
- attach sidecar
- store project settings
- store analysis status
- expose project manifest
- manage project database handle

### Existing Code To Integrate

- `load` and `load_bytes`
- `inspect_shared_cache`
- `project_image`
- UI image upload route
- Workbench server bootstrap and API client

### Migration Strategy

1. Add project runtime beside current image store.
2. Keep upload/disasm endpoints unchanged.
3. Add project creation endpoint.
4. Teach Workbench to use project mode for local files.
5. Keep legacy upload mode for browser demos and tests.

### Risk

If project runtime is built as a UI-only feature, CLI and tests will drift. The
project runtime must be usable from CLI first.

## Subsystem 1A: Native App Trust, Permissions, And Diagnostics Runtime

### Problem

The current Workbench starts a local CLI UI helper and uploads selected files,
but it does not yet model the trust boundary around a distributed native app:
signing, notarization, hardened runtime, local helper authentication, source
access grants, security-scoped bookmarks, diagnostic redaction, crash/hang
recovery, update channels, plugin trust, or debugger entitlement readiness.

### New Modules

Suggested modules:

- `damsel-core::trust`
- `damsel-core::diagnostics`
- `damsel-cli::trust_api`
- `damsel-cli::diagnostics_api`
- `DamselAppCore/Stores/TrustStore.swift`
- `DamselAppCore/Stores/SourceAccessStore.swift`
- `DamselAppCore/Stores/DiagnosticStore.swift`
- `DamselAppCore/Stores/HelperStore.swift`

### Responsibilities

- record app build identity
- record helper identity and protocol version
- authenticate local helper requests
- enforce loopback-only helper binding
- record source access grants and privacy classes
- handle security-scoped bookmark lifecycle when sandboxed
- export redacted diagnostic bundles
- record helper crash and hang summaries
- expose signing/notarization/hardened-runtime state
- model update channel and signature state
- model future plugin and debugger trust records

### Existing Code To Integrate

- `DamselServerProcess`
- `DamselAPIClient`
- `WorkbenchStore`
- local UI server launch path
- current `/api/images` upload route
- future project source registration endpoints

### Migration Strategy

1. Add trust and diagnostics models with dev-mode static values.
2. Add token-authenticated Workbench-mode helper sessions.
3. Move Workbench file opening to project source grants.
4. Add diagnostics export with default redaction.
5. Add helper crash/hang recovery state.
6. Add signing/notarization/update gates only when distribution artifacts
   exist.

### Risk

If this stays as packaging documentation only, Damsel will accidentally design
automation, plugins, debugger, and diagnostics around a soft local trust
boundary. The helper/session/source-grant model must be part of the app and API
architecture early.

## Subsystem 2: Source Registry

### Problem

Apple systems artifacts are multi-source:

- root dyld cache
- subcaches
- `.symbols` sidecars
- projected images
- kernel collection
- fileset members
- KDK bundles
- dSYM files
- panic logs
- extracted IPSW assets

A single `BinaryImage` cannot describe this graph.

### New Model

```text
AnalysisSource {
  id
  kind
  role
  path
  content_hash
  file_size
  uuid
  architecture
  platform
  build_version
  parent_source_id
  relationship
}
```

### Required Refactor

Move source identity out of ad hoc command context and into reusable project
state.

### Integration

- `SharedCacheMember` maps into source registry.
- `ProjectedImageProvenance` maps into source relationships.
- dSYM/KDK matching records attach to source entries.

## Subsystem 2A: Artifact Intake And Registration

### Problem

The source registry needs an intake front door. Users will not always open the
exact final Mach-O slice Damsel wants. They will open universal binaries, dyld
cache roots, subcaches, driver caches, kernel collections, KDK bundles, dSYM
bundles, panic logs, app bundles, directories of extracted Apple assets, or
unknown/protected files.

### New Modules

Suggested modules:

- `damsel-core::intake`
- `damsel-macho::intake`
- `damsel-cli::intake_commands`
- `damsel-cli::intake_api`
- `DamselAppCore/Stores/IntakeStore.swift`

### Responsibilities

- classify inputs before full analysis
- produce candidate source registration plans
- preserve ambiguous candidates
- request loader parameters from the user
- register root/member/sidecar/projected sources
- preserve extraction/projection provenance
- discover KDK/dSYM/panic sidecars
- create artifact overview jobs
- report protected, encrypted, malformed, missing, and mismatched inputs as
  typed problems

### Existing Code To Integrate

- `damsel-macho::loader::load`
- `damsel-macho::loader::load_bytes`
- `select_slice`
- `SharedCacheSession`
- `ProjectedBinaryImage`
- `ProjectedImageProvenance`
- Workbench file open and upload path

### Migration Strategy

1. Add intake models and snapshots without changing existing commands.
2. Add project ingest for standalone and universal Mach-O.
3. Add dyld cache source graph registration beside existing cache commands.
4. Add sidecar and panic attachment as project operations.
5. Move Workbench project-mode opening to intake; keep upload mode for demos.

### Risk

If Damsel keeps intake implicit, later KDK, dSYM, panic, driver-cache, and
IPSW-root work will each create incompatible mini-importers. The artifact graph
should be the shared foundation.

## Subsystem 3: Address-Space Graph

### Problem

Damsel already distinguishes cache vmaddrs, image base vmaddrs, image offsets,
and member file offsets. Hopper-level navigation across shared caches, kernel
collections, projected images, and live/debug addresses needs this generalized.

### New Model

```text
AddressSpace {
  id
  kind
  source_id
  parent_id
  base
  size
  transform
}
```

Kinds:

- file
- standalone image
- universal slice
- dyld cache
- cache member
- projected cache image
- kernel collection
- fileset member
- slid runtime
- debug sidecar

### Required Refactor

Replace new feature code that accepts raw `u64` addresses with address-space
aware types. Existing public commands can keep raw address flags, but internal
records should carry address-space identity.

### Integration

- disassembly requests
- xrefs
- graph nodes
- pointer facts
- panic symbolication
- debugger bridge later

## Subsystem 4: Fact And Provenance Engine

### Problem

As Damsel adds inferred ObjC message targets, C++ virtual calls, kernel table
entries, and pointer-auth interpretation, truth becomes layered. Users need to
know what is known, inferred, ambiguous, or manually overridden.

### New Model

```text
Fact {
  id
  subject
  kind
  payload
  source
  confidence
  analyzer_id
  analysis_run_id
}
```

Confidence:

- exact
- strong
- inferred
- ambiguous
- weak
- user_asserted
- contradicted

### Required Refactor

Current analysis structs can remain, but project-level features should emit
facts. Output views can then be built from facts or from compatibility
structs.

### Integration

- `DisassemblyAnalysis`
- `ObjcMetadata`
- `DyldMetadata`
- `SharedCache` relations
- new C++/kernel analyzers
- annotations

## Subsystem 5: Analysis Database

### Problem

Global search, xrefs, graph navigation, annotation persistence, and reopening
projects need indexed state.

### New Module

Suggested path:

- `damsel-core/src/analysis_db.rs` for model traits/records
- `damsel-macho/src/analysis_db.rs` for in-memory implementation
- optional future `damsel-store` crate for SQLite or sharded persistence

### Required Tables

- sources
- address spaces
- facts
- byte ranges
- functions
- blocks
- edges
- references
- names
- strings
- runtime objects
- kernel objects
- graphs
- annotations
- analysis runs
- problems

### Required Refactor

Stop treating a disassembly response as the primary object for UI navigation.
The primary object becomes the project database; disassembly responses become
views into that database.

## Subsystem 6: Analysis Scheduler

### Problem

Full analysis of dyld caches and kernel collections can be expensive. A
competitive app must remain usable during analysis and allow cancellation.

### New Module

- `analysis_scheduler`

### Responsibilities

- enqueue passes
- track dependencies
- prioritize foreground work
- publish progress
- handle cancellation
- record warnings/errors
- invalidate stale facts

### Required Refactor

Decouple analysis from request/response endpoints. Endpoints should start jobs,
poll jobs, and query completed facts.

### Integration

- CLI `project analyze`
- API `POST /api/projects/{id}/analyze`
- Workbench progress UI
- validation harness

## Subsystem 7: Paged Query Service

### Problem

Huge lists cannot be returned as single JSON arrays:

- functions
- symbols
- strings
- xrefs
- selectors
- C++ types
- kernel symbols
- graph nodes

### New Module

- `query_service`

### Responsibilities

- filters
- sorting
- paging
- cursors
- stable result version
- fuzzy search
- route id resolution

### Required Refactor

Workbench should stop binding large views to entire arrays. It should bind to
paged query models.

### Integration

- Damsel API client
- Workbench stores
- CLI list commands

## Subsystem 8: Route And Navigation Model

### Problem

Hopper-like UX needs a navigation stack across addresses, functions, graph
nodes, ObjC objects, C++ objects, cache images, and kernel members.

### New Model

```text
Route {
  project_id
  target_kind
  target_id
  address?
  representation
  selection?
  viewport?
}
```

### Required Refactor

Selection state should move from only `selectedInstructionAddress` to a route
model that can address any project object.

### Integration

- Workbench navigation stack
- API object ids
- global search
- command palette
- deep links

## Subsystem 9: Annotation And Undo Store

### Problem

Comments, labels, tags, bookmarks, colors, manual refs, and manual function
bounds are essential app features. They must persist and support undo/redo.

### New Model

```text
Annotation {
  id
  target
  kind
  payload
  author
  timestamp
}

AnnotationTransaction {
  id
  operations
  inverse_operations
  timestamp
}
```

### Required Refactor

Names displayed in UI should come from a name resolution service that considers
user labels and inferred names with priority.

### Integration

- function list
- assembly view
- graph view
- xref view
- ObjC/C++/kernel object views

## Subsystem 10: Runtime Object Store

### Problem

Current ObjC metadata is attached to `BinaryImage`. Future ObjC/C++/Swift/kernel
views need normalized runtime objects and cross-object links.

### New Objects

- ObjC class/protocol/category/method/selector/message-send
- C++ namespace/type/method/vtable/typeinfo/virtual-call
- Swift symbol/metadata placeholder
- IOKit class/external method table

### Required Refactor

Keep parser-specific structs as inputs, but normalize to project runtime
objects for navigation and search.

### Integration

- ObjC parser
- disassembly value-flow
- C++ ABI parser
- kernel recognizers
- graph service

## Subsystem 11: Kernel Container Runtime

### Problem

Kernel collections and filesets are not just "large Mach-O files." They contain
members, kext identity, global/member address mappings, KDK symbols, and
kernel-specific tables.

### New Module

- `kernel_collection`

### Responsibilities

- detect fileset/kernel collection
- list members
- map member address spaces
- extract member images
- identify kexts
- attach KDK/dSYM
- run kernel recognizers

### Required Refactor

Generalize shared-cache-like container analysis so kernel collections can be
first-class without corrupting dyld shared-cache concepts.

## Subsystem 12: Graph Service

### Problem

The current disassembly analysis can produce local blocks and edges, but app
graph UX needs durable graphs, layouts, minimaps, and cross-object graph
queries.

### New Module

- `graph_service`

### Responsibilities

- build graph records
- cache layouts
- export JSON/DOT
- page huge graphs
- provide minimap geometry
- sync graph nodes to assembly rows

### Required Refactor

Graph facts should not live only inside one disassembly response. They should
be project records keyed by function/runtime/kernel object.

## Subsystem 13: Source Access, Scheduler, And Index Runtime

### Problem

The current Workbench path uploads bytes into the local UI server and the
server stores loaded images in memory. That works for alpha point queries, but
large dyld caches, kernel collections, driver caches, dSYM/KDK bundles, and
long-running Workbench sessions need path-based source registration,
mmap-backed byte access, cancellable background jobs, and paged indexes.

### New Modules

- `source_access`
- `analysis_scheduler`
- `query_index`
- `paging`
- `event_stream`

### Responsibilities

- register local source paths without whole-file Swift upload
- preserve source identity, hash, UUID, role, and availability
- expose bounded byte windows
- map projected image ranges to parent source ranges
- schedule analysis passes with priority and cancellation
- record job progress and failures
- maintain index status records
- expose paged queries with result versions
- emit coalesced project/job/fact events
- keep current upload/disasm path as compatibility mode during migration

### Required Refactor

Long analysis should not run as an opaque request handler that returns one large
payload. The project runtime should create jobs, write facts incrementally,
serve indexes through paged APIs, and let Workbench show progress, partial
results, cancellation, stale facts, and recoverable source problems.

## Subsystem 14: Performance Telemetry

### Problem

Competitiveness requires measured responsiveness, not vibes.

### New Module

- `telemetry`

### Metrics

- source load time
- parse time
- analysis pass time
- query time
- graph layout time
- API response time
- UI first result time
- UI selection latency
- memory estimates where available

### Required Refactor

Analysis and UI paths need named spans/timers. The validation harness should
capture and compare them.

## Subsystem 15: Automation, MCP, And Extension Boundary

### Problem

Hopper has scripting, MCP, and SDK surfaces. Damsel should eventually support
automation and extensibility, but only through the same project/query/
annotation APIs used by CLI and Workbench. Automation that bypasses those APIs
would fork the product model and make future debugger, patching, and plugin
work unsafe.

### New Modules

- `automation`
- `tool_api`
- `extension_runtime`
- `plugin_manifest`

### First Automation Version

- read-only query scripts
- structured CLI JSON batch mode
- read-only local MCP tools
- annotation writes
- automation activity records
- report export
- batch symbolication

### First Extension Boundary Version

- internal loader traits
- internal sidecar importer traits
- internal analyzer-pass traits
- renderer/exporter traits
- Workbench command descriptors
- versioned plugin manifest schema
- helper-process protocol design

### Required Refactor

Public automation should use the same project/query/annotation APIs as CLI and
Workbench. Extension output should emit facts, problems, metrics, and
provenance through stable schemas. Do not expose unstable internal parser
structs, Swift view internals, or raw mutable database handles directly.

### Required Contracts

- `AutomationCapability`
- `ToolManifest`
- `ToolInvocation`
- `AutomationActivity`
- `PluginManifest`
- `ExtensionRun`
- `ExtensionCapability`
- `PluginFactProvenance`
- `AnalyzerRun`

### Risk Controls

- read-only by default
- byte export gated separately from metadata reads
- mutation through transactions only
- plugin facts include plugin id/version/schema
- helper process crash cannot corrupt the project
- loader, architecture semantics, analyzer, exporter, and Workbench command
  boundaries stay separate

## Subsystem 16: Procedure Database

### Problem

The current disassembly path can report basic blocks, edges, imports, ObjC
uses, cache links, recovered values, jump tables, and indirect targets for a
requested window. Hopper's public `HPProcedure` and `HPBasicBlock` surfaces
show that a competitive app needs durable procedure-local facts: frame layout,
locals, register aliases, local labels, callers, callees, signatures, calling
conventions, tags, conflicts, and status.

### New Module

- `procedure_db`

### Responsibilities

- collect procedure seeds
- recover conservative bounds
- persist ranges, blocks, and edges
- record procedure owners
- record local labels and register aliases
- record stack-local placeholders
- record callsites, callers, and callees
- record conflict and uncertainty problems
- expose function detail APIs

### Required Refactor

`DisassemblyAnalysis` should become an input to project-level procedure facts,
not the only home for procedure analysis. Windowed disassembly remains useful,
but procedure records must survive across API calls and Workbench routes.

## Subsystem 17: Architecture Semantics Layer

### Problem

Capstone can decode AArch64 instructions, but Hopper's public CPU plugin
contract shows that the hard product behavior depends on architecture semantics:
CPU modes, branch destinations, switch recovery, prolog detection, thunk
recognition, stack effects, direct memory references, rendering, assembly, and
decompiler cooperation.

### New Module

- `arch_semantics`

### Responsibilities

- normalize decoded instructions
- model registers and pseudo-registers
- model branch, call, return, and halt effects
- recover ADRP/ADD/LDR materialized addresses
- classify memory references
- detect switch/jump-table idioms
- detect thunks and branch islands
- model stack/frame effects
- normalize PAC-signed targets when evidence supports it
- expose instruction semantics to procedure recovery and IR

### Required Refactor

The AArch64 decoder should remain focused on bytes to instructions. New
function recovery, xref, graph, decompiler-lite, and Apple runtime passes
should depend on the semantics layer, not on scattered instruction string
heuristics.

## Subsystem 18: Decompiler IR Store

### Problem

Hopper's public changelog shows long-running investment in decompiler memory
use, stack variables, liveness, AST optimizations, switch recovery, and
architecture-specific correctness. Damsel cannot reach credible pseudocode with
only per-instruction annotations.

### New Module

- `ir`

### Responsibilities

- store low-level IR functions and blocks
- map IR statements to source instructions
- store def-use and SSA facts
- store stack/local recovery facts
- store ABI and call-effect facts
- attach ObjC/C++/Swift-adjacent/kernel overlays
- record unresolved facts and confidence
- feed decompiler-lite summaries

### Required Refactor

Pseudocode should be a view over IR and enriched procedure facts. It should not
be generated directly from text disassembly.

## Subsystem 19: Patch Preview And Source Revision Store

### Problem

Hopper exposes hex editing, assembler, NOP-region operations, Python byte
mutation, and undo. Damsel should not enable binary writes early, but it still
needs source revisions, patchability, patch previews, invalidation plans, and
copy-on-write export design so the future feature is safe.

### New Module

- `patching`

### Responsibilities

- expose read-only byte windows
- compute patchability for ranges
- create patch preview plans
- integrate architecture assembler/NOP generation
- model source revisions
- record patch transactions
- compute invalidated facts
- drive copy-on-write export later

### Required Refactor

Byte access should move from ad hoc command rendering into a source/revision
provider that can serve assembly, hex, patch preview, search, and future debug
memory comparison consistently.

## Subsystem 20: Debug Session And Runtime Mapping Store

### Problem

A debugger bridge is useful only if live module identity and runtime addresses
map back to static project routes. Without that, debugger UI becomes a separate
app bolted onto Damsel.

### New Module

- `debug_runtime`

### Responsibilities

- model debug sessions
- model runtime modules
- match modules to project sources and revisions
- map runtime addresses to routes
- map routes to runtime breakpoint locations
- store breakpoints and stop events
- expose register, frame, and read-only memory records
- record platform failure problems

### Required Refactor

Debug state should be a projection over project routes and address spaces, not
a Swift-only pane or a raw LLDB transcript.

## Subsystem 21: Diff, Export, And Evidence Package Service

### Problem

Hopper exposes BinDiff/BinExport workflows, assembly/pseudocode export, type
export/import, Objective-C header export, and scriptable extraction. Damsel's
Apple-system niche needs a deeper version of this: reproducible analysis
snapshots, object fingerprints, OS-build diffs, route-backed findings,
redacted evidence packages, and reports that explain dyld, ObjC, C++/IOKit,
kernel, KDK, and panic changes.

### New Module

- `diff_export`

### Responsibilities

- create analysis snapshots
- compute object fingerprints
- match objects across snapshots
- record exact, runtime, fuzzy, and ambiguous matches
- generate diff findings
- export canonical JSON evidence
- render Markdown/HTML/CSV/DOT reports from canonical facts
- enforce redaction policy
- support match pin/reject transactions
- provide Workbench diff query APIs

### Required Refactor

Export should not scrape rendered UI text. Diffing and reporting need stable
project facts, route ids, source identities, analyzer versions, and provenance.
BinExport-like compatibility should come after Damsel's own Apple-aware
evidence model is stable.

## Existing-Code Impact Matrix

| Area | Small change or deep refactor | Why |
| --- | --- | --- |
| Add README links | Small | Documentation discoverability only |
| Add project docs | Small | Docs only |
| Add `project create` CLI | Medium | New model plus command, can be JSON-only first |
| Workbench project mode | Deep | Replaces upload-only local path with source registration |
| Global search | Deep | Requires indexes and route ids |
| Function list | Medium | Needs function index but UI can start simple |
| Procedure database | Deep | Converts window-local disassembly facts into durable procedure records |
| Architecture semantics | Deep | Adds semantic AArch64/AArch64e layer above decode |
| Diff/export/evidence | Deep | Requires snapshots, fingerprints, matching, report generation, redaction, and Workbench diff routes |
| Xref pane | Deep | Needs global bidirectional reference table |
| Graph view | Deep | Needs graph records, layout, UI sync |
| Representation sync | Deep | Requires canonical route selection, graph/layout/feature-map/hex stores, loop prevention, accessibility fallback, export, and Workbench state split |
| ObjC selector browser | Medium to deep | Existing metadata helps, but indexes/routes/UI are new |
| ObjC message-send targets | Deep | Needs value-flow and confidence model |
| C++ vtables | Deep | New ABI analyzer and pointer facts |
| Kernel collection loader | Deep | New container model and address-space graph |
| Type/signature/debug-info store | Deep | Requires provenance-aware type records, signatures, variables, calling conventions, sidecar matching, conflicts, transactions, and Workbench/API integration |
| DWARF/dSYM/KDK importer | Deep | Requires debug-source registry, UUID/build matching, lazy DWARF indexes, source-line routes, variable/type import, privacy controls, and profile-aware invalidation |
| Decompiler IR | Deep | Requires procedure DB, semantics, stack/ABI/type/runtime facts |
| Patch preview | Deep | Requires source revisions, exact address mapping, transactions, invalidation, and architecture assembler support |
| Debugger bridge | Deep | Requires runtime module matching, static-dynamic route mapping, helper process, and permission model |
| KDK/dSYM integration | Deep | Debug info parser and matching rules |
| Panic symbolication | Medium after kernel model | Needs slide/address mapping and symbol index |
| Annotation comments | Medium | Needs persistence and name resolution |
| Undo/redo | Deep | Requires transaction model |
| Paged APIs | Deep | Changes server/UI data contract |
| Performance telemetry | Medium | Cross-cutting timers and reports |

## Refactor Sequencing

### Slice A: Foundation Without UI Risk

1. Add core models.
2. Add in-memory analysis database.
3. Add project creation from existing `BinaryImage`.
4. Add CLI JSON commands.
5. Snapshot outputs.

Reason:

- Stabilizes backend truth before the app depends on it.

### Slice B: Query And Navigation Spine

1. Add function index.
2. Add xref index.
3. Add route ids.
4. Add paged query service.
5. Add CLI/API list and detail endpoints.

Reason:

- Enables app navigation and search.

### Slice C: Workbench Migration

1. Add project mode behind feature path.
2. Add source registration.
3. Add project navigator.
4. Add function/symbol/string panes.
5. Add navigation stack.

Reason:

- Turns Workbench into an app, not only a disassembly request surface.

### Slice D: Domain Browsers

1. ObjC browser.
2. C++ browser.
3. Kernel browser.
4. Shared-cache browser.

Reason:

- Makes Damsel's niche visible.

### Slice E: Performance And Polish

1. Analysis scheduler.
2. Progress panel.
3. Problems panel.
4. Graph layout service.
5. UI latency tests.

Reason:

- Keeps larger features from degrading app feel.

## Anti-Patterns To Avoid

- Adding graph UI directly from one disassembly response.
- Adding ObjC browser as another static table without routes/xrefs.
- Adding C++ demangling only to printed symbols without type records.
- Adding kernel support by filename heuristics.
- Adding search by filtering huge Swift arrays.
- Adding comments as UI-only state.
- Adding patching before undo/project persistence.
- Adding debugger before static address-space mapping is trustworthy.
- Treating performance as a final optimization.

## Decision Records To Add Later

Recommended ADRs:

- ADR: Project database and source registry.
- ADR: Address-space graph and address identity.
- ADR: Fact/provenance/confidence model.
- ADR: Read-only default and patching boundary.
- ADR: Workbench as thin native shell over Rust project APIs.
- ADR: Kernel collection support beside dyld shared-cache support.
- ADR: Persistence backend choice.
