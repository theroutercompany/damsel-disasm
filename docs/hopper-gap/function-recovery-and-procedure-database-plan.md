# Function Recovery And Procedure Database Plan

Status: subsystem architecture plan. This is not an implementation claim.

Purpose: define the durable procedure database Damsel needs before graph,
decompiler, debugger, automation, and Apple runtime workflows can feel
Hopper-grade. This document narrows in on function/procedure recovery, leaving
the full IR and pseudocode pipeline to
[Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md).

Research companion:
[Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
shows why procedures must be modeled as rich editable objects: Hopper's public
SDK exposes procedure frames, blocks, stack offsets, local labels, register
renames, variables, callers, callees, signatures, calling conventions, and
tags.

## Thesis

Hopper treats procedures as rich editable objects. Public SDK procedure objects
include basic blocks, stack facts, local labels, register renames, callers,
callees, variables, signatures, calling conventions, and tags. Damsel currently
has strong window-level disassembly analysis, but Hopper-grade workflows need a
project-persistent procedure database.

The procedure database is the bridge between:

- byte classifications
- disassembly windows
- CFG graph view
- xrefs
- ObjC method IMPs
- C++ vtables and virtual calls
- kernel interface handlers
- decompiler IR
- debugger frames
- user annotations
- automation/MCP commands

## Current Damsel Shape

Current strengths:

- instruction decode
- references and annotations
- basic block and edge summaries
- indirect control records
- jump-table candidate records
- recovered values
- ObjC references
- cache links
- CLI JSON contracts and fixture tests

Current limitation:

- a function is not yet a durable project object with stable identity, bounds,
  local facts, transaction history, user edits, and invalidation.

## Procedure Database Responsibilities

The database must answer:

- What procedures exist in this source?
- Why does each procedure exist?
- Which bytes/ranges belong to each procedure?
- Which instructions and basic blocks belong to it?
- Which edges leave or enter it?
- Which calls does it make?
- Who calls it?
- Which runtime objects own it?
- What names and signatures apply?
- What stack frame and local variables are known?
- What conflicts or uncertainties exist?
- What user hints override analyzer guesses?
- Which analysis stages are stale?

## Core Records

### Procedure

Fields:

- id
- project id
- source id
- address space id
- entry address
- ranges
- primary name id
- all name fact ids
- seed ids
- owner object ids
- status
- confidence
- analysis version
- stale flags
- problem ids

### ProcedureRange

Fields:

- id
- procedure id
- start address
- length
- range kind:
  - primary
  - cold
  - thunk
  - tail-merged
  - conflict
  - unknown
- byte classification ids
- confidence

### ProcedureOwner

Owner kinds:

- symbol
- export
- ObjC method
- C++ method
- C++ vtable slot
- Swift symbol
- kernel syscall
- Mach trap
- MIG routine
- IOKit method
- panic frame
- user label

Fields:

- owner id
- procedure id
- owner object id
- relation kind
- provenance
- confidence

### ProcedureName

Fields:

- name fact id
- procedure id
- display text
- demangled text
- source
- precedence
- confidence

Display precedence uses the type/symbolication plan:

1. user label
2. exact debug/KDK/dSYM
3. export/local symbol
4. ObjC owner/selector
5. C++ demangled name
6. Swift demangled name
7. import/stub
8. analyzer fallback

### ProcedureStatus

States:

- seed only
- queued
- decoding
- partial
- complete
- conflicted
- failed
- stale

The UI should show status directly. A seed-only procedure is not the same as a
fully recovered procedure.

## Seed Collection

### SeedCollector Pipeline

Collectors:

- entrypoint collector
- function-start collector
- symbol/export collector
- stub collector
- ObjC IMP collector
- C++ demangled method collector
- vtable slot collector
- fixup function pointer collector
- kernel table collector
- debug/KDK/dSYM collector
- user hint collector

Each collector emits `ProcedureSeed` records and never directly creates final
procedure ranges without validation.

### Seed Deduplication

Deduplicate by:

- same address space
- same start address
- compatible owner
- equivalent symbol/object owner

Do not deduplicate:

- same raw address in different address spaces
- same symbol name in different cache images
- same selector implemented by different classes/categories

## Recovery Worker

### Work Item

Fields:

- procedure seed id
- requested depth
- analysis profile
- priority
- cancellation token
- dependency versions

Profiles:

- triage
- normal
- deep
- decompiler prep

### Recovery Steps

1. Resolve seed address to executable bytes.
2. Decode semantic instruction window.
3. Recursively follow direct branches.
4. Record calls as outbound callsites.
5. Stop on terminators and procedure boundary conflicts.
6. Build basic blocks.
7. Normalize CFG edges.
8. Detect jump tables.
9. Detect thunks.
10. Detect prolog/epilog and stack hints.
11. Attach runtime owners.
12. Emit problems.

### Cancellation

Recovery must poll cancellation:

- before decoding next block
- before table scan
- before expensive xref search
- before persisting large batch

Canceled jobs leave partial procedure records marked partial, not corrupt.

## Boundary Policy

### Stop Conditions

Stop recursion on:

- return
- trap/halt
- no-return call with exact/strong evidence
- unconditional branch to known external procedure
- branch into non-executable range
- branch into byte range with stronger non-code classification
- decode failure
- cancellation
- analysis budget reached

### Continue Conditions

Continue on:

- fallthrough inside executable mapped range
- direct branch inside same procedure range
- conditional branch target inside executable mapped range
- switch target with validated table bounds
- thunk target only when profile asks to inline thunks

### Tail Calls

Tail calls must be explicit:

- do not merge tail-call target into caller by default
- create call edge with tail-call marker
- allow UI to follow target
- allow decompiler renderer to emit tail call or return callee result

## Conflict Handling

### Conflict Records

Kinds:

- overlap
- nested seed
- split ownership
- code/data conflict
- decode failure
- invalid branch target
- jump-table ambiguity
- thunk ambiguity
- no-return contradiction

Fields:

- conflict id
- procedure ids
- addresses/ranges
- involved facts
- severity
- recovery options

### Recovery Options

Examples:

- split procedure at address
- merge ranges
- mark bytes as data
- mark bytes as code
- set function start
- set no-return
- accept analyzer default
- suppress conflict

Recovery options are future UI actions backed by transactions.

## Call Graph

### Callsite

Fields:

- callsite id
- caller procedure id
- instruction id
- address
- call kind
- callee procedure id, optional
- target object id, optional
- target address, optional
- argument facts, later
- confidence

Call kinds:

- direct
- tail
- import
- stub
- ObjC
- C++ virtual
- Swift
- kernel interface
- indirect
- unresolved

### Caller/Callee Index

Indexes:

- procedure -> callees
- procedure -> callers
- runtime object -> procedures
- selector -> calling procedures
- vtable slot -> calling procedures
- symbol -> calling procedures

## Local Procedure Facts

### LocalLabel

Fields:

- label id
- procedure id
- address
- name
- source
- confidence

### RegisterAlias

Fields:

- alias id
- procedure id
- register
- live range
- name
- source

### LocalVariable

Fields:

- variable id
- procedure id
- storage kind:
  - stack slot
  - register
  - temporary
  - unknown
- range
- name facts
- type facts
- confidence

These are required before pseudocode can be user-correctable.

## Runtime Owner Integration

### ObjC

Procedure route should show:

- owning method(s)
- selector
- owner class/category/protocol
- type encoding
- message-send callers
- implementation IMP provenance

### C++

Procedure route should show:

- demangled method
- class
- vtable slots
- constructor/destructor role
- thunk relationship
- virtual callers

### Kernel

Procedure route should show:

- kext/member
- syscall/trap/MIG ownership
- IOKit class/method
- KDK/dSYM names
- panic frames

## API

### Procedure List

```text
GET /api/projects/{project_id}/procedures?source=&q=&owner_kind=&status=&cursor=
```

Response:

```json
{
  "items": [
    {
      "id": "procedure:source-1:100003f20",
      "entry": {
        "space_id": "addrspace:image-1",
        "value": 4294983456
      },
      "display_name": "-[Greeter greeting]",
      "owners": [],
      "status": "complete",
      "confidence": "strong",
      "problem_count": 0
    }
  ],
  "page": {}
}
```

### Procedure Detail

```text
GET /api/projects/{project_id}/procedures/{procedure_id}
```

Includes:

- ranges
- seeds
- names
- owners
- summary metrics
- status
- stale flags
- problems

### Procedure Graph

```text
GET /api/projects/{project_id}/procedures/{procedure_id}/cfg
```

Includes:

- blocks
- edges
- layout hints optional
- source instruction ids
- problem ids

### Procedure Xrefs

```text
GET /api/projects/{project_id}/procedures/{procedure_id}/callers
GET /api/projects/{project_id}/procedures/{procedure_id}/callees
```

## CLI

```text
damsel project procedures <project>
damsel project procedure <project> <address-or-id>
damsel project procedure-cfg <project> <address-or-id>
damsel project procedure-callers <project> <address-or-id>
damsel project procedure-callees <project> <address-or-id>
damsel project procedure-problems <project> <address-or-id>
damsel project procedure-hint-start <project> <address>
damsel project procedure-hint-bounds <project> <procedure> --start --end
```

## Workbench UI

### Procedure Navigator

Columns:

- name
- address
- owner
- status
- size
- calls
- problems

Filters:

- source/image
- owner kind
- status
- problem severity
- text query

### Procedure Inspector

Modules:

- summary
- names
- owners
- ranges
- CFG metrics
- callers/callees
- local facts
- runtime overlays
- problems
- analysis history

### Procedure Commands

- rename
- add comment
- show graph
- show pseudocode summary
- show callers
- show callees
- mark no-return
- edit signature
- split/merge later

## Storage And Indexing

Required indexes:

- entry address -> procedure
- address range -> procedure candidates
- owner object -> procedures
- name text -> procedures
- status -> procedures
- problem -> procedure
- caller -> callees
- callee -> callers
- source -> procedures

Invalidation:

- byte classification changes
- source bytes changes
- symbolication profile changes
- user boundary hints
- no-return facts
- architecture semantics version changes
- runtime owner changes

## Test Plan

### Unit Tests

- seed deduplication
- seed precedence
- range overlap detection
- tail-call classification
- no-return stop condition
- jump-table target validation
- conflict record creation
- caller/callee index updates
- invalidation from byte classification changes

### Fixture Tests

- linear function
- branch function
- loop function
- jump table function
- function pointer dispatch
- ObjC method IMP
- C++ vtable slot
- malformed function bytes
- overlapping symbol/function-start seeds

### Snapshot Tests

- procedure list JSON
- procedure detail JSON
- CFG JSON
- caller/callee JSON
- conflict/problem JSON

### Performance Tests

- procedure list paging
- batch seed collection on large binary
- cancel recovery on large function
- cache image procedure seed collection without full image decompilation

## Milestones

### P1: Seed Store

Deliver:

- seed collectors from current image facts
- seed deduplication
- seed JSON snapshots

### P2: Procedure Records

Deliver:

- procedure records
- ranges
- status/confidence
- source route integration

### P3: CFG Persistence

Deliver:

- persistent blocks and edges
- caller/callee index
- existing disasm analysis mapped into procedure DB

### P4: Conflict And Problem Model

Deliver:

- overlap/conflict detection
- procedure problems
- Workbench inspector module

### P5: Local Facts

Deliver:

- local labels
- register aliases
- stack/local placeholders
- user transaction hooks

### P6: Runtime Owners

Deliver:

- ObjC method owner links
- C++ owner links
- kernel owner links

## Acceptance Criteria

Procedure database work is done when:

- every recovered procedure has a stable id
- procedure list is pageable
- procedure detail includes seeds, ranges, owners, status, and problems
- CFG blocks and edges are persistent project facts
- caller/callee queries do not require re-decoding the whole function
- ObjC method IMPs link to procedures
- conflicting procedure boundaries are explicit problems
- user hints can create or adjust procedure facts through transactions
- Workbench can navigate procedures independently of transient disassembly
  windows

Until this exists, graph, decompiler, debugger, and automation work will remain
fragile.
