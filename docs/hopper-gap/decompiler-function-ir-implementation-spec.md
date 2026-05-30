# Decompiler, Function Recovery, And IR Implementation Spec

Status: implementation-grade planning spec. This is not an implementation
claim.

Purpose: define the concrete path from Damsel's current disassembly analysis to
Hopper-grade pseudocode and function understanding. The goal is not to render
pretty C-like text early. The goal is to build the recoverable, inspectable,
Apple-aware analysis layers that make pseudocode useful, correctable, and
honest.

Research companions:

- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)

Architecture companions:

- [Decompiler And IR Strategy](decompiler-ir-strategy.md)
- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Type And Symbolication System Plan](type-symbolication-system-plan.md)
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Decompiler Quality And User-Assisted Recovery Implementation Spec](decompiler-quality-user-assisted-recovery-implementation-spec.md)

## Why This Spec Exists

Hopper's public materials make decompilation look simple from the UI: click
Pseudo-Code and a procedure becomes C-like output. The public evidence shows
the implementation is deeper:

- Hopper's tutorial says pseudocode is functionally equivalent to CPU
  instructions but "more or less" like an Objective-C method.
- Hopper's FAQ warns that decompilers do not fully recover lost source and
  generated pseudocode is usually not directly compilable.
- Hopper's changelog shows repeated work on switch/case detection, no-return
  analysis, argument analysis, stack variables, variable liveness, AST
  optimization, dead-code removal, conditional ARM instructions, large-function
  decompilation, decompiler hangs, memory footprint, Objective-C ivar display,
  Swift strings/demangling, and authenticated Objective-C selector trampolines.
- Hopper's SDK exposes CPU decompiler hooks, but explicitly keeps the decompiler
  AST private. That is a signal: decompiler internals are core product
  architecture, not a small plugin layer.

Damsel's current code already provides useful prerequisites:

- `DecodedInstruction` with references, annotations, structured operands, and
  recovered values
- `DisassemblyAnalysis` with basic blocks, edges, direct calls, branch targets,
  indirect controls, imports, ObjC references, cache links, recovered values,
  jump tables, metrics, summary, and overview
- fixture tests around CFG summaries, jump tables, indirect targets, recovered
  values, ObjC message-send references, cache links, and arm64e authenticated
  indirect annotations
- ObjC metadata and dyld shared-cache projection/linkage foundations

The gap is that these are analysis outputs, not yet a durable function-recovery
and IR system. Hopper-grade pseudocode needs a procedure database, architecture
semantics, ABI model, value-flow engine, typed memory model, runtime overlays,
and an uncertainty-aware renderer.

## Product Goals

- Provide useful decompiler-lite summaries before full pseudocode.
- Recover durable procedure records with CFG, stack frame, locals, calls,
  xrefs, signatures, and problem facts.
- Lift arm64/arm64e instructions into a low-level IR with source mappings.
- Build SSA/def-use and value-flow facts over procedures.
- Model Apple ABI calling conventions and call effects.
- Model memory references, stack slots, globals, fixups, ObjC metadata, C++
  vtables, and kernel tables as typed address-space facts.
- Use ObjC/C++/Swift/kernel runtime overlays to improve function summaries and
  later pseudocode.
- Render pseudocode with confidence markers, unresolved regions, and direct
  links back to assembly, graph, types, and provenance.
- Make every stage testable by snapshots and metrics.

## Non-Goals

- No promise of recompilable source.
- No generic multi-architecture decompiler in the first wave.
- No source-level Swift reconstruction in the first wave.
- No complete C++ template or exception semantic reconstruction in the first
  wave.
- No decompiler-driven binary patching.
- No hiding unknowns behind plausible syntax.
- No full pointer-alias proof engine before useful summaries.

## Current Damsel Baseline To Reuse

### Existing Analysis Outputs

Use current disassembly outputs as seeds:

- basic block records
- control-flow edges
- direct calls
- branch targets
- indirect call/branch records
- jump-table candidates
- recovered values
- import references
- ObjC references
- cache links
- data references
- annotations
- metrics

### Existing Test Surface

Current test families provide immediate regression anchors:

- `damsel-macho/tests/disasm_matrix.rs`
- `damsel-core/tests/decode_behavior.rs`
- `damsel-core/tests/reference_annotation_contract.rs`
- `damsel-core/tests/typed_model_contract.rs`
- `damsel-cli/tests/cli_json.rs`
- `damsel-cli/tests/ui_api.rs`
- `damsel-macho/tests/shared_cache_real_env.rs`

Important existing fixture themes:

- semantic branch/control-flow fixtures
- indirect dispatch
- ObjC sample
- malformed ObjC protocol list
- shared-cache real-env smoke
- arm64e authenticated indirect flow when present

### Existing Gap

Current `analyze_disassembly` builds summaries from a decoded instruction
window. It does not yet provide:

- project-persistent function/procedure identity
- whole-image function discovery and conflict handling
- function-bound repair commands
- architecture semantics separate from string decode
- low-level IR
- SSA
- stack/local variable model
- calling-convention and call-effect facts
- alias/memory regions
- decompiler-specific problem records
- pseudocode source maps
- user hints that re-run analysis

## Stage Model

Decompiler work should be divided into stages with separate artifacts:

```text
Procedure Seed Discovery
  -> Procedure Boundary Recovery
  -> CFG Normalization
  -> Architecture Semantics
  -> Low-Level IR
  -> Def-Use And SSA
  -> Stack And Local Recovery
  -> ABI And Call Effects
  -> Type And Memory Enrichment
  -> Runtime Overlays
  -> Control-Structure Recovery
  -> Decompiler-Lite Summary
  -> Pseudocode Renderer
```

Each stage emits:

- records
- facts
- problems
- metrics
- source mappings
- invalidation dependencies

No stage should only emit a final string.

## Procedure Seed Discovery

### Seed Sources

Seed functions from:

- Mach-O entry points
- `LC_FUNCTION_STARTS`
- symbols
- export trie
- import stubs and stub helpers
- ObjC method IMPs
- C++ symbol demangling
- vtable slots
- chained-fixup function pointers
- jump-table function pointer tables
- kernel interface tables
- KDK/dSYM function symbols
- user `make procedure` commands
- debugger/runtime frames later

### ProcedureSeed Record

Fields:

- seed id
- project id
- source id
- address space id
- address
- seed kind
- owner object id, when known
- confidence
- provenance
- analyzer version
- stale flag

Seed kinds:

- entrypoint
- function_start
- symbol
- export
- stub
- ObjC IMP
- C++ method
- vtable slot
- function pointer
- kernel table handler
- debug symbol
- user asserted

### Seed Rules

- User asserted seeds have highest precedence.
- Debug/KDK/dSYM exact function starts outrank inferred starts.
- ObjC IMP seeds link to method objects.
- Vtable slot seeds link to C++ method candidates.
- Low-confidence pointer targets do not create full procedures until validated
  by decode/prolog/control-flow evidence.

## Procedure Boundary Recovery

### ProcedureRecord

Fields:

- procedure id
- entry address
- source id
- address space id
- discovered ranges
- seed ids
- owner runtime object ids
- basic block ids
- exit block ids
- callsite ids
- local label ids
- stack frame id
- signature fact ids
- calling convention
- analysis status
- confidence
- problems
- version

Analysis statuses:

- seed only
- decoding
- partial
- complete
- conflicted
- stale
- failed

### Boundary Algorithm

Initial algorithm:

1. Start from strongest seed.
2. Decode recursively over executable byte ranges.
3. Stop on return, trap, unconditional tail branch, unmapped address, excluded
   range, or existing stronger procedure entry.
4. Follow direct conditional/unconditional branches inside mapped executable
   ranges.
5. Record direct calls as callsites rather than merging callee bytes into the
   current procedure.
6. Record indirect branches/calls as unresolved terminators unless a recovered
   target is exact or strong.
7. Treat jump-table candidates as switch terminators only when table bounds are
   credible.
8. Preserve conflicts instead of silently merging overlapping functions.

### Boundary Conflicts

Conflict kinds:

- overlapping procedure ranges
- branch into middle of stronger procedure
- symbol start inside decoded procedure
- data byte classified inside code range
- code byte classified inside strong data range
- decode failure inside likely function
- impossible fallthrough

Resolution rules:

- user boundaries outrank inferred boundaries
- exact debug boundaries outrank function-start hints
- ObjC IMP boundaries are strong but can be contradicted by decode failure
- conflicts become problem objects and inspector warnings
- ambiguous procedures stay navigable

## CFG Normalization

### BasicBlockRecord

Fields:

- block id
- procedure id
- start address
- end address
- instruction ids
- predecessor edge ids
- successor edge ids
- block kind
- loop role, later
- dominance metadata, later
- problems

Block kinds:

- entry
- ordinary
- dispatch
- switch dispatch
- return
- trap
- tail call
- external thunk
- unresolved

### EdgeRecord

Fields:

- edge id
- procedure id
- source block id
- target block id, optional
- source instruction id
- target address
- edge kind
- condition
- confidence
- provenance

Edge kinds:

- fallthrough
- unconditional branch
- conditional true
- conditional false
- switch case
- call
- tail call
- return
- trap
- unresolved indirect
- exception/unwind later

### CFG Metrics

Store:

- block count
- edge count
- entry count
- exit count
- disconnected block count
- loop header count
- back edge count
- unresolved indirect count
- switch candidate count
- conflict count

## Architecture Semantics Layer

### Trait Shape

The decompiler should not depend directly on rendered Capstone strings. Define
an internal architecture semantics trait:

```rust
trait ArchitectureSemantics {
    fn decode_semantic_instruction(&self, bytes: &[u8], address: AnalysisAddress)
        -> Result<SemanticInstruction, DecodeProblem>;
    fn classify_branch(&self, instruction: &SemanticInstruction) -> BranchSemantics;
    fn lift_instruction(&self, instruction: &SemanticInstruction, ctx: &LiftContext)
        -> LiftResult;
    fn detect_prolog(&self, window: &[SemanticInstruction]) -> PrologEvidence;
    fn detect_epilog(&self, window: &[SemanticInstruction]) -> EpilogEvidence;
    fn detect_thunk(&self, window: &[SemanticInstruction]) -> ThunkEvidence;
    fn detect_switch(&self, procedure: &ProcedureRecord, facts: &FactStore)
        -> SwitchEvidence;
}
```

This is illustrative, not final Rust syntax.

### SemanticInstruction

Fields:

- instruction id
- address
- bytes
- mnemonic
- operand records
- register reads
- register writes
- memory reads
- memory writes
- immediate values
- branch semantics
- call semantics
- flag semantics
- PAC semantics
- source spans for rendering
- confidence

### arm64/arm64e Initial Coverage

High-priority instruction families:

- address materialization: `adr`, `adrp`, `add`, `sub`
- constant materialization: `mov`, `movz`, `movk`, `orr`
- loads/stores: `ldr`, `ldur`, `str`, `stur`, `ldp`, `stp`
- comparisons: `cmp`, `cmn`, `tst`, `ccmp`
- branches: `b`, `bl`, `blr`, `br`, `ret`
- conditional branches: `b.cond`, `cbz`, `cbnz`, `tbz`, `tbnz`
- select/conditional ops: `csel`, `csinc`, `csinv`, `csneg`
- sign/zero extensions
- arithmetic/logical ops needed for switch and pointer math
- PAC/authenticated branches and returns
- ObjC/dyld stub patterns

Coverage policy:

- complete semantics for high-value scalar instructions
- partial semantics with explicit unknowns for lower-value instructions
- vector/NEON support delayed unless needed for control/data-flow
- unknown instructions create low-confidence IR markers

## Low-Level IR

### IRFunction

Fields:

- IR function id
- procedure id
- architecture
- entry block id
- blocks
- source instruction map
- temporary count
- analysis version
- problems
- metrics

### IRBlock

Fields:

- IR block id
- source basic block id
- statements
- terminator
- predecessor/successor ids

### IRValue

Kinds:

- constant
- register
- temporary
- stack slot
- memory region
- memory load
- address
- symbol
- selector
- class object
- method object
- C++ object
- kernel object
- PAC-stripped pointer
- unknown
- poison

Common fields:

- bit width
- type hint id
- signedness hint
- provenance
- confidence
- source instruction id

### IRStatement

Kinds:

- assign
- load
- store
- arithmetic
- bitwise
- compare
- cast
- address computation
- call
- intrinsic
- phi placeholder
- annotation

Intrinsic classes:

- `objc_msg_send`
- `objc_msg_send_super`
- `swift_runtime_call`
- `cxx_virtual_call`
- `ptrauth_strip`
- `ptrauth_auth`
- `dyld_stub_call`
- `kernel_trap`
- `mig_dispatch`
- `unknown_call_effect`

### IRTerminator

Kinds:

- return
- branch
- conditional branch
- switch
- indirect branch
- call and return
- tail call
- trap
- unresolved

Rule:

- If the terminator cannot be proven, use unresolved. Never turn an unresolved
  indirect branch into a fake `if` or `switch` for renderer convenience.

## SSA And Def-Use

### SSA Goals

SSA is needed for:

- value provenance
- stack variable recovery
- argument recovery
- return value recovery
- conditional simplification
- switch recovery
- ObjC selector/receiver propagation
- C++ virtual call propagation

### Initial SSA Scope

First wave:

- scalar general registers
- SP/FP stack offsets
- selected memory regions
- function-local temporaries
- simple phi nodes at merge blocks

Deferred:

- full vector SSA
- complete memory SSA
- precise alias analysis
- exception edge SSA

### DefUseRecord

Fields:

- value id
- definition statement id
- use statement ids
- live range
- source register/stack slot
- type hints
- confidence

### Phi Policy

Use phi nodes when:

- block has multiple predecessors
- same abstract register/temporary has different reaching definitions
- control-flow merge affects later expression rendering

Avoid phi nodes for:

- unresolved indirect control-flow regions
- unknown clobber effects
- memory aliases without region facts

## Stack And Local Variable Recovery

### StackFrameRecord

Fields:

- frame id
- procedure id
- frame pointer usage
- stack pointer delta map
- saved registers
- local slots
- argument spill slots
- red zones or platform-specific frame notes
- prolog evidence
- epilog evidence
- confidence
- problems

### StackSlotRecord

Fields:

- slot id
- frame id
- offset from FP or SP
- size
- access list
- role:
  - local
  - spill
  - saved register
  - outgoing argument
  - incoming stack argument
  - unknown
- type hints
- display name facts
- user name override

### Recovery Algorithm

1. Detect prolog and epilog patterns.
2. Track SP deltas across CFG.
3. Identify FP-relative and SP-relative memory accesses.
4. Group accesses by normalized frame offset.
5. Infer sizes from load/store widths.
6. Mark saved registers.
7. Mark argument spills using ABI register knowledge near entry/calls.
8. Record inconsistent SP/FP facts as problems.

## ABI And Calling Convention Modeling

### AbiProfile

Fields:

- platform
- architecture
- pointer width
- integer register arguments
- floating/vector argument registers
- return registers
- call-clobbered registers
- callee-saved registers
- stack alignment
- variadic rules
- special conventions

Initial profiles:

- macOS arm64
- macOS arm64e
- iOS arm64/arm64e
- kernel arm64/arm64e
- ObjC message send variants
- Swift calling convention placeholder

### CallsiteRecord

Fields:

- callsite id
- procedure id
- address
- callee target object/address
- call kind
- argument value ids
- return value id
- clobber set
- calling convention
- confidence
- provenance

Call kinds:

- direct function
- import/stub
- ObjC dispatch
- C++ virtual
- Swift runtime
- kernel interface
- indirect function pointer
- unknown

### Call Effects

Every call has effects:

- register clobbers
- return value definition
- memory unknown write/read effects
- no-return marker
- exception/unwind marker later
- runtime-specific effects

No-return sources:

- symbol database
- import name heuristics
- KDK/dSYM facts
- user assertions
- observed terminal control flow

No-return facts must include confidence and source.

## Memory Model

### MemoryRegion

Kinds:

- stack
- global data
- constant data
- string
- ObjC metadata
- C++ metadata
- Swift metadata
- chained fixup pointer
- import pointer
- cache image range
- kernel table
- unknown

### MemoryAccessFact

Fields:

- access id
- procedure id
- instruction id
- operation: load/store/address-only
- address expression
- resolved target, optional
- region kind
- size
- type hints
- volatility hint
- provenance
- confidence

### Alias Policy

First wave:

- exact stack slots do not alias other exact stack slots
- exact constant/global addresses do not alias stack slots
- unknown pointer loads may alias unknown memory
- calls with unknown effects clobber unknown memory and call-clobbered registers

Renderer rule:

- never collapse memory expressions into named variables unless the alias and
  slot evidence is strong enough.

## Type And Symbol Enrichment

### Type Inputs

Use:

- ObjC type encodings
- method signatures
- import/stub known signatures later
- KDK/dSYM/DWARF exact signatures
- C++ demangling
- Swift demangling hints
- load/store widths
- ABI register roles
- user signatures

### TypePropagationFact

Fields:

- source type fact id
- target value/slot/procedure id
- propagation rule
- confidence
- contradiction ids
- stale flag

Propagation rules:

- user signature argument
- debug signature argument
- ObjC method encoding argument
- C++ demangled method argument
- call return value
- load/store width
- selector receiver type
- virtual slot target type

## Runtime Semantic Overlays

### Objective-C Overlay

Lift recognized dispatch into IR intrinsic:

```text
objc_msg_send(receiver, selector, args...)
```

Fields:

- selector id
- receiver evidence
- possible method ids
- bound method id, optional
- confidence

Renderer:

- exact binding can render `[receiver selector:arg]`
- unknown receiver can render `objc_msgSend(receiver, @selector(...), ...)`
- ambiguous binding shows warning marker and alternatives in inspector

### C++ Overlay

Lift virtual calls into:

```text
cxx_virtual_call(receiver, vtable_slot, args...)
```

Renderer:

- exact target can render `receiver->method(args)`
- ambiguous target keeps slot notation and alternatives
- constructors/destructors receive role markers

### Swift Overlay

Initial support:

- demangled symbol names
- Swift string references
- metadata placeholders
- runtime call labels

Renderer:

- use demangled names for calls when exact
- avoid Swift source syntax unless metadata support proves it

### Kernel Overlay

Lift recognized kernel interfaces:

- syscall handlers
- Mach traps
- MIG dispatch
- IOKit external methods

Renderer:

- label handlers and table dispatches
- keep assembly-linked evidence
- never invent high-level kernel API source when only table evidence exists

## Control-Structure Recovery

### Target Constructs

First wave:

- if/else
- early return
- while-like loops
- do/while-like loops
- switch when jump-table evidence is strong
- straight-line sequences
- calls and assignments

Deferred:

- exception handling
- complex irreducible CFG recovery
- C++ RAII destructor regions
- Swift structured control reconstruction

### Structuring Algorithm

1. Compute dominators and post-dominators.
2. Identify natural loops from back edges.
3. Identify if/else regions from conditional branches and join blocks.
4. Identify switch regions from jump-table facts.
5. Mark irreducible regions as unstructured.
6. Preserve source block mapping for every rendered construct.

### Unstructured Regions

Do not hide unstructured regions. Render:

- labels and gotos in decompiler-lite
- graph link
- warning marker
- underlying blocks

This is more honest than wrong structured C.

## Decompiler-Lite Summary

Before full pseudocode, ship decompiler-lite.

### Summary Contents

For each procedure:

- display name and aliases
- owner runtime objects
- signature candidates
- calling convention
- entry/exit summary
- callsites grouped by kind
- ObjC messages
- C++ virtual calls
- imports/cache links
- stack slots and locals
- strings and data refs
- unresolved indirect controls
- jump tables
- problems and confidence

### Example Shape

```text
procedure -[Greeter greeting]
  owner: ObjC method Greeter::greeting
  signature: id self, SEL _cmd -> id (from ObjC metadata, strong)
  calls:
    objc_msgSend @selector(stringWithFormat:) receiver=NSString strong
    _objc_retainAutoreleasedReturnValue exact import
  data:
    cfstring "hello"
  unresolved:
    none
```

This is extremely useful even before a C-like renderer exists.

## Pseudocode Renderer

### RenderNode

Kinds:

- function
- block
- statement
- expression
- call
- assignment
- declaration
- branch
- loop
- switch
- return
- comment
- warning
- unresolved

Every render node stores:

- source instruction ids
- source block ids
- fact ids
- route id
- confidence
- hover text

### Renderer Rules

- Render exact facts cleanly.
- Render inferred facts with subtle markers.
- Render ambiguous facts with expandable alternatives.
- Render unresolved regions explicitly.
- Link every variable, call, selector, type, and address to inspector routes.
- Keep raw assembly fallback one click away.

### Display Modes

Modes:

- summary
- pseudocode with uncertainty markers
- pseudocode plus assembly interleave
- IR debug view
- data-flow view

The IR debug view is for maintainers and advanced users. It is critical for
trust during early decompiler work.

## User Hints And Reanalysis

User hints:

- set function start/end
- set no-return
- rename variable
- set stack slot type
- set function signature
- bind ObjC receiver/selector
- bind C++ virtual call target
- mark table as switch
- mark bytes as data/code/undefined

Reanalysis rules:

- hints are transactions
- affected facts are invalidated
- dependent stages rerun incrementally
- stale pseudocode shows stale marker until refreshed

## APIs

### Procedure APIs

```text
GET /api/projects/{project_id}/procedures
GET /api/projects/{project_id}/procedures/{procedure_id}
GET /api/projects/{project_id}/procedures/{procedure_id}/cfg
GET /api/projects/{project_id}/procedures/{procedure_id}/summary
GET /api/projects/{project_id}/procedures/{procedure_id}/problems
```

### IR APIs

```text
POST /api/projects/{project_id}/procedures/{procedure_id}/lift-ir
GET /api/projects/{project_id}/procedures/{procedure_id}/ir
GET /api/projects/{project_id}/procedures/{procedure_id}/def-use
GET /api/projects/{project_id}/procedures/{procedure_id}/stack-frame
```

### Pseudocode APIs

```text
POST /api/projects/{project_id}/procedures/{procedure_id}/decompile
GET /api/projects/{project_id}/procedures/{procedure_id}/pseudocode
GET /api/projects/{project_id}/procedures/{procedure_id}/render-map
```

### Hint APIs

```text
POST /api/projects/{project_id}/procedures/{procedure_id}/hints/no-return
POST /api/projects/{project_id}/procedures/{procedure_id}/hints/signature
POST /api/projects/{project_id}/procedures/{procedure_id}/hints/stack-slot
POST /api/projects/{project_id}/procedures/{procedure_id}/hints/switch-table
POST /api/projects/{project_id}/procedures/{procedure_id}/hints/bounds
```

## CLI Commands

```text
damsel project procedures <project> [--source SOURCE] [--query QUERY]
damsel project procedure <project> <procedure-id-or-address>
damsel project cfg <project> <procedure-id-or-address> --format json|dot
damsel project ir <project> <procedure-id-or-address>
damsel project def-use <project> <procedure-id-or-address>
damsel project stack <project> <procedure-id-or-address>
damsel project summary <project> <procedure-id-or-address>
damsel project pseudocode <project> <procedure-id-or-address>
damsel project hint signature <project> <procedure> ...
damsel project hint stack-slot <project> <procedure> ...
damsel project hint no-return <project> <procedure> ...
```

Existing `damsel disasm --analysis` remains a point-query command. Project
commands are the durable decompiler path.

## Workbench Integration

### Procedure Route

Default layout:

- assembly or summary in center
- CFG tab available
- decompiler-lite tab available
- inspector modules for calls, locals, types, runtime overlays, problems
- feature map highlights procedure range

### Pseudocode View

Requirements:

- click pseudocode expression selects source instruction range
- hover shows confidence and provenance
- warning markers route to problems
- unresolved regions expand to IR/assembly
- variable rename uses transaction API
- signature edit uses transaction API

### Graph Sync

Rules:

- selecting graph block selects assembly range
- selecting pseudocode construct highlights graph blocks
- graph node problems appear in inspector
- unresolved indirect controls remain visible in graph

## Storage

Tables or collections:

- procedure seeds
- procedure records
- basic blocks
- edges
- semantic instructions
- IR functions
- IR blocks
- IR statements
- IR values
- def-use records
- stack frames
- stack slots
- callsites
- call effects
- memory access facts
- type propagation facts
- render nodes
- pseudocode documents
- decompiler problems
- user hints

Invalidation dependencies:

- source bytes
- byte classifications
- symbols/names
- procedure seeds
- architecture semantics version
- ABI profile version
- type facts
- runtime object facts
- user hints
- symbolication profile

## Test Plan

### Unit Tests

- procedure seed precedence
- recursive procedure recovery
- boundary conflict recording
- CFG normalization
- arm64 branch semantics
- ADRP/add target materialization
- stack pointer delta tracking
- FP-relative stack slot grouping
- callsite argument mapping
- no-return propagation
- switch-table validation
- SSA phi insertion
- ObjC dispatch intrinsic lifting
- C++ virtual call intrinsic lifting
- PAC intrinsic lifting
- pseudocode render source map

### Fixture Tests

Fixtures:

- simple linear function
- conditional branch
- loop
- switch/jump table
- indirect dispatch
- ObjC method
- ObjC message send
- C++ vtable/virtual call
- import/stub call
- no-return call
- malformed/conflicted procedure
- arm64e authenticated indirect call when available
- kernel table handler when fixture is available

### Snapshot Tests

Snapshot:

- procedure record
- CFG
- IR
- def-use
- stack frame
- decompiler-lite summary
- pseudocode render map
- problem list

### Differential/Oracle Tests

Compare selected facts against:

- known fixture source structure
- LLVM objdump for instruction boundaries where useful
- current Damsel disassembly JSON contracts
- KDK/dSYM names when exact fixtures exist

Do not compare pseudocode text against one "golden C" too rigidly. Compare
semantic render nodes, source maps, and facts.

### Performance Tests

Budgets:

- procedure recovery can be canceled
- lifting one ordinary function stays interactive
- lifting huge functions reports progress
- decompiler-lite summary loads before full pseudocode
- pseudocode renderer does not block Workbench main thread
- cache-wide decompiler jobs are not launched implicitly

## Milestones

### Milestone D1: Durable Procedure Store

Deliver:

- procedure seeds
- procedure records
- basic blocks
- edges
- conflicts/problems
- CLI/API procedure detail

Exit:

- current disasm analysis fixtures can be represented as procedure records

### Milestone D2: Architecture Semantics

Deliver:

- arm64/arm64e semantic instruction model
- branch semantics
- memory/register read/write facts
- prolog/epilog/thunk hints

Exit:

- disassembly no longer depends on rendered strings for control-flow analysis

### Milestone D3: Low-Level IR

Deliver:

- IR function/block/value/statement/terminator records
- source instruction mapping
- IR JSON snapshots

Exit:

- simple fixtures lift to stable IR with unresolved markers where needed

### Milestone D4: Def-Use, Stack, ABI

Deliver:

- SSA/def-use
- stack frame model
- callsite argument/return facts
- no-return facts

Exit:

- fixture functions show locals, arguments, and call effects with confidence

### Milestone D5: Runtime Overlays

Deliver:

- ObjC dispatch intrinsic
- C++ virtual call intrinsic
- PAC intrinsic
- Swift symbol call labels
- kernel interface intrinsic seeds

Exit:

- ObjC and C++ fixtures produce runtime-aware summaries

### Milestone D6: Decompiler-Lite

Deliver:

- procedure summary view
- Workbench route
- CLI summary command
- problem and confidence display

Exit:

- useful function understanding without C-like pseudocode

### Milestone D7: Pseudocode Renderer

Deliver:

- structured render nodes
- source maps
- uncertainty markers
- assembly/graph sync
- user hint integration

Exit:

- pseudocode is useful, inspectable, and honest on fixture families

## Risks

### Risk: String-Based Analysis Trap

If branch, operand, and call semantics are inferred from rendered strings,
future decompiler work will be fragile.

Mitigation:

- introduce semantic instruction records before IR

### Risk: False Confidence In Pseudocode

Pretty output can be more harmful than assembly if it hides unresolved
indirection or weak type facts.

Mitigation:

- render uncertainty and alternatives
- ship decompiler-lite first
- keep assembly/source mapping visible

### Risk: Full Decompiler Scope Explosion

Full C/C++/Swift recovery is enormous.

Mitigation:

- prioritize Apple runtime understanding and useful summaries
- defer recompilable source and hard language features

### Risk: Cache-Wide Cost Explosion

Decompiling a whole dyld shared cache or kernel collection by default would be
slow and memory-heavy.

Mitigation:

- decompile on demand
- cache summaries separately
- page results
- make jobs cancellable

### Risk: Type Pollution

Weak type inference can overwrite exact debug/user facts.

Mitigation:

- use precedence and provenance from the type/symbolication system
- never replace user facts silently

## Completion Audit

Do not call decompiler support "implemented" until evidence proves:

- procedure records persist beyond one disassembly response
- CFG recovery has conflict/problem handling
- arm64/arm64e semantic instructions exist
- low-level IR exists and maps back to instructions
- def-use or SSA exists for scalar registers
- stack frame recovery exists
- ABI callsite facts exist
- ObjC dispatch is represented as an IR intrinsic or equivalent semantic fact
- pseudocode/decompiler-lite render nodes link back to source facts
- uncertainty is visible in UI/API
- user hints trigger scoped reanalysis
- fixture and performance tests cover the claimed support

Until then, Damsel has strong disassembly analysis, not Hopper-grade
decompilation.
