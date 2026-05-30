# Search, Navigation, And Index Plan

Status: Architecture and UX plan for project-wide search, route identity,
navigators, feature map, and cross-object navigation.

Purpose: Hopper-level UX depends on fast search and trustworthy navigation.
Damsel needs a single route/object/index model that powers symbols, strings,
functions, ObjC, C++, cache images, kernel members, xrefs, problems, bookmarks,
feature map, scripting, MCP, and debugger mapping.

Implementation companion: [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
defines how those route, search, navigator, and feature-map contracts appear in
Workbench surfaces and local UI server endpoints.

Command/navigation companion:
[Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
defines how the route/search/index backbone is invoked from menus, keyboard
shortcuts, command palette results, context menus, focus changes, navigation
history, route-backed selections, and undoable project transactions.

Runtime browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
defines the domain-specific route/search/index consumers for ObjC selectors,
message sends, C++ vtables, IOKit candidates, dyld cache images, kernel
members, KDK symbols, panic frames, and runtime relationship graphs.

## Thesis

Search and navigation are not UI conveniences. They are the skeleton of the
workbench. Every object the user can inspect should be:

- indexed
- searchable
- routeable
- selectable
- linkable from xrefs and inspectors
- stable across refresh where possible

If each feature invents its own navigation model, the app will feel fragmented.

## Core Objects

Searchable object kinds:

- source
- segment
- section
- byte range
- symbol
- import
- export
- string
- function
- basic block
- instruction
- xref
- ObjC class
- ObjC category
- ObjC protocol
- ObjC selector
- ObjC method
- C++ type
- C++ vtable
- C++ method
- cache image
- cache dependency/reexport relation
- kernel member
- kext
- syscall/trap/MIG routine
- panic frame
- annotation
- tag
- bookmark
- problem

Every kind should define:

- object id
- display name
- subtitle/context
- route
- source id
- address/range when applicable
- search terms
- icon/category
- confidence/stale state where applicable

## Route Model

Route kinds:

- project overview
- source overview
- address
- function
- basic block
- instruction
- symbol
- string
- xref
- ObjC object
- C++ object
- cache image
- kernel member/kext
- panic frame
- problem
- annotation/bookmark/tag

Route fields:

- route id
- object id
- source id
- address-space id
- address/range
- preferred view
- secondary selection
- display context

Rules:

- route id is stable enough for back/forward within a project
- object id is primary identity; raw address is not enough
- stale routes resolve to tombstone/recovery state
- route resolution can trigger targeted analysis if allowed

## Navigation Stack

Stack operations:

- push route
- replace current route
- back
- forward
- restore previous session route
- open in split
- reveal in navigator

Rules:

- search result open pushes route
- selection within same function may replace route if user is browsing locally
- cross-function/xref/cache/kernel jumps push route
- user annotations preserve route target
- failed route resolution shows recovery, not silent no-op

## Search Indexes

### Primary Text Index

Inputs:

- raw names
- demangled names
- display names
- aliases
- strings
- comments
- tags
- problem messages
- source/install names

Query modes:

- exact
- prefix
- substring
- fuzzy
- scoped

Scopes:

- all
- current source/image/member
- functions
- symbols
- strings
- ObjC
- C++
- cache
- kernel
- problems
- annotations

### Semantic Indexes

Indexes:

- address -> object
- range -> object
- selector -> implementers/callers
- class -> methods/categories/protocols
- vtable -> slots/methods
- type -> users
- function -> callers/callees
- target -> incoming xrefs
- source -> objects
- problem -> route
- tag/bookmark -> routes

### Ranking

Signals:

- exact match
- prefix match
- current source boost
- user annotation boost
- recently visited boost
- object kind priority
- confidence
- stale penalty
- problem severity boost in problem scope

Rules:

- result scoring is deterministic for snapshots where possible
- ranking can evolve behind versioned index metadata
- user-visible filters should make ranking less mysterious

## Navigator Architecture

Navigator tabs:

- Project
- Sources
- Functions
- Symbols
- Strings
- Imports/Exports
- ObjC
- C++
- Cache
- Kernel
- Problems
- Tags/Bookmarks

Common contract:

- query endpoint
- cursor/page
- result version
- sort key
- filter text
- scope
- selection route
- counts

Do not:

- load every row into Swift
- filter huge arrays on the main actor
- implement per-tab bespoke search without shared index contracts

## Command Palette

Command palette combines commands and indexed objects.

Result kinds:

- command
- route
- search query continuation
- recent route
- problem action
- analysis job action

Commands:

- Open Binary
- Open Shared Cache
- Open Kernel Collection
- Attach dSYM/KDK
- Run Analysis
- Cancel Job
- Add Comment
- Rename Label
- Show Xrefs
- Switch View
- Export Selection

Rules:

- first page under interactive latency budget
- disabled commands explain prerequisites
- dangerous commands require confirmation
- command palette is keyboard-first

## Feature Map Integration

The feature map is navigation over byte classifications and overlays.

Inputs:

- byte classification ranges
- functions
- strings
- data
- ObjC metadata
- C++ metadata
- fixups
- problems
- search hits
- bookmarks/tags
- current route
- viewport range

Interactions:

- click range to route
- drag viewport
- hover shows classification/provenance
- search results appear as overlay
- problems appear as overlay

Performance:

- compact runs
- precomputed per source/image/member
- incremental overlay updates
- no whole-project scan on every selection

## Xref Navigation

Xrefs should support:

- from current instruction
- to current function
- to symbol/import
- to string/data
- to ObjC selector/class/method
- to C++ vtable/type
- to cache provider/importer
- to kernel object

UI:

- xref inspector module
- xref list drawer
- keyboard next/previous xref
- route preview
- source/image context

Rules:

- xrefs include provenance/confidence
- manual refs are distinguishable
- stale xrefs are hidden by default but inspectable

## Search API

Endpoint:

```text
GET /api/projects/{project_id}/search
```

Parameters:

- `q`
- `scope`
- `source`
- `kind`
- `limit`
- `cursor`
- `include_stale`

Result fields:

- result id
- object id
- route
- title
- subtitle
- kind
- score
- highlights
- source context
- confidence
- stale flag
- problem count

## Route API

Endpoints:

```text
POST /api/projects/{project_id}/routes/resolve
GET /api/projects/{project_id}/routes/{route_id}
```

Resolve inputs:

- object id
- address
- source id plus address
- symbol/name
- search result id
- problem id

Resolve outputs:

- route
- preferred view
- availability
- recovery action if stale/missing

## Workbench State

Stores:

- `NavigationStore`
- `SearchStore`
- `SelectionStore`
- `NavigatorStore`
- `FeatureMapStore`

State:

- current route
- back/forward stacks
- selected object
- current source scope
- current search query
- recent routes
- view-specific selection

Rules:

- selection follows route
- pane-specific selection can exist but must resolve to route/object when
  navigating
- route changes drive inspector and feature map

## Performance Budgets

- route resolution: under 100 ms warm
- search first page: under 150 ms warm
- navigator first page: under 150 ms warm
- xref list first page: under 150 ms warm
- feature map overlay update: under 100 ms warm
- command palette first page: under 100 ms warm

Large projects may show partial results, but the UI must say they are partial.

## Validation

Fixtures:

- symbol search
- string search
- ObjC selector search
- C++ vtable search
- cache image search
- problem search
- stale route
- route to missing source
- feature-map click route
- xref next/previous

Workbench tests:

- back/forward stack
- search result opens route
- selection preserved after refresh
- navigator selection sync
- command palette dispatch

Negative tests:

- raw address ambiguous across sources
- stale cursor
- route no longer exists
- search index incomplete

## Implementation Sequence

1. Add object id and route model.
2. Add route resolver for source/section/symbol/address.
3. Add primary search index over symbols/strings/sections.
4. Add paged search endpoint.
5. Add Workbench navigation stack.
6. Add command palette shell.
7. Add function/xref search once function index exists.
8. Add ObjC/C++/cache/kernel indexes as runtime stores land.
9. Add feature-map route integration.
10. Add stale route recovery.

## Acceptance Criteria

The search/navigation system is competitive when:

- every primary object is routeable
- global search covers symbols, strings, functions, ObjC, C++, cache, kernel,
  problems, and annotations
- navigators page and preserve selection
- back/forward works across object types
- xrefs use the same route stack
- feature map clicks open routes
- search and navigation stay responsive on large projects
- stale/missing routes show recovery
- command palette can complete common workflows from keyboard
