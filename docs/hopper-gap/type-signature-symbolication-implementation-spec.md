# Type, Signature, Symbolication, And Debug-Info Implementation Spec

Status: implementation-grade architecture spec.

Date of public-source pass: 2026-05-26.

Purpose: turn the Hopper public-web and public-SDK evidence around types,
signatures, variables, debug data, symbolication, ObjC metadata, C++ metadata,
Swift-adjacent names, and Apple sidecars into a concrete Damsel implementation
plan.

This is a companion to:

- [Type And Symbolication System Plan](type-symbolication-system-plan.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)

## Why This Exists

The earlier type-symbolication plan is directionally right but too small.
Hopper's public surface does not treat types as a small decompiler accessory.
It treats names, byte types, function signatures, calling conventions, local
variables, register aliases, structured data, debug sidecars, user edits,
scripts, and procedure metadata as part of the same document database.

For Damsel, this means the next credible Hopper-adjacent system cannot be:

- only Swift demangling
- only ObjC type encoding display
- only dSYM symbol import
- only a KDK path setting
- only a type parser
- only a pseudocode decoration pass

It has to be a project-wide type and symbolication kernel that every view and
analysis pass can query, mutate, explain, and roll back.

## Public Source Set

Primary Hopper sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page and SDK listing: <https://www.hopperapp.com/download>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper Debugger Server page:
  <https://www.hopperapp.com/HopperDebuggerServer/>
- Hopper SDK archive from the public download page:
  <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>

Public workflow and ecosystem sources:

- Hopper Python API type stubs on PyPI:
  <https://pypi.org/project/types-hopper-disassembler/>
- Hopper Python API type-stub repository:
  <https://github.com/Fi5t/hopper-disassembler-stubs>
- Public Hopper script collection:
  <https://github.com/droe/hopper-scripts>
- macOS Hopper script examples:
  <https://github.com/melomac/Hopper>
- HopperPyMCP wrapper notes:
  <https://glama.ai/mcp/servers/dflatline/HopperPyMCP>
- Public macOS private-framework workflow using Hopper:
  <https://www.jviotti.com/2023/11/20/exploring-macos-private-frameworks.html>
- Reverse Engineering Stack Exchange Hopper type/local-variable discussion:
  <https://reverseengineering.stackexchange.com/questions/14973/basic-actions-in-hopper-disassembler>

External platform/format sources to use during implementation:

- LLVM DWARF and debug-info documentation:
  <https://llvm.org/docs/SourceLevelDebugging.html>
- Itanium C++ ABI:
  <https://itanium-cxx-abi.github.io/cxx-abi/abi.html>
- Apple Developer Downloads for KDK/dSYM acquisition:
  <https://developer.apple.com/download/>

## Public Evidence Summary

### Product Evidence

Hopper's homepage publicly positions Hopper as:

- a native macOS disassembler
- a decompiler
- a debugger with LLDB and GDB support
- an Objective-C and Swift aware tool
- a scriptable Python environment
- an SDK-extensible platform
- an MCP-enabled product

The type/signature implication is that Damsel's type system cannot be a narrow
offline importer. It has to support:

- native UI editing
- decompiler inputs
- debugger overlays
- ObjC and Swift name/type recovery
- scripting and MCP reads
- future audited mutations
- plugin/sidecar imports

### Tutorial Evidence

Hopper's tutorial is especially important because it describes the mental model
of the application, not only a list of features.

Relevant public tutorial facts:

- Hopper transforms bytes into readable analysis by assigning a type to bytes.
- The typed byte states include data, ASCII, code, procedure, and undefined.
- Users can manually change byte types.
- Undo/redo is part of the edit loop.
- Procedure construction is required for richer representations.
- Procedures carry basic-block and stack information.
- Hopper has assembly, graph, pseudocode, and hex representations.
- Hex is synchronized with assembly and knows instruction byte spans.
- Graph nodes can be edited, grouped, colored, and commented.
- The navigation bar shows file-wide byte type regions.
- The inspector changes based on context.
- The inspector supports operand formatting.
- The inspector supports comments, colors, tags, references, and procedure
  details.
- The procedure inspector exposes predecessor/successor context and switch
  hints.

The type/signature implication is that typed bytes, procedure metadata,
operand formats, comments, tags, xrefs, signatures, stack locals, and graph
state are not independent UI features. They are different projections of one
mutable document.

### FAQ And Download Evidence

The FAQ says Hopper:

- handles Intel, ARM, ARM64, and PowerPC binaries
- can disassemble iOS binaries, with FairPlay-encrypted app-store binaries
  explicitly out of scope unless decrypted by the user
- reads Mach-O, PE, and ELF
- supports embedded hex editing, assembling new instructions, and Python byte
  modification
- exposes MCP configuration through Hopper Help

The download page publicly lists an SDK package.

The type/signature implication is:

- Damsel should not design type import only for one local Mach-O file.
- Damsel should model unsupported/encrypted source states as explicit intake
  problems.
- Damsel should design type edits with transaction and invalidation support,
  even if initial Damsel remains read-mostly.
- Damsel's future automation surfaces must read the same type/signature objects
  used by the UI.

### Changelog Evidence

The changelog is the strongest public proof that Hopper's type/signature layer
is product-critical and actively maintained.

Recent and relevant entries include:

- 2026-05-19, v6.3.0: Swift demangler crash fix.
- 2026-05-05, v6.2.9: Python interpreter fix for newer library versions.
- 2026-04-28, v6.2.7: large-binary loading, large-function decompilation, and
  Intel analysis improvements.
- 2026-04-07, v6.2.6: analysis-time optimization for very large binaries.
- 2026-03-27, v6.2.5: MCP server rewrite in C++.
- 2026-03-10, v6.2.0: faster large-database loading, smaller memory footprint,
  MCP reliability, and UI tweaks.
- 2026-02-02, v6.1.0: linear disassembly option and driver-cache dylib reading.
- 2025-12-09, v6.0.12: improved method navigation for files extracted from the
  shared dyld cache.
- 2025-09-10, v6.0.0: new decompiler, MCP server, switch/case improvements,
  Swift string/demangling support, and AArch64 authenticated Objective-C
  selector trampolines.
- 2025-09-11, v6.0.1: fixes for stripped Objective-C declarations and
  Objective-C ivar display in the decompiler.
- 2025-05-09, v5.19.0: stack-variable handling, variable rename, liveness, and
  AST optimization.
- 2025-05-14, v5.19.2: Objective-C stubs and new retain/release functions.
- 2025-01-21, v5.17.4: Objective-C header export from dyld shared cache.
- 2024-08-30, v5.17.1: switch-hint multiplier support and address-difference
  formatting.
- 2024-03-11, v5.14.3: BSS section modification.
- 2024-02-19, v5.14.1: Objective-C header export and C type parser fixes.
- 2024-01-16, v5.14.0: command palette, ARM call-construct detection, and
  variable-assignment decompiler fixes.
- 2023-10-10, v5.13.0: C++ mangled-name method-signature computation.
- 2023-05-05, v5.10.0: iOS and macOS kernel cache support for unencrypted
  caches.
- 2022-11-18, v5.8.0: Swift metadata comments and dyld shared-cache symbol
  handling.
- 2022-11-07, v5.7.8: debugger environment variables, debugger hex column,
  structs with relative pointers, Python script refresh, Objective-C section
  parsing, and old iOS dyld cache crash fixes.
- 2022-05-31, v5.6.0: reorder fields of structures, unions, enums, and method
  signature arguments.
- 2022-03-22, v5.5.0: enum base types in the type editor and quick library
  loading from current dyld cache.
- 2021-11-26, v5.1.4: argument-analysis fix.
- 2021-11-25, v5.1.3: method-argument detection and signature-editor
  improvements.

The implementation implication is direct:

- signatures need creation reasons and provenance
- stack variables need names, types, liveness, and editor hooks
- ObjC ivars and stripped ObjC declarations must be represented as imperfect
  facts, not only display strings
- Swift demangling must be a recoverable/imported fact with failure modes
- C++ demangling must produce signature candidates, not just pretty labels
- shared-cache extracted files need method navigation and type context
- type editing needs field order, enum base type, relative pointers, and method
  argument ordering
- large-binary performance is part of the type system design

### SDK Evidence

The public SDK 6.3.0 headers expose the clearest object model.

The inspected public headers include:

- `HPDisassembledFile.h`
- `HPProcedure.h`
- `HPTypeDesc.h`
- `HPMethodSignature.h`
- `HPMethodArgument.h`
- `HPTypeStructField.h`
- `HPTypeEnumField.h`
- `CommonTypes.h`
- `FileLoader.h`
- `CPUContext.h`
- `CPUDefinition.h`

Important SDK facts:

- `HPDisassembledFile` owns names, comments, byte types, operand formats,
  procedures, colors, tags, problems, assembler hooks, undo/redo transactions,
  type construction, structure placement, type database enumeration, type
  lookup, type import, and type export.
- `HPTypeDesc` models primitive, void, float/double, pointer, function pointer,
  array, structured, enum, named, forward-declared, incomplete, display, field,
  enum, and function-pointer signature facts.
- `HPMethodSignature` owns return type, argument list, argument names, argument
  insertion/removal, and string rendering.
- `HPMethodArgument` owns an argument's name and type.
- `HPTypeStructField` owns field name, comment, display format, field index,
  and rendered strings.
- `HPTypeEnumField` owns enum case name and value.
- `HPProcedure` owns stack-frame facts, basic blocks, local labels, register
  renaming, callers, callees, variable names by displacement, signatures,
  signature propagation, calling conventions, resolved calling conventions, and
  tags.
- `CommonTypes.h` exposes byte types including structure, operand formats
  including stack variable, structured, enum, address difference, signed, and
  leading zero forms.
- `CommonTypes.h` exposes signature creation reasons: unknown, guessed from
  decompilation, guessed from data flow, call-derived, database, demangling,
  and user.
- `CommonTypes.h` exposes calling conventions including C variants, AAPCS,
  AAPCS VFP, Swift, x86_64 variants, and special AArch64 Objective-C runtime
  conventions where argument zero starts in x1 through x28.
- `FileLoader` can load debug data and return mismatch statuses including debug
  mismatch and UUID mismatch.
- `FileLoader` has options for parsing Objective-C, Swift, and exceptions.
- `CPUContext` participates in procedure discovery, prolog/epilog analysis,
  branch analysis, switch detection, instruction-specific analysis, rendering,
  and decompiler hooks.

The SDK does not expose Hopper's private database schema or full decompiler
implementation. That boundary matters. Damsel should not infer private
algorithms. But the public object contracts are enough to infer the shape of a
Hopper-like architecture.

### Python And MCP Ecosystem Evidence

Public Python stubs and scripts show analysts expect document automation over
the same facts visible in the UI.

Public script surfaces include:

- document selection
- segment enumeration
- named-address enumeration
- procedure-name export
- byte reads
- raw selection export
- Yara and shellcode annotation
- comments and labels
- file offset mapping
- imported-name repair
- Swift demangling scripts
- decompile-to-editor scripts

HopperPyMCP publicly exposes a third-party MCP wrapper with tools for:

- document management
- segment listing
- address information
- name and string search
- disassembling and decompiling procedures
- call graph generation
- demangled-name lookup
- comments
- setting comments and names
- marking data type at an address

The implementation implication is:

- Damsel must not make the Workbench the only way to use type information.
- Damsel's type/signature system needs stable object IDs and route-backed JSON.
- MCP and scripts must read exact records and cite sources, not scrape prose.
- Mutations must be audited, source-scoped, and undoable when enabled.

### Public Apple Workflow Evidence

The macOS private-framework workflow article is especially relevant to Damsel's
niche. It describes using Hopper to:

- open the macOS dyld shared cache directly
- browse for a target private framework inside the cache
- select a framework from Hopper's dyld shared-cache loader
- find a method through labels
- inspect pseudocode
- export an Objective-C header file from a framework

The implementation implication is:

- Damsel needs cache-aware type and signature context, not only per-file types.
- Objective-C header export depends on recovered classes, protocols, methods,
  properties, ivars, type encodings, and symbolication profiles.
- A dyld-cache member route must carry enough identity for header export,
  method navigation, source context, and future diffing.
- Damsel's competitive lane should make this workflow deeper than Hopper by
  keeping cache provenance, provider/importer context, KDK/dSYM match state,
  and runtime-owner graphs visible.

The Reverse Engineering Stack Exchange thread also shows users care about:

- adding structures to the database
- assigning types to local variables
- register synonyms
- local variable type editing through the inspector in later versions

That is not official Hopper documentation, but it confirms the analyst mental
model: type editing is an expected interactive workflow, not an afterthought.

## Current Damsel Baseline

Damsel already has strong Apple-analysis substrate:

- standalone Mach-O parsing
- arm64 and arm64e architecture coverage
- universal/thin slice handling
- sections, symbols, imports, relocations, exports
- dyld metadata for standalone images
- Objective-C metadata extraction
- Objective-C classes, categories, protocols, selectors, methods, properties,
  ivars, and pointer references
- ObjC method and ivar type-encoding strings where present
- shared-cache inventory and image records
- cache symbolication against export and local symbol sources
- projected-image analysis
- cache dependency/provider/importer/reexport exploration
- AArch64 disassembly with blocks, edges, calls, branch targets, data
  references, imports, recovered values, jump tables, ObjC references, and
  cache links
- Swift symbol resolution/demangling support in CLI paths
- native Workbench shell and UI server surface in progress

But Damsel does not yet have a Hopper-class type/signature system:

- no project-wide durable type database
- no first-class type record graph
- no provenance-aware type facts
- no structured field/enum editor
- no type import/export snapshot
- no function signature model
- no signature creation reasons
- no signature propagation controls
- no calling convention model
- no special AArch64 Objective-C runtime conventions
- no stack frame record model
- no local variable naming/type model
- no register alias model
- no local-label model
- no dSYM/DWARF importer
- no KDK profile importer
- no source line table records
- no inline callsite records
- no type conflict/problem navigator
- no user-vs-imported precedence engine
- no Workbench type navigator
- no signature editor
- no local variable inspector
- no source panel
- no MCP/script type API
- no type-aware decompiler input layer

## Product Thesis

Damsel should beat Hopper in the Apple systems niche by making type and
symbolication evidence more explainable than Hopper.

The target experience:

- The user opens a Mach-O, shared cache, kernel collection, KDK, dSYM, panic
  log, or extracted artifact root.
- Damsel builds a project with stable source identities.
- Damsel imports names, types, signatures, line tables, ObjC encodings, C++
  demangled names, Swift demangled names, KDK profiles, and cache-local symbols
  as separate facts.
- The Workbench shows which fact won and why.
- Conflicting facts remain visible.
- User edits never disappear under reanalysis.
- dSYM/KDK mismatches become explicit project problems.
- All derived signatures can be traced back to the importer, demangler,
  analyzer, callsite, or user edit that created them.
- The decompiler/IR path consumes typed facts only with declared confidence.
- Automation sees the same objects as the UI.

The rule is simple: Damsel should prefer an honest partial type system over a
pretty but untraceable one.

## Target Architecture

The implementation should add a `TypeAndSymbolicationKernel` above today's
binary image and cache models.

Conceptual shape:

```text
SourceArtifact / AnalysisProject
  |
  +-- NameStore
  +-- TypeStore
  +-- SignatureStore
  +-- VariableStore
  +-- CallingConventionStore
  +-- SymbolicationProfileStore
  +-- DebugInfoStore
  +-- TypeConflictStore
  +-- TypeTransactionLog
  |
  +-- Importers
  |     +-- ObjcTypeEncodingImporter
  |     +-- ObjcPropertyAttributeImporter
  |     +-- CppDemangleImporter
  |     +-- CppRttiVtableImporter
  |     +-- SwiftDemangleImporter
  |     +-- DwarfDsymImporter
  |     +-- KdkProfileImporter
  |     +-- CompactUnwindFrameImporter
  |     +-- UserTypeImportExporter
  |
  +-- Consumers
        +-- SemanticAssemblyRenderer
        +-- ProcedureDatabase
        +-- ObjcRuntimeExplorer
        +-- CppObjcxxExplorer
        +-- KernelExplorer
        +-- DecompilerIR
        +-- WorkbenchInspector
        +-- CLI JSON
        +-- MCP/Scripting
        +-- Export/Diff/Evidence
```

Each store should live in the project/document kernel, not in one UI store or
one CLI response type.

## Core Invariants

### Invariant 1: Facts Are Not Display Strings

Display strings are derived.

The stored facts are:

- name facts
- type records
- type assignments
- signature records
- signature candidates
- variable records
- calling convention records
- debug-info records
- profile records
- conflict records
- transaction records

The assembly renderer, inspector, source panel, ObjC browser, C++ browser,
kernel browser, CLI, and MCP server all derive text from those facts.

### Invariant 2: Provenance Is Mandatory

Every nontrivial type/name/signature fact needs:

- source kind
- source object id or file identity
- importer/analyzer id
- importer/analyzer version
- timestamp or project revision
- confidence
- precedence class
- stale flag
- conflict group id when relevant

Without provenance, a type fact should not be allowed into the durable store.

### Invariant 3: User Facts Are Stronger Than Imported Facts

User-authored labels, types, signatures, local variable names, field names,
enum names, and register aliases must survive:

- reanalysis
- profile changes
- dSYM replacement
- KDK replacement
- source re-registration
- importer upgrades
- cache projection changes

Imported facts can be hidden, marked stale, superseded, or conflict-linked.
They cannot silently overwrite user facts.

### Invariant 4: Exact Sidecars Must Be Matched, Not Trusted By Path

dSYM and KDK sidecars need UUID/build/architecture matching.

Path alone is not proof.

The project must store:

- matched UUIDs
- matched architecture
- matched OS version/build when available
- sidecar path
- source artifact identity
- match status
- mismatch reason
- imported fact counts
- skipped fact counts

### Invariant 5: Weak Inference Stays Weak

Load width, register role, selector name, demangled function name, or C++ vtable
shape can propose types. They should not be rendered like exact debug info.

Weak facts still matter. They should:

- feed search
- feed decompiler hints
- appear in inspector candidates
- explain what Damsel thinks
- be easy to promote to user facts

### Invariant 6: Type Edits Are Transactions

A signature edit might update:

- return type
- argument list
- argument names
- calling convention
- procedure display
- decompiler inputs
- callsite summaries
- local variable records
- export/diff snapshots

Therefore edits need transaction ids, undo/redo support, invalidation scopes,
and reanalysis triggers.

### Invariant 7: Apple Routing Comes First

Damsel's type targets are not only addresses.

Targets include:

- project route
- source artifact route
- source range
- virtual address
- cache global address
- projected image address
- dyld cache image
- kernel collection member
- kext
- function id
- basic block id
- instruction id
- operand id
- ObjC class
- ObjC protocol
- ObjC category
- ObjC selector
- ObjC method
- ObjC ivar
- ObjC property
- C++ typeinfo
- vtable slot
- Swift symbol
- panic frame
- debug compile unit
- source line

If the target model is address-only, the Apple systems workflow will collapse.

## Core Data Model

### Identity Types

Add stable IDs:

- `TypeId`
- `TypeFactId`
- `NameFactId`
- `SignatureId`
- `SignatureCandidateId`
- `CallingConventionFactId`
- `VariableId`
- `FrameId`
- `DebugInfoUnitId`
- `SourceLineId`
- `SymbolicationProfileId`
- `TypeImportReceiptId`
- `TypeConflictId`
- `TypeTransactionId`

IDs should be project-scoped and stable across reopen when the backing fact is
durable.

### Provenance

`FactProvenance`:

- `provenance_id`
- `source_kind`
- `source_route`
- `source_artifact_id`
- `source_revision`
- `importer`
- `importer_version`
- `created_at_project_revision`
- `confidence`
- `precedence`
- `is_user_authored`
- `is_imported`
- `is_inferred`
- `is_stale`
- `notes`

`source_kind` values:

- `user`
- `script`
- `mcp_tool`
- `plugin`
- `mach_o_symbol_table`
- `mach_o_export_trie`
- `mach_o_import`
- `dyld_chained_fixup`
- `dyld_cache_local_symbol`
- `objc_runtime_metadata`
- `objc_type_encoding`
- `objc_property_attributes`
- `swift_demangler`
- `swift_metadata`
- `cpp_demangler`
- `cpp_rtti`
- `cpp_vtable`
- `dwarf`
- `dsym`
- `kdk`
- `compact_unwind`
- `architecture_semantics`
- `dataflow`
- `decompiler`
- `known_signature_database`
- `test_fixture`

### NameFact

`NameFact`:

- `id`
- `target`
- `name`
- `display_name`
- `demangled_name`
- `namespace`
- `owner`
- `language`
- `role`
- `provenance`
- `confidence`
- `precedence`
- `stale`
- `conflict_group`

Roles:

- `function`
- `global`
- `local_label`
- `objc_class`
- `objc_protocol`
- `objc_category`
- `objc_selector`
- `objc_method`
- `objc_ivar`
- `objc_property`
- `cpp_function`
- `cpp_method`
- `cpp_type`
- `swift_symbol`
- `kernel_symbol`
- `import`
- `export`
- `source_file`
- `frame`
- `address_alias`

Display precedence:

1. user label
2. exact debug/KDK/dSYM name
3. exact local symbol
4. exact export symbol
5. ObjC owner/selector method name
6. C++ demangled name
7. Swift demangled name
8. import binding name
9. analyzer-generated name
10. address fallback

All losing names remain queryable.

### TypeRecord

`TypeRecord`:

- `id`
- `kind`
- `name`
- `display_name`
- `size_bits`
- `alignment_bits`
- `language`
- `namespace`
- `owner`
- `is_forward_declaration`
- `is_incomplete`
- `single_line_display`
- `provenance`
- `layout_hash`
- `definition_revision`

Kinds:

- `void`
- `bool`
- `integer`
- `float`
- `pointer`
- `array`
- `struct`
- `union`
- `enum`
- `function`
- `function_pointer`
- `objc_id`
- `objc_class`
- `objc_selector`
- `objc_method`
- `objc_block`
- `objc_protocol`
- `cpp_class`
- `cpp_struct`
- `cpp_union`
- `cpp_enum`
- `cpp_vtable`
- `swift_placeholder`
- `kernel_struct`
- `kernel_opaque`
- `opaque`
- `unknown`

`IntegerType`:

- signedness
- width bits
- display base preference

`PointerType`:

- pointee type id
- pointer width bits
- address space
- pointer_authentication_policy
- relative pointer encoding
- nullability if known

`ArrayType`:

- element type id
- count
- stride bits

`StructType`:

- fields
- packed flag
- source layout mode
- bitfield policy
- relative pointer policy

`StructField`:

- field id
- name
- type id
- offset bits
- size bits
- bit offset
- bit width
- comment
- display format
- provenance
- field order
- conflict group

`UnionType`:

- fields
- active field hints

`EnumType`:

- base type id
- cases
- signedness
- is_options_bitmask

`EnumCase`:

- name
- value
- provenance
- order

`FunctionType`:

- return type id
- arguments
- variadic flag
- calling convention id
- language
- throws or error behavior if known
- ownership convention hints

`FunctionArgumentType`:

- index
- external name
- local name
- type id
- role
- storage hint
- default name provenance

### TypeFact

`TypeFact` assigns a type to a target.

Fields:

- `id`
- `target`
- `type_id`
- `role`
- `provenance`
- `confidence`
- `precedence`
- `stale`
- `conflict_group`
- `display_policy`

Roles:

- `function_return`
- `function_argument`
- `function_signature`
- `callsite_argument`
- `callsite_return`
- `stack_variable`
- `register_variable`
- `global_variable`
- `memory_address`
- `data_range`
- `operand`
- `struct_field`
- `enum_case`
- `objc_method_encoding`
- `objc_ivar_encoding`
- `objc_property_encoding`
- `objc_block_signature`
- `cpp_method_type`
- `cpp_vtable_slot`
- `swift_symbol_hint`
- `kernel_symbol_type`
- `panic_frame_function`

### FunctionSignature

`FunctionSignature`:

- `id`
- `function_id`
- `name`
- `return_type_id`
- `arguments`
- `calling_convention_id`
- `variadic`
- `source_language`
- `source_kind`
- `creation_reason`
- `propagation_state`
- `confidence`
- `provenance`
- `conflicts`

`creation_reason` should mirror Hopper-like categories:

- `unknown`
- `guessed_from_decompilation`
- `guessed_from_dataflow`
- `guessed_from_call`
- `known_database`
- `demangling`
- `objc_type_encoding`
- `dwarf`
- `dsym`
- `kdk`
- `user`
- `script`
- `mcp_tool`

`SignatureArgument`:

- `index`
- `name`
- `type_id`
- `storage`
- `role`
- `is_hidden`
- `is_self`
- `is_selector`
- `is_error_out`
- `is_block`
- `source`
- `confidence`

Important hidden arguments:

- Objective-C `self`
- Objective-C `_cmd`
- Swift context/self conventions when detectable
- C++ `this`
- block invocation context
- kernel trap or MIG hidden context when known

### CallingConvention

`CallingConventionRecord`:

- `id`
- `kind`
- `architecture`
- `name`
- `argument_registers`
- `return_registers`
- `callee_saved_registers`
- `caller_saved_registers`
- `stack_alignment`
- `red_zone_policy`
- `variadic_policy`
- `hidden_argument_policy`
- `provenance`

Kinds:

- `aapcs64`
- `aapcs64_vfp`
- `swift`
- `c`
- `objc_msg_send`
- `objc_msg_send_super`
- `objc_msg_send_stret`
- `objc_authenticated_selector_trampoline`
- `cpp_thiscall_aarch64`
- `kernel_aarch64`
- `mig_server`
- `block_invoke`
- `x86_64_sysv`
- `x86_64_apple`
- `unknown`

Special AArch64 Objective-C runtime conventions:

- model the shift where normal argument zero is not in `x0`
- preserve raw register mapping
- expose why an analyzer thinks `x0` is consumed by runtime dispatch
- avoid pretending this is normal C AAPCS

This is important because Hopper's public SDK explicitly exposes many
`AARCH64_arg0_xN` calling conventions for Objective-C runtime cases.

### StackFrame

`StackFrameRecord`:

- `id`
- `function_id`
- `bp_based`
- `frame_pointer_register`
- `stack_pointer_register`
- `frame_pointer_offset`
- `saved_registers_size`
- `locals_size`
- `purged_bytes`
- `stack_pointer_offsets`
- `variables`
- `provenance`

`StackPointerOffset`:

- `instruction_id`
- `address`
- `offset`
- `confidence`

### VariableRecord

`VariableRecord`:

- `id`
- `function_id`
- `kind`
- `name`
- `type_id`
- `storage`
- `live_range`
- `source`
- `confidence`
- `user_renamed`
- `stale`
- `conflict_group`

Kinds:

- `argument`
- `stack_local`
- `register_local`
- `temporary`
- `return_value`
- `global_alias`
- `unknown`

`VariableStorage`:

- register class and index
- stack displacement
- frame-base displacement
- register plus offset
- memory address
- SSA value id later

Local-variable facts from a decompiler/dataflow pass should be candidate facts
until the user confirms or edits them.

### RegisterAlias

`RegisterAliasRecord`:

- `id`
- `function_id`
- `register_class`
- `register_index`
- `architecture_register_name`
- `alias`
- `scope`
- `provenance`
- `is_user_authored`

Scopes:

- whole function
- basic block
- address range
- live range

### LocalLabel

`LocalLabelRecord`:

- `id`
- `function_id`
- `address`
- `name`
- `provenance`
- `conflict_group`

Local labels must be scoped to a function. A local label should not pollute
global search unless the user asks to search local labels.

### Structured Data Application

`StructuredDataApplication`:

- `id`
- `target_range`
- `type_id`
- `base_address`
- `field_rendering_mode`
- `source`
- `transaction_id`
- `invalidates`

This is the Damsel counterpart to Hopper's ability to define structures at
addresses and format operands as structured or enum values.

### Operand Format Override

`OperandFormatOverride`:

- `id`
- `instruction_id`
- `operand_index`
- `format`
- `relative_to`
- `type_id`
- `enum_type_id`
- `struct_field_path`
- `signedness`
- `leading_zeroes`
- `provenance`

Formats:

- default
- hex
- decimal
- octal
- character
- stack variable
- offset
- address
- float
- binary
- structured
- enum
- address difference

### DebugInfoImport

`DebugInfoImport`:

- `id`
- `profile_id`
- `source_artifact_id`
- `debug_artifact_id`
- `format`
- `uuid_match`
- `architecture_match`
- `os_build_match`
- `status`
- `imported_name_count`
- `imported_type_count`
- `imported_signature_count`
- `imported_line_count`
- `imported_variable_count`
- `skipped_count`
- `problems`

Formats:

- `dsym_dwarf`
- `kdk_dwarf`
- `macho_dwarf`
- `external_symbols`
- `compact_unwind`
- `source_map`

### CompileUnitRecord

`CompileUnitRecord`:

- `id`
- `debug_import_id`
- `producer`
- `language`
- `compile_directory`
- `source_files`
- `low_pc`
- `high_pc`
- `ranges`
- `line_program_id`
- `provenance`

Store paths carefully:

- preserve full path only inside local project storage
- expose basename and redacted path in diagnostics by default
- let users opt into full-path evidence exports

### SourceLineRecord

`SourceLineRecord`:

- `id`
- `compile_unit_id`
- `source_file_id`
- `line`
- `column`
- `address`
- `end_address`
- `is_statement`
- `is_prologue_end`
- `is_epilogue_begin`
- `provenance`

### InlineCallsiteRecord

`InlineCallsiteRecord`:

- `id`
- `function_id`
- `callsite_range`
- `abstract_origin_name`
- `source_line_id`
- `depth`
- `provenance`

Inline source information should improve navigation but should not rewrite the
function identity by default.

### SymbolicationProfile

`SymbolicationProfile`:

- `id`
- `name`
- `target_kind`
- `os_name`
- `os_version`
- `os_build`
- `architecture`
- `primary_uuid`
- `dyld_cache_uuid`
- `kernel_collection_uuid`
- `kdk_identity`
- `dsym_identities`
- `cache_local_symbols_identity`
- `status`
- `created_at`
- `last_validated_at`
- `active`
- `problems`

Statuses:

- `absent`
- `candidate`
- `exact_match`
- `partial_match`
- `uuid_mismatch`
- `architecture_mismatch`
- `os_build_mismatch`
- `stale`
- `import_failed`

Profile switch behavior:

- user facts remain active
- imported facts from inactive profiles become inactive by default
- views can show profile-specific overlays
- source-line panels show active profile status
- problem navigator explains mismatch

### TypeConflict

`TypeConflict`:

- `id`
- `target`
- `winning_fact`
- `candidate_facts`
- `reason`
- `severity`
- `resolution`
- `created_at_project_revision`

Reasons:

- user vs imported
- dSYM vs KDK
- ObjC encoding vs DWARF
- C++ demangler vs DWARF
- Swift demangler vs symbol table
- local symbol vs export
- field layout mismatch
- enum base type mismatch
- calling convention mismatch
- source artifact mismatch
- stale profile

Resolutions:

- unresolved
- user_selected
- hidden
- stale
- accepted_import
- accepted_inference
- split_targets

### TypeTransaction

`TypeTransaction`:

- `id`
- `name`
- `kind`
- `author_kind`
- `author_id`
- `started_at_project_revision`
- `ended_at_project_revision`
- `affected_targets`
- `created_facts`
- `removed_facts`
- `superseded_facts`
- `invalidated_indexes`
- `reanalysis_requests`
- `undo_available`

Transaction kinds:

- user_type_create
- user_type_edit
- user_signature_edit
- user_variable_rename
- user_register_alias
- user_structure_apply
- user_enum_apply
- import_dsym
- import_kdk
- import_objc_encodings
- import_cpp_demangle
- import_swift_demangle
- script_mutation
- mcp_mutation
- reanalysis

## Importer Pipeline

### Stage 0: Source And Profile Preconditions

Before importing types:

- source artifact identity must be known
- route resolver must be available
- architecture must be known
- object id allocator must be active
- project revision must be active
- profile target must be explicit
- mismatch policy must be explicit

Never run sidecar import into anonymous transient command output.

### ObjC Type Encoding Importer

Inputs:

- `ObjcMethodRecord.type_encoding`
- `ObjcIvarRecord.type_encoding`
- `ObjcPropertyRecord.attributes`
- selector and owner records
- implementation address
- architecture pointer width

Outputs:

- `TypeRecord` for primitive encodings
- pointer records
- ObjC object records
- selector records
- method signatures
- hidden `self` and `_cmd` arguments
- return type
- ivar type facts
- property type facts
- type conflict records for malformed encodings

Parser requirements:

- support primitive C encodings
- support object types
- support class and selector encodings
- support pointer encodings
- support array encodings where useful
- support struct name extraction
- support block pointer hints
- preserve unparsed remainder
- never fail the entire import because one encoding is malformed

Workbench behavior:

- method inspector shows raw encoding and decoded signature
- ivar inspector shows raw encoding and decoded type
- malformed encodings show a warning and raw string
- decoded facts can be promoted to user-edited signature/types

### ObjC Property Attribute Importer

Inputs:

- property attributes strings
- property owner
- selector/method linkage

Outputs:

- property type fact
- memory management attributes
- atomicity
- getter/setter names
- readonly/readwrite
- ivar backing hints

This importer should feed Objective-C header export later.

### C++ Demangle Importer

Inputs:

- Mach-O symbols
- export trie names
- local symbols
- cache local symbols
- KDK/dSYM names
- user-provided symbol sidecars later

Outputs:

- demangled name facts
- namespace facts
- function signature candidates
- method owner candidates
- constructor/destructor classification
- operator classification
- type name records
- C++ explorer indexes

Implementation notes:

- use a crate or library that can parse Itanium ABI names
- keep raw mangled and demangled strings
- separate pretty demangling from typed signature recovery
- record parse failures
- preserve overload information

Do not overclaim stripped C++ binaries. Demangling helps only where names
remain.

### C++ RTTI And Vtable Importer

Inputs:

- `__const`, `__data`, and related sections
- symbols where present
- typeinfo patterns
- vtable references
- constructor/destructor references
- IOKit naming and inheritance patterns

Outputs:

- C++ class candidate records
- typeinfo records
- vtable records
- vtable slot type candidates
- virtual call candidates
- inheritance candidates
- conflicts

This belongs to the runtime object model, but it consumes and creates type
records. Its facts should be weaker than exact DWARF/KDK facts unless matched.

### Swift Demangle Importer

Inputs:

- Swift mangled symbols
- Swift metadata comments/facts when available later
- cache symbol records

Outputs:

- demangled name facts
- module/type/function path
- Swift placeholder type records
- function signature hints where confidence is acceptable
- Swift metadata problem records when unsupported

Rules:

- Swift demangled names are not full Swift type recovery.
- Preserve raw mangled names.
- Treat crash/failure as a profile/import problem, not a project failure.
- Avoid claiming full Swift source-like types until metadata reconstruction is
  real.

### DWARF/dSYM Importer

Inputs:

- dSYM bundle
- DWARF object file
- UUID and architecture
- target source artifact identity

Outputs:

- compile units
- line tables
- function names
- address ranges
- inlined callsite records
- variable records
- type records
- function signatures
- source path records
- profile import receipt

Matching requirements:

- match Mach-O UUID where available
- match architecture slice
- mark mismatch explicitly
- support partial imports
- avoid path-only trust

Implementation sequence:

1. import compile units and line tables
2. import function ranges and names
3. import function signatures
4. import local variables
5. import struct/enum/union types
6. import inline records
7. add source panel integration

Use `gimli` or another structured DWARF parser rather than ad hoc string
parsing.

### KDK Profile Importer

Inputs:

- KDK path
- kernel collection or kernelcache identity
- dSYM files inside KDK
- OS build/version
- architecture

Outputs:

- symbolication profile
- kernel function names
- kernel type records
- source line records where available
- kernel struct/type facts
- panic frame symbolication support
- KDK mismatch problems

Rules:

- KDK import is profile-based, not global.
- KDK facts are exact only when UUID/build/architecture match.
- Wrong KDK facts must be visibly worse than absent KDK facts.
- Kernel collection member routes must survive profile switch.

### Compact Unwind And Frame Importer

Inputs:

- compact unwind
- frame-pointer analysis
- architecture semantics
- function ranges

Outputs:

- stack frame facts
- saved register facts
- possible no-return/call-effect facts later
- unwind reliability notes

This importer should feed stack-frame display and debugger frame mapping, but
not replace DWARF local variables.

### User Type Import/Export

Hopper exposes type import/export through the public SDK. Damsel should design
the same class of workflow.

Initial Damsel format:

- JSON or compact binary snapshot
- versioned schema
- project-independent type definitions
- optional project-specific applications
- provenance preserved
- conflict behavior declared

Commands:

- import type snapshot
- export type snapshot
- dry-run import
- show conflicts
- promote imported types to user facts

## Precedence And Conflict Policy

### Name Precedence

Recommended default:

1. user label
2. user-selected imported name
3. exact debug/KDK/dSYM name
4. cache local symbol
5. Mach-O local symbol
6. export symbol
7. ObjC method/class/category/protocol name
8. C++ demangled name
9. Swift demangled name
10. import/stub name
11. analyzer generated name
12. address fallback

### Type Precedence

Recommended default:

1. user-authored type/signature
2. user-selected imported type/signature
3. exact matched dSYM/KDK/DWARF type
4. ObjC type encoding
5. C++ demangled signature
6. C++ RTTI/vtable inference
7. Swift demangled hint
8. known signature database
9. ABI/calling-convention inference
10. dataflow/load-store inference
11. unknown/opaque

### Conflict Rendering

Conflicts should appear in:

- procedure inspector
- type inspector
- symbolication profile panel
- problems navigator
- CLI JSON
- evidence exports

Conflict text should include:

- winning source
- losing sources
- reason
- confidence
- how to override

### Staleness

Mark facts stale when:

- source artifact changed
- profile changed
- importer version changed
- route mapping changed
- function range changed
- user changed a dependent type
- debug sidecar disappeared

Stale facts can remain visible but should not drive default rendering unless no
fresh fact exists and the UI labels them as stale.

## Workbench UX

### Type Navigator

Left or sidebar navigator sections:

- all types
- user types
- imported debug types
- KDK types
- ObjC types
- C++ types
- enums
- structures
- functions/signatures
- conflicts
- recently edited

Search filters:

- name
- source
- kind
- owner
- profile
- conflict state
- stale state

### Type Detail Inspector

Shows:

- display declaration
- fields/cases/arguments
- size and alignment
- provenance stack
- uses in project
- conflicts
- edit history
- export/import status

Actions:

- rename type
- add field
- reorder field
- remove field
- set enum base type
- add enum case
- edit function pointer signature
- duplicate type
- mark as forward declaration
- mark incomplete
- export selected type

### Procedure Signature Panel

Shows:

- active signature
- all candidate signatures
- return type
- arguments
- hidden arguments
- calling convention
- creation reason
- propagation state
- conflicts
- source/provenance

Actions:

- edit return type
- add/remove/reorder argument
- rename argument
- set calling convention
- promote candidate
- reset to imported
- mark no reliable signature
- propagate signature to callsites

### Local Variables Panel

Shows:

- arguments
- stack locals
- register locals
- temporaries
- live ranges when available
- stack displacement
- current type
- candidate types
- source/confidence

Actions:

- rename local
- set type
- clear user type
- jump to first use
- show all uses
- mark as argument
- split live range later

### Register Alias Panel

Shows:

- register aliases for current function
- scope
- source
- uses

Actions:

- add alias
- edit alias
- clear alias
- promote analyzer alias to user alias

### Symbolication Profile Panel

Shows:

- active profile
- target artifact
- OS/build/architecture
- dSYM/KDK paths
- UUID/build match state
- imported facts
- conflicts
- stale status

Actions:

- attach dSYM
- attach KDK
- rescan profile
- switch profile
- detach profile
- export profile report

### Source Panel

Shows:

- source file path, redacted by default where appropriate
- line table
- current function line range
- inline frames
- compile unit
- producer/language
- profile source

Actions:

- copy source location
- reveal source if available and allowed
- show disassembly for line
- show all addresses for line

### Structured Data View

For a data address or operand:

- show active type application
- show field tree
- show raw bytes
- show interpreted values
- show field comments
- show conflicts

Actions:

- apply structure
- apply enum
- clear type application
- create type from data
- jump to type definition

### Command Palette

Commands:

- Create Type
- Manage Types
- Import Type Snapshot
- Export Type Snapshot
- Attach dSYM
- Attach KDK
- Decode ObjC Type Encoding
- Edit Function Signature
- Rename Local Variable
- Set Local Variable Type
- Set Calling Convention
- Apply Structure At Address
- Show Type Conflicts
- Show Symbolication Profile

## CLI And API Contracts

### CLI Commands

Add under project-aware commands:

- `damsel project types list`
- `damsel project types show <type-id-or-name>`
- `damsel project types uses <type-id-or-name>`
- `damsel project types import <path> --dry-run`
- `damsel project types export --output <path>`
- `damsel project signature show <function-route>`
- `damsel project signature set <function-route> --json <path>`
- `damsel project signature candidates <function-route>`
- `damsel project variables list <function-route>`
- `damsel project variables rename <variable-id> <name>`
- `damsel project variables set-type <variable-id> <type-id>`
- `damsel project symbols profiles`
- `damsel project symbols attach-dsym <path>`
- `damsel project symbols attach-kdk <path>`
- `damsel project symbols conflicts`
- `damsel project debug-info lines <function-route>`

Legacy point-query commands should remain stable. New commands should require
project context once project storage exists.

### API Endpoints

Recommended local UI API:

- `GET /api/projects/{project_id}/types`
- `GET /api/projects/{project_id}/types/{type_id}`
- `GET /api/projects/{project_id}/types/{type_id}/uses`
- `POST /api/projects/{project_id}/types/import-dry-run`
- `POST /api/projects/{project_id}/types/import`
- `GET /api/projects/{project_id}/types/export`
- `GET /api/projects/{project_id}/functions/{function_id}/signature`
- `GET /api/projects/{project_id}/functions/{function_id}/signature-candidates`
- `PATCH /api/projects/{project_id}/functions/{function_id}/signature`
- `GET /api/projects/{project_id}/functions/{function_id}/variables`
- `PATCH /api/projects/{project_id}/variables/{variable_id}`
- `GET /api/projects/{project_id}/symbolication/profiles`
- `POST /api/projects/{project_id}/symbolication/profiles/{profile_id}/attach-dsym`
- `POST /api/projects/{project_id}/symbolication/profiles/{profile_id}/attach-kdk`
- `GET /api/projects/{project_id}/debug-info/line-at-address`
- `GET /api/projects/{project_id}/type-conflicts`
- `POST /api/projects/{project_id}/transactions/type-edit`

Responses must include:

- object id
- display text
- source/provenance summary
- stale/conflict state
- route links
- revision number

### JSON Snapshot Requirements

Every snapshot should be deterministic:

- stable key ordering where practical
- sorted arrays by stable id/name
- explicit nulls for important absent fields
- no host-specific absolute paths unless the request opted in
- source path redaction by default
- schema version

## Storage And Indexing

Minimum tables once persistent project storage lands:

- `type_records`
- `type_fields`
- `enum_cases`
- `function_signatures`
- `signature_arguments`
- `calling_conventions`
- `type_facts`
- `name_facts`
- `variables`
- `stack_frames`
- `register_aliases`
- `local_labels`
- `debug_imports`
- `compile_units`
- `source_files`
- `source_lines`
- `inline_callsites`
- `symbolication_profiles`
- `type_conflicts`
- `type_transactions`

Indexes:

- type name
- type kind
- target object id
- source profile id
- function id
- address route
- conflict state
- stale state
- source kind
- provenance id

Performance constraints:

- type list first page under 100 ms for normal projects
- exact type lookup under 20 ms after warm index
- procedure signature fetch under 30 ms
- variable panel fetch under 50 ms
- profile summary under 50 ms
- conflict list first page under 100 ms
- dSYM/KDK imports run as cancellable background jobs
- import progress events include current phase and counts

## Integration With Existing Damsel

### `damsel-core`

Add pure model types first:

- IDs
- provenance records
- type records
- type facts
- name facts
- signatures
- calling conventions
- variables
- debug info
- profiles
- conflicts
- transactions

Do not tie them to one parser.

Existing structs to bridge:

- `Symbol`
- `ObjcMethodRecord`
- `ObjcIvarRecord`
- `ObjcPropertyRecord`
- `ObjcClassRecord`
- `ObjcProtocolRecord`
- `ObjcCategoryRecord`
- `DisassemblyAnalysis`
- `SymbolicationMatch`
- `CacheSymbolSource`
- `ProjectedImageProvenance`
- `CacheImageRecord`

### `damsel-macho`

Add importers incrementally:

- ObjC type encoding parser
- ObjC property attribute parser
- C++ demangle pass
- Swift demangle fact pass
- type fact generation from existing ObjC metadata
- signature candidates for ObjC methods
- type fact generation for known imports later

Keep existing CLI output stable unless a new flag or new command asks for
types.

### `damsel-cli`

Add project-aware commands only after project kernel exists.

Before project storage, limited commands can be fixture-backed:

- decode ObjC type encoding
- show signature candidate for ObjC method
- list type facts in transient analysis JSON

Do not fake persistent type editing in transient commands.

### `damsel-cli/ui`

The web UI can show read-only type facts once the local API exists, but the
native Workbench should own the polished Hopper-class editing experience.

### `damsel-app`

Workbench store split needed:

- `TypeStore`
- `SignatureStore`
- `SymbolicationProfileStore`
- `VariableStore`
- `ProblemStore`
- `TransactionStore`

Avoid stuffing type/signature state into one monolithic `WorkbenchStore`.

### Decompiler/IR

The IR should consume:

- active function signature
- all candidate signatures
- calling convention
- argument storage
- local variables
- register aliases
- type facts for operands/data ranges
- ObjC runtime method facts
- C++ vtable facts
- debug line and inline facts

The IR should output:

- dataflow-derived signature candidates
- stack variable candidates
- type conflicts
- user-reviewable inferred facts

## Implementation Phases

### S0: Public Evidence And Scope Lock

Deliverables:

- this spec
- source links in research docs
- explicit non-goals
- feature-to-subsystem mapping

Acceptance:

- docs name public Hopper object-model evidence
- docs distinguish source fact from inference
- docs connect type system to Workbench, CLI, MCP, decompiler, ObjC, C++, and
  kernel workflows

### S1: Model And Snapshot Foundation

Deliverables:

- `damsel-core` model types
- serde snapshots
- deterministic fixture builders
- precedence enums
- provenance enums
- conflict model

Tests:

- JSON schema snapshots
- precedence sorting tests
- conflict grouping tests
- stale fact tests

### S2: ObjC Encoding To Signature Facts

Deliverables:

- ObjC type encoding parser
- ObjC property attribute parser
- method signature candidates
- ivar/property type facts
- malformed encoding problem records

Tests:

- primitive encodings
- object encodings
- pointer encodings
- block pointer hints
- malformed encodings
- method hidden `self` and `_cmd`
- ivar type facts

### S3: C++ Demangling Signature Candidates

Deliverables:

- demangler integration
- C++ name fact records
- function/method signature candidates
- namespace/type owner records
- parse failure records

Tests:

- functions
- methods
- constructors/destructors
- operators
- overloaded names
- invalid mangled names
- stripped binary negative fixture

### S4: Swift Demangling Facts

Deliverables:

- Swift demangled name facts
- Swift placeholder type records
- failure/problem records
- CLI/API exposure

Tests:

- existing Swift sample
- exact mangled query
- demangled query
- ambiguous demangled query
- demangler unavailable behavior

### S5: Function Signature And Calling Convention Store

Deliverables:

- signature records
- argument records
- creation reasons
- calling convention records
- AArch64 ObjC runtime conventions
- signature candidate selection

Tests:

- creation reason precedence
- user signature preservation
- ObjC runtime calling convention mapping
- argument reorder
- hidden argument display

### S6: Stack Frame, Variables, Register Aliases, Local Labels

Deliverables:

- stack frame records
- variable records
- variable names/types
- register aliases
- local labels
- inspector payloads

Tests:

- frame-pointer fixture
- stack displacement fixture
- rename variable
- set variable type
- register alias scope
- local label scope

### S7: dSYM/DWARF Import MVP

Deliverables:

- dSYM attach command/API
- UUID/architecture matching
- compile units
- line tables
- function names/ranges
- signature import where feasible
- profile import receipt

Tests:

- UUID exact match
- UUID mismatch
- architecture mismatch
- line table snapshot
- function range snapshot
- source path redaction

### S8: KDK Profile MVP

Deliverables:

- KDK attach command/API
- OS build/UUID matching
- kernel symbol import
- kernel type import where feasible
- profile switch behavior
- panic-frame route integration

Tests:

- synthetic KDK profile
- mismatch profile
- profile switch keeps user labels
- panic frame symbolication snapshot

### S9: Type Editor And Structured Data Application

Deliverables:

- create/edit struct/union/enum
- enum base type
- field reorder
- method argument reorder
- function pointer signatures
- apply structure at address
- operand enum/structured formatting
- type import/export snapshot

Tests:

- create struct
- reorder fields
- enum base type
- function pointer signature
- apply structure
- import/export round trip
- undo/redo transaction

### S10: Workbench Type And Signature UI

Deliverables:

- type navigator
- type detail inspector
- procedure signature panel
- local variables panel
- profile panel
- source panel
- conflict/problems integration
- command palette commands

Tests:

- store unit tests
- paged type list
- signature candidate selection
- conflict display
- source path redaction
- profile mismatch UI

### S11: Decompiler/IR Integration

Deliverables:

- IR reads active signatures
- IR emits candidate signatures
- stack/local recovery writes candidate facts
- ObjC/C++ overlays consume type facts
- uncertainty reflected in summaries

Tests:

- signature-driven call summary
- ObjC method signature summary
- C++ candidate summary
- local variable candidate summary
- conflict avoids overconfident pseudocode

### S12: Automation And MCP Exposure

Deliverables:

- CLI JSON endpoints for types/signatures/profiles
- read-only MCP type tools
- audited mutation path later
- activity log for mutations

Tests:

- MCP schema snapshot
- route-backed response fixture
- permission denial fixture
- mutation audit fixture when mutations are enabled

## Validation Matrix

| Area | Fixture | Required Proof |
| --- | --- | --- |
| ObjC encodings | ObjC fixture with methods/ivars/properties | decoded signatures and raw encodings both visible |
| ObjC malformed encodings | synthetic malformed metadata | partial import plus problem record |
| C++ demangle | Itanium names fixture | demangled names and signature candidates |
| C++ stripped | stripped fixture | no fake signatures |
| Swift names | existing Swift sample | raw and demangled names, ambiguity handling |
| Stack variables | synthetic function with frame | stack slots and local names |
| Register aliases | function-scoped alias fixture | alias scoped to function only |
| dSYM match | synthetic or local dSYM | UUID match, line table, function ranges |
| dSYM mismatch | altered UUID fixture | mismatch problem, no trusted imports |
| KDK match | opt-in local KDK evidence | profile import receipt and symbols |
| KDK mismatch | synthetic profile | mismatch problem |
| Type editor | snapshot fixture | field reorder, enum base, export/import |
| Structured data | data section fixture | applied struct renders fields |
| Performance | large synthetic type list | paged query budget |
| Workbench | native store tests | no giant eager arrays |
| MCP | schema fixture | typed object responses |

## Security And Privacy

Type/symbolication data can reveal:

- private source paths
- user names in build paths
- internal project names
- private framework symbols
- local KDK paths
- local dSYM paths
- panic log contents

Default policy:

- redact absolute source paths in diagnostics and exported evidence
- keep full paths in local project storage only
- include opt-in flags for full-path exports
- never send paths through MCP without explicit permission
- log sidecar imports by artifact id and basename by default
- allow local UI reveal actions only after source access is granted

## Non-Goals

Do not implement in the first pass:

- full C/C++ header parser
- full Swift metadata reconstruction
- recompilable pseudocode
- arbitrary script execution
- arbitrary MCP mutation
- automatic trust of mismatched KDK/dSYM data
- in-place patching of shared caches or kernel collections
- whole-program type propagation without provenance
- exact C++ class recovery in stripped binaries

## Risks

### Risk: Type System Becomes A Pretty String Layer

Mitigation:

- store typed records and facts
- require provenance
- make renderers consume facts
- snapshot JSON, not only UI text

### Risk: Wrong KDK Or dSYM Looks Authoritative

Mitigation:

- match UUID/build/architecture
- mark mismatch loudly
- avoid importing exact facts from mismatch profiles by default

### Risk: C++ And Swift Recovery Is Overclaimed

Mitigation:

- separate demangled names from true types
- label signature candidates
- keep raw symbols visible
- require exact debug info for high-confidence types

### Risk: User Edits Are Lost

Mitigation:

- user facts have highest precedence
- every edit is a transaction
- profile switch preserves user facts
- reanalysis supersedes imported facts, not user facts

### Risk: Workbench Loads Giant Type Graphs

Mitigation:

- paged type lists
- lazy type detail fetch
- per-profile counts
- background imports
- query budgets
- UI virtualization

### Risk: Decompiler Consumes Weak Facts As Truth

Mitigation:

- IR reads confidence and source
- pseudocode labels uncertain assumptions
- candidate facts remain reviewable
- exact debug facts and user facts are distinguishable

## Completion Criteria

This subsystem is implementation-complete for the first Hopper-adjacent tier
only when:

- Damsel stores project-wide type/name/signature facts with provenance.
- ObjC type encodings produce method/ivar/property type facts.
- C++ demangling produces name and signature candidates.
- Swift demangling produces name facts and failure records.
- Function signatures have creation reasons and precedence.
- Calling conventions include Apple AArch64 and ObjC runtime cases.
- Stack locals, register aliases, and local labels are modelled.
- dSYM import has UUID matching, line tables, and function names.
- KDK import has profile matching and mismatch problems.
- Type conflicts are visible in CLI JSON and Workbench.
- Workbench can browse types, edit signatures, inspect local variables, and
  show symbolication profile state.
- Type import/export round trips deterministic snapshots.
- User edits survive reanalysis and profile switch.
- Decompiler/IR consumes type facts without hiding uncertainty.
- MCP/automation can query type/signature/profile objects through stable route
  APIs.

Until those are true, Damsel can have useful type features, but it should not
claim Hopper-class type/signature/symbolication parity.
