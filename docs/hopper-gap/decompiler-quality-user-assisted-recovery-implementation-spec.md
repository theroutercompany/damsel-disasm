# Decompiler Quality And User-Assisted Recovery Implementation Spec

Status: implementation-ready planning document.

Purpose: define the quality, evidence, user-correction, regression, and
Apple-runtime recovery architecture required before Damsel can claim
Hopper-competitive pseudocode. This document complements the IR and native
pseudocode Workbench specs by focusing on the loop that makes decompiler
output trustworthy: how the app explains, measures, corrects, retries, caches,
and validates high-level recovery.

## Why This Exists

Decompiler output is persuasive even when it is wrong. A Hopper-competitive
Damsel cannot simply render C-like text and hope analysts trust it. The product
must make uncertainty inspectable, correction cheap, and regression evidence
durable.

The existing Damsel docs already plan:

- function and procedure records
- processor semantics
- low-level IR
- SSA and def-use
- type and debug-info records
- pseudocode routes and source maps
- native Workbench representation
- Apple runtime browsers

This spec adds the missing operating layer:

- decompiler pass ledger
- pseudocode quality report
- source-to-pseudocode evidence graph
- user hint and correction model
- dead-code and unreachable-code policy
- switch/case recovery workflow
- no-return, tailcall, thunk, frame, and calling-convention correction workflow
- ObjC/C++/Swift/XNU overlay quality gates
- decompiler regression corpus
- performance and cancellation proof
- assistant and automation access to decompiler evidence

## Public Hopper Evidence

Official Hopper tutorial evidence:

- Hopper presents pseudocode as the easiest way to read code, but explicitly
  warns there is no magic.
- Hopper says pseudocode may be imperfect and that some parts can disappear
  when Hopper decides code is unreachable.
- Hopper exposes a checkbox to mitigate dead-code disappearance.
- Hopper says most display modes require procedure construction because
  procedures contain basic-block and stack-usage information.
- Hopper exposes switch/case hints from the procedure inspector for indirect
  jumps.
- Hopper supports manual references, comments, labels, tags, operand formats,
  and byte-type changes with undo/redo.
- Hopper's CFG is editable: comments, labels, grouping, colors, and node text
  are user-facing analysis operations.

Official Hopper FAQ evidence:

- Hopper says decompilers cannot fully recover lost source code.
- Hopper says pseudocode can be close to C or Objective-C but is usually not
  directly compilable.
- Hopper frames the decompiler goal as understanding code the analyst did not
  write, not source recovery.

Official Hopper changelog evidence:

- Version 6.0.0 introduced a new decompiler with vastly improved output.
- Recent 6.x updates mention decompiler hangs, memory footprint, large
  function decompilation, and large-binary performance.
- Hopper's 5.19 decompiler refactor mentions stack variables, variable rename,
  variable liveness, and AST optimizations.
- Hopper changelog entries mention switch/case detection, switch hint display,
  switch table entry size, switch multipliers, no-return propagation,
  conditional branches to no-return functions, dead code removal, C++ mangled
  method signatures, Objective-C ivar display, Objective-C stubs, Swift
  demangling, Swift strings, AArch64 conditional instructions, authenticated
  Objective-C selector trampolines, PAC procedure creation, Apple Silicon
  decompilation, dyld shared-cache Objective-C decompilation, and kernel cache
  support.
- Hopper has repeatedly fixed decompiler crashes, hangs, incoherent goto
  statements, large-method pseudocode export, and memory leaks.

Public SDK evidence:

- `CPUContext` exposes `canDecompileProcedure`, `skipHeader`, `skipFooter`, and
  `decompileInstructionAtAddress`.
- `CPUContext` exposes branch analysis, switch detection, thunk detection,
  prolog/epilog analysis, direct-memory-reference filtering, and
  instruction-specific analysis hooks.
- The SDK says `ASTNode` is not publicly exposed, which means Hopper's
  decompiler internals are a protected product core even while CPU plugins can
  participate.
- `HPDisassembledFile` exposes procedures, byte types, operand formats, names,
  comments, tags, problem records, type database, undo/redo transactions,
  assembler, NOP generation, and CPU-mode facts.

Public workflow evidence:

- Public Hopper scripts include exporting the current decompiled procedure to
  an external editor and demangling Swift labels.
- That pressure matters: analysts want pseudocode outside the main UI for
  reading, search, diffing, note-taking, and collaboration.

## Damsel Current-State Evidence

Current repo anchors:

- `damsel-core::DecodedInstruction` holds row-level decoded instruction facts.
- `damsel-core::DisassemblyAnalysis` holds basic blocks, edges, direct calls,
  branch targets, indirect controls, imports, ObjC references, cache links,
  recovered values, jump tables, and summary records.
- `damsel-macho::disasm` builds current CFG and target-use summaries from
  decoded instruction rows and image metadata.
- Damsel already has docs for procedure recovery, processor semantics, type
  and debug info, Apple runtime object models, native pseudocode Workbench
  representation, automation, assistant, and patch/debugger workflows.

Current missing implementation reality:

- no persistent procedure database
- no decompiler pass ledger
- no IR implementation
- no SSA implementation
- no pseudocode renderer
- no pseudocode source map
- no user hint transaction model
- no decompiler issue model
- no decompiler regression corpus
- no Workbench pseudocode view
- no native quality report panel
- no decompiler job scheduler or cache invalidation

## Product Thesis

Damsel's decompiler should optimize for analyst trust before aesthetic
pseudocode. The first competitive version should be able to answer:

- Why did this line appear?
- Which instructions support it?
- Which IR statements support it?
- Which facts are guessed?
- Which facts were user-provided?
- Which blocks disappeared as dead or unreachable?
- Which hints could improve the result?
- Which Apple runtime facts changed the output?
- Which pass made the wrong decision?
- Which fixture protects this behavior from regression?

Hopper's current public surface makes pseudocode a polished representation in a
larger mutable document. Damsel's niche should be stronger evidence and
Apple-system specificity:

- ObjC message sends should show receiver/selector evidence.
- C++ and ObjC++ output should show `this`, vtable, RTTI, constructor, and
  destructor confidence.
- Swift-adjacent output should use demangled names and known ABI roles without
  pretending to recover Swift source.
- XNU output should show kernel profile assumptions, IOKit dispatch hints,
  syscall/trap hints, KDK/dSYM provenance, and panic-route context.
- arm64e/PAC output should preserve authentication state rather than flattening
  targets into raw addresses.

## Non-Goals

This spec does not require:

- a full C compiler-compatible output target
- source-level Swift reconstruction
- complete C++ template, exception, and RAII recovery
- binary patching through pseudocode edits
- making an LLM the decompiler
- launching cache-wide decompiler jobs by default
- exact text parity with Hopper
- replacing the existing IR spec

This spec does require:

- quality evidence for every pseudocode claim
- user correction as project transactions
- source maps from pseudocode to IR, semantics, instructions, and runtime facts
- explicit dead-code and unreachable-code policy
- Apple-runtime overlay validation
- decompiler performance budgets
- regression fixtures before broad output claims

## Vocabulary

`Decompiler run`:

- one attempt to analyze a procedure and produce summaries, IR, source maps,
  quality report, and optional pseudocode.

`Decompiler pass`:

- a named stage inside a run, such as frame recovery, SSA construction,
  switch recovery, ObjC overlay, C++ overlay, or pseudocode rendering.

`Decompiler issue`:

- an inspectable warning, error, unsupported feature, contradiction, or
  low-confidence decision emitted by a pass.

`User hint`:

- an analyst-provided fact that changes analysis, such as no-return target,
  switch table bounds, variable name, variable type, function signature,
  receiver/selector binding, tailcall confirmation, or dead-code inclusion.

`Quality report`:

- a structured summary of confidence, coverage, problems, missing facts,
  runtime overlay use, pass timings, cache state, and suggested corrections.

`Source map`:

- a routeable relationship from pseudocode lines and tokens back to IR,
  semantic facts, instructions, graph blocks, types, runtime objects, and user
  hints.

`Recovery case`:

- a fixture or real-system example with expected semantic behavior, not merely
  a golden pseudocode string.

## Architecture Overview

Add these model families to the project/decompiler layer:

- `DecompilerRun`
- `DecompilerPassRun`
- `DecompilerPassInput`
- `DecompilerPassOutput`
- `DecompilerIssue`
- `DecompilerQualityReport`
- `DecompilerCoverageSummary`
- `DecompilerSourceMap`
- `PseudocodeDocument`
- `PseudocodeLine`
- `PseudocodeToken`
- `DecompilerHint`
- `DecompilerHintApplication`
- `DecompilerCorrectionTransaction`
- `DecompilerCacheKey`
- `DecompilerInvalidationReason`
- `DecompilerRegressionCase`
- `DecompilerRegressionExpectation`
- `DecompilerComparisonReport`
- `DecompilerPerformanceSample`

These records should live above the future project database and below the
native Workbench. They should be available to CLI JSON, Workbench API,
automation, MCP, assistant, diff/export, and evidence packages.

## Decompiler Run Model

### DecompilerRun

Fields:

- `run_id`
- `project_id`
- `procedure_id`
- `source_artifact_id`
- `architecture_profile_id`
- `abi_profile_id`
- `semantic_version`
- `type_profile_id`
- `debug_info_profile_id`
- `runtime_overlay_profile_id`
- `hint_set_version`
- `requested_outputs`
- `status`
- `started_at`
- `finished_at`
- `duration_ms`
- `cancellation_token`
- `cache_key`
- `input_fingerprint`
- `output_fingerprint`
- `pass_runs`
- `quality_report_id`
- `pseudocode_document_id`
- `problems`

Design notes:

- A run is not just output text.
- A run is a reproducible event with input fingerprints.
- The Workbench can show stale pseudocode when a dependency changes.
- The assistant can cite a run id when answering "why does this function look
  like a dispatch wrapper?"

### DecompilerPassRun

Fields:

- `pass_id`
- `run_id`
- `name`
- `phase`
- `inputs`
- `outputs`
- `status`
- `started_at`
- `finished_at`
- `duration_ms`
- `input_count`
- `output_count`
- `issue_count`
- `cache_hit`
- `canceled`
- `problems`

Initial passes:

- procedure boundary validation
- basic-block normalization
- semantic fact import
- frame recovery
- argument and return analysis
- stack local recovery
- scalar def-use
- SSA construction
- memory-reference classification
- no-return propagation
- thunk and tailcall detection
- switch and jump-table recovery
- dead-code classification
- control-structure structuring
- type and signature application
- ObjC message overlay
- C++/ObjC++ overlay
- Swift-adjacent overlay
- dyld and stub overlay
- XNU/kernel overlay
- arm64e/PAC overlay
- pseudocode rendering
- source-map generation
- quality report generation

Why:

- Hopper's changelog shows that decompiler quality changes come from many
  specific subsystems. Damsel needs the same granularity to avoid treating
  decompiler bugs as mysterious bad text.

## Quality Report Model

### DecompilerQualityReport

Fields:

- `report_id`
- `run_id`
- `overall_confidence`
- `coverage`
- `control_quality`
- `dataflow_quality`
- `type_quality`
- `runtime_overlay_quality`
- `source_map_quality`
- `performance_quality`
- `issue_summary`
- `suggested_hints`
- `blocking_issues`
- `non_blocking_issues`
- `unsupported_features`
- `suppressed_issues`
- `exportable`
- `redaction_notes`

### Coverage Summary

Fields:

- `procedure_byte_count`
- `decoded_byte_count`
- `semantic_instruction_count`
- `ir_statement_count`
- `mapped_instruction_count`
- `mapped_pseudocode_line_count`
- `unmapped_instruction_count`
- `unreachable_block_count`
- `included_dead_block_count`
- `excluded_dead_block_count`
- `unknown_control_count`
- `unknown_callsite_count`
- `unknown_stack_slot_count`

Policy:

- Pseudocode that omits blocks must say so.
- Dead-code exclusion is a visible policy, not a hidden optimization.
- The Workbench should let users toggle inclusion where the underlying
  procedure facts can support it.

### Confidence Summary

Confidence categories:

- `direct`: direct branch, direct import, exact symbol, exact debug-info match
- `strong`: stable pattern plus corroborating metadata
- `medium`: common compiler pattern but incomplete context
- `weak`: heuristic only
- `contradicted`: evidence conflicts
- `unsupported`: required model does not exist

Every decompiler line and important token should inherit confidence from
supporting facts.

## Decompiler Issue Model

### DecompilerIssue

Fields:

- `issue_id`
- `run_id`
- `pass_id`
- `severity`: note, warning, error, unsupported
- `category`
- `message`
- `source_routes`
- `affected_pseudocode_routes`
- `affected_ir_routes`
- `affected_instruction_routes`
- `affected_runtime_routes`
- `confidence`
- `suggested_hints`
- `suppressible`
- `suppression_reason`
- `created_by`

Categories:

- procedure-boundary
- decode
- processor-semantics
- CPU-mode
- frame
- stack
- argument
- return
- no-return
- tailcall
- thunk
- indirect-control
- switch
- dead-code
- control-structuring
- SSA
- alias
- type
- debug-info
- ObjC
- C++
- Swift
- dyld
- XNU
- PAC
- rendering
- source-map
- performance

Examples:

- `dead-block-excluded-from-pseudocode`
- `switch-table-bounds-unknown`
- `jump-table-entry-size-unknown`
- `no-return-target-conflict`
- `tailcall-not-proven`
- `stack-variable-overlap`
- `frame-pointer-omitted`
- `objc-selector-unresolved`
- `objc-receiver-ambiguous`
- `cpp-vtable-target-weak`
- `swift-symbol-demangled-but-type-unproven`
- `pac-target-not-resolved`
- `kernel-profile-user-mode-assumption-blocked`
- `pseudocode-line-has-no-source-map`

## User Hint And Correction Model

### DecompilerHint

Fields:

- `hint_id`
- `project_id`
- `procedure_id`
- `kind`
- `target_route`
- `payload`
- `created_by`
- `created_at`
- `reason`
- `confidence`
- `source`
- `transaction_id`
- `invalidates`
- `status`

Hint kinds:

- `procedure-boundary`
- `function-entry`
- `basic-block-boundary`
- `no-return-target`
- `tailcall`
- `thunk-target`
- `switch-table`
- `switch-entry-size`
- `switch-multiplier`
- `dead-code-include`
- `dead-code-exclude`
- `calling-convention`
- `function-signature`
- `argument-name`
- `argument-type`
- `stack-variable-name`
- `stack-variable-type`
- `register-alias`
- `local-label`
- `objc-receiver`
- `objc-selector`
- `objc-method-binding`
- `cpp-this-binding`
- `cpp-vtable-binding`
- `swift-symbol-role`
- `kernel-trap-kind`
- `iokit-external-method`
- `PAC-target`

### Hint Application Policy

Rules:

- Hints are transactions.
- Hints are never silent text edits.
- Hints have source routes and invalidation effects.
- Hints can be disabled, superseded, or contradicted.
- Hints can be exported as evidence.
- Hints can be redacted from reports when they contain private notes.

Why:

- Hopper exposes analyst correction directly in the UI: byte types, references,
  names, operand formats, comments, switch/case hints, and variable rename.
- Damsel should make those corrections more auditable than Hopper by preserving
  hint provenance and reanalysis effects.

## Dead-Code And Reachability Policy

Hopper's tutorial explicitly warns that pseudocode can lose code when Hopper
decides a region is unreachable. Damsel should turn that into a first-class
quality model.

Records:

- `ReachabilityDecision`
- `DeadCodeRegion`
- `DeadCodePolicy`
- `DeadCodeToggleState`

Fields:

- `region_id`
- `procedure_id`
- `block_ids`
- `reason`: no-entry, contradicted-edge, no-return-pruned, exception-only,
  decode-gap, user-excluded, user-included, unsupported
- `included_in_pseudocode`
- `included_in_ir`
- `source_routes`
- `confidence`
- `hints`
- `problems`

Workbench behavior:

- show an "excluded blocks" count in the pseudocode header
- provide a dead-code toggle with explanation
- highlight pseudocode lines generated from weakly reachable blocks
- keep graph selection synchronized even for excluded blocks
- preserve a list fallback when graph layout is unavailable

Acceptance:

- fixture where no-return pruning excludes a block
- fixture where user includes a dead block
- fixture where excluded block stays visible in graph/list
- pseudocode source map covers included dead-code lines

## Switch And Jump-Table Recovery

Hopper exposes switch/case hints as a user-facing repair point, and its
changelog repeatedly mentions switch/case detection. Damsel needs an explicit
switch recovery surface.

Records:

- `SwitchCandidate`
- `SwitchTable`
- `SwitchCase`
- `SwitchHint`
- `SwitchQuality`

Fields:

- `candidate_id`
- `procedure_id`
- `dispatch_instruction`
- `index_expression`
- `table_address`
- `entry_size`
- `entry_count`
- `multiplier`
- `base_address`
- `case_targets`
- `default_target`
- `bounds_check_evidence`
- `source_routes`
- `confidence`
- `hints`
- `problems`

User operations:

- set table address
- set entry size
- set entry count
- set multiplier
- set default target
- mark candidate as not switch
- export switch evidence

Acceptance:

- direct jump table fixture
- register-indirect switch fixture
- ARM/Thumb legacy mode placeholder
- arm64 `ADRP+ADD+LDR+BR` switch fixture
- malformed table negative fixture
- multiplier hint snapshot
- entry-size hint snapshot

## Variable, Stack, And Liveness Recovery

Hopper's changelog calls out stack variables, variable rename, variable
liveness, and AST optimizations. Damsel should make variable recovery a
quality subsystem.

Records:

- `RecoveredVariable`
- `VariableLifetime`
- `StackSlot`
- `VariableNameFact`
- `VariableTypeFact`
- `VariableConflict`

Fields:

- `variable_id`
- `procedure_id`
- `storage`: stack, register, argument, global, field, unknown
- `storage_expression`
- `lifetime_ranges`
- `definition_routes`
- `use_routes`
- `name`
- `type`
- `name_provenance`
- `type_provenance`
- `confidence`
- `conflicts`
- `hints`

Workbench behavior:

- variable rename in pseudocode creates a transaction
- variable type correction invalidates type-dependent pseudocode lines
- hover shows storage and lifetime evidence
- conflict panel shows overlapping stack slots and competing names

Acceptance:

- stack local fixture
- argument variable fixture
- variable rename transaction fixture
- liveness snapshot
- overlap/conflict negative fixture

## No-Return, Tailcall, And Thunk Recovery

Hopper's changelog mentions no-return detection and propagation, conditional
branches to no-return functions, and thunk recovery through CPU hooks. These
facts directly affect dead-code, CFG, and pseudocode.

Records:

- `NoReturnFact`
- `TailcallFact`
- `ThunkFact`
- `CallEffectFact`

Sources:

- known import database
- symbol names
- debug-info attributes
- procedure body shape
- CPU semantics
- user hint
- runtime overlay

Policy:

- no-return facts require source and confidence
- no-return propagation must emit affected block decisions
- tailcalls must not be hidden as ordinary returns
- thunks should preserve route to destination and original thunk body

Acceptance:

- import-backed no-return fixture
- user-provided no-return hint fixture
- contradicted no-return fixture
- tailcall fixture
- dyld stub/thunk fixture

## ObjC Overlay Quality

ObjC pseudocode is a major Apple-focused advantage. Hopper publicly highlights
Objective-C awareness, and its changelog mentions Objective-C stubs, ivar
display, dyld-cache ObjC decompilation, stripped Objective-C declarations, and
authenticated selector trampolines.

Records:

- `ObjcPseudocodeCall`
- `ObjcReceiverEvidence`
- `ObjcSelectorEvidence`
- `ObjcMethodBinding`
- `ObjcIvarAccess`
- `ObjcOverlayIssue`

Policy:

- render ObjC method-style calls only when receiver and selector evidence meet
  a configured threshold
- otherwise render a C-call style expression with ObjC evidence badges
- never hide selector uncertainty
- authenticated selector trampolines carry PAC state
- ivar display requires class/layout evidence

Acceptance:

- `_objc_msgSend` selector-resolved fixture
- receiver-ambiguous fixture
- selector-unresolved fixture
- ivar access fixture
- stripped declarations negative fixture
- authenticated selector trampoline fixture
- dyld shared-cache ObjC fixture

## C++ And ObjC++ Overlay Quality

Apple system binaries contain C++ and Objective-C++ patterns, especially in
IOKit and driver/kernel-facing code.

Records:

- `CppMethodCandidate`
- `CppThisEvidence`
- `CppVtableEvidence`
- `CppVirtualCall`
- `CppCtorDtorRegion`
- `ObjcxxMixedCallsite`

Policy:

- C++ demangled names can seed signatures but do not prove class layout alone
- vtable candidates require table evidence
- virtual calls remain candidates until target relation is proven
- constructor/destructor regions remain confidence-tagged
- Objective-C++ mixed calls should cite both ObjC and C++ evidence

Acceptance:

- C++ demangled method fixture
- vtable call fixture
- weak vtable negative fixture
- ObjC++ mixed call fixture
- IOKit dispatch candidate fixture

## Swift-Adjacent Overlay Quality

Swift support should improve Apple-binary understanding without pretending to
recover Swift source.

Records:

- `SwiftSymbolEvidence`
- `SwiftDemangleFact`
- `SwiftCallRole`
- `SwiftStringReference`
- `SwiftMetadataHint`

Policy:

- demangled names can label functions and calls
- Swift roles such as `self`, error, and async context require ABI evidence
- Swift strings and metadata are facts, not proof of source structure
- pseudocode should not emit Swift syntax in early waves

Acceptance:

- Swift demangle fixture
- Swift string reference fixture
- Swift role unresolved fixture
- demangler failure fixture

## XNU And Kernel Overlay Quality

XNU and system-binary workflows are Damsel's intended niche beyond generic
Hopper parity.

Records:

- `KernelPseudocodeContext`
- `KdkSymbolEvidence`
- `PanicFrameEvidence`
- `SyscallTrapEvidence`
- `IokitDispatchEvidence`
- `KernelProfileIssue`

Policy:

- kernel profile assumptions are visible
- KDK/dSYM evidence has precedence over weak symbol guesses
- panic-frame routes can seed decompiler focus but do not alter static facts
  without a transaction
- IOKit external-method candidates stay weak until table/class evidence
  supports them
- user-mode ABI assumptions are blocked in kernel profiles

Acceptance:

- synthetic kernel fileset fixture
- KDK exact/mismatch fixture
- panic route fixture
- IOKit external method fixture
- user-mode ABI leakage negative fixture

## arm64e/PAC Quality

PAC can invalidate naive pseudocode. Hopper's changelog names authenticated
Objective-C selector trampolines and PAC procedure creation as product issues.

Records:

- `PseudocodePacState`
- `AuthenticatedPointerUse`
- `PacStripEvidence`
- `PacCallsiteIssue`

Policy:

- keep raw, authenticated, stripped, and resolved pointer states distinct
- show when a call target is PAC-protected
- do not render a raw address target if only a signed pointer is known
- carry chained-fixup provenance into pseudocode and quality reports

Acceptance:

- authenticated return fixture
- authenticated indirect call fixture
- authenticated selector trampoline fixture
- chained-fixup pointer fixture
- false raw-target negative fixture

## Workbench Experience

### Pseudocode Header

Display:

- procedure name
- signature confidence
- decompiler status
- stale state
- overall confidence
- excluded block count
- issue count
- last run time
- cache state
- active hint count
- active profile ids

Actions:

- regenerate
- cancel
- include/exclude dead code
- open quality report
- open pass ledger
- open source map
- export pseudocode and evidence

### Quality Drawer

Sections:

- coverage
- confidence
- blocking issues
- suggested hints
- pass timings
- runtime overlay use
- unsupported features
- performance notes

### Pass Ledger

Display:

- each pass name
- duration
- cache hit/miss
- input/output counts
- issue counts
- route to affected pseudocode

Why:

- When pseudocode is wrong, analysts and maintainers need to identify whether
  the failure came from procedure recovery, CPU semantics, SSA, type import,
  runtime overlay, control structuring, or rendering.

### Source Map Inspector

Display for a selected pseudocode token:

- pseudocode route
- line and token range
- IR route
- semantic instruction routes
- assembly routes
- graph block routes
- type/signature routes
- runtime object routes
- user hints
- confidence
- issues

### Hint Deck

The right side of the pseudocode view should offer context-aware hint cards:

- mark no-return
- define switch table
- rename variable
- set variable type
- set function signature
- set calling convention
- bind ObjC selector
- bind ObjC receiver
- bind C++ vtable
- confirm tailcall
- include dead block
- suppress issue

Every hint card should preview:

- what it will change
- what it will invalidate
- which views will refresh
- whether it affects export/evidence packages

## CLI And API Contracts

### CLI

Commands:

```text
damsel decompiler run <project> <procedure>
damsel decompiler quality <project> <procedure>
damsel decompiler source-map <project> <procedure>
damsel decompiler issues <project> <procedure>
damsel decompiler hints <project> <procedure>
damsel decompiler apply-hint <project> <procedure> --kind <kind> --json <payload>
damsel decompiler regression list
damsel decompiler regression run --case <id>
```

### API

Routes:

```text
POST /api/projects/{project_id}/procedures/{procedure_id}/decompiler/runs
GET /api/projects/{project_id}/procedures/{procedure_id}/decompiler/latest
GET /api/projects/{project_id}/procedures/{procedure_id}/decompiler/quality
GET /api/projects/{project_id}/procedures/{procedure_id}/decompiler/passes
GET /api/projects/{project_id}/procedures/{procedure_id}/decompiler/source-map
GET /api/projects/{project_id}/procedures/{procedure_id}/decompiler/issues
GET /api/projects/{project_id}/procedures/{procedure_id}/decompiler/hints
POST /api/projects/{project_id}/procedures/{procedure_id}/decompiler/hints
POST /api/projects/{project_id}/procedures/{procedure_id}/decompiler/hints/{hint_id}/disable
GET /api/projects/{project_id}/decompiler/regression-cases
POST /api/projects/{project_id}/decompiler/regression-runs
```

### Example Quality Payload

```json
{
  "report_id": "dq:proj:main:latest",
  "procedure_id": "proc:proj:image:0x100003f20",
  "overall_confidence": "medium",
  "coverage": {
    "semantic_instruction_count": 84,
    "ir_statement_count": 137,
    "mapped_pseudocode_line_count": 42,
    "excluded_dead_block_count": 1,
    "unknown_control_count": 1
  },
  "issues": [
    {
      "id": "issue:switch-table-bounds-unknown",
      "severity": "warning",
      "category": "switch",
      "message": "Switch table bounds are inferred from a weak range check.",
      "suggested_hints": ["switch-entry-count", "switch-table-address"]
    }
  ],
  "profiles": {
    "architecture": "apple-arm64e-user",
    "abi": "apple-arm64e-user",
    "runtime_overlay": "objc-dyld-cache"
  }
}
```

## Regression Corpus

### Case Model

Fields:

- `case_id`
- `name`
- `artifact_kind`
- `fixture_path`
- `procedure_selector`
- `architecture_profile`
- `abi_profile`
- `required_passes`
- `expected_semantics`
- `expected_issues`
- `forbidden_claims`
- `performance_budget`
- `redaction_policy`

### Expectation Types

Expectations should target behavior:

- procedure boundary exists
- branch preserved
- switch cases recovered
- no-return propagation present
- dead block excluded with visible issue
- ObjC selector resolved
- ObjC receiver unresolved
- C++ vtable candidate weak
- Swift demangled label present
- PAC target not flattened
- KDK symbol overrides weak name
- pseudocode line maps to instruction range
- generated text contains issue marker
- generated text does not claim type

Avoid relying only on exact pseudocode text. Text snapshots are useful, but
they are too brittle to be the sole correctness gate.

### Corpus Families

Required families:

- tiny C direct call
- tiny C branch
- tiny C switch
- tiny no-return
- tiny tailcall
- tiny thunk
- tiny stack local
- tiny ObjC message
- tiny ObjC ivar
- tiny C++ virtual call
- tiny Swift symbol
- tiny arm64e PAC callsite
- dyld stub call
- dyld shared-cache projected ObjC method
- synthetic kernel fileset function
- KDK symbolication function
- panic-routed function
- malformed function negative case
- large function performance case

## Performance And Responsiveness

Budgets:

- initial pseudocode request returns job id quickly
- cached pseudocode open under interactive budget
- quality report page under interactive budget
- source-map token hover under hover budget
- large-function decompiler job cancellable
- Workbench scrolling never waits on whole-function regeneration
- decompiler cache invalidation is scoped to affected procedures

Metrics:

- run duration
- pass duration
- peak memory
- IR statement count
- pseudocode line count
- issue count
- cache hit rate
- cancellation latency
- stale-result display duration
- source-map lookup latency

## Automation, MCP, And Assistant Integration

Automation tools should expose:

- run decompiler
- get quality report
- list issues
- list source-map routes
- list suggested hints
- export pseudocode with evidence

Mutation tools must require permission:

- apply hint
- rename variable
- set type
- set switch bounds
- suppress issue

Assistant behavior:

- cite pseudocode lines through route ids
- cite supporting instructions and IR
- warn when a quality report has blocking issues
- never improve pseudocode silently
- propose hints as reviewed suggestions
- refuse to present pseudocode as recompilable source unless evidence
  specifically supports that claim

## Implementation Slices

### DQ1: Pass Ledger And Quality Skeleton

Deliver:

- `DecompilerRun`
- `DecompilerPassRun`
- `DecompilerQualityReport`
- empty quality API
- CLI quality command

Acceptance:

- fixture run with pass records
- quality JSON snapshot
- cancellation state snapshot

### DQ2: Source Map Foundation

Deliver:

- pseudocode line/token route model
- route from line to IR placeholder
- route from IR placeholder to instruction ranges
- source-map API

Acceptance:

- line-to-instruction snapshot
- unmapped line issue snapshot

### DQ3: Issue Model

Deliver:

- issue categories
- severity model
- Workbench issue payload
- suppressible issue policy

Acceptance:

- dead-code issue fixture
- switch issue fixture
- ObjC unresolved selector issue fixture

### DQ4: Hint Transactions

Deliver:

- hint model
- hint application records
- invalidation reasons
- transaction integration

Acceptance:

- variable rename transaction
- switch hint transaction
- no-return hint transaction
- stale pseudocode marker snapshot

### DQ5: Dead-Code And Switch Recovery UX

Deliver:

- dead-code policy records
- switch candidate records
- hint deck payloads
- pseudocode header counters

Acceptance:

- excluded block fixture
- include dead-code fixture
- switch table bounds fixture

### DQ6: Apple Runtime Overlay Quality

Deliver:

- ObjC overlay quality records
- C++ overlay quality records
- Swift-adjacent quality records
- XNU profile quality records
- PAC quality records

Acceptance:

- ObjC message fixture
- C++ virtual call fixture
- Swift demangle fixture
- kernel profile fixture
- PAC negative fixture

### DQ7: Regression Corpus Runner

Deliver:

- regression case schema
- expectation schema
- runner command
- report output

Acceptance:

- fixture suite report
- forbidden-claim failure
- performance-budget failure

### DQ8: Native Workbench Quality Surfaces

Deliver:

- pseudocode header
- quality drawer
- pass ledger
- source-map inspector
- hint deck
- stale-result state
- cancellation UI

Acceptance:

- Swift model decode tests
- route selection tests
- issue drawer snapshots
- hint preview snapshots
- cancellation UI test

## Completion Criteria

Decompiler quality work is credible when:

- every pseudocode line has a source-map status
- every unsupported recovery area emits an issue
- dead-code inclusion/exclusion is visible
- switch hints are transactions and trigger bounded reanalysis
- variable rename/type edits are transactions
- ObjC/C++/Swift/XNU overlays cite evidence and confidence
- PAC states remain explicit
- quality reports can be exported
- regression cases test semantics and forbidden claims
- large decompiler jobs are cancellable
- cached pseudocode marks staleness after input changes
- automation and assistant tools cite decompiler evidence rather than raw
  narrative guesses

## Source Links

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper SDK `CPUContext`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUContext.h>
- Hopper SDK `CPUDefinition`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUDefinition.h>
- Hopper SDK `HPDisassembledFile`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/HPDisassembledFile.h>
- Public Hopper script workflows: <https://github.com/melomac/Hopper>
- Apple ARM64 ABI guidance: <https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms>
- Clang pointer authentication: <https://clang.llvm.org/docs/PointerAuthentication.html>
- Swift ABI mangling: <https://github.com/swiftlang/swift/blob/main/docs/ABI/Mangling.rst>
