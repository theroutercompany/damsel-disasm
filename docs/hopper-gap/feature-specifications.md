# Feature Specifications

Status: Planning specifications. These are intended to be concrete enough to
turn into issues and tests.

## Spec 1: Analysis Project

### Goal

Create a durable project container that stores source identity, loaded images,
analysis facts, xrefs, graph indexes, runtime objects, and user annotations.

### User Stories

- As a reverse engineer, I can open a binary once, let Damsel analyze it, close
  the app, and reopen the same project without recomputing every fact.
- As a maintainer, I can inspect which analyzer produced a fact and invalidate
  facts when the analyzer changes.
- As a CLI user, I can create a project and query functions, xrefs, symbols,
  strings, ObjC objects, and graphs from deterministic JSON.

### Inputs

- standalone Mach-O path
- dyld shared-cache root/member path
- selected cache image id/install name
- kernelcache/kernel collection path
- optional dSYM/KDK path
- analysis options

### Outputs

- project database
- analysis status
- project manifest
- source registry
- fact tables
- index tables

### Model Sketch

```text
AnalysisProject {
  schema_version
  project_id
  created_by_damsel_version
  sources
  address_spaces
  facts
  indexes
  annotations
  analysis_runs
}
```

### CLI Sketch

```sh
damsel project create --input /path/to/AppKit --project AppKit.damsel
damsel project analyze AppKit.damsel
damsel project status AppKit.damsel
damsel project facts AppKit.damsel --kind function --limit 20
```

### Acceptance Criteria

- Project creation records source hash, size, UUID, architecture, and source
  role.
- Project analysis can be interrupted without corrupting the project.
- Re-running analysis does not duplicate facts.
- Facts include source/provenance/confidence.
- Existing point-query commands continue working without a project.

## Spec 2: Byte Classification

### Goal

Classify address ranges into code, data, string, metadata, pointer/fixup, debug
info, and unknown regions.

### User Stories

- I can see which bytes Damsel believes are code versus data.
- I can find all unknown executable bytes.
- I can ask why a range was classified as ObjC metadata or C++ metadata.

### Classification Kinds

- header
- load command
- segment padding
- code
- function prologue
- thunk
- stub
- literal data
- pointer data
- chained fixup
- cstring
- objc metadata
- c++ metadata
- swift metadata placeholder
- unwind
- dwarf/debug
- kernel table
- unknown
- conflicting

### Sources

- Mach-O sections
- load commands
- symbol table
- export trie
- function starts
- relocations
- chained fixups
- ObjC sections
- C++ ABI recognizers
- string scanners
- kernel recognizers
- user actions

### Acceptance Criteria

- Every section has at least a coarse classification.
- Classified ranges are queryable by address.
- Overlaps are represented explicitly.
- Manual user classification does not erase inferred classification.

## Spec 3: Function Index

### Goal

Build a durable whole-image function index with starts, ranges, blocks, edges,
names, references, callers, callees, and summary facts.

### Inputs

- symbols
- export trie
- function starts
- entrypoints
- ObjC IMPs
- C++ vtable entries
- unwind records
- disassembly references
- user-defined function starts

### Outputs

- function records
- block records
- edge records
- caller/callee indexes
- unresolved edge records

### Function Record

```text
Function {
  id
  address_space
  primary_start
  entry_points
  ranges
  names
  source
  confidence
  block_ids
  edge_ids
  caller_function_ids
  callee_function_ids
  import_calls
  runtime_refs
  data_refs
  summary
}
```

### Acceptance Criteria

- Function starts from `LC_FUNCTION_STARTS` become function records.
- ObjC method IMPs become function records even when unnamed.
- Direct calls create caller/callee relationships.
- Tail calls and unresolved indirect calls are represented without guessing.
- Function records preserve source and confidence.

## Spec 4: Xref Index

### Goal

Provide bidirectional references for addresses, functions, symbols, imports,
strings, ObjC selectors/classes/methods, C++ types/vtables, and kernel semantic
objects.

### Reference Record

```text
Reference {
  id
  kind
  from
  to
  instruction_address?
  function_id?
  source
  confidence
  notes
}
```

### Required Reference Kinds

- call
- branch
- fallthrough
- tail_call
- data_read
- data_write
- address_materialization
- string_reference
- import_call
- stub_reference
- fixup_reference
- objc_selector_reference
- objc_class_reference
- objc_method_implementation
- objc_message_send_possible_target
- cpp_vtable_slot
- cpp_typeinfo_reference
- cpp_virtual_call_possible_target
- kernel_table_entry
- user_manual

### CLI Sketch

```sh
damsel project xrefs AppKit.damsel --to 0x1234
damsel project xrefs AppKit.damsel --from-function _foo
damsel objc callers AppKit.damsel --selector "initWithFrame:"
damsel cpp xrefs Kernel.damsel --type IOService
```

### Acceptance Criteria

- Forward and reverse queries return stable sorted results.
- Cross-image cache references preserve provider/importer image provenance.
- Ambiguous targets are returned as ambiguous records, not hidden.

## Spec 5: Graph Service

### Goal

Provide graph data and layout for CFGs, call graphs, runtime graphs, cache
graphs, and kernel graphs.

### Graph Types

- function CFG
- local call graph
- image call graph
- cache provider/importer graph
- ObjC class hierarchy
- ObjC selector graph
- C++ class hierarchy
- C++ virtual dispatch graph
- kext dependency graph
- syscall/trap/MIG graph

### API Requirements

- graph id
- graph kind
- node list
- edge list
- layout hints
- unresolved/ambiguous edges
- expansion token for large graphs

### UI Requirements

- assembly and graph remain synchronized
- selecting instruction highlights graph node
- selecting graph node scrolls instruction list
- xrefs visible for selected node
- graph can be exported as JSON and DOT

### Acceptance Criteria

- CFG graph can render for a fixture function with branches and loops.
- Large graphs can be paged or expanded incrementally.
- Missing layout does not prevent list-mode graph inspection.

## Spec 6: Objective-C Explorer

### Goal

Turn existing ObjC metadata extraction into a navigable class, selector,
method, and message-send explorer.

### Objects

- ObjC class
- ObjC metaclass
- ObjC protocol
- ObjC category
- ObjC method
- ObjC property
- ObjC ivar
- ObjC selector
- ObjC message-send callsite
- ObjC block placeholder

### Required Views

- class list
- class detail
- superclass/subclass graph
- protocols adopted by class
- categories applied to class
- methods grouped by instance/class/category/protocol
- selector implementers
- selector callers
- method disassembly
- type encoding decode
- ivar/property details

### Message-Send Record

```text
ObjcMessageSend {
  callsite
  function_id
  receiver_value
  receiver_class_candidates
  selector_value
  selector_name
  possible_method_targets
  dispatch_kind
  confidence
  unresolved_reason
}
```

### Dispatch Kinds

- objc_msgSend
- objc_msgSendSuper
- objc_msgSend_stret
- objc_msgSendSuper_stret
- objc_msgSend_fpret
- direct IMP call
- block invoke
- unknown

### Acceptance Criteria

- Class list and selector list are queryable from CLI and UI API.
- Selecting a method can jump to its implementation.
- Selector callers include direct selector refs and inferred message sends.
- Unknown receiver classes remain explicit.
- Type encodings are decoded into readable signatures when possible.

## Spec 7: C++ And Objective-C++ Explorer

### Goal

Recover C++ semantic objects from symbols, ABI metadata, vtables, RTTI, unwind
data, and code references, with special attention to IOKit and XNU.

### Objects

- C++ type
- C++ namespace
- C++ method
- constructor
- destructor
- operator
- thunk
- vtable
- vtable slot
- typeinfo
- inheritance edge
- virtual callsite
- exception landing pad

### Required Views

- demangled symbol list
- C++ type list
- type detail
- inheritance graph
- vtable view
- virtual call targets
- constructor/destructor references
- IOKit class view

### Acceptance Criteria

- Itanium demangling is available in core analysis without shelling out.
- Vtables are discovered in representative fixtures.
- Typeinfo references create type records where present.
- Virtual callsites preserve unresolved/ambiguous status.
- IOKit classes can be recognized from metadata in kernel fixtures.

## Spec 8: Kernel Collection And XNU Analysis

### Goal

Support XNU-style system binaries as first-class analysis targets.

### Inputs

- kernelcache/kernel collection path
- fileset-based Mach-O
- extracted KDK symbols/dSYMs
- optional panic/backtrace text

### Objects

- kernel collection
- fileset member
- kernel image
- kext
- kext dependency
- kernel function
- IOKit class
- syscall table entry
- Mach trap entry
- MIG subsystem
- panic frame

### CLI Sketch

```sh
damsel kernel info kernelcache.release.iphone
damsel kernel members kernelcache.release.iphone
damsel kernel kexts Kernel.damsel
damsel kernel symbols Kernel.damsel --name IOService
damsel kernel panic-symbolicate Kernel.damsel panic.txt
```

### Acceptance Criteria

- Fileset entries are listed with ids, names, offsets, and vmaddr ranges.
- Kernel/kext members have separate address spaces.
- KDK/dSYM UUID mismatch is reported as a typed error.
- Kext dependencies are queryable.
- Panic symbolication accepts a slide and resolves frames where possible.

## Spec 9: Pointer Authentication And Chained Fixup Semantics

### Goal

Represent arm64e pointer-authenticated pointers and chained fixups as semantic
analysis facts rather than opaque numbers.

### Pointer Record

```text
PointerFact {
  raw_value
  storage_address
  interpreted_target
  format
  auth_key?
  discriminator?
  diversity?
  address_diversity?
  source
  confidence
}
```

### Acceptance Criteria

- Damsel does not display signed pointer high bits as ordinary addresses.
- ObjC IMPs, C++ vtable slots, and kernel table entries can attach pointer-auth
  interpretation facts.
- Unsupported pointer formats produce typed errors or explicit unknown records.

## Spec 10: Type And Debug-Info Integration

### Goal

Attach DWARF/dSYM/KDK type, line, inline, and function information to analysis
objects.

### Required Features

- dSYM UUID matching
- KDK bundle discovery/selection
- function symbol import
- line table import
- type import
- argument/local variable import
- source path display
- conflict handling

### Acceptance Criteria

- UUID mismatch is typed.
- Debug names do not silently overwrite user labels.
- Function signatures can be associated with function records.
- Source line context can be displayed when available.

## Spec 11: Decompiler-Lite Summary

### Goal

Deliver useful high-level function understanding before full pseudocode.

### Summary Fields

- likely arguments used
- registers clobbered
- imports called
- local functions called
- selectors messaged
- classes referenced
- C++ virtual calls
- strings referenced
- global data referenced
- switch/jump-table summary
- return facts
- unresolved facts

### Acceptance Criteria

- Summary works without full type info.
- Summary labels uncertain facts as uncertain.
- ObjC and C++ facts are rendered in domain language.

## Spec 12: Read-Only Scripting

### Goal

Expose safe automation over the analysis database before patching or debugger
mutation.

### Allowed Operations

- open project
- run analysis
- query facts
- query xrefs
- export graphs
- add comments/labels/tags
- batch symbolicate
- generate reports

### Disallowed In First Version

- modify binary bytes
- inject into processes
- write to system caches
- bypass platform protections

### Acceptance Criteria

- Scripts cannot mutate input binaries.
- Script actions that change project annotations are recorded in an audit log.
- CLI and UI can show script-created annotations.

## Spec 13: Byte Classification And Feature Map

### Goal

Make every mapped byte range part of the analysis model, so Damsel can support
Hopper-style auto analysis, manual type conversion, hex synchronization, and a
feature-map navigation bar.

### Classification Kinds

- unknown
- file header
- load command
- code
- procedure
- data
- string
- ObjC metadata
- C++ metadata
- Swift metadata
- relocation or fixup
- debug or symbolication data
- unmapped or excluded

### Required Fields

- project/source object id
- virtual address range
- file offset range, when available
- classification kind
- inferred or user provenance
- confidence
- producing analysis pass
- superseded classification id, when a user edit overrides inference

### User Commands

- make code
- make procedure
- make data
- cycle data width
- make ASCII or UTF string
- mark undefined
- mark excluded
- restore inferred classification

### Acceptance Criteria

- Feature map renders from byte classifications, not section names alone.
- Manual classification changes are undoable.
- Reanalysis invalidates only affected facts where practical.
- Hex view and assembly view share the same byte-span selection model.

## Spec 14: Procedure-Local Editing

### Goal

Support the user edits that make a reverse-engineering session accumulate
understanding instead of resetting on each decode.

### Editable Procedure Facts

- local labels
- comments
- inline comments
- register aliases
- stack-variable names
- function signature
- calling convention
- block tags and colors
- manual block grouping, later

### Required Semantics

- Procedure-local names are scoped to a function and do not pollute global
  symbols.
- Register aliases and stack-variable names are applied by the renderer and
  preserved in graph and summary views.
- Signature edits can be marked user-authored, imported from debug info, or
  inferred.
- Calling-convention edits trigger argument/return reanalysis where available.

### Acceptance Criteria

- User can rename a stack slot and see the name in assembly, graph, and summary.
- User can assign a local label without changing the global symbol table.
- User can undo or redo every procedure-local edit.
- Imported symbols/debug names never silently overwrite user edits.

## Spec 15: Semantic Operand Rendering

### Goal

Replace opaque instruction strings with renderable semantic spans so assembly,
inspector, scripting, search, and future MCP tools can agree on what each
operand means.

### Span Kinds

- address
- global symbol
- local label
- register
- register alias
- stack variable
- immediate
- formatted immediate
- selector
- class
- protocol
- string
- comment
- xref target
- unresolved target

### User Overrides

- operand number format
- address-relative display
- selector/class presentation
- inline comment
- symbolic name choice when multiple names apply

### Acceptance Criteria

- Clicking a rendered address span navigates to a route.
- Operand formatting changes are persistent and undoable.
- Search can distinguish text matches from semantic selector/class/symbol
  matches.
- The same semantic spans can feed accessibility labels and MCP/API responses.

## Spec 16: Problem List And User Hints

### Goal

Expose analysis uncertainty as actionable issues instead of hiding it in missing
results.

### Problem Types

- ambiguous function start
- failed instruction decode
- unresolved indirect branch
- possible switch table
- malformed ObjC metadata
- missing cache provider
- missing dSYM/KDK symbol source
- PAC-stripped target could not be resolved
- unsupported chained-fixup encoding

### User Hints

- declare function start
- declare non-code region
- add manual reference
- provide switch table base/count/stride
- bind ObjC receiver or selector
- select symbolication profile

### Acceptance Criteria

- Problems are queryable, sortable, suppressible, and tied to routes.
- Applying a hint records provenance and triggers targeted reanalysis.
- Suppressed problems remain auditable.

## Spec 17: MCP-Ready Document API

### Goal

Prepare Damsel's API shape for future AI/MCP use without coupling analysis
correctness to any model.

### First-Class Objects

- project
- source
- route
- function
- basic block
- instruction
- byte range
- xref
- symbol
- string
- ObjC class/selector/method
- C++ type/vtable/method
- kernel collection/kext
- annotation
- problem

### Tool Categories

- read project state
- search facts
- navigate routes
- summarize function
- list xrefs
- list problems
- add comment/label/tag with explicit mutation permission
- export selected facts

### Acceptance Criteria

- Every API response includes stable object ids and provenance.
- Read-only tools can be exposed before mutation tools.
- Mutation tools use the same undo/audit transaction model as the UI.
- MCP/AI can never become the only way to access a feature; the app and CLI
  must use the same underlying contracts.
