# Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec

Status: implementation-grade planning spec. This is not an implementation
claim and it does not change Damsel's current read-only safety posture.

Purpose: define the native Workbench product surface for Hopper-grade patching,
hex editing, instruction assembly, NOP-region replacement, patch preview,
copy-on-write source revisions, export, reanalysis, diagnostics, and Apple
artifact safety. The existing patching/hex/transaction spec defines the core
architecture. This spec defines the user-facing Patch Studio and the deeper
Apple-specific implementation contracts needed to make binary mutation safe,
auditable, and competitive.

## Why This Spec Exists

Hopper's public tutorial presents patching as a normal part of the disassembly
document:

- bytes have types and can be changed
- the hex editor is synchronized with assembly
- selecting a byte inside an instruction selects the whole instruction byte
  span
- editing instruction bytes can invalidate the instruction and procedure
- undo/redo can recover from a mistake
- an embedded assembler can assemble a new instruction
- a NOP Region command can replace selected instructions

Hopper's FAQ also says the disassembled binary can be modified with the
embedded hex editor, assembler, or Python scripts. Its homepage emphasizes a
native macOS interface, Python scripting, SDK extensibility, interactive CFG
annotation, and real-time debugging. Those surfaces intersect around mutation:
patching is not just "write a byte." It touches document typing, procedures,
assembly, graph, debugger launch, exports, scripts, plugins, and user trust.

Damsel should not rush into mutation. Apple-system binaries are full of sharp
edges:

- Mach-O offsets do not always map cleanly to virtual addresses
- universal binaries have per-slice offsets
- app bundles require executable replacement plus code-signing decisions
- dyld shared caches and projected cache images should not be patched in place
- kernel collections and KDK files should be treated as evidence sources, not
  patch targets
- chained fixups, code signatures, entitlements, debug info, and source maps can
  become stale immediately after a change
- debugger launch against a patched revision must be explicit
- exported patched binaries can be confused with originals unless provenance is
  embedded in the project and report

This spec creates the product layer that makes mutation visible, reversible in
project state, copy-on-write by default, and honest about what became invalid.

## Companion Documents

Read this spec with:

- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
- [Corpus, Benchmark, And Regression Plan](corpus-benchmark-regression-plan.md)
- [Quality And Validation Plan](quality-validation-plan.md)

## Product Thesis

Patch Studio is a revision workbench, not a hex text box.

Every proposed mutation should be a structured patch plan with:

- source identity
- source revision
- address-space mapping
- original bytes
- proposed bytes
- patch kind
- assembler input, when applicable
- affected typed-byte ranges
- affected instructions
- affected procedures
- affected xrefs
- affected runtime objects
- affected debug/source maps
- code-signing impact
- export policy
- analysis invalidation
- safety warnings
- undo/redo transaction
- evidence export

The user should be able to ask:

- What exactly will change?
- Which file offset and virtual address does this touch?
- Is this byte range inside a real file, a projection, zero-fill, sidecar, or
  derived view?
- Which instructions/functions/procedures become stale?
- Does the patch fit in place?
- Does it affect branch range, fixups, code signature, entitlements, or debug
  maps?
- Can this be exported as a standalone binary, app bundle revision, or only a
  project-side patch record?
- Can I launch the patched revision under the debugger?
- Can I prove later which patch was applied and why?

If the UI cannot answer those questions, it should not offer an Apply button.

## Non-Goals

Do not include in early Patch Studio:

- in-place modification of original files
- in-place dyld shared-cache mutation
- in-place kernel collection mutation
- automatic code-signing repair
- automatic entitlement generation
- live process memory writes
- hot patching
- patching through AI/MCP without an explicit audited capability
- recompilation from pseudocode
- multi-function code cave relocation as a first-wave feature
- automatic branch island insertion
- runtime bypass generation
- distributing patched binaries

The first product milestone is safe preview and copy-on-write export, not
aggressive binary rewriting.

## Workbench UX

### Patch Studio Entry Points

Patch Studio should open from:

- hex byte selection
- assembly instruction selection
- graph node selection
- pseudocode line selection, after source maps are stable
- feature-map region selection
- problem suggestion
- command palette
- context menu
- automation dry-run result

Entry commands:

- Open Patch Studio
- Edit Bytes
- Assemble Instruction
- NOP Selected Instructions
- Replace With Branch
- Replace String Bytes
- Create Patch Plan From Selection
- Show Patchability
- Show Patch History
- Export Patched Revision

Every entry point resolves to a canonical route and patchability query before a
plan can be created.

### Patch Studio Layout

Patch Studio should have five regions:

- Target
- Editor
- Preview
- Impact
- Export

Target:

- source
- source revision
- artifact kind
- architecture slice
- address-space mode
- selected range
- file offset
- virtual address
- section/segment
- function/procedure

Editor:

- hex bytes
- assembly text
- NOP action
- string bytes
- branch helper
- comments/reason

Preview:

- original bytes
- proposed bytes
- instruction diff
- disassembly before/after
- graph diff, when available
- pseudocode stale marker, when available
- patch fit status

Impact:

- invalidated instructions
- invalidated procedures
- affected xrefs
- affected ObjC/C++/kernel/runtime objects
- source/debug info impact
- code-signing impact
- export warnings
- reanalysis plan

Export:

- project-only patch
- derived raw binary
- derived Mach-O slice
- derived universal binary
- app bundle executable replacement plan
- evidence package
- debugger launch with patched revision

### Preview Before Apply

The Apply button should remain disabled until:

- source hash precondition is known
- address mapping is unambiguous
- patchability status is not readonly
- generated bytes are available
- affected range is fully known
- invalidation plan exists
- export policy is selected
- warnings are acknowledged

Patch Studio should never let a user "just type bytes" into an unknown mapping.

### Difference Display

Display differences at multiple levels:

- byte diff
- instruction diff
- semantic annotation diff
- function/procedure stale diff
- graph stale diff
- xref stale diff
- signature/fixup warning diff
- export artifact diff

The byte diff is necessary, but not sufficient.

### Patch History

Patch History should show:

- patch revisions
- patch transactions
- author/session
- timestamp
- source revision
- original hash
- patched hash
- command used
- reason/comment
- affected ranges
- export artifacts
- reanalysis status
- debugger launch usage

The user should be able to select any patch revision and see what changed.

## Data Model

### PatchStudioSession

Fields:

- `id`
- `project_id`
- `source_id`
- `source_revision_id`
- `artifact_kind`
- `architecture`
- `address_space_id`
- `route`
- `selected_range`
- `patchability`
- `draft_plan_id`
- `preview_id`
- `impact_id`
- `export_plan_id`
- `created_at`
- `updated_at`

### PatchPlan

Fields:

- `id`
- `project_id`
- `source_id`
- `source_revision_id`
- `kind`
- `route`
- `target_range`
- `original_bytes`
- `proposed_bytes`
- `assembler_input`
- `nop_policy`
- `string_policy`
- `branch_policy`
- `source_hash_precondition`
- `address_mapping`
- `fit_policy`
- `warnings`
- `created_by_command`
- `user_reason`
- `status`

Kinds:

- `byte_replace`
- `assemble_instruction`
- `nop_region`
- `string_replace`
- `branch_replace`
- `project_annotation_only`
- `external_patch_import`

Statuses:

- `draft`
- `preview_ready`
- `blocked`
- `applied_project_revision`
- `exported`
- `superseded`
- `abandoned`

### PatchPreview

Fields:

- `id`
- `patch_plan_id`
- `before_bytes`
- `after_bytes`
- `before_disassembly`
- `after_disassembly`
- `before_annotations`
- `after_annotations`
- `instruction_changes`
- `procedure_changes`
- `xref_changes`
- `runtime_object_changes`
- `debug_info_changes`
- `problems`
- `metrics`

### PatchImpact

Fields:

- `id`
- `patch_plan_id`
- `byte_ranges`
- `instruction_ids`
- `procedure_ids`
- `basic_block_ids`
- `xref_ids`
- `objc_object_ids`
- `cpp_object_ids`
- `kernel_object_ids`
- `fixup_ids`
- `symbolication_profile_ids`
- `debug_source_map_ids`
- `invalidated_fact_ids`
- `required_reanalysis_jobs`
- `safety_warnings`

### PatchRevision

Fields:

- `id`
- `project_id`
- `base_source_revision_id`
- `source_id`
- `patch_plan_ids`
- `source_hash_before`
- `source_hash_after`
- `created_at`
- `created_by`
- `transaction_id`
- `artifact_outputs`
- `analysis_status`
- `debug_launch_compatibility`

### PatchExportPlan

Fields:

- `id`
- `patch_revision_id`
- `export_kind`
- `destination_grant_id`
- `filename_policy`
- `signature_policy`
- `bundle_policy`
- `debug_info_policy`
- `evidence_policy`
- `redaction_policy`
- `problems`

Export kinds:

- `project_patch_only`
- `raw_file_copy`
- `macho_slice`
- `universal_binary`
- `app_bundle_executable`
- `evidence_package`
- `debug_launch_artifact`

## Patchability Model

Patchability is a first-class query.

Statuses:

- `patchable_copy_on_write`
- `patchable_project_only`
- `preview_only`
- `readonly_original`
- `readonly_projection`
- `virtual_bytes`
- `unsupported_container`
- `ambiguous_mapping`
- `blocked_by_policy`

Reasons:

- source is original input
- source is derived projection
- range crosses section boundary
- range crosses instruction boundary
- range is zero-fill
- range is compressed or unavailable
- range maps to dyld shared-cache projection
- range maps to kernel collection projection
- range maps to sidecar/debug info
- range has no stable file offset
- architecture assembler unsupported
- code signature impact unresolved
- export policy missing

Patchability should be visible before the user begins editing.

## Hex Editing UX

### Byte Selection

Selection should support:

- single byte
- contiguous range
- current instruction
- current basic block
- current function range
- data object range
- string range
- fixup pointer range

Selecting a byte inside an instruction should highlight:

- full instruction byte span
- owning instruction row
- owning graph node
- owning function/procedure
- patchability status
- source mapping

### Edit Bytes Sheet

The byte editor should show:

- old bytes
- new bytes
- ASCII preview
- endian helpers
- integer helpers
- pointer helpers
- byte count
- range mapping
- patchability
- invalidation preview

Validation:

- same-size replacement by default
- explicit expand/shrink only for future relocation-enabled patches
- reject writes to unavailable bytes
- reject ambiguous projection mappings
- warn when changing instruction bytes from hex mode

### Typed Byte Fallout

When bytes change:

- byte classification becomes stale
- instruction decode becomes stale
- procedure membership may become stale
- xrefs may become stale
- ObjC/C++/kernel facts may become stale
- decompiler/pseudocode becomes stale
- graph layout may become stale

Patch Studio should show this before applying.

## Assembler UX

### Assemble Instruction Sheet

Fields:

- route
- architecture
- CPU mode
- syntax flavor
- current instruction
- replacement assembly
- fixed-size policy
- generated bytes
- disassembly check
- diagnostics

Default policy:

- replacement must fit exactly in selected instruction bytes
- if generated bytes are shorter, offer pad with NOP when valid
- if generated bytes are longer, block and explain relocation/code-cave future
  work

### Assembler Diagnostics

Diagnostics should include:

- syntax error
- unsupported mnemonic
- unsupported operand
- branch target out of range
- instruction too large
- CPU mode mismatch
- relocation required
- symbol unresolved
- architecture unsupported
- backend assembler unavailable

Each diagnostic should have a location in the input text when possible.

### Disassembly Round Trip

Generated bytes should be decoded immediately:

- show assembled bytes
- show decoded instruction
- show whether decoded instruction matches intent
- show flags for aliases/canonical forms
- show branch target calculation
- show PAC/arm64e concerns when relevant

Do not trust assembler output without round-trip decode.

## NOP Region UX

### NOP Selection Rules

NOP Region should require:

- instruction-aligned selection
- architecture NOP sequence available
- no partial instruction overlap
- no unknown-width instructions
- source range patchable

Show:

- number of instructions
- total bytes
- generated NOP bytes
- affected control-flow edges
- affected xrefs
- affected function/procedure facts
- branch fallthrough after patch

### NOP Policies

Policies:

- exact replacement only
- preserve instruction count where possible
- preserve byte count always
- mark removed instructions as patched-out facts
- create patch evidence note

Future:

- architecture-specific trap instruction replacement
- conditional branch inversion helper
- branch-to-fallthrough helper

## Branch And Control-Flow Helpers

Later Patch Studio should include helpers:

- invert conditional branch
- replace conditional branch with unconditional branch
- force fallthrough
- branch to selected block
- return immediate value stub
- call replacement stub

Early design constraints:

- do not ship these until branch range calculation is reliable
- do not create code caves automatically
- require before/after CFG preview
- preserve evidence that control flow was user-patched

## Apple Artifact Safety

### Standalone Mach-O

Supported early:

- copy-on-write derived Mach-O output
- same-size byte/instruction replacement
- patched source revision in project
- targeted reanalysis

Warnings:

- code signature invalidated
- LC_CODE_SIGNATURE may become stale
- debug info may no longer match
- chained fixups may be affected if pointer bytes changed
- notarization state not preserved

### Universal Binary

Rules:

- patch selected architecture slice only
- preserve other slices byte-for-byte
- recalculate output container offsets if needed
- record slice identity
- warn if export changes container hash

Do not let a patch to arm64 silently affect x86_64 or arm64e slices.

### App Bundle

Patch Studio can plan:

- patch executable inside bundle
- export derived bundle copy
- mark signing invalid
- optionally run user-provided signing step later

Do not auto-sign with arbitrary identity in early waves. Signing is a separate
trust/distribution workflow.

### Dyld Shared Cache

Default:

- no in-place patching
- no full-cache rewriting
- no system cache mutation

Allowed early:

- project-only patch plan against projected image
- evidence export
- derived extracted image patch only when source registration can prove a
  stable writable derived artifact

Warn:

- projected addresses may not map to standalone runnable bytes
- cache links and fixups may become invalid
- debugger launch is usually not meaningful for the patched projection

### Kernel Collections And KDKs

Default:

- readonly evidence sources
- project-only patch experiments
- no export as runnable kernel artifact

Patch Studio should support hypothetical analysis patches for reasoning, but it
must label them as non-deployable project revisions unless a separate future
kernel workflow is designed.

### dSYM, DWARF, And Sidecars

Treat debug info as sidecar metadata, not a patch target in Patch Studio.

If code bytes change:

- source line maps may become stale
- inline callsites may become stale
- variable locations may become stale
- symbolication profile should show mismatch

## Code Signing And Entitlements

Patch Studio should detect and report:

- code signature load command
- signed code directory range
- entitlements blob presence
- hardened runtime flags when available
- executable segment changes
- bundle signing context

Policies:

- no automatic signing first wave
- export can include "unsigned patched artifact"
- export can include "requires resigning" problem
- user-provided signing integration is future work
- never imply a patched app remains trusted/notarized

## Patch Transactions

Patch transaction fields:

- command id
- source revision before
- source revision after
- patch plan id
- preview id
- affected routes
- invalidated facts
- export outputs
- user reason
- warnings acknowledged
- undo record
- redo record

Undo semantics:

- undo reverts project patch revision selection
- undo does not delete exported files unless user explicitly chooses cleanup
- undo should restore analysis view to previous source revision

Redo semantics:

- redo reapplies the project patch revision
- redo should not rewrite external output without confirmation

## Reanalysis Pipeline

After applying a project patch:

1. create patched source revision
2. update typed byte map for changed ranges
3. invalidate decoded instructions in affected windows
4. invalidate function/procedure records crossing changed ranges
5. invalidate xrefs from/to changed instructions/data
6. invalidate graph layouts for affected functions
7. invalidate decompiler/pseudocode for affected functions
8. invalidate ObjC/C++/kernel runtime facts touching changed ranges
9. invalidate debug/source maps if address-to-line assumptions changed
10. enqueue targeted reanalysis jobs
11. create patch problems for any degraded facts

The user should see both the patch and the reanalysis status.

## API Contracts

### Query Patchability

```text
GET /api/projects/{project_id}/sources/{source_id}/patchability
```

Query:

```text
?route=instruction:0x100003f20&length=4
```

Response:

```json
{
  "status": "patchable_copy_on_write",
  "range": {
    "address": "0x100003f20",
    "file_offset": "0x3f20",
    "length": 4
  },
  "reasons": [],
  "warnings": ["code_signature_will_be_invalid"],
  "allowed_patch_kinds": ["byte_replace", "assemble_instruction", "nop_region"]
}
```

### Create Patch Plan

```text
POST /api/projects/{project_id}/patch/plans
```

Request:

```json
{
  "source_id": "source-main",
  "source_revision_id": "rev-original",
  "kind": "assemble_instruction",
  "route": {
    "kind": "instruction",
    "address": "0x100003f20"
  },
  "assembler_input": "mov x0, #0",
  "fit_policy": "exact_or_nop_pad",
  "user_reason": "Test error-path behavior."
}
```

### Preview Patch Plan

```text
POST /api/projects/{project_id}/patch/plans/{plan_id}/preview
```

Response includes:

- before bytes
- after bytes
- before disassembly
- after disassembly
- diagnostics
- impact summary
- exportability

### Apply Patch Plan

```text
POST /api/projects/{project_id}/patch/plans/{plan_id}/apply
```

Request:

```json
{
  "mode": "project_revision",
  "acknowledged_warnings": ["code_signature_will_be_invalid"]
}
```

Response:

```json
{
  "patch_revision_id": "patch-rev-1",
  "transaction_id": "txn-123",
  "reanalysis_jobs": ["job-1", "job-2"],
  "problems": []
}
```

### Export Patch Revision

```text
POST /api/projects/{project_id}/patch/revisions/{revision_id}/export
```

Request:

```json
{
  "export_kind": "raw_file_copy",
  "destination_grant_id": "grant-output",
  "signature_policy": "leave_unsigned",
  "evidence_policy": "include_patch_report"
}
```

## Native Store Architecture

Add stores:

- `PatchStudioStore`
- `PatchabilityStore`
- `PatchPlanStore`
- `PatchPreviewStore`
- `PatchImpactStore`
- `PatchRevisionStore`
- `PatchExportStore`
- `AssemblerStore`
- `NopRegionStore`
- `PatchHistoryStore`
- `PatchProblemStore`

Responsibilities:

`PatchStudioStore`:

- active patch sessions
- selected target
- panel state
- route synchronization

`PatchabilityStore`:

- patchability queries
- status caching
- invalidation on route/source changes

`PatchPlanStore`:

- draft plans
- validation
- warning acknowledgements

`PatchPreviewStore`:

- before/after bytes
- disassembly diff
- diagnostics

`PatchImpactStore`:

- affected facts
- invalidation plan
- reanalysis plan

`PatchExportStore`:

- export options
- destination grants
- signing policy
- evidence policy

`PatchHistoryStore`:

- patch revisions
- transactions
- export artifact links

## Automation And Scripting Boundary

Automation should support read-only patch analysis first:

- query patchability
- create dry-run patch plan
- preview patch plan
- export patch report

Mutation capabilities:

- apply patch plan
- export patched artifact

These require explicit capability:

- `patch.apply`
- `patch.export`

AI/MCP tools should never directly write bytes. They can propose patch plans
that the user reviews in Patch Studio.

## Debugger Integration

Patch Studio should integrate with debugger only after:

- patch revisions exist
- derived output artifacts exist
- debugger launch targets can choose source revision
- code-signing warnings are visible
- route mapping can distinguish original and patched revision

Commands:

- Launch Original Under Debugger
- Launch Patched Revision Under Debugger
- Compare Original And Patched Stop
- Show Patch At Current PC

Never silently launch a patched artifact when the user selected the original.

## Export And Evidence

Patch evidence package includes:

- original source identity
- source hash before
- patch revision id
- source hash after
- patch plans
- original/proposed bytes
- assembler text
- disassembly before/after
- affected facts
- warnings
- analysis versions
- export artifact hashes
- user reason
- redaction policy

This is Damsel's chance to exceed Hopper for research workflows: every patch
should be reproducible and reviewable.

## Performance And Responsiveness

Budgets:

- patchability query for visible selection: under 100 ms
- same-size byte preview: under 150 ms
- single-instruction assembly preview: under 250 ms
- NOP region preview for small selection: under 250 ms
- impact summary for one function: under 500 ms
- patch history list open: under 150 ms
- export progress event cadence: under 250 ms

Large artifacts:

- use mmap or bounded byte windows
- do not copy whole dyld cache into memory
- do not render full binary diff by default
- page patch history
- stream export progress
- cancel preview/export jobs

## Security And Safety

Safety rules:

- original inputs are immutable
- project patch revisions are explicit
- external exports require destination grant
- code-signing invalidation is visible
- scripts/AI cannot apply patches without capability
- diagnostic bundles redact paths and bytes unless explicitly included
- patch plans record user reason and warnings acknowledged
- unsupported artifacts fail closed

## Implementation Phases

### Phase 1: Patchability And Disabled Patch Studio

Deliver:

- patchability query model
- disabled Patch Studio UI
- command disabled reasons
- source/artifact patch policy
- route-to-byte-range validation

Acceptance:

- user can ask why a range is or is not patchable
- no byte mutation exists yet

### Phase 2: Read-Only Preview Model

Deliver:

- Patch Studio layout
- byte selection target panel
- before-byte preview
- impact placeholder
- patch history empty state

Acceptance:

- selecting assembly/hex opens Patch Studio with correct target identity

### Phase 3: Byte Replace Dry Run

Deliver:

- same-size byte replacement draft
- preview API
- before/after bytes
- disassembly before/after for affected range
- diagnostics

Acceptance:

- dry run never writes source bytes
- ambiguous mappings are rejected

### Phase 4: Assembler And NOP Dry Run

Deliver:

- assembler backend abstraction
- single-instruction assembly preview
- NOP sequence generation
- round-trip disassembly check
- diagnostics

Acceptance:

- unsupported instruction emits actionable error
- generated bytes are shown before apply

### Phase 5: Project Patch Revision

Deliver:

- copy-on-write source revision
- patch transaction
- undo/redo project revision selection
- targeted invalidation
- patch history

Acceptance:

- original file remains unchanged
- project can switch between original and patched revision

### Phase 6: Export Derived Artifact

Deliver:

- raw file export
- Mach-O slice export
- universal binary slice preservation
- app bundle executable export plan
- code-signing warnings
- export evidence report

Acceptance:

- exported artifact hash is recorded
- signing invalidation is visible

### Phase 7: Reanalysis And Representation Sync

Deliver:

- targeted decode after patch
- graph stale markers
- pseudocode stale markers
- xref invalidation
- runtime object stale markers
- source/debug map mismatch warnings

Acceptance:

- patched revision views do not show stale analysis as current truth

### Phase 8: Automation Dry Runs

Deliver:

- read-only patchability tool
- patch preview tool
- evidence export tool
- audited apply capability

Acceptance:

- automation cannot apply patch without capability

### Phase 9: Debugger Launch Integration

Deliver:

- launch patched revision command
- original/patched target distinction
- patch-at-PC inspector link
- debugger route mapping by source revision

Acceptance:

- debugger never confuses original and patched bytes

## Validation Matrix

### Fixtures

Required fixtures:

- standalone Mach-O same-size byte patch
- single AArch64 instruction replacement
- NOP one instruction
- NOP multi-instruction aligned range
- partial-instruction selection rejected
- branch out-of-range assembly rejected
- unsupported mnemonic rejected
- universal binary single-slice patch
- code signature invalidation warning
- dyld shared-cache projection readonly
- kernel collection readonly
- virtual zero-fill readonly
- string replacement same-size
- source hash mismatch
- patch undo/redo
- patched revision reanalysis
- export evidence package

### UI Tests

Required UI tests:

- open Patch Studio from assembly
- open Patch Studio from hex
- show patchability disabled reason
- preview byte replacement
- preview assembler replacement
- preview NOP region
- apply project revision
- undo patch revision
- show patch history
- export evidence package

### Performance Tests

Required performance tests:

- patchability query latency
- byte preview latency
- assembler preview latency
- NOP preview latency
- impact summary latency
- large artifact byte-window memory
- export cancellation

### Safety Tests

Required safety tests:

- original file hash unchanged after project patch
- ambiguous projection rejected
- readonly cache projection rejected
- script apply rejected without capability
- diagnostics redact local paths by default
- code-signing warning required before export

## Acceptance Criteria

Damsel should not claim Hopper-competitive patching until:

- hex selection is synchronized with assembly and byte classification
- Patch Studio can explain patchability for every selected range
- byte, assembler, and NOP previews show exact bytes before apply
- original inputs remain immutable
- project patch revisions are undoable and redoable
- affected analysis facts become stale or reanalyzed
- code-signing and export risks are visible
- dyld cache and kernel collection projections fail closed
- scripts/AI cannot patch without explicit capability
- exported patch evidence is reproducible
- debugger launch distinguishes original and patched revisions

## Summary

Hopper makes patching feel native because byte typing, hex editing, assembler,
NOP, undo/redo, and debugger launch all live in one document model. Damsel
should match that coherence without copying the risk profile.

The right Damsel feature is Patch Studio: a route-backed, source-revision-aware,
copy-on-write, evidence-producing Workbench for byte edits, assembly patches,
NOP regions, exports, reanalysis, and debugger handoff. That is more work than
a hex editor, but it is the difference between a toy mutation feature and a
trustworthy Apple-binary reverse-engineering workstation.
