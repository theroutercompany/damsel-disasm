# Decompiler And IR Strategy

Status: Long-range architecture plan. This document is not a promise to ship a
full decompiler first. It defines how Damsel should move toward Hopper-like
pseudocode without making unsupported claims.

Purpose: Hopper publicly markets pseudocode/decompilation, but its tutorial and
FAQ also warn that decompiled output is imperfect. Damsel should treat
pseudocode as the end of a chain of verified analysis layers: function
recovery, xrefs, value flow, type facts, stack frames, ABI modeling, runtime
semantics, and provenance-aware uncertainty.

Implementation companions:

- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
  defines the durable procedure records, seeds, bounds, blocks, call graph,
  locals, aliases, conflict handling, and runtime ownership that decompiler work
  depends on.
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
  expands this strategy into staged implementation contracts for procedure
  recovery, architecture semantics, low-level IR, SSA, stack/local recovery,
  ABI modeling, runtime overlays, control-structure recovery, and pseudocode.
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
  records the public Hopper evidence behind the ordering: Hopper's SDK exposes
  procedure, CPU, type, and transaction layers, while the decompiler AST remains
  private and late-stage.

## Thesis

Decompiler work should not start with pretty C-like output. It should start
with a stable intermediate representation and a UI contract that tells the
truth about confidence.

The competitive path:

1. Structured function summaries.
2. Procedure-local stack/register/value facts.
3. Low-level IR for AArch64/AArch64e.
4. SSA and data-flow over recovered functions.
5. ABI-aware argument, return, and call-effect modeling.
6. ObjC/C++/Swift/kernel semantic overlays.
7. Control-structure recovery.
8. Pseudocode rendering with uncertainty markers.
9. User hints and reanalysis loops.

Damsel can be competitive before full pseudocode if its summaries, xrefs,
runtime object links, graph view, and inspector are excellent. Pseudocode is a
power feature; truthful analysis is the foundation.

## Non-Goals For The First Decompiler Push

Do not attempt:

- recompilable source recovery
- exact source-level variable names without debug/user evidence
- generic multi-architecture decompiler
- whole-program optimization-grade alias analysis
- C++ template reconstruction
- Swift source reconstruction
- binary rewriting through decompiler edits
- hiding unresolved or contradictory evidence

First output can be "decompiler-lite" if it is useful, inspectable, and honest.

## Required Prerequisites

Decompiler work should wait for:

- persistent function records
- full-function CFGs
- xref/reverse-xref index
- byte classification map
- semantic instruction spans
- stack-frame model
- ABI/calling-convention model
- type database
- provenance/confidence facts
- annotation and user-hint transactions
- problem list
- fixture-backed jump-table and indirect-branch analysis

Without those, pseudocode becomes a detached rendering pass that cannot be
debugged or corrected.

## Pipeline Overview

```text
FunctionRecord
  |
  v
Instruction semantics
  |
  v
Low-level IR blocks
  |
  v
CFG normalization
  |
  v
SSA / def-use graph
  |
  v
Value-flow and stack recovery
  |
  v
Type and ABI enrichment
  |
  v
Runtime semantic overlays
  |
  v
Control-structure recovery
  |
  v
Decompiler-lite summary / pseudocode
```

Every stage emits facts, problems, and metrics. No stage should only emit a
string.

## Low-Level IR

### Design Goals

The low-level IR should be:

- deterministic
- architecture-aware enough for AArch64/AArch64e
- simple enough to debug in fixtures
- explicit about memory, registers, flags, calls, and unknowns
- able to represent partial semantics
- serializable for snapshots
- connected to source instruction addresses

### IR Function

Fields:

- function id
- source function record id
- entry block id
- blocks
- variables or temporaries
- address-space references
- source instruction mapping
- analysis pass version
- problems

### IR Block

Fields:

- block id
- source basic block id
- statements
- terminator
- predecessor/successor links
- dominance metadata, later
- loop metadata, later

### IR Values

Kinds:

- constant
- register
- temporary
- stack slot
- memory load
- address
- symbol
- selector
- class
- type reference
- unknown
- poison or invalid

Each value should include:

- bit width
- signedness when known
- provenance
- confidence
- source instruction span

### IR Statements

Kinds:

- assign
- load
- store
- arithmetic op
- bitwise op
- compare
- cast/extend/truncate
- address computation
- call
- intrinsic
- ptrauth operation
- fixup/binding resolution
- annotation marker

### IR Terminators

Kinds:

- return
- branch
- conditional branch
- switch
- call-and-return
- tail call
- indirect branch
- trap
- unresolved

The unresolved terminator is important. It prevents the renderer from inventing
control structure.

## AArch64/AArch64e Semantics

### Registers

Track:

- general registers
- SP
- FP
- LR
- vector registers where relevant
- condition flags when needed

Rules:

- `xN`/`wN` aliasing must be explicit
- zero register must not become a mutable register
- SP and FP receive stack-frame-specific treatment
- call-clobbered registers follow ABI facts

### Instructions

Initial semantic coverage should focus on instructions that feed
reverse-engineering value:

- `adr`, `adrp`, `add`, `sub`
- `ldr`, `str`, `ldp`, `stp`
- `mov`, `movk`, `movz`, `orr`
- `cmp`, `tst`, `ccmp`
- `b`, `bl`, `blr`, `br`, `ret`
- conditional branches
- table/jump instructions
- PAC/authenticated branch and pointer instructions
- ObjC and dyld stub patterns

Partial semantics are acceptable if marked. Unknown instruction semantics should
emit a problem or low-confidence fact, not silently disappear.

### Pointer Authentication

Represent:

- raw pointer value
- authenticated/stripped target when known
- key/discriminator class when inferable
- source fixup or instruction
- failure reason

IR should include PAC-aware intrinsics rather than pretending authentication is
ordinary arithmetic.

Example intrinsic classes:

- `ptrauth_strip`
- `ptrauth_auth`
- `ptrauth_sign`
- `ptrauth_branch_target`
- `authenticated_objc_selector_trampoline`

These intrinsics feed xrefs, target display, and problems.

## SSA And Def-Use

### Why SSA

SSA makes it easier to:

- recover constant addresses
- propagate selector/class values
- resolve indirect branches
- identify stack variables
- summarize call arguments
- detect dead/unreachable code
- expose contradictions

### SSA Scope

Start with per-function SSA. Avoid interprocedural SSA until function summaries
and call effects are stable.

Required components:

- block dominance
- phi nodes for register values
- use-def chains
- def-use chains
- memory versioning approximation
- call clobber model

### Memory Modeling

Use a conservative staged model:

Stage 1:

- stack memory
- constant global loads
- literal pools
- known stub/import tables

Stage 2:

- object fields where ObjC/C++ evidence exists
- vtable loads
- selector/class refs
- chained-fixup targets

Stage 3:

- broader alias analysis
- struct-field type propagation
- interprocedural summaries

Do not pretend every memory load is known. Unknown memory should remain a first
class value.

## Stack Frame Recovery

### Goals

Recover:

- frame size
- saved registers
- local stack slots
- argument spill slots
- stack variable lifetimes
- stack variable types when known
- user names

Inputs:

- prologue/epilogue patterns
- SP/FP arithmetic
- load/store offsets
- call ABI
- compact unwind/DWARF/KDK where available
- user annotations

Outputs:

- `StackFrameRecord`
- `StackSlotRecord`
- variable name facts
- variable type facts
- problems for ambiguous stack use

Rules:

- user stack-variable names override inferred names
- debug/KDK variable names do not silently overwrite user names
- stack slots can be shown in assembly before pseudocode exists

## Calling Convention And Call Effects

### Required Models

Model:

- platform ABI
- argument registers
- return registers
- call-clobbered registers
- callee-saved registers
- variadic call hints
- ObjC `objc_msgSend` family conventions
- C++ method `this` conventions
- block invocation conventions
- syscall/Mach trap conventions later

### Function Summary Facts

For each function:

- likely arguments read
- return values written
- clobbered registers
- calls made
- imported functions called
- local functions called
- strings referenced
- ObjC selectors/classes referenced
- C++ virtual calls
- kernel objects referenced
- unresolved indirect calls

These facts are useful before pseudocode and remain useful after pseudocode.

## Type System Integration

The decompiler should use the project type database, not invent private types.

Type sources:

- user-authored types
- KDK/DWARF/dSYM types
- ObjC type encodings
- C++ RTTI/mangled names/vtables
- Swift demangled symbol hints
- ABI defaults
- inferred load/store widths
- string/data classifications

Type fact fields:

- type id
- target object/range/value
- source
- confidence
- precedence
- contradiction relation

Rules:

- type propagation is provenance-aware
- losing type facts remain inspectable
- unresolved type gaps are visible in UI
- pseudocode renderer can show `unknown` or `opaque` types explicitly

## ObjC Semantic Overlay

ObjC can make Damsel better than a generic pseudocode renderer.

Recognize:

- `objc_msgSend`
- `objc_msgSendSuper`
- `objc_alloc`
- `objc_retain`/`release` family
- block invocation
- selector/class refs
- IMP calls
- category method overlays

Decompiler-lite output should show:

- receiver evidence
- selector
- candidate methods
- confidence
- unresolved receiver reason

Example summary style:

```text
Messages:
  strong: [self initWithFrame:] at 0x...
  inferred: [candidate setNeedsDisplay:] at 0x... receiver from x20
  ambiguous: selector "copy" with 4 implementers
```

Do not render a single Objective-C method call in pseudocode when receiver
evidence is ambiguous unless the uncertainty is visible.

## C++ / ObjC++ Semantic Overlay

Recognize:

- `this` pointer patterns
- constructors/destructors
- vtable loads
- virtual call slots
- RTTI references
- typeinfo names
- inheritance edges
- IOKit-style object patterns

Output:

- virtual call candidate table
- vtable slot references
- class/method names from demangling
- uncertain inheritance notes
- mixed ObjC++ method context where applicable

Pseudocode should not fabricate class hierarchy. Use "candidate" language when
evidence is incomplete.

## Kernel Semantic Overlay

Kernel/XNU analysis needs domain-specific summaries:

- syscall table references
- Mach trap references
- MIG routine references
- kext ownership
- IOKit class/method facts
- panic/log strings
- lock/copyin/copyout patterns later
- entitlement or sandbox-relevant strings later

Decompiler-lite summaries should make kernel context visible before full
pseudocode.

## Control Structure Recovery

### First Targets

Recover:

- if/else
- simple loops
- early returns
- switch/jump tables
- tail calls
- error paths

### Required Inputs

- full-function CFG
- dominance/postdominance
- loop headers
- branch condition semantics
- jump-table records
- indirect-branch hints
- unreachable/dead-code facts

### Dead/Unreachable Code

Hopper publicly warns that pseudocode can omit dead/unreachable code. Damsel
should expose this explicitly:

- "hidden unreachable block" indicator
- toggle to include unreachable blocks
- problem when unreachable decision depends on weak evidence
- block-level provenance in pseudocode

Do not silently drop code from user-visible high-level views.

## Pseudocode Renderer

### Rendering Principles

Pseudocode should be:

- useful
- navigable
- uncertainty-aware
- tied to source addresses
- synchronized with assembly and graph
- editable only through annotations/hints, not free text initially

### Line Model

Each pseudocode line should include:

- line id
- source address range(s)
- related IR statements
- confidence
- warnings/problems
- semantic spans
- route targets

This allows:

- clicking pseudocode to select assembly
- hovering for provenance
- showing xrefs
- explaining unresolved values
- future MCP access without scraping text

### Display Conventions

Use explicit uncertainty markers:

- `unknown`
- `candidate`
- `ambiguous`
- `unresolved`
- `/* inferred */`
- `/* user asserted */`

Avoid misleading precision:

- do not invent variable names
- do not hide multiple candidate targets
- do not collapse uncertain types into exact-looking C
- do not present Objective-C calls as proven if receiver is weak

## User Hints And Reanalysis

Decompiler quality improves when users can correct analysis.

Hints:

- mark function start/end
- mark code/data/string
- add manual xref
- identify jump table base/count/stride
- bind ObjC receiver/selector
- assign function signature
- assign calling convention
- name stack variable
- assign type to address/range/value
- choose virtual call target

Flow:

1. Problem appears.
2. User applies hint.
3. Hint records annotation transaction.
4. Dependent facts are invalidated.
5. Reanalysis runs targeted pass.
6. Updated IR/summary/pseudocode records provenance.

## UI Surfaces

### Function Summary

Ship before pseudocode:

- arguments/returns
- calls
- strings
- ObjC messages
- C++ virtual calls
- kernel facts
- stack frame
- unresolved facts
- problems and hints

### IR View

Developer/power-user view:

- basic blocks
- IR statements
- use-def chains
- types
- problems
- source instruction links

This is invaluable for debugging decompiler output.

### Pseudocode View

Later view:

- synchronized line selection
- uncertainty gutter
- address gutter
- problem indicators
- source/assembly/graph split controls
- toggle raw/inferred names
- toggle dead/unreachable blocks

## Validation

### Fixture Tiers

Tier 1:

- straight-line arithmetic
- simple branches
- simple calls
- stack local load/store
- string load

Tier 2:

- loops
- switch/jump table
- tail call
- indirect call
- ObjC message send
- C++ vtable call

Tier 3:

- arm64e PAC targets
- chained fixup pointer
- kernel-style table
- KDK/DWARF-enriched function
- ambiguous receiver
- dead/unreachable block

### Snapshot Outputs

For every fixture:

- function record
- CFG
- IR
- SSA summary
- stack frame
- function summary
- pseudocode, when available
- problems

### Negative Tests

Prove Damsel does not overclaim:

- unresolved indirect branch remains unresolved
- ambiguous ObjC receiver stays ambiguous
- mismatched type facts remain contradictory
- unsupported instruction emits problem
- dead-code toggle preserves hidden block visibility

## Milestones

### D0: Function Summary Foundation

- persistent function summaries
- stack frame facts
- call/argument facts
- unresolved-fact section
- Workbench summary pane

### D1: Low-Level IR

- AArch64 instruction semantics for common patterns
- IR snapshot fixtures
- source mapping
- unknown semantics problems

### D2: SSA And Data Flow

- register SSA
- basic stack memory SSA
- constant/address propagation
- call clobber model
- selector/class propagation

### D3: Type And ABI Integration

- function signatures
- calling conventions
- stack variables
- type propagation
- ObjC/C++ overlays

### D4: Control Structuring

- if/else
- loops
- switches
- dead/unreachable handling
- hints

### D5: Pseudocode Preview

- line model
- synchronized UI
- uncertainty markers
- source mapping
- fixture comparison

### D6: Apple-Specific Pseudocode Quality

- ObjC message summaries
- C++ virtual call summaries
- kernel facts
- PAC/fixup target rendering

## Completion Criteria

Do not call Damsel's decompiler competitive until:

- function summaries are useful without pseudocode
- IR is inspectable and fixture-tested
- pseudocode lines map back to assembly/graph
- unresolved facts are visible
- user hints improve output through reanalysis
- ObjC/C++/kernel facts appear in high-level views
- dead/unreachable code is not silently lost
- type provenance is visible
- misleading recompilable-source claims are avoided
