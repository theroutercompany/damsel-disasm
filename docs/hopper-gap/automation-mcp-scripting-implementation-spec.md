# Automation, MCP, And Scripting Implementation Spec

Status: implementation architecture plan. This is not an implementation claim
and it does not enable automation in the current product.

Purpose: define how Damsel should expose Hopper-competitive automation through
stable project APIs, CLI batch operations, local MCP tools, future script
bindings, audited annotation mutation, capability checks, privacy controls, and
Workbench-visible activity logs.

This spec treats automation as a product architecture problem, not as a
thin wrapper around existing commands. Hopper's public scripting and MCP
surfaces show that serious reverse-engineering automation needs durable
document objects, stable procedure and segment APIs, mutation reasons,
transactions, byte access policy, and user-visible provenance.

## Research Companions

- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Public Research Notes](hopper-public-research.md)
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)

## Architecture Companions

- [API And Data Contracts](api-data-contracts.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
- [Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec](semantic-editing-reanalysis-hints-implementation-spec.md)

## Public Hopper Evidence

Official facts:

- Hopper homepage markets Python scripting.
- Hopper FAQ says its MCP server is configured from Hopper Help.
- Hopper FAQ says Python scripts can modify individual bytes.
- Hopper changelog shows active MCP work: v6.0.0 added MCP, v6.2.0 improved
  MCP reliability, v6.2.1 changed launch behavior, v6.2.2 and v6.2.3 made MCP
  fixes, v6.2.4 fixed document discovery, and v6.2.5 rewrote the MCP server in
  C++ for shared macOS/Linux code.
- Hopper changelog repeatedly mentions Python compatibility and Python API
  fixes.

Public Python API evidence:

- Public type stubs expose document, segment, section, procedure, basic block,
  instruction, call reference, local variable, tag, byte type, byte read/write,
  mark-code/procedure/data/undefined, comments, inline comments, names,
  references, strings, instruction lookup, decompile-to-string, local labels,
  register renaming, and procedure caller/callee APIs.
- Public scripts use Hopper's current document, current cursor/selection,
  segment byte reads, labels, comments, inline comments, file offsets, Yara
  matches, stack string annotations, import renaming, and byte export.
- A public 2025 procedure-name export workflow uses Hopper's script editor,
  current document, segments, named addresses, and labels to recover data the
  GUI did not export directly.

Public MCP evidence:

- Third-party Hopper MCP wrappers expose current assembly/pseudocode, lookup by
  name/address, comments, inline comments, segment/procedure/data inspection,
  and document control.
- Some wrappers rely on private Hopper interfaces or injection because Hopper's
  public extension surface does not expose every desired automation primitive.
  Damsel can avoid that fragility by designing typed project APIs first.

## Core Thesis

Automation should not get a privileged back door into Damsel internals.

Every automated operation should flow through the same model used by CLI and
Workbench:

```text
Project API
  |
  +-- route resolver
  +-- fact store
  +-- query service
  +-- byte window service
  +-- procedure database
  +-- runtime object store
  +-- annotation transaction service
  +-- job scheduler
  +-- problem store
```

Then projections become:

```text
CLI JSON batch mode
Workbench command registry
MCP tool server
future Python bindings
future plugin tools
```

The same object id, route, provenance, confidence, permission, paging, and
error contracts must appear everywhere. If MCP uses a different surface than
Workbench, Damsel will become brittle in exactly the way Hopper's third-party
wrappers reveal.

## Non-Goals For The First Automation Slice

Do not implement first:

- arbitrary Python execution inside the app
- plugin loading from untrusted bundles
- binary patch application
- process memory writes
- remote network automation
- project-wide raw byte dumps
- LLM-only features with no UI/CLI equivalent
- tools that return ungrounded prose without object references
- mutation tools that bypass transactions
- automatic upload of private binary facts to external services

Initial automation should be local, deterministic, bounded, and read-mostly.

## Automation Levels

### Level 0: CLI JSON Query Foundation

Purpose:

- make project facts scriptable without adding an embedded language

Capabilities:

- project summary
- source list
- route resolution
- search
- function list/detail
- xrefs
- assembly window
- byte window
- strings
- ObjC objects
- C++ objects
- kernel objects
- problems
- graph export

Why first:

- this forces stable schemas and error codes
- it is testable with snapshots
- it works in CI
- it does not add runtime sandboxing risk

### Level 1: CLI Batch Operations

Purpose:

- let users and future agents request multiple related objects in one
  deterministic call

Examples:

- get function plus callers, callees, strings, imports, and problems
- get selector plus implementers and callers
- get panic frame plus symbolication evidence and surrounding assembly
- get cache image plus providers/importers/reexports

Rules:

- each item can fail independently
- responses preserve order
- paging remains explicit
- errors include target object and code

### Level 2: Read-Only MCP Server

Purpose:

- expose Damsel project facts to MCP hosts without granting mutation

Server model:

- local stdio process by default
- no remote listener by default
- one project root or active Workbench session at a time
- tools generated from the same API schemas as CLI
- structured responses only
- provenance and confidence always included

### Level 3: Audited Annotation Mutation

Purpose:

- allow automation to add analyst knowledge without corrupting analysis facts

Allowed mutations:

- add comment
- add inline comment
- set label
- add tag
- add bookmark
- add manual xref
- apply problem hint
- set operand display format
- set local label
- set register alias

Rules:

- every mutation creates an annotation transaction
- transaction includes actor and tool id
- inverse operation is recorded where possible
- invalidated facts are listed
- Workbench activity log shows the change

### Level 4: Analysis Job Control

Allowed operations:

- start analysis pass
- cancel job
- list jobs
- retry failed job
- explain stale facts

Rules:

- automation cannot hide long-running work
- Workbench displays jobs started by automation
- cancellation is best-effort and explicit

### Level 5: Dangerous Operations, Later

Dangerous operations:

- patch preview
- patch apply/export
- debugger launch/attach
- process memory read
- process memory write, much later

Rules:

- separate capabilities
- per-session confirmation
- audit log
- Workbench-visible target identity
- no default MCP access

## Capability Model

Capability groups:

- `project.read`
- `source.read_metadata`
- `source.read_bytes`
- `source.export_selection`
- `route.resolve`
- `search.read`
- `procedure.read`
- `graph.read`
- `xref.read`
- `runtime.read_objc`
- `runtime.read_cpp`
- `runtime.read_kernel`
- `debug_info.read`
- `problem.read`
- `annotation.write`
- `hint.apply`
- `analysis.run`
- `job.cancel`
- `patch.preview`
- `patch.apply`
- `debug.launch`
- `debug.attach`
- `debug.read_memory`
- `debug.write_memory`

Default MCP profile:

- read-only project facts
- no raw byte export above bounded windows
- no mutation
- no debugger
- no patching

Workbench-granted annotation profile:

- all default reads
- annotation writes
- hints
- analysis jobs
- audit required

Dangerous profile:

- patch preview/apply
- debugger launch/attach
- process memory reads
- never enabled by default

## Tool Design Principles

### Tool Responses Must Be Grounded

Every response includes:

- object id or route
- source id
- address-space id where relevant
- provenance
- confidence
- stale/partial flag
- related problems
- result version

Avoid:

- unbounded prose summaries
- raw binary dumps by default
- silent truncation
- bare addresses with no address-space identity
- mutation success without transaction id

### Tools Should Be Narrow But Composable

Bad shape:

```text
analyze_this_binary_deeply()
```

Good shape:

```text
get_project_summary()
search_project(query)
get_function(route, include_xrefs=true)
get_assembly_window(route, context=40)
get_objc_selector(selector)
get_xrefs(target)
add_comment(route, text)
```

The tool server can provide higher-level workflows later, but primitive tools
must stay deterministic and inspectable.

### Batch Support Matters

LLM and script workflows should not need hundreds of round trips for ordinary
questions.

Batch-capable tools:

- resolve routes
- get functions
- get xrefs
- get assembly windows
- get byte windows
- add comments
- add tags
- apply hints

Batch response rule:

- per-item errors are returned inline
- successful items remain usable
- overall transport failure is reserved for malformed requests or permission
  denial

## Initial MCP Tool Set

### Project Tools

- `get_project_summary`
- `list_sources`
- `get_source`
- `list_address_spaces`
- `get_project_metrics`
- `list_jobs`
- `cancel_job`
- `list_problems`

### Navigation And Search Tools

- `resolve_route`
- `search_project`
- `go_to_address`
- `get_related_routes`
- `get_feature_map_window`

### Byte And Assembly Tools

- `get_assembly_window`
- `get_instruction`
- `get_byte_window`
- `get_semantic_spans`
- `get_operand_format`

### Procedure And Graph Tools

- `list_procedures`
- `get_procedure`
- `get_basic_blocks`
- `get_cfg`
- `get_callers`
- `get_callees`
- `get_xrefs`

### Runtime Object Tools

- `list_objc_classes`
- `get_objc_class`
- `get_objc_selector`
- `get_objc_message_sites`
- `list_cpp_types`
- `get_cpp_type`
- `get_vtable`
- `list_kernel_members`
- `get_kernel_member`
- `symbolicate_panic`

### Annotation Mutation Tools

- `add_comment`
- `add_inline_comment`
- `set_label`
- `add_tag`
- `add_bookmark`
- `add_manual_reference`
- `set_operand_format`
- `set_local_label`
- `set_register_alias`
- `apply_problem_hint`

Every mutation tool returns:

- transaction id
- changed targets
- inverse availability
- invalidated facts
- queued jobs
- warnings

### Later Dangerous Tools

- `preview_patch`
- `apply_patch_plan`
- `export_patched_artifact`
- `debug_launch`
- `debug_attach`
- `debug_set_breakpoint`
- `debug_read_memory`

These require separate opt-in profiles.

## API Shape

### Tool Manifest

```json
{
  "server": "damsel-mcp",
  "version": "0.1.0",
  "project_schema_version": "1",
  "tools": [
    {
      "name": "get_function",
      "capabilities": ["procedure.read"],
      "mutation_class": "read_only",
      "destructive": false,
      "bounded": true
    }
  ]
}
```

### Common Request Envelope

```json
{
  "project_id": "project:1",
  "result_version": "index:42",
  "arguments": {},
  "privacy": {
    "allow_raw_bytes": false,
    "allow_private_strings": false
  }
}
```

### Common Response Envelope

```json
{
  "result": {},
  "provenance": [],
  "confidence": "strong",
  "problems": [],
  "warnings": [],
  "result_version": "index:42",
  "truncated": false
}
```

### Mutation Response Envelope

```json
{
  "transaction_id": "tx:annotation:17",
  "changed": [],
  "inverse_available": true,
  "invalidated_facts": [],
  "queued_jobs": [],
  "warnings": []
}
```

## Workbench Integration

Workbench surfaces:

- automation panel
- MCP server status
- active clients
- capability grant dialog
- tool activity log
- transaction history
- privacy redaction settings
- generated report/export list

Every automation action should be visible in at least one place:

- read tools appear in activity log when run from an external client
- mutation tools appear in transaction history
- started jobs appear in jobs panel
- permission denials appear in problems/log panel

Workbench should display origin badges:

- analyzer
- user
- CLI
- script
- MCP
- plugin
- debug observation

## Privacy Model

Sensitive classes:

- source paths
- raw bytes
- strings
- symbols
- ObjC class and selector names from private apps
- C++ type names
- panic logs
- environment variables
- process names
- debug memory

Privacy controls:

- redacted source paths by default in external tools
- raw byte windows bounded by default
- private strings opt-in
- no whole-project export through MCP by default
- no external network transport by default
- activity log records client identity

## Error Model

Common errors:

- project_not_open
- source_not_found
- route_not_found
- route_ambiguous
- permission_denied
- privacy_denied
- stale_result_version
- result_too_large
- unsupported_object_kind
- mutation_requires_transaction
- source_revision_mismatch
- analysis_incomplete
- tool_disabled

Errors should include:

- code
- message
- target
- recovery
- related problem id, if any

## Implementation Plan

### A0: Shared API Schema Extraction

Deliverables:

- shared Rust structs for project query responses
- JSON schema snapshots
- typed error codes
- route and object id serialization tests

### A1: Batch Query CLI

Deliverables:

- `damsel project query`
- batch route resolution
- batch function/xref/assembly queries
- per-item errors
- snapshot tests

### A2: Read-Only MCP Prototype

Deliverables:

- local stdio server
- tool manifest
- project summary/source/search/function/xref tools
- read-only capability profile
- fixture transcript tests

### A3: Annotation Mutation

Deliverables:

- annotation transaction API
- MCP mutation tools for comments, labels, tags, bookmarks, manual refs
- Workbench activity log entries
- permission-denial tests
- undo/redo tests

### A4: Runtime Object Tools

Deliverables:

- ObjC selector/class tools
- C++ type/vtable tools
- kernel member/panic tools
- source/provenance in every response

### A5: Report Export

Deliverables:

- structured report generation from stable queries
- local filesystem export with explicit path choice
- privacy redaction options

### A6: Future Language Bindings

Deliverables:

- generated Python bindings over stable HTTP/CLI/MCP schemas
- no embedded Python runtime in first binding
- transaction-aware mutation calls

## Validation Plan

### Schema Gate

- tool manifest snapshot
- JSON schema snapshot
- object id stability tests
- route serialization tests

### Read Tool Gate

- project summary fixture
- source list fixture
- search fixture
- function detail fixture
- xref fixture
- assembly window fixture
- byte window privacy fixture

### Batch Gate

- multiple functions in one request
- mixed success and per-item failure
- stale result version
- bounded result truncation

### Mutation Gate

- add comment transaction
- set label transaction
- add tag transaction
- manual xref transaction
- permission denied without capability
- undo/redo from automation-created transaction

### Privacy Gate

- raw bytes denied by default
- private strings redacted by default
- source paths redacted for external clients
- panic log redaction fixture

### MCP Gate

- stdio transcript fixture
- tool errors are structured
- tool response includes provenance/confidence
- no mutation tools in read-only profile
- activity log records client/tool

## Risks

### Risk: MCP Overfits To LLM Prompts

Mitigation:

- design typed tools first
- keep natural-language summaries secondary
- include object ids and routes in every response

### Risk: Automation Bypasses UX Safeguards

Mitigation:

- capability checks in core API
- transactions for every mutation
- Workbench-visible activity log
- same undo/redo stack as UI

### Risk: API Freezes Too Early

Mitigation:

- version every schema
- mark experimental tools
- keep internal Rust models separate from public wire contracts

### Risk: Sensitive Data Leaks

Mitigation:

- local-only default
- privacy flags
- bounded byte windows
- redaction snapshots
- no external network transport by default

## Completion Criteria

The automation feature family is implementation-ready when:

- CLI, Workbench, MCP, and future scripts share object ids and routes
- read-only tools can answer common reverse-engineering questions
- batch queries have deterministic partial-failure semantics
- mutation tools create audited annotation transactions
- every tool response carries provenance, confidence, and problems
- privacy defaults prevent raw byte and private-string leaks
- Workbench shows automation activity and permissions
- tool schemas are snapshot-tested

The first product-complete automation slice requires read-only MCP plus audited
annotation mutation over comments, labels, tags, bookmarks, and manual xrefs.
Patch application, debugger control, process memory writes, and embedded Python
remain later scopes.
