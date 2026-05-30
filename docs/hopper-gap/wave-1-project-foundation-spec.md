# Wave 1 Project Foundation Implementation Spec

Status: Implementation-planning spec for the first real branch after the
Hopper-gap documentation pack. This is a proposal, not an implemented feature.

Purpose: define the smallest meaningful implementation wave that moves Damsel
from point-query analysis toward a Hopper-like project workspace while keeping
scope reviewable and preserving existing commands.

Expanded companion: [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
adds the deeper post-Hopper-research implementation plan for byte
classifications, source/address/object identity, fact provenance, routes,
annotation transactions, local UI project storage, Workbench migration, and
testing.

## Goal

Wave 1 should create the durable project identity layer used by every later
feature:

- project
- source
- address space
- object id
- route
- initial facts
- byte classifications
- deterministic snapshots
- initial CLI project commands

This wave should not implement the full Workbench redesign, function index,
SQLite persistence, decompiler, debugger, scripting, plugins, or patching. It
should make those later features possible.

## Non-Goals

Do not include:

- saved project file format beyond deterministic snapshots
- SQLite or long-term store
- Workbench UI overhaul
- annotation editing UI
- whole-image recursive function recovery
- graph layout
- ObjC browser UI
- shared-cache-wide project browser
- kernel collection loader
- scripting/MCP
- LLDB/debugger bridge
- binary patching

These belong to later waves once object identity and project facts are stable.

## Success Criteria

Wave 1 is successful when:

- `damsel-core` exposes stable project/source/address/object/route model types
- `damsel-macho` can build a project snapshot from a standalone `BinaryImage`
- initial byte classifications are derived from existing section/string/symbol
  evidence
- initial facts are emitted with provenance and confidence
- CLI can create and inspect a project snapshot in JSON
- tests prove deterministic output for representative fixtures
- existing CLI commands continue to pass
- Workbench remains compatible with existing image upload/disassembly APIs

## Crate-Level Scope

### `damsel-core`

Add pure model modules:

- `project.rs`
- `address.rs`
- `object.rs`
- `route.rs`
- `facts.rs`
- `byte_map.rs`

Export through `lib.rs` only after the types have tests.

Required types:

- `AnalysisProject`
- `AnalysisProjectId`
- `AnalysisSource`
- `AnalysisSourceId`
- `AnalysisSourceKind`
- `AnalysisSourceRole`
- `AddressSpace`
- `AddressSpaceId`
- `AnalysisAddress`
- `AddressRange`
- `AnalysisObjectId`
- `AnalysisObjectKind`
- `WorkbenchRoute`
- `AnalysisFact`
- `AnalysisFactKind`
- `FactProvenance`
- `FactConfidence`
- `ByteClassification`
- `ByteClassificationKind`
- `ByteClassificationSource`

Do not add parser dependencies to `damsel-core`.

### `damsel-macho`

Add project construction adapter:

- `project.rs`

Responsibilities:

- build `AnalysisProject` from `BinaryImage`
- seed source identity
- seed address spaces
- seed object ids for segments/sections/symbols/imports/strings/ObjC records
- seed facts from existing models
- seed byte classifications
- produce deterministic snapshot payloads

Initial builder functions:

```rust
pub fn create_standalone_project(image: &BinaryImage, options: ProjectBuildOptions)
    -> Result<AnalysisProject, MachoError>;
```

Later, not necessarily Wave 1:

```rust
pub fn create_projected_cache_image_project(
    projected: &ProjectedBinaryImage,
    options: ProjectBuildOptions,
) -> Result<AnalysisProject, MachoError>;
```

### `damsel-cli`

Add a `project` command family:

```text
damsel project create <path> --json
damsel project status <path-or-snapshot> --json
damsel project sources <path-or-snapshot> --json
damsel project byte-map <path-or-snapshot> --json
damsel project facts <path-or-snapshot> --json
```

Wave 1 can use snapshot JSON files or direct binary paths. The CLI contract
should be explicit about which mode is implemented.

Do not remove or rewrite existing standalone commands.

### `damsel-cli ui`

Wave 1 UI server work should be minimal:

- no Workbench migration required
- optional read-only `POST /api/projects` spike only if CLI model is stable
- no project persistence server requirement yet

Reason: the first review should focus on the Rust model and CLI evidence.

### `damsel-app`

No mandatory UI changes.

Allowed:

- add Swift models for future project responses if the API route is included
- add decode tests for those models

Avoid:

- replacing `WorkbenchStore`
- changing file-open behavior
- adding feature-map UI before backend byte-map contract is stable

## Model Details

### Project Id

Suggested shape:

```rust
pub struct AnalysisProjectId(String);
```

Initial deterministic id:

- hash of primary source identity plus project schema version
- stable for test fixtures
- opaque in public JSON

Rules:

- clients do not parse id strings
- tests can compare full JSON snapshots
- ids should not include absolute private paths

### Source

Suggested fields:

```rust
pub struct AnalysisSource {
    pub id: AnalysisSourceId,
    pub kind: AnalysisSourceKind,
    pub role: AnalysisSourceRole,
    pub display_name: String,
    pub path_display: Option<String>,
    pub content_hash: Option<String>,
    pub architecture: Option<Architecture>,
    pub platform: Option<Platform>,
    pub parent: Option<AnalysisSourceId>,
    pub relationship: Option<String>,
}
```

Initial source kinds:

- `standalone_macho`
- `memory_macho`
- `projected_cache_image`

Future source kinds:

- `dyld_shared_cache`
- `driver_cache`
- `kernel_collection`
- `fileset_member`
- `dsym`
- `kdk`
- `panic_log`
- `symbol_map`

Wave 1 can define future enum variants only if unused variants do not imply
implemented support.

### Address Space

Initial address spaces:

- `image_vmaddr`
- `file_offset`

Fields:

```rust
pub struct AddressSpace {
    pub id: AddressSpaceId,
    pub source_id: AnalysisSourceId,
    pub kind: AddressSpaceKind,
    pub display_name: String,
}
```

Rules:

- every address in project facts uses an address-space id
- raw `u64` addresses are not enough in public project contracts
- conversion failures return typed errors

### Object Id

Initial object kinds:

- project
- source
- segment
- section
- symbol
- import
- string
- objc_class
- objc_category
- objc_protocol
- objc_method
- objc_selector
- byte_range

Future object kinds:

- function
- basic_block
- instruction
- xref
- cpp_type
- cpp_vtable
- kernel_member
- kext
- annotation
- problem

Rules:

- object ids resolve to routes when navigable
- object ids are stable within a project snapshot
- object ids should be deterministic for fixtures

### Route

Initial route kinds:

- project overview
- source overview
- address
- section
- symbol
- ObjC object

Fields:

```rust
pub struct WorkbenchRoute {
    pub id: String,
    pub target: AnalysisObjectId,
    pub address: Option<AnalysisAddress>,
    pub source_id: Option<AnalysisSourceId>,
    pub view_hint: Option<RouteViewHint>,
}
```

View hints:

- `overview`
- `assembly`
- `metadata`
- `hex`

Graph and pseudocode hints arrive later.

### Fact

Initial fact kinds:

- source architecture
- source platform
- segment range
- section range
- symbol name
- symbol address
- import binding
- export name
- string literal
- ObjC class
- ObjC method
- ObjC selector
- dyld import
- chained-fixup presence

Fields:

```rust
pub struct AnalysisFact {
    pub id: String,
    pub kind: AnalysisFactKind,
    pub target: AnalysisObjectId,
    pub provenance: FactProvenance,
    pub confidence: FactConfidence,
    pub payload: serde_json::Value,
}
```

Payload can begin as serde JSON if typed payloads would slow Wave 1 too much,
but the enum kind must be explicit. Later waves should migrate hot facts to
typed payload structs.

### Provenance

Initial provenance sources:

- `macho_header`
- `load_command`
- `section_table`
- `symbol_table`
- `export_trie`
- `import_binding`
- `dyld_metadata`
- `objc_metadata`
- `string_scan`
- `project_builder`

Fields:

```rust
pub struct FactProvenance {
    pub source: String,
    pub analyzer: String,
    pub source_id: AnalysisSourceId,
    pub notes: Vec<String>,
}
```

Rules:

- analyzer names are stable strings
- provenance never says "user" in Wave 1 because user annotations are not part
  of this wave
- imported/generated facts remain distinguishable

### Confidence

Initial confidence:

- `exact`
- `strong`
- `inferred`
- `ambiguous`
- `weak`

Do not add `user_asserted` until annotations land.

### Byte Classification

Initial classification kinds:

- `header`
- `load_command`
- `code`
- `procedure_candidate`
- `data`
- `string`
- `objc_metadata`
- `dyld_metadata`
- `relocation_or_fixup`
- `unknown`

Initial sources:

- section kind
- symbol/function-start hints
- string table/scan
- ObjC metadata ranges
- dyld metadata ranges

Rules:

- overlapping classifications are allowed only with precedence rules
- classification records include address range and provenance
- feature-map compact ranges can be derived from classifications

## JSON Snapshot Shape

Initial snapshot top-level:

```json
{
  "schema_version": 1,
  "project": {},
  "sources": [],
  "address_spaces": [],
  "objects": [],
  "facts": [],
  "byte_classifications": [],
  "routes": [],
  "summary": {}
}
```

Rules:

- arrays sorted deterministically
- no absolute paths unless explicit option enables them
- addresses rendered as both numeric value and display string where useful
- schema version required
- analyzer version required

## CLI Contract

### `project create`

Input:

```text
damsel --format json project create fixtures/bin/semantic-switch
```

Output:

- full snapshot or path to snapshot, depending on mode
- schema version
- source summary
- object counts
- fact counts
- byte-classification counts

Errors:

- `project_input_not_found`
- `project_unsupported_format`
- `project_unsupported_architecture`
- `project_build_failed`

### `project status`

Input:

```text
damsel --format json project status fixtures/bin/semantic-switch
```

Output:

- project id
- source count
- schema version
- analyzer version
- summary counts
- stale flag, initially false

### `project sources`

Output:

- source ids
- kind
- role
- display name
- architecture/platform
- hash/UUID where available

### `project byte-map`

Output:

- compact byte classification ranges
- counts by kind
- address-space id

### `project facts`

Output:

- paged or limited fact list
- kind
- target
- provenance
- confidence

If paging is too much for Wave 1, add `--limit` and document default limits.

## Test Plan

### Core Unit Tests

Files:

- `damsel-core/tests/project_model_contract.rs`
- `damsel-core/tests/address_space_contract.rs`
- `damsel-core/tests/byte_map_contract.rs`
- `damsel-core/tests/fact_provenance_contract.rs`

Cases:

- ids are deterministic
- address-space conversion preserves identity
- source display redacts path when configured
- facts sort deterministically
- byte classification overlap resolves predictably

### Mach-O Project Tests

Files:

- `damsel-macho/tests/project_matrix.rs`

Fixtures:

- `arm64-symbolized`
- `arm64-stripped`
- `import-rich`
- `objc-sample`
- `semantic-switch`
- `swift-sample`
- `arm64e-sample`

Snapshots:

- project summary
- source registry
- byte classification counts
- fact counts by kind
- selected object ids/routes

### CLI Tests

Files:

- `damsel-cli/tests/project_cli.rs`
- snapshots under `damsel-cli/tests/snapshots`

Cases:

- `project create --format json`
- `project status --format json`
- `project sources --format json`
- `project byte-map --format json`
- `project facts --format json --limit`
- invalid path
- unsupported x86-only input

### Regression Tests

Existing gates must still pass:

- `cargo test --workspace`
- `./fixtures/build-fixtures.sh --check`
- `./fixtures/tests/build-fixtures-parity.sh`
- `cargo bench -p damsel-macho --bench decode_bench --no-run`

Swift tests are not required if Wave 1 does not touch `damsel-app`.

## Performance Checks

Wave 1 should record lightweight timings:

- project create duration
- source count
- object count
- fact count
- byte classification count

Do not fail benchmarks initially unless pathological regressions appear.

Add a follow-up issue for:

- structured benchmark JSON
- peak memory metrics
- project creation benchmarks for all fixtures

## Review Checklist

Before merging Wave 1:

- no existing CLI output snapshots changed unexpectedly
- project snapshots are deterministic
- no absolute private paths in default snapshots
- source/address/object ids are stable
- all facts have provenance and confidence
- byte classifications cover mapped sections
- unsupported inputs produce typed errors
- no Workbench behavior regressed
- docs are updated to mark Wave 1 implemented or still planned honestly

## Follow-Up Waves Unblocked

Wave 1 unlocks:

- annotation transaction model
- function and xref index
- project UI APIs
- Workbench project open
- feature map
- ObjC runtime explorer
- cache-native project source registry
- corpus/benchmark expansion
