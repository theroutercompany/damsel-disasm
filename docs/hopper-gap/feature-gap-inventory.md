# Feature Gap Inventory

Status: Planning and research. This document maps Damsel's current foundation
to the missing feature families needed for Hopper-adjacent Apple systems
analysis.

## Executive Summary

Damsel has the right low-level direction: Apple-only, Mach-O-first, arm64 and
arm64e, dyld shared-cache aware, ObjC metadata aware, and already capable of
structured disassembly analysis. The missing layer is not "more disassembly" in
the narrow sense. The missing layer is a durable reverse-engineering workspace:
automatic analysis, persistent xrefs, whole-image function recovery, graph
navigation, language/runtime reconstruction, kernelcache support, and enough
type/value-flow infrastructure to make eventual pseudocode plausible.

The biggest gaps are:

1. Persistent analysis database.
2. Whole-image automatic analysis and byte classification.
3. Function discovery and function-boundary recovery.
4. Cross-reference and reverse-reference indexes.
5. Interactive graph navigation.
6. Objective-C dispatch-aware exploration.
7. C++ and Objective-C++ ABI recovery.
8. XNU kernelcache/kernel-collection and KDK/dSYM workflows.
9. arm64e pointer-auth and chained-fixup semantics.
10. Type, DWARF, compact-unwind, and source-symbol integration.
11. Decompiler-grade IR and pseudocode.
12. Debugger, scripting, and patching surfaces.
13. Hopper-style document primitives: feature map, undoable user edits,
    procedure-local names, type database, problem list, and document automation
    APIs.
14. Semantic editing and reanalysis-hint layer: comments, labels, tags, manual
    references, byte/procedure/type/decompiler hints, Apple runtime bindings,
    suggestions, conflicts, stale-route repair, and privacy-aware exports.
15. Native preference and workspace personalization layer: app/project/session
    settings, keymaps, themes, layout restore, analysis profiles, Apple
    artifact source roots, automation privacy gates, and diagnostics presets.
16. Swift runtime metadata and Swift-Objective-C bridge layer: Swift section
    inventory, demangled-name facts, nominal types, fields, protocols,
    conformances, metadata accessors, witness tables, runtime calls, ObjC
    bridge edges, Swift browser UX, and honest static/dynamic evidence split.

## Current Implemented Strengths

### Standalone Mach-O

Current Damsel can parse standalone Mach-O images and expose structured views
for sections, symbols, imports, relocations, dyld metadata, ObjC metadata, and
AArch64 disassembly. That is already a strong base for a focused Apple binary
analysis tool.

Missing relative to Hopper:

- automatic byte typing across the whole image
- persistent project-level names, comments, types, and manual changes
- route-aware semantic edit transactions with reanalysis impact records
- complete function list and function boundaries
- strings list as a first-class navigation pane
- data item creation and reinterpretation
- user-created labels and references
- command/UI workflows for reverse xrefs

### dyld Shared Cache

Damsel already treats the dyld shared cache as a first-class container. It can
inspect cache members, list images, resolve cache addresses, resolve symbols,
project selected cache images into per-image analysis, and query dependencies,
dependents, symbol providers, symbol importers, and reexports.

Missing relative to a Hopper-like Apple systems workflow:

- cache-wide function and xref index
- cross-image call graph
- "follow import to provider implementation" as a primary UI gesture
- cache-wide class/selector/method index
- cache-wide string/data search
- per-cache project database that avoids redoing expensive analysis
- symbolication profiles for multiple OS versions
- dyld closure/prebuilt-loader metadata, where available and useful
- shared-cache relation graph visualization

### Disassembly Analysis

Damsel already records basic blocks, edges, calls, branches, imports, ObjC
references, recovered values, jump tables, indirect controls, cache links, and
summary metrics for a decoded window.

Missing relative to Hopper:

- whole-function graph, not just local decode-window graph
- graph layout and interactive CFG
- persistent path-aware value-flow facts
- reverse edge queries
- interprocedural call graph
- symbolic stack frame, arguments, local variables, and return values
- switch/case correction hints and manual recovery controls
- stable function summary object independent of a single CLI request

### Objective-C Metadata

Damsel already extracts class, protocol, category, method, property, ivar,
selector, and pointer-ref records. That is the most important local advantage
for Hopper-adjacent Apple work.

Missing relative to Hopper:

- class hierarchy browser
- selector browser with implementers and callers
- category overlays on class views
- method type-encoding decoder
- ivar offset/type explorer
- property attribute decoder
- `objc_msgSend` receiver/selector resolution
- super dispatch resolution
- block invocation metadata and captured variables
- manual method binding when static analysis cannot prove the receiver
- xrefs from metadata to code and from code back to metadata

### Swift Runtime Metadata

Damsel can resolve executable Swift symbols by exact mangled name or by
demangled query when `swift-demangle` is available. That is useful, but it is
not yet Hopper-competitive Swift analysis.

Missing relative to Hopper's public Objective-C and Swift positioning:

- Swift section inventory as a first-class browser surface
- durable Swift metadata model beside `ObjcMetadata`
- demangling as project evidence with explicit failure records
- Swift modules, nominal types, fields, protocols, and conformances
- metadata accessor and witness table classification
- Swift runtime call classification
- Swift-Objective-C bridge edges
- Swift metadata problem browser
- dyld shared-cache projected-image Swift routes
- large Swift-name batch performance budgets
- debugger-time runtime metadata observations separated from static facts

### Native macOS Workbench

Damsel Workbench gives the project a Hopper-like native macOS shell path. It is
currently a sibling app over the existing CLI UI server, which is the right
architecture because it keeps decoder logic in Rust.

Missing relative to Hopper:

- persistent project browser
- left-pane symbols/classes/strings/functions/references
- interactive CFG canvas
- xref list and referrer list
- search across the analysis database
- comments, labels, tags, color markings
- multiple synchronized representations
- project save/load
- long-running auto-analysis progress and cancellation

### Hopper-Style Document Primitives

Hopper's public tutorial and SDK surface make several "small" interactive
features look foundational: byte type changes, feature-map navigation,
procedure-local labels, register aliases, stack-variable names, signatures,
operand display formats, manual references, tags, colors, undo/redo, problem
lists, background-process control, and scripting/MCP access to the current
document. These should not be treated as late UI sugar.

Missing relative to Hopper:

- byte-classification map with inferred/user provenance
- manual make-code/make-data/make-string/make-procedure operations
- feature map/navigation bar keyed by byte classifications
- annotation transaction model with undo/redo
- procedure-local labels and register aliases
- stack-frame variables and signature/calling-convention model
- project type database for structs, unions, enums, pointers, arrays, and
  imported debug/KDK types
- problem list for ambiguous analysis and user-fixable issues
- stable document/object API for scripts, plugins, and future MCP tools

## Gap 1: Persistent Analysis Database

### Why It Matters

Hopper behaves like a workspace. The user opens a binary, Hopper analyzes it,
and every later view draws from the accumulated project state. Damsel today is
closer to a set of precise commands over a loaded file or cache. That is good
for correctness, but it caps UX: xrefs, comments, graph navigation, and
pseudocode all need a durable database.

### Required Capabilities

- Address-space model keyed by image provenance.
- Byte classification: unknown, header, load command, code, procedure, data,
  string, ObjC metadata, C++ metadata, relocation/fixup, debug info.
- Analysis facts with provenance and confidence.
- Function table with starts, extents, blocks, edges, names, and discovered
  source.
- Reference table with forward and reverse indexes.
- Symbol/name table with source priority.
- String table and data item table.
- Runtime-object tables for ObjC, Swift placeholders, C++ ABI, IOKit, kernel.
- User edits: labels, comments, manual refs, manual function bounds, tags.
- Cache/project provenance: root cache UUID, member, image id, base address,
  OS/build hints, KDK/dSYM association.
- Incremental invalidation when inputs, settings, or analyzers change.

### Existing Pieces To Reuse

- `BinaryImage` as a loaded image snapshot.
- `SharedCache` and `ProjectedImageProvenance` for cache identity.
- `DisassemblyAnalysis` as the seed for function and graph facts.
- `ObjcMetadata` as runtime object seed data.
- Existing deterministic JSON style for CLI contracts.

### Main Design Choice

Start with an in-memory database plus deterministic JSON serialization before
choosing SQLite or a custom binary store. Damsel needs model stability first.
Once the shape settles, large real-cache performance will likely require a
memory-mapped or SQLite-backed store.

## Gap 2: Whole-Image Automatic Analysis

### Why It Matters

Hopper automatically classifies bytes and recovers procedures after loading.
Without auto-analysis, Damsel remains a precise point-query tool. The Hopper
gap narrows only when the user can open a binary and immediately browse what
Damsel has inferred.

### Required Analyzers

- Load-command and section classifier.
- String scanner for `__cstring`, CF strings, Objective-C method names,
  selector refs, Swift strings, and kernel constant strings.
- Function seed collector from entrypoint, exports, symbol table,
  `LC_FUNCTION_STARTS`, ObjC IMPs, C++ vtables, unwind info, stubs, and
  discovered branch/call targets.
- Recursive-descent disassembler for executable sections.
- Linear sweep fallback for orphan executable ranges.
- Data-in-code handling.
- Fixup and relocation materialization.
- ObjC metadata parser.
- C++ ABI metadata parser.
- Shared-cache import/export relation builder.
- Kernel collection/fileset member extractor.
- XNU semantic recognizers.

### Correctness Rules

- Every inferred fact must have source and confidence.
- Analyzer passes must be idempotent.
- Unknown bytes are acceptable. False certainty is not.
- Manual user overrides must outrank inferred facts but preserve the original
  inferred facts for audit.
- The database must track analyzer version so stale facts can be invalidated.

## Gap 3: Function Discovery And Procedure Model

### Why It Matters

Hopper's assembly, CFG, pseudocode, procedure inspector, and navigation stack
all depend on procedures as durable objects. Damsel has local block/edge
analysis but needs whole-function recovery.

### Required Function Model

Fields:

- stable function id
- image id and address range set
- primary start address
- discovered entry addresses
- names with source and priority
- blocks
- edges
- callers
- callees
- tail calls
- imports called
- ObjC selectors/classes referenced
- C++ vtables/typeinfo referenced
- strings/data referenced
- stack frame summary
- argument summary
- return summary
- confidence and unresolved reasons

### Discovery Sources

- symbols
- export trie
- function starts
- entrypoint load commands
- section starts
- ObjC method IMPs
- category method IMPs
- protocol default implementations where present
- C++ constructor/destructor symbols
- vtable entries
- unwind info
- direct branch/call targets
- jump table targets
- stub helpers
- kernel trap/syscall tables

### Hard Cases

- tail calls between adjacent functions
- thunk islands
- functions split by cold blocks
- exception landing pads
- data embedded in code
- stripped symbols
- arm64e authenticated branches
- compact unwind entries that name function regions but not semantics
- ObjC IMPs that share implementations
- C++ thunks that adjust `this`

## Gap 4: Xrefs And Reverse Xrefs

### Why It Matters

Reverse engineering is mostly "what references this?" Hopper exposes references
and reverse references at the instruction, data, and procedure level. Damsel's
current analysis records references for a decoded window; the missing piece is
global indexing.

### Required Reference Kinds

- code call
- code branch
- fallthrough
- data load/store
- page/address materialization
- import binding
- stub call
- chained fixup
- rebase
- symbol table association
- export association
- Objective-C selector ref
- Objective-C class ref
- Objective-C method IMP
- Objective-C message-send inferred target
- C++ vtable slot
- C++ typeinfo reference
- exception/unwind reference
- string reference
- kernel table entry
- user manual reference

### Query Requirements

- references from address
- references to address
- references from function
- references to function
- references to symbol
- references to ObjC selector/class/method
- references to C++ type/vtable
- references crossing image boundaries
- references crossing cache image boundaries
- references introduced by user edits

## Gap 5: Graph Navigation

### Why It Matters

Damsel has graph facts; Hopper has graph navigation. The gap is a product and
data-model issue, not only a rendering issue.

### Required Graphs

- function CFG
- function call graph
- image import/export graph
- dyld shared-cache provider/importer/reexport graph
- ObjC class hierarchy graph
- ObjC selector implementer/caller graph
- C++ inheritance/vtable graph
- XNU kext dependency graph
- XNU trap/syscall/MIG handler graph

### UI Expectations

- graph canvas for current function
- minimap
- current block highlighting
- keyboard navigation among blocks
- jump stack
- clickable edges and targets
- inline comments and labels
- graph node color/tag support
- list fallback for accessibility and large graphs
- graceful degradation for enormous cache-wide graphs

## Gap 6: Objective-C Deep Recovery

### Why It Matters

Objective-C is the Apple userland reverse-engineering multiplier. Selectors and
method names often survive stripping, and message dispatch exposes semantic
intent even when C symbols are gone.

### Required Features

- Class browser.
- Protocol browser.
- Category browser.
- Selector browser.
- Method browser.
- Class hierarchy.
- Category overlay view.
- Implementers for selector.
- Callers of selector.
- Method IMP disassembly.
- Method type-encoding decode.
- Ivar offsets and types.
- Property attributes.
- ObjC message-send analysis.
- ObjC block analysis.
- Tagged runtime metadata provenance.

### Message-Send Recovery

For `objc_msgSend(receiver, selector, ...)`, Damsel needs to recover:

- callsite address
- selector value and selector string
- receiver value if statically known
- receiver class if statically known
- possible target methods
- argument registers and stack arguments
- return use
- confidence
- unresolved reason

Receiver recovery can use:

- class refs loaded into `x0`
- allocation/init idioms
- previous return from known factory methods
- ivar/property access patterns
- `objc_msgSendSuper` metadata
- local variable provenance
- manual user binding

### Hard Cases

- dynamic selectors
- swizzling
- categories overriding methods
- `forwardInvocation:`
- `NSInvocation`
- `performSelector:`
- blocks and IMP trampolines
- class clusters
- CoreFoundation toll-free bridged classes

## Gap 7: C++ And Objective-C++ Recovery

### Why It Matters

XNU and IOKit are C++ heavy. Objective-C++ apps and frameworks mix ObjC message
dispatch with C++ object layout. Hopper-distance for XNU-style binaries depends
on C++ ABI recovery.

### Required Features

- Itanium C++ demangling.
- Symbol classification for constructors, destructors, thunks, operators, and
  templates.
- Vtable discovery.
- RTTI/typeinfo discovery.
- Class hierarchy recovery from vtable/typeinfo.
- Virtual callsite resolution.
- `this` adjustment detection.
- Constructor/destructor call graph.
- Exception table parsing.
- Compact unwind and DWARF integration.
- IOKit `OSMetaClass` and `OSObject` idiom recovery.

### Hard Cases

- stripped RTTI
- vtables in const segments with pointer auth
- construction vtables
- multiple inheritance
- virtual inheritance
- thunks that adjust `this`
- inlined constructors/destructors
- LTO devirtualization
- kernel-specific runtime constraints

## Gap 8: XNU Kernelcache And KDK Workflows

### Why It Matters

The user's focus explicitly includes XNU-style system binaries. That is not a
normal Mach-O app workflow. Damsel needs container models, symbol sources, and
semantic recognizers for kernels, kernel collections, and kexts.

### Required Features

- Kernelcache/kernel-collection loader.
- `MH_FILESET` support.
- `LC_FILESET_ENTRY` member extraction.
- Kext identity model.
- Kext dependency graph.
- KDK/dSYM symbol ingestion.
- DWARF type ingestion for kernels where available.
- Panic/backtrace symbolication.
- KASLR slide handling.
- arm64e kernel pointer format handling.
- IOKit class recovery.
- Syscall table recognition.
- Mach trap table recognition.
- MIG subsystem and routine recovery.
- Kernel string/xref search.
- Kext-to-source package hints when XNU source paths align.

### Hard Cases

- OS-version variation.
- KDK availability and exact build matching.
- stripped release kernels.
- split collections.
- compressed or packaged kernel artifacts.
- pointer-authenticated pointers.
- closed-source Apple kexts.
- source release lag behind shipping OS builds.

## Gap 9: Pointer Authentication And Chained Fixups

### Why It Matters

arm64e is a core Apple systems reality. Signed pointers affect ObjC metadata,
C++ vtables, function pointers, chained fixups, kernel cache pointers, and
debug/symbolication workflows.

### Required Features

- Pointer-auth aware relocation/fixup model.
- Representation of raw, signed, stripped, authenticated, and unresolved
  pointer states.
- Pointer format classification for userland and kernel cache formats.
- Display of key/discriminator/storage provenance where available.
- Conservative stripping for static analysis with explicit confidence labels.
- Avoid treating high bits as ordinary address bits.
- Integration with ObjC IMPs, C++ vtables, and kernel table entries.

## Gap 10: Types, DWARF, Unwind, And Source Context

### Why It Matters

Pseudocode quality, function signatures, C++ recovery, kernel analysis, and
panic symbolication all improve dramatically when debug info and unwind data are
available.

### Required Features

- dSYM association by UUID.
- DWARF line, function, type, variable, and inline info.
- Compact unwind parsing.
- EH frame parsing where present.
- Function frame recovery.
- Stack variable model.
- Argument and return model.
- Source path and line display when available.
- KDK symbol pack ingestion.
- Symbol source priority and conflict handling.

## Gap 11: Decompiler-Grade IR

### Why It Matters

Hopper's pseudocode is a major feature, but decompiler work is expensive and
depends on all preceding analysis. Starting here would be strategically
backwards.

### Required Layers

- Instruction semantics.
- Low-level IR.
- SSA.
- Memory model.
- Stack model.
- Type propagation.
- Call signature inference.
- Structured control-flow recovery.
- ObjC message expression rendering.
- C++ virtual-call expression rendering.
- Confidence and unresolved annotations.

### First Useful Product

The first user-facing product should be "explain this function" and structured
summaries, not full recompilable C. Examples:

- arguments likely used
- imports called
- selectors messaged
- classes touched
- strings referenced
- global data touched
- switch cases
- callees and callers
- possible return constants
- unresolved branches and why

## Gap 12: Debugger, Scripting, SDK, And Patching

### Why It Matters

Hopper includes debugger, scripting, SDK, and patching capabilities. Damsel can
approach the workflow without implementing all of them immediately.

### Recommended Priority

1. Scripting/read-only automation first.
2. LLDB bridge for local debugging second.
3. Plugin API only after internal models are stable.
4. Patching last, behind an explicit policy decision.

### Read-Only Automation Scope

- run analysis
- query facts
- export graph
- apply labels/comments/tags
- batch symbolicate addresses
- generate reports
- compare two OS builds/caches

### Debugger Scope

- load target under LLDB
- attach to local process
- set breakpoints
- read registers/memory
- map live addresses to analysis database
- annotate current instruction/function
- capture traces for offline analysis

### Patching Scope

Patching should not be in the first Apple systems push. If added later, it
needs:

- explicit write mode
- byte-level diff preview
- instruction assembler integration
- code-signing consequences documented
- undo/redo
- no accidental shared-cache/kernel mutation
