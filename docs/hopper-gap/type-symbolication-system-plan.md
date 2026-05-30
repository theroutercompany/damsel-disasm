# Type And Symbolication System Plan

Status: Architecture and implementation plan for Damsel's type, name, debug,
and symbolication system.

Purpose: define how Damsel should combine symbols, debug info, KDK/dSYM data,
ObjC type encodings, C++ RTTI/mangled names, Swift demangling, ABI inference,
and user-authored types without losing provenance or overwriting user work.

Implementation companion:
[Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
turns this high-level plan into concrete model types, importers, Workbench
surfaces, CLI/API contracts, implementation phases, and validation gates.

Runtime implementation companion:
[Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
defines the concrete ObjC, C++/ObjC++, Swift-adjacent, arm64e/PAC, kernel,
KDK/dSYM, and panic-log objects that consume these name and type facts.

Research companion:
[Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
adds public SDK evidence that Hopper treats names, comments, signatures, type
records, calling conventions, and creation reasons as document-level concepts.

## Thesis

Hopper-like understanding depends on names and types, but Apple-system binaries
mix many evidence sources:

- export trie names
- local symbols
- import/stub names
- ObjC class/category/method/selector metadata
- ObjC type encodings
- Swift mangled names
- C++ mangled names and RTTI
- KDK/dSYM/DWARF symbols and types
- compact unwind/frame facts
- dyld chained fixups
- user labels and signatures

Damsel needs a type and symbolication system that preserves all of those facts,
orders them predictably for display, and exposes contradictions instead of
silently choosing one truth.

## Non-Goals For First Pass

Do not attempt:

- full C parser for arbitrary headers
- complete Swift type metadata reconstruction
- recompilable source recovery
- automatic trust of mismatched KDK/dSYM data
- overwriting user labels with imported symbols
- global type propagation without provenance

First pass should make names/types queryable, explainable, and editable later.

## Core Concepts

### Name Fact

A name fact is an assertion that an object or address should display a
particular name.

Fields:

- name id
- target object or address range
- name text
- demangled text, when applicable
- namespace/owner, when known
- source
- confidence
- precedence
- analyzer/importer version
- stale flag

Name sources:

- user label
- debug/KDK/DWARF
- export trie
- local symbol table
- import binding
- stub helper
- ObjC method metadata
- ObjC selector string
- C++ demangler
- Swift demangler
- analyzer generated
- address fallback

### Type Fact

A type fact is an assertion about a value, address range, function, variable,
field, selector, method, or object.

Fields:

- type fact id
- target
- type id
- role
- source
- confidence
- precedence
- contradiction set
- stale flag

Roles:

- function signature
- return type
- argument type
- stack variable type
- global variable type
- struct field type
- ObjC method encoding
- C++ method type
- vtable slot type
- pointer target type

### Type Record

Type record kinds:

- primitive
- pointer
- array
- struct
- union
- enum
- function
- ObjC class
- ObjC protocol
- ObjC block
- C++ class
- C++ vtable
- Swift nominal placeholder
- opaque
- unknown

Every type record stores provenance and can be imported, inferred, or
user-authored.

## Precedence Rules

### Display Name Precedence

Recommended order:

1. user label
2. exact debug/KDK/dSYM name
3. export or local symbol
4. ObjC owner/selector method name
5. C++ demangled name
6. Swift demangled name
7. import/stub name
8. analyzer-generated function name
9. address fallback

Rules:

- lower-precedence facts remain visible in inspector
- contradictions create problems or inspector warnings
- user can choose display override later

### Type Precedence

Recommended order:

1. user-authored type or signature
2. exact debug/KDK/DWARF type
3. ObjC type encoding
4. C++ RTTI/mangled-name type
5. Swift demangled symbol hint
6. ABI/calling-convention inferred type
7. load/store width inferred type
8. unknown/opaque

Rules:

- imported types never silently replace user-authored types
- weak inferred facts cannot overwrite exact facts
- multiple candidates stay visible

## Symbolication Profiles

Apple-system analysis is OS-build-sensitive. Damsel should model symbolication
profiles explicitly.

Profile fields:

- profile id
- target artifact kind
- OS name/version/build
- architecture
- primary source UUID
- dyld cache UUID
- kernel collection UUID
- KDK path
- dSYM paths
- local symbol sidecar paths
- import timestamp
- match status
- problems

Profile states:

- absent
- candidate
- exact_match
- mismatch
- partial
- stale

Rules:

- profile switch invalidates derived imported facts
- user annotations survive profile switch
- mismatches are typed problems
- Workbench shows profile status prominently

## dSYM And DWARF Import

### Matching

Match on:

- Mach-O UUID
- architecture
- object path/install name where available
- build version where available

Do not import by basename alone.

### Imported Facts

- function names
- source line tables
- inline callsite info later
- function signatures
- type records
- stack/local variables where available

### Errors

- `dsym_not_found`
- `dsym_uuid_mismatch`
- `dsym_arch_mismatch`
- `dwarf_parse_failed`
- `dwarf_unsupported_form`

### UI

- dSYM status in project overview
- source line module in inspector
- imported type/name provenance
- mismatch problem with recovery action

## KDK Import

KDKs are central for kernel workflows.

### Matching

Match on:

- kernel collection UUID
- OS build
- architecture
- contained dSYM UUIDs

Rules:

- exact match required by default
- exploratory mismatch mode must be explicit
- mismatch cannot look like success

### Imported Facts

- kernel function symbols
- kext symbols
- line tables
- C/C++ type records
- function signatures
- struct/enum definitions
- IOKit class hints where derivable

### UI

- KDK status panel
- imported type count
- imported symbol count
- mismatch warnings
- source line links
- type browser scope

## ObjC Type Encoding

ObjC method encodings should become type facts.

Inputs:

- method type encodings
- property attributes
- ivar offsets/types
- protocol method signatures
- block signatures where available

Outputs:

- method signature facts
- argument/return type facts
- ivar field type facts
- property type facts
- selector signature candidates

Ambiguity:

- the same selector can have multiple signatures
- category/protocol signatures can disagree
- dynamic dispatch may not prove receiver class

UI:

- selector page lists signature candidates
- method page shows owner-specific signature
- inspector shows source of type encoding

## C++ Type Sources

Inputs:

- mangled names
- RTTI typeinfo
- vtable layout
- constructor/destructor patterns
- debug/KDK/DWARF types

Outputs:

- C++ class records
- method signature candidates
- vtable type records
- inheritance edges
- virtual call slot types

Rules:

- demangling gives names, not full class truth by itself
- RTTI absence does not mean C++ absence
- vtable scanner emits candidates with confidence

## Swift Boundary

Swift support should begin with symbol understanding.

Inputs:

- Swift mangled symbols
- exported/local symbol names
- metadata references where parser support exists

Outputs:

- demangled display names
- searchable Swift names
- function name facts
- placeholder type records

Non-goals:

- full Swift ABI type reconstruction in the first pass
- Swift pseudocode
- source-level Swift recovery

## User Types And Signatures

User-authored type work arrives after the project database and annotation
transactions.

User actions:

- name type
- define struct/union/enum
- assign function signature
- assign calling convention
- name stack variable
- assign stack/global type
- choose display name source

Rules:

- user type changes are transactions
- changes are undoable
- dependent facts are invalidated
- imported facts remain visible below user facts

## API Surface

Endpoints:

- `GET /api/projects/{project_id}/names?target=...`
- `GET /api/projects/{project_id}/types`
- `GET /api/projects/{project_id}/types/{type_id}`
- `GET /api/projects/{project_id}/symbolication-profiles`
- `POST /api/projects/{project_id}/sources/attach`
- `POST /api/projects/{project_id}/types`
- `POST /api/projects/{project_id}/functions/{function_id}/signature`

CLI:

- `project names`
- `project types`
- `project attach-dsym`
- `project attach-kdk`
- `project symbolication-status`
- `project set-signature`, later

## Workbench Surfaces

### Project Overview

Show:

- active symbolication profile
- dSYM/KDK match status
- imported symbol/type counts
- mismatch problems

### Inspector

Show:

- active display name source
- alternate names
- type facts
- source line info
- contradictions
- user override controls later

### Type Browser

Tabs:

- structs/unions/enums
- function signatures
- ObjC types
- C++ types/vtables
- imported KDK/DWARF types
- user types

### Function View

Show:

- signature
- calling convention
- argument/return facts
- stack variables
- source lines
- name alternatives

## Validation

Fixtures:

- dSYM UUID match
- dSYM UUID mismatch
- ObjC type encoding
- C++ demangle
- RTTI/vtable
- KDK mismatch synthetic
- user signature transaction later

Assertions:

- imported names do not overwrite user labels
- mismatched sidecars produce problems
- type precedence is deterministic
- selector signatures can be multiple
- demangled names preserve raw names

## Implementation Sequence

1. Add name fact model.
2. Add type record and type fact models.
3. Add display precedence resolver.
4. Normalize existing symbol/export/import/ObjC names into name facts.
5. Normalize ObjC type encodings into type facts.
6. Add dSYM attachment and UUID mismatch problem.
7. Add project symbolication profile model.
8. Add C++ demangler/type placeholders.
9. Add KDK matching/import.
10. Add user-authored type/signature transactions.

## Acceptance Criteria

The type/symbolication system is credible when:

- every displayed name can show provenance
- every imported sidecar has match status
- mismatched KDK/dSYM data cannot silently apply
- ObjC type encodings become queryable signatures
- C++ and Swift demangled names are searchable without replacing raw names
- user-authored labels/types have highest precedence
- type/name contradictions remain inspectable
- function summaries can consume the same type facts as the UI
