# Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec

Status: implementation-grade architecture spec. This is not an implementation
claim.

Purpose: define the artifact intake pipeline Damsel needs to compete with
Hopper in the Apple systems lane. This covers how the app accepts, classifies,
registers, expands, projects, enriches, and remembers Apple reverse-engineering
inputs: standalone Mach-O files, universal binaries, dyld shared caches,
driver caches, kernel caches, kernel collections, fileset members, KDKs,
dSYMs, DWARF bundles, panic logs, IPSW/OTA-derived assets, app bundles, and
simulator/device-support artifacts.

This spec is separate from parser correctness. It answers a different product
question: when a user drags a messy real Apple artifact into Damsel, how does
the tool know what it is, what related files it needs, what it is allowed to
open, what can be extracted or projected, what sidecars match, what analysis
jobs should run, and what the user should see first?

The dedicated debug-info implementation companion,
[DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md),
continues the sidecar part of this intake model into dSYM/KDK identity,
matching, DWARF indexing, source-line routes, symbolication profiles, privacy,
and Workbench debug-source UX.

## Why This Exists

Hopper-like competitiveness is not only "can decode a Mach-O after somebody
hands us the exact right file." Hopper's public surface and changelog show a
tool that repeatedly invests in loading, extracting, navigating, and fixing
behavior around real-world containers:

- universal/FAT binaries
- files extracted from dyld shared caches
- dyld shared-cache path discovery
- iOS 16 dyld shared caches
- newer macOS dyld shared caches
- driver-cache dylibs
- iOS and macOS kernel caches
- files extracted from composite containers
- global versus local file offsets
- plugin file loaders
- loader options and progress reporting

Damsel already has strong Apple foundations: Mach-O loading, arm64/arm64e
analysis, shared-cache inventory, projected images, cache linkage, ObjC
metadata, and a native Workbench shell. The missing layer is a durable artifact
intake system that turns "open this thing" into a project graph with source
identity, access grants, extracted members, projections, sidecar matching,
analysis jobs, provenance, and user-visible status.

Without this layer:

- Workbench remains upload-oriented and file-at-a-time.
- KDK/dSYM/panic workflows stay scattered.
- IPSW and iOS workflows require terminal prep.
- dyld and driver cache workflows cannot present a coherent source graph.
- kernel collection members and kexts remain hidden behind parser details.
- diagnostics and evidence exports cannot prove what artifact was analyzed.
- performance work lacks clear source identity and paging boundaries.
- future plugins and external tools have no safe place to contribute members.

## Public Hopper Evidence

### FAQ Evidence

Hopper's FAQ says it can disassemble iOS binaries and explicitly draws a line
around App Store FairPlay DRM: iTunes-distributed apps are encrypted and the
user must provide an unencrypted binary before Hopper can analyze it.

Damsel implication:

- Damsel should support iOS artifacts, but it must not imply DRM bypass.
- Intake should classify encrypted or protected iOS inputs as unsupported with a
  clear reason.
- The Workbench should say "encrypted input" or "FairPlay/protected app" rather
  than pretending the parser failed generically.
- Legal or policy boundaries belong in intake problems and user-facing text.

### Changelog Evidence

Hopper's changelog includes multiple artifact-loading and Apple-container
entries:

- `6.1.0` adds an option to read dylibs from the driver cache.
- `6.0.12` improves method navigation for files extracted from the shared DYLD
  cache.
- `5.17.4` fixes Objective-C header export for files extracted from the DYLD
  shared cache.
- `5.15.4` fixes a hang on mangled files improperly extracted from the DYLD
  shared cache.
- `5.15.0` and `5.14.2` improve compatibility with newer macOS dyld shared
  caches.
- `5.11.5` improves newer dyld-cache handling, especially symbol reading.
- `5.10.0` adds support for unencrypted iOS and macOS kernel caches.
- `5.8.6` exports more symbols from the DYLD shared cache and improves
  extraction time.
- `5.7.8`, `5.7.7`, `5.7.6`, and `5.7.5` fix or improve old and new dyld
  cache loading, including iOS 16.
- `5.7.4` fixes a crash for some files extracted from a DYLD shared cache.
- `5.7.3` fixes hex editor behavior for files extracted from a FAT binary and
  adds a go-to option for local or global file offsets.
- `5.7.0` improves decompilation of Objective-C programs extracted from the
  DYLD cache.
- `5.6.1` improves DYLD cache path discovery on macOS Ventura.
- `5.5.1` fixes Objective-C protocol reading from a DYLD shared cache and
  improves executable selection dialogs.
- `5.5.0` adds a quick way to load a library from the current DYLD cache and
  improves executable dialogs.

Damsel implication:

- extracted/projected members must retain original-container routes
- source identity must distinguish root container, subcache, member, and
  projected image
- local and global offsets must coexist in routes, hex, graph, xrefs, and
  error messages
- dyld cache and driver cache support is a UX and project-model feature, not a
  parser-only feature
- old/new OS-version compatibility belongs in source classification and
  validation, with exact artifact identity recorded

### SDK Loader Evidence

Hopper's SDK quick tour describes a file loader architecture:

- file loader plugins are consulted when a file is opened
- a loader returns `DetectedFileType` records
- a FAT Mach-O loader can return one detected type per architecture
- detected records carry file descriptions and CPU/address-width hints
- loaders can request user parameters such as address fields, checkboxes, and
  CPU selectors
- raw binary loaders can lower their priority
- selected loaders create segments and sections like an operating system would
- loaders can add entry points, potential procedures, labels, and comments
- loaders can load debug data after the file is loaded
- loaders should report progress through a callback

Damsel implication:

- source classification should produce multiple candidates, not a single
  yes/no result
- artifact opening should be a planning step before analysis starts
- loaders must be able to ask for parameters without blocking the whole app
- debug sidecars should attach through the same project/source model as
  binaries
- extraction/projection progress is a job with a cancellable UI surface
- raw or ambiguous inputs should be allowed, but ranked lower and marked as
  less certain

### Product Evidence

Hopper's homepage and tutorial position the tool as an interactive native
workspace where opening a file leads to automatic analysis, multiple
representations, navigation, byte typing, references, graph, hex, pseudocode,
scripts, and plugins.

Damsel implication:

- opening an artifact should create a navigable project immediately
- intake must seed the same routes that assembly, graph, hex, search,
  inspector, automation, and reports use later
- the app should show artifact identity and partial analysis progress before
  deep analysis finishes

## Apple And Ecosystem Evidence

### Apple Security And Privacy

Apple documents macOS file access controls for user-protected locations such as
Documents, Downloads, Desktop, iCloud Drive, network volumes, removable
volumes, and broader privacy settings. Damsel's artifact intake must work with
user-selected file access and security-scoped access where sandboxing is used.

Planning impact:

- intake is coupled to source access grants
- paths and bookmarks are privacy-sensitive
- diagnostics and evidence exports cannot store raw source paths by default

### Kernel Debug Kits And dSYMs

Apple distributes Kernel Debug Kits for matching macOS kernels, and KDK
material is commonly paired with dSYMs and DWARF data for kernel symbolication
and type information. Availability and packaging vary by OS release and
installation environment.

Planning impact:

- KDKs are sidecar artifact roots, not one flat file
- matching must use UUID, architecture, OS build, and bundle metadata
- a mismatched KDK is a typed problem, not a warning that can be ignored
- wrong symbols are worse than missing symbols

### IPSW, OTA, And Device Support Reality

iOS/macOS system analysis often starts from artifacts that are not immediately
loadable Mach-O files:

- IPSW archives
- OTA payloads
- extracted dyld shared caches
- extracted kernel collections or kernel caches
- Xcode DeviceSupport directories
- simulator runtimes
- app bundles with embedded frameworks
- private local corpora

Planning impact:

- Damsel should accept directories and container roots, not only files
- intake should produce an artifact graph and member list
- external extraction tools may be useful, but their output must be captured as
  provenance rather than silently treated as original input
- unsupported encrypted inputs must be explicit

## Current Damsel Baseline

Current repo-grounded strengths:

- `damsel-macho::loader::load` reads a file path and creates a `BinaryImage`.
- `damsel-macho::loader::load_bytes` supports in-memory images for the UI and
  tests.
- `loader.rs` selects Mach-O slices and records available slice descriptors.
- `load_from_source` collects segments, sections, symbols, imports,
  relocations, Objective-C metadata, dyld metadata, platform, slice identity,
  and bytes.
- `damsel-macho::shared_cache::inspect_shared_cache` discovers synthetic and
  real dyld shared caches.
- `SharedCacheSession` preserves real root path, member metadata, projected
  images, import relations, export relations, and cache image resolution.
- `SharedCacheSession::project_image` reuses existing per-image analysis by
  projecting a cache image into a `ProjectedBinaryImage`.
- Existing CLI cache commands expose image inventory, projection-adjacent
  metadata, symbols, imports, ObjC, disassembly, dependencies, dependents,
  providers, importers, and reexports.
- Existing docs already define project/source/address/fact models as the next
  foundation.

Current limitations:

- no unified artifact classification step
- no durable source registry in implemented code
- no intake session or registration plan model
- no directory/container intake path for KDKs, dSYMs, app bundles, IPSWs, or
  extracted asset roots
- no driver-cache source kind in implemented code
- no kernel collection/fileset intake implementation
- no sidecar matching workflow implemented
- no panic log attachment implementation
- no Workbench artifact overview for source graphs
- no external extraction tool receipt model
- no legal/protected-input classification
- no resumable intake jobs
- no per-artifact acquisition receipts for evidence packages

## Product Thesis

Artifact intake should feel like the first screen of the product, not a
precondition the user satisfies in Terminal.

The user should be able to drag in:

- a Mach-O binary
- a universal binary
- a dyld shared cache directory or root member
- a driver cache
- a kernel collection
- a KDK bundle
- a dSYM bundle
- a panic log
- an extracted IPSW asset root
- an app bundle
- a folder of related sidecars

Damsel should answer:

- What is this?
- What members or slices does it contain?
- What related sidecars are nearby?
- What needs user selection or permission?
- What is unsupported, encrypted, missing, or ambiguous?
- What can be shown immediately?
- What jobs will run in the background?
- What source graph and address spaces were created?
- What exact evidence supports every match?

## Design Principles

### Intake Produces A Plan Before Heavy Analysis

Opening a source should produce a `SourceRegistrationPlan`:

- detected candidates
- confidence
- artifact kinds
- required parameters
- related files
- sidecar candidates
- extraction/projection steps
- privacy class
- expected jobs
- blocking problems
- non-blocking problems

The user should see the plan quickly. Expensive parsing and analysis can follow
as cancellable jobs.

### Original Artifacts Are Immutable

Damsel should never mutate original inputs during intake. Extracted or patched
outputs are derived artifacts with separate source ids and provenance.

Rules:

- root artifacts are read-only
- extracted members are derived from a root artifact
- projected images are analysis views, not independent truth
- user edits are project transactions, not source mutations
- patching stays copy-on-write and out of first intake scope

### Artifact Identity Beats Path Identity

Paths move. Artifact identity should combine:

- source kind
- content hash
- file size
- Mach-O UUID where present
- dyld cache UUID where present
- fileset entry id where present
- install name or bundle id where present
- architecture and subtype
- platform and minimum OS where present
- build version where present
- parent container identity
- extraction recipe where derived

Paths remain useful for access and display, but they are not the only identity.

### Containers And Members Are Both Sources

A dyld shared cache is a source. A projected Foundation image is also a source.
A kernel collection is a source. A fileset member is also a source. A KDK is a
source family. A dSYM is a sidecar source.

Every source can have:

- parent source
- child sources
- address spaces
- analysis facts
- sidecar matches
- extraction receipts
- privacy class
- availability state

### Unsupported Inputs Are Product States

Unsupported does not mean "crash" or "generic parse error." Intake should
produce typed problems:

- protected iOS app
- encrypted kernel cache
- unsupported architecture
- missing subcache
- stale source grant
- corrupted archive
- ambiguous dyld root
- mismatched KDK
- mismatched dSYM
- missing local symbols sidecar
- extraction tool unavailable
- external extraction failed
- user canceled parameter prompt

### External Tools Are Adapters

Damsel can eventually integrate external extractors, but they must be wrapped
as adapters with receipts:

- tool name
- tool version
- command shape
- input hashes
- output hashes
- stderr summary
- privacy classification
- success/failure status

The project should never silently trust an external tool output without
recording how it was produced.

### Intake Is Incremental

Large Apple artifacts should not block the Workbench:

- classify first
- show member lists early
- let the user choose a target image/member
- run deep analysis only for selected or prioritized objects
- page large lists
- cache fingerprints and manifests
- resume interrupted intake where safe

## Target Architecture

```text
Workbench open/drop command
  |
  v
Source access grant
  |
  v
Artifact classifier
  |
  v
Source registration plan
  |
  +-- user parameter prompts
  +-- protected/unsupported checks
  +-- related-file discovery
  +-- sidecar candidate discovery
  +-- extraction/projection recipes
  |
  v
Project source registry
  |
  +-- root sources
  +-- member sources
  +-- sidecar sources
  +-- derived/projected sources
  |
  v
Intake job scheduler
  |
  +-- manifest extraction
  +-- lightweight metadata parse
  +-- sidecar matching
  +-- projection generation
  +-- analysis seed jobs
  |
  v
Artifact overview, navigators, routes, and analysis database
```

## Core Model Types

### Artifact Intake Session

```rust
pub struct ArtifactIntakeSession {
    pub id: String,
    pub project_id: String,
    pub created_at: String,
    pub actor: Actor,
    pub requested_inputs: Vec<IntakeInput>,
    pub registration_plans: Vec<SourceRegistrationPlan>,
    pub selected_plan_ids: Vec<String>,
    pub jobs: Vec<IntakeJobId>,
    pub problems: Vec<IntakeProblemId>,
}
```

Rules:

- an intake session can cover one file, many files, or a directory root
- sessions are auditable
- canceled sessions preserve problems and user decisions
- repeated intake should deduplicate sources by identity

### Intake Input

```rust
pub struct IntakeInput {
    pub id: String,
    pub grant_id: String,
    pub display_name: String,
    pub input_kind: IntakeInputKind,
    pub path_redacted: RedactedPath,
    pub file_size: Option<u64>,
    pub content_hash: Option<String>,
    pub privacy_class: SourcePrivacyClass,
}
```

Input kinds:

- file
- directory
- app bundle
- KDK bundle
- dSYM bundle
- panic log
- archive
- device support root
- extracted asset root
- unknown

### Artifact Candidate

```rust
pub struct ArtifactCandidate {
    pub id: String,
    pub input_id: String,
    pub artifact_kind: ArtifactKind,
    pub confidence: FactConfidence,
    pub detector: String,
    pub display_name: String,
    pub architecture: Option<Architecture>,
    pub platform: Option<Platform>,
    pub address_width: Option<u8>,
    pub container_role: Option<ContainerRole>,
    pub requires_parameters: Vec<LoaderParameterRequest>,
    pub blocking_problems: Vec<IntakeProblem>,
    pub non_blocking_problems: Vec<IntakeProblem>,
}
```

Artifact kinds:

- standalone Mach-O
- universal Mach-O
- Mach-O slice
- raw binary
- dyld shared-cache root
- dyld shared-cache subcache
- dyld shared-cache symbols sidecar
- projected dyld image
- driver cache
- driver cache member
- kernelcache
- kernel collection
- fileset member
- kext bundle
- app bundle
- framework bundle
- dSYM bundle
- DWARF file
- KDK bundle
- panic log
- IPSW archive
- OTA archive
- extracted IPSW root
- simulator runtime
- Xcode DeviceSupport root
- encrypted/protected app
- unsupported archive
- unknown

### Loader Parameter Request

```rust
pub struct LoaderParameterRequest {
    pub id: String,
    pub label: String,
    pub parameter_kind: LoaderParameterKind,
    pub default_value: Option<JsonValue>,
    pub required: bool,
    pub reason: String,
}
```

Parameter kinds:

- base address
- first visible address
- architecture selector
- CPU/subtype selector
- endianness selector
- load as raw binary
- associated root path
- sidecar selection
- symbolication profile selection
- extraction mode
- privacy confirmation

This mirrors Hopper SDK evidence that loaders can request address fields,
checkboxes, and CPU selectors while keeping Damsel's API more explicit.

### Source Registration Plan

```rust
pub struct SourceRegistrationPlan {
    pub id: String,
    pub input_id: String,
    pub candidate_id: String,
    pub source_kind: SourceKind,
    pub root_identity: ArtifactIdentity,
    pub child_sources: Vec<PlannedChildSource>,
    pub sidecar_candidates: Vec<SidecarCandidate>,
    pub extraction_recipes: Vec<ExtractionRecipe>,
    pub projection_recipes: Vec<ProjectionRecipe>,
    pub initial_jobs: Vec<PlannedIntakeJob>,
    pub privacy_class: SourcePrivacyClass,
    pub estimated_cost: IntakeCostEstimate,
    pub status: RegistrationPlanStatus,
}
```

Plan statuses:

- ready
- needs user parameters
- needs source access
- needs sidecar selection
- unsupported
- blocked by protected input
- blocked by malformed input
- blocked by missing member

### Artifact Identity

```rust
pub struct ArtifactIdentity {
    pub id: String,
    pub kind: ArtifactKind,
    pub content_hash: Option<String>,
    pub file_size: Option<u64>,
    pub uuid: Option<String>,
    pub install_name: Option<String>,
    pub bundle_identifier: Option<String>,
    pub bundle_version: Option<String>,
    pub architecture: Option<Architecture>,
    pub cpu_subtype: Option<u32>,
    pub platform: Option<Platform>,
    pub minimum_os: Option<String>,
    pub build_version: Option<String>,
    pub dyld_cache_uuid: Option<String>,
    pub fileset_entry_id: Option<String>,
    pub parent_artifact_id: Option<String>,
    pub derivation: Option<ArtifactDerivation>,
}
```

Identity rules:

- deterministic where source bytes are available
- not based only on filename
- parent identity is required for extracted/projected members
- KDK/dSYM identities include metadata and contained UUIDs
- panic log identity includes redacted content hash and parsed build hints

### Artifact Derivation

```rust
pub struct ArtifactDerivation {
    pub derivation_kind: DerivationKind,
    pub source_artifact_id: String,
    pub recipe_id: String,
    pub tool_receipt_id: Option<String>,
    pub parameters: JsonValue,
}
```

Derivation kinds:

- extracted member
- projected image
- selected universal slice
- decompressed archive member
- symbol sidecar association
- KDK profile
- dSYM profile
- panic-log attachment
- future patched revision

### Container Manifest

```rust
pub struct ContainerManifest {
    pub id: String,
    pub source_id: String,
    pub container_kind: ArtifactKind,
    pub members: Vec<ContainerMember>,
    pub relationships: Vec<ContainerRelationship>,
    pub missing_members: Vec<MissingMember>,
    pub problems: Vec<IntakeProblem>,
}
```

Member fields:

- member id
- display name
- install name
- role
- architecture
- platform
- vmaddr range
- file offset range
- source id if registered
- extraction/projection recipe id
- local/global offset transforms

Relationships:

- root to subcache
- root to symbols sidecar
- cache to image
- kernel collection to fileset member
- KDK to contained dSYM
- app bundle to framework
- IPSW root to extracted asset

### Sidecar Candidate

```rust
pub struct SidecarCandidate {
    pub id: String,
    pub sidecar_kind: SidecarKind,
    pub candidate_identity: ArtifactIdentity,
    pub target_source_id: Option<String>,
    pub match_status: SidecarMatchStatus,
    pub evidence: Vec<SidecarMatchEvidence>,
}
```

Sidecar kinds:

- dSYM
- DWARF file
- KDK bundle
- dyld cache symbols
- local symbols file
- BCSymbolMap-like future sidecar
- source-map future sidecar
- panic log

Match statuses:

- exact
- likely
- ambiguous
- mismatched
- stale
- unsupported
- user selected
- rejected

Evidence:

- UUID match
- architecture match
- build version match
- install name match
- bundle id match
- file path hint
- contained dSYM match
- panic log kext UUID match
- user assertion

### Symbolication Profile

```rust
pub struct SymbolicationProfile {
    pub id: String,
    pub target_source_id: String,
    pub sidecar_source_ids: Vec<String>,
    pub profile_kind: SymbolicationProfileKind,
    pub match_status: SidecarMatchStatus,
    pub imported_names: usize,
    pub imported_types: usize,
    pub imported_line_records: usize,
    pub problems: Vec<IntakeProblem>,
}
```

Profile kinds:

- dSYM
- KDK
- dyld symbols
- local symbols
- mixed user-selected

Rules:

- one source may have multiple candidate profiles
- active profile is explicit
- mismatched profiles do not silently apply
- imported facts preserve source profile id

### Extraction Recipe

```rust
pub struct ExtractionRecipe {
    pub id: String,
    pub recipe_kind: ExtractionRecipeKind,
    pub input_source_id: String,
    pub output_source_kind: SourceKind,
    pub member_selector: MemberSelector,
    pub parameters: JsonValue,
    pub required_tool: Option<ExternalToolRequirement>,
    pub expected_outputs: Vec<ExpectedOutput>,
}
```

Recipe kinds:

- dyld cache image projection
- dyld cache image extraction
- driver cache member extraction
- fileset member projection
- universal slice selection
- app bundle framework discovery
- archive extraction
- IPSW asset discovery
- OTA asset discovery
- KDK profile indexing
- dSYM DWARF indexing

Rules:

- projection and extraction are distinct
- extraction writes a derived artifact only under a project-managed cache or
  user-selected destination
- projection can be virtual and byte-backed by parent source
- recipes are resumable where outputs are content-addressed

### Projection Recipe

```rust
pub struct ProjectionRecipe {
    pub id: String,
    pub projection_kind: ProjectionKind,
    pub parent_source_id: String,
    pub member_id: String,
    pub address_space_transform: AddressSpaceTransform,
    pub byte_provider: ByteProviderDescriptor,
    pub provenance: JsonValue,
}
```

Projection kinds:

- shared-cache image
- fileset member
- universal slice
- driver-cache member
- extracted app framework
- future patched revision

### External Tool Receipt

```rust
pub struct ExternalToolReceipt {
    pub id: String,
    pub tool_name: String,
    pub tool_version: Option<String>,
    pub tool_path_redacted: RedactedPath,
    pub adapter_version: String,
    pub input_hashes: Vec<String>,
    pub output_hashes: Vec<String>,
    pub status: ExternalToolStatus,
    pub stderr_redacted: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}
```

Rules:

- no raw commands with private paths in diagnostics by default
- receipts are facts with provenance
- outputs are registered as derived sources
- failed receipts produce typed intake problems

### Intake Problem

```rust
pub struct IntakeProblem {
    pub id: String,
    pub severity: ProblemSeverity,
    pub code: IntakeProblemCode,
    pub target: ObjectId,
    pub message: String,
    pub recovery: Option<String>,
    pub evidence: Vec<Provenance>,
}
```

Problem codes:

- `unsupported_input_kind`
- `protected_ios_app`
- `encrypted_kernel_cache`
- `unsupported_archive`
- `unsupported_architecture`
- `malformed_fat_header`
- `ambiguous_universal_slice`
- `ambiguous_dyld_root`
- `missing_subcache`
- `missing_symbols_sidecar`
- `driver_cache_unsupported`
- `kernel_collection_unsupported`
- `fileset_member_parse_failed`
- `kdk_not_found`
- `kdk_uuid_mismatch`
- `dsym_uuid_mismatch`
- `panic_log_build_mismatch`
- `source_access_denied`
- `security_bookmark_stale`
- `external_tool_missing`
- `external_tool_failed`
- `user_parameters_required`
- `user_canceled_intake`

## Intake Pipeline

### Stage 1: Grant And Fingerprint

Input:

- file URL, directory URL, or drag/drop item

Actions:

- create source access grant
- classify privacy class
- gather file size
- gather partial hash quickly when full hash is expensive
- full hash in background for large artifacts
- record path redacted form

Output:

- `IntakeInput`
- first `ArtifactIdentity` shell
- problems for access denial or unsupported locations

### Stage 2: Lightweight Classification

Actions:

- inspect magic bytes
- inspect bundle directories
- inspect archive headers
- inspect Mach-O load commands
- inspect dyld cache magic
- inspect KDK/dSYM bundle metadata
- inspect panic log text shape
- inspect app bundle structure
- inspect extracted asset root layout

Output:

- `ArtifactCandidate` list
- ranked plan choices
- required parameter prompts

Rules:

- no deep analysis
- no full archive extraction
- no entire large file copied into Swift memory
- ambiguous candidates are preserved

### Stage 3: User Decision And Parameter Collection

Examples:

- choose arm64e slice from universal binary
- choose raw base address
- choose dyld cache image to project first
- choose whether to search sibling directory for sidecars
- choose KDK/dSYM candidate
- confirm encrypted/protected input cannot be analyzed
- choose project-persistent versus session-only access

Output:

- selected `SourceRegistrationPlan`
- user-supplied loader parameters

### Stage 4: Source Registration

Actions:

- insert root source
- insert parent/child relationships
- insert address spaces
- insert source availability status
- insert source privacy class
- insert artifact identity
- insert intake receipt

Output:

- project source graph
- artifact overview route

### Stage 5: Container Manifest Extraction

For dyld caches:

- root member
- subcaches
- symbols sidecar
- image list
- mapping list
- install names
- UUIDs
- local symbol availability
- dependency/import/export overview

For driver caches:

- cache identity
- candidate dylib/member list
- member offsets
- member identity
- unsupported-member reasons

For kernel collections:

- fileset entries
- kext/member names
- vmaddr ranges
- UUID/build hints
- prelink info where available
- potential kext bundle identities

For app bundles:

- main executable
- embedded frameworks
- extensions
- dSYM candidates
- Info.plist metadata
- encrypted/protected hints

For KDK/dSYM:

- contained DWARF files
- UUIDs
- architecture slices
- bundle metadata
- OS build hints

For panic logs:

- panic string
- kernel version/build
- slide
- frames
- kext UUID/build list
- date and OS hints

### Stage 6: Sidecar Discovery And Matching

Search scopes:

- same directory
- selected sidecar directory
- configured local corpus roots
- KDK roots selected by user
- project source graph
- current macOS system roots where policy allows

Match:

- UUID
- architecture
- build version
- install name
- bundle id
- source path hints
- panic log kext list

Output:

- sidecar candidates
- active profile proposals
- mismatch problems

### Stage 7: Projection And Extraction

Virtual projection first:

- dyld shared-cache image views
- fileset member views
- universal slice views
- driver-cache member views where supported

Physical extraction only when:

- user requests export
- a downstream tool requires a real file
- derived source cache is enabled
- privacy policy permits

Output:

- projected source ids
- extracted source ids
- derivation receipts

### Stage 8: Analysis Seed Scheduling

Seed jobs:

- artifact overview
- source manifest
- symbol/string indexes
- section/segment byte classification
- ObjC metadata summary
- dyld metadata summary
- function seed collection
- sidecar import
- KDK/dSYM import
- panic frame route resolution
- selected member deep analysis

Priority:

1. selected source overview
2. user-visible member list
3. search index seed
4. selected function/member analysis
5. sidecar matching
6. broad background enrichment

### Stage 9: Workbench Overview

Immediate first viewport:

- artifact kind
- source graph
- architecture/platform
- slices or members
- sidecar status
- privacy/source access status
- problems
- background jobs
- suggested next actions

No blank loading wall. A huge dyld cache should show something useful before
deep analysis completes.

## Workbench UX

### Open Artifact

Commands:

- Open Artifact
- Open Binary
- Open Shared Cache
- Open Driver Cache
- Open Kernel Collection
- Open App Bundle
- Open Extracted Asset Root
- Attach KDK
- Attach dSYM
- Attach Panic Log

Behavior:

- all commands route through the same intake session
- specialized commands pre-filter candidate kinds
- generic Open Artifact ranks all candidates
- drag/drop uses Open Artifact

### Intake Review Sheet

Displayed when needed:

- detected candidates
- confidence
- required decisions
- sidecar candidates
- unsupported/protected warnings
- privacy class
- estimated cost

Actions:

- Open Selected
- Open Read-Only
- Open Session Only
- Attach Sidecars
- Skip Sidecars
- Cancel

### Artifact Overview

Sections:

- identity
- source graph
- members/slices
- sidecars
- analysis jobs
- problems
- suggested routes
- privacy/source access state

Examples:

- dyld cache overview shows images, subcaches, symbols sidecar, top images, and
  local-symbol status
- kernel collection overview shows fileset members, kexts, KDK match state, and
  panic log status
- app bundle overview shows main executable, frameworks, extensions, and
  protection/encryption status

### Sidecar Association UI

User can:

- attach KDK
- attach dSYM
- select among candidates
- reject mismatch
- mark as user-selected
- view evidence
- detach active profile
- keep sidecar registered but inactive

Rules:

- exact UUID match can auto-propose but should remain visible
- mismatches require explicit override and are labeled user assertions
- overrides do not erase mismatch evidence

### Source Graph UI

Show:

- root artifacts
- derived members
- projections
- sidecars
- panic logs
- symbolication profiles
- extraction receipts

Use routes, not raw file paths:

- `source:root`
- `source:dyld-cache:arm64e`
- `source:cache-image:Foundation`
- `source:kcollection:kernel`
- `source:kext:com.apple.iokit.IOUSBHostFamily`
- `sidecar:kdk:23E224`
- `panic:2026-05-26:local`

### Error And Recovery UI

Examples:

- "This iOS app appears protected. Damsel cannot analyze encrypted app-store
  binaries. Provide an unencrypted local binary."
- "This KDK does not match the kernel collection UUID."
- "The dyld shared cache root is missing a required subcache."
- "This source grant is stale. Locate the file again."
- "The selected directory looks like an IPSW extraction root, but no dyld cache
  or kernel artifact was found."

Each recovery should be concrete and local.

## CLI Surface

### Project Intake

```sh
damsel project ingest <path>
damsel project ingest <path> --kind dyld-cache
damsel project ingest <path> --kind kernel-collection
damsel project ingest <path> --sidecars <path>
damsel project ingest <path> --json
```

Outputs:

- intake session id
- source registration plan
- source ids
- member summary
- problems
- next suggested commands

### Source Listing

```sh
damsel project sources <project>
damsel project source <project> <source-id>
damsel project source-graph <project>
```

### Sidecar Attachment

```sh
damsel project attach-dsym <project> <path>
damsel project attach-kdk <project> <path>
damsel project attach-panic <project> <path>
damsel project sidecars <project>
```

### Manifest And Projection

```sh
damsel project manifest <project> <source-id>
damsel project project-member <project> <member-id>
damsel project extract-member <project> <member-id> --output <path>
```

### Compatibility Commands

Existing commands remain:

- standalone image commands
- `cache info`
- `cache images`
- `cache image`
- `cache exports`
- `cache lookup-address`
- `cache resolve-symbol`
- `cache sections`
- `cache symbols`
- `cache imports`
- `cache dyld`
- `cache objc`
- `cache disasm`
- `cache image-deps`
- `cache dependents`
- `cache symbol-providers`
- `cache symbol-importers`
- `cache reexports`

New project commands should call into the same lower-level loaders instead of
forking behavior.

## API Surface

```text
POST /api/projects/{project_id}/intake
GET  /api/projects/{project_id}/intake/{session_id}
POST /api/projects/{project_id}/intake/{session_id}/select-plan
GET  /api/projects/{project_id}/sources
GET  /api/projects/{project_id}/sources/{source_id}
GET  /api/projects/{project_id}/source-graph
GET  /api/projects/{project_id}/sources/{source_id}/manifest
POST /api/projects/{project_id}/sources/{source_id}/project-member
POST /api/projects/{project_id}/sidecars/attach
GET  /api/projects/{project_id}/sidecars
POST /api/projects/{project_id}/panic-logs/attach
GET  /api/projects/{project_id}/artifact-overview
```

### Intake Request

```json
{
  "grant_ids": ["grant:1"],
  "preferred_kind": "auto",
  "sidecar_search": {
    "enabled": true,
    "scopes": ["same_directory", "selected_roots"]
  },
  "privacy": {
    "persistent_access": false,
    "include_raw_paths_in_project": false
  }
}
```

### Intake Response

```json
{
  "session_id": "intake:1",
  "plans": [
    {
      "id": "plan:1",
      "artifact_kind": "dyld_shared_cache_root",
      "confidence": "strong",
      "display_name": "dyld_shared_cache_arm64e",
      "status": "ready",
      "estimated_cost": {
        "classification_ms": 32,
        "manifest_ms": 850,
        "full_analysis": "background"
      }
    }
  ],
  "problems": []
}
```

### Artifact Overview Response

```json
{
  "project_id": "project:1",
  "root_sources": [],
  "member_counts": {
    "images": 1742,
    "subcaches": 4,
    "sidecars": 1,
    "problems": 0
  },
  "suggested_routes": [],
  "jobs": [],
  "sidecar_status": []
}
```

## Current-Code Integration

### `damsel-macho::loader`

Preserve:

- `load`
- `load_bytes`
- `select_slice`
- segment/section/symbol/import/ObjC/dyld extraction

Extend:

- expose lightweight classification before full load
- expose universal slice candidate records
- expose protected/encrypted hints where detectable
- return registration-friendly source identity
- avoid full byte copies in project mode

Suggested new functions:

```rust
pub fn classify_macho_source(handle: &SourceAccessHandle) -> Result<Vec<ArtifactCandidate>>;
pub fn plan_macho_registration(candidate: &ArtifactCandidate) -> Result<SourceRegistrationPlan>;
pub fn load_registered_source(project: &AnalysisProject, source_id: SourceId) -> Result<BinaryImage>;
```

### `damsel-macho::shared_cache`

Preserve:

- `inspect_shared_cache`
- `SharedCacheSession`
- real/synthetic session split
- `project_image`
- linkage query methods

Extend:

- return `ContainerManifest`
- register root, subcache, symbols sidecar, and projected images as sources
- expose missing subcache as intake problem
- expose local/global offset transforms
- model driver-cache source separately if format detection supports it

### Kernel Loader

Add:

- fileset Mach-O detection
- kernel collection source kind
- fileset member manifest
- kext identity extraction
- member projection recipe
- KDK/dSYM sidecar match entrypoints

Do not make kernel parsing a side path that bypasses the project source graph.

### KDK/dSYM Importer

Add:

- bundle scanning
- contained DWARF UUID extraction
- architecture matching
- build metadata extraction
- symbolication profile generation
- mismatch problem records

### Panic Log Parser

Add:

- panic text source kind
- privacy classification
- kernel build extraction
- slide extraction
- frame extraction
- kext UUID extraction
- frame route resolution

### `damsel-cli`

Add:

- project intake commands
- JSON snapshots for registration plans
- source graph rendering
- sidecar attach commands
- artifact overview command

### `damsel-app`

Add:

- Open Artifact command
- intake review sheet
- artifact overview route
- source graph view
- sidecar association view
- progress and problem integration
- source grant integration

Do not let Swift duplicate binary classification logic. Swift requests
classification and renders the plan.

## Implementation Phases

### Phase I0: Intake Model Skeleton

Deliverables:

- `ArtifactIntakeSession`
- `IntakeInput`
- `ArtifactCandidate`
- `ArtifactIdentity`
- `SourceRegistrationPlan`
- `IntakeProblem`
- serde snapshot tests

Validation:

- standalone Mach-O candidate snapshot
- universal Mach-O candidate snapshot
- unknown input snapshot
- protected input negative fixture if available

### Phase I1: Lightweight Mach-O And Universal Intake

Deliverables:

- classify standalone Mach-O
- classify universal Mach-O
- expose slice candidates
- create source registration plan
- create first artifact overview

Validation:

- arm64 fixture
- arm64e fixture
- universal fixture
- unsupported architecture fixture
- malformed fat fixture

### Phase I2: Project Source Registration

Deliverables:

- project source registry integration
- source identities
- parent/child source relationships
- source availability
- path grant integration
- compatibility bridge to current loaders

Validation:

- project ingest standalone fixture
- reopen project source graph
- missing source problem
- hash-changed problem

### Phase I3: dyld Shared Cache Intake

Deliverables:

- root/subcache/symbols sidecar registration
- cache manifest
- image member list
- projected image recipe
- local/global offset transforms
- initial artifact overview

Validation:

- synthetic single cache
- synthetic split cache
- missing subcache
- symbols sidecar present/absent
- projected image route snapshot
- existing cache command compatibility

### Phase I4: App Bundle And Extracted Asset Root Intake

Deliverables:

- app bundle detection
- framework/extension member discovery
- protected/encrypted app classification where detectable
- extracted IPSW root scan
- simulator runtime and DeviceSupport root scan where practical

Validation:

- local fixture app bundle
- embedded framework fixture
- unsupported/protected fixture metadata
- extracted asset root fixture

### Phase I5: Kernel Collection And Driver Cache Intake

Deliverables:

- kernel collection candidate
- fileset member manifest
- driver-cache candidate
- driver-cache member plan where supported
- kext identity seed
- kernel source graph route

Validation:

- synthetic fileset fixture
- malformed fileset fixture
- driver-cache unsupported fixture
- kernel member projection snapshot

### Phase I6: KDK And dSYM Sidecar Matching

Deliverables:

- dSYM bundle scanner
- DWARF UUID extraction
- KDK bundle scanner
- contained dSYM/DWARF indexing
- sidecar candidate table
- symbolication profile model
- mismatch problem records

Validation:

- exact dSYM match
- dSYM mismatch
- KDK exact match
- KDK build mismatch
- ambiguous sidecar candidates
- user-selected sidecar override

### Phase I7: Panic Log Attachment

Deliverables:

- panic log source kind
- panic parser
- frame records
- slide records
- kext build/UUID records
- route resolution job
- panic overview route

Validation:

- panic text fixture
- slide extraction fixture
- unresolved frame fixture
- KDK-enriched frame fixture
- privacy redaction fixture

### Phase I8: External Tool Adapter Receipts

Deliverables:

- external tool profile model
- receipt model
- derived source registration
- output hash capture
- redacted stderr capture

Validation:

- missing tool fixture
- failed tool fixture
- successful toy extractor fixture
- receipt redaction fixture

### Phase I9: Workbench Intake UX

Deliverables:

- Open Artifact command
- drag/drop intake
- intake review sheet
- artifact overview
- source graph
- sidecar association
- source problem recovery

Validation:

- Swift model decode tests
- Workbench intake smoke with Mach-O fixture
- Workbench dyld cache fixture smoke
- sidecar mismatch UI state test
- no huge-file `Data` upload in project mode

## Validation Matrix

| Requirement | Evidence |
| --- | --- |
| Intake produces candidate plans before analysis | candidate snapshot tests |
| Universal binaries expose slice choices | universal fixture |
| Unknown input is explicit | unknown fixture |
| Protected iOS app boundary is explicit | protected-input negative fixture or documented manual fixture |
| dyld root/subcache/symbols graph is preserved | synthetic split cache snapshot |
| projected cache image has parent provenance | projected source snapshot |
| local/global offsets coexist | route/address conversion tests |
| kernel collection members are sources | synthetic fileset snapshot |
| KDK/dSYM mismatch is typed | mismatch fixtures |
| panic frames route to project objects | panic fixture |
| external extraction is receipted | toy adapter fixture |
| Workbench shows overview before deep analysis | UI smoke |
| diagnostics redact raw paths | diagnostic export fixture |

## Corpus Needs

Checked-in fixtures:

- tiny Mach-O
- universal Mach-O
- unsupported slice
- malformed fat header
- app bundle with embedded framework
- synthetic dyld cache
- split synthetic dyld cache
- missing subcache
- synthetic symbols sidecar
- synthetic fileset/kernel collection
- synthetic dSYM with matching UUID
- synthetic dSYM mismatch
- panic log text
- external tool toy output

Opt-in local fixtures:

- host dyld shared cache
- host dyld cache symbols sidecar if present
- local driver cache if available
- local kernel collection if available
- installed KDK
- real dSYM bundle
- extracted IPSW asset root
- app bundle supplied by user

Each opt-in run records:

- date
- OS version
- artifact kind
- redacted root path
- content identity
- command
- pass/fail
- redaction policy

## Non-Goals For First Implementation

- no DRM circumvention
- no encrypted iOS app decryption
- no automatic internet downloads of IPSWs or KDKs
- no mutation of original artifacts
- no full kernel decompiler
- no remote device extraction
- no generic archive manager
- no broad non-Apple container support
- no native plugin loader dependency

## Completion Criteria

This subsystem is implementation-ready when:

- artifact kinds and source identities are modeled
- intake sessions and registration plans are modeled
- dyld, kernel, sidecar, panic, and app-bundle flows are specified
- Workbench intake UX has concrete surfaces
- CLI/API endpoints are named
- validation fixtures are listed
- source access and diagnostics policies are linked

This subsystem is implemented only when:

- project-mode Workbench opening goes through source registration
- standalone and universal Mach-O intake snapshots pass
- dyld cache source graph snapshots pass
- sidecar mismatch fixtures pass
- panic log attachment fixtures pass
- diagnostics do not leak raw paths by default
- existing cache commands remain compatible
- UI smoke proves artifact overview appears before deep analysis completes

## Source Links

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper SDK 6.3.0: <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- Apple file access controls:
  <https://support.apple.com/guide/security/controlling-app-access-to-files-secddd1d86a6/web>
- Apple Developer downloads:
  <https://developer.apple.com/download/>
