# Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec

Status: implementation-grade product and architecture specification. This is
not an implementation claim.

Purpose: define the subsystem Damsel needs for Hopper-class analyst-authored
knowledge: names, comments, inline comments, tags, colors, bookmarks, manual
references, byte-type overrides, local labels, register aliases, stack
variables, function signatures, switch/case hints, dead-code hints, decompiler
corrections, runtime bindings, and Apple-system-specific evidence notes.

This document exists because "annotation support" is too small a phrase for
what Hopper-like interaction requires. In a serious reverse-engineering tool,
the analyst is continuously correcting analysis. Those corrections must become
durable, undoable, explainable, searchable, automation-safe, and reanalysis
aware.

Primary Damsel audience:

- Rust project-kernel maintainers
- disassembly and procedure-recovery maintainers
- type/signature/decompiler maintainers
- Workbench maintainers
- CLI/API contract maintainers
- automation, MCP, and future plugin maintainers
- validation and fixture maintainers

Primary user audience:

- macOS and iOS reverse engineers
- Apple systems researchers
- XNU, IOKit, DriverKit, kext, and kernelcache analysts
- Objective-C, C++, Objective-C++, Swift-adjacent, and arm64e analysts
- security researchers who need defensible evidence packages
- maintainers comparing Apple OS builds across time

## Companion Documents

Core companions:

- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [Decompiler Quality And User-Assisted Recovery Implementation Spec](decompiler-quality-user-assisted-recovery-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md)
- [API And Data Contracts](api-data-contracts.md)
- [Quality And Validation Plan](quality-validation-plan.md)
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md)

Research companions:

- [Hopper Public Research Notes](hopper-public-research.md)
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)

## Public Hopper Evidence Used

Source URLs:

- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/index.html>
- <https://reverseengineering.stackexchange.com/questions/14973/basic-actions-in-hopper-disassembler>
- <https://pypi.org/project/types-hopper-disassembler/>
- <https://github.com/Fi5t/hopper-disassembler-stubs>
- <https://raw.githubusercontent.com/Fi5t/hopper-disassembler-stubs/master/src/hopper_api-stubs/__init__.pyi>
- <https://github.com/droe/hopper-scripts>
- <https://makigumo.github.io/>

Official Hopper tutorial evidence:

- Hopper transforms bytes into human-readable structure by assigning byte
  categories and running automatic analysis on load.
- The tutorial explicitly lists byte categories such as data, ASCII, code,
  procedure, and undefined.
- Users can manually change byte types by keyboard or toolbar.
- Hopper provides undo and redo for exploratory transformations.
- Hopper's assembly margin exposes bookmarks and breakpoints.
- The CFG view supports comments and labels while viewing graph blocks.
- CFG nodes can be grouped, colored, and have printed text edited.
- The pseudocode view exposes dead-code uncertainty and a user-facing control
  for unreachable-code handling.
- The hex editor is synchronized with assembly and knows the object under the
  cursor.
- Editing bytes can destroy underlying instruction/procedure structure, and
  undo/redo is the safety rail.
- Users can name addresses with a keyboard shortcut and dialog.
- Tags filter symbol lists and can attach to an address, basic block, or whole
  procedure.
- Hopper generates tags during analysis, including Objective-C class/category
  tags.
- The inspector exposes comments, colors, tags, references, procedure
  predecessor/successor information, and switch/case hints.
- The references inspector allows users to add missing references manually.
- The procedure inspector includes a switch/case hint control for indirect
  jumps.
- The assembler and NOP-region operations sit in the same Modify surface as
  other document mutations.

Official Hopper homepage evidence:

- Hopper markets a fast native Cocoa interface with keyboard shortcuts and
  native macOS integration.
- Hopper describes interactive CFG visualization with PDF export.
- Hopper says users can navigate complex code structures and annotate directly
  from the graph view.
- Hopper says it can display assembly, CFG, and pseudocode at the same time.
- Hopper describes Objective-C selector/message extraction and Swift name
  decoding.
- Hopper describes Python scripting and an SDK for extending custom workflows.
- Hopper describes an integrated MCP server for AI-assisted analysis.

Community and public API evidence:

- A Reverse Engineering Stack Exchange answer describes local-variable name and
  type editing through Hopper's procedure inspector in newer versions.
- The same question records a Manage Types workflow and register-name synonym
  behavior in older Hopper versions.
- Public Hopper Python API stubs expose named addresses, comments, inline
  comments, tags, colors, manual references, byte-type setters, local labels,
  register rename overrides, local variables, procedure callees/callers, and
  document save paths.
- Public Hopper script collections include scripts that annotate shellcode,
  annotate stack strings, annotate Yara matches, fix imports by ordinal, copy
  selections, and save bytes, implying scripts can produce analyst-visible
  document knowledge.
- Public plugin indexes show Hopper tool plugins and scripts around references,
  Swift demangling, shared-cache tooling, synchronization, themes, and loader
  extensions.

Inference:

- Hopper's analyst edits are not a cosmetic layer. They affect navigation,
  graph rendering, symbol filtering, decompiler quality, patch safety,
  automation, and later debugger interpretation.
- Damsel needs a single semantic-edit subsystem rather than independent
  one-off implementations for comments, type edits, byte-type edits, manual
  xrefs, AI suggestions, and decompiler hints.
- The Apple-focused niche makes this subsystem more important, not less:
  ObjC dispatch bindings, C++ vtable identifications, IOKit external-method
  annotations, KDK symbol choices, panic-frame notes, PAC/chained-fixup
  decisions, and dyld-cache image relations are all places where a user must be
  able to correct or enrich analysis without losing provenance.

## Current Damsel Evidence

Current strengths:

- Damsel already has standalone Mach-O parsing.
- Damsel already has dyld shared-cache inventory and projected-image analysis.
- Damsel already has ObjC metadata extraction for classes, protocols,
  categories, methods, properties, ivars, selectors, class refs, and method
  IMPs.
- Damsel already has `DisassemblyAnalysis` facts for blocks, edges, imports,
  ObjC references, cache links, recovered values, jump tables, and summary
  metrics.
- Damsel already has a native Workbench path with file opening, analysis
  actions, instruction selection, filtering, diagnostics, and menu skeletons.
- The docs pack already specifies a project kernel, typed byte map, command
  registry, graph/hex sync, type/signature system, decompiler quality loop,
  automation surfaces, and evidence-grounded assistant.

Current gaps:

- No persistent project database is implemented.
- No durable annotation transaction store exists.
- No saved comments, labels, tags, bookmarks, colors, manual references, or
  local-variable edits exist.
- No reanalysis invalidation model protects user edits when facts change.
- No user-vs-imported-vs-inferred precedence model exists in implementation.
- No Workbench inspector can edit semantic facts.
- No CLI/API surface can mutate analyst knowledge in a project.
- No local-variable, register-alias, function-signature, or calling-convention
  edit path exists.
- No switch/case, dead-code, or decompiler-hint transaction path exists.
- No automation or assistant suggestion can be reviewed and applied as a
  typed, undoable edit.
- No evidence package can selectively include analyst annotations with
  redaction.

## Product Thesis

Reverse engineering is a conversation between the analyst and the analyzer.

The analyzer proposes:

- this range is code
- this range is data
- this function starts here
- this reference targets that address
- this selector probably calls these implementations
- this vtable probably belongs to this class
- this stack slot is probably a local variable
- this indirect branch might be a switch
- this decompiler line is probably equivalent to these instructions

The analyst responds:

- this is not code
- this is a table
- this function begins earlier
- this call target is wrong
- this selector call binds to this method in this context
- this class is an IOKit user client
- this local variable is a structure pointer
- this indirect jump is a switch with these cases
- this decompiler output is missing dead code

The product wins when that response becomes durable knowledge, not a note lost
in one view.

Damsel should make user-authored knowledge feel:

- fast enough for interactive use
- persistent across reopen
- visible in every relevant representation
- undoable and redoable
- searchable and filterable
- distinguishable from inferred/imported facts
- inspectable through provenance
- safe under reanalysis
- usable by CLI, automation, MCP, and future plugins
- exportable with redaction controls
- especially strong for Apple runtime and XNU-style workflows

## Non Goals

This spec does not implement:

- collaborative multi-user editing
- cloud sync
- hosted project storage
- patching bytes in place
- live debugger memory writes
- automatic AI mutation without user review
- silently overriding analyzer facts with low-confidence suggestions
- exact source-code recovery
- a global shared knowledge base for proprietary binaries

This spec does define the semantic edit layer that those features would need
if they are ever implemented safely.

## Design Principles

### User Edits Are Facts With Provenance

A user edit is not a UI preference. It is a project fact with:

- stable id
- target route
- edit kind
- old value where applicable
- new value
- actor
- timestamp
- source surface
- confidence or certainty
- invalidation behavior
- undo group
- redaction policy
- downstream analysis impact

### Analyzer Facts Never Pretend To Be User Edits

Analyzer output must remain distinct from user-authored knowledge.

Examples:

- an imported dSYM function name is an imported name fact
- a demangled Swift name is a derived name fact
- an ObjC method name from metadata is a runtime fact
- a user rename is a user edit
- an assistant-suggested rename is a suggestion until the user applies it
- a script-created annotation is an automation edit with script provenance

### User Edits Win Display Precedence But Not Truth

If a user renames a function, the UI should display the user name first. The
project should still preserve:

- original symbol
- imported dSYM name
- demangled name
- ObjC runtime name
- C++ ABI name
- prior user name
- conflict record

Display precedence is not data deletion.

### Reanalysis Is A Consumer Of Edits

Some edits only affect display. Others change analysis.

Examples:

- comment: display/search update only
- tag: display/search update only
- manual reference: xref graph update, downstream function/navigation update
- byte type override: byte-map update, possible procedure invalidation
- function boundary edit: procedure recovery and graph invalidation
- switch/case hint: control-flow and decompiler reanalysis
- local variable type: decompiler and ABI overlay reanalysis
- ObjC dispatch binding: message-send browser and call graph update

The edit transaction should record which reanalysis jobs were scheduled and
why.

### Every Edit Is Route-Based

Edits should not be keyed only by raw address. Apple artifacts have multiple
address spaces:

- file offset
- image VM address
- dyld shared-cache VM address
- projected image address
- kernel collection address
- fileset member address
- runtime slid address
- source-line route
- decompiler token route
- graph node route
- ObjC selector/class/method route
- C++ class/vtable/virtual-call route
- panic-frame route

The target must be a canonical route plus enough address data for debugging.

### Suggestions Are Not Edits

Automation and AI can propose:

- rename
- type assignment
- comment
- dispatch binding
- switch cases
- dead-code inclusion
- manual reference
- vtable ownership
- IOKit external-method classification
- panic-frame note

But proposals become edits only after:

- validation
- user review or configured trusted automation grant
- capability check
- transaction creation
- undo grouping
- provenance recording

## Knowledge Object Taxonomy

### Address-Level Display Edits

Kinds:

- prefix comment
- inline comment
- address name
- global label
- bookmark
- tag
- color
- operand format
- operand display base
- operand symbolic target
- note visibility
- evidence severity marker

Targets:

- instruction route
- data item route
- byte-range route
- feature-map range
- source-line route
- decompiler token route

Required behavior:

- visible in assembly
- visible in inspector
- indexed in search
- included in route preview
- undoable
- exportable by policy

### Byte And Data Classification Edits

Kinds:

- mark undefined
- mark code
- mark procedure
- mark data byte
- mark data short
- mark data int
- mark data long
- mark ASCII string
- mark Unicode string
- mark alignment
- mark structure instance
- mark pointer
- mark C string pointer
- mark selector reference
- mark class reference
- mark CFString candidate
- mark vtable pointer
- mark PAC-signed pointer
- mark chained-fixup pointer

Targets:

- byte range
- typed-byte object
- section range
- data item
- pointer location

Required behavior:

- update byte map
- repair overlapping objects
- mark superseded analyzer facts
- schedule local reanalysis where needed
- preserve original raw bytes
- preserve original analyzer classification
- show impact before destructive classification changes

### Procedure And Control-Flow Edits

Kinds:

- create procedure
- remove procedure
- rename procedure
- adjust function start
- adjust function end
- split function
- merge functions
- add basic-block seed
- remove basic-block seed
- mark tailcall
- clear tailcall
- add manual branch target
- remove manual branch target
- switch/case hint
- dead-code include hint
- dead-code suppress hint
- exception edge hint
- noreturn override
- callsite target override
- calling convention override

Targets:

- function route
- basic-block route
- instruction route
- control-flow edge route
- callsite route
- indirect branch route

Required behavior:

- create reanalysis request
- show stale procedure state while work is pending
- link edit to resulting function version
- record impact on xrefs, graph, IR, decompiler cache, and exports
- prevent impossible edits from silently corrupting the function database

### Reference And Relationship Edits

Kinds:

- add manual code xref
- remove manual code xref
- add manual data xref
- remove manual data xref
- add call edge
- remove call edge
- bind ObjC message-send target
- unbind ObjC message-send target
- bind selector to method implementation
- bind C++ virtual call to vtable slot
- bind vtable to class
- bind IOKit external method to selector/index
- bind MIG message id to handler
- bind syscall/trap number to handler
- bind panic frame to function
- bind runtime address to static route

Targets:

- instruction route
- data route
- xref route
- ObjC message route
- selector route
- C++ virtual-call route
- IOKit method route
- XNU interface route
- runtime frame route

Required behavior:

- distinguish analyzer, imported, script, assistant, and user relationship facts
- support confidence and evidence references
- allow conflict browsing
- update graph/search/navigation indexes
- preserve manual relationships when symbol profiles change

### Type And Name Edits

Kinds:

- rename function
- rename local label
- rename register alias
- rename stack variable
- rename argument
- rename type
- create type
- edit structure field
- edit enum case
- set function signature
- set argument type
- set return type
- set calling convention
- set local variable type
- set register variable type
- assign ObjC method type encoding override
- assign C++ method signature candidate
- assign Swift-adjacent demangle display override
- assign kernel type source override

Targets:

- function route
- variable route
- argument route
- register role route
- type record route
- field route
- enum case route
- ObjC method route
- C++ method route
- kernel symbol route

Required behavior:

- visible in procedure inspector
- visible in decompiler and assembly
- conflict-aware with DWARF/dSYM/KDK facts
- recoverable after reanalysis
- scoped correctly for local labels and variables
- searchable by user name and original name

### Workbench Presentation Edits

Kinds:

- graph node title override
- graph node body text override
- graph node color
- graph node group
- graph group title
- graph group collapse state
- function view preferred representation
- feature-map lane visibility
- inspector section pin
- evidence drawer pin

Targets:

- graph node route
- graph group route
- function route
- route group
- project view state

Required behavior:

- presentation edits must not masquerade as analysis facts
- graph layout cache must include presentation version
- view state should be separable from semantic project facts
- export can include presentation when the user asks

### Decompiler Recovery Hints

Kinds:

- include dead code
- suppress dead code
- switch/case case list
- switch base expression
- force loop structure
- break structuring region
- mark variable as signed
- mark variable as unsigned
- mark boolean
- mark enum
- force pointer type
- define array extent
- define structure field access
- merge variables
- split variable live ranges
- mark function noreturn
- mark call pure
- mark call impure
- mark call target
- force tailcall
- suppress misleading pseudocode line

Targets:

- function route
- IR block route
- instruction route
- decompiler line route
- decompiler token route
- variable route
- callsite route
- switch route

Required behavior:

- create decompiler run invalidation
- cite source instructions affected
- show before/after quality delta
- allow revert without losing unrelated comments
- feed the quality ledger and regression corpus

### Apple Runtime Edits

Kinds:

- ObjC class role note
- ObjC selector semantic tag
- ObjC message binding
- ObjC category ownership correction
- ObjC method family override
- block invoke binding
- Swift demangle display override
- Swift metadata relationship note
- C++ class ownership note
- vtable ownership binding
- virtual-call target binding
- IOKit class role tag
- IOKit external-method index binding
- user-client security note
- MIG subsystem binding
- syscall/trap handler binding
- kext ownership correction
- KDK profile trust note
- panic-frame interpretation note
- PAC/chained-fixup interpretation note

Targets:

- ObjC class route
- ObjC selector route
- ObjC method route
- block route
- Swift symbol route
- C++ class route
- vtable route
- virtual-call route
- IOKit route
- XNU interface route
- kernel member route
- panic frame route
- pointer/fixup route

Required behavior:

- appear in Apple runtime browsers
- update relationship graphs
- feed assistant context with provenance
- remain stable across OS-build diffing
- support evidence-package export with redaction

## Storage Architecture

### Tables

The project database should treat semantic edits as first-class records.

Suggested tables:

- `semantic_edits`
- `semantic_edit_targets`
- `semantic_edit_values`
- `semantic_edit_groups`
- `semantic_edit_conflicts`
- `semantic_edit_impacts`
- `semantic_edit_reanalysis_requests`
- `semantic_edit_suggestions`
- `semantic_edit_reviews`
- `semantic_edit_redactions`
- `semantic_edit_search_terms`
- `semantic_edit_export_receipts`

### `semantic_edits`

Columns:

- `edit_id`
- `project_id`
- `kind`
- `state`
- `actor_kind`
- `actor_id`
- `source_surface`
- `created_at`
- `updated_at`
- `applied_at`
- `undone_at`
- `redo_of_edit_id`
- `supersedes_edit_id`
- `group_id`
- `schema_version`
- `confidence`
- `certainty`
- `review_state`
- `capability_grant_id`
- `redaction_class`
- `summary`

States:

- `suggested`
- `pending_review`
- `applied`
- `superseded`
- `undone`
- `rejected`
- `failed`
- `stale_target`

Actor kinds:

- `user`
- `script`
- `mcp_tool`
- `assistant`
- `plugin`
- `importer`
- `migration`
- `test_fixture`

Source surfaces:

- `workbench_inspector`
- `workbench_graph`
- `workbench_assembly`
- `workbench_hex`
- `workbench_decompiler`
- `workbench_runtime_browser`
- `command_palette`
- `context_menu`
- `cli`
- `api`
- `mcp`
- `script`
- `plugin`
- `import_session`

### `semantic_edit_targets`

Columns:

- `target_id`
- `edit_id`
- `route_id`
- `route_kind`
- `address_space_id`
- `address_value`
- `source_id`
- `function_id`
- `object_id`
- `range_start`
- `range_end`
- `target_version`
- `route_repair_policy`
- `stale_behavior`

Route repair policies:

- `exact_required`
- `nearest_instruction`
- `same_function`
- `same_object_fingerprint`
- `same_symbol_name`
- `same_runtime_object`
- `manual_review_required`

Stale behavior:

- `hide`
- `show_stale`
- `reattach_if_exact`
- `reattach_if_fingerprint`
- `requires_review`

### `semantic_edit_values`

Columns:

- `value_id`
- `edit_id`
- `field`
- `old_value_json`
- `new_value_json`
- `display_old`
- `display_new`
- `value_type`
- `value_hash`

Value types:

- `text`
- `name`
- `tag`
- `color`
- `address`
- `address_range`
- `type_ref`
- `signature`
- `variable_ref`
- `enum_case`
- `xref`
- `relationship`
- `analysis_hint`
- `presentation`
- `redacted`

### `semantic_edit_groups`

Purpose: group many small mutations into one user-visible undo entry.

Examples:

- rename function plus update local callsite labels
- apply KDK profile trust note plus attach imported symbol profile
- import Yara annotations
- add switch/case hint plus cases
- apply assistant-reviewed decompiler recovery suggestion
- bulk tag selected ObjC methods

Columns:

- `group_id`
- `project_id`
- `title`
- `actor_kind`
- `source_surface`
- `created_at`
- `state`
- `undo_label`
- `redo_label`
- `edit_count`
- `requires_reanalysis`
- `redaction_class`

### `semantic_edit_impacts`

Purpose: describe what changed after an edit.

Impact kinds:

- `display_only`
- `search_index_update`
- `xref_index_update`
- `byte_map_update`
- `procedure_version_update`
- `graph_cache_invalidation`
- `decompiler_cache_invalidation`
- `runtime_graph_update`
- `debugger_mapping_update`
- `export_snapshot_update`
- `assistant_context_update`
- `problem_created`
- `problem_resolved`

Columns:

- `impact_id`
- `edit_id`
- `impact_kind`
- `affected_route_id`
- `affected_object_id`
- `job_id`
- `problem_id`
- `summary`

### `semantic_edit_suggestions`

Suggestions are separate from edits.

Columns:

- `suggestion_id`
- `project_id`
- `proposed_edit_kind`
- `proposed_target_json`
- `proposed_value_json`
- `evidence_json`
- `actor_kind`
- `actor_id`
- `confidence`
- `risk_level`
- `review_state`
- `created_at`
- `expires_at`
- `source_surface`
- `privacy_class`

Review states:

- `new`
- `viewed`
- `accepted`
- `accepted_modified`
- `rejected`
- `expired`
- `superseded`

## API Contracts

### List Semantic Edits

```http
GET /api/projects/{project_id}/semantic-edits?route_id=...&kind=...&state=...
```

Response shape:

```json
{
  "items": [
    {
      "edit_id": "edit:42",
      "kind": "set_comment",
      "state": "applied",
      "target": {
        "route_id": "instr:main:100003f20",
        "display": "0x100003f20"
      },
      "summary": "Set inline comment",
      "display_value": "validates entitlement before opening user client",
      "actor": {
        "kind": "user",
        "display": "local user"
      },
      "created_at": "2026-05-30T00:00:00Z",
      "impacts": ["search_index_update"]
    }
  ],
  "next_cursor": null
}
```

### Preview Semantic Edit

```http
POST /api/projects/{project_id}/semantic-edits/preview
```

Request:

```json
{
  "kind": "set_byte_type",
  "target": {
    "route_id": "bytes:main:100004000-100004020"
  },
  "value": {
    "byte_type": "ascii_string"
  }
}
```

Response:

```json
{
  "allowed": true,
  "impact": {
    "summary": "Reclassifies 32 bytes as ASCII string.",
    "invalidates": ["typed_byte_range", "function_overlap_check"],
    "requires_reanalysis": false,
    "warnings": []
  },
  "undo_label": "Set ASCII String"
}
```

Preview must be cheap for simple edits and bounded for complex edits.

### Apply Semantic Edit

```http
POST /api/projects/{project_id}/semantic-edits
```

Request:

```json
{
  "kind": "add_manual_reference",
  "target": {
    "route_id": "instr:main:100003f20"
  },
  "value": {
    "referenced_route_id": "data:main:100008000",
    "reference_kind": "data_read"
  },
  "client_request_id": "workbench-uuid",
  "source_surface": "workbench_inspector"
}
```

Response:

```json
{
  "edit_id": "edit:manual-ref:17",
  "group_id": "edit-group:17",
  "state": "applied",
  "undo_label": "Add Manual Reference",
  "impacts": [
    {
      "impact_kind": "xref_index_update",
      "affected_route_id": "instr:main:100003f20"
    }
  ],
  "queued_jobs": []
}
```

### Apply Edit Group

```http
POST /api/projects/{project_id}/semantic-edit-groups
```

Use for:

- bulk tag selected routes
- import script annotations
- apply assistant-reviewed suggestions
- add switch cases
- import local-variable names from debug info as reviewed user aliases
- mark a C++ class and multiple vtables

Rules:

- all edits in the group commit atomically
- partial failure returns a typed problem
- large groups may stream progress
- undo treats the group as one user-visible operation

### Undo And Redo

```http
POST /api/projects/{project_id}/transactions/undo
POST /api/projects/{project_id}/transactions/redo
```

Undo response:

```json
{
  "transaction_id": "txn:undo:19",
  "undone_group_id": "edit-group:17",
  "label": "Undo Add Manual Reference",
  "impacts": ["xref_index_update"],
  "queued_jobs": []
}
```

Undo must restore:

- semantic edit state
- derived display index
- xref graph where applicable
- stale cache markers
- associated problems
- navigation route previews

Undo must not:

- delete raw bytes
- delete imported symbol records
- erase analysis facts that were not produced by the edit
- assume current analysis version equals the original analysis version

### Suggest Semantic Edit

```http
POST /api/projects/{project_id}/semantic-edit-suggestions
```

Used by:

- assistant
- MCP tool
- script
- plugin
- decompiler quality pass
- ObjC/C++/XNU heuristics

Response:

```json
{
  "suggestion_id": "suggestion:rename:92",
  "review_state": "new",
  "risk_level": "medium",
  "summary": "Rename function to validateEntitlementBeforeOpen",
  "evidence": [
    {
      "route_id": "str:main:100020000",
      "summary": "String reference mentions entitlement validation."
    },
    {
      "route_id": "objc-selector:clientHasEntitlement:",
      "summary": "Nearby selector call."
    }
  ]
}
```

### Accept Suggestion

```http
POST /api/projects/{project_id}/semantic-edit-suggestions/{suggestion_id}/accept
```

Rules:

- user may edit proposed value before applying
- response is a normal semantic edit response
- provenance links accepted edit back to suggestion id
- rejected suggestions remain in evidence log if configured

## CLI Contracts

Core commands:

```sh
damsel project edits list --project file.damsel --route ...
damsel project edits preview --project file.damsel --kind set-comment --route ... --value ...
damsel project edits apply --project file.damsel --kind set-comment --route ... --value ...
damsel project edits undo --project file.damsel
damsel project edits redo --project file.damsel
damsel project edits suggestions list --project file.damsel
damsel project edits suggestions accept --project file.damsel suggestion:...
```

Specialized aliases:

```sh
damsel project comment set --project file.damsel --route ... --text ...
damsel project label set --project file.damsel --route ... --name ...
damsel project tag add --project file.damsel --route ... --tag ...
damsel project color set --project file.damsel --route ... --color "#RRGGBB"
damsel project xref add --project file.damsel --from ... --to ... --kind data-read
damsel project byte-type set --project file.damsel --range ... --type ascii-string
damsel project variable rename --project file.damsel --variable ... --name ...
damsel project variable type --project file.damsel --variable ... --type ...
damsel project switch hint --project file.damsel --instruction ... --cases cases.json
damsel project objc bind-message --project file.damsel --message ... --method ...
damsel project cxx bind-vcall --project file.damsel --callsite ... --slot ...
damsel project xnu bind-mig --project file.damsel --message-id ... --handler ...
```

CLI output requirements:

- JSON by default for machine-readable subcommands
- explicit `--human` for text output
- typed problem codes
- no raw bytes in diagnostics unless explicitly requested
- no comments in diagnostics bundles unless policy includes annotations

## Workbench UX

### Inspector Edit Deck

The Workbench inspector should expose edit decks based on route kind.

Instruction route:

- name/label field
- prefix comment
- inline comment
- tags
- color
- bookmark
- operand format
- references from
- references to
- add manual reference
- byte type
- create procedure
- set call target
- switch/case hint if applicable

Function route:

- display name
- signature
- calling convention
- tags
- color
- comment
- local labels
- register aliases
- stack variables
- arguments
- return type
- noreturn override
- decompiler hints
- user problem suppressions

Graph node route:

- title
- node comment
- color
- group membership
- block tags
- predecessor/successor summary
- local edge overrides

ObjC message route:

- selector
- receiver evidence
- candidate methods
- selected binding
- confidence
- user note
- conflict status

C++ virtual-call route:

- vtable evidence
- slot index
- candidate methods
- selected binding
- class ownership
- user note

Kernel/panic route:

- symbolication source
- KDK profile state
- frame note
- unresolved reason
- manual binding
- export inclusion policy

### Edit Preview Sheet

Complex edits should show preview before commit.

Preview fields:

- edit title
- target summary
- current value
- proposed value
- facts superseded
- objects invalidated
- reanalysis jobs queued
- warnings
- privacy/export effect
- undo label

Required for:

- byte-type range changes
- procedure split/merge
- function boundary edits
- switch/case hints
- type/signature edits
- bulk tag operations
- assistant suggestions
- automation-generated edit groups
- Apple runtime relationship bindings

### Suggestion Tray

The suggestion tray should group candidate edits by source:

- decompiler quality pass
- ObjC dispatch analyzer
- C++ vtable analyzer
- XNU/MIG recognizer
- Yara/script run
- assistant session
- MCP tool
- plugin

Each suggestion shows:

- proposed edit
- confidence
- target route
- evidence routes
- risk label
- accept
- accept with edits
- reject
- explain
- open evidence

### Conflict Browser

Conflicts happen when:

- user name differs from dSYM name
- user type differs from DWARF type
- user function boundary conflicts with analyzer range
- two scripts propose different labels
- ObjC metadata and user binding disagree
- C++ RTTI and manual vtable ownership disagree
- KDK profile changes imported symbol names
- reanalysis makes target route stale

Conflict browser rows:

- target
- current display value
- user value
- imported value
- inferred value
- suggestion value
- confidence
- last changed
- action

Actions:

- keep user value
- switch to imported value
- create alias
- hide imported value
- mark analyzer wrong
- schedule reanalysis
- export conflict evidence

### Stale Edit Display

When a route changes under an edit, the UI should not silently drop it.

States:

- attached
- stale but visible
- repair candidate found
- needs review
- target missing
- superseded

UI:

- stale badge near comment/name/tag
- one-click route repair if exact fingerprint match exists
- compare old and new target context
- keep as historical note if no repair is possible

## Reanalysis Model

### Edit Impact Classifier

Every edit kind declares an impact class:

- `display_only`
- `index_only`
- `local_analysis`
- `function_analysis`
- `interprocedural_analysis`
- `runtime_graph_analysis`
- `decompiler_analysis`
- `artifact_wide_analysis`

Examples:

- set comment: `index_only`
- apply tag: `index_only`
- add manual xref: `interprocedural_analysis`
- set byte type: `local_analysis`
- mark procedure: `function_analysis`
- switch/case hint: `function_analysis` and `decompiler_analysis`
- set local variable type: `decompiler_analysis`
- bind ObjC message: `runtime_graph_analysis` and `decompiler_analysis`
- bind panic frame: `runtime_graph_analysis`

### Reanalysis Request

```json
{
  "request_id": "reanalyze:101",
  "reason": "switch_hint_applied",
  "source_edit_id": "edit:77",
  "scope": {
    "kind": "function",
    "route_id": "function:main:100003e00"
  },
  "invalidates": [
    "cfg",
    "procedure_summary",
    "decompiler_cache"
  ],
  "priority": "interactive",
  "created_at": "2026-05-30T00:00:00Z"
}
```

### Versioned Outputs

Derived analysis records should include:

- `analysis_version`
- `input_edit_version`
- `source_fact_versions`
- `stale_after_edit_id`
- `recomputed_by_job_id`

The UI can then answer:

- is this graph current?
- is this pseudocode stale?
- was this xref created by analysis or user edit?
- did this decompiler output include the user's switch hint?
- did this function summary include the user's local variable type?

### Reanalysis Scheduling

Priority classes:

- `inline`: complete before response for tiny index changes
- `interactive`: queue immediately and stream progress
- `background`: enqueue when idle
- `manual`: user must explicitly request

Rules:

- simple comments should not block on reanalysis
- switch hints should make the current function visibly stale until recomputed
- bulk script annotations should batch index updates
- decompiler hints should invalidate only affected functions where possible
- shared-cache-wide relation edits should update indexes incrementally
- failed reanalysis should leave the edit applied and create a problem

## Apple-System Workflows

### ObjC Dispatch Correction

Scenario:

1. User opens a stripped iOS framework from a dyld shared cache.
2. Damsel shows an `objc_msgSend` call with several candidate methods.
3. The analyst recognizes the receiver from nearby ivar/class evidence.
4. The analyst binds the message send to a method implementation.
5. Damsel updates call graph, selector browser, decompiler overlay, and
   evidence package.

Needed edits:

- `bind_objc_message`
- `set_comment`
- `apply_tag`

Needed validation:

- target method must be in same project
- binding stores candidate list and selected method
- undo restores ambiguous state
- decompiler overlay marks binding as user-authored

### C++ Vtable Ownership Correction

Scenario:

1. User inspects an IOKit binary.
2. Damsel recognizes a possible vtable but cannot prove owner class.
3. User assigns vtable to a class and names a virtual method slot.
4. C++ browser and relationship graph update.

Needed edits:

- `bind_vtable_owner`
- `rename_virtual_method`
- `set_slot_comment`

Needed validation:

- class and vtable address spaces match
- slot offset is in range
- RTTI conflict creates warning, not failure
- undo reverts graph relationship and names

### XNU MIG Handler Binding

Scenario:

1. User searches for a MIG subsystem message id.
2. Damsel finds dispatch table candidates but cannot prove exact handler.
3. User binds message id to handler with a confidence note.
4. XNU interface browser and panic workflows use the binding.

Needed edits:

- `bind_mig_message_handler`
- `set_interface_note`
- `apply_tag`

Needed validation:

- message id is numeric and scoped to subsystem
- handler route belongs to kernel/kext source
- exported evidence includes user-authored marker

### KDK Symbol Trust Note

Scenario:

1. User attaches a KDK to a kernel collection.
2. Damsel reports UUID mismatch but high heuristic similarity.
3. User marks profile as useful for names but not source lines.
4. UI displays imported names with warning badges.

Needed edits:

- `set_symbolication_trust_policy`
- `set_kernel_profile_note`

Needed validation:

- never converts mismatched KDK into exact match
- user note survives profile detach/reattach
- evidence export includes mismatch explanation

### Panic Frame Manual Binding

Scenario:

1. User imports a panic log.
2. One frame cannot be symbolicated because of slide/source mismatch.
3. User binds the frame to a candidate function based on nearby offsets.
4. Panic browser shows manual binding with confidence and evidence.

Needed edits:

- `bind_panic_frame`
- `set_frame_comment`

Needed validation:

- binding uses panic route, not raw address only
- original unresolved reason remains visible
- undo restores unresolved frame

## Automation And AI Integration

### Capability Classes

Read-only:

- list edits
- list suggestions
- inspect conflicts
- inspect stale edits

Low-risk mutation:

- add comment
- add tag
- add bookmark
- set color

Medium-risk mutation:

- rename symbol
- add manual xref
- bind ObjC message
- set local variable name

High-risk mutation:

- byte-type override
- function boundary edit
- switch/case hint
- type/signature edit
- C++ vtable binding
- XNU handler binding

Restricted:

- patch bytes
- debugger control
- remote target control
- exporting comments/private paths

### Assistant Suggestions

Assistant suggestions should always include:

- target route
- proposed edit
- evidence routes
- confidence
- risk
- alternative explanation
- what would be invalidated
- why the assistant is not applying it automatically

Forbidden assistant behavior:

- inventing routes
- overwriting user edits without review
- hiding low confidence
- presenting speculative names as recovered source names
- exporting comments in diagnostics without user consent

### Script Import

Script-created edits should use edit groups.

Examples:

- annotate stack strings
- annotate Yara matches
- label known shellcode idioms
- fix imports by ordinal
- tag suspicious selectors
- mark IOKit external method candidates

Script metadata:

- script id
- script version
- input selection
- capability grant
- start/end time
- edit count
- failed targets
- redaction policy

## Data Validation

### Name Validation

Names should:

- have stable normalized form
- preserve display form
- reject empty strings
- reject control characters
- reject path-like accidental names where not allowed
- allow Objective-C selector punctuation where route kind permits
- allow C++ demangled display names as aliases, not raw ids
- keep original symbol in alternate-name set

### Type Validation

Type edits should:

- parse before commit
- preserve unknown type text if parser fails only as a comment, not a type
- record type source
- detect recursive structs
- detect invalid field offsets
- detect enum base type mismatch
- validate pointer size against address-space architecture
- mark Swift-adjacent types as display facts unless richer metadata exists

### Address Validation

Address-targeted edits should:

- resolve route before commit
- store address-space identity
- reject unmapped address unless edit kind explicitly supports external notes
- detect range overlaps
- detect file-vs-VM confusion
- show cache image/member identity
- show runtime slide if target comes from a debug session

### Reference Validation

Manual references should:

- resolve both endpoints
- store direction
- store reference kind
- detect duplicates
- detect impossible self-reference if kind forbids it
- allow uncertain references with confidence
- update reverse indexes

## Privacy And Redaction

Potentially sensitive fields:

- user comments
- private path names
- app bundle names
- local source paths
- panic logs
- device names
- assistant prompts
- script output
- evidence notes
- manually entered type names
- target host labels

Default diagnostics:

- include edit counts by kind
- include schema versions
- include redacted problem codes
- exclude raw comment text
- exclude private paths
- exclude assistant prompts
- exclude raw bytes

Evidence package options:

- include no user annotations
- include selected annotations
- include all annotations
- include comments but redact paths
- include comments and paths for private package

## Performance Budgets

Interactive budgets:

- apply simple comment: under 100 ms backend time
- apply simple tag: under 100 ms backend time
- rename symbol: under 150 ms backend time
- add manual xref: under 250 ms backend time excluding queued jobs
- preview byte-type edit: under 250 ms for small ranges
- accept suggestion: under 250 ms excluding queued jobs
- undo simple edit: under 100 ms backend time
- route repair query: under 200 ms for current function

Large-project budgets:

- list edits for current route: under 100 ms
- list all edits paged: first page under 250 ms
- bulk import 1000 comments: streaming progress within 500 ms
- search user annotations: first page under 300 ms
- conflict browser first page: under 300 ms
- stale edit scan after reanalysis: incremental, not full-project blocking

Implementation rules:

- never decode all comments into Swift for a shared cache
- never recompute whole-project search index for one comment
- never block menu validation on edit history queries
- batch script edit groups
- compact old edit values after migration only when undo policy allows it

## Diagnostics

New problem codes:

- `semantic_edit.target_not_found`
- `semantic_edit.target_stale`
- `semantic_edit.route_repair_required`
- `semantic_edit.invalid_name`
- `semantic_edit.invalid_type`
- `semantic_edit.invalid_range`
- `semantic_edit.overlap_conflict`
- `semantic_edit.duplicate_reference`
- `semantic_edit.capability_denied`
- `semantic_edit.reanalysis_failed`
- `semantic_edit.undo_conflict`
- `semantic_edit.redaction_required`
- `semantic_edit.suggestion_evidence_missing`
- `semantic_edit.import_partial_failure`

Problem fields:

- problem id
- edit id
- route id
- source surface
- severity
- user-visible summary
- technical reason
- suggested action
- redaction class

## Implementation Slices

### SE1: Edit Schema And Core Store

Deliverables:

- `semantic_edits` schema
- `semantic_edit_targets` schema
- `semantic_edit_values` schema
- edit kind enum
- edit state enum
- actor/source-surface enums
- route-target validation
- snapshot fixtures

Acceptance:

- project can persist and reload a comment edit
- edit ids remain stable after reopen
- raw analyzer facts remain separate from user edits

### SE2: Comment, Label, Tag, Bookmark, Color

Deliverables:

- set/clear comment
- set/clear inline comment
- set/clear label
- add/remove tag
- set/remove color
- set/clear bookmark
- search index update
- Workbench inspector fields
- CLI commands

Acceptance:

- edits render in assembly route payloads
- search finds user labels/comments/tags
- undo/redo works after reopen
- diagnostics redact comment text by default

### SE3: Manual References

Deliverables:

- add/remove manual reference
- reference kind enum
- endpoint validation
- xref index integration
- reverse-reference inspector integration
- graph update impact records

Acceptance:

- manual xref appears in both directions
- duplicate xrefs are rejected or merged deterministically
- undo removes xref without deleting analyzer xrefs
- evidence package marks manual reference provenance

### SE4: Byte-Type And Procedure Hints

Deliverables:

- byte-type override transactions
- data classification edits
- mark code/procedure/undefined
- function start/end hints
- procedure split/merge preview
- local reanalysis request
- stale procedure display

Acceptance:

- byte map records user override separately from analyzer classification
- conflicting analyzer facts become stale/superseded, not deleted
- Workbench feature map updates
- undo restores previous byte classification

### SE5: Type, Signature, Variable, And Register Edits

Deliverables:

- local label rename
- register alias rename
- stack variable rename
- variable type assignment
- function signature edit
- calling-convention override
- type record edits
- conflict records against debug-info imports

Acceptance:

- procedure inspector shows user edits
- decompiler cache invalidates when needed
- user edits survive dSYM/KDK profile switch
- CLI snapshots cover variable rename/type edit

### SE6: Switch, Dead-Code, And Decompiler Hints

Deliverables:

- switch/case hint record
- dead-code include/suppress hint
- decompiler recovery hint records
- source-map target validation
- decompiler run invalidation
- quality-ledger integration

Acceptance:

- accepted switch hint queues function/decompiler reanalysis
- stale pseudocode badge appears until recomputed
- before/after quality report cites hint id
- undo reverts hint and invalidates derived output

### SE7: Apple Runtime Relationship Edits

Deliverables:

- ObjC message binding
- selector/method semantic notes
- C++ vtable ownership binding
- virtual-call target binding
- IOKit external-method binding
- MIG/syscall/trap handler binding
- panic-frame binding
- KDK trust note

Acceptance:

- runtime browsers show manual bindings
- relationship graph updates incrementally
- conflicts with metadata/imported facts are visible
- evidence export distinguishes manual Apple-system facts

### SE8: Suggestions And Review

Deliverables:

- suggestion store
- suggestion evidence records
- review states
- accept/accept-modified/reject API
- Workbench suggestion tray
- assistant and script integration

Acceptance:

- assistant can propose but not silently apply high-risk edits
- accepting suggestion creates normal edit transaction
- rejected suggestion remains auditable when configured
- suggestions with missing evidence are rejected

### SE9: Conflict And Stale-Route Browser

Deliverables:

- conflict detection
- stale target detection
- route repair candidates
- Workbench conflict browser
- Workbench stale-edit browser
- CLI conflict listing

Acceptance:

- profile switch creates visible conflicts instead of overwriting user names
- reanalysis stale target can be repaired
- unresolved stale edit remains visible as historical note

### SE10: Bulk Import, Export, Redaction, And Performance

Deliverables:

- bulk edit groups
- streaming progress
- diagnostics redaction
- evidence package inclusion policies
- metrics
- performance fixtures

Acceptance:

- bulk annotation import does not freeze Workbench
- diagnostics exclude comments by default
- evidence export can include selected annotations
- performance budget snapshots exist

## Test Plan

Fixture classes:

- standalone Mach-O with comments/labels/tags
- fixture with manual data/code xrefs
- fixture with conflicting byte-type overrides
- fixture with function boundary edits
- fixture with local variables and register aliases
- fixture with switch/case hint
- fixture with decompiler dead-code hint
- fixture with ObjC message binding
- fixture with C++ vtable binding
- fixture with IOKit external-method binding
- fixture with kernel panic-frame binding
- fixture with KDK mismatch trust note
- fixture with assistant suggestions
- fixture with script bulk annotations
- fixture with stale route after reanalysis

Required tests:

- schema migration
- create/edit/delete comment
- rename label
- add/remove tag
- set/remove color
- add/remove bookmark
- add/remove manual xref
- byte-type override conflict
- function hint reanalysis request
- switch hint reanalysis request
- variable rename and type assignment
- function signature edit
- user-vs-imported name precedence
- dSYM/KDK profile switch preserving user edits
- ObjC binding undo
- C++ binding undo
- panic frame binding undo
- suggestion accept/reject
- capability denial
- diagnostics redaction
- evidence export inclusion
- bulk import progress
- search index update
- route repair
- stale target display
- undo/redo after reopen

## Acceptance Criteria

Documentation acceptance for this spec:

- public Hopper evidence is mapped to concrete Damsel edit kinds
- user edits, suggestions, analyzer facts, imported facts, and script facts are
  separated
- storage model is explicit enough for implementation planning
- Workbench, CLI, API, automation, and assistant surfaces share one model
- reanalysis and invalidation are not hand-waved
- Apple runtime and XNU-specific semantic edits are included
- privacy/redaction rules are specified
- implementation slices are independently commit-sized

Future implementation acceptance:

- comments, labels, tags, bookmarks, colors, and manual refs persist
- user edits are undoable and redoable after project reopen
- user edits survive reanalysis unless explicitly superseded
- analyzer/imported/user facts remain distinguishable
- Workbench can edit semantic facts through route-aware inspector controls
- CLI/API can apply and list edits with typed problem codes
- decompiler and procedure hints queue explicit reanalysis
- ObjC/C++/XNU bindings update runtime browsers and relationship graphs
- assistant/script suggestions require review or explicit capability grants
- diagnostics redact user annotations by default
- performance budgets are measured on representative fixtures

## Risk Register

### Risk: Edits Become UI-Only State

Mitigation:

- all semantic edits go through Rust project API
- Swift may cache presentation, but Rust is authoritative
- tests reopen project and verify edits persist

### Risk: User Edits Overwrite Imported Truth

Mitigation:

- display precedence is separate from fact storage
- imported/debug facts remain queryable
- conflict browser shows competing facts

### Risk: Reanalysis Deletes User Work

Mitigation:

- edits attach to routes with repair policies
- stale edit states are visible
- reanalysis supersedes derived facts, not user edits

### Risk: Assistant Hallucinates Edits

Mitigation:

- suggestion evidence must cite existing routes/facts
- high-risk suggestions require user review
- invented route ids fail validation
- assistant outputs are suggestions, not edits

### Risk: Bulk Scripts Pollute Projects

Mitigation:

- script edits carry script id/version
- bulk edit groups can be undone
- capability grants limit mutation classes
- import summaries show counts and failures

### Risk: Apple Runtime Bindings Overclaim Certainty

Mitigation:

- bindings carry confidence and evidence
- UI labels manual bindings as user-authored
- exports preserve uncertainty
- alternate candidates remain inspectable

### Risk: Performance Regresses On Shared Caches

Mitigation:

- route-scoped edit queries
- paged annotation lists
- incremental search index updates
- bulk edit batching
- no full-cache Swift materialization

## First Implementation Recommendation

Start with SE1 through SE3 before any richer type/decompiler/runtime edits:

1. core edit schema
2. route target validation
3. comments, labels, tags, bookmarks, colors
4. manual xrefs
5. undo/redo after reopen
6. search/inspector integration

This sequence creates visible Hopper-class value while also proving the hardest
architectural invariant: user-authored knowledge is durable, route-aware, and
not confused with analyzer facts.

Do not start with decompiler hints or ObjC/C++/XNU binding edits. Those are
high-value, but they depend on the same storage, transaction, target, conflict,
and reanalysis model. Building them first would hide foundational mistakes
behind domain complexity.
