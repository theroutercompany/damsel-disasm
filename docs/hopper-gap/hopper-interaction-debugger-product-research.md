# Hopper Interaction, Debugger, And Product Surface Research

Status: public-web research pass focused on Hopper's product behavior, not only
its decoder or SDK.

Research date: 2026-05-26.

Purpose: document the user-facing interaction model Damsel must understand if
the goal is to compete with Hopper in UX, app features, polish, performance,
and responsiveness. This research complements the SDK-focused pass by looking
at tutorial workflows, public debugger pages, changelog details, app packaging
evidence, and long-lived community usage patterns.

## Source Set

Primary official sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page: <https://www.hopperapp.com/download.html>
- Hopper release API:
  <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper Debugger Server:
  <https://www.hopperapp.com/HopperDebuggerServer/>
- Hopper SDK 6.3.0 public archive:
  <https://www.hopperapp.com:443/downloader/public/HopperSDK-6.3.0.zip>

Community and package evidence:

- Mike Ash Hopper walkthrough:
  <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- State of Security review:
  <https://stateofsecurity.com/tool-review-hopper-disassembler-for-os-x/>
- Public Hopper script collection:
  <https://github.com/droe/hopper-scripts>
- Procedure export workflow:
  <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- Desktop Insights package page:
  <https://desktopinsights.vercel.app/apps/hopper-disassembler>

Confidence notes:

- Official pages are authoritative for public features and release facts.
- SDK headers are authoritative for exposed plugin/document concepts, not
  private internals.
- Third-party package pages are lower-confidence but useful for packaging,
  entitlement, and dependency expectations.
- Community scripts and articles are evidence of workflows users expect, not a
  complete product specification.

## Executive Findings

Hopper's interaction model is built around synchronized, mutable, routable
representations of one analysis document:

- assembly, control-flow graph, pseudocode, and hex/data views
- a navigation stack and current cursor/selection
- byte typing and manual conversions between code, procedure, data, string, and
  undefined
- symbol, string, procedure, and segment navigation
- labels, comments, tags, colors, bookmarks, and operand formatting
- contextual inspector content
- background analysis with responsiveness constraints
- local and remote debugger integration
- Python and MCP automation over document facts
- export/import workflows for assembly, pseudocode, BinDiff, types, and user
  generated scripts

The public tutorial and SDK make a key point: Hopper is not a static report
generator. The app is a command-rich editing environment for analysis state. A
competitive Damsel Workbench must therefore implement:

- durable route and selection state
- synchronized representations driven by shared project facts
- keyboard-first command routing
- semantic copy/export operations
- undoable analysis mutations
- cancellable jobs
- problem surfaces
- fast navigation across large projects
- domain-specific Apple navigators for dyld cache, ObjC, C++, kernel, KDK, and
  panic logs

## Hopper's Product Contract From Public Pages

### Native Mac Interaction Is A Feature

Official homepage facts:

- Hopper presents itself as a fast native macOS disassembler.
- It emphasizes a Cocoa interface, keyboard shortcuts, and macOS integration.
- It positions the UI as smooth and professional, not merely available.

Implication for Damsel:

- The native app is part of the analysis engine's value. If the app freezes,
  loses selection, hides errors, or treats keyboard commands as afterthoughts,
  the product is not Hopper-competitive even if the parser is strong.
- The Workbench must have a command registry, native menus, route restoration,
  reliable focus behavior, and immediate visual feedback.

### Multiple Synchronized Representations

Official homepage and tutorial facts:

- Hopper exposes assembly, CFG, and pseudo-code representations.
- It can show different representations of a procedure together.
- Graph view is interactive and can be exported to PDF.
- Hex view and disassembly views are tied to the same document state.

Implication for Damsel:

- Damsel needs one selection model shared by assembly, graph, hex, feature map,
  inspector, and metadata browsers.
- A route change should update all visible panes without recomputing unrelated
  data.
- Representations should be backed by stable object ids, not by each view's
  private copy of decoded rows.

### Byte Typing And Manual Reclassification

Official tutorial and SDK facts:

- Hopper associates types with bytes.
- Users can transform data into code, procedures, strings, undefined bytes, and
  several data widths.
- The SDK exposes byte type query and mutation.
- Hex editing can invalidate previous analysis and return regions to undefined
  state.

Implication for Damsel:

- The typed byte map must be a project-kernel primitive.
- Manual classification must be a command with provenance and invalidation,
  even before binary patching exists.
- The feature map should visualize typed ranges, not only section names.
- Analysis reruns should be scoped to invalidated ranges when possible.

### Contextual Inspector

Official homepage facts:

- Hopper's inspector adapts to context.
- Public SDK objects expose enough detail to support address, procedure,
  segment, section, reference, tag, signature, and type inspection.

Implication for Damsel:

- The inspector should be modular. It should not be a single static block of
  instruction details.
- Inspector modules should be selected by route kind and object capabilities:
  instruction, function, xref, selector, method, class, vtable, cache image,
  kext, panic frame, problem, job, byte range.
- Modules should be lazy and pageable when lists are large.

### Navigation Stack And Cursor Model

SDK facts:

- `HPDocument` exposes current address, selected address range, cursor movement
  without navigation stack, navigation-stack goto, address-string goto, and pop.

Implication for Damsel:

- Damsel should distinguish selection changes from navigation pushes.
- Clicking a row should select; following a target should push a route.
- Back/forward should restore route, selection, representation, and viewport.
- Jump-to-address should parse route-aware address strings, not only raw
  integers.

### Procedure As An Editable Unit

SDK facts:

- Hopper procedures expose entry, basic blocks, exit blocks, stack offsets,
  local labels, register aliases, callers, callees, variable names, signatures,
  calling conventions, and tags.

Implication for Damsel:

- A function record must become an editable procedure record.
- Graph, pseudocode, and summary views should all read the same procedure
  object.
- User local labels and register aliases must be stored in project facts, not
  only the renderer.

### Names, Comments, Tags, Colors, And Creation Reasons

SDK facts:

- Hopper tracks creation reasons for comments, names, and signatures.
- Creation reasons distinguish user, script, import, metadata, automatic,
  dynamic, database, dataflow, decompilation, demangling, and call-derived
  facts.

Implication for Damsel:

- Damsel must preserve provenance when displaying names and types.
- "Why is this named this?" should be answerable in the inspector.
- User facts must not be overwritten by imported symbols or inference.
- Script/MCP-originated edits must be auditably different from user UI edits.

## Detailed Interaction Surfaces

### Opening

Hopper-like user expectation:

- open a file
- choose architecture or contained member if needed
- initial analysis begins
- UI stays responsive
- progress is visible
- partial results become navigable

Damsel target:

- Open Binary
- Open Shared Cache
- Open Driver Cache
- Open Kernel Collection
- Open Project
- Open Recent
- Attach dSYM
- Attach KDK
- Attach Panic Log

Required architecture:

- source registry
- loader/extractor/projector pipeline
- analysis jobs
- progress model
- problem model
- project persistence
- recoverable missing-source state

### Representation Switching

Expected representations:

- assembly
- CFG
- pseudocode or decompiler-lite
- hex/data
- metadata browser
- cross-reference list
- graph/list split for functions and runtime objects

Rules:

- switching representation must not lose selected route
- unsupported representation explains why
- loading state appears inside the target representation, not as global app
  freeze
- command shortcuts stay stable across representations

### Feature Map

Hopper tutorial and SDK imply a feature-map-like model because byte types are
core and navigable.

Damsel target feature map lanes:

- code/procedure
- data
- strings
- ObjC metadata
- C++ metadata
- Swift-related metadata
- chained fixups
- PAC pointer facts
- imports/exports
- debug info
- kernel tables
- problems
- user annotations

Interaction:

- click to route to range
- drag to select range
- hover to inspect range summary
- filter by type
- show current viewport
- show analysis coverage

### Command Palette

Hopper 5.14.0 added a command palette for navigating menus, documents, symbols,
and strings. A comparable Damsel palette should search:

- commands
- routes
- symbols
- functions
- strings
- ObjC classes
- selectors
- C++ classes
- vtables
- cache images
- kexts
- syscalls
- Mach traps
- MIG routines
- panic frames
- problems
- jobs

Command entries should declare:

- id
- title
- subtitle
- icon
- shortcut
- enabled predicate
- required permission
- preview route
- execution route or transaction

### Copy And Export

Hopper users expect to copy and export specific semantic objects. Public
scripts demonstrate exporting procedure names and other document facts.

Damsel should support:

- copy address
- copy file offset
- copy image offset
- copy cache address
- copy runtime address
- copy instruction
- copy bytes
- copy string
- copy symbol name
- copy demangled name
- copy route
- copy function summary
- copy selector implementers
- copy xrefs
- export procedure names
- export assembly range
- export graph DOT/SVG/PDF later
- export project facts JSON
- export ObjC headers later
- export type database later

Rules:

- copy commands should be route-aware
- addresses should include address-space kind when ambiguous
- exported facts include provenance and confidence

### Error And Problem Surfaces

Hopper's SDK exposes `addProblemAt:withString:` and document logging. The
changelog shows repeated work around crashes, hangs, deadlocks, and malformed
files.

Damsel target:

- typed problem objects
- problems drawer
- route from problem to affected object/range
- recovery suggestions
- suppress or mark reviewed later
- crash-free malformed input handling
- background job failures recorded as problems

Problem classes:

- loader parse issue
- unsupported load command
- malformed section
- unresolved fixup
- ambiguous indirect branch
- impossible address conversion
- missing dSYM
- dSYM mismatch
- KDK mismatch
- Swift metadata unsupported
- unsupported kernel collection variant
- panic frame unresolved
- analysis timeout
- memory budget exceeded

## Debugger Surface

### Public Debugger Facts

Official sources state:

- Hopper has debugger integration.
- It supports local debugging and remote debugging through a debugger server.
- The Debugger Server page says the separate server is now legacy unless remote
  debugging is required.
- Debugger Server requires Xcode on the target machine.
- Hopper supports LLDB and GDB-related workflows according to public homepage
  and changelog language.
- The changelog has fixes for debugger sessions, attach dialogs, hardened
  runtime detection, patches before launching, GDB remote targets, environment
  variables, and conditional-jump path highlighting when the program counter
  reaches a branch.

### What This Means Architecturally

Debugger integration is not just "launch LLDB":

- static address to runtime address mapping
- slide tracking
- breakpoint objects
- current thread/frame route
- current PC highlighting
- loaded image list
- dynamic symbolication
- environment configuration
- attach permissions and hardened runtime warnings
- remote target connection state
- debugger log stream
- failure recovery

### Damsel Staging

First phase:

- design route mappings and breakpoint target records
- support imported crash/panic/backtrace routes
- keep live debugging out of default product until static navigation is strong

Second phase:

- read-only LLDB session model
- attach/list processes
- loaded image map
- frame routes
- current PC route
- breakpoints as disabled UI if no debugger backend

Third phase:

- local run/attach
- remote debug transport
- environment variables
- conditional branch path highlighting
- patch-before-run only after explicit mutation policy

## Automation Surface

### Python

Public stubs and scripts show a rich Python API around documents, segments,
sections, procedures, names, comments, references, byte typing, tags, colors,
bookmarks, disassembly, decompilation, assembler, and modified executables.

Damsel implication:

- Start with stable HTTP/CLI project APIs.
- Add a thin script binding only when route/object/fact contracts are stable.
- Treat script edits as transactions with script provenance.

### MCP

Official Hopper 6 added MCP and kept improving it through several releases.
The changelog mentions lower tool-access count for LLM document queries, server
reliability, document discovery, launch behavior, and a C++ rewrite for
macOS/Linux parity.

Damsel implication:

- MCP is a competitive feature, but it must sit on safe project APIs.
- MCP should expose narrow document facts and route actions, not broad raw file
  mutation.
- Tool permissions should distinguish read-only, annotation, analysis job, and
  file mutation classes.

## Product Packaging And Reliability Surface

### Current Hopper Package Clues

The official release API shows:

- Mac app version 6.3.0 on 2026-05-19
- SDK version 6.3.0 on 2026-05-19
- macOS deployment target 12.4 for Mac
- Linux packages at 6.3.3 on 2026-05-22

Desktop Insights package evidence suggests:

- universal Apple Silicon and Intel package
- code signing
- notarization
- hardened runtime
- Sparkle update framework
- protocol-related libraries
- debugger-related entitlements or capabilities

The Desktop Insights data is not official product documentation, but it matches
Hopper's public posture as a native signed macOS app with update and debugger
surfaces.

### Damsel Product Requirements

Native app requirements:

- signed builds
- hardened runtime decisions documented
- entitlement plan for file access and future debugger use
- local server lifecycle hardened
- crash logging strategy
- diagnostics export
- update strategy if distributed
- safe startup if plugins or scripts fail later
- no hanging orphan server process after app quit

Reliability requirements:

- corrupted project does not lose user annotations without recovery attempt
- project open has timeout and cancellation
- malformed binary creates problems, not crashes
- large caches show image list before full analysis
- memory pressure is visible and bounded
- failing analysis job does not poison the whole project

## Current Damsel Workbench Gap From Repo Inspection

Inspected current files:

- `damsel-app/Sources/DamselWorkbench/DamselWorkbenchApp.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- `damsel-app/Sources/DamselAppCore/Services/DamselAPIClient.swift`
- `damsel-app/Sources/DamselAppCore/Views/ContentView.swift`
- `damsel-app/Sources/DamselAppCore/Views/ControlPane.swift`
- `damsel-app/Sources/DamselAppCore/Views/InstructionSurfaceView.swift`
- `damsel-app/Sources/DamselAppCore/Views/InspectorPane.swift`

Current strengths:

- native AppKit/SwiftUI application entrypoint
- explicit menu construction
- local server process reuse
- open/drop binary path
- upload image API
- disassembly request API
- symbol suggestions
- virtualized instruction table path
- three-pane control/instruction/inspector layout
- basic toolbar status
- basic banner errors
- some menu validation

Current gaps relative to Hopper-like interaction:

- no project document type
- no project save/open/autosave
- no source registry or sidecar attachment UI
- no route model
- no route stack
- no command palette
- no feature map
- no graph view
- no hex view
- no synchronized representation switcher
- no byte classification UI
- no annotation transactions
- no undo integration beyond generic menu items
- no problems drawer
- no jobs drawer
- no cancellation UI for analysis jobs
- no progress beyond busy spinner
- no native object navigators beyond current control pane
- no ObjC/C++/kernel/cache browsers
- no semantic copy/export commands
- no persistent window state
- no Workbench-level API client for project endpoints
- no current support for paged large lists

Conclusion:

- The current Workbench is a useful alpha shell around point-query disassembly.
  It is not yet a Hopper-like document application. The next Workbench work
  should be a structured migration, not incremental controls added to the
  current `ControlPane`.

## Requirements Derived From This Research

### R1: Native Project Session

Damsel Workbench must introduce a native session object above the current
`WorkbenchStore`.

It owns:

- project id
- active source id
- selected route
- selected address range
- navigation stack
- active representation
- visible ranges
- open drawers
- job subscriptions
- problems count
- unsaved user transaction state

### R2: Command Registry

All menu, toolbar, keyboard, palette, context-menu, and MCP/script commands
should use a shared command registry.

Command metadata:

- id
- title
- menu path
- shortcut
- icon
- enabled predicate
- hidden predicate
- permission class
- transaction kind
- handler

### R3: Representation Controller

Assembly, graph, hex, metadata, and summary views must share:

- route resolver
- selection model
- scroll/viewport model
- semantic span activation
- copy commands
- inspector target

### R4: Inspector Module System

Each route kind maps to inspector modules:

- summary
- addresses
- references
- names
- types
- provenance
- problems
- runtime details
- related objects
- actions

### R5: Job And Cancellation Model

Workbench must display:

- active jobs
- progress
- current phase
- cancellability
- partial-result status
- failure problem route

### R6: Performance Budgets

Interactive budgets:

- route selection update: under 50 ms
- visible row render after data arrival: under 100 ms
- command palette local filtering: under 50 ms for loaded index slice
- large navigator initial paint: under 150 ms with first page
- progress update cadence: 100 to 250 ms, coalesced
- app remains responsive during uploads and analysis

## Implementation Consequences

Small refactors:

- split current `WorkbenchStore` into server, project session, route, and view
  stores
- move menu command enablement into shared command predicates
- add typed error wrappers around API errors
- add status/progress model beyond `isBusy`
- add route-aware copy commands

Large subsystem changes:

- project/document persistence
- local API project endpoints
- route resolver
- paged object indexes
- command registry
- inspector module framework
- feature-map API and UI
- graph model and UI
- hex/data view model
- transaction log and undo bridge
- job event stream
- problem drawer
- automation permission model

Do not treat these as optional polish. They are part of matching Hopper's
actual user experience architecture.

## Acceptance Criteria

This research item is satisfied when future implementation can prove:

- opening a project restores route, representation, selection, and window state
- all visible objects have routes and inspector modules
- assembly, graph, hex, and metadata views share selection
- Back/Forward work across functions, selectors, classes, xrefs, cache images,
  and kernel objects
- byte classifications are visible and editable through transactions
- long jobs can be canceled without corrupting project state
- the problem drawer can route to affected objects
- command palette can navigate commands, symbols, strings, selectors, classes,
  cache images, kexts, and problems
- copy/export commands include address-space context and provenance where
  relevant
- no large list blocks the main thread

If those are not true, Damsel may have good analysis output, but it is not yet
competitive with Hopper's interaction architecture.
