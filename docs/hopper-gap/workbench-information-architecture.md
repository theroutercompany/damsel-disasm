# Workbench Information Architecture

Status: UX architecture specification for the native Damsel Workbench.

Purpose: specify how the app should be organized so Damsel can compete with
Hopper on UX, app features, polish, and responsiveness while expressing
Damsel's Apple systems niche.

Implementation companion: [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
turns this information architecture into semantic assembly, inspector, graph,
hex, feature-map, search, route-stack, local UI API, Swift model, Workbench
store, and test-gate migration steps.

Detailed implementation companion:
[Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
turns this information architecture into concrete Swift store boundaries,
command registry, route/navigation model, synchronized representation surfaces,
jobs/problems drawers, transactions, and migration phases.

Runtime browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
expands the ObjC, C++, cache, kernel, KDK, panic, and runtime graph sections
below into detailed implementation-ready browser pages, APIs, commands,
confidence states, and performance gates.

Research companion:
[Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
describes the public Hopper interaction model this IA is trying to match or
surpass in Damsel's Apple systems niche.

## Current Shape

Current Workbench:

- boots the local CLI UI server
- uploads a binary
- runs disassembly against section/symbol/address targets
- shows controls on the left
- shows an instruction table in the center
- shows instruction/reference/analysis details in the inspector

This is a good alpha shell. It is not yet the target information architecture.
The target app needs project state, navigators, multiple synchronized
representations, object routes, and persistent user annotations.

## Target App Concepts

### Project

The top-level document.

Contains:

- sources
- analysis database
- user annotations
- UI state
- analysis runs
- project settings

### Source

Any input artifact:

- Mach-O
- dyld cache root/member
- projected cache image
- kernel collection
- fileset member
- dSYM
- KDK
- panic log

### Object

Anything navigable:

- address
- instruction
- function
- block
- symbol
- string
- import/export
- xref
- graph node
- ObjC object
- C++ object
- kernel object
- cache image
- annotation

### Route

A stable UI target:

- project id
- object id
- representation
- selection
- viewport

The route model is the backbone of navigation stack, deep links, search
results, and command palette actions.

## Main Window Structure

### Toolbar

Zones:

- document controls
- navigation controls
- search/jump
- representation controls
- analysis controls
- status

Required controls:

- Open
- Save
- Analyze
- Cancel Analysis
- Back
- Forward
- Search
- Command Palette
- Representation selector
- Address mode selector
- Analysis status

### Left Navigator

The left navigator should not be a static control pane. It should be the
project map.

Sections:

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

Each section can have:

- count
- analysis status
- filter field
- sort control
- source/image/member filter
- virtualized result list

### Center Workspace

The center workspace holds representations:

- Assembly
- Graph
- Pseudocode
- Hex/Data
- Summary
- Runtime Object Detail

The workspace should support tabs or split representations later, but the first
competitive target can use a representation switcher with preserved location.

The pseudocode representation is specified in
[Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md).
Its route model must synchronize line selections with assembly, graph, hex,
source/debug info, runtime browsers, problems, and the inspector.

### Right Inspector

The inspector should be context-sensitive and route-aware.

Sections:

- Overview
- References
- Reverse References
- Runtime
- Types
- Cache/Kernel
- Comments
- Tags/Bookmarks
- Provenance
- Raw Facts

Inspector detail should be lazy. It should not fetch huge xref lists until the
section is opened.

### Bottom Utility Area

Optional collapsible bottom region:

- Analysis Progress
- Problems
- Console/Logs
- Search Results
- Script Output later

This keeps long-running analysis and warnings visible without crowding the
main inspector.

## Welcome Window

Purpose:

- immediate entry into Damsel's niche workflows

Cards/actions:

- Open Mach-O
- Open dyld Shared Cache
- Open Kernel Collection
- Open Recent Project
- Attach KDK/dSYM
- Symbolicate Panic Log
- Compare Apple OS Builds
- Open Fixture for Demo/Validation

Each action should state accepted inputs and what Damsel will do.

## Project Overview Screen

Shown after opening a source or project.

Sections:

- source summary
- architecture/platform
- analysis status
- source list
- object counts
- warnings/problems
- suggested next actions
- recent navigation

Example next actions:

- Browse Functions
- Browse ObjC Classes
- Browse Selectors
- Browse Cache Images
- Attach KDK
- Run Deep C++ Analysis
- Symbolicate Panic Log

## Function Navigation

### Function List

Columns:

- name
- address
- size/range
- source
- callers
- callees
- strings
- selectors
- confidence

Filters:

- text
- source/image/member
- has callers
- has unresolved indirects
- has ObjC refs
- has C++ refs
- has comments/bookmarks
- analysis status

Actions:

- open
- rename
- comment
- show callers
- show callees
- show graph
- export

### Function Detail

Header:

- name
- address/ranges
- source
- confidence
- analysis status

Tabs or sections:

- Assembly
- Graph
- Summary
- Xrefs
- Strings
- Runtime
- Types
- Problems

## Assembly View IA

### Row Anatomy

Columns:

- gutter
- address
- bytes/opcode optional
- label
- mnemonic/operands
- comment
- badges

Gutter:

- current line
- bookmark
- breakpoint later
- branch/flow cue
- problem marker

Badges:

- xrefs
- import
- ObjC
- C++
- kernel
- uncertain

### Interaction

- click row selects instruction
- double-click target jumps
- context menu exposes row actions
- keyboard arrows move selection
- enter follows target
- escape/backspace goes back
- space toggles graph

### Context Menu

Actions:

- Copy Address
- Copy Instruction
- Copy Row
- Rename Here
- Add Comment
- Bookmark
- Show References
- Show Reverse References
- Jump To Target
- Open In Hex
- Interpret Pointer
- Mark As Code/Data/String later

## Graph View IA

### Regions

- main graph canvas
- minimap
- selected node inspector
- graph toolbar

Toolbar:

- fit
- zoom
- center selection
- layout
- show/hide unresolved
- show/hide xref badges
- export

Node content:

- label
- address range
- compact instruction preview
- comments
- badges

Edge content:

- kind
- target
- confidence

### Synchronization

- selecting assembly row selects graph node
- selecting graph node scrolls assembly
- command palette route can open graph at block
- xref click can open graph if target is in current function

## Hex/Data View IA

### Required Modes

- file offset
- image vmaddr
- cache vmaddr
- projected image offset
- runtime slid address later

### Overlays

- current instruction bytes
- string ranges
- pointer ranges
- fixups
- ObjC metadata
- C++ metadata
- kernel table entries
- user annotations

### Inspector Integration

Selecting bytes shows:

- raw bytes
- interpreted value
- address-space mapping
- known facts
- xrefs
- comments

## ObjC Browser IA

### ObjC Home

Shows:

- class count
- protocol count
- category count
- selector count
- method count
- message-send callsite count
- unresolved message-send count

### Class View

Sections:

- overview
- superclass/subclasses
- protocols
- categories
- instance methods
- class methods
- properties
- ivars
- references
- message sends

### Selector View

Sections:

- selector name
- implementers
- callers
- selector refs
- possible dynamic dispatches
- unresolved callsites

### Method View

Sections:

- owner
- selector
- kind
- type encoding
- parsed signature
- IMP address
- function
- callers
- overrides/category context

## C++ Browser IA

### C++ Home

Shows:

- demangled symbol count
- type count
- vtable count
- typeinfo count
- virtual callsite count
- thunk count
- unresolved virtual call count

### Type View

Sections:

- demangled name
- namespaces
- bases
- derived types
- vtables
- methods
- constructors/destructors
- virtual callsites
- references

### Vtable View

Sections:

- owner candidate
- address
- typeinfo pointer
- slots
- function targets
- pointer-auth facts
- xrefs

## Kernel Browser IA

### Kernel Home

Shows:

- collection identity
- fileset members
- kext count
- KDK/dSYM status
- IOKit class count
- syscall/trap/MIG count
- warning count

### Member/Kext View

Sections:

- identity
- address ranges
- dependencies
- symbols
- functions
- IOKit classes
- strings
- problems

### Panic Symbolication View

Sections:

- input panic log
- detected slide
- frame table
- symbolicated target
- kext/member
- source line if available
- unresolved frames
- export report

## Cache Browser IA

### Cache Home

Shows:

- cache UUID
- architecture
- member count
- image count
- local symbols availability
- selected image
- analysis status

### Image View

Sections:

- identity
- sections
- symbols
- imports
- exports
- ObjC
- dependencies
- dependents
- providers/importers
- reexports
- projected analysis

### Provider/Importer View

Should answer:

- who imports this symbol?
- who provides this symbol?
- is it reexported?
- what is the resolved target image?
- can I jump to implementation?

## Search IA

### Search Result Groups

- Commands
- Addresses
- Functions
- Symbols
- Strings
- ObjC
- C++
- Kernel
- Cache
- Comments
- Problems

### Result Row

Fields:

- icon/kind
- title
- subtitle
- source/image/member
- address
- confidence
- action hint

### Ranking

Priority:

1. exact address
2. exact name
3. user labels/bookmarks
4. current source/image/member
5. high-reference-count symbols
6. fuzzy matches

## Menus

### File

- New Project
- Open
- Open Recent
- Save
- Save As
- Attach KDK/dSYM
- Import Panic Log
- Export Report
- Close Project

### Edit

- Undo
- Redo
- Copy
- Rename
- Add Comment
- Add Tag
- Bookmark
- Clear Annotation

### Navigate

- Back
- Forward
- Go To Address
- Go To Symbol
- Go To Function
- Go To Selection Target
- Show References
- Show Reverse References

### View

- Assembly
- Graph
- Hex
- Summary
- Show Navigator
- Show Inspector
- Address Mode

### Analysis

- Run Analysis
- Cancel Analysis
- Run ObjC Analysis
- Run C++ Analysis
- Run Kernel Analysis
- Rebuild Xrefs
- Show Problems

### Tools

- Symbolicate Panic
- Compare Projects
- Export Graph
- Generate Report
- Scripting later

## State Restoration

Persist:

- open project
- selected route
- open tabs
- split sizes
- visible navigator section
- inspector expanded sections
- search query where appropriate
- graph viewport
- address display mode

Do not persist:

- stale transient progress messages
- raw secret-like copied values
- temporary failed upload payloads

## Accessibility And Keyboard

Requirements:

- all primary commands reachable by keyboard
- tables expose labels and values
- graph has list fallback
- color is not the only indicator
- inspector sections have readable labels
- focus order is predictable

## Migration From Current Workbench

### Stage 1: Add Project Navigator Behind Current UI

- keep existing controls
- add project/source summary section
- add function/symbol/string list when project API exists

### Stage 2: Replace Control Pane With Navigator

- move target controls into toolbar/command palette/inspector
- left pane becomes project navigator

### Stage 3: Add Route-Based Workspace

- assembly view uses routes
- graph view uses routes
- inspector uses selected route

### Stage 4: Add Domain Browsers

- ObjC
- C++
- Kernel
- Cache

### Stage 5: Polish And Performance

- command palette
- navigation stack
- annotations
- graph minimap
- problems/progress panels
- state restoration
