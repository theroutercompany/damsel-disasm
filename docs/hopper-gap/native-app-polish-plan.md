# Native App Polish Plan

Status: Product and UX specification for the macOS Workbench. This document is
about interaction quality, not parser correctness.

Purpose: define what "competitive against Hopper in UX, app features, app
polish, system performance, and responsiveness" means in concrete native macOS
terms.

Surface implementation companion:
[Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
defines the semantic row, route, inspector, graph, hex, feature-map, search,
and navigator contracts that native polish depends on.

Detailed Workbench implementation companion:
[Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
defines the concrete Swift/app migration path from the current `WorkbenchStore`
and three-pane decode UI to project sessions, route stores, command registry,
synchronized representations, jobs, problems, transactions, and Apple runtime
browsers.

Command/navigation/undo companion:
[Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
defines how native menus, shortcut routing, command palette, context menus,
focus, selection, route history, transaction-backed undo/redo, activity logs,
and disabled-command explanations should be implemented rather than treated as
small one-off polish items.

Preferences/keymaps/layout companion:
[Preferences, Keymaps, Layout Persistence, And Theming Implementation Spec](preferences-keymaps-layout-theming-implementation-spec.md)
defines the typed settings, keymap, theme-token, layout/session persistence,
route-repair, analysis-profile, Apple-artifact-root, automation/privacy, and
diagnostics-preset architecture behind the polish requirements in this plan.

Apple runtime browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
defines the polished native browser behavior for Apple runtime workflows:
paged lists, detail pages, confidence badges, sidecar/KDK status, panic-frame
routes, relationship graphs, runtime problem drawers, and domain-specific
commands.

Pseudocode Workbench companion:
[Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
defines the native pseudocode surface polish: stable code layout, gutter
controls, line/token selection, confidence marks, source-map navigation,
variable/type edit sheets, job banners, export sheets, accessibility labels,
and large-function responsiveness.

Trust and distribution companion:
[Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
defines the signing, notarization, hardened-runtime, source access, local
helper, diagnostics, update, debugger entitlement, and plugin trust layer that a
native reverse-engineering Workbench needs before it can be distributed with
Hopper-like confidence.

Live debugger companion:
[Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
defines debugger-specific native polish: launch and attach sheets, debugger
bar state, breakpoint gutters, stopped-state layouts, current-PC overlays,
register/frame/memory panes, LLDB/helper failure UX, permission errors, and
redacted session exports.

Patch Studio companion:
[Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md)
defines patching-specific native polish: patchability explanations, byte edit
and assembler sheets, NOP previews, before/after diff panels, impact summaries,
warning acknowledgements, patch history, export sheets, and debugger handoff
for patched revisions.

Automation Studio companion:
[Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec](automation-studio-scripting-plugin-workbench-implementation-spec.md)
defines automation-specific native polish: recipe catalogs, run lists, MCP
session visibility, script and plugin package state, capability prompts,
activity history, output inspectors, diagnostics, and Apple-system workflow
recipes.

AI assistant companion:
[Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md)
defines assistant-specific native polish: context bars, evidence trays,
tool-call timelines, suggestion review, privacy indicators, model profiles,
route/fact citations, and AI output validation before claims or mutations are
presented as trusted.

Research companion:
[Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
grounds the interaction target in Hopper's public tutorial, debugger,
automation, package, and changelog surfaces.

## Thesis

Reverse engineering tools earn trust through small interaction details. Users
notice whether navigation is instant, selection is preserved, errors are clear,
menus are predictable, keyboard shortcuts work, huge lists do not jump, and
long analysis does not freeze the app.

Damsel's app should feel like a native analysis workstation, not a web UI inside
a shell and not a demo panel over CLI output.

## Current Workbench Gap

Current Workbench strengths:

- native macOS entrypoint
- existing CLI UI server reuse
- three-pane layout direction
- virtualized instruction table path
- symbol suggestions and address jumping
- separation of Swift app and Rust decoder logic

Current competitive gaps:

- no project document lifecycle
- no native file/project autosave model
- no durable route/navigation stack
- no command palette
- no complete menus
- no left navigator that owns all object types
- no feature map
- no graph/hex synchronized representation
- no annotation transaction UI
- no problem panel
- no first-class progress/cancellation model
- limited keyboard model
- limited accessibility model
- no polished empty/error/loading states

## Native Document Model

### Document Types

Damsel should distinguish:

- raw opened input
- saved Damsel project
- temporary triage project
- imported sidecar
- exported report

User-facing commands:

- Open Binary
- Open Shared Cache
- Open Kernel Collection
- Open Project
- New Project From Selection
- Attach dSYM
- Attach KDK
- Attach Panic Log
- Save Project
- Save Project As
- Revert To Saved
- Close Project

Behavior:

- opening a binary creates a temporary project until saved
- saving stores analysis state and annotations
- closing with unsaved user annotations prompts clearly
- analyzer-derived stale facts do not count as unsaved user work
- recent projects remember project paths, not raw private binary paths unless
  user chooses that behavior

### Window State

Persist per project:

- selected route
- navigation stack
- open tabs/splits
- active representation
- navigator filters
- inspector module expansion
- graph zoom/pan
- feature-map viewport
- table column widths
- selected theme

Rules:

- restoring window state must never block project opening
- missing sources show recovery UI
- corrupt window state falls back safely without corrupting project data

## App Shell

### Main Regions

Recommended layout:

- toolbar
- project/source navigator
- central workspace
- contextual inspector
- feature map
- status/progress strip
- optional bottom drawer for problems, logs, search results, and jobs

The center should not be a single table forever. It should host synchronized
representations:

- assembly
- graph
- hex/data
- decompiler-lite summary
- metadata browser
- cache/kernel graph

### Toolbar

Toolbar goals:

- show current project/source
- show current route/function
- expose representation switcher
- expose search
- expose analysis progress
- keep primary navigation one click away

Toolbar controls:

- Back
- Forward
- Jump
- Search
- Representation segmented control
- Analyze/Cancel
- Problems
- Inspector toggle
- Navigator toggle

Rules:

- toolbar updates must be under 50 ms
- disabled controls explain why through tooltip/status
- destructive or write-like actions are not toolbar-primary

### Status Strip

Status strip content:

- current address and source
- analysis state
- selected byte length
- current symbol/function
- active job summary
- warnings/problem count
- stale project indicator

Behavior:

- long jobs show progress without modal blocking
- cancellation is accessible and immediate
- partial analysis state is visible
- background errors appear as problem entries, not transient toasts only

## Menus

### File

Commands:

- Open Binary...
- Open Shared Cache...
- Open Kernel Collection...
- Open Project...
- Open Recent
- Save Project
- Save Project As...
- Attach dSYM...
- Attach KDK...
- Attach Panic Log...
- Export
- Close

### Edit

Commands:

- Undo
- Redo
- Cut/Copy/Paste where text fields apply
- Copy Address
- Copy Instruction
- Copy Bytes
- Copy Symbol
- Copy Route
- Find
- Find Next/Previous

### Navigate

Commands:

- Back
- Forward
- Jump To Address
- Jump To Symbol
- Jump To Function
- Jump To Selection Target
- Show Xrefs To
- Show Xrefs From
- Go To Function Start
- Go To Next Problem
- Go To Previous Problem
- Go To Next Bookmark
- Go To Previous Bookmark

### Analyze

Commands:

- Run Triage Analysis
- Run Deep Analysis
- Analyze Current Function
- Reanalyze Selection
- Cancel Analysis
- Mark As Code
- Mark As Procedure
- Mark As Data
- Mark As String
- Mark Undefined
- Add Manual Reference
- Apply Switch Hint

### View

Commands:

- Assembly
- Graph
- Hex
- Summary
- Show Navigator
- Show Inspector
- Show Feature Map
- Show Problems
- Show Jobs
- Toggle Instruction Bytes
- Toggle Branch Cues
- Toggle Demangled Names
- Toggle Raw Names
- Increase/Decrease Font Size

### Window

Commands:

- New Window
- Split Right
- Split Down
- Close Split
- Move Focus To Navigator
- Move Focus To Workspace
- Move Focus To Inspector
- Cycle Tabs

### Help

Commands:

- Damsel Help
- Keyboard Shortcuts
- Diagnostics Export
- Report Issue

## Keyboard Model

Keyboard navigation is a core feature, not accessibility afterthought.

Required shortcuts:

- back/forward
- command palette
- global search
- jump to address/symbol
- xrefs to/from
- add comment
- rename label
- toggle bookmark
- mark code/data/string/procedure
- switch assembly/graph/hex/summary
- next/previous function
- next/previous problem
- next/previous xref
- cancel analysis

Assembly keyboard rules:

- Up/Down move rows
- Page Up/Page Down preserve context
- Left/Right can follow/collapse branch cues where applicable
- Return follows selected target
- Escape returns from transient overlays
- selection remains stable when rows refresh

Graph keyboard rules:

- arrow keys move between blocks
- Return follows selected outgoing edge where unambiguous
- Tab cycles visible nodes/edges
- shortcuts work in graph and assembly consistently

## Command Palette

The command palette is the fastest way to make a complex tool discoverable.

Index:

- app commands
- open documents
- functions
- symbols
- strings
- ObjC classes/selectors/methods
- C++ types/vtables
- kernel members/kexts
- recent routes
- problems
- bookmarks/tags

Behavior:

- first result page under 100 ms after warm index
- commands show keyboard shortcuts
- destructive commands require confirmation
- commands work from any focus region
- disabled commands explain missing prerequisites

## Navigator Polish

Navigator modes:

- Project
- Functions
- Symbols
- Strings
- Imports/Exports
- ObjC
- C++
- Cache
- Kernel
- Tags/Bookmarks
- Problems

Common behavior:

- fast filter field
- scope picker
- count badges
- type-specific icons
- source/image/member grouping
- stale/partial-analysis indicator
- right-click context actions
- keyboard focus and selection preservation

Do not load every row into Swift at once. Navigators must page and cache.

## Inspector Polish

Inspector modules:

- selected address/route
- instruction encoding
- operand details
- byte classification
- comments and labels
- tags/bookmarks/colors
- xrefs
- function/procedure
- basic block
- ObjC runtime object
- C++ object/type
- kernel/kext context
- source/symbolication
- problems/hints

Behavior:

- modules appear only when relevant
- expensive modules show skeleton/loading state
- module expansion persists per project
- user edits are undoable
- contradictory facts are visible

## Error And Empty States

Good reverse-engineering tools tell the truth when analysis is incomplete.

Empty states:

- no project open
- project source missing
- no analysis run
- no functions found
- no xrefs
- no ObjC metadata
- no KDK attached
- no search results

Error states:

- unsupported architecture
- malformed Mach-O
- missing split cache part
- KDK UUID mismatch
- dSYM mismatch
- analysis pass failed
- route no longer exists
- stale result version
- server unavailable

Rules:

- state what happened
- state what Damsel knows
- offer a recovery action
- do not blame the user
- do not hide details needed for debugging
- detailed logs are one click away

## Progress And Cancellation UX

Every long analysis pass should expose:

- pass name
- source/image/member
- current phase
- progress if measurable
- elapsed time
- warning count
- cancellation availability
- partial-result state

Cancellation rules:

- cancel button responds immediately
- pass stops at safe checkpoints
- partial facts are marked partial
- canceled jobs leave problems/warnings if results are incomplete
- user can resume/retry

## Animation And Motion

Use motion only for orientation:

- selection changes
- route transitions
- graph focus changes
- progress updates
- pane reveal/hide

Avoid:

- decorative motion
- large background animations
- transitions that delay navigation
- animation on every row update in huge tables

## Visual Density

Damsel is an expert tool. The UI should be quiet and dense, but not cramped.

Guidelines:

- compact rows for assembly and lists
- strong monospaced typography for code
- subtle color for semantic categories
- high contrast for current selection
- no marketing-card surfaces inside the tool
- icons for common commands
- labels where ambiguity would slow experts down

Semantic colors:

- code
- procedure
- data
- string
- ObjC
- C++
- Swift
- kernel
- relocation/fixup
- problem
- user annotation
- stale fact

Colors must remain distinguishable in light, dark, high-contrast, and reduced
transparency modes.

## Accessibility

Minimum requirements:

- all primary commands reachable by keyboard
- VoiceOver labels for toolbar, navigator, table, graph, and inspector controls
- accessible names for semantic assembly spans
- graph has list alternative
- color is not the only signal
- dynamic type strategy for panes where feasible
- high contrast supported
- reduced motion respected

Graph accessibility:

- block list alternative
- predecessor/successor lists
- current block summary
- edge labels
- keyboard movement

## System Integration

Native integrations:

- standard document open/save panels
- recent documents
- secure file bookmarks where sandboxed
- drag-and-drop binary/project/sidecar open
- Quick Look style source summary later
- Services/Share for selected text/report later
- crash reports and diagnostics export

Security/privacy:

- no telemetry by default
- diagnostics are local and redacted by default
- private strings and comments are opt-in for export
- KDK paths and binary paths can be redacted

## Responsiveness Rules

The app must remain interactive while:

- opening large dyld caches
- indexing symbols/strings
- importing KDK/DWARF
- laying out graphs
- filtering large lists
- showing xrefs
- changing selection rapidly

Rules:

- Swift main actor handles UI state only
- heavy work stays in Rust/server/background tasks
- list APIs page
- row renderers use stable identity
- search debounces and cancels stale requests
- selection changes never trigger whole-image analysis

## Product Polish Acceptance Gates

Workbench is not competitively polished until:

- project open/close/save behavior is predictable
- all common commands are in menus and palette
- back/forward route navigation works across object types
- keyboard-only navigation can complete core workflows
- annotations persist and undo/redo
- errors become recoverable states
- progress/cancel is visible for long work
- navigator, workspace, inspector, and feature map stay synchronized
- common lists remain responsive with large projects
- no primary workflow depends on reading terminal output
