# Product Requirements For A Hopper-Competitive Damsel

Status: Product requirements document for the Hopper-gap program.

Purpose: define what "competitive against Hopper" should mean for Damsel as an
Apple-focused reverse-engineering app. This document translates the broad goal
into product requirements, user journeys, acceptance criteria, and product
quality bars.

## Product Positioning

Damsel should become an Apple systems reverse-engineering workbench. It should
compete with Hopper in everyday reverse-engineering UX while differentiating on
Apple-specific depth.

Competitive axis:

- Hopper-like polish and speed for core reverse-engineering loops.
- Damsel-specific Apple systems intelligence.
- Trustworthy provenance and confidence for inferred facts.
- Excellent dyld shared-cache, Objective-C, C++, IOKit, and XNU workflows.

Supporting detail:

- [Native App Polish Plan](native-app-polish-plan.md) defines the product feel
  required for the macOS app to be competitive in daily use.
- [Apple Systems Specialization Plan](apple-systems-specialization-plan.md)
  defines the Apple-specific product advantage.
- [Analysis Database Blueprint](analysis-database-blueprint.md) defines the
  durable project model needed to make annotations, navigation, and
  responsiveness real.
- [Decompiler And IR Strategy](decompiler-ir-strategy.md) defines when and how
  pseudocode becomes a product feature rather than a misleading renderer.
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)
  defines safe product boundaries for scripting, AI/tool access, debugging, and
  future patching.

Non-positioning:

- Damsel is not trying to be a universal PE/ELF/firmware tool first.
- Damsel is not trying to ship risky binary patching before project persistence,
  undo, and read-only analysis are excellent.
- Damsel is not trying to make pseudocode claims before it has robust function,
  xref, type, and runtime models.

## Primary Users

### Apple Platform Reverse Engineer

Needs:

- inspect macOS/iOS apps and frameworks
- understand Objective-C classes/selectors/methods
- follow framework imports across dyld shared cache
- browse strings, symbols, xrefs, and graphs
- annotate findings
- export reports

Competitive requirement:

- Damsel must feel at least as fast and navigable as Hopper for opening,
  searching, jumping, and annotating Apple binaries.

### Apple Systems Researcher

Needs:

- inspect dyld shared caches
- compare OS versions
- inspect system frameworks
- map cross-image dependencies
- reason about arm64e pointer-auth and chained fixups
- keep provenance for every address and symbol source

Competitive requirement:

- Damsel should beat generic disassemblers on cache-aware navigation and
  Apple-specific address provenance.

### XNU / Kernel Researcher

Needs:

- inspect kernel collections and fileset members
- attach KDK/dSYM symbols
- browse kexts and dependencies
- recover IOKit C++ classes
- inspect syscall/Mach trap/MIG tables
- symbolicate panic logs

Competitive requirement:

- Damsel should make kernel collection and KDK workflows first-class, not an
  afterthought.

### Tool Builder / Automation User

Needs:

- script analysis
- batch symbolicate
- export xrefs and graphs
- generate reports
- compare versions
- query stable JSON contracts

Competitive requirement:

- Damsel should expose deterministic, documented APIs before exposing broad
  plugin mutation hooks.

## Product Pillars

### Pillar 1: Open And Orient Quickly

Requirement:

- Opening a source should quickly produce a truthful overview.

Overview should include:

- source type
- architecture
- platform/build hints
- sections/segments
- symbols/imports/exports
- strings
- ObjC availability
- C++ availability
- cache/kernel provenance
- analysis progress
- warnings/problems

This is a UX requirement and an architecture requirement. The app cannot wait
for full analysis before showing useful orientation.

### Pillar 2: Navigate Anything

Requirement:

- Every meaningful object should be linkable and jumpable.

Objects:

- address
- instruction
- data item
- string
- symbol
- import/export
- function
- basic block
- graph node
- ObjC class/protocol/category/selector/method/property/ivar
- C++ type/vtable/typeinfo/method/virtual call
- dyld cache image/provider/importer/reexport
- kernel collection member/kext/syscall/trap/MIG routine
- panic frame
- user annotation

### Pillar 3: Search Across The Project

Requirement:

- Search should cross object boundaries.

Search scopes:

- all
- functions
- symbols
- strings
- ObjC
- C++
- kernel
- cache
- comments/tags
- problems

Result requirements:

- grouped by kind
- fuzzy matching
- exact address handling
- stable sort
- source/image/member context
- jump action
- confidence/provenance where relevant

### Pillar 4: Explain Why

Requirement:

- Inferred facts should show provenance and confidence.

Examples:

- "method target inferred from selector in x1 and receiver class in x0"
- "vtable target inferred from slot offset and typeinfo"
- "kernel symbol from KDK dSYM UUID match"
- "cache provider resolved through reexport chain"
- "pointer target stripped from arm64e authenticated pointer"

The app should make uncertain facts useful without pretending they are exact.

### Pillar 5: Stay Responsive

Requirement:

- UI remains interactive during analysis.

The app must support:

- background analysis jobs
- cancellation
- partial results
- paged lists
- virtualized rows
- progress reporting
- lazy inspector loading
- graph layout cache

### Pillar 6: Preserve User Work

Requirement:

- User annotations persist and are undoable.

User work:

- comments
- labels
- tags
- bookmarks
- colors
- manual refs
- manual function bounds
- manual type assignments
- notes/reports

### Pillar 7: Apple Niche First

Requirement:

- Apple-specific workflows are visible and optimized.

Niche features:

- dyld cache browser
- projected cache image analysis
- provider/importer/reexport jumps
- ObjC browser
- C++/IOKit browser
- kernel collection browser
- KDK/dSYM association
- panic symbolication
- arm64e pointer-auth interpretation
- OS build comparison

## Competitive Feature Requirements

### P0: Required For Serious Hopper-Competitive UX

- Project create/open/save/reopen.
- Persistent analysis database.
- Source registry.
- Background analysis scheduler.
- Function list.
- Symbol list.
- String list.
- Linear assembly view.
- Function CFG view.
- Inspector with refs/xrefs/provenance.
- Global search.
- Navigation stack.
- Comments/labels/bookmarks/tags.
- Xref and reverse-xref indexes.
- ObjC class/selector/method browser.
- Paged APIs and virtualized UI lists.
- Analysis progress and cancellation.
- Problems panel.
- Performance telemetry.

### P1: Required For Damsel's Apple Niche

- dyld shared-cache browser.
- Cross-image import/provider navigation.
- ObjC message-send target analysis.
- C++ demangling and symbol classification.
- C++ vtable/typeinfo discovery.
- IOKit class recovery.
- Kernel collection/fileset loader.
- KDK/dSYM matching.
- Kernel/kext browser.
- Panic symbolication.
- arm64e pointer-auth facts.
- Chained-fixup materialization.
- Project comparison for OS builds.

### P2: Required For Long-Term Hopper Parity

- Decompiler-lite function summaries.
- Low-level IR.
- Pseudocode view.
- Debugger bridge.
- Read-only scripting.
- Plugin API.
- Binary patching with explicit write mode.
- Collaboration/export packages.
- AI-assisted analysis notes, if product direction wants it, using local facts
  and without leaking binaries.

## Product Requirements By Surface

### Welcome Window

Must provide:

- recent projects
- open file
- open dyld shared cache
- open kernel collection
- attach KDK/dSYM
- symbolicate panic log
- documentation/help links
- current validation/support status

Must avoid:

- marketing page feel
- blank state with no action
- hiding Apple-system workflows behind advanced menus

### Project Overview

Must show:

- source list
- architecture/platform
- analysis status
- object counts
- warning/problem count
- cache/kernel membership where relevant
- KDK/dSYM status
- recently visited objects

### Assembly Surface

Must show:

- address
- bytes/opcode optional
- labels
- mnemonic/operands
- inline comments
- xref indicators
- branch/jump cues
- selection
- search highlights
- confidence or unresolved badges when needed

Must support:

- copy address/instruction/row/function
- rename
- comment
- bookmark
- show xrefs
- jump target
- switch representation

### CFG Surface

Must show:

- basic blocks
- edges
- selected block
- minimap
- labels/comments/colors
- unresolved edges
- xref overlays

Must support:

- pan/zoom
- keyboard navigation
- jump to block
- group/collapse nodes later
- export

### Hex/Data Surface

Must show:

- bytes
- file offset
- vmaddr/image/cache offset modes
- current instruction byte highlight
- strings
- pointers
- fixups
- data item interpretation

Must support:

- read-only by default
- copy bytes
- interpret as type
- jump to referenced target

### Inspector

Must adapt to:

- instruction
- function
- data
- string
- graph node
- ObjC object
- C++ object
- kernel object
- cache object
- problem

Must include:

- summary
- refs
- reverse refs
- comments
- names
- types
- runtime
- provenance
- raw facts behind disclosure

### Search And Command Palette

Must support:

- fuzzy search
- exact address
- command search
- recent targets
- grouped results
- keyboard-only navigation
- quick filters

### ObjC Browser

Must support:

- classes
- protocols
- categories
- selectors
- methods
- properties
- ivars
- implementers
- callers
- message-send callsites
- type encodings

### C++ Browser

Must support:

- demangled symbols
- types
- vtables
- typeinfo
- constructors
- destructors
- thunks
- virtual callsites
- inheritance graph

### Kernel Browser

Must support:

- kernel collection overview
- fileset members
- kexts
- dependencies
- KDK/dSYM status
- IOKit classes
- syscall table
- Mach traps
- MIG routines
- panic symbolication

### Cache Browser

Must support:

- cache members
- images
- dependencies
- dependents
- providers
- importers
- reexports
- projected image analysis
- cross-image xrefs

## Competitive Quality Requirements

### UX Quality

- Common actions available from keyboard and context menu.
- No dead-end views.
- Every object has a route.
- Every route has a back/forward entry.
- Error states are actionable.
- Empty states name the next action.
- Advanced facts are collapsible, not hidden.

### Visual Quality

- Dense but readable layout.
- Stable row heights.
- Monospaced code surfaces.
- Native macOS tables and split views.
- Clear selected/current target.
- No text clipping in compact panes.
- No overlapping controls.
- Consistent iconography.
- Confidence/provenance badges restrained.

### Performance Quality

- No UI freeze during analysis.
- Paged large lists.
- Cancellable analysis.
- Fast indexed search.
- Smooth instruction scrolling.
- Background graph layout.
- Measured performance gates.

### Trust Quality

- Fact provenance visible.
- Inference confidence visible.
- Ambiguity preserved.
- Real-cache/kernel validation not overclaimed.
- KDK/dSYM mismatch explicit.

## Product Metrics

### Activation Metrics

- time to first metadata
- time to first instruction
- time to first function list
- time to first xref query
- time to first ObjC class list

### Navigation Metrics

- median search response time
- median jump response time
- selection-to-inspector latency
- back/forward latency

### Analysis Metrics

- functions discovered
- xrefs discovered
- unresolved edge count
- ObjC classes/selectors/methods discovered
- C++ vtables/typeinfo discovered
- kernel members/kexts discovered
- warnings/problems count

### Quality Metrics

- crash-free app smoke
- no layout overlap in core viewports
- no main-thread freeze during fixture analysis
- snapshot stability
- real-cache evidence matrix coverage

## Explicit Non-Goals For Near Term

- generic PE/ELF parity
- every CPU architecture
- patching as default workflow
- live debugger before project/address-space model
- cloud upload of private binaries
- full recompilable decompiler output
- jailbreak-only acquisition workflows

## Product Completion Definition

The Hopper-competitive product track is not complete until:

- users can create/reopen projects
- analysis persists
- function/xref/search/navigation are durable
- Workbench supports assembly, graph, inspector, and core navigators
- ObjC browser is first-class
- performance remains responsive on representative inputs
- dyld cache and kernel collection workflows have first-class entrypoints
- docs and validation prove what works and what remains inferred or unverified
