# Current Code Integration Map

Status: Repo-grounded implementation map. This document connects the
Hopper-gap architecture to Damsel's current crates, modules, command surfaces,
app files, and test gates.

Purpose: prevent the documentation pack from drifting into abstract product
strategy. The next implementation work should know exactly which existing
pieces to preserve, which modules to extend, and where deep refactors are
required.

Concrete next-architecture companion:
[Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
maps these current code anchors into first implementation modules, project
store migration, Workbench model migration, typed-byte generation, and CLI/API
commands.

Feature-architecture research companion:
[Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
summarizes the public Hopper evidence that motivates the project/document,
typed-byte, procedure, CPU semantics, transaction, automation, debugger, and
native app boundaries in this map.

Source-to-requirement research companion:
[Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
maps Hopper's current public release API, changelog, tutorial, FAQ, SDK,
scripts, MCP wrappers, and workflow evidence to concrete Damsel subsystems.

Function and decompiler implementation companions:
[Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
and
[Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
turn the current `DisassemblyAnalysis` strengths into a staged procedure
database, architecture semantics layer, IR, SSA, ABI/type enrichment, and
runtime-overlay plan.

Decompiler Workbench representation companion:
[Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
maps the same current analysis strengths into future native pseudocode routes,
line/token/source-map models, decompiler jobs, confidence/problem display,
variable/type correction transactions, Apple runtime overlays, and export
contracts.

Mutation and debugger implementation companions:
[Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
and
[Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
show how current byte providers, Workbench selection state, source identity,
and future transactions must grow before Damsel can safely support patch
preview or route-backed debugging.
[Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
extends that bridge into the future product layer: launch/attach UX, debug
targets and sessions, module snapshots, route-backed breakpoints, stop events,
register/frame/memory stores, LLDB helper isolation, Apple runtime live
interpretation, permission errors, and diagnostic redaction.
[Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md)
extends the current byte providers and future transaction model into a native
patching product surface: patchability, byte edit previews, assembler/NOP
plans, copy-on-write source revisions, Apple artifact safety, export policy,
reanalysis, and patch evidence.

Automation and extension implementation companions:
[Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
and
[Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
show how future CLI batch, MCP, script, loader, analyzer, exporter, and
Workbench command surfaces must use the same project/query/annotation APIs.

Performance runtime implementation companion:
[Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
maps current upload-based loading, in-memory image storage, point-query
disassembly, Workbench request ids, and `NSTableView` rendering into the target
source registry, source access, job scheduler, paged index, metrics, and store
split architecture.

Diff/export/evidence implementation companion:
[Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
maps existing shared-cache linkage, ObjC metadata, dyld metadata,
`DisassemblyAnalysis`, CLI JSON, and future project facts into analysis
snapshots, object fingerprints, Apple-system diffs, evidence packages, and
Workbench diff views.

Native app trust/distribution companion:
[Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
maps the current Swift package, `DamselServerProcess`, `DamselAPIClient`,
`WorkbenchStore`, local UI server, upload flow, helper lifecycle, and future
release packaging into source access grants, authenticated helper sessions,
diagnostic redaction, crash/hang recovery, signing/notarization gates, update
state, plugin trust, and debugger trust.

Artifact intake companion:
[Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
maps current Mach-O loading, byte loading, universal slice selection,
`SharedCacheSession`, real/synthetic cache discovery, projected images, and
Workbench file opening into intake sessions, source registration plans, source
graphs, container manifests, sidecar matching, projection recipes, KDK/dSYM
attachment, panic-log attachment, and artifact overview routes.

Type/signature/debug-info companion:
[Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
maps current typed model, ObjC metadata, symbolication, disassembly analysis,
CLI JSON, and Workbench stores into provenance-aware names, type records,
function signatures, variables, calling conventions, debug-info imports,
symbolication profiles, conflict records, and edit transactions.

DWARF/dSYM/KDK debug-info companion:
[DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
maps current Mach-O loading, shared-cache UUID handling, dyld metadata,
symbolication output, `DisassemblyAnalysis`, CLI JSON, and Workbench state into
debug-source registries, dSYM/KDK candidate matching, import jobs, compile-unit
indexes, source-line routes, variable/type import, privacy controls, and
profile-aware rendering.

Graph/feature-map/hex representation companion:
[Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
maps current `DisassemblyAnalysis` basic blocks/edges, recovered values,
jump-table/cache/ObjC facts, CLI JSON, API routes, and Workbench selection
state into durable graph records, layout caches, feature-map lanes, bounded
hex windows, canonical selection events, accessibility fallbacks, and export
contracts.

Command/navigation/undo Workbench companion:
[Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
maps current AppKit menu selectors, `DamselCommands.swift` notifications,
`WorkbenchStore` selection state, instruction filter focus, busy state,
diagnostics, copy actions, and generic Undo/Redo menu entries into a typed
command registry, dispatcher, keymap store, focus store, route-backed
selection, navigation stack, transaction store, activity log, and Apple-system
command families.

Apple runtime Workbench browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
maps current `ObjcMetadata`, `ObjcClassRecord`, `ObjcCategoryRecord`,
`ObjcMethodRecord`, `DisassemblyObjcUse`, cache link summaries, shared-cache
query commands, Swift `DamselModels`, and the current instruction-only
Workbench into paged ObjC, dispatch, C++/ObjC++, IOKit, cache, kernel,
symbolication, panic, runtime graph, and runtime problem browser services.

## Current Repo Shape

Top-level implementation surfaces:

- `damsel-core`: typed model and AArch64 decode contracts.
- `damsel-macho`: Mach-O, dyld, ObjC, shared-cache, and disassembly analysis.
- `damsel-cli`: CLI commands, JSON/text output, and the local UI HTTP server.
- `damsel-app`: native macOS Workbench over the CLI UI server.
- `fixtures`: fixture corpus and parity checks.
- `docs`: roadmap and architecture documentation.

The current architecture has a good separation for a Hopper-like future:

- Rust remains the analysis source of truth.
- CLI JSON remains a good contract proving ground.
- The Swift app is a native shell and should not become a decoder rewrite.
- Shared-cache support already exists below the UI.

The main gap is that current APIs are image/request oriented rather than
project/database oriented.

## Existing Strengths To Preserve

### Typed Core Models

Observed current exports in `damsel-core/src/lib.rs` include:

- `BinaryImage`
- `ProjectedBinaryImage`
- `SharedCache`
- `ObjcMetadata`
- `DyldMetadata`
- `DisassemblyAnalysis`
- `DecodedInstruction`
- `BasicBlock`
- `ControlFlowEdge`
- `DisassemblyCacheLinkUse`
- `DisassemblyImportUse`
- `DisassemblyObjcUse`
- `DisassemblyRecoveredValueUse`
- `DisassemblyJumpTableUse`
- `DisassemblyIndirectUse`

Preserve:

- typed records over ad hoc JSON strings
- deterministic serde-friendly model style
- separation between core model and parser implementation
- explicit enums for architecture, platform, section kind, import kind, etc.

Extend with:

- `AnalysisProject`
- `AnalysisSource`
- `AddressSpace`
- `AnalysisAddress`
- `AnalysisFact`
- `ByteClassification`
- `ObjectId`
- `Route`
- `UserAnnotation`
- `AnnotationTransaction`
- `ProblemRecord`
- `JobRecord`
- `ProcedureRecord`
- `ProcedureRange`
- `ProcedureOwner`
- `ProcedureRecoveryStatus`
- `ProcedureConflict`
- `ProcedureSignature`
- `StackLocal`
- `RegisterAlias`
- `ArchitectureSemantics`
- `IrFunction`
- `SourceRevision`
- `ByteWindow`
- `Patchability`
- `PatchPlan`
- `PatchTransaction`
- `DebugSession`
- `RuntimeModule`
- `RuntimeAddress`
- `BreakpointRecord`
- `DebugStopEvent`
- `AutomationCapability`
- `ToolManifest`
- `ToolInvocation`
- `AutomationActivity`
- `PluginManifest`
- `ExtensionRun`
- `ExtensionCapability`
- `PluginFactProvenance`
- `AnalyzerRun`
- `SourceRegistrationRequest`
- `SourceIdentity`
- `SourceAvailability`
- `SourceAccessHandle`
- `JobProgress`
- `JobEvent`
- `AnalysisPass`
- `IndexStatus`
- `IndexShard`
- `PagedQueryRequest`
- `PagedQueryResponse`
- `QueryCursor`
- `ResultVersion`
- `PerformanceMetric`
- `SlowQueryRecord`
- `CacheMetric`
- `AnalysisSnapshot`
- `ArtifactIdentity`
- `ObjectFingerprint`
- `DiffProject`
- `DiffMatchRecord`
- `DiffFinding`
- `EvidencePackage`
- `ExportManifest`
- `RedactionPolicy`
- `TrustPolicy`
- `AppBuildIdentity`
- `HelperIdentity`
- `LocalServerSession`
- `SourceAccessGrant`
- `SecurityBookmarkRecord`
- `DiagnosticsPolicy`
- `DiagnosticBundle`
- `CrashReportSummary`
- `HangReportSummary`
- `UpdateState`
- `PluginTrustRecord`
- `DebuggerTrustRecord`
- `ArtifactIntakeSession`
- `IntakeInput`
- `ArtifactCandidate`
- `ArtifactIdentity`
- `SourceRegistrationPlan`
- `ContainerManifest`
- `SidecarCandidate`
- `SymbolicationProfile`
- `ExtractionRecipe`
- `ProjectionRecipe`
- `ExternalToolReceipt`
- `IntakeProblem`

### Mach-O And Shared-Cache Implementation

Current important files:

- `damsel-macho/src/loader.rs`
- `damsel-macho/src/disasm.rs`
- `damsel-macho/src/dyld.rs`
- `damsel-macho/src/objc.rs`
- `damsel-macho/src/shared_cache.rs`
- `damsel-macho/src/shared_cache_query.rs`
- `damsel-macho/src/errors.rs`

Current strengths:

- standalone Mach-O loading
- arm64/arm64e disassembly path
- dyld metadata extraction
- ObjC metadata extraction
- shared-cache inspection
- projected cache image support
- cache dependency/provider/importer/reexport queries
- cache-aware disassembly analysis enrichment in the CLI cache path

Preserve:

- standalone image commands
- cache command family
- synthetic fixtures and matrix tests
- real shared-cache tests behind opt-in environment gates

Deep refactor required:

- analysis should become pass-driven over a project database, not only immediate
  functions returning rendered output
- `DisassemblyAnalysis` should seed persistent procedure, block, edge, xref,
  recovered-value, jump-table, indirect-target, ObjC-use, import-use, and
  cache-link records instead of remaining only a window-local summary
- cache context must be represented as source/provenance, not only available
  inside `cache disasm`
- ObjC metadata should be normalized into runtime object tables
- shared-cache projection should feed source registry and address-space
  conversion rather than producing an isolated `BinaryImage`
- Capstone-backed decode should be wrapped in an explicit AArch64/AArch64e
  architecture semantics layer before IR or pseudocode work starts

### CLI Command Surface

Current `damsel-cli/src/main.rs` exposes major point-query families:

- `info`
- `slices`
- `sections`
- `symbols`
- `imports`
- `dyld`
- `objc`
- `disasm`
- `doctor`
- `ui`
- `cache ...`

Current cache subcommands include:

- `cache info`
- `cache images`
- `cache image`
- `cache exports`
- `cache lookup-address`
- `cache resolve-symbol`
- `cache image-deps`
- `cache dependents`
- `cache symbol-providers`
- `cache symbol-importers`
- `cache reexports`
- `cache sections`
- `cache symbols`
- `cache imports`
- `cache dyld`
- `cache objc`
- `cache disasm`

Preserve:

- existing commands as stable point queries
- JSON/text output split
- explicit typed errors such as cache incomplete/image not found/ambiguous
- snapshot tests for CLI output

Add next:

- `project create`
- `project status`
- `project sources`
- `project facts`
- `project byte-map`
- `project functions`
- `project xrefs`
- `project strings`
- `project search`
- `project graph`
- `project annotate`
- `project problems`
- `project jobs`
- `project metrics`
- `project export`

Migration rule:

- do not replace point-query commands immediately
- add project-backed commands beside them
- eventually allow point-query commands to use project internals where it does
  not change user-visible behavior

### Local UI Server

Current `damsel-cli/src/ui.rs` shape:

- in-memory `ImageStore`
- upload route `POST /api/images`
- symbol route `GET /api/images/{image_id}/symbols`
- disassembly route `POST /api/images/{image_id}/disasm`
- multipart upload capped by `MAX_UPLOAD_BYTES`
- current-thread Tokio runtime
- image payload stored as `Arc<BinaryImage>`
- uploaded file bytes are copied into memory

Current limitation:

- no project object
- no source registry
- no persisted state
- no background jobs
- no paging contract
- no route/object identity
- no shared-cache project endpoints
- no annotations/undo/problems
- no source revision or patch preview endpoints
- no runtime module/debug session endpoints

Target migration:

1. Keep `/api/images` while adding `/api/projects`.
2. Add `ProjectStore` beside `ImageStore`.
3. Add project creation from local paths for Workbench.
4. Add project creation from browser upload for existing UI compatibility.
5. Add project/source/job/route/function/xref/search endpoints.
6. Mark image APIs as compatibility once Workbench migrates.

Do not:

- make Workbench depend on private Rust structs
- make project state live only in Swift
- add long-running analysis behind synchronous request handlers

### Swift Workbench

Current Workbench files:

- `damsel-app/Sources/DamselWorkbench/DamselWorkbenchApp.swift`
- `damsel-app/Sources/DamselAppCore/Services/DamselServerProcess.swift`
- `damsel-app/Sources/DamselAppCore/Services/DamselAPIClient.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- `damsel-app/Sources/DamselAppCore/Models/DamselModels.swift`
- `damsel-app/Sources/DamselAppCore/Views/ContentView.swift`
- `damsel-app/Sources/DamselAppCore/Views/ControlPane.swift`
- `damsel-app/Sources/DamselAppCore/Views/InstructionSurfaceView.swift`
- `damsel-app/Sources/DamselAppCore/Views/VirtualizedInstructionTable.swift`
- `damsel-app/Sources/DamselAppCore/Views/InspectorPane.swift`

Current Workbench model:

- launches `damsel-cli ui --no-open --port 0`
- uploads selected file bytes through `DamselAPIClient.uploadImage`
- stores `ImageUploadResponse`
- stores a transient `DisassemblyResponse`
- uses `WorkbenchStore` as a `@MainActor` observable state machine
- filters visible instructions in Swift
- jumps by changing target to address and rerunning disassembly

Current strengths:

- native app shell exists
- server lifecycle is encapsulated
- typed Swift API models exist
- virtualized table path exists
- target controls and inspector are separated
- app already treats Rust server as decoder source of truth

Current limitations:

- upload copies whole file into Swift `Data`
- app and server both hold the file bytes
- no local path registration/project open
- no saved project
- no route stack
- no source/image navigator
- no xref/graph/hex/problem/job APIs
- no annotation persistence
- no background analysis progress
- no project-level paging

Migration rule:

- preserve `DamselServerProcess`
- evolve `DamselAPIClient` from image APIs to project APIs
- split `WorkbenchStore` into smaller stores once project complexity grows
- keep Swift models as API mirrors, not analysis models

Suggested future stores:

- `ProjectStore`
- `NavigationStore`
- `JobStore`
- `SelectionStore`
- `AnnotationStore`
- `SearchStore`
- `InspectorStore`

Suggested future views:

- `ProjectNavigatorView`
- `CommandPaletteView`
- `FeatureMapView`
- `GraphWorkspaceView`
- `HexWorkspaceView`
- `ProblemsPanelView`
- `JobsPanelView`
- `RuntimeBrowserView`
- `CacheBrowserView`
- `KernelBrowserView`

## Concrete Migration Slices

### Slice 1: Project Model In Core

Files:

- add `damsel-core/src/project.rs`
- update `damsel-core/src/lib.rs`
- add `damsel-core/tests/project_model_contract.rs`

Types:

- `AnalysisProject`
- `AnalysisSource`
- `AddressSpace`
- `AnalysisAddress`
- `ObjectId`
- `Route`

Validation:

- deterministic JSON snapshots
- source id stability
- address-space conversion tests

### Slice 2: Project Builder In Mach-O

Files:

- add `damsel-macho/src/project.rs`
- update `damsel-macho/src/lib.rs`
- add `damsel-macho/tests/project_matrix.rs`

Responsibilities:

- create project from `BinaryImage`
- create project from `ProjectedBinaryImage`
- seed sources, segments, sections, symbols, imports, ObjC metadata, dyld
  metadata
- seed byte classifications

Validation:

- standalone fixture
- projected cache image fixture
- malformed input error propagation

### Slice 3: Fact Store And Byte Map

Files:

- add `damsel-core/src/facts.rs`
- add `damsel-core/src/byte_map.rs`
- add tests for fact precedence and byte range queries

Responsibilities:

- store facts with provenance/confidence
- store byte classifications
- support user overrides
- support compact feature-map ranges

Validation:

- byte map snapshot
- overlapping classification test
- user override precedence

### Slice 4: CLI Project Commands

Files:

- update `damsel-cli/src/main.rs`
- update `damsel-cli/src/output.rs`
- add CLI snapshots

Commands:

- `project create`
- `project status`
- `project sources`
- `project facts`
- `project byte-map`

Validation:

- JSON snapshots
- text snapshots
- invalid path errors
- stale schema error later

### Slice 5: Project UI API

Files:

- update `damsel-cli/src/ui.rs`
- add `damsel-cli/tests/ui_api.rs` cases

Routes:

- `POST /api/projects`
- `GET /api/projects/{project_id}`
- `GET /api/projects/{project_id}/sources`
- `GET /api/projects/{project_id}/byte-map`
- `GET /api/projects/{project_id}/feature-map`

Validation:

- project creation API test
- source list API test
- byte-map API test
- error payload tests

### Slice 6: Workbench Project Open

Files:

- update `DamselAPIClient.swift`
- update `WorkbenchStore.swift`
- add project models in `DamselModels.swift`
- add tests in `DamselAppCoreTests`

Behavior:

- register local path with project API instead of uploading whole file
- retain upload compatibility for browser mode
- show project/source summary
- preserve current disassembly view as a compatibility representation

Validation:

- Swift model decode tests
- store transition tests
- missing project/server error tests

### Slice 7: Function/Xref Index

Files:

- add `damsel-core/src/functions.rs`
- add `damsel-core/src/references.rs`
- add `damsel-macho/src/function_index.rs`
- add `damsel-macho/src/xref_index.rs`

Responsibilities:

- function seed collection
- full-function decode
- block/edge persistence
- xref/reverse-xref persistence
- caller/callee indexes

Validation:

- branch fixture
- jump-table fixture
- import fixture
- ObjC IMP seed fixture

### Slice 8: Workbench Navigation And Inspector

Files:

- add route/navigation models in Swift
- update `ContentView.swift`
- update `InspectorPane.swift`
- add navigator and problems panel views

Behavior:

- back/forward route stack
- selection by object id/route
- xref list in inspector
- problem list drawer

Validation:

- route decode tests
- store navigation tests
- UI smoke with fixture

### Slice 9: Shared-Cache Project

Files:

- extend `damsel-macho/src/shared_cache.rs`
- extend `damsel-macho/src/shared_cache_query.rs`
- add project builder support
- add CLI/API routes

Behavior:

- source registry for cache/subcaches/images
- image list as project objects
- dependency/provider/importer facts
- projected image analysis retains cache provenance

Validation:

- synthetic cache fixture
- provider/importer snapshot
- optional real-cache smoke

### Slice 10: Persistence

Files:

- add optional `damsel-store` crate or module once model stabilizes
- add migration tests
- add diagnostics export

Behavior:

- append-only project log first
- SQLite/embedded store later
- migrations preserve annotations
- large indexes page

Validation:

- reopen project
- migration
- stale analyzer version
- diagnostics redaction

## API Compatibility Plan

Keep:

- `/api/images`
- `/api/images/{image_id}/symbols`
- `/api/images/{image_id}/disasm`

Add:

- `/api/projects`
- `/api/projects/{project_id}`
- `/api/projects/{project_id}/sources`
- `/api/projects/{project_id}/jobs`
- `/api/projects/{project_id}/routes/resolve`
- `/api/projects/{project_id}/search`
- `/api/projects/{project_id}/functions`
- `/api/projects/{project_id}/xrefs`
- `/api/projects/{project_id}/byte-map`
- `/api/projects/{project_id}/feature-map`
- `/api/projects/{project_id}/annotations`
- `/api/projects/{project_id}/problems`

Deprecation posture:

- do not remove image APIs until Workbench and browser UI migrate
- keep CLI point queries stable even after they share internals
- prefer typed compatibility wrappers over duplicated analysis logic

## Current Test Gates To Reuse

Rust:

- `cargo test --workspace`
- `cargo test -p damsel-cli --test ui_api`
- `cargo test -p damsel-cli --test cli_snapshots`
- `cargo test -p damsel-macho`
- `cargo bench -p damsel-macho --bench decode_bench --no-run`

Swift:

- `swift test --package-path damsel-app`
- `./script/build_and_run.sh --verify-fixture`

Fixtures:

- `./fixtures/build-fixtures.sh --check`
- `./fixtures/tests/build-fixtures-parity.sh`

Optional:

- real shared-cache smoke through `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT`

New gates should add to these, not replace them.

## Deep Refactor Warnings

### WorkbenchStore Will Become Too Large

`WorkbenchStore` currently owns server state, file selection, target controls,
disassembly payload, symbol suggestions, filtering, validation, and selection.
That is manageable for one uploaded image. It will become too large for
projects, jobs, routes, annotations, search, graph, cache, and kernel browsers.

Planned split:

- keep one high-level coordinator
- move project state to `ProjectStore`
- move navigation stack to `NavigationStore`
- move jobs/progress to `JobStore`
- move search/palette to `SearchStore`
- move annotations to `AnnotationStore`

### UI Server Cannot Stay ImageStore-Only

`ImageStore` is intentionally simple. It should remain for compatibility, but
project work needs:

- project ids
- source ids
- job ids
- route ids
- paged query state
- cancellation
- persistence hooks

### Disassembly Rendering Cannot Be The Data Model

Current `DisassemblyResponse` is useful for displaying rows. Hopper-like
features need:

- function records
- instruction objects
- semantic spans
- byte ranges
- xrefs
- annotations
- problems
- graph records

Rendering should consume those records, not replace them.

### Cache Analysis Must Not Lose Session Context

The current cache disassembly path has a key advantage: projected image and
live shared-cache session coexist. Future project analysis must preserve that
relationship in provenance and source registry, otherwise provider/importer
links will regress.

## First Branch Recommendation

Detailed companion spec:

- [Wave 1 Project Foundation Implementation Spec](wave-1-project-foundation-spec.md)

Start with a branch that does only:

- core project/source/address/object models
- deterministic snapshots
- project creation from standalone `BinaryImage`
- initial byte classifications from sections/strings/symbol starts
- CLI `project create/status/sources`

Avoid in the first branch:

- Workbench UI overhaul
- SQLite
- decompiler
- debugger
- full cache project
- annotation editing

Reason:

- this creates the foundation while limiting blast radius
- it gives reviewable snapshots
- it preserves all existing point-query behavior
