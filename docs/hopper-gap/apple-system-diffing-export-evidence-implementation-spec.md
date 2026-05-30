# Apple System Diffing, Export, And Evidence Implementation Spec

Status: implementation architecture plan. This is not an implementation claim
and it does not assert current Damsel has binary diffing or report-export
parity today.

Purpose: define the diffing, comparison, export, and evidence-package systems
Damsel needs to compete with Hopper while leaning into Damsel's Apple-system
niche. Hopper exposes export surfaces, BinDiff/BinExport workflows, type
export/import, Objective-C header export, assembly/pseudocode export, and
scriptable data extraction. Damsel should go further for macOS, iOS, dyld
shared caches, XNU-style system binaries, Objective-C, C++, Objective-C++,
Swift-adjacent metadata, KDK/dSYM sidecars, and panic analysis.

This is not a generic "save report" feature. It is a project-level evidence
system: stable object identities, reproducible snapshots, diff fingerprints,
version-aware matching, confidence, provenance, privacy policy, and Workbench
flows for understanding how Apple system artifacts changed across releases.

## Research Companions

- [Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
- [Research Sources](research-sources.md)

## Architecture Companions

- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [API And Data Contracts](api-data-contracts.md)
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md)
- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
- [Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec](investigation-casebook-session-timeline-implementation-spec.md)

Casebook companion:
[Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec](investigation-casebook-session-timeline-implementation-spec.md)
owns the analyst-facing layer above diff/export: cases, timeline events,
evidence pins, hypotheses, findings, notebook pages, report builder, redaction
preview, and handoff packages. Diff findings and evidence exports should feed
that casebook instead of becoming isolated files.

## Public Hopper And Ecosystem Evidence

Official Hopper changelog evidence:

- Hopper 5.4.0 added BinDiff support: export a BinExport file for BinDiff and
  import a BinDiff database.
- Hopper 5.8.7 fixed a crash when exporting BinDiff databases.
- Hopper 5.13.1 improved BinDiff exports.
- Hopper 5.13.3 reduced memory footprint for the BinDiff exporter and fixed a
  crash when exporting BinDiff for some large files.
- Hopper 5.11.0 fixed a potential deadlock when exporting assembly or
  pseudocode.
- Hopper 5.8.0 fixed a crash when exporting pseudocode containing very large
  methods.
- Hopper 5.14.1 and 5.17.4 improved Objective-C header export, including for
  files extracted from dyld shared caches.
- Hopper 5.6.0 added editing operations for structures, unions, enums, and
  method signatures; export/import is useful only if type data is durable.
- Hopper 6.0.0 added customization for showing or hiding procedure address
  columns in ASM and pseudocode, which matters for readable exported text.

Official Hopper SDK evidence:

- `HPDisassembledFile` exposes type database import/export.
- `HPDisassembledFile` exposes all names, named addresses, comments, inline
  comments, byte types, operand formats, procedures, colors, tags, problems,
  assembler/NOP helpers, and undo/redo transactions.
- `HPProcedure` exposes basic blocks, stack facts, local labels, register
  aliases, callers, callees, signatures, calling conventions, and variables.
- `HPSegment` exposes xrefs to and from addresses.
- `HopperTool` lets tools operate on the current document.

Public BinExport ecosystem evidence:

- Google's BinExport project describes BinExport as the exporter component of
  BinDiff and a Protocol Buffers disassembly export format.
- BinExport supports full protocol-buffer export, statistics text, and text
  dump outputs.
- BinExport is commonly implemented as a disassembler plugin for IDA Pro,
  Binary Ninja, and Ghidra; this indicates that diff/export is a core
  reverse-engineering workflow, not a single product's vanity format.
- Binary Ninja documentation describes invoking BinExport from a plugin menu or
  command palette, then using BinDiff on the resulting exports.

Public Hopper scripting/workflow evidence:

- Public Hopper scripts export selected bytes, selected assembly-context bytes,
  labels, comments, shellcode annotations, Yara matches, and other analysis
  evidence.
- A public 2025 procedure-name export workflow used Hopper's Python scripting
  to extract named addresses/procedures when the UI did not provide the exact
  export desired.

Architecture inference:

- A competitive reverse-engineering app must make analysis data portable.
- Diff/export features stress the same object model as automation: stable
  procedure ids, xrefs, names, comments, byte maps, runtime objects, type
  records, and provenance.
- Damsel's Apple niche creates a stronger opportunity than generic BinDiff
  parity: compare Apple OS builds and explain the runtime/kernel changes in
  domain vocabulary.

## Core Thesis

Damsel should not start by cloning BinExport byte-for-byte.

The better path is:

1. Define stable Damsel analysis snapshots.
2. Define object fingerprints for functions, runtime objects, types, xrefs,
   cache images, kernel members, and evidence packages.
3. Implement Apple-system-aware diffing over those snapshots.
4. Export structured Damsel evidence bundles.
5. Add human-readable reports and Workbench diff views.
6. Add BinExport-like interoperability later, after procedure and xref facts
   are stable enough to be meaningful.

The first version should answer Apple-specific questions well:

- What changed in this framework between macOS builds?
- Which Objective-C selectors gained or lost implementations?
- Which class, category, protocol, ivar, or property changed?
- Which C++ vtables, RTTI names, or virtual callsites changed?
- Which kernel collection members, kexts, syscalls, Mach traps, MIG routines,
  or IOKit classes changed?
- Which dyld shared-cache images changed providers/importers/reexports?
- Which panic-frame route now resolves differently?
- Which functions are probably the same despite address, symbol, or layout
  drift?
- Which changes are backed by strong evidence versus weak heuristic matches?

## Non-Goals For Early Waves

Do not implement first:

- full BinExport compatibility
- generic cross-architecture binary matching
- machine-learning matching model
- opaque "similarity percent" with no evidence
- whole-cache diff that blocks UI until every function is matched
- exporting raw private binary bytes by default
- cloud upload or remote diffing
- patch generation
- vulnerability scoring
- auto-generated exploitability claims
- report prose not backed by route/object evidence

Initial diff/export should be local, deterministic, explainable, and
Apple-domain-specific.

## Target User Workflows

### Workflow 1: macOS Framework Version Diff

User goal:

- compare two versions of a framework from different macOS builds

Inputs:

- source A: projected image from dyld shared cache
- source B: projected image from another dyld shared cache
- optional dSYM/KDK/symbol sidecars

Outputs:

- matched functions
- added/removed functions
- changed function fingerprints
- changed imports/exports
- changed ObjC classes/selectors/methods
- changed Swift-demangled symbols
- changed strings and CFStrings
- changed call graph neighborhoods
- evidence package with exact source identities

Workbench experience:

- diff overview with artifact identities
- change navigator grouped by function/runtime/type/string/import/export
- side-by-side function detail
- synchronized assembly and graph view
- runtime object diff browser
- exportable Markdown/HTML/JSON evidence pack

### Workflow 2: dyld Shared-Cache OS Build Archaeology

User goal:

- compare two dyld shared caches to understand system framework churn

Inputs:

- cache A
- cache B
- optional local symbols

Outputs:

- image added/removed/changed list
- dependency graph changes
- provider/importer/reexport changes
- per-image symbol and selector changes
- high-churn frameworks
- routeable evidence for each finding

Damsel advantage:

- preserve cache-level and projected-image-level addresses
- know provider/importer/reexport relationships
- route from changed selector to implementations and callers
- avoid treating extracted image paths as the only identity

### Workflow 3: Kernel Collection And KDK Diff

User goal:

- compare kernel collections or kexts across Apple OS builds

Inputs:

- kernel collection A
- kernel collection B
- KDK/dSYM profiles where available
- optional panic logs

Outputs:

- member/kext added/removed/changed
- IOKit class and vtable changes
- syscall/Mach trap/MIG routine changes
- changed external symbols
- KDK symbolication deltas
- panic frame remapping differences

Damsel advantage:

- explain differences using XNU terms, not only generic functions
- keep KDK profile mismatch visible
- preserve kernel collection global and member address spaces

### Workflow 4: Objective-C API Surface Report

User goal:

- export a report of Objective-C surface changes for a framework or app

Outputs:

- class additions/removals
- category changes
- protocol changes
- selector additions/removals
- method implementation changes
- ivar/property changes
- type encoding changes
- caller/implementer changes
- generated header diff where evidence is strong

Rules:

- do not silently present inferred receivers as exact
- ambiguous selector ownership becomes a finding with confidence
- user annotations and manual bindings are recorded separately

### Workflow 5: C++/IOKit ABI Diff

User goal:

- compare C++ or Objective-C++ system binaries, especially IOKit families

Outputs:

- typeinfo changes
- vtable shape changes
- virtual slot changes
- constructor/destructor changes
- virtual callsite changes
- inheritance candidate changes
- symbolication-profile impact

Rules:

- distinguish symbol-backed C++ types from heuristic vtable candidates
- preserve false-positive rejection evidence
- show confidence per match and per change

### Workflow 6: Evidence Package For Review

User goal:

- share local, reproducible evidence without sharing raw proprietary bytes

Outputs:

- manifest
- source identities and redacted paths
- selected facts
- selected disassembly windows
- graphs
- xrefs
- runtime object facts
- type facts
- comments/tags if included
- screenshots or rendered report later
- reproduction commands

Privacy defaults:

- no raw bytes
- no full local paths
- no private strings unless explicitly included
- no comments unless the user includes annotations
- no KDK/dSYM contents

## Data Model

### Analysis Snapshot

An analysis snapshot is a stable, exportable view of a project or source at a
particular project revision.

Fields:

- snapshot id
- project id
- project revision
- source ids
- source identities
- analyzer versions
- architecture semantics versions
- symbolication profile ids
- included fact kinds
- completeness
- redaction policy
- created timestamp

Completeness:

- full_for_selected_scope
- partial_for_selected_scope
- inventory_only
- stale_facts_included
- canceled
- failed

Rules:

- diffing uses snapshots, not live mutable state
- snapshots record which facts were excluded
- snapshots never imply raw bytes are embedded
- snapshots can be rebuilt when project facts change

### Artifact Identity

Fields:

- artifact id
- source id
- kind
- display name
- architecture
- platform
- UUID/build version where available
- content hash where available
- size
- parent source id
- address-space identities
- symbolication profile ids

Kinds:

- standalone Mach-O
- universal slice
- dyld shared cache
- projected cache image
- driver cache member
- kernel collection
- fileset member
- kext
- dSYM
- KDK
- panic log
- evidence bundle

### Diff Project

A diff project compares two or more analysis snapshots.

Fields:

- diff project id
- baseline snapshot id
- comparison snapshot ids
- matching profile
- scope
- match records
- findings
- export artifacts
- privacy policy
- status

Scopes:

- whole project
- source pair
- selected image pair
- selected function pair
- selected runtime object pair
- selector/class/type namespace
- kernel member pair
- panic-frame set

### Object Fingerprint

Each routeable object can have fingerprints.

Common fields:

- object id
- route
- object kind
- source id
- fingerprint kind
- fingerprint value
- confidence
- input fact versions
- analyzer version

Fingerprint kinds:

- identity
- normalized_name
- demangled_name
- address_relative_to_source
- byte_hash
- instruction_mnemonic_sequence
- normalized_instruction_sequence
- basic_block_shape
- cfg_shape
- call_targets
- string_refs
- constant_refs
- import_refs
- selector_refs
- class_refs
- type_signature
- vtable_shape
- runtime_metadata
- kernel_table_entry
- source_context

Rules:

- no single fingerprint decides a fuzzy match
- address-only matches are weak unless source identity is unchanged
- name-only matches are ambiguous in stripped or overload-heavy contexts
- runtime-object fingerprints can outrank raw address similarity for Apple
  frameworks

### Function Diff Record

Fields:

- left procedure route
- right procedure route
- match confidence
- match reasons
- changed status
- byte hash status
- instruction status
- CFG status
- call graph status
- string refs status
- runtime refs status
- type/signature status
- stack frame status
- problem list

Statuses:

- unchanged
- changed
- added
- removed
- moved
- split
- merged
- renamed
- unresolved
- ambiguous

### Runtime Object Diff Record

Kinds:

- ObjC class
- ObjC category
- ObjC protocol
- ObjC method
- selector
- property
- ivar
- C++ type
- C++ vtable
- virtual slot
- Swift demangled symbol
- dyld cache image
- import/export/reexport edge
- kernel collection member
- kext
- IOKit class
- syscall
- Mach trap
- MIG routine
- panic frame

Fields:

- left route
- right route
- identity key
- status
- changed fields
- linked function diffs
- provenance
- confidence

### Diff Finding

Fields:

- finding id
- severity
- category
- title
- summary
- routes
- evidence records
- confidence
- source snapshots
- export inclusion state

Categories:

- artifact_identity
- function_added
- function_removed
- function_changed
- selector_changed
- objc_surface_changed
- cpp_abi_changed
- kernel_member_changed
- kext_changed
- syscall_changed
- mig_changed
- dyld_dependency_changed
- symbolication_changed
- panic_resolution_changed
- ambiguous_match
- analysis_gap

Severity:

- info
- notable
- high
- blocker

Severity is analyst workflow severity, not exploitability.

### Evidence Package

Fields:

- package id
- manifest
- export profile
- source identities
- included snapshots
- included findings
- included route evidence
- included graphs
- included reports
- redaction log
- reproduction commands
- tool versions

Package forms:

- directory
- zip archive
- JSON bundle
- Markdown report plus assets
- HTML report plus assets
- external format later

## Matching Architecture

### Matching Pipeline

Pipeline:

1. Establish artifact identity.
2. Build exact object matches.
3. Build runtime-object matches.
4. Build symbol/name matches.
5. Build structural function matches.
6. Build callgraph-neighborhood matches.
7. Resolve conflicts.
8. Emit unmatched objects.
9. Generate findings.
10. Index diff results for navigation.

### Exact Matches

Exact matching inputs:

- same source hash
- same UUID/build id
- same stable object id after project reopen
- same symbol UUID or debug-info identity
- same runtime metadata identity within same source

Exact matches can still have changed derived facts when analyzers or sidecars
change. Exact source identity does not imply exact analysis identity.

### Runtime Matches

Runtime matching keys:

- ObjC class name
- ObjC category owner/name
- protocol name
- selector string plus owner evidence
- method kind and selector
- property/ivar name plus class
- Swift demangled name
- C++ demangled type/name
- vtable owner candidate
- IOKit class name
- syscall/trap/MIG routine number/name
- kext bundle id
- dyld image install name

Rules:

- runtime identity can survive address churn
- runtime identity can collide
- collisions remain visible as ambiguous matches

### Function Fuzzy Matching

Signals:

- normalized instruction sequence
- basic block count
- edge count
- call targets by matched object
- string reference set
- selector reference set
- import reference set
- constant set
- stack frame shape
- type/signature shape
- CFG hash
- decompiler-lite summary hash later

Scoring:

- each signal contributes a named score
- final score is explainable
- thresholds are profile-specific
- no-match is better than a misleading match

Profiles:

- `same_source_reanalysis`
- `same_binary_different_sidecars`
- `app_version_diff`
- `dyld_cache_build_diff`
- `kernel_collection_build_diff`
- `stripped_binary_diff`

### Conflict Resolution

Conflict classes:

- one left object matches multiple right objects
- multiple left objects match one right object
- symbol match disagrees with structure match
- runtime match disagrees with procedure match
- address match disagrees with source identity
- sidecar symbol changed without byte change

Resolution policy:

- emit ambiguity rather than hiding it
- preserve alternate candidates
- allow analyst to pin a match
- manual match pins are annotations with actor/provenance
- downstream findings include whether a manual pin affected them

## Export Architecture

### Export Profiles

Initial profiles:

- `project_manifest`
- `selected_function`
- `function_list`
- `xref_report`
- `objc_surface`
- `cpp_abi`
- `kernel_surface`
- `dyld_cache_inventory`
- `diff_summary`
- `evidence_package`
- `diagnostics_redacted`

Later profiles:

- `binexport_like`
- `bindiff_interop`
- `sarif_security_notes`
- `symbolication_bundle`

### Export Formats

Initial formats:

- JSON
- NDJSON for large streams
- Markdown
- HTML
- CSV
- DOT
- GraphML
- plain text assembly

Later formats:

- Protocol Buffers
- BinExport-like interchange
- richer report archive

Rules:

- JSON is canonical for Damsel-to-Damsel workflows
- human reports are generated from canonical JSON
- large exports stream rather than materializing everything in memory
- exported objects include routes, provenance, confidence, and completeness
- export manifests include schema versions

### Objective-C Header Export

Inputs:

- class/protocol/category facts
- method signatures
- type encodings
- properties
- ivars
- user type overrides
- symbolication profiles

Outputs:

- header text
- confidence annotations in structured sidecar
- unresolved type list
- source evidence links

Rules:

- generated headers are evidence, not source reconstruction
- uncertain types are marked
- user overrides are identified
- dyld-cache projected image identity is preserved

### Type Export

Inputs:

- type database
- structures/unions/enums
- function signatures
- ObjC type encodings
- C++ type candidates
- KDK/dSYM imported types
- user edits

Outputs:

- Damsel type JSON
- C header fragment where possible
- type provenance report
- importable type bundle

Rules:

- imported KDK/dSYM types retain source profile ids
- user-edited types do not overwrite imported facts silently
- type export can be scoped by source, runtime object, or selection

### Graph Export

Graph types:

- function CFG
- call graph
- xref graph
- ObjC selector graph
- class inheritance graph
- C++ vtable/type graph
- dyld dependency graph
- kernel/kext dependency graph
- panic route graph
- diff match graph

Formats:

- JSON graph
- DOT
- GraphML
- SVG/PNG later through renderer

Rules:

- layout is exportable separately from graph truth
- graph exports include route ids
- huge graphs require scoping or paging

## Workbench UX

### Diff Workspace

Layout:

- top: diff source identity bar
- left: change navigator
- center: side-by-side representation
- right: evidence inspector
- bottom: jobs/problems/findings drawer

Source identity bar shows:

- source names
- architecture/platform
- UUID/build/hash
- symbolication profiles
- snapshot completeness
- stale status

Change navigator groups:

- overview
- artifacts
- functions
- xrefs
- ObjC
- C++/ObjC++
- Swift-adjacent
- dyld cache
- kernel/KDK
- strings/constants
- problems
- ambiguous matches

### Function Diff View

Modes:

- side-by-side assembly
- normalized instruction diff
- CFG diff
- callsite diff
- xref diff
- decompiler-lite summary diff
- hex range diff later

Interactions:

- select left route to sync right matched route
- jump to unmatched functions
- show match reasons
- pin or reject match
- open callers/callees
- export selected evidence

### Runtime Diff Views

Objective-C:

- class list diff
- selector implementer diff
- method signature diff
- ivar/property diff
- category/protocol diff
- header diff preview

C++/IOKit:

- typeinfo list diff
- vtable slot diff
- virtual callsite diff
- inheritance candidate diff
- constructor/destructor diff

Kernel:

- member/kext diff
- syscall/trap/MIG diff
- IOKit class diff
- panic route diff
- KDK symbolication diff

### Evidence Inspector

For every finding, show:

- matched objects
- match confidence
- match reasons
- source identities
- changed fields
- related xrefs
- related strings/constants
- related runtime objects
- included export fragments
- privacy/redaction state

Do not show:

- unsupported certainty
- ungrounded natural-language conclusions
- exploitability claims without a separate security analysis pass

## CLI Surface

Proposed commands:

```text
damsel project snapshot <project>
damsel project export <project> --profile objc-surface --format json
damsel project export <project> --profile evidence-package --out report/
damsel project diff create --left <snapshot-a> --right <snapshot-b>
damsel project diff status <diff-id>
damsel project diff findings <diff-id>
damsel project diff functions <diff-id>
damsel project diff objc <diff-id>
damsel project diff cpp <diff-id>
damsel project diff kernel <diff-id>
damsel project diff export <diff-id> --format markdown
damsel cache diff --left <cache-a> --right <cache-b> --image Foundation
damsel kernel diff --left <kc-a> --right <kc-b> --kdk-left <kdk-a> --kdk-right <kdk-b>
```

Rules:

- commands accept project snapshots, not only raw files
- raw file convenience commands create temporary project snapshots internally
- long diff jobs report progress and can be resumed
- JSON output is deterministic for fixtures

## API Surface

Endpoint groups:

```text
POST /api/projects/{project_id}/snapshots
GET  /api/projects/{project_id}/snapshots/{snapshot_id}
POST /api/diffs
GET  /api/diffs/{diff_id}
GET  /api/diffs/{diff_id}/findings
GET  /api/diffs/{diff_id}/matches
GET  /api/diffs/{diff_id}/functions
GET  /api/diffs/{diff_id}/runtime/{kind}
POST /api/diffs/{diff_id}/matches/pin
POST /api/diffs/{diff_id}/matches/reject
POST /api/diffs/{diff_id}/exports
GET  /api/exports/{export_id}
```

Paged responses:

- findings
- matches
- changed functions
- changed selectors
- changed types
- changed cache images
- changed kernel members

Mutation transactions:

- pin match
- reject match
- add finding note
- include/exclude finding from export
- set finding severity

All mutations record actor and transaction id.

## Storage And Performance

### Snapshot Storage

Snapshots store:

- manifest
- source identities
- fact version references
- selected exported fact copies where needed
- fingerprint tables
- completeness metadata

Snapshots should not duplicate:

- full binary bytes
- full project database
- huge rendered strings unless export profile requires them

### Diff Indexes

Indexes:

- object match index
- unmatched left/right indexes
- finding index
- changed function index
- runtime object change index
- ambiguous match index
- export inclusion index

Rules:

- diff jobs write indexes incrementally
- findings can be paged before all low-priority matching is finished
- selected object matching can be prioritized

### Performance Budgets

Targets:

- create snapshot manifest for small fixture quickly enough for every commit
- show diff identity overview before full function matching
- first page of changed functions under interactive budget after index exists
- selected function diff can be computed on demand
- whole-cache diff is job-backed and cancelable
- export package streams to disk

## Privacy And Redaction

Default redactions:

- full local paths
- raw binary bytes
- private strings
- analyst comments
- KDK/dSYM contents
- panic logs unless explicitly included

Export policy fields:

- include raw bytes: false by default
- include strings: selected or redacted by default
- include comments: false by default
- include local paths: redacted by default
- include source hashes: true by default
- include UUIDs/build ids: true where safe
- include screenshots: false by default

Every evidence package includes a redaction log explaining omitted data.

## Integration With Existing Damsel

Current foundations:

- `SharedCacheSession` already knows cache images, projections, dependencies,
  providers, importers, and reexports.
- `ObjcMetadata` already stores classes, protocols, categories, methods,
  properties, ivars, selectors, class refs, and method IMPs.
- `DyldMetadata` already stores imports, rpaths, exports, function starts,
  chained-fixup presence, bindings, stubs, and helpers.
- `DisassemblyAnalysis` already stores basic blocks, edges, imports, ObjC
  references, cache links, recovered values, jump tables, and indirect uses.
- CLI JSON output already creates deterministic test surfaces.

Required refactors:

- persist those facts in `AnalysisProject`
- add stable route/object ids
- add procedure database and xref indexes
- add fingerprint computation
- add snapshot creation
- add diff job scheduler
- add export service
- add Workbench diff stores and views

Compatibility:

- existing CLI commands remain
- project diff commands can be added beside current commands
- external comparison scripts remain useful as validation inputs

## Implementation Phases

### Phase D0: Export Foundation

Deliverables:

- export manifest schema
- project snapshot manifest
- deterministic JSON export for project summary
- deterministic JSON export for selected function
- redaction policy model
- export job record

Validation:

- export manifest snapshot
- redaction policy snapshot
- selected function export fixture
- no raw bytes by default test

### Phase D1: Object Fingerprints

Deliverables:

- fingerprint trait/model
- function fingerprints
- runtime object fingerprints
- type fingerprints
- xref fingerprints
- dyld image fingerprints
- kernel member fingerprints

Validation:

- fingerprint stability fixture
- analyzer version invalidation fixture
- source hash change fixture
- collision fixture

### Phase D2: Snapshot Diff Core

Deliverables:

- diff project model
- exact matching
- runtime-object matching
- function fuzzy matching v1
- conflict records
- finding records
- paged finding API

Validation:

- added/removed function fixture
- renamed function fixture
- ObjC selector change fixture
- ambiguous match fixture
- conflict resolution fixture

### Phase D3: Apple Runtime Diff

Deliverables:

- ObjC class/category/protocol/method diff
- selector implementer/caller diff
- Swift-demangled symbol diff
- C++ type/vtable diff
- IOKit class/vtable diff

Validation:

- ObjC surface fixture
- selector collision fixture
- C++ vtable fixture
- ObjC++ mixed fixture
- Swift symbol fixture

### Phase D4: dyld And Kernel Diff

Deliverables:

- dyld cache image diff
- dependency/provider/importer/reexport diff
- projected image route matching
- kernel collection member diff
- kext identity diff
- syscall/Mach trap/MIG diff
- panic route diff

Validation:

- synthetic split-cache diff fixture
- provider/importer diff fixture
- kernel member diff fixture
- KDK mismatch diff fixture
- panic frame remap fixture

### Phase D5: Workbench Diff UI

Deliverables:

- diff workspace
- change navigator
- side-by-side function diff
- ObjC diff browser
- C++/IOKit diff browser
- kernel diff browser
- evidence inspector
- export sheet

Validation:

- Workbench model tests
- selection sync test
- paged findings test
- pinned match transaction test
- UI smoke on fixture diff

### Phase D6: Evidence Packages

Deliverables:

- Markdown report
- HTML report
- JSON evidence bundle
- CSV export for lists
- DOT/GraphML export for selected graphs
- redaction log
- reproduction command list

Validation:

- report snapshot
- evidence package fixture
- redaction fixture
- graph export fixture
- reproducibility check

### Phase D7: Interop Spikes

Deliverables:

- BinExport-like schema study
- BinExport writer spike if procedure/xref facts are mature
- BinDiff database import study if legally and technically practical
- external diff import abstraction

Validation:

- schema mapping doc
- unsupported-field report
- round-trip fixture where possible

## Validation Corpus

Required fixtures:

- two small Mach-O versions with one function changed
- function added/removed fixture
- symbol rename fixture
- string change fixture
- ObjC class/method change fixture
- selector collision fixture
- C++ vtable slot change fixture
- ObjC++ mixed change fixture
- synthetic dyld cache image added/removed fixture
- provider/importer/reexport change fixture
- synthetic kernel member change fixture
- KDK profile mismatch fixture
- panic frame remap fixture
- export redaction fixture
- ambiguous fuzzy match fixture

Opt-in real artifacts:

- two macOS dyld cache versions
- two versions of a selected framework
- two kernel collections
- matching/mismatching KDK bundles
- sanitized panic logs

Rules:

- real artifact evidence must record exact date, artifact kind, and run
  environment
- local paths are redacted unless user opts in
- no public claim can cite a real diff that was not actually run

## Risks And Mitigations

Risk: fuzzy matching creates false confidence.

Mitigation:

- expose match reasons and competing candidates
- use conservative thresholds
- allow manual pin/reject transactions
- keep ambiguous matches visible

Risk: exported reports leak private data.

Mitigation:

- redaction-by-default policy
- export preview
- redaction log
- tests for no raw bytes/comments/paths by default

Risk: diff UI becomes expensive on huge caches.

Mitigation:

- snapshot manifests first
- paged findings
- job-backed matching
- selected object prioritization
- whole-cache diff as opt-in heavy job

Risk: BinExport compatibility distorts Damsel's Apple-specific model.

Mitigation:

- define Damsel native evidence first
- map to external formats later
- report unsupported or lossy fields explicitly

Risk: reports imply source reconstruction.

Mitigation:

- label generated headers and pseudocode as recovered evidence
- include confidence and provenance
- show unresolved types and ambiguous ownership

## Acceptance Gates

Gate D0:

- deterministic snapshot/export manifest exists
- no raw bytes in default export
- selected function JSON export includes routes/provenance/confidence

Gate D1:

- fingerprints are stable across project reopen
- analyzer version drift invalidates derived fingerprints
- collision cases are represented

Gate D2:

- diff project can compare two fixture snapshots
- added/removed/changed functions are reported
- ambiguous matches are visible
- findings are paged

Gate D3:

- ObjC class/selector/method changes are routeable
- C++ vtable changes are routeable
- runtime diff confidence is visible

Gate D4:

- dyld image/dependency changes are routeable
- kernel member/kext changes are routeable
- KDK mismatch affects symbolication diff visibly

Gate D5:

- Workbench diff workspace can navigate findings
- side-by-side function diff synchronizes routes
- match pin/reject creates transactions

Gate D6:

- evidence package contains manifest, redaction log, source identities,
  findings, and reproduction commands
- Markdown/HTML/JSON exports are generated from the same canonical facts

Completion for this tranche requires all gates for the claimed phase. A JSON
export alone does not prove diffing. A diff finding list alone does not prove
evidence packaging. A report alone does not prove Workbench route
synchronization.
