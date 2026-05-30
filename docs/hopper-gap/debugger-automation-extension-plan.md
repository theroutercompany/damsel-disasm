# Debugger, Automation, Extension, And MCP Plan

Status: Long-range architecture and product plan. This document defines how
Damsel should approach Hopper-adjacent debugger, scripting, plugin, SDK, and
MCP/AI capabilities without compromising the static-analysis core.

Purpose: Hopper publicly exposes debugging, Python scripting, SDK/plugin
extension points, and MCP/AI integration. Damsel does not need to clone all of
that immediately, but it does need an architecture that can grow into those
surfaces without creating parallel data models.

Implementation companions:

- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
  defines CLI JSON automation, batch operations, read-only MCP, audited
  annotation mutation, capability checks, privacy policy, and Workbench
  activity logs.
- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
  defines loader, sidecar importer, architecture semantics, analyzer,
  renderer/exporter, and Workbench command extension boundaries.
- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
  defines analysis snapshots, object fingerprints, diff projects, evidence
  packages, report exports, redaction policy, and future BinExport-like
  interoperability.
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
  defines the safe byte-editing, patch preview, assembler/NOP, source revision,
  invalidation, and copy-on-write export path.
- [Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
  defines the local debugger path for runtime module matching, route-backed
  breakpoints, current-PC overlays, registers, frames, and read-only memory.

## Thesis

The safe order is:

1. Stable project/object/route API.
2. Read-only automation over project facts.
3. Audited annotation mutation.
4. MCP-style local tool surface over the same API.
5. Loader/analyzer/exporter plugin boundaries.
6. LLDB debugger bridge.
7. Patch/binary mutation, only after undo and project persistence are mature.

Every automation surface should use the same project database, object ids,
routes, annotations, provenance, and transaction log as the Workbench UI and
CLI. If scripting or MCP gets a separate internal API, the product will fork.

## Non-Goals

Do not start with:

- arbitrary Python execution inside the app
- binary patching
- remote debugging service
- kernel debugging
- plugin ABI exposing raw parser internals
- MCP tools that can mutate without audit
- debugger UI before static-to-runtime address mapping is trustworthy

## Shared Capability Model

All dynamic/extension surfaces should declare capabilities.

Capability classes:

- read_project
- read_source_metadata
- read_bytes
- read_strings
- read_instructions
- read_graph
- read_xrefs
- read_runtime_objects
- read_debug_info
- export_reports
- write_annotations
- apply_hints
- run_analysis
- attach_sidecars
- debug_process
- read_process_memory
- write_process_memory
- patch_binary

Default profile:

- read-only project facts
- no raw binary export unless user grants it
- no process access
- no binary mutation

Mutation profile:

- write annotations
- apply problem hints
- run targeted analysis
- explicit audit log

Dangerous profile:

- debugger
- process memory reads
- binary patching
- disabled by default
- requires user confirmation per session

## Scripting Strategy

### Script Surface

Initial scriptable objects:

- project
- source
- address
- route
- byte range
- function
- basic block
- instruction
- xref
- symbol
- string
- ObjC class/selector/method
- C++ type/vtable
- kernel member/kext
- annotation
- problem

Initial read operations:

- open project
- list sources
- list functions
- get function detail
- get assembly page
- search
- get xrefs to/from
- list strings
- list ObjC selectors/classes
- list C++ vtables/types
- list kernel members/kexts
- list problems
- export graph/report data

Initial write operations:

- add label
- add local label
- add comment
- add tag
- add bookmark
- add manual reference
- apply problem hint

Do not allow scripts to modify input bytes in the first scripting version.

### Language Choice

Options:

- structured CLI batch mode
- embedded scripting language
- Python bridge
- JavaScript/TypeScript plugin runtime
- Starlark-like deterministic DSL

Recommendation:

Start with structured CLI/API batch workflows before embedding a language.
This forces the project API to become stable and testable. Add embedded
scripting only after object contracts and permission models are solid.

Why not Python first:

- packaging/runtime complexity in a macOS app
- arbitrary execution risk
- version compatibility churn
- hard-to-sandbox mutations

Why Python may still matter later:

- reverse-engineering users expect it
- Hopper exposes Python
- community workflows often use Python

Compromise:

- first expose stable JSON/HTTP/CLI contracts
- later expose Python bindings generated over those contracts
- keep mutation through transaction APIs

### Script Audit Log

Every mutating script action records:

- script id/name
- actor
- timestamp
- command
- target object/range
- old value
- new value
- transaction id
- invalidated facts
- queued jobs

Script-created annotations are visible in UI with origin metadata.

## MCP / AI Integration

### Product Position

Hopper now publicly markets MCP/AI. Damsel should not make AI the product, but
it should expose project facts in a tool-friendly way.

AI/MCP should help answer:

- what is this function?
- where is this selector used?
- what calls this import?
- which cache image provides this symbol?
- what are unresolved problems in this function?
- summarize this panic backtrace
- list likely virtual call targets
- explain why this KDK was rejected

AI/MCP should not:

- fabricate analysis facts
- hide confidence/provenance
- mutate silently
- send private binaries externally by default
- become the only UI for a feature

### Tool Families

Read-only tools:

- `get_project_summary`
- `list_sources`
- `search_project`
- `resolve_route`
- `get_function`
- `get_assembly`
- `get_cfg`
- `get_xrefs`
- `list_strings`
- `get_objc_class`
- `get_objc_selector`
- `get_cpp_type`
- `get_vtable`
- `get_kernel_member`
- `list_problems`
- `symbolicate_panic`
- `export_selection`

Mutation tools:

- `add_comment`
- `set_label`
- `add_tag`
- `add_bookmark`
- `add_manual_reference`
- `apply_problem_hint`
- `start_analysis`
- `cancel_job`

Dangerous tools, later:

- debugger attach
- read process memory
- set breakpoint
- patch bytes
- assemble instruction

### MCP Response Rules

Responses must include:

- object ids
- routes
- provenance
- confidence
- stale/partial flags
- related problems
- source privacy classification

Avoid returning:

- large raw byte ranges by default
- private strings unless requested
- whole project dumps
- ungrounded prose without object references

## Plugin / SDK Strategy

### Plugin Boundary Types

Potential plugin classes:

- loader plugin
- sidecar importer
- analysis pass
- symbolication provider
- runtime recognizer
- renderer/exporter
- report generator
- search/ranking extension

Do not expose:

- internal mutable database handles
- raw parser lifetimes
- unstable Rust structs as long-term ABI
- Workbench-specific Swift internals

### Loader Plugins

Loader plugin responsibilities:

- identify supported input
- extract source metadata
- define address spaces
- expose segments/sections/ranges
- expose raw bytes through controlled provider
- emit initial facts

Example future plugins:

- alternate Apple firmware container
- custom extracted cache format
- private lab symbol map

### Analyzer Plugins

Analyzer plugin responsibilities:

- declare required facts
- declare produced facts
- run deterministically over selected scope
- emit facts/problems
- avoid direct annotation writes
- report metrics

Analysis plugins should be sandboxable and cancellable.

### Export Plugins

Export plugin responsibilities:

- consume stable query APIs
- write reports
- write graph files
- write JSON packages
- later write BinExport-like artifacts

Export plugins must honor privacy controls.

### SDK Versioning

Every plugin API needs:

- SDK version
- required Damsel schema version
- capability declarations
- feature flags
- compatibility errors
- migration story

## Debugger Strategy

### Why Debugger Comes Later

A debugger is useful only when static and dynamic identities align.

Prerequisites:

- project source identity
- address-space model
- runtime slide model
- function route resolution
- breakpoint route mapping
- symbolication profile
- stable UI route/navigation state
- clear permission model

Without those, debugger state becomes an isolated pane that cannot enrich the
static database.

### LLDB Bridge

Use LLDB as the transport where practical.

Bridge responsibilities:

- launch or attach process
- list modules/images
- compute slides
- map runtime addresses to project sources
- set/remove breakpoints
- read registers
- read memory
- get backtrace
- map current frame to function/route
- pause/resume/step

Initial local-only target:

- user-selected executable
- no remote debug server
- no kernel debugging
- no process memory writes

### Static-To-Dynamic Mapping

Mapping inputs:

- Mach-O UUID
- install name/path
- runtime load address
- slide
- dyld shared cache UUID
- image vmaddr
- project source registry

Mapping outputs:

- runtime address -> project route
- project address -> runtime address candidates
- breakpoint route -> LLDB breakpoint location
- frame -> function/source/line if available

Problems:

- binary mismatch
- source not in project
- slide unknown
- dSYM mismatch
- dyld cache mismatch
- optimized/inlined frame ambiguity

### Debug UI

Surfaces:

- debug session panel
- process/module list
- registers
- stack frames
- breakpoints
- memory viewer
- current instruction marker
- runtime/static address toggle

Rules:

- debug markers overlay assembly/graph
- breakpoints are route-backed
- current frame follows route navigation
- process errors appear in problem/log panel
- stopping debugger does not discard static annotations

### Safety

Debugger actions can surprise users and hit platform protections.

Rules:

- explicit user action to launch/attach
- clear indication of target process
- no background attach
- handle SIP/entitlement failures honestly
- no process memory writes in first version
- never debug system processes by default

## Patching And Binary Mutation

Patching is a Hopper feature but should be delayed.

Prerequisites:

- project persistence
- byte classification map
- patch transaction model
- undo/redo
- binary writer/exporter
- code-signing warnings
- invalidation/reanalysis
- opt-in write mode

Patch objects:

- original byte range
- new byte range
- source hash precondition
- assembler text, if generated
- transaction id
- affected facts
- reanalysis result

Rules:

- never patch shared caches or kernel collections in place
- write new output files only
- make code-signing breakage explicit
- require explicit preview/confirm
- patching is separate from ordinary annotation undo

## Security And Privacy

Threats:

- script exfiltrates private binary strings
- plugin reads arbitrary files
- MCP tool sends sensitive facts externally
- debugger attaches to unintended process
- patcher corrupts input
- extension crashes app or corrupts project

Controls:

- capability declarations
- per-session permissions
- audit log
- redacted diagnostics
- no raw bytes by default
- sandboxed plugin execution where possible
- transaction preconditions
- explicit dangerous-action confirmation

## Workbench Integration

Automation should be visible:

- Scripts panel
- MCP/tool activity log
- plugin manager later
- debug session panel
- transaction history
- permission prompts
- problem entries for failed tools

The UI should show when a fact or annotation came from:

- analyzer
- imported debug info
- user
- script
- plugin
- MCP tool
- debugger observation

## CLI Integration

CLI commands:

- `damsel project query`
- `damsel project script`
- `damsel project annotate`
- `damsel project transactions`
- `damsel project mcp-server`, later
- `damsel debug launch`, later
- `damsel debug attach`, later
- `damsel plugin list`, later

Rules:

- JSON first
- stable object ids
- no terminal-only features
- Workbench and CLI use the same project store

## Validation

### Automation Gates

- read-only script query fixture
- annotation script transaction fixture
- audit log fixture
- permission denial fixture
- large search paging fixture
- redacted diagnostics fixture

### MCP Gates

- tool schema snapshot
- read-only project summary fixture
- xref query fixture
- mutation permission denial fixture
- mutation audit fixture
- private-string redaction fixture

### Plugin Gates

- plugin manifest validation
- incompatible SDK version error
- analyzer plugin emits facts/problems
- plugin cancellation
- plugin crash isolation

### Debugger Gates

- local launch smoke
- UUID/slide mapping test
- breakpoint route mapping test
- register read test
- backtrace route mapping test
- mismatch error test

### Patching Gates

Only after patching is in scope:

- patch preview fixture
- write-copy-only test
- source hash precondition test
- undo transaction test
- reanalysis invalidation test
- code-signing warning test

## Milestones

### A0: Stable Query API

- object ids
- routes
- project search
- function/xref queries
- provenance in responses

### A1: Read-Only Batch Automation

- CLI/API batch mode
- report export
- script fixtures

### A2: Audited Annotation Mutation

- comments/labels/tags/manual refs
- transaction log
- undo/redo
- script origin display

### A3: MCP Prototype

- local read-only tools
- tool schema snapshots
- no mutation by default

### A4: Plugin Boundary Spike

- manifest
- loader/analyzer/exporter capability shapes
- SDK versioning

### A5: Local LLDB Bridge Spike

- launch/attach local sample
- map frame to route
- breakpoint overlay

### A6: Debugger UX

- session panel
- registers/frames/breakpoints
- error handling

### A7: Patch Design Only

- policy doc
- transaction model
- writer/exporter spike
- no default write mode

## Completion Criteria

This feature family is not complete until:

- scripts/MCP/plugins use stable project objects
- mutating actions are audited and undoable
- read-only automation can answer common reverse-engineering questions
- MCP responses include provenance/confidence
- plugin boundaries do not expose unstable internals
- LLDB can map runtime frames to static routes
- debugger errors are understandable
- patching remains impossible unless explicitly enabled by design
