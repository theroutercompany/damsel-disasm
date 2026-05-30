# Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec

Status: implementation-grade planning spec. This is not an implementation
claim.

Purpose: define the product, data, API, persistence, analysis, and native
Workbench changes required to make Damsel's eventual pseudocode feature feel
like a first-class Hopper-class representation. The companion IR specs describe
how to recover semantics. This spec describes how that recovered meaning becomes
a usable reverse-engineering workspace for macOS, iOS, dyld shared caches,
Objective-C, Objective-C++, C++, and XNU-style system binaries.

## Why This Spec Exists

The existing decompiler documents answer the backend question:

- how functions become durable procedure records
- how AArch64 and AArch64e instructions become architecture semantics
- how low-level IR, SSA, stack slots, argument facts, control structures, and
  runtime overlays are computed
- how pseudocode should remain honest about uncertainty

That is necessary, but not enough to compete with Hopper.

Hopper's public product surface makes pseudocode an interactive representation
inside a native document app. Its homepage says Hopper can show assembly, CFG,
and pseudocode for a procedure at the same time. Its tutorial describes
pseudocode as another display mode tied to procedure recovery, basic blocks,
stack usage, dead-code handling, and view synchronization. Its changelog shows
that decompiler quality, switch/case recovery, Objective-C ivar display,
large-function performance, memory footprint, hangs, and view synchronization
are recurring product work, not a single backend feature. Its FAQ is blunt that
decompiled output is not lost source code and usually is not directly
compilable.

Damsel therefore needs a decompiler workbench, not merely a decompiler pass.
The user should be able to open a function, see a pseudocode representation,
understand where each line came from, rename variables, set types, jump between
assembly/graph/hex/source/debug info, inspect runtime evidence, re-run analysis
with hints, and trust the UI when the answer is partial.

## Companion Documents

Read this spec with:

- [Decompiler And IR Strategy](decompiler-ir-strategy.md)
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
- [Type And Symbolication System Plan](type-symbolication-system-plan.md)
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
- [API And Data Contracts](api-data-contracts.md)
- [Quality And Validation Plan](quality-validation-plan.md)
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md)

This spec should be treated as the missing bridge between the decompiler backend
and the app experience.

## Source-Grounded Product Requirements

### Hopper Evidence To Respect

Public Hopper evidence implies these requirements:

- Pseudocode is a view mode, not a disconnected export.
- Pseudocode depends on procedures, basic blocks, stack usage, and type
  classification.
- Pseudocode must be synchronized with assembly, CFG, and hex views.
- Dead-code handling must be visible because the decompiler may discard code it
  believes is unreachable.
- The inspector should adapt to the selected pseudocode element.
- Navigation, naming, tags, comments, references, and undo/redo must work across
  representations.
- Switch/case recovery needs explicit user hints.
- Objective-C, Swift demangling, authenticated selector trampolines, dyld cache
  navigation, kernel cache support, and large binary performance are all
  competitive Apple-lane concerns.
- Decompiler performance work must include large functions, memory footprint,
  cancellation, hangs, and export limits.
- The app must avoid implying that pseudocode is exact source code.

### Damsel-Specific Niche

Damsel should not chase Hopper by becoming a generic clone. The niche should be:

- Apple-system-first evidence. Pseudocode should show ObjC, C++, dyld cache,
  KDK/dSYM, arm64e, chained fixup, PAC, IOKit, and XNU provenance inline.
- Truthful decompilation. Every pseudocode region should expose confidence,
  source mappings, unresolved facts, and contradiction reasons.
- Native Workbench coherence. Pseudocode should participate in the same route,
  command, selection, undo, job, problem, and search models as assembly, graph,
  hex, feature map, and runtime browsers.
- Large artifact responsiveness. Decompiled views should remain usable on dyld
  shared caches, driver caches, kernel collections, KDK overlays, and app
  bundles without waiting for whole-world analysis.
- Research-grade correction loops. User hints should produce auditable
  transactions and reanalysis, not untracked UI state.

## Product Thesis

Pseudocode is not a string. It is a representation graph.

Each rendered line should be backed by:

- function identity
- IR node or control-structure node identity
- source instruction spans
- basic block membership
- address ranges
- byte ranges where possible
- stack/local variable bindings
- type and signature facts
- runtime object references
- confidence and provenance
- decompiler issue records
- invalidation dependencies

This lets the Workbench answer questions that matter in real reverse
engineering:

- Which instructions produced this line?
- Which ObjC selector, class, ivar, or method evidence made this call readable?
- Is this variable a stack slot, register temporary, global, ivar, argument, or
  inferred alias?
- Did a user rename, dSYM, KDK, DWARF, symbol, ObjC type encoding, C++ RTTI, or
  heuristic produce the displayed type?
- Did the decompiler drop unreachable code?
- Did a switch hint, no-return override, or function-bound edit affect this
  block?
- Can this line be trusted enough for a report?

If Damsel cannot answer those questions, pseudocode will be pretty but shallow.

## Non-Goals

The first Workbench decompiler feature must not promise:

- recompilable source
- source-level Swift reconstruction
- perfect C++ template or exception recovery
- perfect alias analysis
- decompiler-edited binary rewriting
- automatic proof of semantic equivalence
- whole-cache eager decompilation
- hiding unknowns behind plausible syntax
- private user source paths in exported reports without explicit consent

The correct early posture is: useful, inspectable, partial, and correctable.

## Current Damsel Baseline

### Reusable Rust Analysis Surface

Current Damsel already has analysis outputs that can seed pseudocode:

- decoded instructions with addresses, encodings, operands, annotations, and
  references
- basic blocks
- control-flow edges
- branch targets
- direct calls
- indirect controls
- data references
- import references
- Objective-C references
- dyld shared-cache links
- recovered values
- jump-table records
- summary and overview data

These are not yet enough for pseudocode, but they provide the first
source-mapped evidence layer.

### Reusable Apple Runtime Surface

Current model types already represent Apple runtime facts such as:

- Objective-C classes
- protocols
- categories
- methods
- selectors
- type encodings
- implementation addresses
- message-send references
- cache linkage records

This is a strong advantage. Pseudocode can use it earlier than a generic
decompiler would, especially for ObjC and ObjC++ binaries where selector and
type-encoding evidence often survives stripping.

### Workbench Gap

The current Workbench direction has the right native shape, but it does not yet
have a decompiler representation model. Missing pieces include:

- project-backed decompiler jobs
- pseudocode route state
- pseudocode line models
- source maps from pseudocode to assembly/graph/hex
- variable/type editing stores
- confidence and issue display
- decompiler cache state
- stale-result handling
- command integration
- decompiler export and evidence packaging

The backend can begin before the app is complete, but the product will not feel
competitive until those Workbench pieces exist.

## Representation Architecture

### Core Objects

Add a decompiler representation layer with these durable object families:

- `DecompilerFunctionView`
- `PseudocodeLine`
- `PseudocodeToken`
- `DecompilerSourceMap`
- `DecompilerIssue`
- `DecompilerConfidence`
- `DecompilerVariableBinding`
- `DecompilerTypeUse`
- `DecompilerRuntimeOverlay`
- `DecompilerControlRegion`
- `DecompilerJob`
- `DecompilerCacheEntry`
- `DecompilerHint`
- `DecompilerExport`

These should live above raw IR. The UI should not render directly from SSA or
backend-internal AST objects. It should render from stable, serializable,
versioned representation contracts.

### Object Identity Rules

Every decompiler object needs stable identity:

- project id
- source id
- image id when applicable
- function id
- analysis run id
- decompiler pass version
- local object id

Line ids must remain stable across small changes when possible. If a user
renames a variable, expands a source map, opens a popover, or exports a report,
the selection should not evaporate after a background refresh unless the
underlying line truly disappeared.

### Representation Versions

Version every serialized contract:

- `decompiler_model_version`
- `ir_model_version`
- `control_recovery_version`
- `runtime_overlay_version`
- `renderer_version`
- `source_map_version`

The version record should be included in cached results, exports, snapshots,
tests, and problem reports. Pseudocode regressions are otherwise impossible to
debug.

## DecompilerFunctionView

`DecompilerFunctionView` is the Workbench-facing aggregate for one function.

Fields:

- `id`
- `project_id`
- `source_id`
- `image_id`
- `function_id`
- `entry_address`
- `display_name`
- `signature`
- `signature_confidence`
- `language_flavor`
- `lines`
- `variables`
- `parameters`
- `returns`
- `control_regions`
- `runtime_overlays`
- `issues`
- `source_maps`
- `metrics`
- `cache_state`
- `invalidated_by`
- `generated_at`
- `analysis_run_id`
- `model_versions`

`language_flavor` should be an enum:

- `c_like`
- `objc_method_like`
- `objc_function_like`
- `cpp_method_like`
- `objcxx_method_like`
- `kernel_c_like`
- `swift_adjacent`
- `unknown`

The flavor is not a source-language claim. It is a rendering mode that controls
tokens, runtime overlays, and standard library naming.

## PseudocodeLine

Each line is an addressable object, not just text.

Fields:

- `id`
- `ordinal`
- `indent`
- `kind`
- `tokens`
- `address_range`
- `instruction_ids`
- `basic_block_ids`
- `ir_node_ids`
- `control_region_ids`
- `variable_ids`
- `runtime_object_ids`
- `issue_ids`
- `confidence`
- `is_user_affected`
- `is_stale`
- `fold_state`
- `display_flags`

Line kinds:

- `signature`
- `declaration`
- `statement`
- `call`
- `objc_message`
- `cpp_virtual_call`
- `assignment`
- `return`
- `branch`
- `switch_header`
- `case_label`
- `loop_header`
- `goto`
- `inline_asm`
- `unresolved_region`
- `dead_code_region`
- `comment`
- `source_line_annotation`
- `problem_banner`

The Workbench should be able to select a line and ask for:

- corresponding assembly rows
- corresponding graph nodes
- corresponding hex bytes
- associated source lines, if debug info exists
- associated runtime objects
- associated issues
- commands valid for the current line

## PseudocodeToken

Tokens enable selection, styling, hover cards, rename actions, copy behavior,
and evidence display.

Fields:

- `id`
- `line_id`
- `kind`
- `text`
- `semantic_id`
- `route`
- `confidence`
- `provenance`
- `source_span`
- `hover_summary`
- `editable`
- `copy_role`

Token kinds:

- `keyword`
- `type_name`
- `function_name`
- `variable`
- `parameter`
- `field`
- `ivar`
- `selector`
- `class_name`
- `protocol_name`
- `namespace`
- `literal`
- `address`
- `operator`
- `punctuation`
- `comment`
- `unknown`
- `issue_marker`

Editable token examples:

- variable name
- parameter name
- function name
- inferred type
- stack local name
- ObjC selector alias note
- user comment

Non-editable token examples:

- syntax punctuation
- generated `if`, `for`, `while`, `return`
- immutable source address token
- analysis warning marker

## Source Maps

Source maps are the heart of the feature.

### Required Map Types

The decompiler layer should emit:

- pseudocode line to instruction ids
- pseudocode line to address ranges
- pseudocode line to basic block ids
- pseudocode line to IR statement ids
- pseudocode line to graph node ids
- pseudocode line to hex byte ranges
- token to runtime object
- token to symbol/type/signature fact
- variable to stack/register/global/ivar storage
- control region to CFG region
- decompiler issue to evidence span
- debug source line to pseudocode line, when available

### Selection Semantics

When the user selects a pseudocode line:

- assembly highlights the contributing instruction rows
- graph highlights the owning block or control region
- hex highlights the backing byte ranges when safe
- feature map highlights the function and block region
- inspector switches to decompiler context
- runtime browsers highlight linked ObjC/C++/kernel objects

When the user selects an assembly instruction:

- pseudocode highlights the nearest line with that instruction in its source map
- if multiple pseudocode lines refer to the instruction, the UI shows a small
  ambiguous mapping control
- if no pseudocode line owns the instruction, pseudocode shows an unresolved or
  omitted-code marker

This must be deterministic. View synchronization cannot depend on current scroll
position or string search.

## Confidence Model

Pseudocode confidence should be visible but not noisy.

### Confidence Levels

Use a small enum:

- `exact_evidence`
- `strong_inference`
- `weak_inference`
- `conflicting_evidence`
- `unknown`
- `user_asserted`

Each level should have:

- display color
- inspector explanation
- export wording
- acceptance-test expectations

### Confidence Dimensions

Confidence should be tracked separately for:

- function boundary
- control flow
- stack frame
- variable identity
- variable type
- call target
- call signature
- return value
- memory region
- runtime overlay
- source line mapping
- rendered syntax

A line can have strong control-flow confidence but weak type confidence. The UI
should not collapse that into one vague warning.

### User-Facing Display

Default display:

- clean pseudocode text
- subtle gutter marks for low-confidence or user-affected lines
- hover cards for confidence details
- inspector section for the selected line
- Problems pane for serious issues

Avoid painting every line with warnings. The normal reading flow should remain
calm, with uncertainty available on demand.

## Decompiler Issues

Issues are structured records.

Fields:

- `id`
- `severity`
- `kind`
- `title`
- `summary`
- `affected_line_ids`
- `affected_instruction_ids`
- `affected_block_ids`
- `suggested_commands`
- `provenance`
- `first_seen_analysis_run`
- `resolved_by`

Issue kinds:

- `function_boundary_conflict`
- `unrecovered_indirect_branch`
- `switch_case_unresolved`
- `dead_code_removed`
- `unreachable_code_disputed`
- `stack_slot_conflict`
- `variable_lifetime_conflict`
- `call_signature_unknown`
- `objc_msgsend_unresolved`
- `objc_ivar_unresolved`
- `cpp_virtual_call_unresolved`
- `pac_target_unresolved`
- `chained_fixup_unresolved`
- `debug_info_mismatch`
- `decompiler_timeout`
- `decompiler_memory_limit`
- `internal_invariant_failure`

Problems should be navigable and actionable. Each issue should expose one or
more commands, such as set switch hint, mark no-return, edit function bounds,
set type, rename variable, or re-run decompiler.

## Workbench UX

### Top-Level Placement

Pseudocode should be a peer representation beside:

- Assembly
- Graph
- Hex
- Feature Map
- Source
- Runtime Browser

The default function workspace should support split views:

- Assembly + Pseudocode
- Graph + Pseudocode
- Pseudocode + Inspector
- Pseudocode + Source
- Pseudocode + Runtime Browser

Users should not need to choose between "code" and "meaning". The best layout
for Apple reverse engineering is often pseudocode on the left, runtime evidence
on the right, and assembly/graph one shortcut away.

### Pseudocode Gutter

The gutter should show:

- line ordinal
- primary address or address range
- confidence marker
- breakpoint marker when debugger integration exists
- bookmark marker
- comment marker
- issue marker
- stale marker

The address column must be hideable, because Hopper's current changelog
explicitly treats address-column visibility in ASM and pseudocode modes as a
customizable UI detail. Damsel should expose the same class of polish.

### Inline Banners

Use inline banners for:

- not generated yet
- generating
- stale due to changed type or hint
- partial result
- timeout
- memory budget exceeded
- function too large for current settings
- unsupported architecture semantic
- backend invariant failure

Each banner needs a primary action:

- Generate
- Re-run
- Show last result
- Increase budget
- Open issue details
- Show assembly only

### Decompiler-Lite Summary

Before full pseudocode, Damsel should ship a decompiler-lite summary panel.

It should show:

- inferred signature
- calls
- message sends
- referenced globals
- stack frame size
- local variables
- arguments
- returns
- switch candidates
- loop candidates
- no-return calls
- panic/log/assert patterns
- linked runtime objects
- high-confidence natural-language summary

This is valuable even before C-like rendering. It also establishes data
contracts for later pseudocode.

### Line Inspector

Selecting a pseudocode line opens an inspector with sections:

- Rendered line
- Source instructions
- Control-flow region
- Variables
- Type facts
- Runtime evidence
- Debug/source info
- Issues
- Commands
- Provenance

For an ObjC message line, the inspector should show:

- receiver expression
- selector
- class candidates
- method candidates
- type encoding
- IMP address
- message-send instruction span
- selector reference evidence
- confidence
- alternate candidates

For a C++ virtual call line, the inspector should show:

- object expression
- vtable pointer source
- slot index
- candidate class
- candidate method
- RTTI/vtable evidence
- constructor/destructor hints
- confidence

For XNU/kernel pseudocode, the inspector should show:

- KDK profile
- dSYM match status
- kernel collection member
- syscall/MIG/IOKit interface hints
- panic/assert/log call classification
- exported symbol relation
- PAC/chained-fixup evidence

### Hover Cards

Hover cards should be brief:

- variable storage and type
- call target and signature
- selector/class evidence
- address and instruction count
- confidence summary
- issue count
- command hint

Do not put entire provenance dumps in hover cards. Use the inspector for that.

### Folding

Support folding for:

- function signature metadata
- large switch bodies
- loop bodies
- unresolved regions
- dead-code regions
- inline assembly blocks
- compiler boilerplate
- autorelease/retain/release noise, when runtime analysis marks it safe

Fold state should be local UI state, but exported evidence should record if a
region was folded during export.

## Editing And User Corrections

### Editable Actions

The pseudocode Workbench should support:

- rename variable
- rename function
- rename local label
- set variable type
- set function signature
- set parameter name
- set return type
- mark call target
- mark no-return call
- set switch/case hint
- force dead-code visibility
- edit function bounds
- add comment
- add tag
- choose receiver class candidate
- choose C++ vtable candidate
- attach KDK/dSYM profile
- suppress a known false warning

Every action must become a typed transaction.

### Transaction Requirements

Transactions must include:

- command id
- route before action
- route after action
- selected object ids
- old value
- new value
- analysis dependencies invalidated
- timestamp
- user-visible summary
- undo handler
- redo handler

Never store corrections only inside rendered pseudocode text. Corrections should
feed analysis and regenerate the representation.

### Rename Variable Flow

Expected flow:

1. User selects a variable token.
2. Workbench validates rename command.
3. Rename sheet shows current storage and all affected lines.
4. User enters a name.
5. Store creates `RenameDecompilerVariable` transaction.
6. Backend records a user alias fact.
7. Function decompiler cache invalidates only affected representation layers.
8. Pseudocode refreshes in place.
9. Assembly, graph, inspector, search, and exports use the new alias.
10. Undo restores the old alias and refreshes dependent views.

Acceptance gate: rename must not require a full project reanalysis.

### Set Type Flow

Expected flow:

1. User selects variable or function signature token.
2. Type editor opens with candidate sources:
   - current inferred type
   - dSYM/KDK/DWARF type
   - ObjC type encoding
   - C++ demangled signature
   - imported function signature
   - user-defined type
3. User chooses or writes type.
4. Parser validates type.
5. Backend records user type fact.
6. Decompiler re-runs type propagation for the function.
7. Affected pseudocode lines update with provenance markers.

If type parsing fails, the command should fail before mutation.

## Command Palette And Menus

Decompiler commands should be first-class commands:

- Open Pseudocode
- Open Pseudocode Beside Assembly
- Generate Pseudocode For Function
- Re-run Decompiler For Function
- Cancel Decompiler Job
- Show Decompiled Line In Assembly
- Show Decompiled Line In Graph
- Show Decompiled Line In Hex
- Show Decompiled Line In Source
- Show IR For Line
- Show Source Map For Line
- Rename Variable
- Set Variable Type
- Set Function Signature
- Mark Call As No-Return
- Set Switch/Case Hint
- Toggle Dead Code Visibility
- Toggle Address Column
- Toggle Confidence Markers
- Toggle Runtime Overlays
- Export Pseudocode
- Export Pseudocode With Evidence
- Copy Decompiled Line
- Copy Decompiled Function
- Copy Source-Mapped Evidence

Command validation must depend on route, selection, permissions, job state, and
cache state.

## Apple Runtime Overlays

### Objective-C Message Sends

ObjC calls should render as message-like pseudocode when evidence supports it:

```text
[ClassName selector:arg]
[receiver selector:arg other:arg2]
```

But the representation should keep source truth:

- original call target was likely `_objc_msgSend`
- selector came from selector reference
- receiver class may be inferred, not proven
- IMP candidate may be from class metadata or cache projection
- type encoding may be partial

The inspector should make these distinctions visible.

### Objective-C Ivars

Ivar display should use:

- class metadata
- ivar offsets
- method ownership
- receiver storage
- type encodings
- KDK/dSYM type facts when present

If receiver class is uncertain, render an annotated field access:

```text
receiver->ivar_candidate /* weak class evidence */
```

or keep a lower-level expression with an inline issue marker. Do not invent a
field name without evidence.

### Objective-C Blocks

Block-related patterns should be recognized:

- block literal setup
- invoke pointer
- descriptor
- captured variables
- copy/dispose helpers

Early pseudocode may show a structured `BlockLiteral` helper rather than trying
to synthesize exact source closure syntax.

### C++ And ObjC++

C++ overlays should target useful partial recovery:

- demangled names
- namespaces
- methods
- constructors/destructors
- vtable slots
- RTTI names when present
- virtual call candidates
- object lifetime hints
- operator names

ObjC++ functions can mix ObjC messages and C++ calls. The renderer should allow
hybrid lines instead of forcing one language flavor.

### Swift-Adjacent Evidence

Do not claim Swift source reconstruction in the first wave.

Do support:

- Swift demangled names
- Swift string evidence
- metadata references when the runtime model supports them
- symbol and type names in call summaries
- explicit "Swift-adjacent" flavor when pseudocode is C-like but evidence is
  Swift runtime-heavy

### Kernel And XNU Patterns

Kernel pseudocode should add overlays for:

- panic and assert calls
- os_log and kernel logging
- sysctl tables
- syscall tables
- MIG dispatch
- IOKit user-client external methods
- kmod start/stop
- vfs/sysent-like tables
- locking primitives
- copyin/copyout
- entitlement and code-signing checks

These should be evidence-backed labels, not guessed source constructs.

### Arm64e And PAC

For arm64e:

- show authenticated indirect call/branch annotations
- keep PAC strip/auth operations visible when semantically important
- avoid collapsing authenticated selector trampolines until runtime evidence is
  strong
- show chained-fixup pointer provenance for call targets
- expose PAC unresolved issues in the inspector

Pseudocode that hides PAC too early can mislead security analysis.

## Backend API Contracts

### Generate Pseudocode Job

Endpoint:

```text
POST /api/projects/{project_id}/functions/{function_id}/decompiler/jobs
```

Request:

```json
{
  "mode": "pseudocode",
  "priority": "interactive",
  "force": false,
  "include_runtime_overlays": true,
  "include_debug_info": true,
  "visible_dead_code": "project_default",
  "budget": {
    "max_ms": 750,
    "max_memory_mb": 128,
    "allow_partial": true
  }
}
```

Response:

```json
{
  "job_id": "decomp-job-123",
  "function_id": "fn-456",
  "status": "queued",
  "cache_hit": false
}
```

### Job Status

Endpoint:

```text
GET /api/projects/{project_id}/decompiler/jobs/{job_id}
```

Response:

```json
{
  "job_id": "decomp-job-123",
  "status": "running",
  "phase": "control_structure_recovery",
  "progress": {
    "completed_units": 4,
    "total_units": 9
  },
  "partial_view_available": true,
  "metrics": {
    "elapsed_ms": 218,
    "peak_memory_mb": 42
  }
}
```

### Fetch Pseudocode View

Endpoint:

```text
GET /api/projects/{project_id}/functions/{function_id}/pseudocode
```

Query:

```text
?overlays=objc,cpp,kernel,debug
&dead_code=visible
&confidence=compact
&model_version=current
```

Response root:

```json
{
  "view": {},
  "cache_state": {},
  "problems": [],
  "links": {}
}
```

### Fetch Source Map

Endpoint:

```text
GET /api/projects/{project_id}/functions/{function_id}/pseudocode/source-map
```

The response should support both full-function maps and line-scoped maps:

```text
?line_id=pseudo-line-12
```

### Fetch IR For Debugging

Endpoint:

```text
GET /api/projects/{project_id}/functions/{function_id}/ir
```

The IR endpoint is for developer/debug views and advanced users. It should not
be required for normal pseudocode rendering.

### Apply User Hint

Endpoint:

```text
POST /api/projects/{project_id}/decompiler/hints
```

Request:

```json
{
  "function_id": "fn-456",
  "hint_kind": "mark_no_return",
  "target": {
    "call_site_address": "0x100003f40"
  },
  "value": {
    "target_function_id": "fn-abort"
  },
  "reason": "User confirmed abort-like call from KDK symbol evidence."
}
```

Response:

```json
{
  "hint_id": "hint-789",
  "transaction_id": "txn-111",
  "invalidated": [
    "decompiler_cache:function:fn-456",
    "graph_layout:function:fn-456"
  ]
}
```

### Export Pseudocode

Endpoint:

```text
POST /api/projects/{project_id}/functions/{function_id}/pseudocode/export
```

Export modes:

- plain text
- markdown
- JSON evidence package
- report fragment
- source-map bundle

Every export should include warnings that pseudocode is reconstructed analysis,
not original source, unless the user explicitly disables that warning for a
private workflow.

## Persistence And Caching

### Cache Entries

Decompiler cache entries should include:

- project id
- source id
- function id
- function revision
- analysis run id
- backend pass versions
- runtime overlay versions
- debug-info profile id
- user hint revision
- renderer settings
- generated representation
- metrics
- problems

### Invalidation Inputs

Invalidate decompiler results when any of these change:

- source bytes or source revision
- function bounds
- basic blocks
- CFG edges
- jump-table facts
- no-return facts
- type database entries used by function
- signature facts used by function
- ObjC metadata used by function
- C++ vtable/RTTI facts used by function
- KDK/dSYM profile used by function
- user hints used by function
- renderer model version

### Partial Invalidation

Prefer partial invalidation:

- variable rename invalidates rendered tokens and search index, not IR
- type change invalidates type propagation and pseudocode, not instruction
  decode
- switch hint invalidates CFG normalization and downstream passes
- function-bound edit invalidates procedure recovery and downstream passes
- runtime overlay toggle invalidates representation only

The cache model should make these distinctions explicit.

## Scheduler And Performance

### Interactive Budgets

Initial budgets:

- open cached pseudocode: under 100 ms
- show stale cached pseudocode while revalidating: under 150 ms
- generate simple function: under 750 ms
- generate medium function: under 2 s with progress
- large function: partial result under 2 s, full result in background
- cancellation response: under 100 ms
- line selection synchronization: under 50 ms

Budgets are targets, not claims. The important design requirement is that the
UI never blocks the main thread while decompilation runs.

### Job Priority

Priorities:

- `visible_interactive`
- `nearby_prefetch`
- `user_batch`
- `background_index`
- `export_required`

Rules:

- visible function jobs outrank prefetch jobs
- user cancellation interrupts queued and cooperative running jobs
- export jobs may run longer but must remain cancellable
- background jobs must back off during interaction
- large dyld/cache/kernel jobs need memory caps

### Large Function Handling

Large functions need special behavior:

- show summary first
- show partial pseudocode by control region
- isolate expensive regions
- render unresolved regions explicitly
- offer "decompile selected region"
- allow budget increase per function
- record timeout and memory issues

Do not hang the whole Workbench because one optimized function is pathological.

### Memory Discipline

Decompiler jobs must avoid retaining:

- full source bytes when mmap slices suffice
- duplicate instruction text for every representation
- large IR debug dumps by default
- whole-cache runtime maps inside a per-function job
- rendered strings for hidden functions

Cache compressed representation if needed, but keep active UI models cheap.

## Native Workbench Store Changes

### Store Split

Add stores:

- `DecompilerWorkspaceStore`
- `DecompilerJobStore`
- `PseudocodeViewStore`
- `DecompilerSelectionStore`
- `VariableEditStore`
- `TypeEditStore`
- `DecompilerProblemStore`
- `DecompilerExportStore`

These should not be crammed into one monolithic `WorkbenchStore`.

### Store Responsibilities

`DecompilerWorkspaceStore`:

- owns open pseudocode tabs
- tracks active function route
- coordinates layout
- remembers representation preferences

`DecompilerJobStore`:

- starts jobs
- observes job events
- handles cancellation
- surfaces progress and partial results

`PseudocodeViewStore`:

- holds rendered line models
- manages folding and scroll state
- exposes selection commands
- tracks stale/cache state

`DecompilerSelectionStore`:

- maps pseudocode selections to canonical routes
- coordinates assembly/graph/hex/source/runtime highlight state

`VariableEditStore`:

- manages rename sheets
- validates variable names
- previews affected lines
- submits transactions

`TypeEditStore`:

- manages type editor state
- calls type parser/validator
- previews propagation impact

`DecompilerProblemStore`:

- indexes issues
- maps issues to lines and commands
- feeds global Problems UI

`DecompilerExportStore`:

- builds export options
- requests export bundles
- tracks privacy redaction choices

## Rendering Design

### Text Layout

The pseudocode view should be a virtualized code surface:

- monospaced text
- stable line heights
- address gutter
- token-level hit testing
- keyboard selection
- mouse selection
- copy as text
- copy with addresses
- copy with source-map evidence
- folded regions
- inline issue markers

Avoid making each line a heavy general-purpose SwiftUI hierarchy if performance
degrades on large functions. A custom AppKit text surface or hybrid renderer may
be required.

### Syntax And Runtime Styling

Styles should distinguish:

- keywords
- types
- variables
- parameters
- functions
- ObjC selectors
- ObjC classes
- C++ namespaces/classes
- kernel symbols
- literals
- addresses
- comments
- unknowns
- user assertions
- low-confidence inference

The palette must remain restrained. This is a reverse-engineering tool, not a
rainbow syntax demo.

### Accessibility

Accessibility requirements:

- each line exposes address, text, and issue count
- tokens expose semantic role
- confidence markers have text labels
- keyboard can reach commands
- inspector content is readable by VoiceOver
- color is not the only issue signal
- reduced-motion setting disables animated refresh effects

## Search Integration

Pseudocode should be indexed.

Searchable fields:

- rendered text
- function signatures
- variable names
- type names
- ObjC selectors/classes
- C++ symbols
- kernel symbols
- comments
- issue titles
- runtime overlay labels

Search results should route to pseudocode lines, not only addresses. Selecting a
search result should open the function and scroll to the line.

Index invalidation should be line-based when possible.

## Export And Reporting

### Plain Export

Plain text export includes:

- function signature
- pseudocode lines
- optional addresses
- optional comments
- warnings about reconstructed output

### Evidence Export

Evidence export includes:

- pseudocode
- source maps
- confidence summaries
- issues
- runtime overlays
- type/signature provenance
- user hints
- analysis versions
- source artifact identity
- privacy redaction metadata

This is where Damsel can beat Hopper for research workflows: a report should not
only show "what the tool thinks"; it should show why.

### Privacy Rules

Export should redact by default:

- local absolute paths
- user account names
- private source paths from dSYM/DWARF
- raw source snippets unless explicitly enabled
- panic logs with device identifiers unless explicitly enabled

## Debugger Integration

Future debugger integration should map runtime state onto pseudocode:

- current PC highlights pseudocode line
- breakpoints can be set from pseudocode lines
- stepping updates pseudocode selection
- register values annotate variables
- stack frame values annotate locals
- memory reads annotate pointers
- unloaded source map regions fall back to assembly

Do not build this before static source maps are solid. Debugger overlays depend
on the same identity model.

## Automation And MCP Integration

Decompiler automation should be capability-gated.

Read-only capabilities:

- list decompiled functions
- fetch pseudocode for function
- fetch pseudocode line source map
- explain decompiler issue
- export evidence package

Mutation capabilities:

- rename variable
- set type
- add hint
- mark no-return
- set switch hint
- add comment

Mutation should require explicit audited capability. MCP/automation tools should
never silently alter project facts in response to speculative model output.

## Implementation Phases

### Phase 1: Representation Shell

Deliver:

- `DecompilerFunctionView` contract
- pseudocode route type
- placeholder "not generated" Workbench tab
- job-state banners
- command palette entries
- source-map model stubs
- snapshots for empty/placeholder states

Acceptance:

- selecting Open Pseudocode on a function opens a stable route
- no backend decompiler required
- Workbench can show not-generated, generating, stale, failed, and cached states

### Phase 2: Decompiler-Lite Summary

Deliver:

- summary view from existing `DisassemblyAnalysis`
- calls/message sends/globals/stack/jump-table summary
- confidence records
- line-to-instruction maps for summary rows
- inspector integration

Acceptance:

- ObjC fixture shows message-send summary with selector evidence
- jump-table fixture shows switch candidate issue
- arm64e fixture shows authenticated indirect-control issue

### Phase 3: Job And Cache Pipeline

Deliver:

- decompiler job endpoints
- job events
- cancellation
- cache entries
- invalidation records
- Workbench progress UI
- metrics

Acceptance:

- cancelling a large function job updates UI within budget
- cached simple function opens quickly
- changing a user alias invalidates only representation/cache layers

### Phase 4: Source Maps Across Views

Deliver:

- pseudocode line to assembly map
- pseudocode line to graph map
- pseudocode line to hex map
- selection synchronization
- ambiguous/no-map UI states

Acceptance:

- selecting pseudocode highlights assembly and graph
- selecting assembly highlights pseudocode
- omitted/dead code is visible as an issue, not invisible loss

### Phase 5: Variable And Type Editing

Deliver:

- variable model
- rename transaction
- type edit transaction
- type parser validation
- affected-line preview
- undo/redo support
- search index invalidation

Acceptance:

- variable rename updates pseudocode, assembly annotations, inspector, and search
- undo restores old name and source maps remain stable

### Phase 6: Control-Structure Pseudocode

Deliver:

- if/else rendering
- loop rendering
- switch rendering
- goto fallback
- unresolved region rendering
- dead-code visibility toggle
- switch hint command

Acceptance:

- switch fixture can improve after a user hint
- unresolved indirect branch remains visibly unresolved
- no result is silently prettier than evidence supports

### Phase 7: Objective-C Runtime Overlay

Deliver:

- ObjC message rendering
- selector/class/method inspector
- ivar access candidates
- type encoding display
- receiver class ambiguity UI

Acceptance:

- ObjC sample function links pseudocode lines to selector, method, and IMP
  evidence
- stripped ObjC declarations produce explicit partial-confidence output

### Phase 8: C++ And ObjC++ Overlay

Deliver:

- demangled C++ signatures
- vtable candidate display
- virtual call inspector
- constructor/destructor hints
- mixed ObjC++ rendering

Acceptance:

- vtable fixture displays candidate slot and confidence
- ambiguous virtual calls remain inspectable

### Phase 9: XNU And Kernel Overlay

Deliver:

- KDK/dSYM integration in pseudocode
- panic/assert/log classification
- IOKit external method table rendering
- syscall/MIG table hints
- kernel collection context

Acceptance:

- kernel fixture routes pseudocode from panic frame to KDK-backed function
- IOKit table fixture displays external method candidates with source evidence

### Phase 10: Export, Automation, And Evidence

Deliver:

- plain export
- markdown export
- JSON evidence export
- automation read endpoints
- audited mutation endpoints
- privacy redaction

Acceptance:

- exported evidence package can reconstruct line-to-instruction provenance
- private paths are redacted by default

## Validation Matrix

### Fixtures

Required fixtures:

- simple function with straight-line calls
- if/else branch
- loop
- switch/jump table
- unresolved indirect branch
- no-return call
- dead-code/unreachable region
- stack locals with overlapping lifetimes
- renamed variable
- user-set type
- ObjC message send
- ObjC ivar access
- stripped ObjC declarations
- C++ demangled function
- C++ virtual call
- ObjC++ mixed method
- arm64e authenticated indirect branch
- chained-fixup call target
- Swift-demangled symbol reference
- kernel panic/assert/log call
- IOKit external method table
- KDK/dSYM source-line match
- huge optimized function
- malformed function bounds

### Snapshot Types

Snapshot:

- pseudocode view JSON
- source maps
- confidence records
- issues
- export evidence
- Workbench route state
- command validation state

### UI Tests

UI tests:

- open pseudocode from function navigator
- split pseudocode with assembly
- select pseudocode line and verify assembly highlight
- select assembly row and verify pseudocode highlight
- rename variable and undo
- set type and undo
- toggle dead-code visibility
- cancel job
- show partial result after timeout
- export with redaction

### Performance Tests

Performance tests:

- cached open latency
- simple generation latency
- medium function generation latency
- large function partial result latency
- cancellation latency
- selection synchronization latency
- memory peak per function
- cache entry size
- virtualized scroll stability

## Failure Modes

### Pretty But Wrong

Risk: renderer emits plausible C-like output that hides unresolved evidence.

Mitigation:

- confidence markers
- unresolved regions
- source maps
- issue records
- snapshot tests that expect uncertainty

### UI State Detached From Analysis

Risk: user edits text-like pseudocode but backend facts do not change.

Mitigation:

- no in-place pseudocode text mutation
- typed transactions
- analysis invalidation
- undo/redo records

### Whole-App Stalls

Risk: large functions or dyld/cache analysis block the Workbench.

Mitigation:

- job scheduler
- cancellation
- partial results
- memory caps
- cached stale display
- virtualized rendering

### Apple Runtime Overreach

Risk: renderer invents ObjC/C++/kernel semantics from weak evidence.

Mitigation:

- runtime overlay confidence
- alternate candidates
- provenance inspector
- lower-level fallback syntax

### Cache Incoherence

Risk: renames, type edits, KDK profile changes, or function-bound edits leave
stale pseudocode.

Mitigation:

- explicit dependency graph
- representation versioning
- stale banners
- tests for invalidation classes

### Export Misrepresentation

Risk: exported pseudocode reads like original source.

Mitigation:

- export warning
- confidence summary
- evidence package option
- provenance footer

## Required Refactors

### Rust Core

Add or refactor:

- durable function ids
- representation ids
- decompiler model module
- decompiler job module
- source-map module
- confidence/provenance types
- decompiler issue records
- user hint records
- cache dependency records
- export serializers

### Mach-O Analysis

Extend:

- decoded instruction ids
- instruction-to-byte mappings
- basic block identity stability
- jump-table facts
- PAC/chained-fixup target evidence
- ObjC reference provenance
- cache link provenance

### API Server

Add:

- decompiler job routes
- pseudocode fetch route
- source-map fetch route
- hint mutation route
- export route
- event stream updates
- metrics route additions

### Swift App Core

Add:

- decompiler response models
- pseudocode line/token models
- job event models
- confidence/issue models
- store split
- command definitions
- transaction models
- route models

### Native Workbench

Add:

- pseudocode representation tab
- split-view presets
- gutter renderer
- token renderer
- line inspector
- variable rename sheet
- type editor sheet
- job banners
- export sheet
- source-map highlighting
- runtime overlay highlights

## Data Model Sketch

```rust
pub struct DecompilerFunctionView {
    pub id: DecompilerViewId,
    pub project_id: ProjectId,
    pub source_id: SourceId,
    pub image_id: Option<ImageId>,
    pub function_id: FunctionId,
    pub entry_address: u64,
    pub display_name: String,
    pub signature: Option<FunctionSignatureView>,
    pub language_flavor: DecompilerLanguageFlavor,
    pub lines: Vec<PseudocodeLine>,
    pub variables: Vec<DecompilerVariableBinding>,
    pub control_regions: Vec<DecompilerControlRegion>,
    pub runtime_overlays: Vec<DecompilerRuntimeOverlay>,
    pub issues: Vec<DecompilerIssue>,
    pub source_maps: Vec<DecompilerSourceMap>,
    pub cache_state: DecompilerCacheState,
    pub model_versions: DecompilerModelVersions,
}
```

```swift
public struct PseudocodeLine: Identifiable, Equatable, Codable {
    public let id: String
    public let ordinal: Int
    public let indent: Int
    public let kind: PseudocodeLineKind
    public let tokens: [PseudocodeToken]
    public let addressRange: AddressRange?
    public let instructionIds: [String]
    public let basicBlockIds: [String]
    public let issueIds: [String]
    public let confidence: DecompilerConfidence
    public let displayFlags: PseudocodeDisplayFlags
}
```

These sketches are intentionally stable and boring. The internal IR can evolve;
the Workbench contract should not churn every time an analysis pass changes.

## Acceptance Criteria

Damsel should not call this feature Hopper-competitive until:

- pseudocode is a route-backed native Workbench representation
- line selection synchronizes with assembly, graph, hex, and inspector
- every line has source-map and confidence data
- variables can be renamed through transactions and undo/redo
- types/signatures can be edited with validation and provenance
- ObjC message sends show selector/class/method evidence
- C++ virtual calls show vtable evidence when available
- XNU/kernel overlays show KDK/dSYM and panic/log evidence when available
- large functions produce partial results without freezing the UI
- decompiler jobs are cancellable
- stale cached results are labeled
- exports include source-map evidence and privacy redaction
- tests cover unresolved and wrong-evidence cases, not only happy paths

## Implementation Order

Recommended order:

1. Add representation contracts and placeholder UI.
2. Add decompiler-lite summary from existing analysis outputs.
3. Add jobs, cache, events, and cancellation.
4. Add source-map synchronization with assembly/graph/hex.
5. Add variable rename and type-edit transactions.
6. Add early control-structure rendering.
7. Add ObjC overlay.
8. Add C++/ObjC++ overlay.
9. Add XNU/kernel overlay.
10. Add export/evidence automation.

This order gives users value early and avoids waiting for full pseudocode before
building the Workbench foundations that full pseudocode needs.

## Summary

The competitive decompiler feature is not "Damsel prints C-like text." It is a
native, synchronized, evidence-backed pseudocode workspace with source maps,
confidence, Apple runtime overlays, user correction loops, responsive jobs,
exports, and truthful failure states.

That is a larger system than a renderer. It touches the project kernel,
function database, IR pipeline, type system, runtime object model, API server,
native stores, command registry, transaction system, search index, scheduler,
export path, and validation corpus. Building it this way is slower than a
string renderer, but it creates the product surface that can actually compete
with Hopper while preserving Damsel's Apple-system niche.
