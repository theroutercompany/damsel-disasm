# Semantic Workbench Surfaces Implementation Spec

Status: implementation architecture spec. This is not implemented yet.

Purpose: define how Damsel should turn the project kernel and typed byte map
into Hopper-competitive user-facing surfaces: semantic assembly, graph, hex,
feature map, contextual inspector, project navigators, search, route history,
and Workbench state. This document sits above the project kernel spec and below
final app polish. It answers: once Damsel has object ids, facts, routes, and
byte classifications, how should the app actually use them?

Primary conclusion: Workbench should stop treating disassembly as a transient
table response and start treating every visible row, graph node, inspector
section, search result, feature-map segment, and xref as a routeable project
object with semantic spans, lazy details, provenance, and latency budgets.

Dedicated representation companion:
[Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
expands this document's graph/hex/feature-map outline into concrete graph
records, layout jobs, feature-map lanes, hex windows, canonical selection
events, Workbench stores, APIs, export contracts, performance budgets, and
validation gates.

## Why This Spec Exists

Hopper's public workflow feels coherent because its views are synchronized over
one document:

- the assembly listing knows addresses, bytes, names, operands, comments, and
  branch cues
- the graph knows basic blocks, labels, colors, groupings, comments, and current
  selection
- the hex view knows which bytes belong to the current instruction
- the inspector changes with the selected object
- search, symbols, strings, tags, xrefs, and navigation all land on the same
  route stack
- scripting and MCP can operate on the document model rather than on isolated
  UI strings

Damsel currently has a good alpha surface, but the current state is still
response-oriented:

- `DamselAPIClient.uploadImage` reads the whole file into `Data` and sends it to
  `/api/images`.
- `WorkbenchStore` keeps `loadedImage`, `disassembly`, target controls, symbol
  suggestions, and `selectedInstructionAddress`.
- `InstructionRow` has `address`, `opcode`, `mnemonic`, `rendered`, optional
  operand strings, JSON structured operands, references, annotations, and
  recovered values.
- `InstructionSurfaceView` displays a filtered instruction list.
- `InspectorPane` inspects the selected instruction and its local references.
- `jump(to:)` changes the target kind to address and reruns disassembly.

That model can show a useful window. It cannot become a Hopper-class workbench
without a semantic surface contract.

## Scope

In scope:

- semantic assembly rows and spans
- route-backed selection
- lazy contextual inspector modules
- graph view model and graph presentation state
- read-only hex/data view contract
- feature map contract
- project navigators and search results
- Workbench store migration
- local UI server endpoints
- Swift model additions
- latency and responsiveness requirements
- tests and acceptance gates

Out of scope for this first surface layer:

- complete decompiler UI
- live debugger UI
- binary patch editing
- embedded script editor
- plugin manager
- full multi-document collaboration
- final visual design polish

This spec still reserves the route/object contracts those later features need.

## Design Principles

### One Object Model

Every visible thing should have a project object id when possible.

Examples:

- instruction row
- operand span
- function header
- basic block
- graph edge
- string
- selector
- ObjC method
- C++ vtable
- chained fixup
- problem
- bookmark
- search result

If a visible thing cannot yet resolve to a stable object id, it should carry an
unresolved route with the reason.

### One Route Stack

Every cross-object jump should go through a shared route model.

Push route:

- search result open
- xref jump
- call target jump
- branch target jump outside current local browse mode
- ObjC method to IMP
- selector to implementers/callers
- cache image/provider/importer jump
- problem to source
- bookmark open

Replace route:

- local row selection within already-open function
- expanding inspector modules
- changing filter text
- changing graph zoom/pan unless user explicitly opens a new target

### Semantic Rows, Not Rendered Strings

The `rendered` field can remain for compatibility, but Workbench should move to
semantic spans.

Why:

- operand click targets need exact span identity
- comments and tags need precise placement
- xref badges need stable object ids
- accessible labels need token roles
- search highlights should not parse raw text
- future pseudocode and script APIs need the same operand semantics

### Lazy Detail, Fast First Paint

The main surface should paint quickly with summary data. Heavy details should be
lazy:

- xref lists
- raw fact lists
- large ObjC method lists
- graph layout
- decompiler-lite summaries
- cache-wide provider/importer lists
- C++ type hierarchies
- kernel symbolication details

Hopper's changelog repeatedly shows large-binary responsiveness and memory
footprint are active product concerns. Damsel should make laziness a default
architecture rule, not a late optimization.

### Project Facts Drive UI

Do not invent Swift-only derived state for facts that Rust already owns.

Swift can cache view models, but canonical facts should live in the project
kernel:

- byte classification
- xrefs
- object ids
- names
- comments
- tags
- problems
- provenance
- routes
- analysis state

## Target Workbench State Model

### Current State To Preserve During Migration

Do not break these immediately:

- `serverState`
- current server bootstrap behavior
- `loadedImage`
- `disassembly`
- `symbolSuggestions`
- `selectedInstructionAddress`
- current disassembly request controls
- current image upload/disassembly routes

They should coexist with project-backed state during migration.

### New Store State

Add:

```swift
@Published public private(set) var loadedProject: ProjectSummary?
@Published public private(set) var projectSources: [ProjectSource] = []
@Published public private(set) var currentRoute: WorkbenchRoute?
@Published public private(set) var navigationStack: NavigationStackModel = .empty
@Published public private(set) var currentSurface: WorkbenchSurface = .assembly
@Published public private(set) var assemblyWindow: AssemblyWindow?
@Published public private(set) var inspectorModules: [InspectorModuleModel] = []
@Published public private(set) var featureMap: FeatureMapModel?
@Published public private(set) var projectProblems: ProblemListModel?
@Published public private(set) var activeJobs: [AnalysisJobModel] = []
```

Add user-controlled state:

```swift
@Published public var globalSearchText = ""
@Published public var selectedNavigator: NavigatorKind = .project
@Published public var selectedRepresentation: RepresentationKind = .assembly
@Published public var inspectorExpandedModuleIDs: Set<String> = []
@Published public var graphZoom: Double = 1.0
@Published public var hexColumns: Int = 16
```

Rules:

- `currentRoute` replaces address-only selection as the primary navigation
  target.
- `selectedInstructionAddress` can remain as compatibility derived state until
  assembly rows are route-backed.
- `assemblyWindow` should be paged by route/window, not stored as the full
  project instruction set.
- `inspectorModules` should be requested for the selected route and module set.

## Project API Endpoints For Surfaces

These are local UI server contracts. Names can evolve, but the shape should
stay route/object/fact oriented.

### Project Open And Summary

```text
POST /api/projects
GET  /api/projects/{project_id}
GET  /api/projects/{project_id}/sources
GET  /api/projects/{project_id}/jobs
GET  /api/projects/{project_id}/problems
```

Used by:

- welcome window
- project overview
- toolbar status
- source navigator
- problems utility pane

### Route Resolution

```text
POST /api/projects/{project_id}/routes/resolve
POST /api/projects/{project_id}/routes/back
POST /api/projects/{project_id}/routes/forward
```

Route resolve request:

```json
{
  "route": {
    "kind": "address",
    "address": {
      "space": "aspace:primary:image-vmaddr",
      "value": "0x100003f10"
    },
    "preferredRepresentation": "assembly"
  },
  "origin": "typed_address"
}
```

Route resolve response:

```json
{
  "route": {
    "kind": "instruction",
    "objectId": "object:instruction:...",
    "preferredRepresentation": "assembly"
  },
  "title": "0x100003f10",
  "subtitle": "__TEXT,__text",
  "selection": {
    "kind": "instruction",
    "objectId": "object:instruction:..."
  },
  "availableRepresentations": ["assembly", "hex", "inspector"],
  "warnings": []
}
```

Rules:

- route resolution can return unresolved or stale states
- unresolved routes should show recovery suggestions
- route resolution must not silently rerun a broad disassembly unless requested

### Assembly Window

```text
POST /api/projects/{project_id}/assembly/window
```

Request:

```json
{
  "route": { "kind": "function", "objectId": "object:function:..." },
  "window": {
    "beforeRows": 80,
    "afterRows": 240,
    "maxRows": 500
  },
  "options": {
    "includeBytes": true,
    "includeOpcodes": true,
    "includeXrefBadges": true,
    "includeInlineComments": true,
    "includeProvenance": false
  }
}
```

Response:

```json
{
  "route": { "kind": "function", "objectId": "object:function:..." },
  "rows": [],
  "nextCursor": "cursor:...",
  "previousCursor": "cursor:...",
  "summary": {
    "rowCount": 240,
    "functionName": "_main",
    "startAddress": "0x100003f10",
    "endAddress": "0x100004020"
  }
}
```

Rules:

- rows are ordered by address and local presentation order
- large functions must be pageable
- current row should be centered when possible
- local filtering should use server-side semantic fields for large windows

### Inspector Modules

```text
POST /api/projects/{project_id}/inspector/modules
GET  /api/projects/{project_id}/inspector/modules/{module_id}
```

The first endpoint returns module headers and cheap summaries. The second loads
expanded detail.

Module header response:

```json
{
  "route": { "kind": "instruction", "objectId": "object:instruction:..." },
  "modules": [
    {
      "id": "instruction.overview",
      "title": "Instruction",
      "state": "ready",
      "summary": "adrp x0, ...",
      "badgeCount": null
    },
    {
      "id": "xrefs.incoming",
      "title": "Reverse References",
      "state": "collapsed",
      "summary": "12 refs",
      "badgeCount": 12
    }
  ]
}
```

Rules:

- inspector module details are lazy
- module ids are stable
- module output should include commands available for that object
- modules can be hidden when not applicable, but the reason should be testable

### Graph View

```text
POST /api/projects/{project_id}/graph/function
POST /api/projects/{project_id}/graph/layout
POST /api/projects/{project_id}/graph/presentation
```

Graph function response:

```json
{
  "function": { "objectId": "object:function:..." },
  "nodes": [],
  "edges": [],
  "summary": {
    "nodeCount": 14,
    "edgeCount": 18,
    "entryCount": 1,
    "exitCount": 2
  },
  "presentation": {
    "layoutVersion": "none",
    "userGroups": [],
    "colors": []
  }
}
```

Rules:

- CFG facts and presentation facts are separate
- graph layout can be cached and invalidated
- graph layout can be a job for large functions
- user node colors, grouping, and labels are annotation/presentation facts

### Hex And Feature Map

```text
POST /api/projects/{project_id}/hex/window
POST /api/projects/{project_id}/feature-map
```

Hex window request:

```json
{
  "route": { "kind": "instruction", "objectId": "object:instruction:..." },
  "bytesBefore": 128,
  "bytesAfter": 512,
  "columns": 16
}
```

Feature map request:

```json
{
  "sourceId": "source:primary",
  "addressSpaceId": "aspace:primary:image-vmaddr",
  "tileCount": 512,
  "layers": ["base", "objc", "fixups", "problems"]
}
```

Rules:

- hex selection should highlight current instruction bytes
- feature map must come from byte classifications, not section names alone
- feature map tiles should include route targets
- problem overlays should be visible

### Search And Navigators

```text
GET  /api/projects/{project_id}/search
GET  /api/projects/{project_id}/navigator/{kind}
```

Search query parameters:

```text
q
scope
source_id
limit
cursor
include_kinds
exclude_kinds
```

Navigator kinds:

- project
- sources
- functions
- symbols
- strings
- imports
- exports
- ObjC
- C++
- cache
- kernel
- problems
- tags
- bookmarks

Rules:

- navigators are paged
- navigator rows include route
- filtering large lists happens server-side
- search result ranking is deterministic enough to test

## Semantic Assembly Row Model

### Row Types

Assembly rows should not only represent instructions.

Row kinds:

- function header
- basic block header
- label
- instruction
- data directive
- string directive
- comment
- problem
- padding
- analysis note
- collapsed range

### Instruction Row Fields

Suggested model:

```json
{
  "id": "object:instruction:...",
  "kind": "instruction",
  "route": { "kind": "instruction", "objectId": "object:instruction:..." },
  "address": {
    "space": "aspace:primary:image-vmaddr",
    "value": "0x100003f10"
  },
  "fileRange": {
    "space": "aspace:primary:file-offset",
    "start": "0x3f10",
    "end": "0x3f14"
  },
  "bytes": "ff4300d1",
  "mnemonic": "sub",
  "spans": [],
  "references": [],
  "badges": [],
  "annotations": [],
  "problems": []
}
```

### Span Roles

Required span roles:

- address
- bytes
- mnemonic
- register
- immediate
- memory
- symbol
- local_label
- global_name
- function_name
- import_name
- export_name
- string_literal
- cfstring
- objc_class
- objc_selector
- objc_protocol
- objc_method
- cxx_type
- cxx_vtable
- kernel_symbol
- cache_image
- branch_target
- call_target
- data_target
- comment
- inline_comment
- tag
- problem
- punctuation

Span fields:

- text
- role
- route
- object id
- provenance
- confidence
- selectable
- copy value
- tooltip summary

### Compatibility With Existing `InstructionRow`

Keep existing fields initially:

- `address`
- `opcode`
- `mnemonic`
- `rendered`
- `operands`
- `structuredOperands`
- `references`
- `annotations`
- `recoveredValues`

Add new project-backed model:

- `AssemblyRow`
- `AssemblySpan`
- `AssemblyBadge`
- `AssemblyReference`
- `AssemblyProblem`

Migration:

1. Add Swift decode models for `AssemblyWindow`.
2. Add endpoint returning semantic rows.
3. Build a new `SemanticInstructionTable` beside `VirtualizedInstructionTable`.
4. Keep old table until project-backed row rendering is stable.
5. Use old `rendered` fallback for rows without spans.

## Inspector Module System

### Module Registry

Inspector modules should be registered by object kind.

Object kind to modules:

- instruction:
  instruction overview, operands, bytes, references, reverse references,
  recovered values, comments/tags, provenance, problems
- operand:
  operand value, format override, references, runtime overlay, provenance
- function:
  overview, callers, callees, blocks, stack frame, signatures, ObjC/C++ links,
  strings, problems, comments/tags, provenance
- basic block:
  overview, predecessors, successors, instructions, problems, presentation
- ObjC method:
  owner, selector, IMP route, callers, message-send candidates, type encoding,
  cache image, comments
- ObjC class:
  methods, properties, ivars, protocols, categories, subclasses where known,
  cache image/source
- C++ vtable:
  slots, methods, RTTI, constructors/destructors, callsites, symbolication
- chained fixup:
  raw value, resolved target, auth/PAC details, binding source, byte range
- cache image:
  install name, dependencies, dependents, providers, importers, exports,
  ObjC classes, Swift symbols
- problem:
  severity, reason, target, suggested hints, related facts, suppress command

### Module States

States:

- unavailable
- collapsed
- loading
- ready
- partial
- stale
- failed

Rules:

- collapsed modules show cheap summary only
- expanded modules can make extra API calls
- failed modules show structured errors
- stale modules show analyzer/version reason

### Commands In Modules

Initial commands:

- copy address
- copy symbol/name
- jump to target
- reveal in navigator
- add comment
- add tag
- bookmark
- set name
- add manual reference
- report problem note

Later commands:

- make code/data/string/procedure
- set operand format
- rename local label
- rename stack variable
- apply switch hint
- patch preview
- set breakpoint

All commands should become annotation transactions or explicit analysis jobs.

## Graph Surface

### Graph Facts

Graph facts come from project function and block records:

- function object id
- block object ids
- block address ranges
- block labels
- block instruction counts
- predecessor/successor edges
- call edges
- problem overlays
- entry/exit flags
- loop/back-edge flags
- unresolved indirect flags

### Graph Presentation Facts

Presentation facts are separate:

- node position
- node color
- node custom title
- group id
- collapsed state
- viewport
- zoom
- layout engine version

### Graph UI Requirements

Minimum:

- show current function CFG
- select block and sync inspector
- click edge target and route
- show minimap
- keyboard move between blocks
- preserve selection when switching assembly/graph
- lazy layout for large functions

P1:

- node colors
- comments from graph
- labels from graph
- group/collapse nodes
- graph export

P2:

- side-by-side graph and assembly
- decompiler block overlay
- path exploration mode
- debugger current-PC overlay

## Hex And Data Surface

### Read-Only First

Initial hex/data view is read-only.

Required:

- route to bytes around current object
- highlight current instruction range
- highlight selected byte range
- show base classification and overlays
- show file offset and VM address
- show source/image/cache context
- show ASCII/UTF-8 side text when safe
- show unknown/unmapped bytes explicitly

No initial:

- byte editing
- NOP region
- assembler
- patch export

But the UI model must reserve:

- patch preview commands
- transaction id
- invalidation summary
- export target

### Data Interpretations

Inspector or side panel should offer:

- u8/u16/u32/u64
- signed variants
- pointer target
- C string
- UTF-16 string
- ObjC selector
- ObjC class pointer
- chained fixup target
- PAC-stripped target where known

Do not mutate byte classification merely by viewing an interpretation. Mutation
requires a command.

## Feature Map

### Purpose

The feature map is Hopper's navigation bar idea adapted to Damsel's Apple
systems niche.

It should show:

- code
- procedure
- strings
- data
- undefined/unknown
- ObjC metadata
- Swift metadata
- C++ metadata
- chained fixups
- stubs
- cache image boundaries
- kernel/fileset/kext boundaries
- problems
- bookmarks
- current viewport
- current route

### Implementation

The Rust API returns tiles:

```json
{
  "sourceId": "source:primary",
  "addressSpaceId": "aspace:primary:image-vmaddr",
  "tiles": [
    {
      "start": "0x100000000",
      "end": "0x100001000",
      "dominantKind": "code",
      "overlays": ["objc_selector_ref"],
      "problemCount": 0,
      "route": { "kind": "range", "objectId": "object:range:..." }
    }
  ]
}
```

Rules:

- feature map should be cheap to refresh
- tile count should be bounded
- detail appears on hover/selection through inspector route
- clicks route to best object in tile

## Search And Navigator Surfaces

### Global Search

Required result fields:

- result id
- object id
- object kind
- display title
- subtitle/context
- matched text spans
- source id
- address/range
- route
- confidence
- problem/stale badges

Scopes:

- all
- functions
- symbols
- strings
- ObjC
- C++
- cache
- kernel
- problems
- annotations

### Navigator Rows

Common row fields:

- object id
- title
- subtitle
- right detail
- icon role
- route
- count badge
- source badge
- confidence/problem badge

Do not:

- load all symbols/strings/functions into Swift arrays for large projects
- filter navigators on the main actor
- make each navigator define a separate row contract

### Command Palette

The command palette should use the same search/index route model.

Command categories:

- navigate
- analyze
- annotate
- view
- export
- project
- debug later
- patch later

Each command declares:

- id
- title
- shortcut
- required selection kind
- enabled state
- mutation class
- route or transaction result

## Workbench Migration Plan

### PR A: Swift Models Only

Add models:

- `ProjectSummary`
- `ProjectSource`
- `AddressSpaceModel`
- `WorkbenchRoute`
- `RouteResolution`
- `AssemblyWindow`
- `AssemblyRow`
- `AssemblySpan`
- `AssemblyBadge`
- `InspectorModuleModel`
- `FeatureMapModel`
- `SearchResultModel`
- `NavigatorRowModel`
- `ProblemModel`
- `AnalysisJobModel`

Tests:

- JSON decode fixtures
- missing optional fields
- unknown enum fallback where appropriate

### PR B: API Client Methods

Add:

- `createProject(fileURL:)`
- `projectSummary(projectId:)`
- `projectSources(projectId:)`
- `resolveRoute(projectId:route:)`
- `assemblyWindow(projectId:route:)`
- `inspectorModules(projectId:route:)`
- `featureMap(projectId:request:)`
- `projectSearch(projectId:query:)`
- `navigator(projectId:kind:)`

Keep old methods:

- `uploadImage`
- `symbolSuggestions`
- `disassemble`

### PR C: Store State Coexistence

Add project state beside image state.

Behavior:

- load file creates image and project if both APIs exist
- old disassembly remains visible
- project summary appears in project overview or status
- route resolution can be tested without replacing instruction table

### PR D: Semantic Assembly Surface

Add `SemanticAssemblyTable`.

Behavior:

- project route selects function/address
- fetch assembly window
- render spans with fallback text
- selection updates `currentRoute`
- inspector modules load for selected row

Keep old `InstructionSurfaceView` behind compatibility flag until parity is
proven.

### PR E: Inspector Module Surface

Replace fixed instruction inspector with module-driven inspector.

Behavior:

- cheap module headers load on selection
- expanded modules lazy load details
- module errors are visible
- commands are disabled or enabled based on route/object kind

### PR F: Search/Navigator Route Stack

Add:

- global search box
- route-backed results
- back/forward actions
- navigator rows from server
- route restoration after app relaunch if project persistence exists

### PR G: Graph, Hex, Feature Map

Add these in order:

1. Feature map, because it only requires byte classifications.
2. Hex/data view, because it reuses byte ranges and route selection.
3. Graph view, because it needs function records and layout.

## Local UI Server Migration Plan

### Server PR A: Project Endpoints

Add `ProjectStore`.

Endpoints:

- `POST /api/projects`
- `GET /api/projects/{id}`
- `GET /api/projects/{id}/sources`

No existing route removed.

### Server PR B: Route And Assembly

Endpoints:

- `POST /api/projects/{id}/routes/resolve`
- `POST /api/projects/{id}/assembly/window`

Implementation can initially adapt existing disassembly analysis into semantic
rows.

### Server PR C: Inspector

Endpoints:

- `POST /api/projects/{id}/inspector/modules`
- `GET /api/projects/{id}/inspector/modules/{module_id}`

Initial modules:

- instruction overview
- references
- recovered values
- provenance
- problems

### Server PR D: Search/Navigator

Endpoints:

- `GET /api/projects/{id}/search`
- `GET /api/projects/{id}/navigator/{kind}`

Initial navigators:

- sources
- sections
- symbols
- strings if available
- functions if analysis exists
- problems

### Server PR E: Feature Map And Hex

Endpoints:

- `POST /api/projects/{id}/feature-map`
- `POST /api/projects/{id}/hex/window`

## Rust Surface Model

### Assembly View Model Types

Add to `damsel-core` or a `damsel-ui-contracts` module if separation becomes
useful:

- `AssemblyWindow`
- `AssemblyRow`
- `AssemblyRowKind`
- `AssemblySpan`
- `AssemblySpanRole`
- `AssemblyBadge`
- `AssemblyBadgeKind`
- `AssemblyReference`
- `AssemblyProblem`

Do not store these as canonical analysis facts unless needed. They are view
models derived from facts, routes, and annotations.

### Inspector Contract Types

Add:

- `InspectorModule`
- `InspectorModuleId`
- `InspectorModuleState`
- `InspectorModuleSummary`
- `InspectorModuleDetail`
- `InspectorCommand`

Details can be a typed enum for common modules plus JSON escape hatch during
early iteration. Prefer typed modules as they stabilize.

### Graph Contract Types

Add:

- `FunctionGraph`
- `GraphNode`
- `GraphEdge`
- `GraphNodeKind`
- `GraphEdgeKind`
- `GraphPresentation`
- `GraphLayoutJob`

### Feature Map Types

Add:

- `FeatureMapRequest`
- `FeatureMapResponse`
- `FeatureMapTile`
- `FeatureMapLayer`
- `FeatureMapDominantKind`

These should derive from `ByteMap`.

## Performance Requirements

### UI Thread Rules

Swift main actor should not:

- filter massive instruction arrays
- parse large JSON payloads if it can be paged
- read whole multi-gigabyte files into `Data`
- compute graph layout
- compute search ranking
- scan strings

### Latency Budgets

Initial budgets:

- project summary after upload: under 500 ms for small fixture
- route resolution: under 30 ms for loaded project
- assembly window fetch: under 100 ms for 500 rows from in-memory project
- inspector headers: under 50 ms
- expanded small inspector module: under 150 ms
- search keystroke response: under 100 ms for indexed small/medium projects
- feature map tile response: under 100 ms for bounded tile count

Large-artifact budgets should be measured separately and reported in docs.

### Paging Rules

Must page:

- assembly rows above 1,000 rows
- search results
- symbols
- strings
- functions
- xrefs above 500 entries
- cache images
- ObjC methods/selectors in large caches
- C++ symbols/types
- problems

### Cancellation

Long tasks must expose cancellation:

- graph layout
- broad analysis
- cache-wide search
- deep ObjC indexing
- C++ vtable indexing
- kernel/KDK symbolication

## Accessibility And Native Polish Requirements

Semantic surfaces make accessibility easier if built correctly.

Requirements:

- assembly spans expose role labels
- route targets have accessibility names
- graph nodes have keyboard navigation
- inspector module headers are buttons with state
- feature map has textual summary and keyboard alternatives
- search results announce kind, source, and address
- all icon-only controls have labels/tooltips
- no critical workflow depends only on color

Keyboard:

- back/forward
- focus global search
- focus navigator filter
- next/previous search result
- next/previous xref
- jump to address
- switch representation
- expand/collapse inspector module
- reveal current route in navigator

## Test Plan

### Rust Unit Tests

- assembly span generation for instruction with symbol and branch target
- assembly row route generation
- inspector module registry for instruction/function/ObjC object
- feature map tile generation from byte map
- route resolution from address to instruction/function
- search result route generation
- graph node/edge model serialization

### Rust Snapshot Tests

Fixtures:

- small function with branch
- function with call/import
- ObjC method IMP
- cstring and selector refs
- chained fixup if available
- projected cache image if available

Snapshots:

- assembly window
- inspector headers
- feature map
- search results
- navigator rows
- graph model

### HTTP Tests

- create project
- fetch assembly window
- resolve address route
- fetch inspector modules
- fetch feature map
- fetch search results
- invalid project id
- unresolved route
- large limit is clamped

### Swift Model Tests

- decode assembly row with spans
- decode unknown span role gracefully
- decode inspector module states
- decode feature-map response
- decode route resolution
- decode search result

### Workbench Smoke Tests

Manual or automated:

- load fixture
- project summary appears
- open assembly route
- select instruction
- inspector updates
- click xref route
- back returns
- search opens result
- feature map click routes to range
- old disassembly path still works during migration

## Acceptance Criteria

This surface layer is ready when:

- semantic assembly row contracts exist and are tested
- Workbench can render project-backed assembly rows for a fixture
- selecting a row updates a route, not only an address
- inspector modules are route-backed and lazy
- search/navigator rows return routes
- feature map is generated from byte classifications
- old `/api/images` flow remains available until replacement parity is proven
- Swift main actor does not filter huge project-global arrays
- route and assembly endpoints have structured errors
- docs and tests prove no broad completion claim is being made beyond the
  implemented surfaces

## Risks

### Risk: Building UI Before Stable Project Objects

If the UI surface lands before object ids and routes are stable, it will bake in
address-only assumptions.

Mitigation:

- require object ids in semantic rows
- allow unresolved routes but mark them explicitly
- keep compatibility rows separate from project rows

### Risk: Inspector Becomes A Large JSON Dump

Dumping all facts into the inspector would be easy and unusable.

Mitigation:

- module registry
- lazy detail calls
- typed summaries
- per-module latency budgets
- raw facts module as an advanced section only

### Risk: Search Duplicates Navigator Logic

Search and navigators could become separate ranking/filter systems.

Mitigation:

- shared `SearchResult`/`NavigatorRow` route contract
- shared index service
- scoped queries are filters over the same object index

### Risk: Graph Layout Blocks The App

Large functions can freeze a naive graph view.

Mitigation:

- graph facts first
- layout as a cancellable job
- minimap generated from graph summary where possible
- cap rendered nodes until layout completes

### Risk: Workbench Keeps Uploading Huge Files Through Swift Data

The current `uploadImage(fileURL:)` copies the entire file into memory. That
will not work for dyld caches and kernel collections.

Mitigation:

- keep upload for small compatibility fixtures
- add project open by local file path or app-mediated file bookmark
- memory-map in Rust where possible
- expose file-size gates and clear errors

## Relationship To Other Docs

This spec depends on:

- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md)
- [Workbench Information Architecture](workbench-information-architecture.md)
- [API And Data Contracts](api-data-contracts.md)
- [Native App Polish Plan](native-app-polish-plan.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Performance And Responsiveness Architecture](performance-responsiveness-architecture.md)

It refines those docs by providing concrete UI/API/model migration steps for
the surfaces users will touch every minute.

## Bottom Line

The project kernel makes Hopper-like state possible. The semantic Workbench
surfaces make it usable.

Damsel should migrate from:

```text
image upload -> disassembly request -> instruction table -> address jump
```

to:

```text
project -> route -> semantic surface -> contextual inspector -> route stack
```

That shift is what lets assembly, graph, hex, search, ObjC/C++/kernel views,
automation, and future debugger/decompiler work feel like one coherent native
analysis app instead of many separate reports.
