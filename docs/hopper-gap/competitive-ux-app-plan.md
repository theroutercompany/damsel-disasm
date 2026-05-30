# Competitive UX And App Feature Plan

Status: Product and architecture plan. This document treats Hopper-level
competitiveness as an application-quality goal, not only a decoder-feature
goal.

Purpose: define the app experience Damsel needs to become extremely competitive
against Hopper Disassembler while keeping Damsel's own niche: Apple system
binaries, dyld shared caches, XNU-style kernel artifacts, Objective-C, C++,
Objective-C++, arm64e, provenance, and fast offline analysis.

Trust and distribution companion:
[Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
defines the native app trust work behind this experience: authenticated local
helper sessions, source access grants, signing and notarization gates,
diagnostic redaction, update state, plugin trust, crash/hang recovery, and
debugger entitlement readiness.

Artifact intake companion:
[Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
defines the "open this real Apple artifact" experience behind the product:
classification, source registration, member projection, sidecar matching,
panic-log attachment, KDK/dSYM status, app-bundle/IPSW-root handling, and the
first artifact overview screen.

Command/navigation companion:
[Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
defines the product control surface behind this UX: typed commands, menus,
keyboard shortcuts, command palette search, focus ownership, route-backed
selection, back/forward history, undoable transactions, activity logs, job
commands, and Apple-system route actions.

Apple runtime browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
defines how the Apple niche becomes a visible app experience through ObjC,
dispatch, C++/ObjC++, IOKit, cache, kernel, KDK/dSYM, XNU interface, panic,
relationship-graph, and runtime-problem browsers.

## Product Thesis

Damsel should not try to beat Hopper by being a broader generic disassembler.
Hopper's public surface spans native macOS UI, automatic analysis, assembly,
CFG, pseudocode, inspector, references, annotations, patching, debugging,
scripting, SDK extensibility, and multi-format support. Damsel's strongest path
is to become the best-feeling Apple systems exploration tool:

- faster orientation in huge dyld caches
- clearer provenance for inferred facts
- better Objective-C selector/class/callsite navigation
- better C++/IOKit/XNU workflows
- better cache-to-framework-to-kernel mental model
- responsive native app behavior even when analysis is still running
- focused workflows for kernel collections, KDK symbols, panic logs, and
  cross-version Apple OS research

The user should feel that Damsel is not "Hopper but smaller." The user should
feel that Damsel understands Apple's runtime and system binary world more
directly.

## Current Workbench Reality

The current Damsel Workbench is valuable, but it is only the beginning:

- It launches the existing `damsel-cli ui --no-open --port 0` server.
- It uploads a Mach-O file to the local server.
- It exposes section/symbol/address disassembly targets.
- It has a three-pane shape: target controls, instruction listing, inspector.
- It virtualizes the instruction table with `NSTableView`.
- It can show structured operands, annotations, recovered values, and summary
  graph metrics in the inspector.
- It does not yet have a project database, persistent navigation stack,
  symbol/function/string browsers, graph canvas, xref pane, ObjC/C++/kernel
  browsers, project save/load, command palette, or cache/kernel workflows in
  the native app.

That means app competitiveness requires new backend state and new frontend
surfaces. It cannot be achieved by minor styling alone.

## Competitive UX Principles

### 1. Fast Orientation Before Deep Analysis

When a user opens a binary, Damsel should immediately show:

- what was opened
- architecture and platform
- code/data/string/metadata coverage
- likely entrypoints
- top symbols and functions
- ObjC classes/selectors if present
- C++ types/vtables if present
- dyld/cache provenance if present
- kernel/fileset members if present
- analysis progress and what is still pending

The first screen should never be blank while full analysis runs.

### 2. Progressive Disclosure

Damsel should show high-confidence facts first and let the user drill down into
low-confidence or ambiguous facts. This is especially important for ObjC dynamic
dispatch, C++ virtual calls, and arm64e pointer-authenticated metadata.

Default UI:

- clean name and target
- confidence badge only when it matters
- one-click "why" or inspector provenance
- unresolved reasons visible but not noisy

Deep UI:

- source facts
- raw addresses
- raw pointer values
- fixup records
- analyzer pass
- competing interpretations

### 3. Navigation Is The Product

Reverse engineering is a loop:

1. notice a name/string/selector/xref
2. jump
3. inspect
4. mark up
5. return
6. branch to a related path

Damsel must make this loop feel immediate. A competitive app needs:

- back/forward navigation stack
- jump history
- bookmarks
- address/symbol quick jump
- command palette
- fuzzy search
- "show callers"
- "show callees"
- "show references"
- "show implementers"
- "jump to provider image"
- "jump to projected cache image"
- "jump to kext/member"

### 4. Keep The User's Place

Large binaries punish tools that reset context.

Damsel should preserve:

- selected function
- selected instruction
- graph viewport
- active representation
- scroll position
- inspector expansion state
- selected filters
- open tabs
- navigation stack
- project window geometry
- analysis status

This requires persistent UI state separate from analysis facts.

### 5. Analysis Must Not Freeze Interaction

Hopper-level polish is partly the absence of waiting. Damsel should let users
interact with partial results while heavier passes continue.

Required behavior:

- opening a project shows metadata quickly
- analysis runs as cancellable jobs
- lists can populate incrementally
- graph layout can compute in the background
- expensive panes show loading states
- user can search existing indexed facts while deeper analysis continues
- slow passes explain what they are doing

### 6. Apple Niche Should Be Visible

Damsel should make its niche visible in the first five minutes:

- dyld shared-cache image map
- cross-image import/provider/reexport jumps
- ObjC class/selector browser
- C++/IOKit browser
- kernel collection member/kext browser
- KDK/dSYM association status
- arm64e pointer-auth interpretation
- panic/backtrace symbolication
- XNU semantic recognizers

These are not hidden expert flags. They are the app's differentiator.

## Competitive App Shell

### Main Window Regions

Target shape:

```text
+-----------------------------------------------------------------------+
| Toolbar: Open | Analyze | Search | Back | Forward | Mode | Status     |
+-------------------+-----------------------------------+---------------+
| Navigator          | Representation Workspace          | Inspector     |
| - Project          | tabs: Assembly / Graph / Hex      | - Context     |
| - Functions        |       Pseudocode later            | - Xrefs       |
| - Symbols          | current function or object        | - Provenance  |
| - Strings          |                                   | - Comments    |
| - ObjC             |                                   | - Types       |
| - C++              |                                   | - Runtime     |
| - Kernel           |                                   |               |
| - Cache            |                                   |               |
+-------------------+-----------------------------------+---------------+
| Feature map / analysis progress / problems / logs                     |
+-----------------------------------------------------------------------+
```

The current three-pane Workbench can evolve into this shape, but only if the
left pane becomes a navigator instead of only target controls and the center
becomes a representation workspace instead of only an instruction list.

### Toolbar

Required toolbar controls:

- Open binary/cache/kernel project.
- Create project.
- Run/pause/cancel analysis.
- Back/forward.
- Quick jump/search.
- Current mode segmented control: Assembly, Graph, Pseudocode, Hex, Summary.
- Address display mode: vmaddr, file offset, image offset, cache vmaddr, slid
  runtime address when applicable.
- Analysis status.

Pseudocode mode is not only a text renderer. Its detailed Workbench contract is
defined in
[Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md):
line/token models, confidence, source maps, Apple runtime overlays, edit
transactions, jobs, cancellation, cache state, and exports.

Toolbar rules:

- Keep actions icon-first where conventional.
- Do not overload the toolbar with every analysis option.
- Expose advanced analysis settings in command palette or inspector.

### Navigator

The left pane should be a tabbed or sectioned navigator:

- Project
- Functions
- Symbols
- Strings
- Imports
- Exports
- ObjC
- C++
- Kernel
- Cache
- Bookmarks
- Problems

Navigator requirements:

- Fuzzy filter.
- Tag filter.
- Kind filter.
- Source/image/member filter.
- Sort by name/address/reference count/confidence.
- Counts and analysis status per category.
- Virtualized list rendering.
- Stable keyboard navigation.

### Representation Workspace

The center should support synchronized representations:

- Linear assembly.
- Function graph.
- Hex/data view.
- Function summary.
- Pseudocode later.
- Runtime object detail pages.
- Kernel/kext member pages.
- Cache image relation pages.

Representation rules:

- Same selection across views.
- Same navigation stack across views.
- Switching representation should not lose location.
- Missing representation should explain why, not fail silently.

### Inspector

The inspector should be context-sensitive:

- selected instruction
- selected function
- selected reference
- selected data item
- selected ObjC method
- selected C++ type/vtable
- selected kernel object
- selected graph node

Inspector sections:

- Summary
- References
- Reverse References
- Names
- Types
- Runtime
- Cache/Kernel Provenance
- Analysis Confidence
- Comments/Tags/Bookmarks
- Raw Facts

The inspector should not be a dumping ground. Common facts should be compact;
raw facts should be collapsible.

### Feature Map

Hopper's tutorial describes a navigation bar that colors byte regions by type.
Damsel should have a richer Apple-aware feature map:

- code
- functions
- strings
- data
- imports/stubs
- ObjC metadata
- C++ metadata
- chained fixups
- kernel tables
- unknown
- user bookmarks
- current viewport
- analysis problems

For shared caches and kernel collections, the map should have multiple scales:

- whole container
- selected image/member
- selected function

## Core App Features

### Projects

Feature:

- Create, save, reopen, and autosave Damsel projects.

Why:

- Hopper-like analysis is cumulative.
- Comments, labels, xrefs, graphs, and expensive analysis need persistence.

App behavior:

- Recent projects menu.
- Open recent from welcome window.
- Autosave indicator.
- Project health/provenance panel.
- Warn when source files moved or hashes changed.

Backend requirements:

- project database
- source registry
- analyzer versions
- stale fact invalidation
- annotation log

### Welcome And Triage Screen

Feature:

- A first screen for opening files, recent projects, and specialized Apple
  workflows.

Modes:

- Open Mach-O
- Open dyld shared cache
- Open projected cache image
- Open kernel collection
- Attach dSYM/KDK
- Symbolicate panic log
- Compare OS builds

This is where the niche should shine.

### Command Palette

Feature:

- Search every command and navigation action from the keyboard.

Examples:

- "Go to symbol"
- "Show callers"
- "Open selector"
- "Jump to provider image"
- "Decode as string"
- "Run ObjC analysis"
- "Attach KDK"
- "Export CFG"

Backend requirements:

- command registry
- command enablement state
- stable object ids and routes

### Global Search

Feature:

- One search box across symbols, functions, strings, selectors, classes, C++
  types, kexts, addresses, comments, and tags.

Search result fields:

- display title
- kind
- location
- source image/member
- confidence
- secondary text
- jump action

Performance requirement:

- Search existing indexes in under interactive latency budgets even when deep
  analysis is still running.

### Navigation Stack

Feature:

- Back/forward navigation across every jump.

Navigation entries:

- address
- function
- basic block
- instruction
- graph node
- ObjC method/class/selector
- C++ type/vtable
- kernel object
- cache image/member
- search result

Each entry should preserve:

- representation mode
- selection
- scroll/viewport where applicable

### Comments, Labels, Tags, Bookmarks, Colors

Feature:

- User annotation layer.

Objects:

- address
- instruction
- function
- basic block
- data item
- ObjC object
- C++ object
- kernel object
- graph node

Requirements:

- comments support multiline text
- labels override display name but preserve source names
- tags apply to navigation filters
- colors apply to rows and graph nodes
- bookmarks appear in navigator and feature map
- all annotations persist in project database
- all annotation changes have undo/redo

### Xref Workflows

Feature:

- References and reverse references as first-class UI.

User actions:

- Show references from selection.
- Show references to selection.
- Add manual reference.
- Remove manual reference.
- Jump to next/previous xref.
- Filter xrefs by kind.
- Expand cross-image xref.
- Expand candidate dynamic dispatch xrefs.

UI behavior:

- xrefs in inspector for selection
- dedicated xref list for large sets
- graph overlay for reference networks

### Assembly View

Feature:

- Fast, polished linear assembly.

Requirements:

- virtualized rows
- stable row height
- address column mode
- opcode column toggle
- bytes column toggle
- symbols and labels
- inline comments
- colored branch arrows or lightweight equivalent
- current instruction highlight
- breakpoint gutter later
- bookmark gutter
- xref badges
- confidence badges
- jump target hover preview
- copy address/instruction/row/function

The current `NSTableView` direction is good. The next step is to bind it to a
paged project-backed function/instruction API instead of a single decoded
payload array.

### Graph View

Feature:

- Interactive function CFG and other graph types.

Requirements:

- minimap
- fit to selection
- keyboard block navigation
- clickable edges
- node comments/labels/colors
- xref overlay
- selected instruction sync
- collapsed/grouped nodes
- graph export
- list fallback

Graph performance:

- Layout must not block selection or scrolling.
- Large graphs need progressive layout and partial rendering.

### Hex And Data View

Feature:

- Synchronized bytes/data representation.

Requirements:

- file offset and vmaddr display
- selected instruction byte highlight
- data item interpretation
- endian-aware display
- pointer interpretation
- string decoding
- fixup overlays
- read-only default
- future patching behind explicit mode

### ObjC Browser

Feature:

- Apple-native runtime browser.

Views:

- Classes
- Protocols
- Categories
- Selectors
- Methods
- Properties
- Ivars
- Message Sends

Competitive differentiator:

- selector implementers and callers should be faster to reach than in generic
  tools
- category overlay should make it obvious which methods came from which image
  or framework
- message-send candidates should expose confidence and dynamic ambiguity

### C++ And IOKit Browser

Feature:

- ABI-driven C++ browser with XNU/IOKit focus.

Views:

- Demangled symbols
- Types
- Vtables
- Typeinfo
- Constructors/destructors
- Virtual callsites
- IOKit classes
- External method dispatch tables

Competitive differentiator:

- IOKit-specific workflows should be surfaced as first-class, not hidden as
  generic C++ metadata.

### Kernel Browser

Feature:

- XNU/kernel collection explorer.

Views:

- collection members
- kexts
- kext dependencies
- kernel symbols
- IOKit classes
- syscalls
- Mach traps
- MIG subsystems
- panic symbolication
- KDK/dSYM status

Competitive differentiator:

- Damsel should let a user move from panic frame to kext to function to IOKit
  class to vtable to xrefs without leaving the app.

### Shared Cache Browser

Feature:

- dyld shared cache as a map, not only commands.

Views:

- cache members
- images
- dependencies
- dependents
- providers
- importers
- reexports
- ObjC classes/selectors per image
- cross-image xrefs

Competitive differentiator:

- cache-aware "jump to provider implementation" should feel native.

## App Polish Requirements

### Native macOS Conventions

Required:

- standard Open, Save, Save As, Recent, Close, Find, Go, View, Window, Help menus
- command-key shortcuts
- toolbar customization later
- accessible focus ring behavior
- native table selection
- native split-view resizing
- drag-and-drop files
- recent projects
- document restoration
- crash-safe autosave

### Keyboard Shortcuts

Baseline:

- Command-O: open
- Command-S: save project
- Command-F: search current view
- Command-Shift-F: global search
- Command-L: jump to address/symbol
- Command-K: command palette
- Escape/Command-[ back
- Command-] forward
- Space: toggle graph/linear for current function
- N: rename/label
- Semicolon or Command-/: comment
- B: bookmark
- X: xrefs
- G: go to address

Shortcut design should avoid conflicting with text editing fields.

### Error And Empty States

Competitive tools feel polished partly because failures are explainable.

Required error state fields:

- what failed
- why Damsel thinks it failed
- whether user can fix it
- suggested next action
- raw technical detail behind disclosure

Examples:

- incomplete dyld cache set
- missing `.symbols` sidecar
- KDK UUID mismatch
- unsupported pointer format
- malformed load command
- address outside selected image
- no ObjC metadata found
- analysis cancelled
- analysis timed out

### Analysis Progress

Progress should be pass-aware:

- loading source
- parsing Mach-O
- indexing symbols
- classifying bytes
- discovering functions
- decoding functions
- building xrefs
- parsing ObjC
- parsing C++
- recognizing kernel objects
- laying out graphs
- importing DWARF/KDK

Each pass should report:

- state
- current item
- completed/total if known
- elapsed time
- cancellability
- warning count

### Problems Panel

Add a problems panel for analysis warnings:

- unsupported load command
- unparsed fixup format
- suspicious function overlap
- ambiguous symbol
- pointer-auth unknown format
- unresolved fileset member
- KDK mismatch
- truncated debug info

Problem entries should link to the affected object.

### Undo/Redo

First version should cover user annotations:

- comments
- labels
- bookmarks
- tags
- colors
- manual refs
- manual function bounds

Future patching would extend undo/redo to binary modifications, but that is a
separate risk tier.

## Competitive Workflows

### Workflow: Open Objective-C Framework

Target experience:

1. User opens a framework binary.
2. Damsel immediately shows framework identity, architecture, sections, symbol
   counts, ObjC class count, selector count, and analysis status.
3. User types a selector in global search.
4. Results show implementers, callers, and selector refs.
5. User opens selector graph.
6. User jumps to a method IMP.
7. Assembly, CFG, xrefs, and ObjC method metadata stay synchronized.

Needed systems:

- project database
- ObjC runtime index
- global search
- xref index
- graph service
- Workbench ObjC browser

### Workflow: Investigate dyld Shared Cache Import

Target experience:

1. User opens dyld shared cache.
2. User selects a framework image.
3. User disassembles a function that calls an import.
4. Inspector shows provider image and reexport chain.
5. User jumps to provider implementation in projected image.
6. Navigation stack records the cross-image jump.

Needed systems:

- project source registry
- cache image map
- provider/importer index
- cross-image xrefs
- projected image routing
- navigation stack

### Workflow: Investigate IOKit Class In Kernel Collection

Target experience:

1. User opens kernel collection.
2. Damsel shows fileset members and KDK match status.
3. User opens IOKit browser and searches `IOService`.
4. Damsel shows class hierarchy, vtable, methods, and external method dispatch
   table candidates.
5. User jumps to `newUserClient` implementation and sees callers/xrefs.

Needed systems:

- fileset loader
- KDK/dSYM matching
- C++ ABI analysis
- IOKit recognizers
- kernel browser
- graph service

### Workflow: Symbolicate Panic Log

Target experience:

1. User opens kernel project.
2. User drags panic log into Damsel.
3. Damsel extracts frames, asks for slide if needed, and symbolicates.
4. Frames link to functions, kexts, source lines if KDK/DWARF exists, and xrefs.
5. User can save the symbolicated report.

Needed systems:

- panic parser
- address-space slide model
- kernel symbol index
- KDK/dSYM integration
- report export

### Workflow: Compare OS Builds

Target experience:

1. User opens two dyld caches or kernel collections.
2. Damsel indexes comparable images/members.
3. User selects a function/class/selector/kext.
4. Damsel shows added/removed/changed symbols, functions, methods, strings, and
   references.

Needed systems:

- content-addressed source registry
- stable semantic object ids
- project comparison engine
- diff UI

This workflow is a Damsel niche opportunity. Hopper can inspect binaries, but
Damsel can become unusually good at Apple OS version archaeology.

## UX Acceptance Gates

The app is not competitive merely because a feature exists. It needs to pass UX
gates.

### Open Gate

- Opening a small Mach-O shows useful metadata quickly.
- Opening a large dyld cache does not freeze the app.
- User sees what analysis is pending.

### Navigation Gate

- Every shown address/object can be jumped to or copied.
- Back/forward works across assembly, graph, ObjC, C++, cache, and kernel
  views.
- Search results are actionable.

### Graph Gate

- User can switch between linear and graph views without losing location.
- Graph selection syncs with assembly.
- Large graphs degrade gracefully.

### Annotation Gate

- User can label, comment, tag, color, and bookmark.
- Annotations persist after reopen.
- Undo/redo works for annotation changes.

### Responsiveness Gate

- Scrolling long instruction lists remains smooth.
- Search feels immediate on existing indexes.
- Selection and inspector updates are under perceptual latency.
- Analysis can be cancelled.

### Trust Gate

- Ambiguous dynamic analysis is labeled.
- Provenance is inspectable.
- Errors explain recovery steps.

## Large Refactors Required

The following competitive UX features require substantial backend and app
changes:

- Projects and autosave require project database and source registry.
- Global search requires indexed facts across all object kinds.
- Navigation stack requires stable route ids across views.
- Graph view requires durable graph records and layout service.
- ObjC/C++/kernel browsers require normalized runtime/kernel object models.
- Smooth large-cache UX requires paged APIs and background analysis jobs.
- Annotation persistence requires undoable project mutation layer.
- Feature map requires byte classification and range indexes.
- Cross-image cache jumps require unified address-space and source provenance.
- Panic symbolication requires kernel project model and slide-aware address
  mapping.

These should not be treated as cosmetic work. They are product-defining
subsystems.
