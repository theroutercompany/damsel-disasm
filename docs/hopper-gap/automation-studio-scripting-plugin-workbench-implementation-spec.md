# Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec

Status: implementation-grade planning spec. This is not an implementation
claim and it does not enable scripting, MCP, or plugins in the current product.

Purpose: define the analyst-facing automation and extension Workbench that
Damsel would need to compete with Hopper's Python scripting, MCP, SDK, plugins,
and native app integration while preserving Damsel's Apple-system focus,
project integrity, privacy, provenance, and performance. The existing
automation and plugin boundary specs define core APIs and safety principles.
This spec defines the product surface: Automation Studio, script runs, MCP
sessions, plugin manager, capability prompts, extension outputs, activity logs,
Apple-system recipes, validation, and rollout.

## Why This Spec Exists

Hopper's public materials make automation a first-class feature family:

- the homepage markets Python scripting support
- the homepage markets AI capabilities through an integrated MCP server
- the homepage markets an extensible SDK for file parsers and CPU backends
- the FAQ says MCP configuration lives in Hopper Help
- the FAQ says Python scripts can modify individual bytes
- public Python stubs exist for Hopper's scripting API
- public Hopper scripts demonstrate current-document access, byte reads,
  selection export, file-offset annotation, Yara annotation, stack-string
  annotation, import renaming, and saved byte extraction
- public SDK evidence in this pack shows loader, CPU, tool, document,
  disassembled-file, procedure, byte-type, transaction, and Python boundaries

Those facts imply a product requirement: automation is not a sidecar after the
GUI is done. It is a second way to use the same document model.

Damsel should not respond by embedding a scripting language too early or by
letting MCP mutate project internals directly. The right architecture is a
typed project API with multiple front ends:

- CLI JSON
- batch query runner
- native Automation Studio
- local MCP server
- future script runtime
- future plugin manager
- future SDK packages

Every surface should share:

- object ids
- routes
- paging
- provenance
- confidence
- permissions
- transactions
- problems
- diagnostics
- privacy redaction
- performance budgets

If each surface grows separately, Damsel will become brittle and impossible to
trust in exactly the workflows automation is supposed to improve.

## Companion Documents

Read this spec with:

- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)
- [API And Data Contracts](api-data-contracts.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
- [Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
- [Quality And Validation Plan](quality-validation-plan.md)

## Product Thesis

Automation is a Workbench capability, not a hidden back door.

Every automated action should be:

- route-aware
- capability-checked
- bounded
- cancellable
- logged
- provenance-labeled
- visible in activity history
- reproducible from structured inputs
- redacted by default when exporting diagnostics

The user should be able to answer:

- Which script, plugin, MCP client, or CLI command produced this fact?
- Which project/source/revision did it run against?
- What permissions did it request?
- What bytes, functions, selectors, classes, or kernel objects did it read?
- Did it mutate annotations, types, patch plans, or analysis hints?
- What facts did it emit?
- Which facts became stale when it ran?
- Can I undo the mutations?
- Can I rerun the workflow on another OS build or dyld cache?
- Can I export the evidence without leaking local paths or private bytes?

This is how Damsel can make automation feel powerful without making it unsafe.

## Non-Goals

Do not ship first:

- arbitrary Python execution inside the Workbench process
- arbitrary dynamic library loading into the Workbench process
- remote MCP listener by default
- scripts with unrestricted filesystem access
- scripts with unrestricted network access
- plugins that directly mutate SQLite tables
- plugins that bypass project transactions
- AI tools that write facts without user-visible provenance
- patch application through automation without explicit capability
- debugger launch/attach through automation without explicit human approval
- marketplace or auto-update for plugins
- private binary upload to external services

The first automation milestone should be local, deterministic, read-mostly,
and schema-driven.

## Workbench UX

### Automation Studio

Automation Studio is the native surface for:

- saved query recipes
- batch operations
- script packages
- MCP sessions
- plugin runs
- output artifacts
- activity log
- capability grants
- diagnostics

It should not be a generic code editor first. It should be a workflow console
for structured reverse-engineering automation.

Primary tabs:

- Recipes
- Runs
- MCP
- Scripts
- Plugins
- Capabilities
- Activity
- Outputs
- Diagnostics

### Recipes

Recipes are structured workflows that do not require arbitrary code.

Examples:

- export ObjC selector implementers and callers
- summarize all methods on a class
- export dyld shared-cache provider/importer graph
- list C++ vtable candidates with weak evidence
- summarize IOKit external methods
- map panic frames to KDK/dSYM-backed routes
- collect unresolved branch/call targets
- export pseudocode evidence for selected functions
- compare two Apple system artifacts

Recipe fields:

- id
- name
- description
- input schema
- output schema
- required capabilities
- default privacy policy
- query plan
- supported artifact kinds
- performance budget

Recipes should be the first "scripting" user experience because they are
testable and safe.

### Runs

Run list fields:

- status
- name
- actor
- started at
- duration
- project/source/revision
- capabilities used
- output count
- mutation count
- problem count
- cancelled/failed reason

Run detail shows:

- inputs
- resolved routes
- job graph
- logs
- emitted facts
- emitted problems
- mutations
- output artifacts
- diagnostics
- rerun button

### MCP Panel

The MCP panel should show:

- server status
- transport type
- active client sessions
- exposed tools
- tool schemas
- capability grants
- recent calls
- denied calls
- current project/session binding
- stop server button

Default:

- local stdio only
- no network listener
- read-only tools only
- one active project binding
- no hidden background server

The user should see when an AI/MCP client is connected and which project it can
inspect.

### Script Library

The script library is a future layer after stable APIs exist.

Script package metadata:

- name
- id
- version
- author
- description
- language
- entry point
- required Damsel API version
- required capabilities
- supported artifact kinds
- declared outputs
- declared mutations
- test fixtures
- trust state

Script states:

- installed
- disabled
- incompatible
- quarantined
- running
- failed

### Plugin Manager

Plugin manager handles extension packages:

- loaders
- sidecar importers
- architecture semantics providers
- analyzers
- renderers/exporters
- Workbench command providers
- recipe providers

Plugin manager should show:

- manifest
- version compatibility
- capabilities
- trust source
- sandbox mode
- run history
- emitted facts
- crashes/timeouts
- disable/uninstall controls

Early plugin support should prefer out-of-process JSON tools before dynamic
library ABI plugins.

## Data Model

### AutomationRecipe

Fields:

- `id`
- `name`
- `version`
- `description`
- `input_schema`
- `output_schema`
- `required_capabilities`
- `supported_artifact_kinds`
- `query_plan`
- `default_privacy_policy`
- `performance_budget`
- `owner`

### AutomationRun

Fields:

- `id`
- `project_id`
- `source_id`
- `source_revision_id`
- `actor`
- `entry_kind`
- `entry_id`
- `status`
- `inputs`
- `capabilities_granted`
- `started_at`
- `ended_at`
- `outputs`
- `mutations`
- `emitted_facts`
- `emitted_problems`
- `logs`
- `metrics`
- `diagnostics_policy`

Entry kinds:

- `recipe`
- `cli_batch`
- `mcp_tool`
- `script`
- `plugin`
- `workbench_command`

### CapabilityGrant

Fields:

- `id`
- `project_id`
- `actor`
- `capabilities`
- `scope`
- `expires_at`
- `created_at`
- `reason`
- `revoked_at`

Scopes:

- project
- source
- source revision
- route
- artifact kind
- command family
- single run
- session

### ScriptPackage

Fields:

- `id`
- `name`
- `version`
- `language`
- `entry_point`
- `manifest_path`
- `hash`
- `required_api_version`
- `required_capabilities`
- `declared_outputs`
- `declared_mutations`
- `trust_state`
- `install_state`
- `last_run_id`

### PluginPackage

Fields:

- `id`
- `name`
- `version`
- `kind`
- `manifest`
- `binary_or_script_path`
- `hash`
- `required_api_version`
- `capabilities`
- `sandbox_mode`
- `compatibility`
- `trust_state`
- `crash_count`
- `last_run_id`

Plugin kinds:

- loader
- sidecar importer
- architecture semantics
- analyzer
- renderer
- exporter
- command provider
- recipe provider

### ExtensionOutput

Fields:

- `id`
- `run_id`
- `kind`
- `route`
- `object_ids`
- `payload`
- `schema_version`
- `privacy_classification`
- `export_policy`

Output kinds:

- table
- graph
- report fragment
- evidence package
- annotation proposal
- patch proposal
- analysis hint proposal
- problem list
- file artifact

### EmittedFact

Fields:

- `id`
- `run_id`
- `fact_kind`
- `target_route`
- `payload`
- `confidence`
- `provenance`
- `expires_with_revision`
- `invalidates`

Automation-emitted facts should never pretend to be core analyzer facts. They
need actor/tool provenance.

## Capability Model

Capability families:

- `project.read`
- `source.read`
- `bytes.read.window`
- `bytes.read.bulk`
- `search.read`
- `function.read`
- `xref.read`
- `objc.read`
- `cpp.read`
- `kernel.read`
- `graph.read`
- `pseudocode.read`
- `debug.read`
- `annotation.write`
- `hint.write`
- `type.write`
- `patch.plan`
- `patch.apply`
- `patch.export`
- `debug.launch`
- `debug.attach`
- `debug.control`
- `filesystem.export`
- `network.access.future`

Default first wave:

- no mutation
- no bulk byte dumps
- no patch apply
- no debugger control
- no network access

Capability prompts should explain:

- what the tool wants
- why it wants it
- scope
- duration
- examples of data exposed
- whether mutations are undoable

## MCP Tool Design

### Tool Principles

MCP tools should:

- return structured JSON
- include object ids and routes
- include provenance and confidence
- page large results
- avoid raw bytes unless explicitly requested
- avoid free-form prose as primary output
- include problem records
- expose disabled/permission errors clearly

### First-Wave Tools

Read-only tools:

- `damsel_project_summary`
- `damsel_list_sources`
- `damsel_resolve_route`
- `damsel_search`
- `damsel_get_function`
- `damsel_get_assembly_window`
- `damsel_get_xrefs`
- `damsel_get_graph`
- `damsel_get_objc_class`
- `damsel_get_selector`
- `damsel_get_cpp_type`
- `damsel_get_kernel_symbol`
- `damsel_get_problem`
- `damsel_export_evidence_fragment`

Mutation tools later:

- `damsel_add_comment`
- `damsel_add_tag`
- `damsel_set_label`
- `damsel_apply_hint`
- `damsel_create_patch_plan`

Patch apply/debug control tools should be separate, gated, and disabled by
default.

### MCP Session Binding

An MCP server instance should bind to:

- project id
- source/revision scope
- tool allowlist
- capability grants
- redaction policy
- session id

The Workbench should show the binding visibly.

## Script Runtime Strategy

### Why Not Embed Arbitrary Python First

Arbitrary embedded scripting creates:

- filesystem risk
- network risk
- UI-thread risk
- ABI/runtime compatibility risk
- privacy risk
- reproducibility risk
- testability risk

Damsel should first provide:

- CLI JSON
- recipes
- local MCP
- out-of-process script runner
- typed SDK generated from schemas

An embedded scripting console can come later, after the API is stable.

### Script Runner

Out-of-process script runner:

- receives a capability-limited API endpoint
- receives project/session token
- has time and memory limits
- writes structured outputs
- cannot access internal database files
- cannot mutate without transaction API
- returns metrics and logs

This keeps scripts useful without letting them corrupt Workbench state.

## Plugin SDK Strategy

### Extension Boundary Order

Recommended order:

1. CLI/importer sidecar tools.
2. Analyzer sidecars emitting facts/problems.
3. Renderer/exporter sidecars emitting artifacts.
4. Recipe providers.
5. Architecture semantics plugins.
6. Loader plugins.
7. In-process performance plugins only if proven necessary.

The later steps are much riskier and need a stable API first.

### Plugin Manifest

Manifest fields:

- id
- name
- version
- vendor
- kind
- entry point
- supported platforms
- supported artifact kinds
- required API version
- required capabilities
- output schemas
- mutation schemas
- timeout policy
- memory policy
- trust policy

### Plugin Fact Emission

Plugins emit:

- facts
- problems
- annotations proposals
- runtime object proposals
- type proposals
- patch proposals
- report fragments

Every emitted item includes:

- plugin id
- plugin version
- run id
- input routes
- confidence
- schema version
- invalidation dependencies

## Apple-System Recipes

### ObjC Selector Audit

Inputs:

- selector name or pattern
- source scope
- confidence threshold

Outputs:

- selector record
- implementers
- callers
- message-send sites
- receiver class candidates
- unresolved/ambiguous sites
- evidence package

### Dyld Cache Provider Graph

Inputs:

- cache source
- image filter
- symbol/class/selector scope

Outputs:

- image dependency graph
- providers/importers
- reexports
- ObjC class ownership
- cache link problems
- exportable graph

### IOKit External Method Audit

Inputs:

- kernel collection/KDK profile
- class filter

Outputs:

- user-client classes
- external method tables
- selector numbers
- handler candidates
- entitlement/gate checks where detected
- weak evidence labels

### Panic Frame Pack

Inputs:

- panic log
- KDK/dSYM profile
- kernel collection

Outputs:

- frame routes
- symbolication evidence
- source context
- unresolved frames
- module/profile mismatch problems
- report fragment

### C++ Vtable Sweep

Inputs:

- image/source
- namespace/class pattern

Outputs:

- vtable candidates
- RTTI names
- virtual call sites
- constructor/destructor hints
- ambiguous slots
- confidence histogram

### Patch Plan Review

Inputs:

- patch plan id

Outputs:

- patchability
- original/proposed bytes
- impacted facts
- code-signing warnings
- export policy
- evidence summary

## Workbench Store Architecture

Add stores:

- `AutomationStudioStore`
- `AutomationRecipeStore`
- `AutomationRunStore`
- `McpSessionStore`
- `ScriptPackageStore`
- `PluginPackageStore`
- `CapabilityGrantStore`
- `ExtensionOutputStore`
- `AutomationActivityStore`
- `AutomationDiagnosticsStore`

Responsibilities:

`AutomationStudioStore`:

- active tab
- selected run
- selected output
- project binding

`AutomationRecipeStore`:

- recipe catalog
- input validation
- supported artifact filtering

`AutomationRunStore`:

- run lifecycle
- cancellation
- logs
- metrics
- output routing

`McpSessionStore`:

- server state
- client sessions
- tool list
- recent calls
- denied calls

`CapabilityGrantStore`:

- grants
- revocations
- prompt state
- expiration

`PluginPackageStore`:

- installed packages
- compatibility
- trust state
- crash state

## API Contracts

### List Recipes

```text
GET /api/projects/{project_id}/automation/recipes
```

### Run Recipe

```text
POST /api/projects/{project_id}/automation/runs
```

Request:

```json
{
  "entry_kind": "recipe",
  "entry_id": "objc-selector-audit",
  "inputs": {
    "selector": "viewDidLoad",
    "source_scope": "current_project"
  },
  "capability_grant_ids": ["grant-read-objc"]
}
```

### Get Run

```text
GET /api/projects/{project_id}/automation/runs/{run_id}
```

### Cancel Run

```text
POST /api/projects/{project_id}/automation/runs/{run_id}/cancel
```

### Start MCP Session

```text
POST /api/projects/{project_id}/automation/mcp/sessions
```

Request:

```json
{
  "transport": "stdio",
  "tool_allowlist": ["damsel_project_summary", "damsel_search"],
  "capability_scope": "read_only",
  "redaction_policy": "default"
}
```

### Install Plugin Package

```text
POST /api/projects/{project_id}/automation/plugins/install-plan
```

Early install should create an install plan, not blindly install.

### Grant Capability

```text
POST /api/projects/{project_id}/automation/capabilities/grants
```

Request:

```json
{
  "actor": {
    "kind": "mcp_session",
    "id": "mcp-1"
  },
  "capabilities": ["project.read", "function.read"],
  "scope": {
    "kind": "project",
    "id": "project-1"
  },
  "duration": "session",
  "reason": "Allow read-only MCP inspection."
}
```

## Trust And Privacy

### Trust States

Packages and sessions:

- builtin
- local_user
- project_local
- signed_known_vendor
- unsigned
- quarantined
- blocked

### Redaction Classes

Data classes:

- public metadata
- project metadata
- local path
- source bytes
- user annotation
- debug/register/memory
- launch argument/environment
- private source path
- exported artifact

Default automation diagnostics should not include source bytes, environment
values, raw memory, or local absolute paths.

## Provenance Rules

Every automation output must include:

- actor
- tool/script/plugin id
- version
- run id
- input routes
- source revision
- timestamp
- capability grant ids
- confidence
- invalidation dependencies

Workbench should display provenance wherever automation output appears:

- comments
- tags
- problem records
- report fragments
- graph overlays
- runtime object candidates
- patch proposals

## Activity Log

Activity events:

- recipe run started
- recipe run completed
- MCP session started
- MCP tool called
- MCP call denied
- script installed
- script run failed
- plugin installed
- plugin disabled
- capability granted
- capability revoked
- mutation applied
- run cancelled
- output exported

The log is not optional. Automation without visible activity erodes trust.

## Performance And Responsiveness

Budgets:

- open Automation Studio: under 150 ms with cached metadata
- list recipes: under 100 ms
- start read-only run: under 250 ms
- cancel run acknowledgement: under 100 ms
- MCP tool schema listing: under 100 ms
- render run detail: under 200 ms for normal runs
- stream large output progressively

Large outputs:

- page tables
- cap graph node count
- export large evidence as files
- avoid loading whole source bytes
- summarize first, expand on demand

## Implementation Phases

### Phase 1: Schema-Driven CLI And Recipes

Deliver:

- generated API schemas
- recipe catalog format
- read-only recipe runner
- run records
- output records
- activity log

Acceptance:

- ObjC selector audit recipe runs from CLI JSON and Workbench
- outputs include routes and provenance

### Phase 2: Automation Studio

Deliver:

- native Automation Studio shell
- Recipes tab
- Runs tab
- Outputs tab
- Activity tab
- cancellation UI
- diagnostics view

Acceptance:

- user can rerun/cancel/read outputs without a terminal

### Phase 3: Read-Only MCP

Deliver:

- local stdio MCP server
- read-only tools
- Workbench MCP panel
- capability scope display
- denied-call log

Acceptance:

- MCP client can read project summary/search/function details
- Workbench shows connected client and recent calls

### Phase 4: Audited Annotation Mutation

Deliver:

- comment/tag/label mutation APIs
- transaction records
- undo/redo
- activity log entries
- capability prompts

Acceptance:

- automation mutation is visible, undoable, and provenance-labeled

### Phase 5: Script Package Runner

Deliver:

- script manifest
- out-of-process runner
- capability-limited project API token
- logs/metrics
- package install plan

Acceptance:

- script cannot access internal DB directly
- timeout/cancel works

### Phase 6: Plugin Sidecar Runtime

Deliver:

- plugin manifest
- sidecar analyzer/exporter protocol
- emitted fact/problem schema
- crash isolation
- compatibility checks

Acceptance:

- toy plugin emits fact/problem records with provenance

### Phase 7: Apple-System Recipe Pack

Deliver:

- ObjC selector audit
- dyld cache provider graph
- IOKit external method audit
- panic frame pack
- C++ vtable sweep
- patch plan review

Acceptance:

- each recipe has fixtures, schemas, outputs, and evidence export

### Phase 8: Advanced Capabilities

Deliver later:

- patch plan creation
- type hint mutation
- analysis job control
- debugger read-only session summaries

Keep patch apply and debugger control separate, gated, and disabled by default.

## Validation Matrix

### Fixtures

Required fixtures:

- recipe catalog validation
- recipe run success
- recipe run partial failure
- run cancellation
- output pagination
- activity log snapshot
- MCP read-only tool call
- MCP denied mutation call
- capability grant/revoke
- comment mutation transaction
- undo automation comment
- script timeout
- plugin crash isolation
- plugin incompatible version
- emitted fact provenance
- ObjC selector audit recipe
- dyld cache provider graph recipe
- panic frame recipe
- patch review recipe

### UI Tests

Required UI tests:

- open Automation Studio
- run recipe
- cancel run
- inspect output
- start MCP session
- see MCP call log
- deny capability prompt
- grant read-only capability
- install plugin plan review
- disable plugin

### Security Tests

Required tests:

- MCP defaults to local stdio
- mutation denied without capability
- bulk byte read denied without capability
- plugin cannot write DB directly
- script runner token scope enforced
- diagnostics redact local paths/source bytes
- activity log records mutation

### Performance Tests

Required tests:

- recipe list latency
- run start latency
- cancellation latency
- MCP schema list latency
- output pagination latency
- plugin timeout latency

## Acceptance Criteria

Damsel should not claim Hopper-competitive automation until:

- CLI JSON, Workbench recipes, MCP tools, scripts, and plugins share one typed
  project API
- Automation Studio shows runs, outputs, capabilities, activity, and diagnostics
- MCP is local/read-only by default and visibly bound to a project
- automation mutations are transaction-backed and undoable
- outputs include routes, object ids, confidence, and provenance
- plugin/script runs are cancellable and isolated
- capability prompts are scoped and understandable
- Apple-system recipes cover ObjC, dyld cache, C++/IOKit, panic/KDK, and patch
  review workflows
- diagnostics redact sensitive data by default
- tests cover denial paths, not only happy paths

## Summary

Hopper proves that scriptability, MCP, and SDK extension are part of a mature
reverse-engineering workstation. Damsel's opportunity is to make those surfaces
more coherent and safer by building them on one typed, route-backed,
provenance-aware project API.

Automation Studio should be the native face of that API: recipes first, MCP
second, audited mutation after that, out-of-process scripts and sidecar plugins
later. That order gives analysts useful automation early while preserving the
trust boundaries needed for Apple-system binaries, private artifacts, patching,
debugging, and evidence exports.
