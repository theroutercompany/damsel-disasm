# Patching, Hex, Assembly, And Transaction Implementation Spec

Status: implementation architecture plan. This is not an implementation claim
and it does not change Damsel's current read-only safety posture.

Purpose: define the complete path from a read-only synchronized hex view to
Hopper-grade byte editing, assembler-driven patches, NOP-region commands,
patch preview, copy-on-write export, reanalysis, undo/redo, and auditability.

The goal is not to rush binary patching into Damsel. The goal is to make sure
the project kernel, byte map, source registry, address-space graph,
architecture semantics, Workbench commands, and automation permissions are
designed so that future patching can be safe instead of bolted on.

## Research Companions

- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)

## Architecture Companions

- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [API And Data Contracts](api-data-contracts.md)
- [Architecture Decision And Risk Register](architecture-decision-risk-register.md)
- [Corpus, Benchmark, And Regression Plan](corpus-benchmark-regression-plan.md)

## Public Hopper Evidence

Official tutorial evidence:

- Hopper has a hex mode synchronized with the assembly view.
- When the cursor is on a byte inside an instruction, Hopper selects the whole
  instruction byte span.
- Double-clicking a byte can modify its value.
- If a byte edit changes an instruction, Hopper may destroy the instruction
  structure and reset the associated type to undefined.
- If the instruction was part of a procedure, the procedure may be destroyed.
- Undo/redo can roll back mistakes.
- Hopper exposes an embedded assembler through a Modify menu command.
- Hopper exposes a NOP Region command for selected instructions.

Official FAQ evidence:

- Hopper says binaries can be modified through the embedded hex editor,
  assembler, and Python scripts.

Official SDK evidence:

- `HPDisassembledFile` exposes byte type mutation, procedure creation/removal,
  assembler output, NOP data generation, colors, tags, comments, names,
  operand format overrides, problem creation, and undo/redo transactions.
- `HPDocument` exposes integer/address write methods at virtual addresses.
- `CPUDefinition` exposes NOP generation and assembler capability flags.
- `CPUContext` exposes raw instruction assembly for a CPU mode and syntax.

Changelog evidence:

- Hopper has maintained assembler error handling, TBZ/TBNZ assembly issues,
  BSS modifications, patches applied before debugger launch, and hex-editor
  issues in extracted FAT/shared-cache contexts.
- These are product maintenance signals: patching touches loader context,
  file/container offsets, CPU mode, debugger launch, and error UX.

## Core Thesis

Binary mutation is not a local byte-array edit. In a reverse-engineering app it
is a project transaction with source preconditions, byte-map invalidation,
procedure invalidation, xref invalidation, provenance, undo/redo, reanalysis,
preview, export, and platform warnings.

For Damsel, the right sequence is:

1. Read-only byte windows.
2. Synchronized assembly/hex selection.
3. Typed byte classifications and feature map.
4. Annotation transactions for names/comments/tags/manual refs.
5. Patch planning objects that cannot write yet.
6. Patch preview against copy-on-write source revisions.
7. Assembly and NOP generation through architecture semantics.
8. Patch application to a derived output artifact only.
9. Reanalysis over the patched revision.
10. Debugger launch integration only after patch preview and export are stable.

## Non-Goals For Early Waves

Do not implement first:

- in-place edits to source files
- in-place edits to dyld shared caches
- in-place edits to kernel collections
- process memory writes
- patching through MCP or scripts by default
- code signing repair
- entitlements repair
- recompilation from pseudocode
- hot patching a live process
- remote patch deployment

The first useful product surface is a read-only synchronized hex/data view plus
transaction-ready project models. Actual byte writes are deliberately later.

## Product Requirements

### Read-Only Hex/Data View

User story:

- As an analyst, I can select an instruction in assembly and see its exact
  bytes in a synchronized hex/data view.
- As an analyst, I can select bytes in hex and route back to the owning
  instruction, byte classification, section, function, ObjC metadata, C++
  object, or kernel object when known.

Required behavior:

- display file offsets, image addresses, cache addresses, and runtime addresses
  where available
- indicate unmapped, virtual-zero-fill, and unavailable bytes distinctly
- highlight current instruction bytes
- highlight current procedure bytes
- highlight byte classification ranges
- show source provenance for projected shared-cache images
- show whether bytes are patchable, derived, virtual, compressed, or sidecar
  backed
- support large windows with paging
- avoid loading whole source bytes into Swift memory

### Patch Preview

User story:

- As an analyst, I can propose a byte edit, assemble an instruction, or NOP a
  region and inspect the exact affected bytes, facts, procedures, and warnings
  before any output file is written.

Required behavior:

- show original bytes and proposed bytes
- show address and file-offset mappings
- show source hash precondition
- show affected byte classifications
- show affected procedures and xrefs
- show invalidated facts
- show code-signing and container warnings
- show whether the patch is exportable
- reject ambiguous or unsupported source mappings

### Patch Application

User story:

- As an analyst, I can apply a reviewed patch to a copy-on-write output
  artifact and keep the original input untouched.

Required behavior:

- never modify original input in place
- write a new output file or project artifact revision
- record a patch transaction and patch revision
- preserve original source hash and new source hash
- re-run targeted analysis over affected ranges
- show any failed or degraded analysis facts as problems
- allow patch transaction undo inside the project model
- distinguish project undo from deleting exported files

### Assembler And NOP Commands

User story:

- As an analyst, I can assemble an instruction at a route or replace selected
  instructions with NOP bytes when the architecture semantics layer supports
  it.

Required behavior:

- require a concrete source, address space, CPU mode, syntax, and instruction
  size policy
- reject instructions that cannot fit when fixed-size replacement is required
- permit multi-instruction assembly only when explicitly supported
- show generated bytes before application
- produce architecture-specific errors with actionable messages
- record assembler text in patch provenance
- use architecture semantics, not ad hoc string rewriting

## Data Model

### Byte Window

```text
ByteWindow {
  project_id
  source_id
  address_space_id
  start_address
  file_offset?
  cache_address?
  image_address?
  runtime_address?
  bytes[]
  byte_states[]
  classifications[]
  ownership[]
  patchability[]
  page
}
```

`byte_states` values:

- mapped
- virtual_zero_fill
- absent
- protected
- derived
- sidecar
- patched_preview
- patched_committed

### Patchability

```text
Patchability {
  range
  status
  reason
  export_policy
  required_capabilities
}
```

Statuses:

- patchable_copy_only
- patchable_project_preview_only
- readonly_source
- derived_projection
- virtual_bytes
- unsupported_container
- ambiguous_mapping
- missing_writer
- unsafe_without_resign

### Patch Plan

```text
PatchPlan {
  id
  project_id
  source_id
  source_revision_id
  source_hash_precondition
  address_space_id
  route
  operations[]
  affected_ranges[]
  affected_objects[]
  warnings[]
  invalidation_plan
  generated_by
  created_at
}
```

Operation kinds:

- replace_bytes
- assemble_instruction
- nop_region
- fill
- restore_original
- import_patch_file

### Patch Operation

```text
PatchOperation {
  id
  kind
  target_range
  original_bytes
  replacement_bytes
  assembler_text?
  cpu_mode?
  syntax?
  fixed_size_policy?
  provenance
}
```

Fixed-size policies:

- must_equal_original_length
- may_shrink_with_nop_padding
- may_expand_only_if_relayout_supported

Initial Damsel policy:

- only `must_equal_original_length` and `may_shrink_with_nop_padding` are
  allowed.
- relayout is out of scope.

### Patch Transaction

```text
PatchTransaction {
  id
  project_id
  patch_plan_id
  source_revision_before
  source_revision_after
  operations[]
  inverse_operations[]
  invalidated_facts[]
  scheduled_jobs[]
  actor
  timestamp
  status
}
```

Statuses:

- planned
- previewed
- applied_to_project_revision
- exported
- reverted_in_project
- failed

### Source Revision

```text
SourceRevision {
  id
  source_id
  parent_revision_id?
  content_hash
  size
  created_by_transaction_id?
  output_path?
  is_original
  is_exported
}
```

Rules:

- original source revision is immutable
- patch application creates a derived revision
- analysis facts always refer to a source revision
- Workbench displays the active revision explicitly

## Address Mapping Requirements

Patching needs stronger address conversion than read-only navigation.

Every patch target must resolve:

- project id
- source id
- source revision id
- address-space id
- virtual address or file offset
- mapped file range
- byte length
- section/segment ownership
- container ownership

Reject a patch when:

- the route maps to multiple source ranges
- the route maps only to virtual zero-fill bytes
- the range crosses non-contiguous file spans
- the range crosses a protected container boundary
- the range belongs to a projected dyld-cache image without a writer policy
- the source hash does not match the patch precondition
- the active source revision changed after preview

## Byte Classification Invalidation

Patch preview must compute:

- byte classifications affected
- instructions affected
- procedures affected
- xrefs affected
- runtime objects affected
- decompiler/IR facts affected
- search index ranges affected
- symbolication assumptions affected

Simple rule:

- any patched byte inside a procedure invalidates the procedure's decode,
  blocks, edges, xrefs, recovered values, IR, decompiler-lite summary, and
  graph layout

More precise later:

- patching non-control instruction operands may only invalidate instruction,
  value-flow, xrefs, and summaries
- patching a branch/call/return invalidates CFG and caller/callee facts
- patching a literal pointer invalidates data refs and runtime refs
- patching metadata invalidates runtime object records

The first implementation should choose conservative invalidation and expose
its breadth as a problem/notice rather than pretending precision.

## Architecture Semantics Integration

Assembler and NOP support belongs in the architecture semantics layer.

Required trait capabilities:

```text
ArchitectureSemantics {
  assemble(address, cpu_mode, syntax, text) -> AssemblyResult
  nop_bytes(address, cpu_mode, length) -> NopResult
  instruction_span(address) -> InstructionSpan
  can_patch_instruction(address, replacement) -> PatchFit
}
```

`AssemblyResult` includes:

- bytes
- instruction count
- diagnostics
- required CPU mode
- syntax variant
- fixed-size fit
- possible branch target changes
- possible data reference changes

`NopResult` includes:

- bytes
- exact coverage flag
- unsupported length reason
- alignment notes

AArch64 policy:

- fixed-width 4-byte instructions are a good first target
- replacement must preserve 4-byte alignment
- AArch64e PAC instructions must be decoded and displayed accurately, but
  patching PAC/authentication-sensitive flows should warn aggressively
- branch immediate range checks must be explicit
- conditional branches and test-bit branches must report range limitations

## Workbench UX

### Hex/Data View

UI surfaces:

- hex bytes
- ASCII side column
- address/file-offset gutter
- byte classification overlay
- source revision badge
- patchability badge
- selection sync with assembly/feature map/inspector
- context menu for copy bytes/address/range
- disabled patch commands until write mode exists

Interaction rules:

- selecting assembly selects instruction bytes
- selecting byte range updates route selection
- double click does not edit in early read-only mode
- edit affordances are visible only behind future write mode
- unsupported bytes explain why they are not patchable

### Patch Preview Panel

Panel sections:

- summary
- original bytes
- replacement bytes
- generated assembly, if any
- affected procedures
- affected xrefs
- invalidation plan
- warnings
- export target
- capability requirements

Warnings:

- will break code signature
- source is a projected cache image
- source is a kernel collection member
- patch touches ObjC metadata
- patch touches branch target
- patch touches procedure entry
- patch invalidates decompiler facts
- reanalysis may fail

### Command Registry

Commands:

- open hex at selection
- copy selected bytes
- copy file offset
- copy image address
- copy cache address
- preview byte patch
- preview assembled instruction
- preview NOP region
- apply patch to project revision
- export patched file
- revert patch transaction

Each command must declare:

- required route kind
- required selection kind
- required capability
- project mutation class
- undo support
- confirmation level

## API Contracts

### Read Byte Window

```text
GET /api/projects/{project_id}/bytes/window
  ?source_id=
  &space_id=
  &start=
  &length=
  &revision_id=
```

Response includes:

- `ByteWindow`
- classifications
- patchability
- route mappings
- provenance

### Create Patch Preview

```text
POST /api/projects/{project_id}/patches/preview
```

Request:

```json
{
  "source_id": "source:1",
  "revision_id": "source-rev:original",
  "space_id": "addrspace:image:1",
  "range": { "start": "0x100003f20", "length": 4 },
  "operation": {
    "kind": "assemble_instruction",
    "text": "mov w0, #0",
    "cpu_mode": "aarch64",
    "syntax": "apple"
  }
}
```

Response:

```json
{
  "patch_plan": {},
  "preview_bytes": {},
  "warnings": [],
  "affected_objects": [],
  "invalidation_plan": {},
  "can_apply": false
}
```

Initial `can_apply` can remain false while Damsel only supports preview.

### Apply Patch Plan

```text
POST /api/projects/{project_id}/patches/{patch_plan_id}/apply
```

This endpoint should not exist until source revisions, copy-on-write output,
and reanalysis are implemented.

### Export Patched Artifact

```text
POST /api/projects/{project_id}/patches/{patch_transaction_id}/export
```

Rules:

- explicit output path or app save panel
- never overwrite original without explicit user-selected path
- warn about code signing and notarization
- include patch manifest sidecar option

## CLI Contracts

Read-only first:

```text
damsel project bytes <project> --source <id> --addr <addr> --len <n> --json
damsel project patch preview <project> --source <id> --addr <addr> --bytes <hex>
damsel project patch assemble <project> --source <id> --addr <addr> --asm <text>
damsel project patch nop <project> --source <id> --addr <addr> --len <n>
```

Later:

```text
damsel project patch apply <project> <patch-plan-id>
damsel project patch export <project> <transaction-id> --output <path>
damsel project patch revert <project> <transaction-id>
```

CLI output must include:

- original bytes
- replacement bytes
- source hash precondition
- affected facts
- warnings
- whether application is supported

## Automation And MCP Policy

Read-only automation may:

- read byte windows
- ask whether a range is patchable
- create patch previews if user grants capability
- explain patch warnings

Automation may not initially:

- apply patch plans
- export patched artifacts
- write process memory
- patch shared caches
- patch kernel collections

Mutating patch tools require:

- explicit dangerous capability
- local-only execution
- visible Workbench activity log
- transaction id
- source hash precondition
- confirmation prompt
- audit record

## Source-Specific Policies

### Standalone Mach-O

Initial patchable target.

Allowed later:

- copy-on-write output
- same-size instruction replacement
- NOP region
- raw byte replacement

Warnings:

- code signature invalidation
- entitlements may be invalid
- offsets may differ from loaded runtime address
- hardened runtime may prevent debugging patched output

### Universal Binary

Policy:

- patch selected slice only
- preserve other slices byte-for-byte
- output universal container with updated slice content
- report slice architecture and UUID changes

Risks:

- code signature covers entire container
- file offsets for following slices may shift if size changes

Initial rule:

- no size-changing patch.

### Dyld Shared Cache

Policy:

- no in-place patching
- no direct cache rewrite in first implementation
- allow projected-image patch preview only
- allow extracting a projected image revision later if legally and technically
  supported

Reason:

- cache mappings are container-derived, multi-image, OS-managed, and often
  code-signed or authenticated in ways Damsel should not pretend to repair.

### Kernel Collections

Policy:

- no patching in early or middle waves
- allow patch preview for educational analysis only after mapping is exact
- never overwrite kernel artifacts

Reason:

- kernel artifacts are high-risk, signed, platform-protected, and easy to
  misuse.

### BSS And Virtual Sections

Policy:

- represent as virtual bytes
- allow annotations and type hints
- do not write bytes unless an output format can represent the change

## Validation Plan

### Read-Only Byte Window Gate

Fixtures:

- standalone Mach-O text bytes
- data section bytes
- string bytes
- universal slice bytes
- projected cache image bytes
- virtual zero-fill bytes

Evidence:

- byte windows return stable bytes
- instruction selection maps to exact byte span
- byte classification overlays are correct
- unavailable bytes are represented explicitly

### Patch Preview Gate

Fixtures:

- replace one data byte
- replace one AArch64 instruction with same length
- NOP one AArch64 instruction
- reject unaligned instruction address
- reject virtual zero-fill patch
- reject ambiguous projected-cache mapping
- reject stale source hash

Evidence:

- preview reports original/replacement bytes
- warnings are deterministic
- invalidation plan names affected procedure and xrefs
- no input file changes

### Assembler Gate

Fixtures:

- valid fixed-width AArch64 instruction
- invalid assembly text
- branch immediate out of range
- instruction requiring unsupported CPU mode
- NOP length not divisible by instruction width

Evidence:

- diagnostics are actionable
- generated bytes match known encoding fixtures or assembler oracle
- unsupported cases fail closed

### Transaction Gate

Fixtures:

- patch plan creates transaction
- project undo reverts active source revision
- redo reapplies only if preconditions still match
- branch of edits invalidates redo
- transaction appears in Workbench history

Evidence:

- original revision remains immutable
- transaction log is deterministic
- invalidated facts are marked stale
- reanalysis jobs are scheduled

### Export Gate

Fixtures:

- standalone Mach-O copy output
- universal binary same-size slice patch
- output path conflict
- write permission failure

Evidence:

- output hash changes
- original hash unchanged
- manifest records patch operations
- code-signing warning emitted

## Milestones

### P0: Read-Only Hex Foundation

- byte window API
- Workbench hex/data view
- selection sync with assembly and feature map
- patchability model with all writes disabled
- fixture tests for byte windows

### P1: Patch Planning Model

- patch plan data model
- source revision model
- invalidation plan model
- CLI/API patch preview
- Workbench preview panel

### P2: Architecture Assembly Spike

- AArch64 same-length assembly preview
- AArch64 NOP generation
- assembler diagnostics
- branch range checks
- fixture oracle

### P3: Project Revision Application

- apply patch to derived source revision
- transaction log
- undo/redo over active revision
- targeted reanalysis jobs
- no file export yet

### P4: Copy-On-Write Export

- export standalone patched Mach-O
- patch manifest sidecar
- code-signing warnings
- save panel integration

### P5: Debugger Patch Integration

- optional launch of patched derived artifact
- static route mapping to patched revision
- explicit user confirmation
- no process memory patching

## Risk Register

### Risk: Patching Corrupts User Input

Mitigation:

- original sources are immutable
- only derived revisions can be written
- export never overwrites without explicit user-selected output

### Risk: Patch Preview Lies About Address Mapping

Mitigation:

- require exact file-range mapping
- reject ambiguous source mappings
- show every address-space conversion in preview

### Risk: Reanalysis Is Too Expensive

Mitigation:

- conservative targeted invalidation first
- background jobs
- visible stale facts
- cancellation

### Risk: Code Signing Is Misrepresented

Mitigation:

- always warn that code signatures may be invalid after mutation
- do not claim to repair signatures unless Damsel actually implements and
  verifies signing

### Risk: Automation Mutates Unsafely

Mitigation:

- read-only default
- dangerous capability for patch apply/export
- Workbench activity log
- transaction audit

## Completion Criteria

This feature family is implementation-ready when:

- byte windows are modeled as project/source/revision/address-space objects
- patchability is explicit for every byte range
- patch previews can be generated without mutating inputs
- assembler/NOP preview goes through architecture semantics
- invalidation plans identify affected procedures, xrefs, byte types, and
  runtime facts
- Workbench can show patch warnings before application
- source revisions make original inputs immutable
- transaction logs can undo project patch state
- export writes only derived artifacts
- automation cannot patch by accident

The feature family is not product-complete until a user can preview, apply,
undo, reanalyze, and export a same-size AArch64 patch on a standalone Mach-O
without corrupting the original and with all warnings/provenance visible.

