# API And Data Contracts

Status: Planning contract. This document defines the API/data shape needed for
the competitive Workbench and CLI. It is not a committed wire format yet.

Purpose: ensure that new app features are backed by stable project, query,
analysis, graph, runtime, kernel, and annotation contracts instead of one-off
view payloads.

Surface implementation companion:
[Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
adds concrete project-backed endpoint shapes for route resolution, assembly
windows, inspector modules, graph models, hex windows, feature maps, search,
and navigators.

Runtime implementation companion:
[Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
adds concrete API contracts for ObjC selectors/classes/methods/messages,
C++ classes/vtables/virtual calls, dyld shared-cache images/edges,
symbolication profiles, kernel collections/kexts/interfaces, and panic frames.

Runtime Workbench browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
adds concrete API contract shapes for Apple project overview pages, cache
browser pages, image/framework pages, ObjC class/selector/message-send pages,
C++/ObjC++ class/vtable/virtual-call pages, IOKit pages, kernel member/kext
pages, XNU interface pages, KDK/dSYM symbolication pages, panic frame pages,
runtime relationship graphs, and runtime problem pages.

SDK research companion:
[Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
grounds these contracts in Hopper's public document, disassembled-file,
procedure, byte-type, reference, transaction, Python API, MCP, loader, CPU, and
tool surfaces.

Feature architecture companion:
[Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
grounds the API shape in Hopper's public document, byte map, procedure,
basic-block, type/signature, architecture-semantics, annotation, decompiler,
debugger, performance, and automation feature families.

Type/signature companion:
[Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
defines the API objects for type records, type facts, function signatures,
signature candidates, calling conventions, variables, register aliases, local
labels, debug-info imports, symbolication profiles, type conflicts, and
type-edit transactions.

Processor semantics companion:
[Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)
defines the API objects for architecture profiles, ABI profiles, CPU modes,
register roles, instruction semantics, control effects, stack/frame effects,
callsite ABI interpretations, PAC states, semantic problems, and CPU-mode,
switch-hint, and calling-convention override transactions.

Debug-info companion:
[DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
defines the API objects for debug sources, dSYM/KDK candidates, match
explanations, import sessions, compile units, source files, line rows, inline
callsites, variables, location expressions, debug type records, symbolication
profiles, source-line queries, and debug-info problems.

Graph/representation companion:
[Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
defines the API objects for graph composition, graph nodes and edges, layout
jobs, feature-map lanes and ranges, bounded hex windows, canonical selection
resolution, synchronized representation updates, graph export, accessibility
fallbacks, and Apple-runtime graph variants.

Workbench implementation companion:
[Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
maps these contracts onto concrete Swift client groups, route/session stores,
representation stores, job/problem drawers, and transaction/undo integration.

Command/navigation companion:
[Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
adds concrete API contract shapes for command introspection, command
availability, command execution, navigation-session persistence, transaction
stack bridging, activity logs, disabled reasons, and route-backed command
results shared by native UI, CLI, MCP, and future plugins.

Procedure and decompiler companions:
[Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
and
[Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
define the API objects for persistent procedures, blocks, edges, callsites,
local labels, register aliases, stack locals, recovery problems, IR snapshots,
and decompiler-lite summaries.

Decompiler Workbench companion:
[Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
defines the API objects for decompiler jobs, cached pseudocode views,
line/token source maps, confidence records, decompiler issues, user hints,
variable/type correction transactions, and evidence exports.
[Decompiler Quality And User-Assisted Recovery Implementation Spec](decompiler-quality-user-assisted-recovery-implementation-spec.md)
expands those contracts into decompiler runs, pass ledgers, quality reports,
coverage summaries, source-map inspectors, user hint applications, dead-code
and switch recovery records, regression cases, comparison reports, and
performance samples.

Mutation and debugger companions:
[Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
and
[Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
define the API objects for byte windows, patchability, patch plans, source
revisions, patch transactions, debug sessions, runtime modules, breakpoints,
stop events, registers, frames, and read-only memory windows.
[Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
expands the live debugger API surface into debug targets, launch
configurations, attach candidates, session event streams, control commands,
module/source matching, route-backed breakpoints, register/value
interpretations, memory read windows, diagnostics, and redacted exports.
[Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md)
expands the patching API surface into patchability queries, patch plans,
previews, impact records, source revisions, assembler/NOP diagnostics,
copy-on-write export plans, patch history, debugger launch handoff, and
evidence packages.

Automation and extension companions:
[Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
and
[Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
define the API objects for tool manifests, invocation envelopes, capability
checks, automation activity, annotation mutation transactions, plugin
manifests, extension runs, plugin provenance, helper-process isolation, and
plugin-produced fact/problem records.
[Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec](automation-studio-scripting-plugin-workbench-implementation-spec.md)
expands that surface into recipe catalogs, automation runs, MCP sessions,
script packages, plugin packages, capability grants, extension outputs, emitted
facts/problems, Apple-system recipe outputs, diagnostics, and activity logs.
[Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md)
defines assistant API contracts for sessions, context, evidence items,
tool-call timelines, model profiles, privacy policies, route/fact citations,
reviewed suggestions, and redacted assistant evidence exports.

Performance runtime companion:
[Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
defines the API objects for path-based source registration, source identity,
availability, byte windows, job progress, event streams, index status, paged
queries, result versions, metrics, slow queries, caches, and diagnostics export
policy.

Diff/export/evidence companion:
[Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
defines the API objects for analysis snapshots, artifact identities, object
fingerprints, diff projects, match records, findings, evidence packages, export
manifests, redaction policy, and Workbench diff navigation.

Native app trust companion:
[Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
defines the API objects for trust status, app/helper identity, local server
sessions, source access grants, security bookmarks, diagnostics policy,
diagnostic bundles, crash/hang summaries, update state, plugin trust, and
debugger trust.

Artifact intake companion:
[Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
defines the API objects for intake sessions, intake inputs, artifact
candidates, source registration plans, artifact identities, container manifests,
projection and extraction recipes, sidecar candidates, symbolication profiles,
external tool receipts, and intake problems.

## Contract Principles

- Every navigable object has a stable id.
- Every address includes address-space identity.
- Every inferred fact includes provenance and confidence.
- Every large list is pageable.
- Every long-running operation is a job.
- Every API error has a typed code.
- CLI JSON and UI API should share model vocabulary.
- Existing point-query APIs remain supported during migration.
- Local app/helper trust, source access, and diagnostics APIs use the same
  problem, actor, capability, provenance, and redaction vocabulary as analysis
  APIs.
- Artifact intake APIs produce registration plans before heavy analysis so
  Workbench, CLI, automation, and diagnostics can reason about the same source
  graph.

## Shared Primitives

### Object Id

```json
{
  "id": "function:project-1:image-2:100003f20",
  "kind": "function"
}
```

Rules:

- ids are opaque to UI
- ids are stable within a project
- ids are deterministic where practical
- ids include enough internal structure for debugging, but clients should not
  parse them

### Address

```json
{
  "space_id": "addrspace:cache-image:Foundation",
  "value": 4294983456,
  "display": "0x100003f20",
  "kind": "image_vmaddr"
}
```

Optional fields:

- `file_offset`
- `image_offset`
- `cache_vmaddr`
- `member_file_offset`
- `runtime_address`
- `slide`

### Provenance

```json
{
  "source": "objc_method_list",
  "analyzer": "objc-metadata-v2",
  "analysis_run_id": "run:12",
  "confidence": "strong",
  "notes": ["selector resolved from __objc_methname"]
}
```

Confidence values:

- exact
- strong
- inferred
- ambiguous
- weak
- user_asserted
- contradicted

### Paged Response

```json
{
  "items": [],
  "page": {
    "limit": 100,
    "cursor": "opaque-next-cursor",
    "next_cursor": "opaque-next-cursor-2",
    "total": 1234,
    "total_is_exact": true,
    "result_version": "index:42"
  }
}
```

Rules:

- `total` may be null when expensive
- cursor is opaque
- result version helps detect stale paging

### Problem

```json
{
  "id": "problem:17",
  "severity": "warning",
  "code": "kdk_uuid_mismatch",
  "message": "Attached KDK does not match the kernel collection UUID.",
  "target": { "id": "source:kdk-1", "kind": "source" },
  "recovery": "Attach a KDK matching the kernel collection build.",
  "details": {}
}
```

Severity:

- info
- warning
- error
- fatal

### Actor

```json
{
  "kind": "user",
  "id": "local-user",
  "display": "Local User"
}
```

Actor kinds:

- user
- cli
- workbench
- automation_tool
- mcp_client
- plugin
- analyzer

Rules:

- any mutation records an actor
- actor ids are stable enough for audit logs
- sensitive external client details are redacted by default

### Capability

Capability groups:

- `project.read`
- `source.read_metadata`
- `source.read_bytes`
- `route.resolve`
- `search.read`
- `procedure.read`
- `graph.read`
- `xref.read`
- `runtime.read`
- `debug_info.read`
- `annotation.write`
- `analysis.job_control`
- `patch.preview`
- `patch.apply`
- `debug.launch`
- `debug.attach`
- `extension.run`

Rules:

- read capabilities are split from byte export
- mutation requires explicit capabilities
- dangerous capabilities are never granted by default to MCP or plugins

### Tool Manifest

```json
{
  "tool_id": "damsel.get_function",
  "version": "1",
  "description": "Return route-backed function facts.",
  "capabilities": ["project.read", "procedure.read", "xref.read"],
  "input_schema": { "type": "object" },
  "output_schema": { "type": "object" }
}
```

Rules:

- CLI batch, MCP, and future script bindings should be generated from the same
  manifest source where practical
- manifests are versioned independently from the app version
- schemas must prefer object ids, routes, provenance, and page cursors over
  unstructured prose

### Tool Invocation

```json
{
  "invocation_id": "toolrun:42",
  "tool_id": "damsel.get_function",
  "tool_version": "1",
  "actor": {
    "kind": "mcp_client",
    "id": "local-mcp-session"
  },
  "project_id": "project:1",
  "capabilities": ["project.read", "procedure.read"],
  "input": {
    "route": "route:function:project-1:image-2:100003f20"
  }
}
```

Response envelope:

```json
{
  "invocation_id": "toolrun:42",
  "ok": true,
  "result": {},
  "provenance": [],
  "warnings": [],
  "redactions": []
}
```

Rules:

- each invocation is logged when it reads sensitive data or performs mutation
- partial batch failures include per-item errors
- tool results should include enough routes to reproduce the answer in the UI

### Automation Activity

```json
{
  "id": "activity:automation:42",
  "time": "2026-05-26T20:00:00Z",
  "actor": {
    "kind": "mcp_client",
    "id": "local-mcp-session"
  },
  "tool_id": "damsel.add_comment",
  "project_id": "project:1",
  "targets": [
    { "id": "route:addr:project-1:image-2:100003f20", "kind": "route" }
  ],
  "transaction_id": "txn:17",
  "summary": "Added an inline comment."
}
```

Rules:

- Workbench displays automation activity that mutates project state
- read-only activity can be summarized or sampled depending on privacy policy
- raw binary bytes are never stored in activity logs by default

### Plugin Manifest

```json
{
  "plugin_id": "com.example.damsel.objc-extra",
  "schema_version": "1",
  "plugin_version": "0.1.0",
  "kind": "analyzer",
  "entrypoint": {
    "type": "helper_process",
    "command": "objc-extra"
  },
  "capabilities": ["project.read", "procedure.read"],
  "emits": ["fact", "problem"],
  "requires": {
    "damsel_api": "1"
  }
}
```

Rules:

- plugin facts record plugin id, plugin version, and schema version
- loader, sidecar, analyzer, renderer/exporter, architecture semantics, and
  Workbench command extensions have distinct manifests
- external plugins run outside the Workbench process unless explicitly trusted

### Source Registration

```json
{
  "path": "/System/Library/dyld/dyld_shared_cache_arm64e",
  "kind_hint": "dyld_shared_cache",
  "analysis_profile": "inventory_first",
  "privacy": "local_path"
}
```

Response:

```json
{
  "source_id": "source:dyld-cache:1",
  "kind": "dyld_shared_cache",
  "display_name": "dyld_shared_cache_arm64e",
  "availability": "local",
  "content_identity": {
    "size_bytes": 4294967296,
    "hash": "sha256:...",
    "uuid": null
  },
  "registered_jobs": ["job:source-scan:1"]
}
```

Rules:

- path registration is local-only
- diagnostics redact local paths by default
- source identity is separate from byte storage
- missing sources become recoverable availability problems

### Job Record

```json
{
  "job_id": "job:analysis:42",
  "project_id": "project:1",
  "source_id": "source:1",
  "kind": "xref_index",
  "priority": "interactive",
  "state": "running",
  "progress": {
    "phase": "procedure batch",
    "completed_units": 120,
    "total_units": 1000,
    "message": "Indexing cross references"
  },
  "cancelable": true,
  "dependencies": ["job:procedure-index:17"]
}
```

Job states:

- created
- queued
- running
- cancel_requested
- canceled
- completed
- completed_with_warnings
- failed
- blocked
- stale

Priority classes:

- foreground
- interactive
- background
- maintenance
- opt_in_heavy

Rules:

- long-running work gets a job record
- cancellation is acknowledged immediately and checked in worker passes
- failed jobs create problem records when user-visible

### Index Status

```json
{
  "index_id": "index:xrefs:source-1",
  "kind": "xref",
  "source_id": "source:1",
  "version": "index:42",
  "state": "partial",
  "fresh_for_project_revision": "project-rev:91",
  "jobs": ["job:xref-index:7"]
}
```

States:

- missing
- building
- partial
- ready
- stale
- failed

Rules:

- paged responses include result versions
- stale cursors produce typed errors
- UI can show partial results with associated jobs

### Byte Window

```json
{
  "source_id": "source:1",
  "range": {
    "start": { "space_id": "addrspace:image-1", "value": 4294983456 },
    "length": 64
  },
  "bytes_base64": "...",
  "classification_runs": [],
  "source_revision": "source-rev:1",
  "redacted": false
}
```

Rules:

- byte windows are bounded
- virtual zero-fill ranges are explicit
- projected cache windows preserve parent source identity
- byte access has separate capability checks from metadata reads

### Analysis Snapshot

```json
{
  "snapshot_id": "snapshot:1",
  "project_id": "project:1",
  "project_revision": "project-rev:91",
  "source_ids": ["source:1"],
  "analyzer_versions": ["procedure-db-v1"],
  "included_fact_kinds": ["procedure", "xref", "objc"],
  "completeness": "full_for_selected_scope",
  "redaction_policy": "default"
}
```

Rules:

- diff jobs compare snapshots, not live mutable state
- snapshots include source identities and analyzer versions
- snapshots record excluded fact kinds
- raw bytes are excluded by default

### Object Fingerprint

```json
{
  "object_id": "function:project-1:image-2:100003f20",
  "route": "route:function:project-1:image-2:100003f20",
  "kind": "procedure",
  "fingerprints": [
    {
      "kind": "cfg_shape",
      "value": "sha256:...",
      "confidence": "strong",
      "input_versions": ["index:42"]
    }
  ]
}
```

Rules:

- fingerprints are evidence inputs, not absolute identity by themselves
- address-only fingerprints are weak across OS builds
- runtime-object fingerprints can survive address churn

### Diff Project

```json
{
  "diff_id": "diff:1",
  "baseline_snapshot_id": "snapshot:old",
  "comparison_snapshot_ids": ["snapshot:new"],
  "matching_profile": "dyld_cache_build_diff",
  "status": "running",
  "jobs": ["job:diff-match:1"]
}
```

### Diff Finding

```json
{
  "finding_id": "finding:1",
  "category": "selector_changed",
  "severity": "notable",
  "title": "Selector gained a new implementation",
  "routes": ["route:objc-selector:project-1:doThing:"],
  "confidence": "strong",
  "evidence": [],
  "included_in_export": true
}
```

Rules:

- severity is analyst workflow severity, not exploitability
- findings include routeable evidence
- ambiguous matches remain visible

### Evidence Package Manifest

```json
{
  "package_id": "evidence:1",
  "schema_version": "1",
  "source_snapshots": ["snapshot:old", "snapshot:new"],
  "included_findings": ["finding:1"],
  "formats": ["json", "markdown"],
  "redactions": ["local_paths", "raw_bytes", "private_strings"],
  "reproduction_commands": []
}
```

Rules:

- export manifests include schema versions
- redaction logs are mandatory
- human-readable reports are generated from canonical structured facts

### Byte Classification

```json
{
  "id": "byteclass:source-1:100003f20-100003f34",
  "source_id": "source:1",
  "range": {
    "start": { "space_id": "addrspace:image-1", "value": 4294983456 },
    "length": 20
  },
  "file_range": {
    "offset": 16160,
    "length": 20
  },
  "kind": "code",
  "provenance": {
    "source": "function_start_analysis",
    "confidence": "strong"
  },
  "user_override": false
}
```

Classification kinds:

- unknown
- header
- load_command
- code
- procedure
- data
- string
- objc_metadata
- cpp_metadata
- swift_metadata
- relocation_or_fixup
- debug_info
- excluded

Apple-runtime extension kinds from the runtime object spec:

- objc_selector_ref
- objc_class_ref
- objc_method
- objc_message
- cpp_vtable
- cpp_rtti
- swift_symbol
- pac_pointer
- chained_fixup
- cache_image_boundary
- kernel_table
- mig_descriptor
- panic_frame_target

### Source Revision

```json
{
  "id": "source-revision:source-1:original",
  "source_id": "source:1",
  "parent_revision_id": null,
  "content_hash": "sha256:...",
  "is_original": true,
  "is_exported": false,
  "created_by_transaction_id": null
}
```

Rules:

- original source revisions are immutable
- patch application creates derived revisions
- analysis facts reference the source revision they were produced from
- Workbench must display active revision identity before any patch workflow

### Patch Plan

```json
{
  "id": "patch-plan:1",
  "source_id": "source:1",
  "source_revision_id": "source-revision:source-1:original",
  "source_hash_precondition": "sha256:...",
  "operations": [],
  "affected_objects": [],
  "warnings": [],
  "can_apply": false
}
```

Rules:

- preview can exist before apply support
- apply requires source hash precondition to still match
- patch plans never mutate original inputs
- shared-cache and kernel sources default to preview-only or unsupported

### Debug Session

```json
{
  "id": "debug-session:1",
  "project_id": "project:1",
  "kind": "local_launch",
  "state": "stopped",
  "target": {},
  "permissions": ["debug_launch", "read_process_memory"]
}
```

Rules:

- launch/attach are explicit user actions
- runtime modules are matched to project sources by UUID/hash/cache identity
- breakpoints are route-backed objects
- read-only memory windows are bounded and audited

### Semantic Span

```json
{
  "kind": "selector",
  "text": "@selector(initWithFrame:)",
  "range": { "start": 24, "length": 25 },
  "target": { "id": "objc-selector:initWithFrame:", "kind": "objc_selector" },
  "operand_index": 1
}
```

Span kinds:

- address
- global_symbol
- local_label
- register
- register_alias
- stack_variable
- immediate
- formatted_immediate
- selector
- class
- protocol
- string
- comment
- xref_target
- unresolved_target

Apple-runtime span kinds:

- objc_message_send
- objc_method
- cpp_symbol
- cpp_vtable_slot
- swift_demangled_symbol
- pac_target
- chained_fixup_target
- cache_image
- kext
- kernel_interface
- panic_frame

## Project Endpoints

### Create Project

```text
POST /api/projects
```

Request:

```json
{
  "name": "Foundation Research",
  "sources": [
    {
      "kind": "file",
      "path": "/System/Library/dyld/dyld_shared_cache_arm64e"
    }
  ],
  "options": {
    "analysis_profile": "triage"
  }
}
```

Response:

```json
{
  "project_id": "project:1",
  "manifest": {},
  "initial_jobs": []
}
```

Notes:

- Path registration is for local Workbench and CLI workflows.
- Browser upload mode can continue using existing `/api/images` endpoint.
- Sandboxed app distribution may need security-scoped bookmarks later.

### Open Project

```text
GET /api/projects/{project_id}
```

Returns:

- manifest
- source summary
- analysis summary
- current schema version
- stale/migration status

### Close Project

```text
POST /api/projects/{project_id}/close
```

Effects:

- flush pending annotations
- cancel background jobs if requested
- release hot caches

## Source Endpoints

### List Sources

```text
GET /api/projects/{project_id}/sources
```

Source fields:

- id
- kind
- role
- path display
- hash
- UUID
- architecture
- platform
- parent source
- relationship
- status

### Register Source Path

```text
POST /api/projects/{project_id}/sources/register-path
```

Rules:

- local-only path registration
- records source identity and availability
- starts inventory jobs instead of blocking until deep analysis completes
- compatibility upload endpoints can remain during migration

### Attach Sidecar

```text
POST /api/projects/{project_id}/sources/attach
```

Sidecar kinds:

- dSYM
- KDK
- dyld symbols sidecar
- panic log
- external symbol map

Errors:

- `sidecar_not_found`
- `uuid_mismatch`
- `unsupported_sidecar`
- `sidecar_parse_failed`

## Job Endpoints

### Start Analysis

```text
POST /api/projects/{project_id}/analyze
```

Request:

```json
{
  "profile": "triage",
  "targets": [
    { "kind": "source", "id": "source:1" }
  ],
  "passes": ["symbols", "objc", "functions", "xrefs"]
}
```

Response:

```json
{
  "jobs": [
    {
      "job_id": "job:1",
      "state": "queued"
    }
  ]
}
```

### Job Status

```text
GET /api/projects/{project_id}/jobs
GET /api/projects/{project_id}/jobs/{job_id}
```

Fields:

- job id
- pass id
- state
- priority
- progress
- current item
- warnings
- errors
- started/finished timestamps
- cancellable

### Cancel Job

```text
POST /api/projects/{project_id}/jobs/{job_id}/cancel
```

Response:

- cancellation accepted
- already completed
- not cancellable

## Query Endpoints

### Global Search

```text
GET /api/projects/{project_id}/search?q=foo&scope=all&limit=50
```

Result:

```json
{
  "items": [
    {
      "id": "result:1",
      "kind": "function",
      "title": "-[NSView initWithFrame:]",
      "subtitle": "Foundation / Objective-C method",
      "target": { "id": "function:...", "kind": "function" },
      "route": {},
      "score": 0.97
    }
  ],
  "page": {}
}
```

### Resolve Route

```text
POST /api/projects/{project_id}/routes/resolve
```

Purpose:

- convert search result or object id into a view route
- allow Workbench to navigate consistently

## Byte Map And Feature Map Endpoints

### Byte Map

```text
GET /api/projects/{project_id}/byte-map?source=source:1&range=0x100000000..0x100010000
```

Returns:

- byte classification ranges
- mapped/unmapped gaps
- classification provenance
- user override marker
- result version

### Override Byte Classification

```text
POST /api/projects/{project_id}/byte-map/override
```

Request:

```json
{
  "source_id": "source:1",
  "range": {
    "start": { "space_id": "addrspace:image-1", "value": 4294983456 },
    "length": 20
  },
  "kind": "procedure",
  "reason": "user_marked_function_start"
}
```

Response:

- transaction id
- invalidated facts
- queued reanalysis jobs

### Feature Map

```text
GET /api/projects/{project_id}/feature-map?route=route:current
```

Returns:

- compact byte-type runs
- current cursor
- viewport range
- search hit ranges
- problem ranges
- bookmark/tag ranges
- cache-image/member boundaries when applicable

## Problem Endpoints

### List Problems

```text
GET /api/projects/{project_id}/problems?severity=warning&limit=100
```

Fields:

- problem id
- severity
- code
- route
- target object
- analysis pass
- suggested hint actions
- suppressed state

### Apply Hint

```text
POST /api/projects/{project_id}/problems/{problem_id}/hints
```

Examples:

- declare function start
- add manual reference
- provide switch table base/count/stride
- bind ObjC receiver or selector
- select symbolication profile

Response:

- transaction id
- updated problem state
- queued reanalysis jobs

## Function Endpoints

### List Functions

```text
GET /api/projects/{project_id}/functions?filter=&source=&limit=100
```

Function list item:

- id
- display name
- start address
- range summary
- source/image/member
- callers count
- callees count
- string refs count
- runtime refs count
- confidence
- analysis status

### Function Detail

```text
GET /api/projects/{project_id}/functions/{function_id}
```

Includes:

- metadata
- names
- ranges
- blocks summary
- edge summary
- callers/callees summary
- runtime refs
- source/provenance
- comments/tags
- problems

### Procedure Local Edits

```text
POST /api/projects/{project_id}/functions/{function_id}/local-labels
POST /api/projects/{project_id}/functions/{function_id}/register-aliases
POST /api/projects/{project_id}/functions/{function_id}/stack-variables
POST /api/projects/{project_id}/functions/{function_id}/signature
POST /api/projects/{project_id}/functions/{function_id}/calling-convention
```

Rules:

- every mutation returns a transaction id
- imported or inferred facts cannot overwrite user-authored facts silently
- renderer output must include the current local labels, aliases, variables,
  signature, and calling convention

### Semantic Assembly

```text
GET /api/projects/{project_id}/functions/{function_id}/assembly?semantic=true
```

Each row includes:

- route
- address
- byte span
- text
- semantic spans
- comments
- xrefs
- problems
- current byte classification

### Function Instructions

```text
GET /api/projects/{project_id}/functions/{function_id}/instructions?cursor=&limit=200
```

Instruction row:

- id
- address
- bytes
- opcode
- mnemonic
- operands
- rendered
- labels
- comments
- refs summary
- badges

## Xref Endpoints

### Query Xrefs

```text
GET /api/projects/{project_id}/xrefs?to=object_id
GET /api/projects/{project_id}/xrefs?from=object_id
```

Reference item:

- id
- kind
- from
- to
- instruction address
- function id
- source/provenance
- confidence
- route

Requirements:

- bidirectional
- paged
- stable sorted
- cross-image provenance included

## Graph Endpoints

### Get Graph

```text
GET /api/projects/{project_id}/graphs/{graph_id}
```

Graph response:

- id
- kind
- subject
- nodes
- edges
- layout status
- layout data when available
- expansion cursor when partial

### Request Graph Layout

```text
POST /api/projects/{project_id}/graphs/{graph_id}/layout
```

Response:

- job id
- cached layout id if already available

## ObjC Endpoints

### Classes

```text
GET /api/projects/{project_id}/objc/classes
```

Class item:

- id
- name
- superclass
- method count
- category count
- protocol count
- source/image/member

### Selectors

```text
GET /api/projects/{project_id}/objc/selectors?q=init
```

Selector item:

- id
- name
- implementer count
- caller count
- ref count

### Method Detail

```text
GET /api/projects/{project_id}/objc/methods/{method_id}
```

Fields:

- owner
- selector
- kind
- type encoding raw/parsed
- IMP function id
- source/provenance
- callers

### Message Sends

```text
GET /api/projects/{project_id}/objc/message-sends?selector=...
```

Fields:

- callsite
- selector
- receiver candidates
- possible methods
- dispatch kind
- confidence
- unresolved reason

## C++ Endpoints

### Types

```text
GET /api/projects/{project_id}/cpp/types?q=IOService
```

Type item:

- id
- demangled name
- namespace
- vtable count
- method count
- base count
- derived count
- confidence

### Vtable Detail

```text
GET /api/projects/{project_id}/cpp/vtables/{vtable_id}
```

Fields:

- address
- owner candidate
- typeinfo
- slots
- pointer-auth facts
- xrefs

### Virtual Calls

```text
GET /api/projects/{project_id}/cpp/virtual-calls?type=...
```

Fields:

- callsite
- slot offset
- candidate vtables
- candidate targets
- confidence

## Kernel Endpoints

### Kernel Overview

```text
GET /api/projects/{project_id}/kernel
```

Fields:

- collection identity
- member count
- kext count
- KDK status
- IOKit class count
- syscall/trap/MIG counts
- problems

### Members

```text
GET /api/projects/{project_id}/kernel/members
```

Fields:

- id
- name
- address range
- source
- kext identity if known

### Kexts

```text
GET /api/projects/{project_id}/kernel/kexts
```

Fields:

- id
- bundle id
- version
- dependencies
- member
- symbol count
- IOKit class count

### Panic Symbolication

```text
POST /api/projects/{project_id}/kernel/panic-symbolicate
```

Request:

- panic text
- optional slide
- optional frame extraction settings

Response:

- frames
- symbols
- kext/member
- source lines if available
- unresolved frames
- warnings

## Cache Endpoints

### Cache Overview

```text
GET /api/projects/{project_id}/cache
```

Fields:

- cache UUID
- architecture
- member count
- image count
- local symbols status

### Images

```text
GET /api/projects/{project_id}/cache/images?q=Foundation
```

Fields:

- id
- install name
- basename
- base vmaddr
- size
- member
- analysis status

### Provider/Importer Queries

```text
GET /api/projects/{project_id}/cache/symbol-providers?symbol=_objc_msgSend
GET /api/projects/{project_id}/cache/symbol-importers?symbol=_objc_msgSend
```

Requirements:

- provider/importer image
- reexport chain
- route to implementation where possible

## Annotation Endpoints

### Create Annotation

```text
POST /api/projects/{project_id}/annotations
```

Kinds:

- label
- comment
- tag
- bookmark
- color
- manual_reference
- manual_function_bound
- manual_type

Response:

- annotation id
- transaction id
- updated display name/facts if applicable

### Undo/Redo

```text
POST /api/projects/{project_id}/undo
POST /api/projects/{project_id}/redo
```

Rules:

- annotation transactions first
- patch transactions only after future write-mode design

## Snapshot, Diff, And Export Endpoints

### Create Snapshot

```text
POST /api/projects/{project_id}/snapshots
```

Returns:

- snapshot id
- source identities
- included fact kinds
- completeness
- export/redaction policy

### Create Diff

```text
POST /api/diffs
```

Request:

```json
{
  "baseline_snapshot_id": "snapshot:old",
  "comparison_snapshot_ids": ["snapshot:new"],
  "matching_profile": "dyld_cache_build_diff",
  "scope": {
    "kind": "source_pair",
    "left_source_id": "source:old",
    "right_source_id": "source:new"
  }
}
```

### Diff Queries

```text
GET /api/diffs/{diff_id}/findings
GET /api/diffs/{diff_id}/matches
GET /api/diffs/{diff_id}/functions
GET /api/diffs/{diff_id}/runtime/{kind}
```

Rules:

- large diff result sets are paged
- every finding includes routes and evidence
- ambiguous matches are first-class records
- match pin/reject operations create transactions

### Export Evidence

```text
POST /api/diffs/{diff_id}/exports
GET /api/exports/{export_id}
```

Export profiles:

- project_manifest
- selected_function
- objc_surface
- cpp_abi
- kernel_surface
- dyld_cache_inventory
- diff_summary
- evidence_package
- diagnostics_redacted

Rules:

- exports are job-backed when large
- JSON is the canonical Damsel format
- Markdown/HTML/CSV/DOT exports are derived from canonical facts
- raw bytes, local paths, comments, and private strings are redacted by default

## MCP-Ready Document API

The UI API and CLI JSON should be structured so a future MCP server can expose
the same stable objects without adding a parallel model.

Initial read-only tool shapes:

- `get_project_summary`
- `list_sources`
- `search_project`
- `get_route`
- `get_byte_window`
- `get_function`
- `get_function_assembly`
- `get_function_graph`
- `get_xrefs`
- `get_objc_class`
- `get_objc_selector`
- `get_cpp_type`
- `get_kernel_member`
- `list_problems`
- `list_jobs`
- `export_selection`

Mutation-capable tools, if enabled later:

- `add_comment`
- `set_label`
- `add_tag`
- `add_manual_reference`
- `apply_problem_hint`

Rules:

- read-only tools are the default
- mutation tools require explicit permission
- mutation tools use annotation transactions and audit logs
- MCP responses include provenance and confidence, not just prose summaries
- raw byte windows are bounded, redacted from logs, and denied unless
  `source.read_bytes` is granted
- tool invocations emit `AutomationActivity` records when they mutate state

## Extension API Contracts

Extension-facing APIs are a layer over the same project contracts. They should
not expose parser internals or Swift UI objects.

Initial extension records:

- `PluginManifest`
- `ExtensionRun`
- `ExtensionCapability`
- `ExtensionFactEmission`
- `ExtensionProblemEmission`
- `ExtensionMetric`

Extension classes:

- loader
- sidecar_importer
- architecture_semantics
- analyzer
- renderer_exporter
- workbench_command

Rules:

- loaders can register sources, address spaces, segments, sections, entrypoint
  seeds, initial byte classifications, names, comments, and load problems
- sidecar importers can attach symbols, types, profiles, line tables, and
  mismatch problems to existing sources
- analyzers emit facts and problems but do not overwrite user annotations
- renderer/exporters read project facts and produce reports or external graph
  files
- Workbench command extensions invoke project APIs and cannot modify Swift view
  internals directly
- architecture semantics providers remain first-party until the semantic
  contract is stable

## Performance Endpoints

### Metrics

```text
GET /api/projects/{project_id}/metrics
```

Fields:

- analysis pass timings
- query timings
- fact counts
- object counts
- warning counts
- cache sizes
- source access handle counts
- response size samples
- index status
- job timing summaries
- recent slow queries
- Workbench-facing latency probes where available

### Diagnostics Export

```text
POST /api/projects/{project_id}/diagnostics/export
```

Must avoid:

- raw binary bytes by default
- private strings unless explicitly included

## Error Codes

Core:

- `project_not_found`
- `source_not_found`
- `object_not_found`
- `route_not_found`
- `stale_result_version`
- `invalid_cursor`
- `analysis_not_complete`
- `job_not_found`
- `job_not_cancellable`

Source:

- `file_not_found`
- `source_hash_changed`
- `unsupported_format`
- `unsupported_architecture`
- `malformed_macho`
- `incomplete_shared_cache`
- `unsupported_fileset`
- `sidecar_uuid_mismatch`

Analysis:

- `address_not_mapped`
- `function_conflict`
- `ambiguous_symbol`
- `unsupported_pointer_format`
- `analysis_cancelled`
- `analysis_timeout`

## CLI Mapping

Every major API object should have a CLI JSON equivalent:

- `damsel project status --json`
- `damsel project sources --json`
- `damsel project functions --json`
- `damsel project xrefs --json`
- `damsel project graph --json`
- `damsel objc classes --json`
- `damsel cpp types --json`
- `damsel kernel members --json`
- `damsel cache images --json`

The CLI is the easiest way to snapshot and stabilize contracts before the UI
depends on them.
