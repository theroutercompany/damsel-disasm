# DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec

Status: implementation-grade architecture spec.

Date of public-source pass: 2026-05-26.

Purpose: define the debug-info subsystem Damsel needs in order to approach
Hopper-class Apple reverse-engineering workflows while keeping Damsel's own
niche: macOS, iOS, dyld shared caches, driver caches, XNU-style kernel
collections, KDKs, dSYMs, panic logs, Objective-C, C++, Objective-C++, and
arm64e system binaries.

This document is a companion to:

- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
- [Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)

## Why This Exists

The Hopper-gap pack already mentions dSYMs, DWARF, KDKs, source lines, local
variables, imported types, and symbolication profiles in many places. That is
not enough. Debug info is not an attachment checkbox. For Apple systems work,
debug info is the bridge between:

- raw addresses and source locations
- stripped binaries and useful names
- compiler-emitted types and decompiler signatures
- stack offsets and real local variables
- inlined functions and misleading flat call graphs
- kernel panic frames and actionable kernel/kext source context
- KDK bundles and XNU object models
- user trust and sidecar mismatch safety

Hopper's public SDK makes this architectural shape visible even when its
private implementation is not public. Loader plugins can load debug data after
a file is loaded. Debug-data loading can return mismatch statuses. The document
object owns names, procedures, type databases, signatures, stack facts,
variables, operand formats, comments, problems, and transactions. That implies
debug data is not a separate report. It mutates or enriches the same analysis
document that powers assembly, graph, pseudocode, inspector, debugger, scripts,
and MCP.

For Damsel, a Hopper-competitive Apple lane needs a first-class debug-info
kernel. Without it, later docs would keep pretending that KDK/dSYM support is
just "parse DWARF and add symbols." It is deeper:

- intake has to discover, classify, and match sidecars
- matching has to be trustable and explainable
- import has to be lazy enough for huge KDKs
- facts have to be provenance-aware and conflict-tolerant
- source paths have to be privacy-sensitive
- source-line views need stable route ids
- variables need validity ranges and optimized-out states
- type import must merge with ObjC/C++/Swift-adjacent runtime facts
- symbolication profiles need user-visible status
- debugger overlays and panic frames need the same mapping model

## Public Source Set

Primary Hopper sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page and public SDK listing: <https://www.hopperapp.com/download>
- Hopper Debugger Server page:
  <https://www.hopperapp.com/HopperDebuggerServer/>
- Hopper public SDK archive:
  <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>

Hopper public SDK headers inspected locally from SDK 6.3.0:

- `FileLoader.h`
- `HPDisassembledFile.h`
- `HPProcedure.h`
- `HPTypeDesc.h`
- `HPMethodSignature.h`
- `HPMethodArgument.h`
- `HPTypeStructField.h`
- `HPTypeEnumField.h`
- `CommonTypes.h`
- `CPUContext.h`

External format and platform sources:

- DWARF standard site: <https://dwarfstd.org/>
- DWARF 5 standard PDF: <https://dwarfstd.org/doc/DWARF5.pdf>
- LLVM source-level debug-info documentation:
  <https://llvm.org/docs/SourceLevelDebugging.html>
- LLVM `dsymutil` command guide:
  <https://llvm.org/docs/CommandGuide/dsymutil.html>
- LLVM `llvm-dwarfdump` command guide:
  <https://llvm.org/docs/CommandGuide/llvm-dwarfdump.html>
- Apple Xcode debugging information documentation:
  <https://developer.apple.com/documentation/xcode/building-your-app-to-include-debugging-information>
- Apple developer downloads for KDK acquisition:
  <https://developer.apple.com/download/>

Public workflow sources already used elsewhere in this pack:

- Public macOS private-framework workflow using Hopper:
  <https://www.jviotti.com/2023/11/20/exploring-macos-private-frameworks.html>
- Hopper Python type stubs:
  <https://pypi.org/project/types-hopper-disassembler/>
- Hopper Python type-stub repository:
  <https://github.com/Fi5t/hopper-disassembler-stubs>

## Source Evidence Summary

### Hopper Evidence

Observed public SDK facts:

- `FileLoader` exposes `canLoadDebugFiles`.
- `FileLoader` exposes `loadDebugData`.
- `FileLoaderLoadingStatus` includes debug mismatch statuses:
  `DIS_DebugMismatch` and `DIS_DebugUUIDMismatch`.
- `HPDisassembledFile` owns names, comments, byte types, operand formatting,
  procedures, tags, problems, assembler hooks, reading, and undo/redo.
- `HPProcedure` owns frame facts, stack pointer offsets, local labels, register
  aliases, callers, callees, variable names by displacement, signatures,
  signature reasons, and calling conventions.
- `HPTypeDesc` owns named, primitive, pointer, array, structured, enum,
  incomplete, forward-declared, and function-pointer type facts.
- `HPMethodSignature` owns return type and argument records.
- `CommonTypes.h` exposes `SignatureCreationReason` values for user facts,
  demangling, database facts, data-flow guesses, called-function inference, and
  decompiler guesses.
- `CommonTypes.h` exposes Swift and AAPCS calling conventions plus special
  AArch64 Objective-C runtime calling conventions.

Interpretation:

- Hopper treats debug data as an import path into the editable document model.
- Debug mismatch is an expected first-class outcome, not an exceptional crash.
- Signatures and types have competing sources. Debug info is one source among
  user edits, demangling, embedded databases, data-flow, and decompiler guesses.
- Procedure-local facts are not just decompiler output. The document/procedure
  model itself has stack, register, variable, and signature concepts.

### DWARF And LLVM Evidence

DWARF is a structured debug-information format for symbolic and source-level
debugging. It encodes compile units, source files, line programs, subprograms,
lexical scopes, inline call sites, variables, types, address ranges, location
expressions, and language metadata.

LLVM's source-level debugging docs frame debug information as metadata mapping
source-language program structure to generated code. That matches Damsel's
need: debug info should become structured facts that enrich procedures,
variables, types, source routes, and debugger/panic overlays.

`dsymutil` is particularly relevant for Apple's world: it links DWARF debug
information for an executable and normally places the result in a `.dSYM`
bundle. Damsel should therefore treat dSYM bundles as sidecar sources with
identity, architecture slices, contained DWARF objects, and source path
metadata, not as a generic symbol file.

### Apple Evidence

Apple's Xcode documentation directs distribution builds to use "DWARF with dSYM
File" when symbol information is needed for debugging and crash reports. For
Damsel, that means:

- dSYMs are normal Apple artifacts, not rare optional extras.
- crash/panic symbolication and reverse engineering share some matching needs.
- app bundles, frameworks, KDKs, dSYMs, and debug roots have to be modeled as
  acquisition/intake sources.

Apple KDKs are not just symbol files. They are build-specific kits that can
contain kernel and kext debug material for an OS build. Damsel's XNU niche must
handle KDK matching, mismatch visibility, and opt-in indexing as part of the
project model.

## Product Thesis

Damsel should compete with Hopper in the Apple lane by making debug-info
enrichment more explainable and more Apple-system aware than a generic
disassembler:

- show exactly which dSYM/KDK/debug source matched which image/member/kext
- explain why a candidate did not match
- let users keep multiple symbolication profiles
- never silently apply a mismatched KDK/dSYM
- import source lines, function boundaries, local variables, inline callsites,
  types, and signatures as provenance-rich facts
- expose debug-info coverage and gaps in the Workbench
- make panic frames, debugger PCs, decompiler summaries, and static functions
  consume the same source-line/address mapping
- let users prove an evidence package without leaking local source paths

The first implementation should be useful even before a full decompiler:

- exact debug names
- source line ranges
- compile-unit and source-file indexes
- function range correction
- local-variable names and stack/register locations where reliable
- type and signature candidates
- symbolication profile UI
- mismatch/problem reporting

## Non-Goals

Initial debug-info work should not try to implement:

- a full LLDB replacement
- expression evaluation
- source-code editing
- source-level stepping
- arbitrary remote dSYM download services
- silent internet lookup of symbols
- Swift source reconstruction
- decompiler truth claims from debug info alone
- exact variable values at runtime
- applying mismatched debug info because it "looks close"
- indexing all KDK DWARF eagerly at project open
- storing private source file contents inside project bundles by default

## Current Damsel Baseline

Observed current repo state:

- `damsel-core::BinaryImage` models standalone loaded Mach-O images.
- `damsel-core::SharedCache` models dyld shared-cache state.
- `damsel-core::ProjectedBinaryImage` models cache-projected images.
- `damsel-core::ObjcMetadata` already records Apple runtime metadata.
- `damsel-core::DyldMetadata` already records dyld imports, exports, fixups,
  function starts, bindings, stubs, rpaths, and platform facts.
- `damsel-core::DisassemblyAnalysis` already records basic blocks, edges,
  imports, ObjC references, cache links, recovered values, jump tables, and
  analysis metrics.
- `damsel-macho::loader::load` reads bytes and builds a `BinaryImage`.
- `damsel-macho::shared_cache` already records root/member UUIDs for dyld
  cache sets.
- CLI cache symbolication exists for dyld shared-cache queries.
- The native Workbench is currently a shell over the local CLI UI server.

Missing debug-info facts:

- no dSYM bundle model
- no DWARF section parser
- no debug source registry
- no Mach-O `LC_UUID` model exposed for standalone images
- no per-architecture dSYM matching
- no compile-unit table
- no source-file table
- no line table import
- no inline callsite model
- no debug variable model
- no variable location expression model
- no type DIE import
- no symbolication profile model implemented in code
- no KDK bundle inventory
- no KDK build matching
- no Workbench debug-info status panel
- no source-line route
- no debug-info problem navigator
- no API to ask "why did this dSYM not match?"

These gaps are expected. This spec exists so the next implementation does not
reduce debug info to a one-off symbol overlay.

## Architecture Overview

The subsystem should have four layers:

```text
Artifact intake and source registry
  |
  +-- debug-info source discovery
  |     +-- dSYM bundle classifier
  |     +-- standalone DWARF/Mach-O debug file classifier
  |     +-- KDK bundle classifier
  |     +-- debug-map and source-root hints
  |
  +-- identity and matching
  |     +-- Mach-O UUID / architecture matching
  |     +-- dyld cache image/member identity
  |     +-- kernel collection/fileset/kext identity
  |     +-- build version / kernel version / KDK identity
  |     +-- exact, weak, mismatch, ambiguous, stale outcomes
  |
  +-- lazy debug-info import
  |     +-- compile units
  |     +-- source files
  |     +-- line tables
  |     +-- subprogram ranges
  |     +-- inline callsites
  |     +-- variables and location lists
  |     +-- types and signatures
  |
  +-- project fact integration
        +-- symbolication profile
        +-- name/type/signature facts
        +-- procedure range corrections
        +-- source-line routes
        +-- debugger and panic overlays
        +-- Workbench views, CLI, MCP, reports
```

The central rule:

Debug info never writes directly to a rendered row. It imports facts into the
project database with source identity, match confidence, validity range,
language, compile-unit context, and conflict behavior.

## Proposed Modules

Rust core:

- `damsel-core::debug_info`
- `damsel-core::debug_source`
- `damsel-core::source_line`
- `damsel-core::symbolication_profile`

Mach-O and Apple artifact handling:

- `damsel-macho::debug_info`
- `damsel-macho::dsym`
- `damsel-macho::dwarf`
- `damsel-macho::kdk`
- `damsel-macho::debug_match`

CLI:

- `damsel-cli::debug_info_commands`
- `damsel-cli::debug_info_output`
- `damsel-cli::project_debug_api`

Workbench:

- `DebugInfoStore.swift`
- `SymbolicationProfileStore.swift`
- `SourceLineStore.swift`
- `DebugSourceListView.swift`
- `DebugInfoInspectorModule.swift`
- `SourceLineInspectorModule.swift`
- `SymbolicationProfilePanel.swift`

Project database:

- `debug_sources`
- `debug_source_matches`
- `debug_import_sessions`
- `debug_compile_units`
- `debug_source_files`
- `debug_line_rows`
- `debug_subprograms`
- `debug_inline_callsites`
- `debug_lexical_scopes`
- `debug_variables`
- `debug_location_ranges`
- `debug_type_records`
- `debug_type_edges`
- `debug_imported_facts`
- `symbolication_profiles`
- `symbolication_profile_members`
- `debug_info_problems`

## Data Model

### DebugInfoSourceId

Stable id for a debug-info source.

Fields:

- `value`
- `project_id`
- `source_artifact_id`
- `kind`
- `content_hash_prefix`
- `discovery_path_hash`
- `created_at`

Rules:

- The id must not encode a private absolute path in user-visible exports.
- Content hash should be optional for huge sources until a background hash job
  finishes.
- The id should survive project reopen as long as the registered source record
  survives.

### DebugInfoSourceKind

Enum:

- `DsymBundle`
- `DwarfObject`
- `MachODebugObject`
- `KdkBundle`
- `KdkDsym`
- `DebugMap`
- `SymbolMap`
- `SourceRoot`
- `PanicLogSymbolHints`
- `UserSuppliedBundle`
- `Unknown`

Notes:

- A KDK bundle is a container. Contained dSYMs get their own source ids.
- A source root can provide source text availability without contributing
  symbols or types.
- Panic logs can include symbol or slide hints but should not be trusted as
  debug info.

### DebugObjectIdentity

The identity extracted from a binary or debug source.

Fields:

- `architecture`
- `cpu_subtype`
- `macho_uuid`
- `install_name`
- `bundle_identifier`
- `binary_basename`
- `image_path`
- `cache_uuid`
- `cache_image_index`
- `cache_member_uuid`
- `kernel_collection_kind`
- `fileset_member_name`
- `kext_bundle_id`
- `kext_version`
- `build_version_platform`
- `min_os`
- `sdk_version`
- `kernel_version`
- `kdk_build`

Rules:

- UUID plus architecture is the strongest ordinary dSYM match.
- Paths are hints, not proof.
- Basenames are weak hints.
- KDK build and kernel version are necessary but not always sufficient.
- For cache-projected images, identity must include cache context.
- For fileset kernel members, identity must include fileset member context.

### DebugInfoMatchCandidate

Represents a possible match between a project object and a debug source.

Fields:

- `candidate_id`
- `target_route`
- `debug_source_id`
- `match_status`
- `match_strength`
- `matched_keys`
- `mismatched_keys`
- `missing_keys`
- `architecture`
- `uuid`
- `candidate_display_name`
- `notes`
- `created_by`

`match_status` enum:

- `Exact`
- `ExactButInactive`
- `CompatibleContainer`
- `WeakPathMatch`
- `WeakNameMatch`
- `Ambiguous`
- `RejectedUuidMismatch`
- `RejectedArchitectureMismatch`
- `RejectedBuildMismatch`
- `RejectedUnsupportedFormat`
- `RejectedMissingDebugData`
- `RejectedUser`
- `PendingInspection`

Rules:

- Exact matches may auto-activate only if the project policy allows it.
- Weak matches require explicit user activation.
- Rejected candidates should remain visible so the user can understand why a
  sidecar was ignored.
- Ambiguous candidates should never silently apply.

### DebugInfoImportSession

Records one import job.

Fields:

- `session_id`
- `project_id`
- `debug_source_id`
- `target_route`
- `symbolication_profile_id`
- `started_at`
- `finished_at`
- `status`
- `phase`
- `bytes_scanned`
- `units_indexed`
- `line_rows_indexed`
- `functions_imported`
- `variables_imported`
- `types_imported`
- `problems_created`
- `cancelled_by_user`
- `importer_version`

`status` enum:

- `Queued`
- `IndexingContainer`
- `Matching`
- `ImportingCompileUnits`
- `ImportingLineTables`
- `ImportingFunctions`
- `ImportingTypes`
- `ImportingVariables`
- `CommittingFacts`
- `Completed`
- `CompletedWithProblems`
- `Failed`
- `Cancelled`

Rules:

- Large dSYM/KDK imports must be cancellable.
- Partial import should leave a clear import-session record.
- Facts committed before cancellation must be traceable to the session.
- Re-running an import should produce deterministic facts when inputs match.

### DebugCompileUnitRecord

Fields:

- `compile_unit_id`
- `debug_source_id`
- `unit_offset`
- `language`
- `producer`
- `comp_dir`
- `name`
- `low_pc`
- `high_pc`
- `ranges_id`
- `address_size`
- `dwarf_version`
- `line_program_offset`
- `primary_source_file_id`
- `import_status`

Rules:

- `comp_dir` may include private paths. Store it with privacy class.
- Address ranges can be non-contiguous.
- Compile units can overlap after LTO or generated code; do not assume a
  function belongs to only one unit without evidence.

### DebugSourceFileRecord

Fields:

- `source_file_id`
- `compile_unit_id`
- `declared_path`
- `normalized_path`
- `basename`
- `directory_index`
- `checksum_kind`
- `checksum`
- `privacy_class`
- `source_availability`
- `mapped_local_path`
- `content_hash`

`source_availability` enum:

- `Unknown`
- `PathOnly`
- `MappedLocalFile`
- `EmbeddedSource`
- `Missing`
- `Redacted`

Rules:

- Never expose full local source paths in exported evidence by default.
- Allow users to map source roots explicitly.
- A missing source file should not block line-table import.
- Embedded source, if encountered, must be privacy-classified.

### DebugLineRow

Fields:

- `line_row_id`
- `compile_unit_id`
- `source_file_id`
- `address_space_id`
- `address_range`
- `line`
- `column`
- `is_stmt`
- `basic_block`
- `end_sequence`
- `prologue_end`
- `epilogue_begin`
- `isa`
- `discriminator`

Rules:

- Line rows are ranges, not point labels.
- Multiple line rows can map to the same address.
- An address can map to generated code or compiler helper ranges.
- Workbench should show line coverage uncertainty, not one fake exact line.

### DebugSubprogramRecord

Fields:

- `debug_function_id`
- `compile_unit_id`
- `die_offset`
- `linkage_name`
- `name`
- `decl_file`
- `decl_line`
- `address_ranges`
- `inline_kind`
- `external`
- `declaration_only`
- `frame_base_expression`
- `type_ref`
- `imported_signature_fact_id`
- `matched_procedure_id`

Rules:

- A debug subprogram can be a declaration only.
- A subprogram can have multiple address ranges.
- `linkage_name` and symbol table names can conflict.
- Debug function ranges can refine procedure discovery but must enter the
  conflict system.

### DebugInlineCallsiteRecord

Fields:

- `inline_callsite_id`
- `parent_debug_function_id`
- `abstract_origin_ref`
- `call_file`
- `call_line`
- `call_column`
- `address_ranges`
- `callee_name`
- `callee_linkage_name`

Rules:

- Inline callsites should be visible in the inspector and decompiler summary.
- Inline callsites should not become ordinary call edges unless marked as
  inline edges.
- Panic/debugger views should be able to expand an address into inline frames.

### DebugLexicalScopeRecord

Fields:

- `scope_id`
- `compile_unit_id`
- `parent_scope_id`
- `die_offset`
- `address_ranges`
- `name`
- `source_file_id`
- `line`

Rules:

- Lexical scopes provide variable visibility.
- Scopes can be nested and address-range sparse.
- If imports are partial, missing scope records should be explicit.

### DebugVariableRecord

Fields:

- `debug_variable_id`
- `compile_unit_id`
- `scope_id`
- `debug_function_id`
- `die_offset`
- `name`
- `linkage_name`
- `kind`
- `decl_file`
- `decl_line`
- `type_ref`
- `location_kind`
- `location_expression_id`
- `valid_ranges`
- `optimized_out`
- `imported_variable_fact_id`

`kind` enum:

- `Parameter`
- `Local`
- `Global`
- `StaticLocal`
- `Member`
- `Constant`
- `Unknown`

`location_kind` enum:

- `StackFrameOffset`
- `Register`
- `RegisterPiece`
- `MemoryAddress`
- `LocationExpression`
- `LocationList`
- `ConstantValue`
- `OptimizedOut`
- `Unknown`

Rules:

- Debug variable visibility is range-bound.
- Optimized-out variables should be first-class states.
- Damsel should not pretend to know runtime values from static DWARF.
- Stack-frame offsets should be merged carefully with Damsel's own stack
  analysis and user renames.

### DebugLocationExpression

Fields:

- `location_expression_id`
- `encoding`
- `raw_bytes`
- `decoded_ops`
- `range_list_ref`
- `requires_runtime_registers`
- `requires_cfa`
- `translation_status`
- `translation_problem_id`

Rules:

- The first wave can decode only a subset needed for stack offsets and simple
  register locations.
- Unsupported expressions should remain inspectable as raw bytes plus a
  problem record.
- Dynamic expressions should not be flattened into static stack variables.

### DebugTypeRecord

Fields:

- `debug_type_id`
- `compile_unit_id`
- `die_offset`
- `kind`
- `name`
- `byte_size`
- `alignment`
- `encoding`
- `type_ref`
- `containing_type_ref`
- `declaration_only`
- `complete`
- `language`
- `canonical_key`
- `imported_type_fact_id`

`kind` enum:

- `Base`
- `Pointer`
- `Reference`
- `RvalueReference`
- `Const`
- `Volatile`
- `Restrict`
- `Array`
- `Struct`
- `Class`
- `Union`
- `Enum`
- `Enumerator`
- `Subroutine`
- `Typedef`
- `Namespace`
- `TemplateParameter`
- `Member`
- `Inheritance`
- `ObjCInterface`
- `ObjCClass`
- `ObjCProtocol`
- `SwiftTypeName`
- `Unknown`

Rules:

- Debug types are not the same as final project types.
- Debug types are imported into Damsel's type store through a translator.
- The translator must tolerate incomplete types, declarations, duplicate DIEs,
  and One Definition Rule conflicts.
- Objective-C runtime metadata can outrank or complement debug types for class
  layout depending on source confidence.

### DebugTypeEdge

Fields:

- `edge_id`
- `from_type_id`
- `to_type_id`
- `edge_kind`
- `offset_bits`
- `size_bits`
- `name`
- `visibility`
- `virtuality`

`edge_kind` enum:

- `PointsTo`
- `Qualifies`
- `ArrayElement`
- `Member`
- `BaseClass`
- `ReturnType`
- `ParameterType`
- `TypedefTarget`
- `EnumCase`
- `TemplateArgument`

Rules:

- Type graph import should be lazy.
- Type cycles are normal.
- Missing target DIEs should create dangling references with problem records.

### DebugImportedFact

Fields:

- `fact_id`
- `debug_source_id`
- `import_session_id`
- `target_object_id`
- `fact_kind`
- `value`
- `confidence`
- `source_rank`
- `valid_range`
- `compile_unit_id`
- `die_offset`
- `created_at`

`fact_kind` enum:

- `Name`
- `LinkageName`
- `ProcedureRange`
- `FunctionSignature`
- `CallingConvention`
- `StackVariable`
- `RegisterVariable`
- `SourceLine`
- `InlineCallsite`
- `Type`
- `StructField`
- `EnumCase`
- `GlobalVariable`
- `Problem`

Rules:

- Imported facts flow into the same resolver as user facts, runtime facts,
  demangled facts, and decompiler facts.
- Debug facts should not overwrite user facts.
- Debug facts should be easy to disable by profile.

### SymbolicationProfile

Fields:

- `profile_id`
- `project_id`
- `name`
- `status`
- `active`
- `target_scope`
- `created_by`
- `created_at`
- `updated_at`
- `members`

`target_scope` enum:

- `WholeProject`
- `StandaloneImage`
- `SharedCache`
- `CacheImage`
- `KernelCollection`
- `FilesetMember`
- `Kext`
- `PanicSession`
- `DebuggerSession`

Profile member fields:

- `debug_source_id`
- `target_route`
- `match_candidate_id`
- `activation_status`
- `user_note`
- `priority`

Rules:

- Multiple profiles can coexist.
- Profile switching should be a transaction-like event that invalidates
  resolved names/types/signatures but preserves raw imported facts.
- Evidence exports should name the active profile and all inactive candidates.

### DebugInfoProblem

Problem kinds:

- `DsymUuidMismatch`
- `DsymArchitectureMismatch`
- `DsymMissingDwarfObject`
- `DsymEmptyDebugInfo`
- `DwarfUnsupportedVersion`
- `DwarfMalformedSection`
- `DwarfMissingLineTable`
- `DwarfUnsupportedForm`
- `DwarfUnsupportedLocationExpression`
- `DwarfAddressRangeOutsideImage`
- `DwarfOverlappingCompileUnits`
- `DwarfTypeCycleLimit`
- `DwarfTypeImportConflict`
- `KdkBuildMismatch`
- `KdkMissingKernelDsym`
- `KdkAmbiguousKernelDsym`
- `KdkUnsupportedLayout`
- `SourcePathPrivate`
- `SourceFileMissing`
- `SourceRootAmbiguous`
- `ImportCancelled`
- `ImportTimedOut`

Fields:

- `problem_id`
- `severity`
- `target_route`
- `debug_source_id`
- `import_session_id`
- `message`
- `technical_detail`
- `suggested_action`
- `privacy_class`
- `suppressed`

Rules:

- Problems must be route-backed.
- Privacy-sensitive paths must be redacted by default.
- A debug-info import with problems can still be useful.

## Matching Policy

### Match Strength Order

Preferred match hierarchy:

1. exact Mach-O UUID plus architecture
2. exact cache image identity plus contained debug UUID
3. exact kernel collection member identity plus KDK build and UUID
4. exact kext bundle id/version plus UUID
5. exact build version plus UUID where available
6. exact install name plus UUID where available
7. path and basename hints
8. user-forced match

Rules:

- A user-forced match is allowed only after a warning and should be marked as
  `ForcedByUser`.
- Path-only matches are never automatic.
- KDK build match without UUID is not exact.
- Multiple exact candidates should be marked ambiguous until the user chooses.

### Mismatch Behavior

Mismatch outcomes should become user-visible records:

- expected UUID vs found UUID
- expected architecture vs found architecture
- target build vs KDK build
- target fileset member vs dSYM member name
- target install name vs dSYM DWARF object name
- target platform vs debug source platform

Do not erase rejected candidates. They are part of the analysis audit trail.

### Symbolication Profile Activation

Activation should:

- validate candidate status
- create or update profile member
- enqueue import jobs
- mark existing resolved facts stale where the profile affects them
- emit project event
- update Workbench status

Deactivation should:

- leave raw imported facts in storage
- remove them from active resolution
- mark dependent render caches stale
- keep user-authored facts untouched

## Import Pipeline

### Stage 0: Discovery

Inputs:

- user-selected file
- user-selected directory
- KDK root
- dSYM bundle
- app bundle
- Xcode archive
- extracted IPSW/OTA root
- project-relative source root

Outputs:

- source candidates
- debug source records
- candidate match records
- problems

Implementation notes:

- Classify `.dSYM` bundles by bundle layout and contained DWARF objects.
- Classify KDK roots by known bundle/file patterns and metadata.
- Classify standalone DWARF-bearing Mach-O files separately from stripped
  target binaries.
- Do not recursively index huge roots until the user confirms or a policy
  allows it.

### Stage 1: Container Index

For each debug source:

- identify architecture slices
- identify Mach-O UUIDs where present
- identify contained DWARF object names
- index debug section presence
- index line-table presence
- index compile-unit count when cheap
- record size and estimated import cost

Performance rule:

The container index must be quick enough to run during intake. Deep DIE import
belongs in background jobs.

### Stage 2: Matching

For each target object:

- gather target identity
- compare candidate identity keys
- create candidate records
- create mismatch problems
- mark candidates exact, weak, rejected, or ambiguous

Target object types:

- standalone Mach-O image
- universal Mach-O slice
- app/framework executable
- dyld shared-cache image
- driver-cache dylib
- kernel collection
- fileset member
- kext
- panic session module
- debugger runtime module

### Stage 3: Profile Decision

Profile decisions can come from:

- automatic exact-match policy
- explicit user activation
- project setting
- CLI command
- future MCP mutation with permission

The activation result must be observable:

- status badge in Workbench
- project event
- CLI JSON response
- problem list updates

### Stage 4: Lazy Unit Import

Import compile units and cheap indexes first:

- compile-unit records
- source-file records
- address ranges
- line program summaries
- subprogram index
- type index roots

Avoid:

- full type graph import for all units
- full variable location decoding for all functions
- loading source files
- materializing giant strings into Swift

### Stage 5: Address And Line Import

Line import should build:

- address-to-line pages
- line-to-address pages
- source-file-to-function index
- procedure-to-source-span summary
- inline-frame summary where available

Workbench should be able to ask:

- "what source lines cover this instruction?"
- "what instructions correspond to this source line?"
- "which functions are in this source file?"
- "what inline stack exists at this address?"

### Stage 6: Function Import

Function import should:

- create debug subprogram records
- create name facts
- create procedure seed candidates
- refine procedure ranges when exact
- create conflicts when Damsel's recovered range disagrees
- attach source declaration locations
- attach inline callsite summaries

Conflict examples:

- debug function range overlaps two recovered procedures
- recovered function starts before debug `low_pc`
- debug range is declaration-only
- multiple debug subprograms map to one optimized function
- one debug subprogram maps to multiple discontiguous ranges

### Stage 7: Variable Import

Variable import should:

- import parameters and locals for active or requested functions
- translate simple location expressions
- attach stack offset facts where exact
- attach register variable facts where range-bound
- mark optimized-out variables
- record unsupported expressions
- avoid false certainty

Initial supported location forms:

- frame-base plus constant offset
- CFA plus constant offset if frame model exists
- direct register
- simple location list with address ranges

Unsupported initially:

- complex arithmetic expressions
- pieces requiring runtime register reconstruction
- target-specific implicit values
- expressions requiring memory reads

### Stage 8: Type Import

Type import should:

- import only types reachable from requested functions first
- canonicalize base types
- import pointer/array/qualifier chains lazily
- import structs/classes/unions/enums with field records
- import subroutine types as signature candidates
- record declaration-only and incomplete states
- detect ODR conflicts
- merge with Objective-C runtime class/ivar facts carefully
- merge with C++ RTTI/vtable facts carefully

Type import should not:

- assume debug type layout is correct for a mismatched source
- overwrite user-authored type names
- flatten all compile-unit-local types into global type names
- import entire KDK type graphs upfront

### Stage 9: Fact Resolution

Imported facts enter resolvers:

- name resolver
- type resolver
- signature resolver
- procedure resolver
- variable resolver
- source-line resolver
- inline-callsite resolver

Resolution order should align with the type/signature spec:

1. user-authored facts
2. exact active debug/KDK/dSYM facts
3. exact runtime metadata where stronger for Apple runtime layout
4. demangled facts
5. embedded database facts
6. data-flow/decompiler guesses
7. weak source/path facts

### Stage 10: Cache Invalidation

Profile changes invalidate:

- rendered assembly rows with names/source annotations
- function lists
- procedure summaries
- source-line panels
- decompiler summaries
- type inspectors
- variable panes
- evidence export caches
- debugger/panic frame symbolication

Invalidation should be object-scoped whenever possible.

## Workbench UX

### Debug Sources Navigator

The Workbench needs a navigator listing:

- active profiles
- attached dSYMs
- attached KDKs
- discovered but inactive candidates
- rejected candidates
- source roots
- import jobs
- problems

Each row should show:

- source kind
- match status
- target object
- architecture
- UUID/build status
- import progress
- coverage summary
- privacy status

### Symbolication Profile Panel

Profile panel commands:

- create profile
- duplicate profile
- activate profile
- deactivate profile
- add dSYM
- add KDK
- add source root
- reject candidate
- force candidate with warning
- explain match
- export profile summary

Profile status summary:

- exact matches
- weak matches
- rejected candidates
- ambiguous candidates
- imported compile units
- imported source lines
- imported functions
- imported variables
- imported types
- stale facts

### Source Line Inspector

For an instruction route:

- source file basename
- redacted path or mapped source root
- line and column
- inline frame stack
- compile unit
- debug source
- match confidence
- active profile
- "open source" only when source is mapped and allowed

If no line exists:

- show "no active debug line"
- show whether a candidate dSYM exists but is inactive
- show whether line tables are absent
- show whether import is still running

### Function Debug Info Panel

For a function route:

- debug name and linkage name
- declaration source location
- compile unit
- imported range(s)
- recovered range(s)
- range conflicts
- inline callsites
- parameters
- locals
- imported signature
- active calling convention
- type import status

### Variable Pane

For a function route:

- parameters
- stack locals
- register variables
- globals referenced by debug info
- optimized-out variables
- unknown/unsupported locations
- validity ranges
- source declaration lines
- type links

UX rules:

- Do not hide optimized-out variables.
- Show unsupported location expressions as unsupported, not missing.
- Preserve user variable renames above debug names.
- Let users toggle "debug names" vs "user names" vs "analysis names."

### Source File Browser

The browser should support:

- list source files from active profile
- search by basename
- filter by compile unit
- show coverage counts
- show mapped/unmapped/private source status
- open line route when source content is unavailable
- open actual source only when mapped

This lets Damsel offer source-level navigation even when local source text is
not available.

### Debug Info Problem Navigator

Problem rows should be actionable:

- mismatched UUID -> show expected/found and target route
- missing source file -> offer source-root mapping
- KDK build mismatch -> show target build and KDK build
- unsupported DWARF form -> show import scope and severity
- import cancelled -> offer resume
- ambiguous candidate -> offer candidate picker

### Evidence Export UX

When exporting evidence:

- include active profile id/name
- include debug sources by redacted identity
- include match status
- include imported fact provenance
- redact private source paths by default
- omit source file contents by default
- include mismatch problems if they affect findings

## CLI And API Contracts

### CLI Commands

Suggested commands:

```sh
damsel project debug sources <project>
damsel project debug discover <project> <path>
damsel project debug candidates <project> --target <route>
damsel project debug attach-dsym <project> <path> --target <route>
damsel project debug attach-kdk <project> <path> --target <route>
damsel project debug activate <project> <candidate-id> --profile <name>
damsel project debug profile <project> --json
damsel project debug import <project> --profile <profile-id>
damsel project debug line <project> --address <route-or-address>
damsel project debug source-files <project> --query <text>
damsel project debug function <project> --function <route>
damsel project debug variables <project> --function <route>
damsel project debug types <project> --query <text>
damsel project debug explain-match <project> <candidate-id>
```

Standalone transitional commands:

```sh
damsel debug discover <binary> <debug-path> --json
damsel debug explain-match <binary> <debug-path> --json
damsel debug line <binary> <debug-path> --address 0x...
```

Transitional commands are allowed only as stepping stones. The real system
belongs in project mode.

### HTTP API Endpoints

Suggested project API:

- `GET /api/projects/:id/debug/sources`
- `POST /api/projects/:id/debug/discover`
- `GET /api/projects/:id/debug/candidates`
- `POST /api/projects/:id/debug/profiles`
- `GET /api/projects/:id/debug/profiles`
- `POST /api/projects/:id/debug/profiles/:profile/activate-candidate`
- `POST /api/projects/:id/debug/profiles/:profile/deactivate-candidate`
- `POST /api/projects/:id/debug/import`
- `GET /api/projects/:id/debug/line`
- `GET /api/projects/:id/debug/source-files`
- `GET /api/projects/:id/debug/functions/:function/debug-info`
- `GET /api/projects/:id/debug/functions/:function/variables`
- `GET /api/projects/:id/debug/types`
- `GET /api/projects/:id/debug/problems`
- `GET /api/projects/:id/debug/candidates/:candidate/explain`

Response rules:

- All paged lists use cursors.
- All private paths include redaction state.
- All imported facts include provenance ids.
- All match responses include positive and negative evidence.
- All long imports return job ids and event stream updates.

### MCP And Automation Surface

Read-only first:

- list debug sources
- explain debug match
- get source line for address
- get function debug info
- get variable list
- get imported signature
- get debug problems
- summarize active symbolication profile

Mutation later, permissioned:

- attach dSYM
- attach KDK
- activate candidate
- reject candidate
- add source root mapping

Every mutation must create an activity-log entry.

## Integration With Existing Code

### `damsel-core`

Add debug-info models without coupling them to Mach-O parsing internals.

Initial exports:

- `DebugInfoSource`
- `DebugObjectIdentity`
- `DebugInfoMatchCandidate`
- `DebugInfoImportSession`
- `DebugCompileUnitRecord`
- `DebugSourceFileRecord`
- `DebugLineRow`
- `DebugSubprogramRecord`
- `DebugVariableRecord`
- `DebugTypeRecord`
- `SymbolicationProfile`
- `DebugInfoProblem`

Add route types:

- `DebugSourceRoute`
- `CompileUnitRoute`
- `SourceFileRoute`
- `SourceLineRoute`
- `DebugFunctionRoute`
- `DebugVariableRoute`
- `DebugTypeRoute`
- `SymbolicationProfileRoute`

### `damsel-macho`

Responsibilities:

- extract Mach-O UUIDs for target images
- expose UUID/build identity in `BinaryImage`
- classify dSYM bundles
- parse contained Mach-O debug objects
- index DWARF sections
- parse enough DWARF for compile units, line tables, subprograms, variables,
  and type skeletons
- classify KDKs
- match KDK dSYMs to kernel targets
- feed project debug-info models

Implementation options:

- start with `object` crate DWARF support where adequate
- consider `gimli` directly for DWARF parsing
- keep parser outputs independent from project storage
- build fixture-focused parsers before real KDK scale work

### `damsel-cli`

Responsibilities:

- expose JSON contracts first
- preserve exact key ordering in snapshots
- provide transitional standalone debug commands if useful
- route project debug commands through project APIs
- avoid printing private source paths by default

### Workbench

Responsibilities:

- never load entire dSYM/KDK contents into Swift memory
- display import jobs and cancellation
- show sidecar match explanations
- show active profile state
- add source-line inspector modules
- add variable panes
- add source-file browser
- handle redaction and source-root mapping

### Decompiler And Procedure Database

Debug info should enrich:

- procedure seeds
- function ranges
- names
- parameters
- locals
- stack slots
- register aliases
- source spans
- inline callsites
- signatures
- type constraints

Debug info should not:

- force the decompiler to trust invalid ranges
- hide conflicts with recovered CFGs
- remove user edits
- make unsupported variables appear known

### Debugger And Panic Logs

Debug info should power:

- current PC source line
- inline frame expansion
- route-backed breakpoints
- panic frame symbolication
- kext/function/source mapping
- source-line context for crash frames

Debugger runtime addresses need:

- module load address
- slide
- static route mapping
- active symbolication profile
- source-line lookup

Panic addresses need:

- kernel slide
- collection/fileset member
- kext mapping
- KDK/dSYM profile
- frame trust level

## Storage And Indexing

### Tables

Minimum tables:

- `debug_sources`
- `debug_source_identities`
- `debug_match_candidates`
- `debug_import_sessions`
- `debug_compile_units`
- `debug_source_files`
- `debug_line_rows`
- `debug_subprograms`
- `debug_inline_callsites`
- `debug_scopes`
- `debug_variables`
- `debug_location_expressions`
- `debug_types`
- `debug_type_edges`
- `debug_imported_facts`
- `symbolication_profiles`
- `symbolication_profile_members`
- `debug_info_problems`

### Indexes

Required indexes:

- source id by project
- candidate by target route
- candidate by status
- UUID plus architecture
- KDK build
- cache image id
- fileset member
- compile unit by address range
- line row by address range
- line row by source file and line
- subprogram by address range
- variable by function and range
- type by canonical key
- problem by severity and route

### Large-Artifact Rules

- Store raw DWARF bytes outside ordinary relational rows.
- Use memory-mapped access where possible.
- Import line tables and function indexes before full type graphs.
- Shard indexes by debug source and address-space range.
- Cache decoded abbreviation/string tables per import job.
- Cancel safely between compile units.
- Persist progress after each commit batch.

## Privacy And Security

Debug info can reveal:

- developer usernames
- home directory paths
- private repository paths
- source filenames
- source content
- project structure
- build machine paths
- internal type names
- proprietary comments in embedded source

Default policy:

- store private paths with privacy class
- redact private paths in reports
- do not copy source contents into project bundles unless user opts in
- do not send debug facts to MCP/external tools without permission
- include privacy summary in evidence exports
- allow source-root mapping without publishing full root path

Security considerations:

- DWARF parsers must treat inputs as untrusted.
- KDK/dSYM files can be huge and malformed.
- Parser errors must become problems, not panics.
- Import jobs need memory and time limits.
- Debug-source plugins must be capability-restricted.

## Implementation Phases

### D0: Current-State Audit

Deliverables:

- verify current UUID/build metadata exposure
- identify missing model fields in `BinaryImage`
- identify shared-cache image identity gaps
- identify fixture support for dSYM generation
- choose initial DWARF parser approach

Validation:

- repo-grounded audit document or issue
- no implementation claim

### D1: Identity Foundation

Deliverables:

- expose Mach-O UUID in standalone `BinaryImage`
- expose build-version/platform where available
- expose UUID/arch in CLI JSON
- add target debug identity model
- add tests for UUID extraction

Validation:

- synthetic Mach-O UUID fixture
- universal binary arch/UUID fixture
- CLI JSON snapshots

### D2: Debug Source Registry

Deliverables:

- add debug source model
- add dSYM classifier
- add standalone debug object classifier
- add debug source registry commands
- add Workbench debug source list skeleton

Validation:

- dSYM bundle layout fixture
- missing DWARF object fixture
- unsupported source fixture

### D3: Matching Engine

Deliverables:

- UUID/arch match engine
- mismatch problem records
- candidate explanation API
- profile model skeleton

Validation:

- exact dSYM fixture
- UUID mismatch fixture
- arch mismatch fixture
- ambiguous candidate fixture

### D4: Compile Unit And Line Table Import

Deliverables:

- parse compile units
- parse source-file tables
- parse line programs
- create address-to-line and line-to-address queries
- add source-line inspector endpoint

Validation:

- simple C line table fixture
- C++ line table fixture
- ObjC line table fixture if feasible
- stripped binary plus dSYM fixture

### D5: Workbench Source-Line UX

Deliverables:

- source-line inspector
- source-file browser
- profile badge
- import job progress
- candidate explanation sheet

Validation:

- Swift store tests
- UI API tests
- Workbench smoke with fixture dSYM

### D6: Function Range And Name Import

Deliverables:

- parse subprogram DIEs
- import function names/linkage names
- import declaration source locations
- create procedure seed candidates
- create procedure range conflicts

Validation:

- function range fixture
- inexact recovered range conflict fixture
- declaration-only fixture
- discontiguous range fixture

### D7: Inline Callsite Import

Deliverables:

- parse inline subroutine records
- add inline-frame lookup by address
- show inline stack in inspector
- expose inline frames to debugger/panic mapping

Validation:

- optimized inline fixture
- nested inline fixture
- no-inline fallback fixture

### D8: Variable Import

Deliverables:

- parse parameters and locals
- translate simple stack/register locations
- import validity ranges
- represent optimized-out variables
- add variable pane

Validation:

- stack local fixture
- register variable fixture
- optimized-out fixture
- unsupported location expression fixture

### D9: Type Skeleton Import

Deliverables:

- parse base/pointer/array/qualifier types
- parse structs/classes/unions/enums
- parse subroutine type signatures
- import reachable types lazily
- create type conflict records

Validation:

- C struct fixture
- C++ class fixture
- enum base fixture
- duplicate type conflict fixture

### D10: KDK Bundle Support

Deliverables:

- KDK classifier
- KDK build/version identity
- contained dSYM inventory
- kernel/fileset/kext match engine
- KDK profile activation

Validation:

- synthetic KDK layout fixture
- KDK build mismatch fixture
- kernel dSYM match fixture
- kext dSYM match fixture

### D11: Panic And Debugger Integration

Deliverables:

- panic frame source-line lookup
- runtime PC source-line lookup
- inline frame expansion
- profile-aware breakpoint source context

Validation:

- synthetic panic frame fixture
- local debug executable with dSYM
- slide mapping fixture

### D12: Performance Hardening

Deliverables:

- lazy type graph loading
- compile-unit shard indexes
- import cancellation and resume
- memory budget metrics
- KDK-scale opt-in smoke

Validation:

- large synthetic DWARF corpus
- import cancellation fixture
- memory budget report
- opt-in KDK smoke if available

## Validation Matrix

| Requirement | Fixture evidence | Real evidence | UI evidence |
| --- | --- | --- | --- |
| dSYM classification | synthetic `.dSYM` bundle | user-supplied app dSYM | source list row |
| UUID matching | exact/mismatch UUID fixtures | local app binary+dSYM | match explanation |
| line import | C/C++/ObjC line fixtures | local debug executable | source-line inspector |
| function import | subprogram fixtures | framework sample with dSYM | function panel |
| inline frames | optimized inline fixture | local optimized sample | inline stack display |
| variables | stack/register/optimized-out fixtures | local debug sample | variable pane |
| type import | struct/class/enum fixtures | local C++ sample | type navigator |
| KDK matching | synthetic KDK layout | installed KDK opt-in | KDK status panel |
| panic mapping | synthetic panic frames | sanitized panic opt-in | panic route source lines |
| privacy | private path fixture | redacted evidence package | redaction badge |
| cancellation | large import fixture | large dSYM/KDK opt-in | cancellable job row |

## Failure Modes

High-risk failures:

- mismatched dSYM silently applied
- KDK from the wrong OS build trusted
- private source path leaked in exported report
- Workbench freezes while indexing KDK DWARF
- debug names overwrite user labels
- optimized-out variable shown as available
- inline callsite rendered as real call edge
- line table point mapped as exact range incorrectly
- huge type graph imported eagerly
- malformed DWARF crashes importer
- source-root mapping exposes private repo path to MCP

Mitigations:

- explicit match status model
- profile activation gate
- route-backed problems
- privacy classes on source paths
- lazy import
- cancellable jobs
- conflict records
- provenance and source-rank resolver
- parser fuzz/fixture tests

## Completion Criteria

This subsystem is not complete when Damsel can only print a dSYM symbol name.
Minimum credible completion:

- project can register dSYM/KDK debug sources
- exact and mismatched candidates are visible and explainable
- active symbolication profile controls imported facts
- line-table import works for checked-in fixtures
- function names/ranges import as conflict-aware facts
- variables import at least simple stack/register cases
- types import at least reachable C/C++ records and signatures
- Workbench shows source-line, profile, and problem state
- CLI/API expose the same facts as Workbench
- privacy redaction is tested
- imports are cancellable
- large debug sources are lazy/paged
- evidence matrix names real artifacts only when actually run

## Open Decisions

- Use `gimli` directly or rely on `object` wrappers first?
- Store imported DWARF raw ranges in project DB or external sidecar cache?
- How much source-root mapping belongs in project settings versus app
  preferences?
- Should symbolication profiles be global project objects or scoped per
  target artifact by default?
- How aggressively should debug subprogram ranges correct procedure recovery?
- Should KDK indexing require explicit opt-in every time?
- Should source line info be shown in assembly rows by default or inspector
  only?
- What is the first acceptable Swift/ObjC debug-type representation?

## Summary

The debug-info subsystem is a deep refactor, not a small importer. It touches
source intake, identity, matching, project storage, type resolution, procedure
recovery, Workbench UX, privacy, performance, debugger overlays, panic
workflows, evidence export, and automation.

That is exactly why it belongs in the Hopper-gap pack. Hopper's public SDK
shows debug data as a loader/document/procedure/type integration point.
Damsel's Apple niche makes that integration even more important: KDKs, dSYMs,
source lines, XNU filesets, kexts, ObjC/C++ metadata, and panic traces should
become one explainable symbolication system rather than disconnected features.
