# Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec

Status: implementation-grade architecture spec.

Date of public-source pass: 2026-05-30.

Purpose: make Swift analysis a first-class Apple-systems lane in Damsel
instead of a small demangled-name appendix to the type system.

This is a companion to:

- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
- [Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec](semantic-editing-reanalysis-hints-implementation-spec.md)
- [Preferences, Keymaps, Layout Persistence, And Theming Implementation Spec](preferences-keymaps-layout-theming-implementation-spec.md)

## Why This Exists

The current pack mentions Swift in many places, but most references deliberately
stay at "Swift-adjacent" or "demangled name facts." That was the right
conservative framing for the earlier pass. It is no longer enough for a
Hopper-competitive Apple systems product.

Public Hopper evidence puts Swift on the product surface. Hopper's homepage
markets Objective-C and Swift awareness together. Its 2026 changelog still
contains Swift demangler fixes. Its 2025 v6.0 release called out improved Swift
strings and demangling. Its 2022 v5.8 release called out a first iteration on
loading Swift metadata. That sequence matters: a competitive Apple disassembler
does not stop at making mangled names pretty. It must turn Swift sections,
descriptors, conformances, metadata accessors, thunks, ObjC bridges, closures,
async continuations, and failure modes into routeable project facts.

Damsel currently has a useful seed:

- `damsel-cli/src/main.rs` supports `--swift-symbol` target resolution.
- `swift_symbol_candidates()` scans executable symbols for Swift mangling
  prefixes.
- `resolve_swift_symbol_target()` resolves exact mangled names first, then
  shells out to `swift-demangle` for demangled-name matching.
- CLI JSON tests exercise a checked-in `swift-sample` fixture.
- There is no Swift metadata module in `damsel-macho`.
- `BinaryImage` has an `ObjcMetadata` field, but no parallel `SwiftMetadata`.
- The Workbench browser plan has a Swift-adjacent browser placeholder, but not
  a complete parser, object model, store, API, or UX contract.

This spec turns that gap into a concrete subsystem.

## Product Positioning

Swift support should be part of Damsel's Apple niche, not a generic language
feature.

The product promise should be:

- excellent Swift symbol and metadata navigation for macOS and iOS binaries
- honest recovery labels when Swift metadata is partial, stripped, malformed,
  optimized, or only reachable through runtime accessors
- strong Swift-Objective-C bridge visibility
- dyld shared-cache and app-bundle awareness
- lightweight enough to run on large Swift-heavy apps and caches
- useful even before source-like Swift decompilation exists

The product promise should not be:

- full Swift source reconstruction
- perfect generic specialization recovery
- full Swift type-checker reimplementation
- runtime memory introspection without an explicit debugger session
- blind trust in demangler text as a real type model

The useful niche is "Swift runtime cartography": show the analyst what Swift
objects exist, how they are connected to code, how they bridge to Objective-C,
where metadata accessors and witness tables live, and what evidence supports
each claim.

## Public Source Set

Primary Hopper sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page and SDK listing: <https://www.hopperapp.com/download.html>

Swift and LLVM sources:

- Swift ABI mangling documentation:
  <https://github.com/swiftlang/swift/blob/main/docs/ABI/Mangling.rst>
- Swift ABI type metadata documentation:
  <https://github.com/swiftlang/swift/blob/main/docs/ABI/TypeMetadata.rst>
- Swift ABI metadata structures:
  <https://github.com/swiftlang/swift/blob/main/include/swift/ABI/Metadata.h>
- Swift metadata values:
  <https://github.com/swiftlang/swift/blob/main/include/swift/ABI/MetadataValues.h>
- LLVM Mach-O Swift section-name constants:
  <https://www.llvm.org/docs/doxygen/namespacellvm_1_1orc.html>

Public reverse-engineering context:

- Swift metadata reverse-engineering writeup:
  <https://knight.sc/reverse%20engineering/2019/07/17/swift-metadata.html>
- Hopper procedure-export workflow with Swift-heavy symbols:
  <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- `ipsw` Swift and Objective-C class-dump workflow:
  <https://instagit.com/blacktop/ipsw/how-to-perform-objc-and-swift-class-dumping-from-dyld-shared-cache/>

The implementation should prefer official Swift and LLVM sources for binary
format and ABI facts. Community sources are useful for workflow expectations
and reverse-engineering pressure, but not as normative layout contracts.

## Public Evidence Summary

### Hopper Homepage Evidence

Hopper publicly describes itself as a fast native macOS disassembler. It
explicitly says the Objective-C and Swift lane is specialized for macOS and iOS
development, with Swift mangled-name decoding and Objective-C selector/message
extraction.

Implementation implication:

- Damsel's Swift support belongs beside ObjC, C++/ObjC++, dyld cache, and XNU
  workflows.
- The browser and search surfaces should present Swift modules, types,
  functions, conformances, strings, and ObjC bridges as first-class objects.
- Demangled text should be visible everywhere a raw mangled name appears, but
  raw mangled identity must remain available and stable.

### Hopper Changelog Evidence

Relevant entries:

- 2026-05-19, Hopper 6.3.0: fixes a Swift demangler crash.
- 2025-09-10, Hopper 6.0.0: improved Swift strings and demangling.
- 2022-12-09, Hopper 5.8.3: fixes a crash when parsing Swift metadata on
  ciphered files.
- 2022-11-18, Hopper 5.8.0: first iteration on loading Swift metadata, with
  retrieved information available as comments in dedicated sections.
- 2022-11-07, Hopper 5.7.8: better parses Objective-C sections and fixes old
  dyld cache crash cases.

Implementation implication:

- Swift parsing is crash-prone and must be defensive.
- Ciphered, stripped, malformed, and old-cache inputs need problem records, not
  parser panics.
- Swift support is a maintained product surface, not a one-time demangle pass.
- The first Damsel metadata pass can expose comments and dedicated-section
  facts, but should quickly graduate to routeable objects and browser APIs.

### Swift ABI Mangling Evidence

Swift mangled names carry much more than display text. The ABI documentation
defines stable mangling prefixes, symbolic references, type metadata symbols,
metadata access functions, nominal type descriptors, protocol descriptors,
protocol conformance records, witness tables, ObjC thunks, async continuation
functions, and dynamic-dispatch thunks.

Implementation implication:

- The demangler result should be parsed into a structured `SwiftNameFact`.
- Mangling role classification should be separate from demangled display.
- Symbolic references in compiler-emitted metadata require careful bounds and
  input-trust rules.
- Name facts can seed metadata routes, but cannot prove complete type recovery
  alone.

### Swift Type Metadata Evidence

Swift type metadata has dynamic states and two-phase initialization. Metadata
can be abstract, layout-complete, non-transitively complete, or complete.
Protocol conformance descriptors connect protocols, conforming types, and
witness-table access.

Implementation implication:

- Static metadata parsing should distinguish "descriptor identity" from
  "runtime metadata state."
- Damsel can parse nominal descriptors and conformance descriptors from the
  file, but should not claim to know all runtime-instantiated generic metadata.
- The debugger can later enrich static facts with live metadata state, but
  static and dynamic evidence must stay separate.

### LLVM Mach-O Section Evidence

LLVM names Mach-O Swift sections including:

- `__TEXT,__swift5_entry`
- `__TEXT,__swift5_fieldmd`
- `__TEXT,__swift5_proto`
- `__TEXT,__swift5_protos`
- `__TEXT,__swift5_typeref`
- `__TEXT,__swift5_types`

Implementation implication:

- Damsel should start with section inventory and bounded decoders over the
  common Swift 5 sections.
- Section presence should drive project facts, source diagnostics, and Workbench
  browser availability.
- Section parsing must be lazy for large binaries and dyld cache projections.

## Current Damsel Baseline

### Current Strengths

Damsel already has:

- Mach-O slice selection
- section inventory
- symbols and imports
- relocations
- ObjC metadata parsing
- dyld shared-cache inventory and projected-image analysis
- disassembly analysis with blocks, edges, recovered values, imports,
  Objective-C references, jump tables, and cache links
- `--swift-symbol` CLI target resolution for executable Swift symbols
- a `swift-sample` fixture used by CLI JSON tests

Those are real advantages. Swift metadata should build on them instead of
forking a separate analysis path.

### Current Weaknesses

Damsel lacks:

- `damsel-macho::swift` module
- `SwiftMetadata` model
- Swift section classifier beyond generic `Section`
- Swift 5 relative-pointer helpers
- bounded typeref parser
- bounded field metadata parser
- nominal descriptor parser
- protocol descriptor parser
- protocol conformance parser
- witness table/accessor classifier
- metadata accessor classifier
- async continuation and thunk classifier
- Swift string/field-name relationship model
- Swift-ObjC bridge relation model
- Swift module/type/function browser API
- Swift metadata problem records
- Swift metadata fixture corpus beyond symbol-target lookup

The important baseline fact is that Swift is currently a symbol-resolution
feature, not a runtime analysis subsystem.

## Design Goals

### Goal 1: Durable Swift Object Model

Swift facts should be saved as project objects:

- module
- nominal type
- type descriptor
- field descriptor
- field record
- protocol descriptor
- protocol conformance
- witness table
- witness accessor
- metadata accessor
- value witness table reference
- type reference
- reflection string
- replacement record
- closure/capture record, where supported
- async continuation symbol
- Swift-ObjC bridge relation
- Swift problem

Every object needs:

- stable id
- source id
- image id
- address or section-range route
- raw bytes range if applicable
- parser version
- ABI profile
- confidence
- provenance
- invalidation key

### Goal 2: Demangling Is Evidence, Not Truth

The demangler should produce:

- raw mangled name
- normalized mangled payload
- demangled display
- parsed role, when safely inferred
- module candidate
- context/type candidate
- entity candidate
- generic/specialization markers, when safely inferred
- parser diagnostics
- demangler tool identity

It should not directly overwrite:

- symbol identity
- type database canonical name
- function signature truth
- decompiler type truth
- Swift metadata object identity

### Goal 3: Swift-ObjC Bridge Visibility

Swift on Apple platforms often crosses Objective-C:

- `@objc` exposed Swift classes and methods
- Swift classes with Objective-C class objects
- ObjC categories interacting with Swift types
- Swift methods compiled through ObjC thunks
- Objective-C selectors reachable from Swift code
- Swift bridging around Foundation and CoreFoundation values
- dynamic replacement and swizzling-like behavior
- Swift resilient class stubs and ObjC metadata update callbacks

Damsel should make these edges visible instead of making the analyst bounce
between a Swift symbol list and an ObjC browser.

### Goal 4: Honest Static/Dynamic Split

Static parsing can show:

- sections
- descriptors
- relative pointers
- type references
- fields
- conformances
- symbols
- references
- accessors and thunk candidates

It cannot prove:

- actual runtime generic metadata instances
- full dynamic dispatch target sets
- complete generic argument values
- complete actor/task runtime state
- live type metadata state

Live debugger integration can later add runtime metadata observations. Those
must be separate `RuntimeObservation` facts tied to a debug session.

### Goal 5: Large-Artifact Performance

Swift-heavy apps and dyld cache projections can be large. Swift support must
avoid:

- parsing every Swift section synchronously at open
- demangling every symbol on the main thread
- loading complete symbol lists into SwiftUI arrays
- scanning all dyld cache images for Swift metadata without scheduler budgets
- turning failed relative-pointer parsing into repeated hot-loop work

The first paint should show a Swift summary and queued jobs, not block on full
metadata reconstruction.

## Non-Goals

This spec does not require:

- perfect Swift source reconstruction
- a Swift type checker
- SIL reconstruction
- Swift AST reconstruction
- live Swift runtime memory inspection before debugger integration
- mutation of Swift metadata
- deobfuscation of protected/ciphered Swift metadata
- private Apple symbol acquisition
- remote device Swift introspection before remote debugger support exists

## Architecture Overview

```text
Mach-O source / dyld projection
        |
        v
section inventory + symbol table + ObjC metadata + relocations
        |
        v
swift section classifier
        |
        +--> SwiftNameImporter
        +--> SwiftSectionIndex
        +--> SwiftMetadataParser
        +--> SwiftConformanceParser
        +--> SwiftBridgeAnalyzer
        +--> SwiftProblemEmitter
        |
        v
project database
        |
        +--> Swift browser APIs
        +--> search/index routes
        +--> type/signature store
        +--> decompiler overlays
        +--> ObjC browser bridge edges
        +--> automation/MCP read APIs
        +--> evidence export
```

The parser should live near Mach-O parsing, but canonical facts should belong
to the project/fact database.

Suggested module split:

```text
damsel-macho/src/swift.rs
damsel-macho/src/swift/sections.rs
damsel-macho/src/swift/relative.rs
damsel-macho/src/swift/names.rs
damsel-macho/src/swift/metadata.rs
damsel-macho/src/swift/conformance.rs
damsel-macho/src/swift/bridge.rs

damsel-core/src/swift_model.rs
damsel-core/src/swift_routes.rs
damsel-core/src/swift_problems.rs

damsel-cli/src/swift_output.rs
damsel-cli/src/swift_commands.rs

damsel-app/Sources/DamselAppCore/Stores/SwiftBrowserStore.swift
damsel-app/Sources/DamselAppCore/Views/SwiftBrowserView.swift
damsel-app/Sources/DamselAppCore/Views/SwiftTypeDetailView.swift
```

Do not make Swift metadata a SwiftUI-only model. The Rust project database must
own canonical facts.

## Swift Source Kinds

The parser should record the source kind because identical-looking Swift
sections have different context depending on where they came from:

- standalone app executable
- app framework
- system framework
- dyld shared-cache image
- projected dyld shared-cache image
- driver-cache dylib
- kernel collection member, if Swift appears in future DriverKit-adjacent
  artifacts
- dSYM/debug sidecar carrying Swift reflection sections
- memory-loaded fixture

`SwiftSourceContext`:

```rust
pub struct SwiftSourceContext {
    pub source_id: SourceId,
    pub image_id: ImageId,
    pub source_kind: SourceKind,
    pub architecture: Architecture,
    pub platform: Option<Platform>,
    pub minimum_os: Option<AppleOsVersion>,
    pub swift_abi_profile: SwiftAbiProfile,
    pub is_dyld_cache_projection: bool,
    pub original_container_route: Option<RouteId>,
}
```

## ABI Profile

Do not use one undifferentiated `swift5` flag. Use a profile object:

```rust
pub struct SwiftAbiProfile {
    pub major_family: SwiftAbiFamily,
    pub stable_abi: bool,
    pub detected_from: Vec<SwiftAbiEvidence>,
    pub minimum_runtime_version: Option<String>,
    pub section_set: BTreeSet<SwiftSectionKind>,
    pub has_symbolic_references: bool,
    pub has_async_symbols: bool,
    pub has_dynamic_replacement: bool,
}
```

`SwiftAbiFamily`:

- `none`
- `pre_stable`
- `swift5_stable`
- `embedded_swift`
- `unknown`

Evidence:

- mangled prefix
- section names
- load command hints
- linked Swift runtime libraries
- symbols
- dyld cache image context
- user override

This matters because Damsel should tolerate older and newer Swift artifacts
without pretending every layout is identical.

## Section Classifier

### Section Kinds

Initial section kinds:

- `swift5_entry`
- `swift5_typeref`
- `swift5_reflstr`
- `swift5_fieldmd`
- `swift5_assocty`
- `swift5_proto`
- `swift5_protos`
- `swift5_types`
- `swift5_types2`
- `swift5_capture`
- `swift5_replace`
- `swift5_replac2`
- `swift5_acfuncs`
- `swift5_builtin`
- `swift5_mpenum`
- `swift_const_global`
- `swift_text_global`
- `unknown_swift`

The first implementation should fully support:

- `swift5_entry`
- `swift5_typeref`
- `swift5_reflstr`
- `swift5_fieldmd`
- `swift5_proto`
- `swift5_protos`
- `swift5_types`

Other sections should be inventoried and emitted as "recognized but not parsed"
until parser support is added.

### Section Record

```rust
pub struct SwiftSectionRecord {
    pub id: SwiftSectionId,
    pub source_id: SourceId,
    pub image_id: ImageId,
    pub segment: String,
    pub section: String,
    pub kind: SwiftSectionKind,
    pub vmaddr: u64,
    pub file_offset: u64,
    pub size: u64,
    pub entry_width: Option<u8>,
    pub parse_state: SwiftParseState,
    pub problem_count: u32,
}
```

### Parse States

- `not_started`
- `indexed`
- `parsed_partial`
- `parsed_complete`
- `unsupported`
- `malformed`
- `ciphered_or_unreadable`
- `stale`

The browser should show parse state. It should not hide unsupported Swift
sections.

## Relative Pointer Model

Swift metadata is full of relative references. Do not copy ObjC pointer helpers
blindly.

Required helper types:

```rust
pub struct RelativePointer {
    pub field_address: Address,
    pub raw_value: i32,
    pub target_address: Option<Address>,
    pub target_kind: RelativeTargetKind,
    pub is_indirect: bool,
    pub flags: u8,
    pub resolution: RelativeResolution,
}
```

`RelativeResolution`:

- `resolved`
- `target_out_of_range`
- `target_unmapped`
- `target_not_aligned`
- `indirect_pointer_unreadable`
- `null`
- `unsupported_encoding`

Rules:

- Every relative pointer read must be bounds checked.
- Every target must resolve through the project address-space model.
- A bad relative pointer becomes a problem record, not a parser panic.
- Indirect and direct references must preserve their original encoding.
- Pointer-authenticated targets must be labeled as authenticated candidates,
  not stripped silently.

## Name Model

### SwiftNameFact

```rust
pub struct SwiftNameFact {
    pub id: SwiftNameId,
    pub source_id: SourceId,
    pub image_id: ImageId,
    pub route: RouteId,
    pub raw_symbol: String,
    pub normalized_mangled: Option<String>,
    pub demangled: Option<String>,
    pub role: SwiftNameRole,
    pub module: Option<String>,
    pub context_path: Vec<String>,
    pub entity_name: Option<String>,
    pub signature_hint: Option<SignatureHintId>,
    pub demangler: DemanglerEvidence,
    pub confidence: Confidence,
    pub problems: Vec<ProblemId>,
}
```

### SwiftNameRole

- `function`
- `method`
- `initializer`
- `getter`
- `setter`
- `modify_coroutine`
- `read_coroutine`
- `async_continuation`
- `partial_application_forwarder`
- `objc_partial_application_forwarder`
- `objc_thunk`
- `swift_as_objc_thunk`
- `objc_as_swift_thunk`
- `dynamic_dispatch_thunk`
- `protocol_witness_thunk`
- `metadata`
- `full_metadata`
- `metadata_pattern`
- `metadata_accessor`
- `nominal_type_descriptor`
- `module_descriptor`
- `extension_descriptor`
- `protocol_descriptor`
- `protocol_conformance`
- `witness_table`
- `field_descriptor`
- `global_variable`
- `dynamic_replacement`
- `unknown`

Role classification should be deterministic and conservative. If the demangler
or parser cannot safely classify a name, it remains `unknown` with the
demangled display preserved.

## Demangler Integration

### Short-Term Tooling

Current CLI behavior shells out to `swift-demangle` or `xcrun --find
swift-demangle`. Keep that as the first implementation path, but move it behind
a project service:

```rust
pub trait SwiftDemangler {
    fn identity(&self) -> DemanglerIdentity;
    fn demangle_batch(&self, names: &[String]) -> Result<Vec<DemangleResult>>;
}
```

`DemanglerIdentity`:

- tool path
- version string if available
- invocation mode
- timeout policy
- max batch size

### Medium-Term Parser

Add a Rust wrapper or parser when the project needs:

- deterministic CI behavior without Xcode tools
- structured demangle tree
- symbolic-reference policy
- role classification without fragile text parsing
- sandboxed batch performance

Until then, the shell-out path must produce explicit degraded capability
records when unavailable.

### Failure Modes

Record these:

- demangler unavailable
- demangler timeout
- demangler nonzero exit
- invalid UTF-8 in output
- output line missing mapping marker
- duplicate mapping
- symbolic reference refused
- demangled text too large
- batch truncated

Do not silently omit failed names. A Swift browser with "demangler unavailable"
is more trustworthy than an empty Swift browser.

## Metadata Object Model

### SwiftModuleRecord

```rust
pub struct SwiftModuleRecord {
    pub id: SwiftModuleId,
    pub source_id: SourceId,
    pub image_ids: BTreeSet<ImageId>,
    pub name: String,
    pub name_sources: Vec<SwiftNameSource>,
    pub type_count: u32,
    pub function_count: u32,
    pub conformance_count: u32,
    pub objc_bridge_count: u32,
    pub confidence: Confidence,
}
```

Module records may be inferred from demangled names before metadata parsing is
complete. Keep `name_sources` visible.

### SwiftNominalTypeRecord

```rust
pub struct SwiftNominalTypeRecord {
    pub id: SwiftTypeId,
    pub module_id: Option<SwiftModuleId>,
    pub source_id: SourceId,
    pub image_id: ImageId,
    pub descriptor_route: Option<RouteId>,
    pub metadata_route: Option<RouteId>,
    pub name: SwiftDisplayName,
    pub kind: SwiftNominalKind,
    pub generic_signature: Option<SwiftGenericSignatureId>,
    pub superclass: Option<SwiftTypeRefId>,
    pub fields: Vec<SwiftFieldId>,
    pub methods: Vec<SwiftMethodId>,
    pub conformances: Vec<SwiftConformanceId>,
    pub objc_bridge: Option<ObjcBridgeId>,
    pub layout_evidence: LayoutEvidence,
    pub confidence: Confidence,
    pub problems: Vec<ProblemId>,
}
```

`SwiftNominalKind`:

- `class`
- `struct`
- `enum`
- `protocol`
- `extension`
- `opaque_type`
- `foreign_class`
- `unknown`

### SwiftFieldRecord

```rust
pub struct SwiftFieldRecord {
    pub id: SwiftFieldId,
    pub owner_type_id: SwiftTypeId,
    pub name: Option<String>,
    pub mangled_type_ref: Option<SwiftTypeRefId>,
    pub demangled_type_hint: Option<String>,
    pub flags: SwiftFieldFlags,
    pub order: u32,
    pub route: RouteId,
    pub confidence: Confidence,
}
```

Fields from `__swift5_fieldmd` are field metadata facts. They are not
guaranteed to prove full runtime layout without layout evidence and resilience
rules.

### SwiftTypeRefRecord

```rust
pub struct SwiftTypeRefRecord {
    pub id: SwiftTypeRefId,
    pub source_id: SourceId,
    pub image_id: ImageId,
    pub route: RouteId,
    pub raw_bytes_hash: String,
    pub mangled_payload: Option<String>,
    pub demangled: Option<String>,
    pub symbolic_references: Vec<SwiftSymbolicReference>,
    pub resolved_type_id: Option<SwiftTypeId>,
    pub confidence: Confidence,
}
```

Type references often connect many other records. They need their own identity.

### SwiftProtocolRecord

```rust
pub struct SwiftProtocolRecord {
    pub id: SwiftProtocolId,
    pub module_id: Option<SwiftModuleId>,
    pub name: SwiftDisplayName,
    pub descriptor_route: Option<RouteId>,
    pub requirements: Vec<SwiftProtocolRequirementId>,
    pub inherited_protocols: Vec<SwiftProtocolId>,
    pub objc_protocol_id: Option<ObjcProtocolId>,
    pub confidence: Confidence,
}
```

### SwiftProtocolConformanceRecord

```rust
pub struct SwiftProtocolConformanceRecord {
    pub id: SwiftConformanceId,
    pub source_id: SourceId,
    pub image_id: ImageId,
    pub route: RouteId,
    pub type_ref: SwiftTypeRefId,
    pub protocol_ref: SwiftProtocolRef,
    pub witness: SwiftWitnessKind,
    pub conditional_requirements: Vec<SwiftConformanceRequirement>,
    pub objc_bridge: Option<ObjcBridgeId>,
    pub confidence: Confidence,
    pub problems: Vec<ProblemId>,
}
```

`SwiftWitnessKind`:

- `table`
- `unconditional_accessor`
- `conditional_accessor`
- `reserved`
- `unresolved`

### SwiftMetadataAccessorRecord

```rust
pub struct SwiftMetadataAccessorRecord {
    pub id: SwiftMetadataAccessorId,
    pub function_id: Option<FunctionId>,
    pub route: RouteId,
    pub owner_type_id: Option<SwiftTypeId>,
    pub accessor_kind: SwiftAccessorKind,
    pub request_state_hint: Option<SwiftMetadataState>,
    pub return_state_hint: Option<SwiftMetadataState>,
    pub calls_runtime: Vec<RuntimeCallId>,
    pub confidence: Confidence,
}
```

Accessor classification should use:

- mangled role
- symbol names
- call graph to Swift runtime functions
- references to descriptors
- entrypoint usage

### SwiftRuntimeCallRecord

```rust
pub struct SwiftRuntimeCallRecord {
    pub id: RuntimeCallId,
    pub function_id: FunctionId,
    pub instruction_route: RouteId,
    pub callee_symbol: String,
    pub runtime_family: SwiftRuntimeFunctionFamily,
    pub argument_evidence: Vec<ValueFactId>,
    pub related_type_refs: Vec<SwiftTypeRefId>,
    pub confidence: Confidence,
}
```

Runtime function families:

- metadata state check
- type metadata access
- generic metadata instantiation
- protocol conformance lookup
- dynamic cast
- value witness call
- allocation
- reference count
- ObjC bridge
- actor/task/concurrency
- unknown Swift runtime

## Swift-ObjC Bridge Model

### Bridge Relations

```rust
pub struct ObjcSwiftBridgeRecord {
    pub id: ObjcBridgeId,
    pub source_id: SourceId,
    pub image_id: ImageId,
    pub swift_object: SwiftObjectRef,
    pub objc_object: ObjcObjectRef,
    pub relation: ObjcSwiftBridgeKind,
    pub route: RouteId,
    pub evidence: Vec<EvidenceId>,
    pub confidence: Confidence,
    pub problems: Vec<ProblemId>,
}
```

`ObjcSwiftBridgeKind`:

- `swift_class_has_objc_class`
- `swift_method_exposed_as_objc_selector`
- `objc_selector_thunk_to_swift`
- `swift_method_thunk_to_objc`
- `objc_protocol_used_by_swift_conformance`
- `swift_protocol_objc_compatible`
- `foundation_bridge_function`
- `objc_resilient_class_stub`
- `objc_metadata_update_callback`
- `dynamic_replacement_affects_objc_entry`
- `unknown_bridge`

### Bridge Evidence

Evidence sources:

- Swift mangled name role
- ObjC class list
- ObjC selector references
- method symbols
- metadata update callback symbols
- thunk function calls
- `objc_msgSend` analysis
- selector strings
- type descriptors
- protocol conformance descriptors
- imports from Swift runtime and Objective-C runtime

### Bridge Browser

The Swift browser and ObjC browser should both expose bridge edges.

Swift type detail should show:

- ObjC class name if known
- exposed selectors
- ObjC thunks
- Objective-C protocols
- message-send sites involving the Swift type
- dynamic replacement edges

ObjC class detail should show:

- Swift nominal type candidate
- Swift module
- Swift methods with ObjC selectors
- Swift-only methods not exposed to ObjC
- bridge confidence
- unresolved bridge problems

## Swift Browser UX

### Swift Home

Header:

- Swift status
- ABI profile
- modules
- nominal types
- protocols
- conformances
- metadata accessors
- runtime calls
- ObjC bridges
- parse problems
- demangler status

Cards:

- Modules
- Types
- Protocols
- Conformances
- Functions
- Metadata Accessors
- Runtime Calls
- ObjC Bridges
- Dynamic Replacements
- Swift Strings And Fields
- Problems

### Module List

Columns:

- module
- source image
- type count
- function count
- conformance count
- ObjC bridges
- confidence
- problem count

Interactions:

- double-click opens module detail
- filter by image/cache member
- filter by app/system module
- filter by Swift-only vs ObjC-bridged
- command palette route

### Type List

Columns:

- name
- kind
- module
- source
- field count
- method count
- protocol conformance count
- ObjC bridge
- confidence
- problems

Badges:

- descriptor parsed
- field metadata parsed
- runtime metadata symbol only
- demangled only
- ObjC bridge
- partial
- malformed

### Type Detail

Sections:

- identity
- raw symbols
- descriptor bytes
- fields
- methods
- conformances
- metadata accessors
- runtime calls
- ObjC bridges
- references
- decompiler overlays
- problems
- evidence

Commands:

- reveal descriptor in hex
- reveal metadata accessor
- reveal related ObjC class
- copy mangled name
- copy demangled name
- export type evidence
- create user note
- mark demangle display override

### Protocol Detail

Sections:

- protocol identity
- requirements
- conforming types
- witness accessors
- ObjC protocol bridge
- generic/associated type hints
- problems

### Conformance Detail

Sections:

- conforming type
- protocol
- witness kind
- witness route
- conditional requirements
- related thunks
- evidence
- problems

### Runtime Call Detail

Sections:

- caller function
- instruction
- Swift runtime callee
- inferred role
- argument value facts
- related type refs
- confidence
- debugger enrichment placeholder

### Problems Pane

Problem examples:

- Swift section recognized but unsupported
- relative pointer target out of range
- indirect reference unreadable
- symbolic reference refused
- typeref demangle failed
- field descriptor truncated
- conformance descriptor truncated
- protocol reference unresolved
- witness accessor outside executable range
- ObjC bridge ambiguous
- demangler unavailable
- metadata record version unknown
- ciphered or unreadable source

Each problem needs:

- route
- severity
- evidence
- suggested next action
- whether reanalysis is possible

## API Contracts

### Summary

```text
GET /api/projects/{project_id}/swift/summary
```

Response:

```json
{
  "project_id": "proj_1",
  "swift": {
    "status": "partial",
    "abi_profile": "swift5_stable",
    "demangler_status": "available",
    "modules": 12,
    "types": 230,
    "protocols": 41,
    "conformances": 812,
    "metadata_accessors": 358,
    "runtime_calls": 1204,
    "objc_bridges": 74,
    "problems": 19,
    "parse_state": "parsed_partial"
  }
}
```

### Sections

```text
GET /api/projects/{project_id}/swift/sections
GET /api/projects/{project_id}/swift/sections/{section_id}
```

### Modules

```text
GET /api/projects/{project_id}/swift/modules
GET /api/projects/{project_id}/swift/modules/{module_id}
```

### Types

```text
GET /api/projects/{project_id}/swift/types
GET /api/projects/{project_id}/swift/types/{type_id}
GET /api/projects/{project_id}/swift/types/{type_id}/fields
GET /api/projects/{project_id}/swift/types/{type_id}/methods
GET /api/projects/{project_id}/swift/types/{type_id}/conformances
```

### Protocols And Conformances

```text
GET /api/projects/{project_id}/swift/protocols
GET /api/projects/{project_id}/swift/protocols/{protocol_id}
GET /api/projects/{project_id}/swift/conformances
GET /api/projects/{project_id}/swift/conformances/{conformance_id}
```

### Names And Demangling

```text
GET /api/projects/{project_id}/swift/names
POST /api/projects/{project_id}/swift/demangle
POST /api/projects/{project_id}/swift/demangle/batch
```

Batch demangle should be job-backed for large inputs.

### Metadata Accessors And Runtime Calls

```text
GET /api/projects/{project_id}/swift/metadata-accessors
GET /api/projects/{project_id}/swift/metadata-accessors/{accessor_id}
GET /api/projects/{project_id}/swift/runtime-calls
GET /api/projects/{project_id}/swift/runtime-calls/{call_id}
```

### Bridge Relations

```text
GET /api/projects/{project_id}/swift/objc-bridges
GET /api/projects/{project_id}/swift/objc-bridges/{bridge_id}
GET /api/projects/{project_id}/objc/classes/{class_id}/swift-bridges
GET /api/projects/{project_id}/objc/selectors/{selector_id}/swift-bridges
```

### Problems

```text
GET /api/projects/{project_id}/swift/problems
GET /api/projects/{project_id}/swift/problems/{problem_id}
POST /api/projects/{project_id}/swift/problems/{problem_id}/recheck
```

## CLI Contracts

### Summary

```text
damsel swift summary <path>
damsel cache swift summary <cache-root> --image /System/Library/...
```

### Sections

```text
damsel swift sections <path> --json
damsel swift section <path> --section __swift5_fieldmd --json
```

### Names

```text
damsel swift names <path> --kind metadata-accessor --json
damsel swift demangle '_$s17DamselSwiftSample9publicAddyS2iF'
damsel swift demangle-batch names.txt --json
```

### Types

```text
damsel swift types <path> --module DamselSwiftSample --json
damsel swift type <path> --name DamselSwiftSample.Widget --json
```

### Conformances

```text
damsel swift conformances <path> --protocol Swift.Equatable --json
damsel swift conformance <path> --id swift_conf_123 --json
```

### Bridges

```text
damsel swift objc-bridges <path> --json
damsel objc swift-bridges <path> --class MySwiftClass --json
```

### Problem Output

All Swift CLI commands should include a stable `problems` array in JSON:

```json
{
  "code": "swift_relative_pointer_out_of_range",
  "severity": "warning",
  "route": "swift-section:...",
  "message": "Swift field metadata contains a relative pointer outside mapped sections.",
  "recoverable": false
}
```

## Project Database Tables

Suggested tables:

- `swift_sections`
- `swift_names`
- `swift_modules`
- `swift_type_refs`
- `swift_nominal_types`
- `swift_fields`
- `swift_protocols`
- `swift_protocol_requirements`
- `swift_conformances`
- `swift_witnesses`
- `swift_metadata_accessors`
- `swift_runtime_calls`
- `swift_objc_bridges`
- `swift_dynamic_replacements`
- `swift_parse_jobs`
- `swift_problems`

Indexes:

- module name
- demangled name trigram
- mangled name exact
- route id
- source id and image id
- type kind
- protocol id
- conformance pair
- ObjC object id
- problem severity

Important uniqueness rules:

- raw symbol plus image id is not enough for type identity
- descriptor route plus source id is stronger than demangled name
- module names can collide across images
- Swift and ObjC names can collide and should not share ids

## Job Model

Swift analysis should be split into jobs:

1. Swift section inventory
2. Swift symbol role classification
3. batch demangle
4. typeref parse
5. reflection string index
6. field metadata parse
7. nominal descriptor parse
8. protocol descriptor parse
9. conformance parse
10. metadata accessor classification
11. runtime call classification
12. ObjC bridge linking
13. browser index build

Each job should be:

- cancellable
- resumable
- source-versioned
- bounded by time and byte budgets
- able to emit partial results
- visible in Workbench activity

Default open policy:

- run section inventory and symbol role classification immediately
- queue demangle in batches
- parse small metadata sections opportunistically
- defer deep conformance and bridge linking until the Swift browser opens,
  search needs it, or a project profile requests it

## Performance Budgets

Small standalone app:

- Swift summary visible under 300 ms after project open
- first module/type page under 500 ms after index exists
- demangle first 500 visible names under 750 ms
- type detail under 250 ms after parse

Large Swift app:

- summary visible under 750 ms
- section inventory under 1 s
- first type page under 1 s
- background demangle cancellable within 150 ms
- no full symbol list materialized in SwiftUI

Dyld shared-cache projection:

- Swift section inventory per selected image under 1 s
- no cache-wide Swift parse at open unless explicitly requested
- cache-wide Swift search is job-backed and paged
- memory growth bounded by active pages and parser caches

## Workbench Store Plan

New Swift stores:

```swift
@MainActor
final class SwiftBrowserStore: ObservableObject {
  @Published var summary: SwiftSummary?
  @Published var modulesPage: PagedResult<SwiftModuleRow>?
  @Published var typesPage: PagedResult<SwiftTypeRow>?
  @Published var selectedRoute: Route?
  @Published var problems: [SwiftProblemRow] = []
}
```

Supporting stores:

- `SwiftTypeDetailStore`
- `SwiftProtocolDetailStore`
- `SwiftConformanceDetailStore`
- `SwiftBridgeStore`
- `SwiftDemanglerStore`

Rules:

- SwiftUI stores own visible pages and selections only.
- Rust owns canonical metadata.
- Lists are paged.
- Detail views fetch by object id.
- Route selection is shared with assembly, graph, hex, ObjC browser, and
  decompiler.
- Problems are shown inline, not hidden in logs.

## Inspector Integration

Swift-aware inspector modules:

- Swift symbol module
- Swift descriptor module
- Swift type-ref module
- Swift field module
- Swift protocol conformance module
- Swift runtime call module
- Swift-ObjC bridge module
- Swift problem module

Example instruction inspector additions:

- demangled callee name
- runtime call family
- metadata accessor owner
- related type reference
- protocol conformance evidence
- ObjC bridge edge
- confidence

Example hex inspector additions:

- relative pointer field
- decoded target
- section-local record kind
- raw bytes
- parse diagnostics

## Search Integration

Search scopes:

- Swift modules
- Swift types
- Swift fields
- Swift protocols
- Swift conformances
- Swift functions
- Swift runtime calls
- Swift ObjC bridges
- Swift problems
- raw mangled names
- demangled names

Ranking:

1. exact raw mangled
2. exact demangled
3. exact module/type path
4. exact field/protocol name
5. prefix module/type path
6. fuzzy demangled
7. problem text

Search results must include route, source image, confidence, and whether the
result is demangled-only or metadata-backed.

## Decompiler And IR Integration

Early decompiler overlays should use Swift facts conservatively:

- show demangled function labels
- label metadata accessors
- label witness thunks
- label async continuations
- annotate Swift runtime calls
- show Swift type candidates near metadata operations
- show ObjC bridge selectors when evidence exists
- emit uncertainty notes for generic metadata and dynamic dispatch

Do not:

- generate Swift-like source syntax as a default
- claim exact generic type values from static descriptors alone
- replace low-level calls with high-level Swift semantics without evidence
- hide retain/release/bridge calls if they are relevant to control or dataflow

## Debugger Integration Later

When debugger support exists, Swift runtime metadata should accept dynamic
observations:

- live metadata pointer
- metadata state
- dynamic type name
- generic argument metadata pointers
- protocol conformance lookup result
- witness table pointer
- actor/task runtime observations
- live ObjC class object for Swift class

But dynamic observations must include:

- debug session id
- process id
- module slide
- memory protection state
- timestamp
- whether the read was partial
- permission status

Static facts and dynamic observations should be mergeable in the UI but not
stored as the same kind of evidence.

## Automation And MCP Integration

Read-only tools:

- list Swift modules
- list Swift types
- get Swift type detail
- list Swift conformances
- explain Swift-ObjC bridge
- list Swift problems
- demangle names
- export Swift evidence summary

Mutation tools should wait until semantic editing is stable:

- annotate Swift type
- attach user bridge note
- set demangle display override
- suppress Swift problem

Every mutation must be:

- permissioned
- audited
- undoable
- tied to route/object ids
- included in diagnostics redaction policy

## Diagnostics And Redaction

Swift diagnostics can leak:

- app module names
- type names
- field names
- source-derived debug strings
- dSYM source paths
- user comments
- raw strings from binaries
- local file paths

Diagnostics presets:

- `minimal`: counts, statuses, problem codes, no names
- `developer`: module/type names, redacted paths, no raw strings
- `full-local`: raw names and selected bytes, never uploaded automatically

Evidence export should include a preview of what Swift names and field strings
will be included.

## Validation Matrix

### Fixture Tiers

Synthetic fixtures:

- Swift symbol-only fixture
- Swift field metadata fixture
- Swift protocol conformance fixture
- Swift ObjC bridge fixture
- malformed typeref fixture
- truncated field metadata fixture
- invalid symbolic-reference fixture
- large demangle batch fixture

Local opt-in fixtures:

- Swift app executable
- Swift framework
- SwiftUI app
- app with mixed ObjC/Swift
- dyld shared-cache image with Swift metadata
- dSYM with Swift reflection sections, where available

### Unit Tests

- section classifier recognizes Swift sections
- unknown Swift sections become inventory records
- relative pointer helper bounds checks every target
- typeref parser rejects symbolic references from untrusted inputs
- demangler unavailable produces problem record
- malformed demangler output produces problem record
- field descriptor parser is bounded
- conformance descriptor parser is bounded
- protocol reference resolution handles ObjC class references
- role classifier recognizes metadata accessors, witness tables, thunks, and
  async continuations from mangled role evidence

### Integration Tests

- `damsel swift summary fixtures/bin/swift-sample --json`
- `damsel swift names fixtures/bin/swift-sample --json`
- `damsel swift types fixtures/bin/swift-metadata-sample --json`
- `damsel swift objc-bridges fixtures/bin/swift-objc-bridge --json`
- Workbench Swift home loads summary
- Workbench type detail route reveals descriptor in hex
- search finds raw and demangled names
- ObjC browser shows Swift bridge edge
- decompiler overlay shows Swift runtime call label

### Snapshot Tests

Snapshots should include:

- Swift summary JSON
- Swift sections JSON
- Swift names JSON
- Swift type row JSON
- Swift field row JSON
- Swift conformance JSON
- Swift bridge JSON
- Swift problem JSON

### Performance Tests

- demangle 10,000 names with batching and timeout
- parse large `__swift5_typeref` section without allocation spike
- parse malformed section with bounded work
- first Swift browser page latency
- cancel deep Swift parse job
- no full Swift list materialized in SwiftUI

## Migration Slices

### S1: Model And Inventory

Deliverables:

- `SwiftMetadata` root model
- `SwiftSectionRecord`
- section classifier
- JSON output for Swift sections
- problem records for unsupported sections

Validation:

- section classifier tests
- `swift-sample` summary snapshot
- unknown-section fixture

### S2: Demangler Service

Deliverables:

- `SwiftDemangler` trait
- current shell-out implementation moved behind service
- demangler identity and failure records
- batch demangle API
- cached demangled name facts

Validation:

- exact mangled lookup still works
- demangler unavailable test
- ambiguous demangled query test
- timeout fixture

### S3: Name Role Classification

Deliverables:

- `SwiftNameFact`
- `SwiftNameRole`
- module/type/function extraction from demangled names where safe
- role classifier for metadata, accessors, descriptors, thunks, witnesses,
  async continuations, and dynamic replacement candidates

Validation:

- symbol role corpus
- raw and demangled search snapshots
- role downgrade test when evidence is weak

### S4: Type References And Reflection Strings

Deliverables:

- `SwiftTypeRefRecord`
- reflection string index
- symbolic-reference policy
- typeref parser
- route to typeref bytes

Validation:

- typeref fixture
- invalid symbolic reference fixture
- malformed type-ref bounded parse test

### S5: Field Metadata

Deliverables:

- `SwiftNominalTypeRecord`
- `SwiftFieldRecord`
- field metadata parser
- field/type browser rows
- problem records for truncated descriptors

Validation:

- field metadata fixture
- field order snapshot
- incomplete descriptor test

### S6: Protocols And Conformances

Deliverables:

- `SwiftProtocolRecord`
- `SwiftProtocolConformanceRecord`
- protocol descriptor parser
- conformance parser
- witness kind classifier

Validation:

- protocol fixture
- conformance fixture
- witness accessor fixture
- ObjC class reference conformance fixture

### S7: Metadata Accessors And Runtime Calls

Deliverables:

- metadata accessor classifier
- Swift runtime call classifier
- relation to function and IR facts
- instruction inspector module

Validation:

- metadata accessor fixture
- runtime call fixture
- decompiler overlay snapshot

### S8: Swift-ObjC Bridge

Deliverables:

- `ObjcSwiftBridgeRecord`
- Swift-to-ObjC class relation pass
- selector thunk relation pass
- protocol bridge relation pass
- Swift browser and ObjC browser bridge views

Validation:

- mixed ObjC/Swift fixture
- ambiguous bridge problem fixture
- browser route sync test

### S9: Workbench Browser

Deliverables:

- Swift Home
- module list
- type list
- type detail
- protocol detail
- conformance detail
- runtime calls page
- ObjC bridges page
- problems page

Validation:

- Swift Home UI smoke
- paged list tests
- route reveal tests
- keyboard navigation tests
- no whole-list SwiftUI allocation test

### S10: Cache And Large-Artifact Integration

Deliverables:

- dyld cache projected-image Swift metadata support
- per-image parse jobs
- cache-wide Swift search job
- source route preservation
- performance samples

Validation:

- projected image fixture
- optional real cache smoke
- route from cache Swift type back to original container
- cancellation and paging benchmarks

## Risk Register

### Risk: Demangling Is Mistaken For Type Recovery

Mitigation:

- keep `SwiftNameFact` separate from `SwiftNominalTypeRecord`
- show demangled-only badges
- require descriptor evidence for metadata-backed types
- keep raw mangled names visible

### Risk: Swift Metadata Parser Crashes On Malformed Inputs

Mitigation:

- central relative-pointer helper
- byte budget per parser
- all parser errors become problem records
- malformed fixtures added with every parser slice

### Risk: SwiftUI Main Actor Gets Large Lists

Mitigation:

- paged APIs
- query stores
- no global arrays in Swift stores
- performance tests on large synthetic name sets

### Risk: ObjC Bridge Edges Overclaim

Mitigation:

- confidence buckets
- evidence list on every bridge record
- ambiguous bridge problem records
- no automatic selector binding without proof

### Risk: ABI Drift

Mitigation:

- ABI profile object
- unknown section inventory
- parser version recorded on facts
- forward-compatible metadata kind handling
- source links to Swift ABI docs

### Risk: Static And Dynamic Evidence Merge Incorrectly

Mitigation:

- separate `RuntimeObservation` records
- session ids on debugger facts
- static browser badges dynamic observations separately
- export shows evidence class

## Acceptance Criteria

Documentation acceptance:

- this spec is linked from the Hopper-gap README
- feature gap inventory names Swift runtime metadata as a first-class gap
- implementation epics include a Swift runtime metadata epic
- evidence matrix lists Swift runtime metadata validation
- research sources cite Hopper and Swift ABI evidence
- API/data contracts mention Swift route/object/API families
- current-code map states current Swift support is symbol targeting only

Implementation acceptance for first Swift milestone:

- `SwiftMetadata` model exists
- Swift sections are inventoried and exposed via CLI JSON
- demangler service emits facts and explicit failures
- Swift name facts are persisted or project-backed
- Swift browser summary can load without full parse
- malformed Swift metadata fixtures do not panic
- raw and demangled lookup both work
- Swift problem records are visible

Implementation acceptance for Hopper-competitive Swift lane:

- modules, types, protocols, conformances, accessors, runtime calls, and bridge
  relations are routeable
- Swift and ObjC browsers cross-link bridge relations
- Swift metadata facts feed search, inspector, graph/decompiler labels,
  automation, and evidence export
- dyld cache projected images preserve original container routes
- large Swift-heavy binaries remain responsive
- all unsupported or weak evidence is labeled honestly

## First Implementation Recommendation

Start with `S1` through `S3`:

1. Add `damsel-macho/src/swift.rs` with section inventory and a `SwiftMetadata`
   root model.
2. Move the existing CLI demangler shell-out into a reusable service.
3. Emit `SwiftNameFact` records for executable Swift symbols.
4. Add CLI JSON for `damsel swift summary`, `sections`, and `names`.
5. Add tests around `swift-sample`, demangler unavailable behavior, and a
   synthetic malformed Swift section.

This is the smallest real slice because it converts today's ad hoc
`--swift-symbol` feature into a project-shaped subsystem without pretending to
parse every Swift metadata layout at once. It also creates the route/object/API
shape needed for field metadata, conformances, ObjC bridges, and Workbench
browsers later.
