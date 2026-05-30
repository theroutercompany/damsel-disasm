# Target Architecture

Status: Architecture proposal. Names are intentionally concrete enough to
implement, but not yet API commitments.

Purpose: describe how Damsel can evolve from a precise Mach-O and shared-cache
analysis tool into a Hopper-adjacent Apple systems analysis workspace without
discarding the current implementation.

## Architectural Thesis

The right architecture is a layered analysis database over the current loader
and disassembler stack.

Current Damsel already has loaders, parsers, and command surfaces:

- `damsel-macho` loads Mach-O images and dyld shared caches.
- `damsel-core` owns typed analysis models.
- `damsel-cli` exposes stable text and JSON commands.
- `damsel-cli ui` exposes HTTP APIs.
- Damsel Workbench wraps the existing UI server as a native macOS app.

The Hopper-gap architecture should add:

- project/session database
- analysis scheduler
- fact/provenance system
- whole-image function and xref indexes
- language/runtime analyzers
- kernel container analyzers
- graph service
- type/symbol service
- UI navigation model

It should not move binary parsing into the Swift app. It should not turn the CLI
into a thin wrapper over UI state. Rust remains the source of truth for binary
analysis.

Companion blueprints:

- [Analysis Database Blueprint](analysis-database-blueprint.md) defines the
  concrete storage, indexing, invalidation, provenance, migration, and
  transaction model behind this architecture.
- [Native App Polish Plan](native-app-polish-plan.md) defines the macOS
  document, menu, keyboard, command-palette, progress, accessibility, and
  interaction quality required above these APIs.
- [Apple Systems Specialization Plan](apple-systems-specialization-plan.md)
  defines the dyld-cache, driver-cache, kernel-collection, ObjC, C++, Swift,
  arm64e, KDK, and panic workflows that give Damsel its niche.
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
  defines the native Workbench browser suite that turns Apple runtime facts,
  cache objects, C++/IOKit evidence, KDK state, XNU interface recognizers, and
  panic routes into paged, route-backed, command-integrated app workflows.
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
  defines how raw files, directories, Apple containers, extracted assets,
  sidecars, and panic logs become durable project sources, projections,
  manifests, and analysis jobs.
- [End-To-End Workflow Playbooks](end-to-end-workflow-playbooks.md) defines the
  user-visible sessions this architecture must make smooth.
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md) defines
  the route/object/search/index layer connecting all views.
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
  defines the graph, feature-map, hex/data, layout, route-selection,
  accessibility, export, and cross-view synchronization layer that makes those
  routes feel like one Hopper-class Workbench surface.
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
  defines the typed command registry, menu/keyboard/palette routing,
  route-backed focus and selection, navigation history, undoable project
  transactions, activity logs, and Apple-system command families above the
  project/session architecture.
- [Type And Symbolication System Plan](type-symbolication-system-plan.md)
  defines the name/type/symbolication system that feeds function summaries,
  inspectors, runtime browsers, and kernel workflows.
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
  defines the concrete type records, signature/calling-convention store,
  local variable model, debug-info importers, Workbench editors, and CLI/API
  contracts needed to make that system real.
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
  defines the dedicated debug-source, dSYM/KDK matching, DWARF line/function,
  inline callsite, variable location, source route, privacy, and symbolication
  profile architecture needed by Apple-system workflows.
- [Decompiler And IR Strategy](decompiler-ir-strategy.md) defines the staged
  path from summaries to low-level IR, SSA/data-flow, control recovery, and
  uncertainty-aware pseudocode.
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
  defines the Workbench-facing decompiler representation layer: pseudocode
  routes, line/token/source-map contracts, confidence and issue display, user
  correction transactions, Apple runtime overlays, decompiler jobs/caches, and
  export/evidence flows.
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)
  defines the safe route for scripting, MCP/AI, plugins/SDK boundaries, LLDB
  bridge work, and future binary mutation.
- [Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
  defines the native runtime-overlay architecture for launch/attach workflows,
  debug targets/sessions/modules, route-backed breakpoints, stop-event UI,
  registers, frames, memory windows, LLDB helper isolation, permissions,
  diagnostics, and Apple-runtime interpretation.
- [Current Code Integration Map](current-code-integration-map.md) maps this
  architecture onto today's `damsel-core`, `damsel-macho`, `damsel-cli`, and
  Workbench implementation.

## Layer Diagram

```text
Input sources
  standalone Mach-O
  universal Mach-O
  dyld shared cache members
  projected shared-cache images
  iOS/macOS extracted artifacts
  kernelcache / kernel collection
  KDK / dSYM / DWARF / symbol sidecars
        |
        v
Source registry
  immutable source identity
  content hashes
  UUID/build metadata
  cache/kcollection membership
        |
        v
Loaders and parsers
  existing Mach-O loader
  existing shared-cache loader
  new fileset/kernel loader
  new debug-info loader
        |
        v
Analysis database
  address spaces
  byte classifications
  functions and blocks
  refs and xrefs
  symbols and names
  strings and data items
  ObjC runtime objects
  C++ ABI objects
  XNU semantic objects
  user annotations
        |
        v
Analysis passes
  function discovery
  recursive descent
  xref indexing
  ObjC message analysis
  C++ vtable/typeinfo analysis
  kernel semantic recognizers
  type and DWARF enrichment
  graph composition
        |
        v
Presentation services
  CLI commands
  JSON APIs
  web UI API
  native workbench state
  report/export tools
```

## Core Principle: Facts With Provenance

Every derived record should be represented as a fact:

```text
Fact {
  id
  kind
  subject
  payload
  source
  confidence
  analyzer
  created_at_analysis_generation
}
```

Example fact sources:

- Mach-O header
- load command
- section record
- symbol table
- export trie
- bind opcode
- chained fixup
- function starts
- Capstone decode
- ObjC runtime metadata
- C++ ABI metadata
- compact unwind
- DWARF
- KDK symbol
- recursive-descent inference
- value-flow inference
- user label
- user manual reference

Why this matters:

- Hopper-like tools often display inferred information confidently. Damsel
  should be better at explaining why it thinks something.
- Apple system binaries combine weak and strong evidence. A KDK DWARF function
  name, an export trie symbol, and a branch target are different evidence.
- arm64e and kernelcache analysis may need to strip or reinterpret pointers. The
  raw pointer and interpreted target both need to survive.

## Address-Space Model

The current Damsel model has image-local and cache-level address vocabulary.
The new database should preserve that precision.

Required address identities:

- source file offset
- Mach-O image vmaddr
- dyld cache vmaddr
- projected image vmaddr
- kernel collection global vmaddr
- fileset member vmaddr
- slid runtime address
- unslid runtime address
- debug-info address

Suggested model:

```text
AddressSpace {
  id
  kind: standalone_image | shared_cache | projected_cache_image |
        kernel_collection | fileset_member | debug_sidecar
  source_id
  base_vmaddr
  size
  parent_space_id?
  transform_to_parent?
}

Address {
  space_id
  value
}
```

The main rule: never collapse cache addresses, image-local addresses, and slid
runtime addresses into a single `u64` without provenance.

## Source Registry

The source registry is the immutable input layer.

Source types:

- standalone file
- universal slice
- dyld shared-cache root
- dyld subcache member
- dyld `.symbols` sidecar
- projected cache image
- kernelcache/kernel collection
- fileset member
- dSYM
- KDK bundle
- extracted IPSW artifact
- user-created project database

Fields:

- source id
- canonical path if file-backed
- content hash
- file size
- UUIDs
- architecture
- platform
- OS/build hints
- source role
- parent source id
- extraction provenance

The registry lets Damsel answer questions like:

- Which cache image did this function come from?
- Which KDK supplied this symbol?
- Is this projected image stale relative to the cache member?
- Did this analysis use a `.symbols` sidecar?
- Which OS build produced this kernel collection?

## Analysis Database Modules

### Image Module

Responsibilities:

- loaded image metadata
- segments and sections
- load commands
- imports and exports
- relocations/fixups
- symbol table records
- source-to-address mappings

Current reuse:

- `BinaryImage`
- `Section`
- `Symbol`
- `Import`
- `DyldMetadata`
- `ProjectedImageProvenance`

### Byte Classification Module

Responsibilities:

- classify byte ranges as header, load command, code, data, string, pointer,
  ObjC metadata, C++ metadata, debug info, unknown, or conflicting
- track overlapping interpretations
- expose "make code", "make data", "make string" later

This module is required for Hopper-like unknown/data/code workflows and for
eventual patching.

### Function Module

Responsibilities:

- function identity
- function names and aliases
- starts and ranges
- basic blocks
- edges
- calls and callers
- stack frame facts
- argument and return facts
- summary metrics
- confidence and unresolved reasons

Current reuse:

- `BasicBlock`
- `ControlFlowEdge`
- `DisassemblyAnalysis`
- `DisassemblySummary`

The current local-window analysis becomes an input to durable functions.

### Reference Module

Responsibilities:

- forward references
- reverse references
- typed reference kinds
- cross-image references
- confidence/provenance
- user references

This is the module that turns Damsel from a report generator into an explorer.

### Name Module

Responsibilities:

- symbol names
- user labels
- ObjC method names
- Swift demangled names
- C++ demangled names
- KDK/DWARF names
- conflict handling
- display priority

Priority sketch:

1. user label
2. exact debug/DWARF/KDK symbol
3. ObjC/C++ runtime-derived semantic name
4. export symbol
5. symbol table local name
6. import/stub name
7. analyzer-generated synthetic name

### Runtime Module

Responsibilities:

- Objective-C classes, protocols, categories, methods, properties, ivars
- Objective-C selector and class refs
- message-send callsites and possible targets
- C++ typeinfo, vtables, class hierarchy, methods, virtual calls
- Swift symbols and future metadata stubs
- blocks and closure metadata

This module should not own raw parser logic. It owns normalized runtime objects
created from parser and analyzer facts.

### Kernel Module

Responsibilities:

- kernel collection/fileset model
- kernel image and kext identity
- kext dependencies
- KDK/dSYM associations
- IOKit classes and vtables
- syscalls
- Mach traps
- MIG subsystems and routines
- panic/backtrace symbolication records
- kernel-specific pointer-auth/fixup interpretations

This module is separate because XNU workflows use different vocabulary than
ordinary userland Mach-O.

### Graph Module

Responsibilities:

- graph indexes
- graph layouts
- graph queries
- graph exports
- graph summaries

Graph types:

- CFG
- call graph
- xref graph
- cache dependency graph
- import/provider graph
- ObjC class graph
- ObjC selector graph
- C++ class graph
- kext graph
- syscall/trap/MIG graph

### Annotation Module

Responsibilities:

- comments
- labels
- tags
- colors
- bookmarks
- manual function bounds
- manual references
- manual type assignments
- audit trail

This is required for a Hopper-like project experience even before patching.

## Analysis Scheduler

Hopper-like automatic analysis needs a scheduler, not a monolithic `analyze_all`
function.

Pass types:

- source registration
- loader pass
- section classification
- symbol/import/export pass
- function seed pass
- decode pass
- CFG pass
- xref pass
- string/data pass
- ObjC metadata pass
- ObjC message-send pass
- C++ ABI pass
- kernel collection pass
- kernel semantic pass
- DWARF/KDK enrichment pass
- graph layout pass
- summary/report pass

Pass properties:

- stable id
- version
- dependencies
- input selectors
- output fact kinds
- invalidation policy
- progress reporting
- cancellation support
- max-cost category

Example:

```text
Pass: objc-message-send-v1
Depends on:
  disassembly-facts-v2
  objc-metadata-v1
  function-index-v1
Reads:
  instructions
  recovered values
  import/stub refs
  ObjC selector refs
Writes:
  ObjcMessageSendCallsite
  ReferenceKind::ObjcMessage
  possible target refs
```

## CLI Architecture

Existing CLI commands should remain stable. New commands should query the
analysis database rather than rerun heavy analysis every time.

New command families:

- `project create`
- `project analyze`
- `project status`
- `project facts`
- `project xrefs`
- `project functions`
- `project strings`
- `project graph`
- `project comments`
- `objc classes`
- `objc selectors`
- `objc method`
- `objc callers`
- `cpp types`
- `cpp vtables`
- `kernel info`
- `kernel members`
- `kernel kexts`
- `kernel symbols`
- `kernel panic-symbolicate`

Compatibility rule:

- Existing `disasm`, `objc`, and `cache` point-query commands should continue to
  work without requiring a project database.
- When a project is available, they may enrich output from the database.

## Web UI API Architecture

The current UI API can continue serving direct upload/disassembly workflows.
The Hopper-gap UI needs project endpoints.

Endpoint groups:

- project lifecycle
- source list
- analysis progress
- function list
- function details
- graph data
- xrefs
- strings
- symbols
- ObjC browser
- C++ browser
- kernel browser
- comments/labels/tags

API design rules:

- Keep heavy graph/layout work server-side.
- Page large lists.
- Make every object linkable by stable id.
- Return provenance and confidence on inferred records.
- Use streaming or polling for long-running analysis progress.

## Native Workbench Architecture

Damsel Workbench should remain a thin native shell over the server APIs.

Major panes:

- source/project navigator
- symbols/functions/strings tabs
- ObjC/C++/Kernel tabs when applicable
- central representation area: assembly, graph, pseudocode later, hex
- inspector/xrefs pane
- search bar
- analysis progress/status area

State rules:

- workbench state can cache view models, not analysis facts
- project database and Rust server remain source of truth
- every navigation target should be addressable by route-like state
- long-running analysis must be cancellable

## Data Flow Examples

### Open A macOS Framework

1. User creates project from framework Mach-O.
2. Source registry records path, UUID, arch, hash.
3. Loader creates `BinaryImage` facts.
4. Section classifier records code/data/string/meta ranges.
5. Function seed pass uses symbols, exports, and function starts.
6. Disassembly pass decodes executable ranges.
7. CFG pass records function blocks and edges.
8. ObjC pass normalizes classes/selectors/methods.
9. Xref pass records code/data/runtime references.
10. UI displays functions, classes, selectors, strings, CFG, and xrefs.

### Open A dyld Shared Cache Image

1. User opens shared-cache root or member.
2. Existing cache discovery builds `SharedCache`.
3. User selects image.
4. Projection builds `ProjectedBinaryImage`.
5. Project records both cache-level and projected-image address spaces.
6. Analysis runs over projected image.
7. Cache provider/importer/reexport facts enrich xrefs.
8. UI allows jumping from an import call to the provider image/function.

### Open A Kernel Collection

1. User opens kernelcache/kernel collection.
2. Fileset loader records container and member address spaces.
3. Kext/member registry records identifiers and dependencies.
4. KDK/dSYM matcher attaches symbols/types when UUID/build matches.
5. Function pass seeds from symbols, unwind, fileset entries, and known tables.
6. C++ ABI pass recovers vtables/typeinfo/IOKit classes.
7. Kernel semantic pass recognizes syscalls, Mach traps, MIG subsystems, and
   panic-relevant symbols.
8. UI displays kext graph, IOKit classes, kernel functions, and panic
   symbolication tools.

## Persistence Strategy

### Stage 1: JSON Project Snapshot

Use deterministic JSON to validate model shape and tests.

Pros:

- easy snapshots
- easy review
- consistent with current CLI JSON culture

Cons:

- expensive for large caches
- weak random access

### Stage 2: SQLite Or Similar Embedded Store

Use SQLite once entities and indexes stabilize.

Required indexes:

- address to facts
- fact kind
- source id
- image id
- function start
- reference source
- reference target
- name text
- string text
- runtime object id
- graph id

### Stage 3: Large Cache Optimization

For multi-GB dyld caches and kernel collections:

- lazy source mapping
- persistent decode cache
- separate derived index files
- chunked analysis
- background graph layout
- content-addressed fact shards

## Error Model

New typed errors should distinguish:

- unsupported architecture
- unsupported file format
- malformed container
- incomplete cache member set
- missing KDK/dSYM
- UUID mismatch
- unsupported pointer format
- ambiguous image/member/symbol
- address not mapped
- function boundary conflict
- analyzer timeout
- cancelled analysis
- stale project database

Do not collapse these into generic "analysis failed" errors. The user needs to
know whether to provide a sidecar, choose a member, reduce analysis scope, or
file a parser bug.

## Security And Privacy Boundaries

Damsel should avoid copying private binary content into logs by default.

Rules:

- log paths, hashes, UUIDs, counts, and typed errors
- avoid logging raw instructions around secrets unless user asks
- avoid uploading binaries unless explicit future cloud workflow exists
- keep KDK/dSYM paths local
- keep project databases local by default
- mark generated evidence packages if they contain symbol names or strings

## Why This Architecture Fits Damsel

This architecture keeps the current shape:

- `damsel-macho` remains the parser/loader/analysis crate.
- `damsel-core` remains the typed model crate.
- `damsel-cli` remains the deterministic CLI/API layer.
- Workbench remains a native shell over server APIs.

The new work is additive:

- project database over existing models
- analyzer passes over existing loaders
- graph/xref indexes over existing disassembly facts
- runtime analyzers over existing ObjC/dyld/shared-cache facts
- kernel container support beside shared-cache support
