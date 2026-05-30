# Graph, Feature Map, Hex, And Representation Sync Implementation Spec

Status: implementation-grade architecture spec.

Date of public-source pass: 2026-05-26.

Purpose: define the visual navigation and representation-synchronization
subsystem Damsel needs to feel close to Hopper Disassembler in day-to-day
reverse-engineering work: interactive function graphs, feature maps, hex/data
views, assembly synchronization, inspector synchronization, route-backed
selection, graph export, accessibility fallbacks, performance budgets, and
Apple-specific overlays.

This document is a companion to:

- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md)
- [API And Data Contracts](api-data-contracts.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)

## Why This Exists

The current pack mentions graph view, feature map, and hex view in many places.
That is not enough. Hopper's public tutorial and SDK make synchronized
representations central to the product:

- a procedure can be shown as assembly, graph, pseudocode, and hex/data
- the graph is not an afterthought; it is a navigation surface
- the inspector changes with the selected object
- the navigation/feature map gives file-wide orientation
- byte typing and manual type conversion are visible in the UI
- graph nodes and procedures carry comments, tags, colors, xrefs, stack facts,
  and problems
- hex view knows instruction byte spans
- undoable edits and operand formatting change how views render

For Damsel, this means "add a graph" is too small. The needed subsystem is a
representation synchronization layer:

- one canonical route selection
- multiple views over the same project facts
- stable object ids for functions, blocks, edges, instructions, byte ranges,
  runtime objects, source lines, and problems
- graph facts separated from graph layout
- progressive layout and cached geometry
- feature-map lanes generated from the typed byte map and Apple overlays
- hex/data windows backed by source access and address-space mapping
- inspector modules driven by selected route capabilities
- accessibility list-mode fallbacks
- export surfaces that use stable graph facts, not screenshot scraping

Without that layer, Damsel can have a strong decoder and still feel unlike
Hopper. Users would click an instruction, lose context, wait for recompute,
open a graph that cannot map back to assembly, or see a pretty canvas that
cannot support xrefs, comments, export, keyboard navigation, or Apple runtime
objects.

## Public Source Set

Primary Hopper sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page and SDK listing: <https://www.hopperapp.com/download>
- Hopper public SDK archive:
  <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>

Hopper public SDK headers inspected locally from SDK 6.3.0:

- `HPBasicBlock.h`
- `HPProcedure.h`
- `HPDisassembledFile.h`
- `HPASMLine.h`
- `CPUContext.h`
- `CommonTypes.h`

Relevant public SDK evidence:

- `HPBasicBlock` exposes block range, procedure, index, predecessors,
  successors, and tags.
- `HPProcedure` exposes basic-block queries, stack facts, local labels, call
  graph, variables, signatures, calling conventions, and tags.
- `HPDisassembledFile` exposes byte types, type mutation, procedure creation,
  colors, tags, operand formats, problems, reading file bytes, assembler hooks,
  and undo/redo transactions.
- `HPASMLine` exposes attributed rendering pieces for mnemonics, registers,
  names, local names, variable names, comments, strings, CoreFoundation
  strings, Objective-C classes, selectors, protocols, addresses, numbers,
  operands, and hex bytes.
- `CPUContext` exposes procedure analysis, basic-block creation callbacks,
  branch analysis, switch detection, instruction-specific analysis, and
  rendering hooks.

Interpretation:

- Hopper's graph is backed by procedure/block facts, not by an isolated layout
  widget.
- Assembly lines are semantic rendered objects, not raw text.
- Graph, assembly, hex, inspector, and pseudocode are projections over one
  mutable document.
- Damsel needs a graph/representation service that can consume the same
  persistent facts as CLI, Workbench, automation, debugger, and export.

## Product Thesis

Damsel should make visual navigation a product advantage for Apple systems
work. Hopper is broad across formats and CPUs. Damsel can be sharper:

- cache-aware function graphs that show cross-image call/provider context
- ObjC selector graphs that connect classes, methods, IMPs, and callers
- C++/IOKit graphs for vtables, inheritance, and virtual dispatch candidates
- dyld dependency/provider/importer graphs
- kernel collection graphs for fileset members, kexts, syscalls, traps, and
  panic frames
- feature-map overlays for ObjC metadata, PAC/chained fixups, KDK symbols,
  dSYM source coverage, problems, bookmarks, search hits, and current route
- hex/data views that understand mapped, virtual, projected, unavailable, and
  sidecar-provided bytes
- graph and feature-map exports that produce evidence packages for Apple OS
  build archaeology

The first implementation should be conservative and useful:

- function CFG JSON
- simple graph layout
- route-backed graph node selection
- synchronized assembly and hex selection
- feature map for byte classifications and selected route
- list-mode graph fallback
- graph export to JSON/DOT
- performance metrics

Then grow into richer interactive graph operations.

## Non-Goals

Initial work should not try to implement:

- full decompiler graph rewriting
- arbitrary graph editing as source of truth
- animated decorative graph rendering
- whole-dyld-cache graph layout in one canvas
- GPU-only graph with no accessibility fallback
- graph screenshots as canonical export
- patching from hex view before patch transactions exist
- graph grouping that changes CFG semantics
- source-level stepping
- automatic proof that graph layout implies analysis correctness

Graph layout is presentation. Procedure, xref, byte, runtime, and debug-info
facts are truth.

## Current Damsel Baseline

Observed current repo facts:

- `damsel-core::DisassemblyAnalysis` records basic blocks and control-flow
  edges for decoded windows.
- `damsel-macho::disasm` already has jump-table, indirect-target, ObjC use,
  import use, cache-link, and recovered-value logic.
- `damsel-core::ObjcMetadata` already models classes, protocols, categories,
  methods, properties, ivars, selectors, and IMPs.
- `damsel-core::SharedCache` and `damsel-macho::SharedCacheSession` already
  support dyld shared-cache inventory, projection, symbolication, dependencies,
  providers, importers, and reexports.
- `damsel-cli` has JSON output and UI routes for current transient analysis.
- `damsel-app` has a native Workbench over the local CLI UI server.

Missing current capabilities:

- no persistent graph records
- no graph layout service
- no graph layout cache
- no graph route ids
- no Workbench graph canvas
- no graph list fallback
- no feature map generated from typed byte facts
- no hex/data pane
- no byte-window API tied to project address spaces
- no synchronized assembly/graph/hex/inspector selection model
- no graph export command
- no graph performance metrics
- no graph accessibility test
- no Apple runtime graphs in Workbench

## Architecture Overview

The subsystem should have six layers:

```text
Project facts
  |
  +-- procedure/block/xref/runtime/debug/type facts
  |
  +-- graph composition service
  |     +-- CFG graph
  |     +-- call graph
  |     +-- xref graph
  |     +-- ObjC selector/class graph
  |     +-- C++/IOKit graph
  |     +-- dyld/cache graph
  |     +-- kernel/panic graph
  |
  +-- presentation derivation
  |     +-- graph layout jobs
  |     +-- feature-map lanes
  |     +-- hex/data windows
  |     +-- semantic assembly rows
  |
  +-- route and selection sync
  |     +-- selected route
  |     +-- visible route ranges
  |     +-- cross-view scroll anchors
  |     +-- navigation history
  |
  +-- Workbench renderers
  |     +-- graph canvas
  |     +-- graph list fallback
  |     +-- feature map
  |     +-- hex/data pane
  |     +-- inspector modules
  |
  +-- export and automation
        +-- graph JSON
        +-- DOT
        +-- SVG/PDF later
        +-- MCP graph queries
        +-- evidence package embedding
```

Core rule:

Graph data, layout data, and view state are separate records. Selection is a
project route, not a row index, pixel coordinate, or decoded-line offset.

## Proposed Modules

Rust core:

- `damsel-core::graph`
- `damsel-core::feature_map`
- `damsel-core::hex_window`
- `damsel-core::representation_sync`
- `damsel-core::view_route`

Mach-O/analysis:

- `damsel-macho::graph_builder`
- `damsel-macho::feature_map_builder`
- `damsel-macho::hex_provider`
- `damsel-macho::semantic_row_builder`

CLI/API:

- `damsel-cli::graph_commands`
- `damsel-cli::feature_map_commands`
- `damsel-cli::hex_commands`
- `damsel-cli::representation_api`

Workbench:

- `GraphStore.swift`
- `GraphLayoutStore.swift`
- `FeatureMapStore.swift`
- `HexDataStore.swift`
- `RepresentationSyncStore.swift`
- `SelectionStore.swift`
- `GraphCanvasView.swift`
- `GraphListFallbackView.swift`
- `FeatureMapView.swift`
- `HexDataView.swift`

Project database:

- `graphs`
- `graph_nodes`
- `graph_edges`
- `graph_layouts`
- `graph_layout_jobs`
- `graph_view_states`
- `feature_map_lanes`
- `feature_map_ranges`
- `hex_windows`
- `selection_events`
- `representation_sync_snapshots`

## Data Model

### GraphId

Stable id for a graph projection.

Fields:

- `value`
- `project_id`
- `kind`
- `subject_route`
- `fact_revision`
- `graph_version`
- `created_by`

Rules:

- A graph id identifies the graph facts, not a layout.
- If facts change, create a new `graph_version` or mark the graph stale.
- Subject route can be a function, ObjC selector, class, cache image, kernel
  member, panic session, source file, or search result set.

### GraphKind

Enum:

- `FunctionCfg`
- `FunctionCallGraph`
- `ProcedureLocalXrefs`
- `ObjCClassHierarchy`
- `ObjCSelectorImplementers`
- `ObjCMessageSendCandidates`
- `CxxInheritance`
- `CxxVtable`
- `VirtualDispatchCandidates`
- `DyldDependency`
- `DyldProviderImporter`
- `SharedCacheImageGraph`
- `KernelCollectionMemberGraph`
- `KextDependency`
- `SyscallTrapGraph`
- `PanicFrameGraph`
- `DebugInlineCallGraph`
- `EvidenceDiffGraph`
- `CustomQueryGraph`

Rules:

- Start with `FunctionCfg`.
- Other graph kinds should share node/edge/query/layout contracts.
- Whole-cache graphs must support paging and expansion cursors.

### GraphRecord

Fields:

- `graph_id`
- `kind`
- `subject_route`
- `title`
- `description`
- `fact_revision`
- `node_count`
- `edge_count`
- `partial`
- `expansion_cursor`
- `created_at`
- `stale_reason`
- `problems`

Rules:

- `partial` must be explicit for paged/expanded graphs.
- Problems should include unresolved indirect branches, missing blocks,
  ambiguous xrefs, stale layout, and hidden large-subgraph truncation.
- Graph records should be deterministic for fixture tests.

### GraphNode

Fields:

- `node_id`
- `graph_id`
- `node_kind`
- `route`
- `label`
- `subtitle`
- `address_range`
- `source_range`
- `summary`
- `badges`
- `style_hints`
- `sort_key`
- `collapsed`
- `expandable`
- `problems`

`node_kind` enum:

- `BasicBlock`
- `Instruction`
- `Function`
- `Callsite`
- `ExternalCallTarget`
- `UnresolvedBranch`
- `SwitchCase`
- `ObjCClass`
- `ObjCMethod`
- `ObjCSelector`
- `CxxClass`
- `Vtable`
- `VtableSlot`
- `Kext`
- `KernelMember`
- `SharedCacheImage`
- `DyldLibrary`
- `SourceLine`
- `InlineCallsite`
- `PanicFrame`
- `Problem`
- `Group`

Rules:

- Every node should route to a project object when possible.
- Address-only nodes are allowed only for unresolved or synthetic nodes.
- Node labels should be generated from fact resolvers, not string scraping.
- Node styles are hints, not semantic truth.

### GraphEdge

Fields:

- `edge_id`
- `graph_id`
- `source_node_id`
- `target_node_id`
- `edge_kind`
- `route`
- `label`
- `confidence`
- `provenance`
- `style_hints`
- `conditional`
- `fallthrough`
- `backedge`
- `unresolved`
- `problems`

`edge_kind` enum:

- `ControlFlow`
- `Fallthrough`
- `ConditionalTrue`
- `ConditionalFalse`
- `SwitchCase`
- `Call`
- `TailCall`
- `PossibleCall`
- `Return`
- `DataReference`
- `ObjCMessage`
- `ObjCInheritance`
- `ObjCProtocolConformance`
- `CxxInheritance`
- `VirtualDispatch`
- `DyldDependency`
- `SymbolProvider`
- `SymbolImporter`
- `KextDependency`
- `InlineExpansion`
- `PanicFrameToFunction`
- `DiffMatch`
- `GroupContains`

Rules:

- Unresolved edges remain visible; hiding uncertainty makes graph UX worse.
- Conditional edges should carry branch polarity when known.
- CFG backedges should be marked for layout and decompiler hints.
- Edge provenance must identify the pass/fact source.

### GraphLayoutId

Fields:

- `layout_id`
- `graph_id`
- `layout_kind`
- `layout_version`
- `viewport_hint`
- `created_at`
- `created_by_job`
- `status`
- `metrics`

`layout_kind` enum:

- `LayeredCfg`
- `Tree`
- `ForceDirected`
- `AdjacencyList`
- `Matrix`
- `Timeline`
- `Manual`
- `Imported`

Rules:

- Layouts are derived caches.
- A graph can have several layouts.
- Layout can be stale even when graph facts are current.
- Layout failure should not block list-mode graph inspection.

### GraphLayoutNode

Fields:

- `layout_id`
- `node_id`
- `x`
- `y`
- `width`
- `height`
- `rank`
- `order`
- `visible`
- `collapsed`
- `anchor`

Rules:

- Units should be logical layout units, not device pixels.
- Workbench converts logical units to view pixels.
- Hidden/collapsed status is view/presentation state.

### GraphLayoutEdge

Fields:

- `layout_id`
- `edge_id`
- `points`
- `label_position`
- `routing_style`
- `visible`

Rules:

- Edge routing should be optional at first.
- Workbench can draw straight or stepped edges if route points are missing.
- Edge labels should be sparse; dense labels cause visual noise.

### GraphViewState

Fields:

- `view_state_id`
- `graph_id`
- `layout_id`
- `selected_route`
- `focused_node_id`
- `viewport_rect`
- `zoom`
- `visible_layers`
- `collapsed_groups`
- `hidden_edge_kinds`
- `pinned_nodes`
- `created_at`

Rules:

- View state belongs to Workbench/project session, not graph facts.
- It should persist across reopen where useful.
- It should not make CLI graph output nondeterministic.

### FeatureMap

A feature map is a compact, navigable representation of mapped address ranges.

Fields:

- `feature_map_id`
- `project_id`
- `subject_route`
- `address_space_id`
- `address_range`
- `scale`
- `lanes`
- `selected_route`
- `visible_window`
- `revision`

Rules:

- It is not only a section strip.
- It is generated from typed-byte, source, analysis, runtime, debug, problem,
  bookmark, and selection facts.
- It must support cache-projected and kernel fileset address spaces.

### FeatureMapLane

Lane kinds:

- `ByteType`
- `Procedure`
- `InstructionDensity`
- `Strings`
- `Data`
- `Undefined`
- `Symbols`
- `Imports`
- `Exports`
- `ObjCMetadata`
- `CxxMetadata`
- `SwiftNames`
- `Fixups`
- `PAC`
- `Xrefs`
- `DebugLineCoverage`
- `KDKCoverage`
- `Problems`
- `Bookmarks`
- `SearchHits`
- `CurrentSelection`
- `VisibleWindow`
- `PatchPreview`
- `DiffChanges`

Fields:

- `lane_id`
- `feature_map_id`
- `kind`
- `title`
- `priority`
- `ranges`
- `summary`
- `enabled_by_default`

Rules:

- Lanes must be pageable or compressed.
- Workbench should allow lane toggles.
- Lane colors should be stable and accessible.
- The current selection lane should always be visible.

### FeatureMapRange

Fields:

- `range_id`
- `lane_id`
- `address_range`
- `route`
- `kind`
- `confidence`
- `style_hint`
- `label`
- `summary`

Rules:

- Ranges can overlap across lanes.
- Ranges within a lane should be coalesced when possible.
- Clicking a range navigates by route, not raw address.

### HexWindow

Fields:

- `hex_window_id`
- `project_id`
- `subject_route`
- `address_space_id`
- `address_range`
- `bytes`
- `byte_states`
- `row_width`
- `base_address`
- `revision`
- `partial`
- `unavailable_ranges`

Rules:

- Hex windows fetch bounded byte ranges.
- A window can contain unavailable/virtual/projected bytes.
- Byte states include code/data/string/procedure/undefined/fixup/patched later.
- The hex view should never require loading an entire dyld cache into Swift.

### HexByteState

Fields:

- `address`
- `file_offset`
- `byte`
- `availability`
- `byte_type`
- `instruction_route`
- `data_route`
- `source_route`
- `selection_state`
- `problems`

`availability` enum:

- `Mapped`
- `Projected`
- `VirtualZeroFill`
- `Unavailable`
- `Redacted`
- `PatchedPreview`

Rules:

- Unavailable bytes should render explicitly.
- Projected bytes should show their source member.
- Instruction byte spans should highlight when an instruction is selected.

### RepresentationSelection

Fields:

- `selection_id`
- `project_id`
- `selected_route`
- `primary_address`
- `address_range`
- `function_route`
- `block_route`
- `instruction_route`
- `source_line_route`
- `origin_view`
- `reason`
- `timestamp`

`reason` enum:

- `UserClick`
- `KeyboardMove`
- `SearchJump`
- `XrefJump`
- `GraphNodeSelected`
- `HexByteSelected`
- `FeatureMapSelected`
- `DebuggerPcChanged`
- `PanicFrameSelected`
- `HistoryBack`
- `HistoryForward`
- `Programmatic`

Rules:

- Selection changes should be debounced for expensive inspector fetches.
- Selection loops must be prevented by origin/reason tracking.
- Every visible representation should update from the same selection event.

## Graph Composition

### Function CFG Graph

Inputs:

- procedure record
- basic block records
- control-flow edges
- jump-table records
- unresolved branch records
- local labels
- comments
- tags/colors/bookmarks
- problem records
- source line coverage
- IR/decompiler summaries later

Node strategy:

- one node per basic block
- optional synthetic entry node for clarity
- optional synthetic unresolved target node
- optional switch case group nodes
- collapsed group nodes for large switch/case clusters later

Edge strategy:

- fallthrough edges
- conditional true/false edges
- unconditional branch edges
- switch-case edges
- exceptional/control-unknown edges if modeled later
- unresolved indirect branch edges

Node labels:

- function-local block label
- start address
- short instruction preview
- comments if present
- problem badge
- source-line badge

Acceptance:

- graph JSON is deterministic
- graph node selection can locate assembly rows
- assembly selection can locate graph node
- unresolved control-flow remains visible
- large functions can fall back to list mode

### Call Graph

Inputs:

- function records
- call xrefs
- tail-call xrefs
- import uses
- ObjC message candidates
- virtual-call candidates
- cache-link uses

Node strategy:

- one node per function
- external import nodes
- unresolved indirect call nodes
- runtime method nodes for ObjC/C++ where useful

Rules:

- Do not collapse possible indirect call targets into exact calls.
- Cross-image calls include image/source identity.
- Call graph defaults should be local to selected function or image, not whole
  shared cache.

### Xref Graph

Inputs:

- selected route
- incoming xrefs
- outgoing xrefs
- data refs
- string refs
- metadata refs
- runtime refs

Graph shape:

- center selected object
- inbound nodes on one side
- outbound nodes on the other
- edge labels show reference kind
- paged expansion

Use cases:

- "who calls this?"
- "what does this selector reach?"
- "which functions read this string?"
- "which cache images import this symbol?"

### ObjC Graphs

Graph kinds:

- class hierarchy
- protocol conformance
- category extension graph
- selector implementer graph
- selector caller graph
- message-send candidate graph
- block invoke graph

Apple-specific requirements:

- class nodes link to ObjC metadata and implementation functions
- method nodes link to IMP procedure routes
- selector nodes link to callers and implementers
- cache-aware graphs can cross projected images
- confidence must be visible for message-send targets

### C++ And IOKit Graphs

Graph kinds:

- inheritance graph
- vtable slot graph
- virtual dispatch candidate graph
- constructor/destructor relationship graph
- IOKit class graph

Requirements:

- demangled names are labels, not proof alone
- vtable slot edges carry source/provenance
- multiple inheritance must not be flattened
- ambiguous virtual calls stay ambiguous

### Dyld And Shared Cache Graphs

Graph kinds:

- image dependency graph
- provider/importer graph
- reexport graph
- cache image neighborhood graph
- cross-image xref graph

Requirements:

- graphs are paged by default
- nodes identify root/subcache/member/projection
- provider/importer edges include symbol names
- image dependency graph can filter system libraries
- whole-cache rendering requires expansion/collapse and list fallback

### Kernel And Panic Graphs

Graph kinds:

- kernel collection member graph
- kext dependency graph
- syscall/trap graph
- panic frame graph
- panic frame to function/source graph
- IOKit class graph

Requirements:

- KDK/dSYM profile status appears in graph badges
- panic frames retain trust level and slide evidence
- kernel member address spaces are explicit
- missing symbolication appears as a problem node, not a blank graph

## Layout Service

### Principles

- Layout is derived.
- Layout is cancellable.
- Layout is cached.
- Layout failure does not break graph inspection.
- Large graphs degrade to list/matrix/adjacency views.
- User selection should not trigger full relayout.

### Layout Jobs

Job phases:

- `Queued`
- `LoadingGraph`
- `RankingNodes`
- `OrderingNodes`
- `RoutingEdges`
- `ComputingBounds`
- `CommittingLayout`
- `Completed`
- `CompletedPartial`
- `Failed`
- `Cancelled`

Job inputs:

- graph id
- layout kind
- viewport hint
- expansion scope
- hidden edge kinds
- collapsed groups
- priority

Job outputs:

- layout id
- layout node table
- layout edge table
- metrics
- problems

### First Layout Algorithm

For `FunctionCfg`, start with a deterministic layered layout:

1. Identify entry block.
2. Compute reachable block order.
3. Assign ranks by shortest distance from entry.
4. Mark backedges separately.
5. Keep fallthrough successors close.
6. Place switch cases in compact groups.
7. Route edges as simple polylines or straight lines.
8. Record unresolved targets on a side lane.

This is not the prettiest possible graph, but it is deterministic and testable.

### Layout Metrics

Collect:

- node count
- edge count
- layout duration
- edge crossing approximation
- viewport bounds
- cache hit/miss
- cancellation point
- memory used if available
- largest node label width
- hidden/truncated nodes

Metrics should feed:

- performance dashboard
- developer diagnostics
- regression tests
- Workbench warning thresholds

## Feature Map

### Purpose

The feature map is the user's file-wide or function-wide spatial memory. It
should answer:

- where am I?
- what kind of bytes surround me?
- where are functions, strings, data, metadata, problems, and search hits?
- how much of this image is analyzed?
- where do ObjC/C++/KDK/debug overlays exist?
- where is my visible window?

### Initial Feature Map Scope

Wave 1:

- selected image/function address range
- byte-type lane
- procedure lane
- symbols lane
- problems lane
- bookmarks lane
- search hits lane
- selected route/visible window lane

Wave 2:

- ObjC metadata lane
- strings lane
- xrefs lane
- fixups lane
- source-line coverage lane
- KDK/dSYM coverage lane

Wave 3:

- cache projection lane
- kernel member lane
- diff lane
- patch preview lane
- decompiler confidence lane

### Compression

Feature map range compression:

- coalesce adjacent ranges of same kind/confidence
- keep problem/search/bookmark ranges precise
- preserve selected range exactly
- use tile summaries for very large images
- fetch details only near cursor/viewport

### Interaction

User actions:

- click range to navigate
- drag visible window to scroll
- hover for tooltip summary
- toggle lanes
- jump to next problem/bookmark/search hit
- context menu for range actions later

Rules:

- Clicks resolve to routes.
- Dragging must not block on analysis.
- Hover tooltips should use cached summaries.

## Hex/Data View

### Purpose

Hopper-level hex view is not a dump. It synchronizes with analysis:

- selected instruction highlights bytes
- bytes carry type/classification
- data interpretations link to routes
- unavailable/projected bytes are explicit
- patch preview later builds on the same window model

### Initial Hex View

Features:

- bounded byte window by route/address
- address column
- hex bytes
- ASCII column
- byte classification colors
- selected instruction span
- selected data range
- mapped/projected/unavailable state
- jump-to-address
- copy selected bytes
- open selection in assembly

Apple-specific overlays:

- fixup marker
- PAC-authenticated pointer marker
- ObjC metadata pointer marker
- C string/CFString marker
- dyld binding/stub marker
- KDK/dSYM line coverage marker
- cache projection source marker

### Byte Interpretation Panel

For selected bytes:

- unsigned/signed integer values
- pointer interpretation
- resolved symbol/name if pointer-like
- endian-aware views
- string preview
- section/segment context
- fixup/binding context
- source member for projected bytes
- problems

This panel should read from existing typed models and future type facts. It
must not invent confidence.

### Hex Performance

Rules:

- fetch small windows
- prefetch adjacent windows opportunistically
- never materialize whole dyld cache in Swift
- keep row width stable
- selection does not trigger full refetch if bytes are cached
- patch preview later uses copy-on-write source revisions

## Representation Synchronization

### Canonical Route

Every view observes one canonical selected route:

- assembly row route
- function route
- block route
- instruction route
- graph node route
- hex byte range route
- feature map range route
- xref route
- source line route
- runtime object route
- problem route

The selected route owns:

- current address
- containing function
- containing block
- source line if active
- visible byte range
- inspector module set
- graph focus

### Sync Rules

Assembly -> Graph:

- select instruction
- resolve containing block
- focus graph node
- do not relayout
- scroll node into view if graph is active

Graph -> Assembly:

- select node
- resolve route to block/instruction range
- scroll assembly to first instruction
- select block range or first instruction depending on mode

Assembly -> Hex:

- select instruction
- fetch containing byte window if missing
- highlight byte span

Hex -> Assembly:

- select byte
- resolve containing instruction/data route
- scroll assembly if instruction exists
- show byte-only route if not code

Feature Map -> All Views:

- click range
- resolve route
- update selected route
- visible views update through selection event

Debugger/Panic -> Graph:

- runtime PC or panic frame resolves to static route
- graph opens selected function
- node focus uses route mapping
- unresolved runtime-only state appears as problem/overlay

### Loop Prevention

Every selection event includes:

- origin view
- reason
- route revision
- sequence number

Views apply selection but do not echo the same sequence as a new event.

### Missing Object Behavior

If a route cannot resolve:

- keep the route in navigation history
- show stale/missing problem
- offer nearest valid address
- do not silently jump elsewhere
- do not clear user context

## Workbench UX

### Graph Canvas

Canvas capabilities:

- pan
- zoom
- fit graph
- center selection
- select node
- select edge
- keyboard move to predecessor/successor
- show minimap
- show visible viewport in minimap
- toggle edge kinds
- toggle unresolved nodes
- open selected node in assembly/hex
- export graph

Visual design:

- restrained colors
- node labels fit
- badges instead of verbose text where possible
- selected route highly visible
- problem badges visible but not alarming for expected uncertainty
- edge confidence visible for ambiguous edges
- current PC/panic frame overlays later

### Graph List Fallback

Required for:

- accessibility
- huge graphs
- layout failure
- low-power mode
- tests

List rows:

- node label
- route
- address/range
- predecessor count
- successor count
- problems
- badges

Actions:

- reveal in assembly
- reveal in hex
- expand neighbors
- copy route
- export selected subgraph

### Graph Minimap

The minimap is a simplified layout:

- node rectangles only
- selected node
- viewport rectangle
- problem density markers
- no labels

Rules:

- minimap updates from layout cache
- minimap does not force edge routing
- minimap remains useful for medium functions

### Feature Map View

Placement:

- top or bottom of representation area
- optionally vertical side strip for long views
- visible in assembly/graph/hex modes

Controls:

- lane toggles
- fit to file/function
- jump to selection
- next problem
- next bookmark
- next search hit

### Hex/Data Pane

Modes:

- split below assembly
- side pane
- full representation tab

Commands:

- jump to address
- copy bytes
- copy address
- interpret pointer
- reveal in feature map
- reveal in graph if code
- later patch preview

### Inspector Integration

Graph selection inspector:

- graph kind
- node/edge route
- address range
- predecessors/successors
- xrefs
- comments/tags/colors
- source/debug coverage
- problems
- export actions

Hex selection inspector:

- address range
- byte state
- section/segment/source context
- data interpretations
- xrefs/fixups
- problems

Feature map selection inspector:

- lane
- range
- route
- summary
- coverage/provenance

## API Contracts

### Get Graph

```text
GET /api/projects/{project_id}/graphs/{graph_id}
```

Query options:

- `include_layout`
- `layout_id`
- `node_cursor`
- `edge_cursor`
- `max_nodes`
- `max_edges`
- `visible_layers`
- `route_context`

Response:

- graph record
- nodes page
- edges page
- layout summary
- problems
- cursors
- fact revision

### Compose Graph

```text
POST /api/projects/{project_id}/graphs/compose
```

Request:

- `kind`
- `subject_route`
- `depth`
- `filters`
- `max_nodes`
- `max_edges`

Response:

- graph id
- existing or new graph record
- partial status
- layout availability

### Request Layout

```text
POST /api/projects/{project_id}/graphs/{graph_id}/layout
```

Request:

- `layout_kind`
- `viewport_hint`
- `hidden_edge_kinds`
- `collapsed_groups`
- `priority`

Response:

- job id
- cached layout id if available
- estimated cost

### Get Feature Map

```text
GET /api/projects/{project_id}/feature-map
```

Query:

- `subject_route`
- `address_space_id`
- `range`
- `lanes`
- `scale`
- `selected_route`

Response:

- feature map id
- lanes
- compressed ranges
- visible window
- revision
- problems

### Get Hex Window

```text
GET /api/projects/{project_id}/hex-window
```

Query:

- `route`
- `address_space_id`
- `start`
- `length`
- `row_width`
- `include_states`

Response:

- address range
- rows
- byte states
- unavailable ranges
- source context
- revision

### Resolve Selection

```text
POST /api/projects/{project_id}/selection/resolve
```

Request:

- `route`
- `origin_view`
- `reason`
- `visible_context`

Response:

- canonical route
- containing function
- containing block
- instruction range
- hex range
- graph focus
- feature map focus
- inspector modules
- warnings

## CLI Contracts

Initial commands:

```sh
damsel project graph compose <project> --kind function-cfg --route <route> --json
damsel project graph get <project> <graph-id> --json
damsel project graph layout <project> <graph-id> --json
damsel project graph export <project> <graph-id> --format dot
damsel project feature-map <project> --route <route> --json
damsel project hex-window <project> --route <route> --length 256 --json
damsel project selection resolve <project> --route <route> --json
```

Transitional commands can operate on standalone fixture files, but project mode
is the target architecture.

## Export

### JSON Export

Graph JSON should include:

- graph record
- nodes
- edges
- layout id if included
- provenance summary
- problems
- fact revision

### DOT Export

DOT export should:

- escape labels safely
- include route ids as attributes
- include edge kinds
- include problem/confidence badges where possible
- avoid private paths by default

### PDF/SVG Later

PDF/SVG export should be generated from graph facts and layout records, not
from a Workbench screenshot. That keeps export deterministic and usable from
CLI and automation.

### Evidence Package Integration

Evidence packages should be able to include:

- selected graph JSON
- graph image export later
- feature map lane summaries
- selected hex windows
- route ids
- active symbolication profile
- redaction policy

## Performance Budgets

Warm project target budgets:

- selection resolve: under 50 ms for cached routes
- graph metadata fetch: under 100 ms
- first CFG graph JSON for small function: under 100 ms
- cached graph layout fetch: under 50 ms
- graph canvas first paint from cached layout: under 150 ms
- feature map fetch for loaded image: under 100 ms
- hex window fetch: under 75 ms
- selection to inspector update: under 100 ms

Cold or large graph work:

- return job id quickly
- show partial graph/list fallback
- progress events every meaningful phase
- cancellation within bounded time

Large graph thresholds:

- over 300 nodes: warn and list-mode fallback available
- over 1000 nodes: require expansion/paging by default
- whole-cache graph: never layout all nodes by default

These numbers are starting budgets. They should become measured gates when the
implementation exists.

## Storage And Caching

Cache keys:

- project id
- graph kind
- subject route
- fact revision
- filters
- layout kind
- layout options

Invalidation triggers:

- function CFG changed
- xref index changed
- byte map changed
- annotation/comment/tag/color changed
- symbolication profile changed
- runtime metadata changed
- graph filters changed
- user collapsed/expanded groups

Persistence:

- graph facts can be rebuilt from project facts
- expensive layouts can be cached
- user view state should persist in session/project UI state
- exported graph snapshots should include fact revision

## Accessibility

Accessibility is required, not optional.

Requirements:

- every graph has list fallback
- keyboard can move between nodes
- node/edge labels are accessible
- minimap is not the only navigation path
- color is not the only meaning channel
- feature map ranges have text descriptions
- hex bytes can be navigated by keyboard
- selection state is announced in accessible labels where platform allows
- large graph warning is accessible

Tests:

- list fallback renders for graph fixture
- keyboard predecessor/successor movement works
- selection sync works without mouse
- high-contrast color semantics remain distinguishable

## Implementation Phases

### G0: Current-State Audit

Deliverables:

- inspect current `DisassemblyAnalysis` block/edge coverage
- map current UI selection path
- identify current byte-window access points
- confirm current API payload shapes
- choose first graph fixture

Validation:

- repo-grounded audit note or issue

### G1: Core Graph Models

Deliverables:

- add `GraphRecord`, `GraphNode`, `GraphEdge`
- add route references
- add deterministic serialization
- add fixture snapshots

Validation:

- small branch fixture graph JSON
- switch fixture graph JSON
- unresolved indirect fixture graph JSON

### G2: CFG Graph Builder

Deliverables:

- build graph from function/procedure records
- include blocks and control-flow edges
- include unresolved branch nodes
- include jump-table edges
- include comments/tags/problem badges when available

Validation:

- `semantic-switch` CFG fixture
- branch-heavy fixture
- indirect target fixture

### G3: Graph API And CLI

Deliverables:

- compose graph endpoint
- get graph endpoint
- CLI graph JSON
- CLI DOT export
- paged node/edge response shape

Validation:

- CLI JSON snapshots
- DOT snapshot smoke
- API route tests

### G4: Layout Service

Deliverables:

- deterministic layered CFG layout
- layout job model
- layout cache
- cancellation hook
- metrics

Validation:

- layout snapshot with stable coordinates or normalized ranks
- large graph cancellation fixture
- cache hit/miss fixture

### G5: Workbench Graph View

Deliverables:

- GraphStore
- GraphCanvasView
- GraphListFallbackView
- selection focus
- minimap skeleton
- graph toolbar

Validation:

- Swift store tests
- graph fixture render smoke
- list fallback test
- selection sync test

### G6: Feature Map

Deliverables:

- feature map model
- byte-type/procedure/problem/selection lanes
- feature map endpoint
- Workbench FeatureMapView
- click-to-route

Validation:

- range compression fixture
- click selection test
- visible-window update test

### G7: Hex/Data View

Deliverables:

- bounded hex-window endpoint
- byte states
- instruction byte span selection
- Workbench HexDataView
- byte interpretation inspector

Validation:

- byte-window fixture
- assembly-to-hex sync test
- hex-to-assembly sync test
- unavailable/projected byte fixture

### G8: Representation Sync Store

Deliverables:

- canonical selection event model
- origin/reason loop prevention
- route resolve API integration
- assembly/graph/hex/feature-map/inspector sync

Validation:

- no selection loop test
- route history test
- stale route resolution test

### G9: Apple Runtime Graphs

Deliverables:

- ObjC selector implementer graph
- cache provider/importer graph
- C++ vtable graph skeleton
- kernel/kext graph skeleton

Validation:

- ObjC fixture graph
- synthetic cache provider graph
- C++ vtable fixture graph
- synthetic kernel member graph

### G10: Export And Evidence

Deliverables:

- graph JSON export
- DOT export
- feature-map summary export
- selected hex-window export
- evidence package integration

Validation:

- export snapshots
- privacy/redaction fixtures
- deterministic rerun check

### G11: Performance And Accessibility Hardening

Deliverables:

- metrics panels/logs
- large graph fallback
- keyboard graph navigation
- high-contrast lane palette
- responsiveness probes

Validation:

- graph first-paint probe
- feature map fetch probe
- hex window fetch probe
- accessibility fallback test

## Test Matrix

| Requirement | Fixture evidence | UI evidence | Performance evidence |
| --- | --- | --- | --- |
| CFG graph JSON | branch/switch/indirect fixtures | graph list fallback | graph fetch metric |
| Layout cache | layout snapshot | canvas first paint | cache hit/miss metric |
| Assembly/graph sync | block route fixture | selection sync test | no relayout on select |
| Assembly/hex sync | byte span fixture | hex highlight test | bounded fetch metric |
| Feature map | byte/procedure/problem ranges | click-to-route test | compressed response size |
| ObjC graph | ObjC method fixture | selector graph smoke | paged response |
| Cache graph | synthetic provider/importer | cache graph smoke | expansion cursor |
| Kernel graph | synthetic fileset/kext | kernel graph list fallback | no whole-cache layout |
| Export | JSON/DOT snapshots | export command smoke | deterministic rerun |
| Accessibility | graph list fixture | keyboard movement test | no canvas dependency |

## Failure Modes

High-risk failures:

- graph layout becomes source of truth
- graph node ids are unstable
- selection loops between views
- graph recomputes on every selection
- Workbench freezes during layout
- whole-cache graph tries to render all nodes
- feature map scans entire project per mouse move
- hex view loads full files into Swift
- graph hides unresolved branches
- ambiguous call targets look exact
- accessibility depends on canvas only
- export captures pixels instead of facts

Mitigations:

- graph facts separate from layout
- route-backed nodes and edges
- selection event sequence ids
- layout jobs and caches
- paged graph responses
- list fallback
- compressed feature-map lanes
- bounded hex windows
- explicit confidence/provenance
- export from graph facts

## Completion Criteria

This subsystem is not complete when a canvas draws boxes. Minimum credible
completion:

- project graph API exists for function CFGs
- graph nodes and edges use stable route ids
- graph layout is derived and cached
- Workbench can show graph canvas and list fallback
- assembly and graph selection synchronize without loops
- feature map shows byte/procedure/problem/selection lanes
- feature-map clicks navigate by route
- hex/data view fetches bounded byte windows
- assembly and hex selection synchronize
- graph export uses stable facts
- accessibility fallback is tested
- large graph fallback is implemented
- performance metrics exist for graph fetch/layout/render and hex/feature map

## Open Decisions

- Use server-side layout only, or allow Workbench-side layout for small graphs?
- Which layout engine should be used after the deterministic first layout?
- How much graph view state belongs in project persistence versus user
  preferences?
- Should graph node comments/colors share the annotation store immediately?
- How should pseudocode construct selection map back to graph blocks?
- What threshold switches from canvas to list-mode fallback?
- Should feature-map lanes be user-configurable per project?
- Should DOT export be the first official graph export format?
- How should graph exports represent private source/debug paths?

## Summary

Graph view, feature map, hex/data view, and selection synchronization are a
single product architecture problem. Hopper's public surface makes this clear:
assembly, graph, pseudocode, hex, inspector, byte typing, comments, tags,
colors, xrefs, and procedures all reflect one mutable document.

Damsel's version should preserve that lesson while leaning into its Apple
systems niche. A route-backed graph/feature-map/hex subsystem can make dyld
cache, ObjC, C++, kernel, KDK, panic, and debug-info workflows navigable in a
way a generic disassembler does not specialize for. The implementation must
start with deterministic facts, simple layouts, bounded windows, and strong
sync tests, then grow into richer visual navigation without sacrificing
correctness or responsiveness.
