# Native Workbench Hopper-Grade Implementation Spec

Status: implementation-grade UX and app architecture spec. This is not an
implementation claim.

Purpose: define how to evolve the current Damsel Workbench into a native,
responsive, Hopper-competitive analysis workstation for the Apple systems
niche. This spec is intentionally explicit about small refactors and deep
subsystem changes because UX parity depends on architecture, not only visual
layout.

Research companion:
[Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
explains why these features are required from public Hopper evidence.
[Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
maps the broader public Hopper feature set to the Workbench subsystems that
must share one document/session model.

Core architecture companions:

- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [API And Data Contracts](api-data-contracts.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec](automation-studio-scripting-plugin-workbench-implementation-spec.md)
- [Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md)
- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
- [Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md)
- [Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
- [Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)

## Current Workbench Baseline

Repo files inspected:

- `damsel-app/Sources/DamselWorkbench/DamselWorkbenchApp.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- `damsel-app/Sources/DamselAppCore/Services/DamselAPIClient.swift`
- `damsel-app/Sources/DamselAppCore/Views/ContentView.swift`
- `damsel-app/Sources/DamselAppCore/Views/ControlPane.swift`
- `damsel-app/Sources/DamselAppCore/Views/InstructionSurfaceView.swift`
- `damsel-app/Sources/DamselAppCore/Views/InspectorPane.swift`
- `damsel-app/Sources/DamselAppCore/Views/VirtualizedInstructionTable.swift`
- `damsel-app/Sources/DamselAppCore/Models/DamselModels.swift`

Current shape:

- a single `WorkbenchStore` owns server state, loaded image, selected file,
  disassembly response, symbol suggestions, busy state, status text, controls,
  filter text, and selected instruction address
- `DamselAPIClient` speaks image upload, symbol suggestions, and disassembly
  endpoints
- `ContentView` lays out `ControlPane`, `InstructionSurfaceView`, and
  `InspectorPane`
- app menus expose Open Binary, Disassemble, Find in Instructions, Clear
  Instruction Filter, Reveal Binary, and Copy Binary Path
- toolbar exposes Open Binary, Disassemble, and a compact status item

This is appropriate for alpha point-query disassembly. It is not enough for
Hopper-grade workflows because it lacks project documents, durable routes,
object navigators, transactions, jobs, problems, graph/hex synchronized views,
and Apple runtime browsers.

## Target User Experience

The first screen after opening a source should be the analysis workbench, not a
configuration form.

The user should be able to:

- open a binary, shared cache, driver cache, kernel collection, or project
- immediately see source/member/image overview
- search symbols, selectors, strings, classes, functions, kexts, syscalls, and
  panic frames from one palette
- navigate by route, not by transient table row
- move backward and forward across routes
- switch between assembly, graph, hex/data, metadata, and summary views without
  losing selection
- inspect any object with provenance and confidence
- make comments, labels, classifications, and bindings as transactions
- see active analysis jobs and cancel them
- see problems and jump to affected objects
- copy/export semantic facts with address-space context
- save and reopen the project without losing analysis or user work

## Target App Architecture

### Layers

```text
SwiftUI/AppKit shell
  Command registry
  Project document controller
  Workbench session store
  Route and navigation store
  Representation stores
  Inspector module store
  Jobs/problems stores
  Damsel API client
  Local server process manager
Rust local UI API
  Project API
  Route resolver
  Search/index API
  Assembly/graph/hex/feature map API
  Runtime object APIs
  Job event API
  Transaction API
Rust core/project kernel
  AnalysisProject
  AnalysisDatabase
  Byte map
  Object ids
  Runtime objects
  Transactions
  Jobs
  Problems
```

### Swift Store Split

Replace the single broad `WorkbenchStore` with narrower stores:

- `ApplicationStore`
- `ServerStore`
- `ProjectDocumentStore`
- `WorkbenchSessionStore`
- `RouteStore`
- `SelectionStore`
- `NavigatorStore`
- `RepresentationStore`
- `InspectorStore`
- `JobStore`
- `ProblemStore`
- `CommandStore`

Migration rule:

- keep a facade named `WorkbenchStore` temporarily if needed, but move
  ownership into sub-stores behind it to avoid one giant object controlling the
  app indefinitely.

### ApplicationStore

Owns app-level state:

- local server lifecycle
- current project document
- recent project metadata
- global preferences
- global command registry
- app diagnostics

Does not own:

- selected instruction
- disassembly response
- route stack
- specific navigator filters

### ProjectDocumentStore

Owns document lifecycle:

- open raw source
- open project
- create temporary project
- save project
- save project as
- revert project
- close project
- attach sidecars
- track unsaved user transactions
- recover missing source paths

Public state:

- document id
- project id
- project path
- project name
- source summary
- dirty user transaction count
- save state
- missing source problems

Required API calls:

```text
POST /api/projects
GET /api/projects/{project_id}
POST /api/projects/{project_id}/save
POST /api/projects/{project_id}/attach-source
POST /api/projects/{project_id}/attach-dsym
POST /api/projects/{project_id}/attach-kdk
POST /api/projects/{project_id}/attach-panic-log
```

### WorkbenchSessionStore

Owns UI session state:

- active route
- active source/member/image
- active representation
- split layout
- open drawers
- visible navigator tabs
- inspector module expansion
- window restoration payload

Rules:

- session state is persisted separately from analysis facts
- corrupt session state can be dropped without losing project facts
- project opens before session restoration completes

### RouteStore

Owns:

- current route
- selected range
- back stack
- forward stack
- pending route request
- route resolution result
- route errors

Route kinds:

- source
- segment
- section
- address
- instruction
- procedure
- basic block
- xref
- string
- symbol
- byte range
- ObjC selector
- ObjC class
- ObjC method
- ObjC message
- C++ class
- C++ vtable
- C++ virtual call
- Swift symbol
- cache image
- cache edge
- kernel collection
- kernel member
- kext
- kernel interface
- panic log
- panic frame
- problem
- job

Route operations:

- `select(route)`
- `goto(route)`
- `replace(route)`
- `back()`
- `forward()`
- `revealInNavigator(route)`
- `copyRoute(route)`

Rule:

- selection changes that do not represent user navigation should not push onto
  the back stack.

### SelectionStore

Owns:

- selected route
- selected address range
- selected instruction id
- selected byte range
- selected semantic span
- selected graph node
- selected inspector row

Rules:

- selection is always routeable or explicitly local to a representation
- selection broadcasts to all visible representations through route ids
- selection updates are throttled/coalesced if generated by scrolling

## Command Registry

### Command Definition

Every menu item, toolbar item, command-palette action, context-menu action,
keyboard shortcut, and future automation action should use a shared command
definition:

```swift
struct WorkbenchCommand {
    let id: CommandID
    let title: String
    let menuPath: [String]
    let shortcut: KeyboardShortcut?
    let iconName: String?
    let permission: CommandPermission
    let isEnabled: (CommandContext) -> Bool
    let reasonDisabled: (CommandContext) -> String?
    let handler: (CommandContext) async throws -> CommandResult
}
```

Command permissions:

- read only
- navigation
- analysis job
- annotation mutation
- source attachment
- project save
- debugger read
- debugger mutation
- binary mutation

Binary mutation remains disabled in the first Hopper-gap implementation wave.

### Required Command Families

File:

- Open Binary
- Open Shared Cache
- Open Driver Cache
- Open Kernel Collection
- Open Project
- Open Recent
- Save Project
- Save Project As
- Revert Project
- Attach dSYM
- Attach KDK
- Attach Panic Log
- Export Project Facts
- Export Procedure Names
- Close Project

Edit:

- Undo
- Redo
- Copy
- Copy Address
- Copy File Offset
- Copy Image Offset
- Copy Cache Address
- Copy Runtime Address
- Copy Instruction
- Copy Bytes
- Copy Symbol
- Copy Demangled Name
- Copy Route
- Copy Xrefs
- Add Comment
- Rename
- Add Bookmark
- Add Tag
- Set Color

Navigate:

- Back
- Forward
- Jump To Address
- Jump To Symbol
- Jump To Route
- Jump To Selection Target
- Show Xrefs To
- Show Xrefs From
- Go To Function Start
- Go To Next Function
- Go To Previous Function
- Go To Next Problem
- Go To Previous Problem
- Reveal In Navigator

Analyze:

- Run Triage Analysis
- Run Deep Analysis
- Analyze Current Source
- Analyze Current Function
- Build Xref Index
- Build ObjC Index
- Build C++ Index
- Build Cache Index
- Build Kernel Index
- Cancel Current Job
- Cancel All Jobs

View:

- Assembly
- Graph
- Pseudocode
- Hex
- Metadata
- Summary
- Toggle Navigator
- Toggle Inspector
- Toggle Feature Map
- Toggle Pseudocode Confidence
- Toggle Pseudocode Address Column
- Toggle Problems
- Toggle Jobs
- Toggle Logs
- Increase Font Size
- Decrease Font Size

Runtime:

- Show ObjC Classes
- Show ObjC Selectors
- Show C++ Classes
- Show Vtables
- Show Cache Images
- Show Kexts
- Show Syscalls
- Show Mach Traps
- Show MIG Routines
- Show Panic Frames

Future Debug:

- Attach To Process
- Launch Under Debugger
- Continue
- Pause
- Step Over
- Step Into
- Step Out
- Toggle Breakpoint
- Show Loaded Images

## Project Shell UI

### Main Layout

Target layout:

```text
toolbar
source/object navigator | central representations | inspector
feature map
status/progress strip
bottom drawer: problems/jobs/logs/search results
```

The current `ControlPane` should be replaced by a project navigator. Decode
configuration moves into commands, route-specific actions, or inspector modules.

### Toolbar

Controls:

- Back
- Forward
- Jump/Search field or command palette trigger
- representation segmented control
- active project/source display
- jobs progress indicator
- problems indicator
- navigator toggle
- inspector toggle

Rules:

- toolbar controls are command-backed
- disabled state has a reason
- status view is not the only place progress exists

### Source/Object Navigator

Top-level sections:

- Sources
- Address Spaces
- Segments
- Sections
- Functions
- Symbols
- Strings
- Xrefs
- ObjC
- C++
- Swift
- Shared Cache
- Kernel
- Panic Logs
- Problems

Requirements:

- every list is pageable
- local filtering under 50 ms for loaded page
- remote search for large indexes
- selected route is revealed
- stale/partial analysis state is visible
- counts can be approximate when expensive

### Central Workspace

Representations:

- assembly table
- CFG graph
- hex/data view
- decompiler-lite or summary
- metadata object view
- cross-reference list
- cache/kernel graph

Workspace rules:

- representation switch preserves route
- split views can show two representations of same route
- route open can choose default representation by object kind
- unsupported representation shows reason and suggested action

### Inspector

Inspector modules by category:

- Summary
- Address
- Bytes
- Names
- Types
- References
- Procedure
- Runtime
- Provenance
- Problems
- Actions

Example instruction route modules:

- instruction summary
- operands and semantic spans
- direct targets
- xrefs from
- xrefs to
- recovered values
- byte classification
- runtime overlays
- provenance
- problems

Example ObjC selector modules:

- selector summary
- implementers
- callers
- string locations
- cache images
- type encodings
- unresolved candidates
- provenance

Example C++ vtable modules:

- vtable summary
- slots
- target procedures
- RTTI link
- PAC/fixup evidence
- virtual callsites
- provenance

Example panic frame modules:

- raw frame
- unslid address
- matched kext/member
- function route
- symbolication profile
- source line if available
- unresolved evidence

### Feature Map

Feature map lanes:

- code/procedure
- data
- strings
- ObjC metadata
- C++ metadata
- Swift metadata
- fixups/PAC
- symbols/imports
- kernel tables
- problems
- user annotations

Interactions:

- hover summary
- click route
- drag select range
- show viewport
- show current function extent
- show analysis coverage
- show stale regions

Data contract:

```json
{
  "source_id": "source:1",
  "address_space_id": "addrspace:image:1",
  "runs": [
    {
      "start": "0x100000000",
      "length": 4096,
      "kind": "procedure",
      "confidence": "strong",
      "route": { "kind": "procedure", "id": "procedure:..." }
    }
  ],
  "viewport": {
    "start": "0x100003000",
    "length": 2048
  }
}
```

## Representation Details

### Assembly View

Current state:

- `InstructionSurfaceView` shows header, filter, and virtualized instruction
  table.
- `VirtualizedInstructionTable` renders decoded rows from a single
  `DisassemblyResponse`.

Target:

- route-backed instruction windows
- semantic spans
- gutter for address, bytes, labels, breakpoints later, problems, bookmarks
- row decorations for xrefs, comments, runtime objects, byte classifications
- local labels
- function boundary markers
- inline comments
- selection and hover actions

Data loading:

- request visible window by route/address
- prefetch before/after viewport
- update rows incrementally as analysis facts arrive
- keep row identity stable across fact updates

API:

```text
GET /api/projects/{project_id}/assembly/window?route=&before=&after=
GET /api/projects/{project_id}/assembly/row/{instruction_id}
```

### Graph View

Target:

- function CFG
- selected block syncs to assembly
- selected instruction syncs within block
- edges distinguish fallthrough, branch, call, exceptional, synthetic
- graph node decorations for comments/problems/runtime objects
- minimap
- fit selected route
- export DOT first, SVG/PDF later

API:

```text
GET /api/projects/{project_id}/graphs/cfg?procedure_id=
GET /api/projects/{project_id}/graphs/call?scope=
GET /api/projects/{project_id}/graphs/runtime?object_id=
```

### Hex/Data View

Target:

- byte grid with address gutter
- ASCII column
- typed range overlays
- selected range syncs to assembly/feature map
- pointer target hints
- strings/fixups/ObjC/C++/kernel metadata overlays
- read-only first
- patching disabled until mutation policy exists

API:

```text
GET /api/projects/{project_id}/hex/window?space_id=&start=&length=
GET /api/projects/{project_id}/byte-map?space_id=&start=&length=
```

### Metadata Browser

Target route kinds:

- ObjC class
- ObjC selector
- ObjC method
- C++ class
- C++ vtable
- Swift symbol
- cache image
- kext
- syscall
- Mach trap
- MIG routine
- panic log

Rule:

- metadata browsers are not separate mini-apps. They are route views sharing
  selection, copy, inspector, problems, and command palette.

## Jobs And Progress

### Job Model

Job fields:

- id
- title
- kind
- target route/source
- phase
- progress fraction, optional
- total/current units, optional
- cancellable
- started at
- last updated at
- status
- partial result availability
- problem ids

Job kinds:

- load source
- project source
- build byte map
- disassemble range
- build function index
- build xref index
- build ObjC index
- build C++ index
- build cache index
- build kernel index
- import dSYM
- import KDK
- parse panic log
- export

UI:

- toolbar condensed active job
- status strip detail
- jobs drawer with all jobs
- cancel button where allowed
- route from job to target
- route from failed job to problem

API:

```text
GET /api/projects/{project_id}/jobs
GET /api/projects/{project_id}/jobs/{job_id}
POST /api/projects/{project_id}/jobs/{job_id}/cancel
GET /api/projects/{project_id}/events
```

Events should be server-sent events or websocket later; polling is acceptable
for the first pass if coalesced and bounded.

## Problems

Problem drawer columns:

- severity
- code
- message
- target
- source
- status

Problem actions:

- reveal target
- show details
- copy diagnostic
- retry job when applicable
- attach missing sidecar when applicable
- mark reviewed later

Problem routing:

- clicking a problem routes to the affected object/range
- inspector shows provenance and recovery action

## Transactions And Undo

### Transaction Kinds

- label
- comment
- bookmark
- tag
- color
- byte classification
- make procedure
- function bounds adjustment
- manual xref
- ObjC message binding
- C++ virtual call binding
- type/signature edit
- attach sidecar

### Swift Undo Bridge

Requirements:

- command registry starts project transaction
- local API returns transaction id and inverse availability
- Swift registers undo action with `UndoManager`
- undo calls transaction API
- redo calls transaction API
- menu enabled state reflects project transaction stack

API:

```text
GET /api/projects/{project_id}/transactions
POST /api/projects/{project_id}/transactions/{transaction_id}/undo
POST /api/projects/{project_id}/transactions/{transaction_id}/redo
```

Rule:

- analyzer-derived facts are not undoable user transactions unless initiated
  through an explicit user command.

## Search And Command Palette

### Search Indexes

Local indexes:

- loaded commands
- loaded navigator page
- recent routes
- visible instruction rows

Remote indexes:

- symbols
- strings
- functions
- selectors
- classes
- methods
- C++ names
- cache images
- kexts
- kernel interfaces
- problems

### Palette Result Model

Fields:

- id
- kind
- title
- subtitle
- route
- command id
- score
- source
- icon
- keyboard hint

API:

```text
GET /api/projects/{project_id}/search?q=&kinds=&limit=
```

Performance:

- first local results under 50 ms
- remote results update asynchronously
- stale search result versions are discarded

## API Client Migration

Current `DamselAPIClient` endpoints:

- `POST /api/images`
- `GET /api/images/{imageId}/symbols`
- `POST /api/images/{imageId}/disasm`

Target client groups:

- `ProjectClient`
- `RouteClient`
- `AssemblyClient`
- `GraphClient`
- `HexClient`
- `SearchClient`
- `NavigatorClient`
- `RuntimeClient`
- `TransactionClient`
- `JobClient`
- `ProblemClient`

Migration rule:

- keep existing image endpoints during transition
- add project endpoints without breaking current CLI UI
- wrap old image upload as "create temporary project from binary" once project
  kernel exists

## Performance And Responsiveness Requirements

### Main Thread Rules

- no file reading on main actor
- no JSON decoding of large responses on main actor
- no graph layout on main actor
- no filtering huge remote result sets on main actor
- no synchronous project save on main actor

### Budgets

Interaction:

- route select visual feedback: under 50 ms
- row selection inspector header update: under 50 ms
- representation switch shell update: under 100 ms
- command palette local search: under 50 ms
- navigator page render: under 150 ms

Background:

- progress update coalescing: 100 to 250 ms
- large JSON decode: detached task
- table row diffing: stable ids and incremental updates
- graph layout: background task with cancellation

Memory:

- do not hold all disassembly rows for every cache image in Swift
- keep visible windows and small prefetch ranges
- page navigator lists
- evict inactive large payloads while preserving route state

## Accessibility And Keyboard Requirements

Keyboard:

- all primary commands have menu items
- command palette is keyboard reachable
- route list supports arrow navigation
- assembly view supports up/down/page/home/end
- graph view has list fallback
- inspector modules are keyboard navigable
- feature map has accessible range summary

Accessibility:

- semantic labels for all icon buttons
- table rows expose address, mnemonic, operands, and comments
- graph nodes expose block summary
- problem rows expose severity and recovery
- progress exposes phase and percent when available
- color is not sole carrier of byte classification

## Migration Plan

### Phase 0: Stabilize Existing Shell

Small refactors:

- move server state into `ServerStore`
- move image/disassembly endpoint calls into an API facade
- define `WorkbenchRoute` enum without using it everywhere yet
- define command ids for current commands
- route current instruction selection through `SelectionStore`
- add route-aware copy address and copy instruction commands

Exit criteria:

- existing open/disassemble/filter flow still works
- menu actions call command registry
- selected instruction has a route id

### Phase 1: Project Session Skeleton

Build:

- project document model in Swift
- project open/create endpoint wrappers
- source navigator replacing most of `ControlPane`
- route stack
- Back/Forward menu and toolbar
- route persistence in window state

Exit criteria:

- opening a binary creates a temporary project route
- route back/forward works across instruction and symbol jumps
- app can restore last route for a saved project fixture

### Phase 2: Semantic Assembly And Inspector Modules

Build:

- assembly window endpoint
- semantic instruction row model
- span activation
- inspector module registry
- xref module
- provenance module
- problem module

Exit criteria:

- selecting an operand span updates inspector
- xref target click pushes route
- stale or unresolved facts are visible

### Phase 3: Jobs, Problems, And Feature Map

Build:

- job list API/client
- problem list API/client
- jobs drawer
- problems drawer
- feature-map API/client/view
- cancel command

Exit criteria:

- long analysis shows job progress
- cancellation works
- problem route jumps to affected object
- feature map reflects byte classifications

### Phase 4: Object Navigators

Build:

- functions navigator
- symbols navigator
- strings navigator
- ObjC navigator
- C++ navigator
- cache navigator
- kernel navigator
- panic navigator

Exit criteria:

- navigators are pageable
- route reveal works
- command palette searches runtime objects

### Phase 5: Graph And Hex

Build:

- CFG graph endpoint and view
- graph selection sync
- graph minimap/list fallback
- hex window endpoint and view
- typed byte overlays

Exit criteria:

- assembly and graph selections sync
- hex selection syncs with feature map
- large function graph does not block main thread

### Phase 6: Transactions And Undo

Build:

- transaction API
- Swift undo bridge
- comment/rename/bookmark/tag commands
- byte classification commands
- ObjC/C++ manual binding commands

Exit criteria:

- user comment persists
- undo/redo works through project API
- transaction provenance is visible

### Phase 7: Packaging And Diagnostics

Build:

- diagnostics export
- local server crash recovery
- project corruption recovery path
- signing/entitlement notes
- hardened runtime plan
- app lifecycle tests

Exit criteria:

- app quits without orphan server
- server failure shows recovery UI
- diagnostics package contains logs without private binary bytes by default

## Test Plan

### Unit Tests

Swift:

- route parsing and equality
- command enabled predicates
- command permission classification
- navigation stack semantics
- selection synchronization
- job event reduction
- problem grouping
- transaction undo state
- address formatting by address-space kind

Rust/API:

- route resolver
- assembly window shape
- graph model shape
- hex window shape
- feature-map run compaction
- search response paging
- job cancellation state
- transaction inverse availability

### UI Smoke Tests

Scenarios:

- open binary and route to entry point
- jump to symbol and back
- filter/search instructions
- select xref target
- open function graph
- open hex at selected address
- show problem drawer and route to problem
- cancel analysis job
- add comment and undo
- restore project route after reopen

### Performance Tests

Scenarios:

- large instruction window scroll
- command palette over large symbol list
- feature-map render for large address space
- graph layout cancellation
- cache image navigator first page
- ObjC selector browser with many selectors
- jobs event stream under frequent updates

### Accessibility Tests

Scenarios:

- VoiceOver summary for assembly row
- keyboard-only route navigation
- keyboard-only command palette use
- inspector module expansion
- graph list fallback
- problem drawer navigation

## Acceptance Gates

The Workbench should not be called Hopper-grade until:

- project documents can be saved and reopened
- route stack works across all major object kinds
- assembly, graph, hex, feature map, navigator, and inspector share selection
- byte classifications are visible in at least assembly, hex, and feature map
- comments and labels are transactional and undoable
- jobs are visible and cancellable
- problems are routeable
- command palette searches commands and project objects
- large lists are paged
- main thread remains responsive during open/analyze/search/graph operations
- ObjC/cache/kernel niche objects have native browsers, not JSON dumps

## Engineering Risks

### Risk: Giant WorkbenchStore

The current store already mixes app, server, file, request, selection, filter,
status, and result state. Adding project features there would make correctness
fragile.

Mitigation:

- split stores before adding route stack and transactions

### Risk: Web-API Mindset In Native App

If every action waits for a full response before changing UI state, Workbench
will feel slow.

Mitigation:

- optimistic route shell updates
- skeleton states
- partial results
- event stream for jobs
- background JSON decoding

### Risk: Graph And Hex As Isolated Views

If graph and hex do not share selection and route state, they will feel like
separate tools.

Mitigation:

- central `SelectionStore`
- route ids in every representation payload
- inspector reads selected route, not selected view type

### Risk: Automation Unsafe By Default

Once MCP/script commands exist, broad mutation APIs can damage project state.

Mitigation:

- command permissions
- transaction provenance
- undo where possible
- read-only default MCP profile

### Risk: Apple Niche Hidden Under Generic UI

If ObjC/C++/cache/kernel/panic objects are buried under generic JSON panels,
Damsel loses its intended advantage.

Mitigation:

- native navigators and inspector modules for Apple runtime objects in the main
  app IA

## Definition Of Done For This Spec

This spec is implemented when code evidence shows:

- a split store architecture or equivalent ownership boundaries
- a route model used by selection, navigation, inspector, and copy commands
- a project document model and save/reopen flow
- command registry powering menus, toolbar, palette, and context actions
- semantic assembly window backed by project routes
- at least one synchronized non-assembly representation
- feature map backed by typed byte classifications
- job and problem drawers backed by project APIs
- transaction and undo path for comments or labels
- native browsers for at least ObjC selectors/classes and cache images
- performance tests or measurements proving responsiveness budgets on a
  representative large fixture

Until then, Workbench remains an improving alpha UI, not a Hopper-grade app.
