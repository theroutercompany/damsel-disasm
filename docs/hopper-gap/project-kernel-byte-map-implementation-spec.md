# Project Kernel And Typed Byte Map Implementation Spec

Status: implementation architecture spec. This is not implemented yet.

Purpose: turn the deeper Hopper research into an implementation-ready subsystem
plan for Damsel's next major architecture move: a project/document kernel and
typed byte map that can support Hopper-like UX, Apple-system-binary depth, and
future Workbench, CLI, scripting, MCP, decompiler, debugger, and patching
features without rebuilding the foundation later.

Primary conclusion: the next serious Hopper-gap work should not be another
single parser or another isolated view. It should be a shared Rust project
kernel that owns sources, address spaces, object ids, byte classifications,
facts, annotations, routes, jobs, problems, and deterministic snapshots. Every
other competitive feature should hang from that kernel.

Semantic editing companion:
[Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec](semantic-editing-reanalysis-hints-implementation-spec.md)
expands the annotation portion of this kernel into concrete semantic edit
records, suggestions, review states, conflict handling, stale-route repair,
reanalysis impacts, Workbench/CLI/API flows, and Apple runtime/XNU bindings.

## Why This Spec Exists

The first Damsel architecture docs correctly named a future analysis database,
but the deeper Hopper pass raises the priority of one layer: Hopper's core
product behavior is a mutable document model with typed bytes. The app can show
assembly, CFG, pseudocode, hex, inspectors, tags, xrefs, and scripts because
there is a shared document object underneath those views.

Damsel's current implementation is already useful, but it is still mostly
point-query oriented:

- Load an image.
- Ask for symbols.
- Ask for a disassembly window.
- Optionally include analysis facts in that response.
- Render those response objects in CLI, web UI, or Workbench.

That is not enough for a Hopper-competitive app because the user needs to build
and preserve understanding across many jumps, edits, sources, and sessions:

- mark this address as code
- mark this range as data
- name this target
- tag this function with the owning ObjC class
- add a manual xref
- keep a back/forward route stack
- view the same selected object in assembly, graph, hex, inspector, search, and
  future scripts
- resume the project later
- prove which facts came from Mach-O, dyld cache, ObjC metadata, KDK/dSYM,
  Capstone, heuristics, or a user override

This spec defines that missing center.

## Goals

Implement a shared project kernel that can:

- represent a reverse-engineering document independent of any single UI view
- ingest existing `BinaryImage`, `ProjectedBinaryImage`, `SharedCache`, ObjC,
  dyld, symbol, section, import, export, and disassembly facts
- represent every address with an explicit address space
- assign stable object ids to source, ranges, sections, symbols, functions,
  instructions, runtime objects, xrefs, problems, and annotations
- classify bytes and ranges as code, procedure, string, data, metadata,
  fixup, padding, unknown, or Apple-specific overlay ranges
- persist or snapshot facts with provenance and confidence
- accept user overrides through undoable transactions
- expose project queries for CLI, local UI server, Workbench, scripts, and
  future MCP
- stay compatible with existing CLI commands and the current Workbench while
  new project APIs land incrementally

## Non-Goals For The First Implementation

Do not implement these in the first branch:

- full SQLite project persistence
- binary patch export
- embedded Python or JavaScript scripting
- MCP server
- decompiler IR persistence
- local LLDB bridge
- remote debugger server
- graph layout engine
- complete Workbench redesign
- kernel collection loader if not already available
- Swift metadata recovery beyond name/demangle facts

The project kernel must reserve space for those features, but it should not try
to ship them all in the first implementation wave.

## Current Code Anchors

This spec is grounded in current repo state as of this pass.

### `damsel-core`

Current strengths:

- `damsel-core/src/lib.rs` already exports typed analysis records instead of
  loose JSON.
- Important current types include `BinaryImage`, `ProjectedBinaryImage`,
  `SharedCache`, `ObjcMetadata`, `DyldMetadata`, `DisassemblyAnalysis`,
  `DecodedInstruction`, `BasicBlock`, `ControlFlowEdge`, `DisassemblyObjcUse`,
  `DisassemblyImportUse`, `DisassemblyCacheLinkUse`, jump-table records,
  recovered-value records, symbolication records, segments, sections, imports,
  exports, stubs, and shared-cache models.
- `damsel-core/src/model.rs` already centralizes many product-facing structs.

Current limitation:

- There is no `AnalysisProject`, no durable source registry, no explicit
  address-space registry, no stable object id namespace, no typed byte map, no
  route model, no annotation transaction model, and no project-level problem or
  job model.

### `damsel-macho`

Current strengths:

- Existing loader/disassembly/ObjC/dyld/shared-cache modules already know how
  to build rich facts from Mach-O and dyld cache data.
- Shared-cache support is an advantage Damsel should preserve and lift into the
  project model instead of replacing.

Current limitation:

- Analysis facts are attached to `BinaryImage` outputs or command-specific
  workflows rather than a project-wide database with provenance, source
  identity, and address-space identity.

### `damsel-cli`

Current strengths:

- The CLI already has stable point-query commands and JSON/text output.
- The local UI server already proves a Rust server can be the analysis source
  of truth for Workbench.

Current limitation:

- `damsel-cli/src/ui.rs` stores uploaded images in an in-memory `ImageStore`
  keyed by a small generated image id.
- The HTTP API is image-centric: `/api/images`, symbol suggestions, and
  disassembly windows.
- There are no project endpoints, no background job objects, no paging, no
  annotations, no route objects, and no persisted source registry.

### `damsel-app`

Current strengths:

- Workbench already launches `damsel-cli ui --no-open --port 0`.
- `DamselAPIClient`, `WorkbenchStore`, and Swift models keep the native app
  separate from decoding internals.
- Virtualized instruction rendering exists.

Current limitation:

- `WorkbenchStore` is transient:
  `loadedImage`, `disassembly`, symbol suggestions, target controls, and
  selected instruction address.
- Loading a file reads bytes into Swift `Data` and uploads the whole file.
- Jumping to an address changes the disassembly target and reruns a decode
  request instead of navigating a shared project route.

## Target Architecture Overview

The project kernel should add these layers:

```text
Workbench / CLI / scripts / future MCP
        |
Project API contracts
        |
AnalysisProject
        |
+----------------------+---------------------+----------------------+
| Source Registry      | Address Spaces      | Object Id Registry   |
+----------------------+---------------------+----------------------+
| Byte Map             | Fact Store          | Annotation Store     |
+----------------------+---------------------+----------------------+
| Route Stack          | Job Store           | Problem Store        |
+----------------------+---------------------+----------------------+
        |
Adapters from existing BinaryImage, ProjectedBinaryImage, SharedCache,
ObjC metadata, dyld metadata, disassembly analysis, symbolication sources
```

The kernel should be pure Rust model and query code first. It should not depend
on Swift, HTTP, or local UI concerns.

## New Rust Modules

### `damsel-core/src/project.rs`

Owns:

- `AnalysisProject`
- `AnalysisProjectId`
- `ProjectSchemaVersion`
- `ProjectSummary`
- `ProjectSettings`
- `ProjectSnapshot`

Responsibilities:

- root object for all project state
- deterministic snapshot serialization
- schema version storage
- analyzer version storage
- source, address-space, fact, byte-map, route, annotation, job, and problem
  collections
- read-only query facade for common project operations

Do not put Mach-O parsing here.

### `damsel-core/src/source.rs`

Owns:

- `AnalysisSource`
- `AnalysisSourceId`
- `AnalysisSourceKind`
- `AnalysisSourceRole`
- `AnalysisSourceRelationship`
- `SourceAvailability`
- `SourcePrivacyClass`
- `SourceContentIdentity`

Responsibilities:

- describe inputs and sidecars
- model parent/child relationships such as dyld cache -> projected image or
  kernel collection -> fileset member
- separate display name from private absolute path
- store content hash, UUID, build version, architecture, and platform where
  known

### `damsel-core/src/address.rs`

Owns:

- `AddressSpace`
- `AddressSpaceId`
- `AddressSpaceKind`
- `AnalysisAddress`
- `AnalysisAddressRange`
- `AddressConversion`
- `AddressDisplayStyle`

Responsibilities:

- prevent naked `u64` addresses from leaking into project APIs
- convert between file offsets, image VM addresses, cache VM addresses,
  projected image VM addresses, kernel collection addresses, member addresses,
  and future runtime slid addresses
- return typed conversion errors

### `damsel-core/src/object.rs`

Owns:

- `AnalysisObjectId`
- `AnalysisObjectKind`
- `AnalysisObjectRef`
- `ObjectPath`

Responsibilities:

- stable identity for project-visible things
- support route and API references
- avoid ambiguous address-only identity

Object kinds should include at least:

- project
- source
- address space
- segment
- section
- symbol
- import
- export
- string
- byte range
- instruction
- function
- basic block
- control-flow edge
- call edge
- xref
- ObjC class
- ObjC category
- ObjC protocol
- ObjC selector
- ObjC method
- C++ type
- C++ vtable
- dyld binding
- chained fixup
- shared-cache image
- shared-cache edge
- kernel object
- type record
- annotation
- problem
- job

Unused variants can be defined conservatively, but API docs must not imply they
are fully populated until implemented.

### `damsel-core/src/facts.rs`

Owns:

- `AnalysisFact`
- `AnalysisFactId`
- `AnalysisFactKind`
- `FactSubject`
- `FactValue`
- `FactProvenance`
- `FactConfidence`
- `AnalysisPassId`
- `AnalyzerIdentity`
- `FactInvalidation`

Responsibilities:

- represent assertions from parsers, disassemblers, heuristics, symbolication
  sources, and user overrides
- store provenance and confidence
- make analysis explainable
- support reanalysis and invalidation without destroying user edits

Fact kinds should include:

- segment defined
- section defined
- symbol defined
- import defined
- export defined
- string defined
- byte classification inferred
- instruction decoded
- function start inferred
- function range inferred
- basic block defined
- control-flow edge inferred
- call edge inferred
- xref inferred
- ObjC class defined
- ObjC method defined
- ObjC selector defined
- dyld binding defined
- chained fixup defined
- cache dependency defined
- cache provider/importer edge defined
- problem reported
- user override applied

### `damsel-core/src/byte_map.rs`

Owns:

- `ByteMap`
- `ByteRangeClassification`
- `ByteClassificationKind`
- `ByteClassificationLayer`
- `ByteClassificationSource`
- `ByteClassificationConflict`
- `FeatureMapTile`

Responsibilities:

- store typed ranges in address-space-aware form
- distinguish inferred classification from user override
- support overlapping overlay categories
- generate feature-map tiles
- answer "what is at this address" for hex, assembly, inspector, and scripts

This module is the immediate Damsel answer to Hopper's typed-byte model.

### `damsel-core/src/route.rs`

Owns:

- `WorkbenchRoute`
- `RouteTarget`
- `RouteSelection`
- `NavigationStack`
- `RouteOrigin`

Responsibilities:

- represent navigable targets across all project surfaces
- power back/forward navigation
- decouple UI selection from disassembly request shape

Targets should include:

- address
- range
- source
- segment
- section
- symbol
- function
- basic block
- instruction
- operand
- string
- xref
- ObjC object
- C++ object
- cache image
- kernel object
- problem
- search result

### `damsel-core/src/annotation.rs`

Owns:

- `UserAnnotation`
- `AnnotationKind`
- `AnnotationTarget`
- `AnnotationTransaction`
- `AnnotationActor`
- `UndoRecord`
- `RedoRecord`

Responsibilities:

- record names, comments, inline comments, tags, colors, bookmarks, manual refs,
  operand formats, local labels, register aliases, stack variable names, and
  future patch intents
- provide undo/redo transaction boundaries
- support UI, CLI, script, and future MCP mutation origins

### `damsel-core/src/problem.rs`

Owns:

- `ProblemRecord`
- `ProblemSeverity`
- `ProblemKind`
- `ProblemStatus`
- `ProblemHint`

Responsibilities:

- project-level diagnostics
- ambiguous analysis feedback
- missing source/symbolication warnings
- parse failures
- invalid address conversions
- jump-table uncertainty
- ObjC/C++/Swift metadata contradictions
- user suppressions and hints

### `damsel-core/src/job.rs`

Owns:

- `AnalysisJob`
- `AnalysisJobId`
- `AnalysisJobKind`
- `AnalysisJobStatus`
- `AnalysisJobProgress`
- `AnalysisJobEvent`
- `CancellationTokenId`

Responsibilities:

- staged analysis progress
- cancellation
- partial results
- diagnostics
- future Workbench progress UI

## Core Model Contracts

### Project

Suggested shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisProject {
    pub id: AnalysisProjectId,
    pub schema_version: ProjectSchemaVersion,
    pub analyzer: AnalyzerIdentity,
    pub display_name: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub sources: Vec<AnalysisSource>,
    pub address_spaces: Vec<AddressSpace>,
    pub objects: Vec<AnalysisObjectRecord>,
    pub facts: Vec<AnalysisFact>,
    pub byte_map: ByteMap,
    pub annotations: Vec<UserAnnotation>,
    pub problems: Vec<ProblemRecord>,
    pub jobs: Vec<AnalysisJob>,
}
```

Wave 1 can omit timestamps or set them only in non-deterministic status output.
Deterministic snapshots should avoid wall-clock fields unless the test harness
normalizes them.

### Project Ids

Rules:

- Opaque to clients.
- Stable for deterministic snapshots.
- Do not include private absolute paths.
- Derived from source content identity, source kind, architecture, platform, and
  schema version when possible.
- For memory uploads without content hash, use a generated id but mark the
  snapshot as non-deterministic unless bytes are hashed.

Suggested ids:

```text
project:<short-hash>
source:<short-hash>
aspace:<source-id>:image-vmaddr
object:<kind>:<stable-key>
fact:<short-hash>
annotation:<short-hash-or-sequence>
problem:<short-hash>
job:<sequence>
```

The exact string format is not API-stable. Clients must treat ids as opaque.

### Source

Suggested shape:

```rust
pub struct AnalysisSource {
    pub id: AnalysisSourceId,
    pub kind: AnalysisSourceKind,
    pub role: AnalysisSourceRole,
    pub display_name: String,
    pub path_display: Option<String>,
    pub content_identity: Option<SourceContentIdentity>,
    pub architecture: Option<Architecture>,
    pub platform: Option<Platform>,
    pub parent: Option<AnalysisSourceId>,
    pub relationship: Option<AnalysisSourceRelationship>,
    pub availability: SourceAvailability,
    pub privacy: SourcePrivacyClass,
}
```

Initial source kinds:

- `standalone_macho`
- `memory_macho`
- `projected_cache_image`
- `dyld_shared_cache`

Reserved future source kinds:

- `driver_cache`
- `kernel_cache`
- `kernel_collection`
- `fileset_member`
- `kdk`
- `dsym`
- `panic_log`
- `external_symbol_map`
- `patched_revision`

Initial source roles:

- `primary_input`
- `container`
- `member`
- `projection`
- `symbolication`
- `diagnostic`

### Address Spaces

Required first address-space kinds:

- `file_offset`
- `image_vmaddr`
- `cache_vmaddr`
- `projected_image_vmaddr`

Reserved future kinds:

- `kernel_collection_vmaddr`
- `fileset_member_vmaddr`
- `runtime_vmaddr`
- `panic_frame_vmaddr`

Rules:

- Every project address includes address-space id and value.
- Public project APIs should never return a bare address without address-space
  context.
- Display helpers can still render `0x...`, but the model keeps identity.
- Conversion errors become `ProblemRecord`s when relevant to user-facing views.

### Object Ids

The same address can mean several different things:

- an instruction start
- a function start
- a basic block start
- a symbol
- a string
- an ObjC method IMP
- a selector ref
- a chained fixup
- a cache address
- a runtime slid address

Therefore route and API targets must not use address alone as identity.

Suggested object key ingredients:

- object kind
- source id
- address-space id
- range start/end where applicable
- stable semantic key where available, such as symbol name or ObjC class/method
  tuple
- parent object id where scoped, such as basic block within function

### Facts

Suggested fact shape:

```rust
pub struct AnalysisFact {
    pub id: AnalysisFactId,
    pub kind: AnalysisFactKind,
    pub subject: AnalysisObjectRef,
    pub value: FactValue,
    pub provenance: FactProvenance,
    pub confidence: FactConfidence,
    pub invalidates: Vec<AnalysisFactId>,
}
```

Provenance examples:

- `macho_load_command`
- `macho_section`
- `macho_symbol_table`
- `macho_export_trie`
- `dyld_chained_fixup`
- `objc_metadata`
- `shared_cache_mapping`
- `shared_cache_local_symbols`
- `capstone_decode`
- `damsel_cfg_analysis`
- `damsel_jump_table_analysis`
- `user_annotation`
- `kdk_symbol`
- `dsym_dwarf`
- `heuristic`

Confidence levels:

- `certain`
- `high`
- `medium`
- `low`
- `user_asserted`
- `contradicted`

Rules:

- User assertions are not automatically "certain"; they are a separate class.
- A fact can have high confidence and still be invalidated by a user override.
- Conflicting facts should be visible to the inspector and problem list.

### Byte Map

Byte classification should support a base layer and overlay layers.

Base kinds:

- `unknown`
- `undefined`
- `code`
- `procedure`
- `data`
- `cstring`
- `utf16_string`
- `cfstring`
- `pointer`
- `padding`

Apple overlay kinds:

- `objc_class_metadata`
- `objc_category_metadata`
- `objc_protocol_metadata`
- `objc_method_list`
- `objc_selector_ref`
- `objc_class_ref`
- `objc_super_ref`
- `swift_metadata`
- `swift_string`
- `cxx_vtable`
- `cxx_rtti`
- `dyld_binding`
- `chained_fixup`
- `stub`
- `stub_helper`
- `pac_pointer`
- `selector_trampoline`
- `cache_image_boundary`
- `kernel_table`
- `syscall_table`
- `mach_trap_table`
- `mig_descriptor`
- `iokit_class_metadata`
- `problem_region`

User override kinds:

- `make_code`
- `make_procedure`
- `make_data`
- `make_string`
- `make_unknown`
- `manual_overlay`

Rules:

- Base classifications should be non-overlapping within an address space.
- Overlay classifications may overlap base ranges.
- User overrides have explicit range and transaction provenance.
- Conflicts are stored and exposed, not silently resolved.
- Feature-map queries should be able to collapse overlay layers by priority.

Suggested range record:

```rust
pub struct ByteRangeClassification {
    pub id: AnalysisObjectId,
    pub range: AnalysisAddressRange,
    pub kind: ByteClassificationKind,
    pub layer: ByteClassificationLayer,
    pub provenance: FactProvenance,
    pub confidence: FactConfidence,
    pub source_fact: Option<AnalysisFactId>,
}
```

### Routes

Routes should be serializable and stable enough for Workbench state
restoration, CLI JSON output, and future scripting.

Suggested route examples:

```json
{
  "kind": "instruction",
  "objectId": "object:instruction:...",
  "address": {
    "space": "aspace:...",
    "value": "0x100003f10"
  }
}
```

```json
{
  "kind": "objc_method",
  "objectId": "object:objc_method:...",
  "relatedAddress": {
    "space": "aspace:...",
    "value": "0x1a2345678"
  }
}
```

Rules:

- Every route has an object id when possible.
- Address-only routes are allowed for unresolved user input but should resolve
  to richer object routes after query.
- Back/forward navigation records route origin:
  search, xref click, graph click, ObjC browser, CLI command, script, debugger,
  user typed address.

## Ingesting Existing Damsel Models

### `BinaryImage` -> `AnalysisProject`

Required steps:

1. Create a primary `AnalysisSource`.
2. Create `file_offset` and `image_vmaddr` address spaces.
3. Create segment and section objects.
4. Insert section-derived byte classifications.
5. Insert symbol/import/export objects and facts.
6. Insert dyld metadata facts.
7. Insert ObjC metadata objects and facts.
8. Insert string objects where current models expose them or where sections
   make discovery cheap.
9. Create initial problems for unsupported or partial features.
10. Produce deterministic project snapshot JSON.

Section-derived initial classification:

- executable text sections -> `code`
- cstring sections -> `cstring`
- literal string sections -> string kind where known
- data sections -> `data`
- stub sections -> `stub`
- symbol pointer sections -> `pointer` plus dyld binding overlays when known
- ObjC sections -> ObjC overlay kinds
- unknown sections -> `data` or `unknown` depending on current `SectionKind`

Do not over-infer:

- A section being executable does not prove every byte is a valid instruction.
- A symbol start does not always prove a complete function range.
- ObjC metadata pointers should keep raw pointer and resolved target facts
  separate.

### `ProjectedBinaryImage` -> `AnalysisProject`

Projected cache image projects need more provenance than standalone files.

Required source registry:

- dyld shared cache source
- projected image source
- optional local symbols source

Required address spaces:

- cache VM address
- projected image VM address
- file offset where available

Required relationships:

- projected image has parent dyld cache
- section ranges retain original cache mapping provenance
- xrefs can point outside the projected image through cache address space

Important rule:

- Do not erase cache identity when materializing a projected image. Hopper's
  public changelog shows method navigation for files extracted from shared
  caches matters. Damsel's edge is to keep cache context richer than an
  extracted file.

### `DisassemblyAnalysis` -> Project Facts

Map current records:

- `DecodedInstruction` -> instruction object and decoded-instruction fact
- `BasicBlock` -> basic-block object and range fact
- `ControlFlowEdge` -> control-flow edge object and xref fact
- import uses -> call/data xref facts with import provenance
- ObjC uses -> ObjC selector/class/method references
- cache link uses -> cross-source xrefs
- recovered values -> value facts attached to instruction/operand
- jump table uses -> switch candidate facts and optional problem records
- indirect uses -> indirect target candidate facts

Important rule:

- Keep the original disassembly analysis as a pass output, but do not make it
  the project database itself. It should feed project facts.

### ObjC Metadata -> Runtime Objects

Map current metadata:

- classes -> `ObjcClassRecord` object
- categories -> `ObjcCategoryRecord` object
- protocols -> `ObjcProtocolRecord` object
- methods -> `ObjcMethodRecord` object
- properties -> property object or fact
- ivars -> ivar object or fact
- selector refs -> selector object and xref facts
- class refs -> class ref fact
- method IMPs -> route from ObjC method to instruction/function address

Apple-specific target:

- A selector should be both a string-like object and a runtime message key.
- A method IMP should link to the function/procedure object when analysis has
  created one.
- A class/category should become an automatic tag source for associated method
  implementations, mirroring Hopper's public tutorial behavior.

### Dyld Metadata -> Linkage Facts

Map current metadata:

- imports -> import objects and symbol facts
- exports -> export objects and symbol facts
- chained fixups -> fixup objects and pointer target facts
- bindings -> binding objects and target facts
- stubs/stub helpers -> stub objects and instruction/data xrefs
- rpaths/dylibs -> source relationship facts or linkage facts

Required provenance:

- dyld facts must know whether they came from load commands, export trie,
  chained fixups, classic binding info, symbol table, stubs, cache local
  symbols, or resolver heuristics.

## Query API Shape

The same query concepts should exist in Rust, CLI JSON, HTTP API, and future
automation. Initial API does not need every endpoint, but it should not choose
a shape that blocks them.

### Rust Query Facade

Suggested methods:

```rust
impl AnalysisProject {
    pub fn summary(&self) -> ProjectSummary;
    pub fn sources(&self) -> &[AnalysisSource];
    pub fn address_spaces(&self) -> &[AddressSpace];
    pub fn object(&self, id: &AnalysisObjectId) -> Option<&AnalysisObjectRecord>;
    pub fn facts_for_object(&self, id: &AnalysisObjectId) -> Vec<&AnalysisFact>;
    pub fn byte_classifications_at(&self, address: AnalysisAddress)
        -> Vec<&ByteRangeClassification>;
    pub fn byte_classifications_in(&self, range: AnalysisAddressRange)
        -> Vec<&ByteRangeClassification>;
    pub fn route_for_address(&self, address: AnalysisAddress) -> WorkbenchRoute;
    pub fn resolve_route(&self, route: &WorkbenchRoute) -> RouteResolution;
    pub fn problems_for_object(&self, id: &AnalysisObjectId) -> Vec<&ProblemRecord>;
}
```

Do not optimize this facade prematurely. It can be backed by vectors in Stage 0
and indexes later.

### CLI Commands

Add project commands without replacing existing commands:

```text
damsel project create <path> --json
damsel project summary <snapshot-or-path> --json
damsel project sources <snapshot-or-path> --json
damsel project address-spaces <snapshot-or-path> --json
damsel project objects <snapshot-or-path> --kind section --json
damsel project facts <snapshot-or-path> --json
damsel project byte-map <snapshot-or-path> --json
damsel project route <snapshot-or-path> --address 0x... --json
```

Wave 1 minimal command set:

- `project create`
- `project summary`
- `project sources`
- `project byte-map`
- `project facts`

Rules:

- JSON output must be deterministic for fixtures.
- Text output can be concise and human-oriented.
- Commands should accept standalone Mach-O first.
- Projected shared-cache image support can be a second project command branch
  if doing both at once risks destabilizing the review.

### HTTP API

Add beside existing `/api/images` routes:

```text
POST /api/projects
GET  /api/projects/{project_id}
GET  /api/projects/{project_id}/sources
GET  /api/projects/{project_id}/address-spaces
GET  /api/projects/{project_id}/byte-map
GET  /api/projects/{project_id}/objects
GET  /api/projects/{project_id}/facts
POST /api/projects/{project_id}/route/resolve
```

Initial `POST /api/projects` modes:

- upload bytes, matching current image upload compatibility
- later: open local path through secure app-mediated file access

Do not block the local server while running long analysis. For Stage 0 project
creation from a small fixture, synchronous creation is acceptable, but the API
shape should include future job ids:

```json
{
  "projectId": "...",
  "status": "ready",
  "jobs": []
}
```

Future async shape:

```json
{
  "projectId": "...",
  "status": "analyzing",
  "jobs": [
    {
      "jobId": "...",
      "kind": "initial_analysis",
      "state": "running"
    }
  ]
}
```

### Swift Workbench Models

Add models without changing all UI at once:

- `ProjectCreateResponse`
- `ProjectSummaryResponse`
- `ProjectSource`
- `AddressSpaceModel`
- `ProjectObject`
- `ProjectFact`
- `ByteClassificationRange`
- `WorkbenchRouteModel`
- `ProblemModel`
- `AnalysisJobModel`

Add store state beside existing image state:

```swift
@Published public private(set) var loadedProject: ProjectCreateResponse?
@Published public private(set) var projectSummary: ProjectSummaryResponse?
@Published public private(set) var currentRoute: WorkbenchRouteModel?
```

Migration rule:

- Do not delete `loadedImage` or current disassembly flow immediately.
- First prove project creation and summary display.
- Then migrate symbol suggestions and disassembly routes to project-backed
  endpoints.

## Project Store Inside `damsel-cli ui`

> Supersession note (2026-06-12, Decision D-3 of the
> [RE Power program](../re-power/wave-4-analysis-kernel-spec.md)): the
> staged store rollout below was the original Apple-track plan. The accepted
> RE Power decision builds the embedded SQLite store from the first
> implementation PR (schema, migrations, WAL, event log table); Stage 0/1
> remain valid only as in-process migration steps for the UI server's
> upload-compatibility path, not as storage stages.

### Stage 0 Store

Add:

```rust
struct ProjectStore {
    next_id: AtomicU64,
    projects: Mutex<HashMap<String, StoredProject>>,
}

struct StoredProject {
    project: Arc<AnalysisProject>,
}
```

Keep existing:

```rust
struct ImageStore { ... }
```

Reason:

- Workbench can migrate gradually.
- Existing web UI keeps working.
- The review can isolate new project behavior.

### Stage 1 Store

Once project APIs are stable:

- Make uploaded images create both `StoredImage` and `StoredProject`.
- Teach disassembly routes to resolve through project source/address identity.
- Deprecate image-only state in the app, not in CLI immediately.

### Stage 2 Store

When SQLite or persistent project files arrive:

- `ProjectStore` tracks open project handles.
- Raw bytes are memory-mapped or referenced by source bookmarks.
- Large fact indexes page from embedded store.
- HTTP responses are paged and cancellable.

## Typed Byte Map Generation Algorithm

### Inputs

Initial inputs:

- segment ranges
- section ranges
- section kind
- symbols
- exports
- imports
- stubs
- stub helpers
- dyld bindings
- chained fixups
- ObjC metadata
- disassembly instruction ranges if analysis is requested
- string sections and scanners

### Pass 1: Seed Source Ranges

Create base classifications from sections:

```text
__TEXT,__text             -> code
__TEXT,__stubs            -> stub overlay + code base
__TEXT,__stub_helper      -> stub_helper overlay + code base
__TEXT,__cstring          -> cstring
__TEXT,__objc_methname    -> objc_selector_ref or cstring overlay
__TEXT,__objc_classname   -> objc_class_metadata or cstring overlay
__DATA,__objc_classrefs   -> objc_class_ref overlay + pointer/data base
__DATA,__objc_selrefs     -> objc_selector_ref overlay + pointer/data base
__DATA,__objc_superrefs   -> objc_super_ref overlay + pointer/data base
__DATA,__objc_const       -> objc metadata overlay + data base
__DATA,__got              -> pointer/data
__DATA_CONST              -> data with dyld/fixup overlays where known
```

This mapping should be implemented through current `SectionKind` first, with
section-name fallbacks only where `SectionKind` is too coarse.

### Pass 2: Overlay Symbols And Runtime Objects

Create overlays:

- symbol start at address
- exported symbol
- import binding slot
- ObjC class metadata range
- ObjC method list range
- selector string and selector ref range
- method IMP address
- chained fixup slot
- stub and helper range

Do not overwrite base classification unless the overlay proves a better base.

### Pass 3: Overlay Disassembly Facts

When instructions are decoded:

- mark instruction byte ranges as `code`
- mark function ranges as `procedure`
- mark block ranges as function subranges
- attach branch/call/xref overlays
- create problem overlay for decode gaps or conflicts

Rules:

- Instruction ranges win over generic section `code` only for exact decoded
  spans.
- Decode gaps in executable sections remain `unknown` or `undefined` depending
  on whether the analyzer attempted them.

### Pass 4: Detect Conflicts

Create conflicts when:

- decoded instruction overlaps a confirmed string
- ObjC metadata pointer points outside available address spaces
- fixup target cannot be converted
- user override contradicts inferred fact
- symbol range conflicts with function range
- projected cache address has no parent cache mapping

Conflicts should produce `ProblemRecord`s, not silent lossy merges.

### Pass 5: Feature-Map Tiles

Build coarse tiles for UI:

- input: address range and desired tile count or byte stride
- output: dominant base kind, overlay flags, problem count, confidence summary
- include first/last object ids in tile for quick navigation

The feature map should be generated from byte-map facts, not from section names.

## Annotation Transaction Model

### Required Initial Commands

Wave 1 can expose model support before UI commands. The model should support:

- set name
- set comment
- set inline comment
- add tag
- remove tag
- set color
- add bookmark
- remove bookmark
- add manual reference
- remove manual reference
- set operand format
- set byte classification override
- clear byte classification override

### Transaction Shape

```rust
pub struct AnnotationTransaction {
    pub id: AnnotationTransactionId,
    pub actor: AnnotationActor,
    pub origin: AnnotationOrigin,
    pub label: String,
    pub changes: Vec<AnnotationChange>,
    pub created_at: Option<String>,
}
```

Origins:

- `ui`
- `cli`
- `script`
- `mcp`
- `importer`
- `test`

Actor:

- local user
- automated tool
- imported data source

Wave 1 can keep timestamps out of deterministic fixtures.

### Undo/Redo Rule

Undo/redo should operate on transactions, not single fields.

Examples:

- "Make Procedure" may set byte classification, create function object, create
  basic block seed, and enqueue reanalysis. Undo must revert all linked edits.
- "Import IL2CPP names" may set hundreds of names and comments. Undo should
  revert the import batch as one transaction.
- "Apply KDK symbolication profile" should be separate from user labels so user
  labels are not lost when symbolication is detached.

## Analysis Job Model

### Initial Job Kinds

- `load_source`
- `seed_project`
- `classify_bytes`
- `decode_entrypoints`
- `build_function_index`
- `build_xref_index`
- `objc_index`
- `dyld_linkage_index`
- `shared_cache_index`
- `problem_scan`

Wave 1 likely implements only:

- `seed_project`
- `classify_bytes`
- optional `decode_entrypoints`

### Job Events

Events should be appendable:

- started
- progress
- produced fact batch
- produced problem
- completed
- cancelled
- failed

Even before persistence, this shape helps Workbench present progress honestly.

## Migration Plan

### PR 1: Pure Core Types

Files:

- `damsel-core/src/project.rs`
- `damsel-core/src/source.rs`
- `damsel-core/src/address.rs`
- `damsel-core/src/object.rs`
- `damsel-core/src/facts.rs`
- `damsel-core/src/byte_map.rs`
- `damsel-core/src/route.rs`
- `damsel-core/src/annotation.rs`
- `damsel-core/src/problem.rs`
- `damsel-core/src/job.rs`

Deliverables:

- types compile
- serde where appropriate
- constructors for deterministic ids
- unit tests for address ranges and byte-map overlap behavior
- no parser dependencies added to core

Verification:

- `cargo test -p damsel-core`

### PR 2: Standalone Mach-O Project Builder

Files:

- `damsel-macho/src/project.rs`
- `damsel-macho/src/lib.rs`
- tests under `damsel-macho/tests` or existing fixture tests

Deliverables:

- `create_standalone_project(&BinaryImage, options)`
- source/address-space seeding
- segment/section/symbol/import/export facts
- byte-map seeding from sections and existing metadata
- deterministic JSON snapshot helper

Verification:

- fixture snapshot tests
- existing Mach-O tests still pass

### PR 3: CLI Project Commands

Files:

- `damsel-cli/src/main.rs`
- `damsel-cli/src/output.rs`
- new project command module if needed

Deliverables:

- `damsel project create <path> --json`
- `damsel project summary <path-or-snapshot> --json`
- `damsel project byte-map <path-or-snapshot> --json`
- text mode summaries

Verification:

- CLI snapshot tests
- existing command snapshots unchanged except deliberate additions

### PR 4: Project Store In Local UI Server

Files:

- `damsel-cli/src/ui.rs`
- local UI API tests if present

Deliverables:

- `ProjectStore` beside `ImageStore`
- `POST /api/projects`
- `GET /api/projects/{project_id}`
- `GET /api/projects/{project_id}/byte-map`
- response models using core serde types or thin API wrappers

Verification:

- server route tests if available
- manual local server smoke test
- existing `/api/images` routes still work

### PR 5: Workbench Read-Only Project Awareness

Files:

- `DamselAPIClient.swift`
- `DamselModels.swift`
- `WorkbenchStore.swift`
- small view additions only if needed

Deliverables:

- create project from selected file
- store `loadedProject`
- show project summary and source count in existing UI
- do not replace disassembly flow yet

Verification:

- Swift model decode tests
- Workbench smoke run if available

### PR 6: Project-Backed Search And Routes

Deliverables:

- route resolution endpoint
- project symbol/string/function search endpoint
- Workbench route state
- back/forward route stack model

Verification:

- jump to symbol/address uses route model
- route JSON round trips

### PR 7: Project-Backed Disassembly Rows

Deliverables:

- semantic assembly row endpoint
- instruction object ids
- byte spans and operand spans
- inspector can query selected instruction by object id

Verification:

- existing disassembly output parity for text where expected
- new semantic row snapshots

### PR 8: Annotation Transactions

Deliverables:

- CLI or HTTP command to set comment/name/tag on project object
- transaction journal in memory
- undo/redo model tests
- Workbench inspector can display annotations if added by API

Verification:

- create project, set comment, snapshot includes annotation
- undo removes it
- redo restores it

## API Compatibility Rules

Preserve:

- existing CLI command behavior unless explicitly changed
- existing `/api/images` HTTP routes until Workbench migrates
- existing Swift image/disassembly models until project-backed replacements are
  complete

Avoid:

- moving existing fields without compatibility wrappers
- making project ids parseable client contracts
- returning raw absolute paths by default
- coupling Workbench directly to Rust-internal struct names where API wrappers
  are more stable

## Performance Design

### Immediate Budgets

Initial implementation should measure:

- project create time on small fixture
- project create time on medium local binary
- byte-map query time for address
- byte-map query time for range
- JSON snapshot size
- memory retained by `ProjectStore`

### Future Budgets

Set targets before Workbench migration:

- first project summary available under 500 ms for typical small Mach-O
- symbol/search interaction under 50 ms for indexed project
- route resolve under 30 ms for loaded project
- byte-map tile generation under 50 ms for viewport-sized map
- cancellation observed within 100 ms for long-running analysis jobs
- UI lists paged before 10,000 rows

### Storage Rules

Stage 0:

- pure in-memory vectors and BTreeMaps
- deterministic JSON snapshots

Stage 1:

- append-only event log for mutations
- snapshot plus event replay

Stage 2:

- SQLite/WAL or equivalent embedded store
- indexes for object kind, address range, name, tag, xref endpoints, source,
  problem severity, and route target

Stage 3:

- sharded caches for large disassembly rows, graph layout, decompiler-lite
  summaries, string indexes, and cache-wide xrefs

## Security And Privacy Design

Rules:

- Do not store absolute source paths in shared diagnostics by default.
- Keep raw binary bytes out of project snapshots unless explicitly requested.
- Mark source path fields as local/private.
- Future scripting and MCP must use read-only defaults.
- Mutation APIs require explicit commands and audit records.
- Patch-intent records must not write modified binaries unless a separate
  export command is invoked.

Workbench local server:

- keep binding to `127.0.0.1`
- keep project APIs local-only
- avoid exposing arbitrary filesystem read endpoints
- file-open via Workbench should pass either bytes or a security-scoped local
  path only through app-mediated flow

## Testing Plan

### Unit Tests

Core:

- deterministic id generation
- address range containment and overlap
- address-space mismatch errors
- byte-map overlay behavior
- byte-map conflict detection
- route serialization
- annotation transaction apply/undo/redo
- fact provenance serialization

Macho adapter:

- standalone source creation
- section classification
- symbol facts
- import/export facts
- ObjC metadata facts
- dyld binding/fixup facts where fixtures contain them

### Snapshot Tests

Create snapshots for:

- small arm64 fixture
- fixture with cstrings
- fixture with ObjC metadata
- fixture with stubs/imports
- fixture with chained fixup data if available
- projected cache image fixture if current tests support it

Snapshot invariants:

- stable object ids
- no absolute private paths unless explicitly allowed
- byte ranges sorted
- facts sorted deterministically
- problems sorted deterministically

### CLI Tests

Commands:

- `project create --json`
- `project summary --json`
- `project sources --json`
- `project byte-map --json`
- invalid path error
- unsupported file error

### HTTP Tests

If server test harness exists or is added:

- create project from multipart upload
- fetch summary
- fetch byte map
- unknown project id returns structured error
- existing image upload route still works

### Workbench Tests

Swift decode tests:

- project create response
- source model
- address-space model
- byte-map response
- route model

Manual smoke:

- launch Workbench
- load file
- project summary visible
- old disassembly still works

## Acceptance Criteria For First Implementation Wave

The first wave is done only when:

- `damsel-core` exposes project/source/address/object/fact/byte-map/route
  models with unit tests
- `damsel-macho` can build a deterministic standalone project snapshot from a
  `BinaryImage`
- initial byte classifications come from real existing section and metadata
  evidence
- facts include provenance and confidence
- `damsel project create <path> --json` works on at least one fixture
- project snapshots do not leak private absolute paths by default
- existing CLI commands still pass their current tests
- existing Workbench image upload/disassembly path is not broken
- docs link this spec from the Hopper-gap map and roadmap

## Risks

### Risk: Model Explosion

The project kernel can become too abstract too quickly.

Mitigation:

- implement only standalone Mach-O project construction first
- reserve enum variants but populate only proven facts
- snapshot all outputs so review can see actual data

### Risk: Private Paths In Snapshots

Project source records may leak local absolute paths.

Mitigation:

- split `display_name`, `path_display`, and private full path
- default snapshots to display name and content hash only
- add tests that reject `/Users/` in fixture snapshots

### Risk: Address Identity Confusion

Cache/projected/runtime addresses can be conflated.

Mitigation:

- require address-space id on every project address
- forbid xrefs with bare `u64` in project APIs
- add conversion tests with intentional failure cases

### Risk: Workbench Migration Breaks Current UX

Project APIs could disrupt the existing working upload/disassembly path.

Mitigation:

- keep `ImageStore` and image routes until project-backed route/disassembly is
  proven
- add Workbench project awareness read-only first
- migrate one surface at a time

### Risk: Byte Map Becomes Section Map Only

If byte classification just mirrors sections, it will not support Hopper-like
manual analysis.

Mitigation:

- model user overrides and overlays in Wave 1 even if the first UI does not edit
  them
- include instruction and ObjC/fixup overlays where evidence exists
- add conflict records

### Risk: Future Persistence Forces Rewrite

An in-memory model might not map cleanly to SQLite.

Mitigation:

- keep object ids stable and explicit
- keep facts normalized
- avoid storing derived rendered strings as canonical facts
- keep snapshots close to future table boundaries

## Relationship To Other Docs

This spec refines these existing docs:

- [Analysis Database Blueprint](analysis-database-blueprint.md): this document
  turns the database concept into first implementation modules and contracts.
- [Wave 1 Project Foundation Implementation Spec](wave-1-project-foundation-spec.md):
  this document expands Wave 1 with typed-byte, route, annotation, job, and
  migration details after the deeper Hopper research pass.
- [Current Code Integration Map](current-code-integration-map.md): this document
  provides the concrete bridge from current `BinaryImage`/`ImageStore`/
  `WorkbenchStore` architecture to project-backed APIs.
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md):
  this document implements the architectural correction from that research.
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md): this
  project kernel supplies the object ids and routes needed by search.
- [API And Data Contracts](api-data-contracts.md): project endpoints should be
  updated to use these object, route, fact, and byte-map shapes.

## Bottom Line

The smallest real step toward Hopper competitiveness is not a bigger
disassembly response. It is a project kernel with explicit source identity,
address spaces, typed bytes, facts, object ids, routes, annotations, jobs, and
problems.

Once that exists, the rest of the Hopper-like app becomes composable:

- assembly is a semantic view over project facts
- graph is a procedure view plus presentation facts
- inspector is a module set over selected objects
- search is an index over project objects
- ObjC/C++/Swift/kernel workflows are runtime overlays
- Workbench route history is a project route stack
- scripting and MCP are safe project APIs
- debugger mapping is runtime addresses resolved into project routes
- patching is a transaction and export workflow, not a direct byte write

Without this kernel, each new feature will invent its own state and Damsel will
keep drifting away from the product architecture that makes Hopper feel fast,
coherent, and durable.
