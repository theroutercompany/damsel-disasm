# End-To-End Workflow Playbooks

Status: Product and implementation playbook. This document describes concrete
reverse-engineering sessions Damsel should support as a Hopper-competitive
Apple systems workbench.

Purpose: connect the architecture, APIs, data model, performance work, and
Workbench surfaces into real user journeys. A feature is not competitive just
because a backend command exists; it is competitive when the user can complete
an investigation quickly, with preserved context and visible evidence.

Artifact intake companion:
[Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
defines the shared intake/source-registration flow that should precede these
playbooks whenever the user opens a raw file, dyld cache, kernel collection,
KDK, dSYM, panic log, app bundle, or extracted Apple asset root.

Investigation companion:
[Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec](investigation-casebook-session-timeline-implementation-spec.md)
defines how these playbooks become recoverable case records: sessions,
timeline events, evidence pins, hypotheses, findings, notebook pages, reports,
redaction previews, and handoff packages.

## Playbook Principles

Every playbook should specify:

- user intent
- required sources
- initial app state
- primary route sequence
- data needed from Rust/project APIs
- Workbench surfaces involved
- expected latency and progress behavior
- errors and recovery
- validation evidence

Common route pattern:

1. Open project or source.
2. Show artifact identity and analysis status.
3. Let user search or navigate to an object.
4. Preserve route stack.
5. Keep assembly, graph, hex/data, inspector, xrefs, and feature map synced.
6. Surface uncertainty as problems, not silence.
7. Let user annotate or apply a hint.
8. Record provenance and transaction history.

## Playbook 1: Standalone Mach-O Triage

### User Intent

The user has a macOS or iOS Mach-O and wants to quickly understand entrypoints,
imports, strings, functions, ObjC metadata, and suspicious code regions.

### Current Damsel Starting Point

Current Damsel can load standalone Mach-O files, show sections/symbols/imports,
extract ObjC/dyld metadata, and disassemble selected windows. Workbench can open
a file, upload it to the local server, and show a disassembly table.

### Target Flow

1. User opens a binary.
2. Damsel creates a temporary project.
3. Project overview appears within the latency budget.
4. Source summary shows architecture, platform, UUID, entrypoint, sections,
   import/export counts, ObjC counts, string counts, and analysis status.
5. Triage analysis starts in background.
6. User searches for a symbol/string/selector while analysis continues.
7. User opens a function route.
8. Assembly shows semantic rows, labels, comments, xref badges, branch cues, and
   byte spans.
9. Inspector shows instruction details, function summary, xrefs, byte type, and
   problems.
10. User adds a comment/bookmark/tag.
11. Project records annotation transaction.

### Required Backend

- project creation from standalone `BinaryImage`
- source registry
- byte classification
- symbol/string/import/export indexes
- function seeds
- route resolver
- analysis jobs
- annotation transactions

### Required Workbench

- Open Binary
- Project Overview
- Navigator: Functions, Symbols, Strings, Imports, ObjC
- Assembly Workspace
- Inspector
- Problems/Jobs drawer
- Feature map
- Command palette

### Failure States

- unsupported architecture
- malformed Mach-O
- empty input
- source hash changed
- analysis pass failed

### Evidence

- standalone project snapshot fixture
- Workbench open fixture smoke
- search latency probe
- annotation transaction test
- malformed input typed errors

## Playbook 2: dyld Shared Cache Framework Investigation

### User Intent

The user wants to investigate a system framework inside a dyld shared cache,
follow imports to provider implementations, and inspect ObjC classes/selectors.

### Target Flow

1. User opens a dyld shared cache root.
2. Damsel identifies cache parts and image inventory.
3. Cache overview appears before deep analysis.
4. User filters image list for `Foundation`, `UIKitCore`, or another framework.
5. Image page shows install name, base address, size, dependencies, dependents,
   imports, exports, ObjC counts, symbols sidecar status, and analysis status.
6. User searches selector or symbol across the cache.
7. User opens selector route.
8. Selector page lists implementers by class/category/image and likely callers.
9. User opens a method implementation.
10. Assembly route keeps cache image and cache vmaddr context visible.
11. User follows an import to provider implementation through reexport chains.
12. Back/forward returns across cache images without losing context.

### Required Backend

- cache source registry
- subcache/member model
- image object ids
- cache/image address spaces
- provider/importer/reexport facts
- cache-wide symbol/string/search indexes
- cache-wide ObjC runtime object store
- projected image facts retaining cache provenance

### Required Workbench

- Open Shared Cache
- Cache Browser
- Image Overview
- ObjC Runtime Browser
- Global Search
- Cross-image Breadcrumbs
- Assembly/Inspector with cache context
- Dependency graph later

### Failure States

- missing subcache
- unsupported cache version
- local symbols absent
- image ambiguous
- provider not found
- reexport chain unresolved

### Evidence

- synthetic shared-cache corpus
- provider/importer/reexport snapshots
- projected cache route tests
- optional real-cache smoke with exact artifact context

## Playbook 3: ObjC Selector Investigation

### User Intent

The user wants to answer: "Where is this selector implemented, and who calls
it?"

### Target Flow

1. User opens command palette and enters selector text.
2. Damsel returns selector objects, method implementers, and string matches.
3. User selects selector object.
4. Selector page shows implementers, categories, protocols, caller candidates,
   and unresolved references.
5. User filters implementers by image/class/category.
6. User opens method route.
7. Inspector shows owner class/category, type encoding, IMP address, function
   route, xrefs, and metadata provenance.
8. User opens caller candidate.
9. Inspector explains receiver evidence, selector evidence, and confidence.
10. If ambiguous, user applies manual receiver binding.
11. Damsel records hint transaction and reruns targeted analysis.

### Required Backend

- ObjC selector index
- method implementer index
- IMP to function links
- message-send recognizer
- recovered value propagation
- caller confidence model
- manual binding annotation
- targeted reanalysis

### Required Workbench

- Selector search results
- Selector detail page
- Implementers table
- Caller candidates table
- Message-send inspector module
- Problem/hint UI

### Failure States

- selector string missing
- malformed method list
- dynamic selector construction
- receiver class unknown
- multiple implementers with weak evidence

### Evidence

- ObjC fixture selector implementers
- selector caller snapshots
- ambiguous receiver negative test
- manual binding transaction test

## Playbook 4: C++ / ObjC++ Virtual Dispatch Investigation

### User Intent

The user wants to understand virtual dispatch, vtables, typeinfo, and class
relationships in an Apple framework or kernel extension.

### Target Flow

1. User opens a binary/cache/kernel project.
2. Damsel demangles C++ symbols and scans vtable candidates.
3. C++ browser lists classes/types, vtables, methods, constructors, and
   destructors.
4. User opens a vtable route.
5. Vtable view lists slots, target functions, raw/interpreted pointers, and PAC
   or fixup evidence.
6. User opens a virtual callsite.
7. Inspector shows candidate vtable, slot offset, possible targets, and
   confidence.
8. If ambiguous, user can choose a candidate target as a hint.
9. Function summary includes the virtual call with uncertainty.

### Required Backend

- demangler
- C++ symbol classifier
- RTTI/typeinfo parser
- vtable scanner
- virtual callsite recognizer
- PAC/fixup-aware pointer facts
- type database
- manual virtual-call target hint

### Required Workbench

- C++ Browser
- Vtable View
- Virtual Callsite Inspector
- Inheritance Graph
- Assembly/Graph sync

### Failure States

- stripped RTTI
- construction vtable
- multiple inheritance ambiguity
- pointer-auth target unresolved
- false-positive vtable

### Evidence

- vtable fixture
- virtual call fixture
- RTTI fixture
- false-positive rejection fixture
- IOKit-style fixture later

## Playbook 5: Kernel Collection Triage

### User Intent

The user wants to inspect an XNU-style kernel collection, identify members and
kexts, attach KDK symbols, and navigate to relevant functions.

### Target Flow

1. User opens kernel collection.
2. Damsel detects collection/fileset structure.
3. Kernel overview shows build/UUID, architecture, members, kexts, symbolication
   status, and top problems.
4. User attaches KDK/dSYM.
5. Damsel verifies UUID/build match.
6. Symbols/types/source lines enrich functions and runtime objects.
7. User opens kext browser.
8. User searches syscall, Mach trap, MIG routine, panic string, or IOKit class.
9. User opens function route with kernel/member address context visible.

### Required Backend

- kernel collection/fileset loader
- member/kext source model
- global/member address conversions
- KDK/dSYM matcher
- symbol/type/line import
- kernel-specific recognizers
- IOKit C++ integration

### Required Workbench

- Open Kernel Collection
- Kernel Overview
- KDK Status Panel
- Kext Browser
- Kernel Search
- IOKit Browser
- Function/Assembly route with member context

### Failure States

- unsupported kernel format
- KDK mismatch
- missing member
- missing symbols
- address outside known member
- private fixture unavailable

### Evidence

- synthetic fileset fixture
- KDK mismatch fixture
- optional KDK-matched real smoke
- kernel member route tests

## Playbook 6: Panic Log Symbolication

### User Intent

The user has a panic log and wants to map frames to functions, kexts, source
lines, and nearby xrefs.

### Target Flow

1. User opens kernel project.
2. User attaches panic log.
3. Damsel parses panic string, slide, backtrace frames, loaded kexts, and build
   metadata.
4. Damsel matches frames to kernel collection members.
5. KDK/dSYM data enriches symbol names and source lines.
6. Panic view lists frames with confidence and unresolved reasons.
7. User clicks a frame.
8. Assembly route opens at function/frame address.
9. Inspector shows panic-frame context and symbolication provenance.

### Required Backend

- panic parser
- slide extraction
- kext list parser
- frame-to-route resolver
- symbolication profile
- KDK/dSYM source matching

### Required Workbench

- Attach Panic Log
- Panic Symbolication View
- Frame List
- Unresolved Frame Problems
- Route-to-Assembly

### Failure States

- slide missing
- KDK mismatch
- frame outside known member
- source line unavailable
- panic log redacted too heavily

### Evidence

- panic log fixture
- slide/no-slide tests
- unresolved frame tests
- optional real sanitized panic smoke

## Playbook 7: Large Project Search And Triage

### User Intent

The user wants to search across a large project without waiting for every deep
analysis pass.

### Target Flow

1. User opens large cache or kernel project.
2. Initial source inventory and shallow indexes build first.
3. User types in global search.
4. Search returns first page across symbols, strings, selectors, classes,
   imports, functions, tags, and problems.
5. Results show source/image/member context and confidence.
6. User scopes search to current image, ObjC, strings, functions, or problems.
7. User opens result route.
8. If deep analysis is incomplete, Damsel shows partial status and queues
   targeted analysis.

### Required Backend

- incremental index builder
- paged search endpoint
- result versioning
- cancellation
- route resolver
- partial index state

### Required Workbench

- Command Palette
- Global Search
- Scope Filters
- Search Result Routes
- Partial Analysis Indicator

### Failure States

- stale cursor
- search index incomplete
- source missing
- query too broad

### Evidence

- search paging fixture
- result version tests
- Workbench search latency probe
- large-list memory benchmark

## Playbook 8: Function Understanding Without Pseudocode

### User Intent

The user wants to understand what a function does before full decompiler
pseudocode exists.

### Target Flow

1. User opens function route.
2. Summary pane shows arguments/returns, calls, strings, ObjC messages, C++
   virtual calls, stack frame, branches, jump tables, and unresolved facts.
3. Graph view shows CFG.
4. Inspector shows selected instruction and semantic spans.
5. Problems list explains unresolved indirect branches or ambiguous runtime
   facts.
6. User applies hints where useful.
7. Damsel reruns targeted summary analysis.

### Required Backend

- function records
- CFG
- xrefs
- stack frame facts
- call/argument summaries
- ObjC/C++/kernel overlays
- problem/hint model

### Required Workbench

- Function Summary
- Assembly
- Graph
- Inspector
- Problems/Hints

### Failure States

- incomplete function bounds
- unresolved indirect branch
- unknown stack slot
- ambiguous ObjC receiver
- unknown virtual call target

### Evidence

- function summary snapshots
- unresolved-fact negative tests
- graph/source sync tests

## Playbook 9: Automation-Assisted Report

### User Intent

The user wants to generate a local report about selected functions, xrefs,
selectors, or panic frames without sending private binaries anywhere.

### Target Flow

1. User selects functions or search results.
2. User runs local export/report command.
3. Script or future MCP tool queries project facts.
4. Report includes routes, facts, provenance, confidence, and unresolved
   problems.
5. Mutating annotations require explicit permission and audit.

### Required Backend

- stable object API
- read-only query surface
- export service
- audit log for mutations
- privacy redaction settings

### Required Workbench

- Export Selection
- Report Preview
- Script/MCP Activity Log later
- Diagnostics/Privacy Options

### Failure States

- selected route stale
- private strings redacted
- permission denied for mutation
- query too broad

### Evidence

- read-only automation fixture
- export snapshot
- redaction test
- audit log mutation test later

## Cross-Playbook Shared Requirements

### Backend

- source registry
- address-space conversion
- object ids
- route resolver
- project facts
- byte classifications
- search indexes
- xref indexes
- job scheduler
- problem/hint system
- annotation transactions

### Workbench

- project overview
- navigator
- central synchronized workspace
- inspector
- feature map
- command palette
- problems/jobs drawer
- keyboard navigation
- status/progress strip

### Quality Gates

- route stack tests
- search latency probes
- large-list paging tests
- fixture snapshots
- real-artifact evidence only when claimed
- Workbench smoke for each primary workflow
