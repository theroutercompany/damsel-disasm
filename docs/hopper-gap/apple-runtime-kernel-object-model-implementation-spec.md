# Apple Runtime And Kernel Object Model Implementation Spec

Status: implementation-grade planning spec. This is not an implementation
claim.

Purpose: define the concrete object model, analysis passes, API contracts, UI
surfaces, tests, and sequencing required for Damsel's Apple-focused competitive
advantage: Objective-C, Objective-C++, C++, Swift-adjacent symbol knowledge,
arm64e/PAC/chained fixups, dyld shared caches, driver caches, kernel
collections, KDK/dSYM symbolication, and panic-log navigation.

This spec assumes the project/document kernel and typed byte map described in
[Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
and the semantic UI layer described in
[Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md).
It also assumes the type/signature/debug-info contracts described in
[Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md),
and the dedicated debug-source matching/import rules described in
[DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md),
because ObjC encodings, C++ RTTI/vtables, Swift-adjacent names, KDK/dSYM
profiles, panic-frame symbolication, and kernel structs all consume or produce
type and signature facts.

Swift runtime companion:
[Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec](swift-runtime-metadata-implementation-spec.md)
owns the deeper Swift object model that this document previously summarized as
Swift-adjacent facts: Swift sections, demangled-name evidence, nominal types,
fields, protocols, conformances, metadata accessors, runtime calls, and
Swift-ObjC bridge relations.

Workbench browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
defines how the object records and analysis passes in this document become
native browser pages, paged lists, detail views, command families, relationship
graphs, confidence displays, and runtime problem surfaces.

## Why This Spec Exists

Hopper's public surface is broad, but its Apple-specific lane is visible in
official sources:

- the homepage advertises Objective-C and Swift support
- the FAQ says Hopper handles Mach-O and iOS binaries
- the tutorial shows Objective-C method tags generated during loading
- the changelog repeatedly mentions dyld shared cache, Objective-C headers,
  ObjC stubs, Swift metadata, Swift strings, Swift demangling, chained fixups,
  PAC, authenticated Objective-C selector trampolines, driver cache dylibs,
  kernel caches, and kext crashes
- the public SDK exposes `FLS_ParseObjectiveC`, `FLS_ParseSwift`,
  `Call_ObjectiveC`, Swift and Objective-C calling-convention hints, metadata
  name reasons, type records, method signatures, and procedure-level calling
  conventions

Damsel already has a stronger Apple starting point than a generic disassembler:

- Mach-O parsing
- arm64/arm64e disassembly
- dyld shared-cache inventory and projection
- shared-cache symbolication and dependency/provider queries
- ObjC metadata extraction for classes, protocols, categories, methods,
  properties, ivars, selector refs, class refs, and method IMPs
- disassembly analysis with imports, ObjC uses, cache links, recovered values,
  jump tables, and indirect targets

The missing piece is normalization. Current Damsel facts are mostly per-image
outputs. Hopper-like workflows need normalized runtime objects with stable ids,
cross-image routes, provenance, confidence, and UI/API queries.

## Goals

- Make Objective-C selectors, classes, categories, protocols, methods, ivars,
  properties, IMPs, and message-send callsites first-class project objects.
- Make dyld shared-cache images, install names, provider/importer edges,
  reexports, projected images, and cross-image runtime xrefs first-class
  project objects.
- Add a C++ and Objective-C++ model for Itanium ABI symbols, RTTI, vtables,
  constructors, destructors, virtual methods, virtual callsites, and IOKit
  overlays.
- Add a Swift-adjacent model for demangled names, Swift string facts, known
  metadata regions, and cautious placeholders, without claiming full Swift
  metadata reconstruction.
- Add an arm64e/PAC/chained-fixup model that records raw pointers, stripped
  targets, authentication context where recoverable, and confidence.
- Add a kernel collection/fileset/kext model with member address spaces, kext
  identity, KDK/dSYM status, syscalls, Mach traps, MIG routines, IOKit classes,
  and panic frames.
- Add symbolication profiles that can combine local symbols, dyld local-symbol
  sidecars, dSYM, DWARF, KDK, ObjC encodings, C++ demangling, Swift demangling,
  and user overrides without overwriting higher-precedence facts.
- Make every runtime object searchable, routeable, inspectable, and exportable.

## Non-Goals

- No broad PE/ELF parity.
- No generic CPU plugin SDK in this phase.
- No jailbreak workflow as a product requirement.
- No full Swift type metadata reconstruction in the first implementation.
- No recompilable decompiler output claims.
- No binary patching or modified executable production in this phase.
- No live process debugging in this phase.
- No silent trust of mismatched KDK/dSYM data.

## Current Damsel Inputs To Reuse

### Existing Core Inputs

Reuse existing per-image models:

- `BinaryImage`
- Mach-O load commands
- segments and sections
- symbols and exports
- `DyldMetadata`
- `ObjcMetadata`
- `DisassemblyAnalysis`
- import, stub, binding, and helper facts
- function starts
- chained-fixup presence and existing fixup parse data
- recovered values and semantic disassembly annotations

### Existing Shared-Cache Inputs

Reuse existing shared-cache models:

- `SharedCache`
- `SharedCacheSession`
- projected binary image model
- image inventory
- image dependencies
- provider/importer/reexport queries
- cache symbolication
- cache route/provenance concepts from current docs

### Existing CLI/API Inputs

Reuse and extend current commands rather than breaking them:

- standalone analysis remains available
- cache inventory remains available
- cache projection remains available
- cache disassembly with analysis remains the natural place for cache-aware
  enrichment until the project kernel takes over

## Object Model Overview

Runtime objects live in the project database. Every object stores:

- stable id
- kind
- owning source id
- address space id, when addressable
- primary address or range, when addressable
- display name
- provenance set
- confidence
- stale flag
- related objects
- problems

Runtime object ids should be deterministic where possible:

```text
objc-selector:<normalized-selector>
objc-class:<source-id>:<runtime-name>
objc-category:<source-id>:<class-name>:<category-name>
objc-method:<source-id>:<owner-id>:<selector>:<kind>
objc-message:<procedure-id>:<address>
cxx-class:<source-id>:<mangled-or-demangled-name>
cxx-vtable:<source-id>:<address>
kernel-member:<source-id>:<member-name-or-uuid>
kernel-syscall:<source-id>:<number>
panic-frame:<panic-log-id>:<index>
```

The exact string format is internal. API clients treat ids as opaque.

## Objective-C Object Model

### ObjC Selector Object

Represents a selector string across the project.

Fields:

- selector id
- selector text
- normalized text
- string addresses
- selector-ref addresses
- owning sources/images
- implementer method ids
- message-send candidate ids
- direct metadata reference ids
- confidence summary
- provenance list

Queries:

- implementers by image/class/category
- callsites by confidence
- references by source
- selector string locations
- related selectors by prefix or family

Acceptance:

- Opening a selector route shows all known implementers and all known callers,
  separated by evidence class.

### ObjC Class Object

Represents an Objective-C class recovered from metadata.

Fields:

- class id
- class name
- source/image id
- runtime address
- metaclass address
- superclass reference
- protocols
- methods
- properties
- ivars
- categories extending this class
- class refs
- method implementation function ids
- header-generation status
- provenance list

Special fields:

- `is_external_reference`: true when class is referenced but not implemented in
  the current source
- `resolved_provider`: class object in another cache image when linked through
  shared-cache indexes
- `ambiguous_providers`: multiple candidates when names collide

Acceptance:

- Class route can answer: "where is this class defined, what methods does it
  implement, what images reference it, and what method implementation functions
  are linked?"

### ObjC Category Object

Fields:

- category id
- category name
- target class name
- target class id, when resolved
- source/image id
- methods
- properties
- protocols
- runtime address
- provenance list

Acceptance:

- Category methods appear both under the category route and under the target
  class route with category provenance preserved.

### ObjC Protocol Object

Fields:

- protocol id
- protocol name
- source/image id
- required instance methods
- required class methods
- optional instance methods
- optional class methods
- properties
- conforming classes/categories, when known
- references
- provenance list

Acceptance:

- Protocol route exposes method requirements and all known conformers in the
  currently loaded project/cache scope.

### ObjC Method Object

Represents a method declaration/implementation pair when possible.

Fields:

- method id
- owner kind: class, metaclass, category, protocol
- owner id
- selector id
- method kind: instance, class, unknown
- implementation address
- implementation function id
- type encoding raw string
- decoded signature id
- source/image id
- runtime metadata address
- name fact id
- provenance list

Derived display name:

```text
-[Class selector:]
+[Class selector:]
-[Class(Category) selector:]
```

Acceptance:

- Method object links to function/procedure route.
- Function/procedure route links back to method object.
- Type encoding, decoded arguments, and provenance are visible in inspector.

### ObjC Ivar And Property Objects

Ivar fields:

- ivar id
- owner class id
- name
- offset
- type encoding
- decoded type id
- memory range, when static metadata address is known
- provenance

Property fields:

- property id
- owner id
- name
- attributes raw string
- decoded attributes
- getter selector id
- setter selector id
- inferred backing ivar id
- provenance

Acceptance:

- Class route shows ivars and properties.
- Selector route links getter/setter selectors back to properties where
  evidence exists.

## Objective-C Dispatch Model

### ObjC Message Send Candidate

Represents an instruction-level candidate for Objective-C dispatch.

Fields:

- candidate id
- procedure id
- source address
- call instruction address
- call target symbol or import id
- dispatch family:
  - `objc_msgSend`
  - `objc_msgSendSuper`
  - `objc_msgSendSuper2`
  - `objc_msgSend_stret`
  - `objc_msgSend_fpret`
  - retain/release/autorelease family
  - authenticated selector trampoline
  - unknown objc runtime call
- receiver evidence
- selector evidence
- selector id, when resolved
- possible implementer method ids
- confidence
- provenance
- problems

Receiver evidence kinds:

- direct class ref
- metaclass ref
- `self`
- `super`
- allocation result
- known return type
- argument type
- local variable type
- manual binding
- unknown

Selector evidence kinds:

- immediate selector ref
- recovered ADRP/add/ldr sequence
- stub/trampoline semantic
- metadata nearby
- manual binding
- unknown

Confidence values:

- exact: selector and receiver class both known
- strong: selector known and receiver family strongly inferred
- inferred: selector known, receiver unknown, implementers listed as candidates
- ambiguous: multiple possible selectors or receiver classes
- weak: pattern resembles ObjC dispatch but key evidence missing

Acceptance:

- Instruction inspector shows dispatch family, selector, receiver evidence,
  candidates, confidence, and why it is unresolved if unresolved.

### ObjC Dispatch Binding

Represents a chosen or inferred target relationship from a message send to a
method implementation.

Fields:

- binding id
- message candidate id
- selector id
- receiver object id, when known
- method id
- implementation function id
- binding source:
  - analyzer
  - type propagation
  - cache provider index
  - user assertion
- confidence
- is_user_override
- stale flag

Rules:

- User binding outranks inferred binding.
- Inferred binding never deletes alternatives.
- Ambiguous bindings remain visible as alternatives.
- Binding invalidates when source facts or symbolication profile changes.

Acceptance:

- Selector route can separate "proven callers", "likely callers", and
  "ambiguous dispatch candidates".

## Dyld Shared Cache Runtime Model

### Cache Source Object

Fields:

- cache source id
- path
- architecture
- platform
- UUID/build when known
- subcache parts
- image count
- local symbols sidecar status
- analysis status
- problems

### Cache Image Object

Fields:

- image id
- cache source id
- install name
- image path
- image index
- image VM range
- cache VM range
- projected source id, when projected
- dependencies
- dependents
- reexports
- exports
- imports
- ObjC classes/selectors/protocols
- C++ objects
- strings
- analysis status

Acceptance:

- Cache overview opens quickly with paged image list.
- Image route shows dependencies, dependents, exports, imports, classes,
  selectors, strings, and analysis state.

### Provider, Importer, And Reexport Edges

Edge fields:

- edge id
- edge kind: dependency, provider, importer, reexport
- from image id
- to image id
- symbol or install-name target
- resolution state
- provenance
- confidence

Acceptance:

- "Follow provider" from an import lands on the provider image/function when
  resolvable.
- Ambiguous providers are listed with reasons.

### Cross-Image Runtime Xref

Fields:

- xref id
- source image id
- source address/object
- target image id
- target address/object
- xref kind
- via import/stub/fixup/reexport/selector
- provenance
- confidence

Acceptance:

- Function xrefs include cross-image edges without losing cache/image address
  context.

## C++ And Objective-C++ Object Model

Apple userland and kernel binaries often mix C, Objective-C, C++, and
Objective-C++. Damsel should target the Itanium C++ ABI used by Apple Clang for
Mach-O C++ names and RTTI, with Apple-specific system idioms layered on top.

### C++ Symbol Object

Fields:

- symbol id
- raw mangled name
- demangled name
- source/image id
- address
- symbol role:
  - function
  - method
  - constructor
  - destructor
  - vtable
  - typeinfo
  - typeinfo name
  - thunk
  - guard variable
  - static initializer
  - unknown
- namespace components
- class name, when applicable
- method name, when applicable
- signature hint
- provenance

Acceptance:

- Demangled names create name facts without overwriting user labels or exact
  debug names.

### C++ Class Object

Fields:

- class id
- demangled name
- namespace
- source/image id
- typeinfo object id
- typeinfo name address
- vtable ids
- base class candidates
- method ids
- constructor ids
- destructor ids
- virtual method slots
- ObjC bridge evidence, when ObjC++ patterns exist
- IOKit overlay id, when recognized
- provenance
- confidence

Acceptance:

- C++ browser lists classes with vtables, constructors, destructors, and
  methods.

### C++ Vtable Object

Fields:

- vtable id
- source/image id
- address
- class id, when resolved
- address point
- slots
- raw pointer facts
- stripped pointer targets for arm64e
- typeinfo pointer
- construction vtable marker, when suspected
- provenance
- confidence

Slot fields:

- slot index
- slot address
- raw pointer value
- stripped target address
- target function id
- target symbol id
- thunk marker
- method role
- confidence

Acceptance:

- Vtable route shows all slots and function targets.
- Slot targets route to procedures.
- PAC/chained-fixup evidence is visible for authenticated pointers.

### C++ RTTI Object

Fields:

- RTTI id
- source/image id
- address
- class id
- typeinfo kind
- typeinfo name
- base descriptors, when parsed
- vtable links
- provenance
- confidence

Acceptance:

- RTTI route explains which bytes/fixups produced the class relationship.

### C++ Virtual Callsite Object

Fields:

- callsite id
- procedure id
- address
- receiver value evidence
- vtable load evidence
- slot offset
- possible vtable ids
- possible method targets
- confidence
- provenance
- manual binding id, when present

Acceptance:

- Instruction inspector can say "virtual call slot 0x28" and list possible
  target methods if vtables are known.

### IOKit Overlay

IOKit is central for XNU and driver analysis. The first implementation should
not pretend to fully understand every IOKit convention, but it should detect and
surface high-value patterns.

IOKit class object fields:

- IOKit class id
- C++ class id
- bundle/kext id
- service/provider/user-client markers
- superclass candidates
- vtable id
- external method table candidates
- dispatch table candidates
- matching dictionary string refs, when recoverable
- lifecycle methods:
  - `init`
  - `start`
  - `stop`
  - `probe`
  - `free`
  - `newUserClient`
  - external method handlers
- provenance
- confidence

Acceptance:

- Kernel/kext route can list likely IOKit classes and route to vtables and
  external methods.

## Swift-Adjacent Object Model

Damsel should support useful Swift navigation without overclaiming full Swift
reconstruction.

### Swift Symbol Object

Fields:

- symbol id
- mangled name
- demangled name
- source/image id
- address
- role hint:
  - function
  - method
  - thunk
  - protocol witness
  - metadata accessor
  - nominal descriptor
  - type metadata
  - string/reference helper
  - unknown
- module name, when demangled
- type name, when demangled
- function signature text, when demangled
- provenance
- confidence

### Swift Metadata Placeholder

Fields:

- placeholder id
- source/image id
- address/range
- kind hint
- related Swift symbol ids
- related string refs
- raw bytes route
- confidence
- problems

Rules:

- Placeholder means "known Swift-related region or symbol", not complete
  semantic recovery.
- UI must label it as partial.

Acceptance:

- Swift names and strings improve search, symbolication, and inspector context
  without claiming recompilable Swift.

## arm64e, PAC, And Chained Fixup Model

### Pointer Target Fact

Fields:

- pointer fact id
- source id
- address/range of pointer
- raw encoded value
- stripped target value, when recoverable
- target address/object, when resolved
- pointer kind:
  - raw pointer
  - chained fixup
  - authenticated pointer
  - bind
  - rebase
  - selector ref
  - class ref
  - vtable slot
  - function pointer
  - unknown
- auth key, discriminator, diversity, or address-diversity fields when
  recoverable from format
- provenance
- confidence

Acceptance:

- Hex/inspector can show both raw pointer encoding and resolved target when
  available.

### Authenticated Selector Trampoline Object

Fields:

- trampoline id
- source/image id
- address
- selector id, when recovered
- target runtime call
- PAC/authentication evidence
- callsite ids
- provenance
- confidence

Acceptance:

- ObjC dispatch recognizer handles authenticated selector trampolines as a
  first-class dispatch family, matching Hopper's public changelog direction.

### Chained Fixup Segment Model

Fields:

- fixup segment id
- source id
- segment/section
- page starts
- fixup entries
- imports
- binds
- rebases
- authenticated entries
- target objects
- parse problems

Acceptance:

- Fixup route can answer which runtime objects were created from chained
  fixups and which fixups remain unresolved.

## Kernel Collection And XNU Object Model

### Kernel Collection Source

Fields:

- collection id
- path
- architecture
- UUID/build
- fileset/kc kind
- global address space id
- member address spaces
- members
- kexts
- KDK/dSYM profile status
- analysis status
- problems

### Fileset Member Object

Fields:

- member id
- collection id
- name/install name/bundle id
- UUID
- source range
- global VM range
- member VM range
- segments/sections
- exports/imports
- symbols
- ObjC/C++/Swift objects
- IOKit overlay objects
- analysis status

Acceptance:

- Kernel collection overview lists members quickly and supports routing between
  global and member addresses.

### Kext Object

Fields:

- kext id
- collection id
- member id
- bundle id
- version
- executable name
- dependencies
- IOKit classes
- symbols
- start/stop routines, when recoverable
- entitlements/info metadata if available from source bundle or sidecar
- provenance

Acceptance:

- Kext browser supports search by bundle id, class name, symbol, and address.

### Kernel Interface Objects

Syscall entry fields:

- syscall id
- number
- name
- table address
- handler address
- handler function id
- argument count/type hints, when known
- provenance
- confidence

Mach trap fields:

- trap id
- number
- name
- table address
- handler address
- handler function id
- provenance
- confidence

MIG routine fields:

- MIG routine id
- subsystem name/number, when known
- message id
- routine name
- dispatch table address
- handler function id
- request/reply type hints, when known
- provenance
- confidence

Acceptance:

- Kernel project route exposes syscall, Mach trap, and MIG browsers.
- Interface entries route to functions and xrefs.

## KDK, dSYM, DWARF, And Symbolication Profiles

### Symbolication Profile

Fields:

- profile id
- project id
- target source/member ids
- OS version/build
- architecture
- primary UUIDs
- KDK path
- dSYM paths
- local symbol sidecar paths
- imported symbol count
- imported type count
- imported line table count
- match status:
  - absent
  - candidate
  - exact_match
  - mismatch
  - partial
  - stale
- problems

Rules:

- Match by UUID/build/architecture, never basename alone.
- User can attach mismatched data only in explicit exploratory mode.
- Profile changes mark imported facts stale.
- User labels and user signatures survive profile changes.

### Imported Symbol Fact

Fields:

- symbol fact id
- source profile id
- target object/address
- symbol text
- demangled text
- source kind:
  - dSYM
  - KDK
  - DWARF
  - local symbols
  - export trie
  - ObjC metadata
  - C++ demangler
  - Swift demangler
  - user
- confidence
- precedence
- stale flag

Acceptance:

- Inspector lists all names and tells why the displayed name won.

## Panic Log Object Model

### Panic Log Source

Fields:

- panic log id
- raw text hash
- parsed OS/build
- kernel UUID/build
- architecture
- kernel slide
- panic string
- backtrace frames
- loaded kext list
- parse problems
- matched project id
- symbolication profile id

### Panic Frame Object

Fields:

- frame id
- panic log id
- index
- raw address
- unslid address
- address space id
- source/member/kext id
- function/procedure id, when resolved
- symbolicated name
- source line, when available
- confidence
- problems

Acceptance:

- Panic frame route opens matching function/procedure and shows frame context.
- Unresolved frames explain missing slide, missing KDK, missing kext, or address
  conversion failure.

## Analysis Pass Pipeline

### Pass 1: Loader Seed Pass

Inputs:

- Mach-O load commands
- segments/sections
- symbols
- exports
- imports
- function starts
- chained-fixup headers
- ObjC metadata sections
- Swift-related sections

Outputs:

- source object
- address spaces
- segment/section objects
- byte classifications
- entrypoint/procedure seeds
- initial name facts
- initial type placeholders
- problems

### Pass 2: ObjC Metadata Normalization

Inputs:

- existing `ObjcMetadata`
- selector/class/method/property/ivar/protocol sections
- chained fixup pointer facts

Outputs:

- selector objects
- class objects
- category objects
- protocol objects
- method objects
- ivar/property objects
- method IMP to procedure seeds
- name/type facts
- byte classifications for ObjC metadata ranges

### Pass 3: Dyld And Fixup Linkage

Inputs:

- `DyldMetadata`
- imports/exports
- stubs and stub helpers
- chained fixups
- shared-cache image index

Outputs:

- pointer target facts
- import/export objects
- provider/importer/reexport edges
- cross-image xrefs
- authenticated pointer facts
- problem records for unresolved imports/fixups

### Pass 4: Function And IMP Linking

Inputs:

- function starts
- symbols
- exports
- ObjC method IMPs
- C++ method symbols
- vtable slot targets
- disassembly seeds

Outputs:

- procedure records
- method-to-procedure links
- class method lists enriched with procedure ids
- function name facts

### Pass 5: Message-Send Recognition

Inputs:

- disassembly analysis
- recovered values
- import/stub refs
- ObjC selectors/classes
- arm64e authenticated selector trampolines

Outputs:

- message-send candidates
- dispatch bindings
- selector caller indexes
- implementer candidate lists
- confidence/problems

### Pass 6: C++ Symbol And RTTI Pass

Inputs:

- symbol table
- export names
- local symbols
- dSYM/KDK symbols
- relevant sections
- pointer target facts

Outputs:

- C++ symbol objects
- class objects
- RTTI objects
- vtable candidates
- constructor/destructor candidates
- name/type facts

### Pass 7: Vtable And Virtual Callsite Pass

Inputs:

- C++ class/vtable/RTTI objects
- pointer target facts
- disassembly memory references
- recovered register/value facts

Outputs:

- vtable slots
- virtual callsite objects
- possible target edges
- IOKit overlay seeds

### Pass 8: IOKit Overlay Pass

Inputs:

- C++ class/vtable objects
- kernel/kext metadata
- known IOKit method names and lifecycle patterns
- string refs and dispatch tables

Outputs:

- IOKit class objects
- lifecycle method links
- external method table candidates
- service/provider/user-client hints

### Pass 9: Kernel Collection Pass

Inputs:

- kernel collection/fileset Mach-O
- fileset entries
- kext/member data
- KDK/dSYM sources

Outputs:

- kernel collection object
- fileset member objects
- kext objects
- member address spaces
- symbolication profile
- kernel interface object seeds

### Pass 10: Kernel Interface Pass

Inputs:

- symbolication facts
- known XNU table patterns
- disassembly and data references
- KDK types/symbols

Outputs:

- syscall objects
- Mach trap objects
- MIG routine objects
- table byte classifications
- xrefs to handlers

### Pass 11: Panic Symbolication Pass

Inputs:

- panic log source
- kernel collection source
- kext list
- symbolication profile

Outputs:

- panic log object
- panic frame objects
- frame route links
- unresolved-frame problems

## Project Kernel Integration

Runtime objects are stored as facts and indexed objects.

Required indexes:

- object id to object
- address to objects
- source/image to objects
- selector text to selector object
- class name to class objects
- method selector to method objects
- procedure id to runtime owners
- runtime object to procedure id
- vtable address to vtable object
- vtable slot target to procedure id
- C++ demangled name to object
- KDK/dSYM source to imported facts
- kernel member to objects
- panic frame to function route

Required route kinds:

- `objc-selector`
- `objc-class`
- `objc-category`
- `objc-protocol`
- `objc-method`
- `objc-message`
- `cache-image`
- `cache-edge`
- `cxx-class`
- `cxx-vtable`
- `cxx-virtual-call`
- `swift-symbol`
- `fixup`
- `kernel-collection`
- `kernel-member`
- `kext`
- `kernel-interface`
- `panic-log`
- `panic-frame`

## API Contracts

### ObjC APIs

```text
GET /api/projects/{project_id}/objc/classes
GET /api/projects/{project_id}/objc/classes/{class_id}
GET /api/projects/{project_id}/objc/selectors
GET /api/projects/{project_id}/objc/selectors/{selector_id}
GET /api/projects/{project_id}/objc/methods/{method_id}
GET /api/projects/{project_id}/objc/messages/{message_id}
POST /api/projects/{project_id}/objc/messages/{message_id}/bind
```

Selector response shape:

```json
{
  "id": "objc-selector:initWithFrame:",
  "text": "initWithFrame:",
  "implementers": [],
  "callers": {
    "exact": [],
    "strong": [],
    "inferred": [],
    "ambiguous": []
  },
  "string_locations": [],
  "provenance": []
}
```

### C++ APIs

```text
GET /api/projects/{project_id}/cxx/classes
GET /api/projects/{project_id}/cxx/classes/{class_id}
GET /api/projects/{project_id}/cxx/vtables
GET /api/projects/{project_id}/cxx/vtables/{vtable_id}
GET /api/projects/{project_id}/cxx/virtual-calls/{callsite_id}
POST /api/projects/{project_id}/cxx/virtual-calls/{callsite_id}/bind
```

### Cache APIs

```text
GET /api/projects/{project_id}/cache/images
GET /api/projects/{project_id}/cache/images/{image_id}
GET /api/projects/{project_id}/cache/edges
GET /api/projects/{project_id}/cache/providers?symbol=
GET /api/projects/{project_id}/cache/selectors?selector=
```

### Kernel APIs

```text
GET /api/projects/{project_id}/kernel/collections
GET /api/projects/{project_id}/kernel/members
GET /api/projects/{project_id}/kernel/kexts
GET /api/projects/{project_id}/kernel/syscalls
GET /api/projects/{project_id}/kernel/mach-traps
GET /api/projects/{project_id}/kernel/mig-routines
GET /api/projects/{project_id}/kernel/iokit/classes
```

### Symbolication APIs

```text
GET /api/projects/{project_id}/symbolication/profiles
POST /api/projects/{project_id}/symbolication/profiles
POST /api/projects/{project_id}/symbolication/profiles/{profile_id}/attach-dsym
POST /api/projects/{project_id}/symbolication/profiles/{profile_id}/attach-kdk
GET /api/projects/{project_id}/symbolication/names?target_id=
GET /api/projects/{project_id}/symbolication/types?target_id=
```

### Panic APIs

```text
POST /api/projects/{project_id}/panic-logs
GET /api/projects/{project_id}/panic-logs
GET /api/projects/{project_id}/panic-logs/{panic_log_id}
GET /api/projects/{project_id}/panic-logs/{panic_log_id}/frames
GET /api/projects/{project_id}/panic-logs/{panic_log_id}/frames/{frame_id}
```

## CLI Contracts

```text
damsel project objc classes <project>
damsel project objc class <project> <class>
damsel project objc selector <project> <selector>
damsel project objc method <project> <method-id>
damsel project objc messages <project> --selector ...
damsel project objc bind-message <project> <message-id> --method ...

damsel project cxx classes <project>
damsel project cxx class <project> <class-id>
damsel project cxx vtables <project>
damsel project cxx vtable <project> <vtable-id>
damsel project cxx virtual-call <project> <callsite-id>

damsel project cache images <project>
damsel project cache image <project> <image>
damsel project cache providers <project> <symbol>
damsel project cache selector <project> <selector>

damsel project kernel members <project>
damsel project kernel kexts <project>
damsel project kernel syscalls <project>
damsel project kernel mach-traps <project>
damsel project kernel mig <project>
damsel project kernel iokit <project>

damsel project symbols profiles <project>
damsel project symbols attach-dsym <project> <path>
damsel project symbols attach-kdk <project> <path>

damsel project panic attach <project> <panic-log>
damsel project panic frames <project> <panic-log-id>
```

Output rules:

- JSON by default for machine-readable subcommands.
- Paged output for large lists.
- Every result includes ids and routes.
- Every inferred relation includes confidence and provenance.
- Unresolved or ambiguous data is explicit, not omitted silently.

## Workbench Surfaces

### ObjC Browser

Panels:

- classes
- selectors
- protocols
- categories
- method implementers
- message-send callsites

Inspector modules:

- class summary
- method list
- selector callers
- method implementation procedure
- type encoding decoder
- metadata provenance
- ambiguity/problems

### C++ Browser

Panels:

- classes
- vtables
- RTTI
- constructors/destructors
- virtual callsites
- IOKit overlays

Inspector modules:

- vtable slots
- class hierarchy candidates
- method target routes
- demangling evidence
- virtual-call target candidates

### Cache Browser

Panels:

- images
- dependencies
- providers/importers
- selectors/classes by image
- cross-image xrefs

Inspector modules:

- install name and ranges
- projected source status
- dependencies/dependents
- export/import details
- route conversions

### Kernel Browser

Panels:

- collection members
- kexts
- IOKit classes
- syscalls
- Mach traps
- MIG routines
- KDK/dSYM status

Inspector modules:

- member address conversions
- KDK match status
- symbol/type provenance
- handler routes
- unresolved problems

### Panic Browser

Panels:

- attached logs
- frames
- kext list
- unresolved frames

Inspector modules:

- panic string
- slide
- frame symbolication
- function route
- missing evidence recovery

## Test Plan

### Unit Tests

- selector normalization
- ObjC method id determinism
- type encoding decoder
- message-send confidence ordering
- C++ demangler classification
- vtable slot parser
- RTTI parser
- arm64e pointer target stripping helpers
- chained fixup target conversion
- address-space conversion for cache images
- address-space conversion for kernel members
- symbolication profile matching
- panic frame unsliding

### Fixture Tests

Fixture classes:

- small Objective-C Mach-O with classes, categories, properties, ivars, and
  protocols
- Objective-C binary with simple `objc_msgSend`
- arm64e fixture with selector refs and authenticated selector trampoline when
  legally available
- C++ Mach-O with vtable, RTTI, constructor, destructor, and virtual call
- ObjC++ fixture with ObjC class calling into C++ object or C++ method carrying
  ObjC refs
- small shared-cache projection fixture
- dyld cache image with cross-image import/provider edge
- kernel/fileset fixture when legally available
- KDK/dSYM matching fixture
- mismatched dSYM fixture
- panic log text fixture

### Snapshot Tests

Snapshot outputs:

- ObjC class route
- selector route
- method route
- message-send candidate
- C++ class route
- vtable route
- virtual callsite
- cache image route
- provider edge
- kernel member route
- syscall route
- symbolication profile
- panic frame route

### Performance Tests

Targets:

- selector search in large cache under interactive budget
- class browser paging under interactive budget
- vtable scan cancellable on large executable range
- symbolication profile attach progress visible and cancellable
- panic log parse effectively instant for ordinary panic logs
- cache image overview loads without projecting every image

## Acceptance Criteria By Milestone

### ObjC Runtime Explorer Acceptance

- Selector route lists implementers and callers with confidence buckets.
- Method route links metadata, type encoding, and implementation procedure.
- Instruction inspector explains `objc_msgSend` evidence.
- Shared-cache selector query can cross image boundaries.
- User binding of ambiguous message send persists as user provenance.

### C++/ObjC++ Explorer Acceptance

- Demangled C++ names create name facts.
- Vtables are detected and routeable.
- Vtable slots link to function/procedure routes.
- Virtual callsites list possible targets when evidence exists.
- IOKit class candidates are visible with confidence and evidence.

### Kernel Workflow Acceptance

- Kernel collection opens into member/kext browser.
- Member/global address conversions work and are visible.
- KDK/dSYM attach verifies UUID/build/architecture.
- Syscall/Mach trap/MIG browsers are present when evidence exists.
- Panic log frames route to functions when symbolication data is sufficient.

## Sequencing

### Phase A: Normalized ObjC Objects

Build:

- selector/class/category/protocol/method/ivar/property objects
- method IMP to procedure seeds
- selector implementer index
- class and selector API/CLI
- Workbench ObjC browser skeleton

Depends on:

- project kernel
- object ids
- byte map
- existing ObjC metadata extraction

### Phase B: ObjC Dispatch

Build:

- message-send recognizer
- selector evidence extraction
- receiver evidence extraction
- dispatch binding model
- instruction inspector module
- manual binding command

Depends on:

- function/xref index
- architecture semantics
- recovered value facts

### Phase C: Cache-Wide Runtime Indexes

Build:

- cache image objects
- provider/importer/reexport edges
- cross-image selector/class indexes
- cross-image runtime xrefs
- image overview

Depends on:

- shared-cache projection
- project source relationships
- ObjC object model

### Phase D: C++ And ObjC++ Foundation

Build:

- demangler wrapper
- C++ symbol classifier
- RTTI/vtable scanner
- C++ class browser
- virtual callsite candidates

Depends on:

- symbol/name fact system
- procedure index
- pointer target facts

### Phase E: IOKit And Kernel Foundation

Build:

- kernel collection/fileset source model
- member/kext browser
- IOKit overlays
- KDK/dSYM profile attach
- basic kernel interface tables

Depends on:

- source/container model
- C++ model
- symbolication profiles

### Phase F: Panic Integration

Build:

- panic log parser
- frame unsliding
- kext matching
- frame route generation
- panic browser

Depends on:

- kernel source model
- symbolication profiles

## Risks And Mitigations

### Risk: Overconfident Runtime Edges

ObjC dispatch and C++ virtual calls are often ambiguous.

Mitigation:

- Always store confidence.
- Show alternatives.
- Preserve manual bindings separately.
- Avoid changing call graph semantics from weak evidence alone.

### Risk: Swift Scope Explosion

Swift metadata is deep and version-sensitive.

Mitigation:

- Start with demangled names, string facts, and explicit placeholders.
- Label partial Swift support honestly.
- Add full metadata parsing only after ObjC/C++/kernel foundations.

### Risk: KDK/dSYM Mismatch Pollution

Wrong symbols can make analysis worse than no symbols.

Mitigation:

- Match by UUID/build/architecture.
- Require explicit exploratory mode for mismatch.
- Mark imported facts by profile.
- Make profile switch invalidate stale imported facts.

### Risk: Shared-Cache Memory Blowup

Cache-wide indexes can explode if every image is fully projected.

Mitigation:

- Build lightweight cache-level indexes first.
- Project images lazily.
- Page lists.
- Store compact cross-image edges.
- Keep cancellation and progress visible.

### Risk: Kernel Format Drift

Kernel collections and filesets change across OS generations.

Mitigation:

- Model source/member relationships generically.
- Keep parser errors typed.
- Add fixtures per OS generation only when legally available.
- Do not infer support for untested generations.

## Completion Audit Questions

Before claiming this spec implemented, answer with code and test evidence:

- Can a selector route list implementers and callers across a shared cache?
- Can a message-send instruction explain selector, receiver, and confidence?
- Can a class route link methods to implementation procedures?
- Can a C++ vtable route show slots and targets?
- Can a virtual callsite list possible method targets?
- Can a kernel collection route show members and kexts?
- Can a KDK/dSYM profile reject mismatched symbols?
- Can a panic frame route into a symbolicated function?
- Are all names/types/fixups/runtime edges provenance-bearing?
- Are large lists paged and long analyses cancellable?

If not, Damsel may have useful Apple metadata features, but it is not yet close
to Hopper in the Apple runtime and XNU-style system-binary lane.
