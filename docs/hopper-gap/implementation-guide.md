# Implementation Guide

Status: Engineering guide for future implementation. This document focuses on
how to build the Hopper-gap features inside the current Damsel architecture.

## Implementation Strategy

Build upward from the existing verified stack:

1. Keep existing point-query commands stable.
2. Add project/database models in `damsel-core`.
3. Add loader adapters and analysis passes in `damsel-macho`.
4. Add CLI commands that exercise the database before the UI depends on it.
5. Add UI API endpoints over the same database.
6. Add Workbench panes over the API.
7. Add real-system evidence gates only after fixture coverage is strong.

The strategic mistake would be to start with a decompiler or debugger. Those
are visible Hopper features, but they depend on lower layers that Damsel does
not yet have as durable project state.

## Crate-Level Plan

### `damsel-core`

Add pure model types:

- `AnalysisProject`
- `AnalysisSource`
- `AddressSpace`
- `AnalysisAddress`
- `AnalysisFact`
- `FactSource`
- `FactConfidence`
- `ByteClassification`
- `ByteClassificationOverride`
- `FunctionRecord`
- `FunctionName`
- `FunctionRange`
- `ProcedureLocalLabel`
- `RegisterAlias`
- `StackVariableRecord`
- `FunctionSignatureRecord`
- `ReferenceRecord`
- `RuntimeObject`
- `ObjcMessageSendRecord`
- `CppTypeRecord`
- `CppVtableRecord`
- `KernelCollectionRecord`
- `KernelMemberRecord`
- `KextRecord`
- `GraphRecord`
- `UserAnnotation`
- `AnnotationTransaction`
- `ProblemRecord`
- `SemanticSpan`
- `AnalysisRun`
- `AnalysisPassStatus`

Rules:

- No file I/O in model constructors.
- Keep `serde` representations deterministic.
- Prefer explicit enums over stringly typed public contracts.
- Record provenance on inferred facts.
- Preserve existing models while adding conversion/adaptation layers.

### `damsel-macho`

Add analysis and parser modules:

- `project.rs`: project construction over loaded inputs.
- `analysis_db.rs`: in-memory database implementation.
- `byte_classification.rs`: section and range classification.
- `annotation_store.rs`: comments, labels, tags, colors, bookmarks, manual refs,
  local labels, register aliases, stack variables, and undo transactions.
- `problem_store.rs`: actionable analysis uncertainty and user hints.
- `semantic_render.rs`: structured assembly rows with operand spans and routes.
- `function_index.rs`: function seed and function record builder.
- `xref_index.rs`: global reference builder.
- `graph.rs`: graph composition and export.
- `objc_analysis.rs`: message-send and higher-level ObjC analysis.
- `cpp.rs`: C++ demangling, ABI metadata, vtables, RTTI.
- `kernel.rs`: kernelcache/fileset/kernel collection loader.
- `kdk.rs`: KDK/dSYM matching and symbol import.
- `ptrauth.rs`: pointer-auth and chained-fixup interpretation helpers.
- `dwarf.rs`: debug-info integration, likely behind a feature flag once deps
  are chosen.

Rules:

- Parser modules should emit typed records, not print output.
- Analysis passes should be deterministic.
- Analysis passes should be separately testable with fixtures.
- Real-system parsers should have fixture reproductions for every bug found.

### `damsel-cli`

Add user-visible command families only after core records exist:

- `project`
- `project functions`
- `project xrefs`
- `project graph`
- `project strings`
- `project annotate`
- `project byte-map`
- `project problems`
- `project undo`
- `project redo`
- `project feature-map`
- `objc classes`
- `objc selectors`
- `objc callers`
- `cpp types`
- `cpp vtables`
- `kernel info`
- `kernel members`
- `kernel kexts`
- `kernel panic-symbolicate`

Rules:

- JSON first, text second.
- Stable sorting for every list.
- `--limit` and filters for large outputs.
- Explicit typed errors for missing sidecars, ambiguous symbols, unsupported
  formats, and stale project DB.

### `damsel-cli ui`

Add endpoints after CLI output proves the model:

- `GET /api/projects/{id}`
- `POST /api/projects`
- `POST /api/projects/{id}/analyze`
- `GET /api/projects/{id}/analysis-status`
- `GET /api/projects/{id}/byte-map`
- `POST /api/projects/{id}/byte-map/override`
- `GET /api/projects/{id}/feature-map`
- `GET /api/projects/{id}/problems`
- `GET /api/projects/{id}/functions`
- `GET /api/projects/{id}/functions/{function_id}`
- `GET /api/projects/{id}/xrefs`
- `GET /api/projects/{id}/graphs/{graph_id}`
- `GET /api/projects/{id}/objc/classes`
- `GET /api/projects/{id}/objc/selectors`
- `GET /api/projects/{id}/cpp/types`
- `GET /api/projects/{id}/kernel/kexts`
- `POST /api/projects/{id}/annotations`

Rules:

- Page large result sets.
- Keep server-side graph layout optional.
- Keep every response linkable by stable ids.
- Avoid returning raw full-binary content.

### `damsel-app`

Add Workbench features after API shape stabilizes:

- project/source navigator
- function/symbol/string tabs
- ObjC browser
- C++ browser
- kernel browser
- xref inspector
- graph canvas
- annotation controls
- analysis progress/cancel UI

Rules:

- No parser logic in Swift.
- Swift models mirror API contracts.
- The Workbench can cache UI state, not analysis truth.
- Large lists must be virtualized.

## Phase 1: Project Database

### Step 1: Add Model Skeleton

Add `damsel-core` types for:

- source identity
- address spaces
- facts
- functions
- references
- byte classifications
- annotations
- analysis runs

Start with JSON serialization even if the first storage layer is in-memory.
Snapshots will make contract drift visible.

### Step 2: Build In-Memory Database

Create an `AnalysisDatabase` with:

- source table
- address-space table
- fact table
- function table
- reference table
- name table
- byte-classification interval table
- annotation table

Use deterministic ids initially:

- source id from source role plus content hash/UUID
- address-space id from source id plus kind
- function id from address-space id plus primary start
- reference id from kind/from/to/source tuple

### Step 3: Adapt Existing Loaded Image

Implement:

```text
AnalysisDatabase::from_binary_image(BinaryImage)
AnalysisDatabase::from_projected_image(ProjectedBinaryImage)
AnalysisDatabase::from_shared_cache(SharedCache)
```

These adapters should not lose existing information.

### Step 4: Add CLI Smoke

Add:

```sh
damsel project create --input fixtures/bin/semantic-switch --json
damsel project facts PROJECT --kind section --json
damsel project status PROJECT --json
```

Acceptance:

- project manifest snapshot is deterministic
- existing fixture binary produces sources/address spaces/sections/symbol facts
- no existing CLI command output changes

## Phase 2: Function And Xref Index

### Function Seed Collector

Implement a pass that creates function seeds from:

- entrypoint
- symbol table definitions
- export trie entries
- function starts
- ObjC method IMPs
- stub helpers
- direct call targets
- branch targets that look like function starts
- vtable slots after C++ pass exists
- unwind entries after unwind pass exists
- user-defined starts

Each seed should record:

- address
- source
- strength
- optional name
- optional runtime owner

### Function Builder

Use recursive descent from seeds:

1. Decode basic block.
2. Stop at return, unconditional branch, invalid instruction, range end, or
   hard analyzer limit.
3. Add conditional branch targets and fallthrough.
4. Add direct call refs without necessarily entering callee.
5. Add jump-table targets when resolved.
6. Mark unresolved indirect edges.
7. Merge blocks into function record.

Important constraints:

- Do not blindly include callee code in caller function.
- Preserve external targets.
- Track tail-call candidates.
- Track conflict when two functions claim the same block.

### Xref Builder

Build references from:

- instruction references
- recovered values
- annotations
- import bindings
- stubs
- relocation/fixup records
- ObjC pointer refs and method IMPs
- C++ vtable/typeinfo refs
- kernel tables
- user manual refs

Index both `from` and `to`.

Acceptance:

- `semantic-switch` produces function, block, branch, jump-table, and data refs.
- `import-rich` produces import-call refs and provider links when cache context
  exists.
- `objc-sample` produces selector/class/method refs.
- Direct caller/callee queries are deterministic.

## Phase 3: Graph Service

Detailed companion:
[Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
defines the graph record/layout/selection/feature-map/hex-window contracts
behind this phase.

### Data Model

Add:

- `GraphRecord`
- `GraphNode`
- `GraphEdge`
- `GraphLayout`
- `GraphExpansion`

Graph node ids should point back to function/block/runtime/kernel object ids.

### CFG Graph

Build from function records:

- one node per block
- one edge per control-flow edge
- include unresolved external target nodes only when useful
- annotate nodes with instruction count, start/end, labels, comments, summary
  facts

### Call Graph

Build from caller/callee refs:

- one node per function
- one edge per call/tail-call/possible-indirect target
- preserve unresolved indirect call records separately

### Runtime Graphs

Build from runtime objects:

- ObjC class hierarchy
- selector implementer/caller graph
- C++ inheritance graph
- virtual dispatch graph
- kext dependency graph
- cache provider/importer graph

### Layout

Start with server-side DOT export and simple layered layout hints. Full native
interactive graph can come later. A list-mode graph should always exist.

Acceptance:

- CLI can export CFG as JSON and DOT.
- UI API can return graph nodes and edges.
- Workbench can render a simple graph or list-mode view without recomputing.

## Phase 4: Objective-C Deep Analysis

### Reuse Existing Metadata

The current `ObjcMetadata` model already has most raw runtime records needed.
The new work should normalize and index those records.

Add normalized objects:

- `ObjcClassObject`
- `ObjcProtocolObject`
- `ObjcCategoryObject`
- `ObjcMethodObject`
- `ObjcSelectorObject`
- `ObjcIvarObject`
- `ObjcPropertyObject`
- `ObjcMessageSendCallsite`

### Type Encoding Decoder

Implement an Objective-C type encoding parser:

- primitive types
- object types
- class types
- selector
- pointer
- arrays
- structs/unions as lossy names when full detail is hard
- method return and argument layout

Output should preserve:

- raw encoding
- parsed display
- unsupported fragments

### Selector Index

Build indexes:

- selector name to selector object
- selector to implementers
- selector to callsites
- class to methods
- class to categories
- class to protocols
- method IMP to function

### Message-Send Pass

Algorithm sketch:

1. Identify calls to `_objc_msgSend` family through imports/stubs/cache links.
2. Recover selector from `x1`.
3. Recover receiver from `x0`.
4. Classify receiver:
   - direct class ref
   - result of allocation/factory method
   - `self`
   - `super`
   - unknown
5. Resolve candidate methods:
   - exact class plus selector
   - superclass chain
   - category methods
   - protocol methods as weaker evidence
   - all implementers if class unknown
6. Record confidence and unresolved reason.

Important conservative rule:

- Do not present dynamic dispatch as a single certain target unless evidence is
  strong. Use candidate sets.

### Blocks

First pass:

- recognize block literal structures where section/data patterns are clear
- record invoke pointer
- record descriptor/signature pointer if available
- link block invoke to function record

Acceptance:

- CLI can list classes/selectors/methods.
- CLI can show selector implementers and callers.
- Workbench can jump from selector to method IMP disassembly.
- Type encoding parser snapshots cover representative encodings.

## Phase 5: C++ And Objective-C++ Analysis

### Demangling

Add a Rust demangler dependency rather than shelling out. Requirements:

- Itanium ABI support.
- Stable fallback to raw symbol if demangling fails.
- structured classification when possible:
  - namespace
  - type
  - method
  - constructor
  - destructor
  - operator
  - thunk
  - template

### Vtable Discovery

Sources:

- symbols containing vtable names
- `__const`/read-only data pointer arrays
- typeinfo refs
- relocations/fixups to functions
- known ABI layout
- kernel/IOKit metadata

Record:

- vtable address
- owner type candidate
- slots
- function targets
- destructor slots
- offset-to-top where available
- typeinfo pointer where available
- pointer-auth interpretation if arm64e

### Typeinfo Discovery

Parse ABI typeinfo objects where present:

- name
- kind
- base class relations
- vtable/typeinfo associations
- confidence

### Virtual Callsite Resolution

Use value-flow and call pattern recognition:

1. Identify load from object pointer.
2. Identify vtable load.
3. Identify slot load.
4. Identify indirect branch/call through slot.
5. Map slot offset to candidate vtable slots.
6. Record possible targets.

Do not overfit first version. Even identifying "virtual call through slot 0x28"
with candidate types is useful.

### IOKit Recovery

Recognize:

- `OSMetaClass`
- `OSObject`
- class metadata symbols
- vtables for IOKit classes
- constructors/destructors
- `init`, `start`, `stop`, `probe`, `newUserClient`
- external method dispatch tables

Acceptance:

- C++ fixture with vtable and virtual call resolves at least one vtable and
  slot.
- Demangled names appear in function records without replacing raw names.
- Kernel/IOKit fixture identifies at least one class and vtable.

## Phase 6: Kernel Collection And XNU Analysis

### Loader Responsibilities

Add a kernel container loader that can:

- identify fileset-based Mach-O
- list `LC_FILESET_ENTRY` members
- map member offsets and vmaddrs
- create address spaces per member
- extract member Mach-O views for reuse by existing image analyzers
- preserve global kernel collection address vocabulary

Do not special-case by filename alone. Use Mach-O headers and load commands.

### Kext Model

Fields:

- kext id
- bundle identifier
- display name
- version
- member/source id
- address space
- dependencies
- exports/imports
- symbols
- IOKit classes

Sources:

- fileset entry names
- `__PRELINK_INFO` where present
- symbols
- KDK metadata
- Info.plist if available from extracted bundles

### KDK/dSYM Matching

Implement:

- UUID extraction
- architecture matching
- build/version hint matching
- path-based KDK discovery only when user opts in or configures roots
- typed mismatch errors

Never silently apply a mismatched KDK. Wrong symbols are worse than missing
symbols.

### Kernel Semantic Recognizers

Recognizers should be modular:

- syscall table
- Mach trap table
- MIG subsystem table
- IOKit class/vtable
- external method dispatch
- panic string xrefs
- boot arg/string tables
- entitlement/security policy hooks later

Each recognizer returns:

- objects
- references
- confidence
- unresolved notes

### Panic Symbolication

Inputs:

- panic/backtrace text
- kernel collection project
- slide if not inferable
- optional KDK/dSYM

Outputs:

- frame list
- function/symbol names
- image/member/kext context
- offsets
- confidence
- unresolved frames

Acceptance:

- Synthetic fileset fixture lists members.
- Kernel fixture can create member address spaces.
- KDK UUID mismatch test returns typed error.
- Panic text fixture symbolicates known addresses.

## Phase 7: Pointer Authentication And Fixups

### Pointer Fact Layer

Create a pointer interpretation module that can represent:

- raw stored value
- stripped target
- signed pointer
- authenticated branch target
- chained fixup target
- bind target
- rebase target
- unknown pointer

### Integration Points

- dyld chained fixups
- ObjC class/method pointers
- ObjC selector/class refs
- C++ vtable slots
- function pointers
- kernel tables
- fileset fixups

### Display Rules

- show raw value when relevant
- show interpreted target separately
- show pointer format
- show confidence
- show unsupported/unknown formats explicitly

Acceptance:

- arm64e fixture displays interpreted pointer without pretending high bits are
  address bits.
- Unsupported pointer format is visible in JSON.
- ObjC/C++/kernel analyses consume pointer facts instead of reimplementing
  pointer stripping.

## Phase 8: Type And Debug-Info Integration

### Dependencies

Likely crates:

- `gimli` for DWARF
- `object` for object/debug file parsing
- a demangler crate for C++ names

Dependency choice should be made after a small spike, because debug-info
parsing can expand compile time and API surface.

### DWARF Import

Import:

- compile units
- functions
- line tables
- types
- variables
- inline scopes

Normalize into Damsel:

- function names and ranges
- source locations
- function signatures
- type records
- variable records

### Conflict Handling

Do not overwrite:

- user labels
- existing raw symbol names
- ObjC runtime names
- C++ demangled names

Instead store multiple names with source priority.

Acceptance:

- dSYM UUID mismatch test.
- fixture dSYM imports at least one function and line table.
- function detail view shows debug signature and source path when available.

## Phase 9: Decompiler-Lite

### Do Not Start With Full Pseudocode

Build high-level function summaries first.

Inputs:

- function CFG
- value-flow facts
- xrefs
- imports
- ObjC message sends
- C++ virtual calls
- strings
- data refs
- debug types

Outputs:

- likely arguments
- callees
- message sends
- virtual calls
- strings
- global data
- switch summary
- return summary
- unresolved notes

### IR Seed

When ready, define a low-level IR:

- registers
- memory loads/stores
- constants
- calls
- branches
- compares
- pointer-auth operations
- symbolic references

Then add:

- SSA
- stack variable recovery
- type propagation
- structured control-flow recovery
- high-level expression rendering

Acceptance:

- Summary snapshots are stable.
- ObjC callsites render as message-send summaries.
- C++ virtual calls render as virtual-call summaries with candidate targets.
- Unresolved facts remain visible.

## Phase 10: Scripting, Debugger, And Patching

### Scripting First

Expose read-only scripting over project facts:

- query functions
- query xrefs
- query ObjC/C++/kernel objects
- export graphs
- create comments/labels/tags
- batch symbolicate

Keep binary mutation out of first scripting version.

### Debugger Later

Use LLDB as transport:

- attach/run
- breakpoints
- register read
- memory read
- current frame to Damsel address mapping
- trace capture

Challenges:

- entitlements
- SIP
- codesigning
- sandboxing
- differing local/debug target paths

### Patching Last

Patching requires:

- explicit write mode
- assembler integration
- byte diff
- undo
- code signing consequences
- no shared-cache/kernel mutation by accident

## Fixture Strategy

### Existing Fixture Families To Extend

- branch-heavy fixture
- jump-table fixture
- import-rich fixture
- ObjC sample
- Swift sample
- synthetic shared cache
- real shared cache env-gated smoke

### New Fixture Families

- C++ vtable fixture
- C++ multiple inheritance fixture
- ObjC++ fixture
- ObjC message-send fixture with known class/selector targets
- block literal fixture
- arm64e pointer-auth fixture if buildable locally
- synthetic fileset/kernel collection fixture
- KDK/dSYM miniature fixture
- panic/backtrace text fixture
- IOKit-style C++ fixture

### Real Evidence Matrix

Opt-in real evidence should include:

- macOS dyld shared cache from current host
- at least one other macOS release cache
- extracted iOS dyld cache where available
- one KDK-matched kernel collection
- one stripped kernel collection without KDK
- one Objective-C-heavy framework
- one C++-heavy framework or kernel extension

Do not claim these ran unless the env-gated command actually ran.

## Validation Gates

### Model Gate

- serde snapshots stable
- ids deterministic
- no duplicate facts on re-analysis
- stale analyzer version invalidates derived facts

### Parser Gate

- malformed input tests
- incomplete cache/member tests
- unsupported architecture tests
- typed error snapshots

### Analysis Gate

- function discovery fixture
- xref fixture
- ObjC fixture
- C++ fixture
- kernel fixture
- graph fixture
- pointer-auth fixture

### CLI Gate

- JSON contract tests
- text snapshots
- stable sorting
- filter and limit tests

### UI Gate

- API contract tests
- upload/open project smoke
- function list smoke
- graph smoke
- ObjC/C++/kernel pane smoke as features land

### Performance Gate

- decode/index benchmark
- project open benchmark
- shared-cache selected-image benchmark
- large graph pagination benchmark
- memory use ceiling for real cache smoke

## Worktree Hygiene

Because the current repo often has in-progress app and disassembler edits, the
implementation path should avoid broad edits:

- add models behind new modules first
- avoid changing existing command output until new flags/commands exist
- snapshot new output separately
- keep generated project databases out of git
- update `.gitignore` when new local artifacts appear

## Dependency Spikes

Before committing large dependencies, run short spikes for:

- C++ demangling crate
- DWARF parsing with `gimli`
- graph layout option
- SQLite or embedded database option
- LLDB transport strategy

Spike outputs should be docs plus tiny fixtures, not half-integrated product
code.
