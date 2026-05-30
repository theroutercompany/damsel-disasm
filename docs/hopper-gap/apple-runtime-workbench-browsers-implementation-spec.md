# Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec

Status: implementation-grade product, UX, API, and architecture specification.
This is not an implementation claim.

Purpose: define the native Workbench browser suite that turns Damsel's Apple
runtime object model into a Hopper-competitive analyst experience for
Objective-C, Objective-C++, C++, Swift-adjacent symbols, dyld shared caches,
driver caches, kernel collections, KDK/dSYM enrichment, IOKit, XNU interfaces,
and panic-log navigation.

This spec sits above the model layer. It assumes that the project kernel,
route system, command system, runtime object store, type/symbolication store,
and performance scheduler exist or are being built from the companion specs.
Its job is to specify how those systems become a fast, polished, discoverable
native app experience that makes Damsel's Apple niche visible in the first
minutes of use.

## Companion Documents

Architecture companions:

- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Apple Systems Specialization Plan](apple-systems-specialization-plan.md)
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec](swift-runtime-metadata-implementation-spec.md)
- [Apple SDK, Header, And Semantic Packs Implementation Spec](apple-sdk-semantic-packs-implementation-spec.md)
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)

Validation companions:

- [Corpus, Benchmark, And Regression Plan](corpus-benchmark-regression-plan.md)
- [Quality And Validation Plan](quality-validation-plan.md)
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md)
- [Implementation Epic Breakdown](implementation-epic-breakdown.md)

## Source Evidence Used

Hopper public surface:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/HopperDebuggerServer/>

Apple and ABI references:

- <https://developer.apple.com/documentation/objectivec/objective-c-runtime>
- <https://developer.apple.com/documentation/foundation/objective-c-runtime-utilities>
- <https://developer.apple.com/documentation/apple-silicon/debugging-a-custom-kernel-extension>
- <https://developer.apple.com/documentation/driverkit/debugging-and-testing-system-extensions>
- <https://developer.apple.com/documentation/kernel/mach-o>
- <https://github.com/apple-oss-distributions/objc4>
- <https://github.com/apple-oss-distributions/xnu>
- <https://github.com/apple-oss-distributions/dyld>
- <https://itanium-cxx-abi.github.io/cxx-abi/abi.html>

Current Damsel files inspected:

- `damsel-core/src/model.rs`
- `damsel-macho/src/objc.rs`
- `damsel-macho/src/disasm.rs`
- `damsel-app/Sources/DamselAppCore/Models/DamselModels.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- `damsel-app/Sources/DamselAppCore/Views/ContentView.swift`
- `damsel-app/Sources/DamselAppCore/Views/InstructionSurfaceView.swift`
- `damsel-app/Sources/DamselAppCore/Views/InspectorPane.swift`
- `damsel-app/Sources/DamselAppCore/Views/ControlPane.swift`

## Evidence Summary

Hopper evidence:

- Hopper markets Objective-C and Swift support as a first-class feature.
- Hopper advertises extraction of Objective-C selectors and messages.
- Hopper presents synchronized assembly, control-flow graph, pseudocode, and
  inspector views.
- Hopper's tutorial says automatically generated tags include Objective-C
  class or category tags for implementations.
- Hopper's tutorial emphasizes symbol and string filtering, tag scopes, the
  inspector, references, comments, colors, and navigation stack behavior.
- Hopper's public pages emphasize native Cocoa polish, keyboard shortcuts,
  debugger support, Python scripting, MCP, and SDK extensibility.

Apple evidence:

- Apple's Objective-C runtime documentation identifies classes, selectors,
  methods, method implementations, ivars, properties, protocols, type
  encodings, and `objc_msgSend` family functions as runtime concepts.
- Apple's Objective-C utilities expose class and selector string conversion,
  which reinforces that class and selector names are core user-facing runtime
  identifiers.
- Apple's kernel debugging documentation describes KDK use, kernel panic core
  inspection, LLDB kernel debugging, and KDK search paths.
- Apple's system extension documentation notes entitlement/signing
  requirements and arm64e/PAC relevance for kexts on Apple silicon.
- Apple's Mach-O kernel documentation identifies Mach-O runtime interfaces as
  kernel-facing artifacts.
- Apple's open-source `objc4`, `xnu`, and `dyld` repositories provide primary
  source-grounded names and structures for deeper recognizers.
- The Itanium C++ ABI specifies RTTI, virtual table, implicit `this`, thunks,
  dynamic casts, virtual calls, and type-info name behavior that matter for
  Apple Clang C++ and ObjC++ analysis.

Current Damsel evidence:

- `ObjcMetadata` already records class names, selectors, method names, image
  flags, pointer refs, classes, protocols, and categories.
- `ObjcClassRecord` already has class, superclass, metaclass, method list,
  property list, protocol list, ivar list, method, property, ivar, and adopted
  protocol fields.
- `ObjcCategoryRecord` already distinguishes runtime-list categories from
  symbol-synthesized categories.
- `ObjcMethodRecord` already carries owner kind, class-method state, selector,
  implementation, and type encoding.
- `DisassemblyAnalysis` already summarizes ObjC selector, class, method-list,
  and message-send uses.
- `DisassemblyCacheLinkUse` already records provider/importer-style cache link
  information.
- Swift `DamselModels` currently expose upload/disassembly responses, sections,
  instructions, references, recovered values, analysis summaries, and basic
  blocks, but no project-backed runtime browser models.

Inference:

- Damsel has useful Apple facts but lacks the app layer that turns them into a
  runtime workbench: persistent indexes, domain navigators, route-backed
  browser pages, confidence buckets, relationship graphs, sidecar status,
  analysis job controls, and Apple-specific command families.

## Product Thesis

Damsel should not ask Apple systems analysts to start from raw sections,
symbols, and instruction windows. It should recognize when the source is an
Apple runtime artifact and present the appropriate domain browser immediately.

For a Hopper-competitive Apple niche, "open binary" should quickly become:

- "open this ObjC class"
- "show this selector's implementers"
- "show likely senders of this message"
- "open this dyld cache image"
- "jump from import to provider implementation"
- "open this C++ class and its vtable"
- "show likely IOKit external methods"
- "open this kernel collection member"
- "attach the matching KDK"
- "symbolicate this panic frame"
- "show why this target is only a candidate"

The browser suite is the product expression of Damsel's niche. It should make
Apple runtime knowledge visible, fast, searchable, and evidence-backed.

## Goals

- Define a native Workbench browser suite for Apple runtime and XNU workflows.
- Specify the UX for ObjC, C++/ObjC++, dyld cache, kernel, KDK, IOKit, and panic
  views.
- Specify the local API contracts required for browser pages, lists, filters,
  relationship graphs, and inspectors.
- Map current Damsel facts into future browser indexes without rewriting the
  existing parser stack.
- Define performance budgets for large caches, large frameworks, and kernel
  collections.
- Define error, ambiguity, stale-data, and provenance behavior.
- Define implementation phases and validation gates.

## Non Goals

- No claim that Damsel already has these browsers implemented.
- No generic PE/ELF runtime browser work.
- No full Swift metadata reconstruction in the first browser release.
- No debugger UI beyond route compatibility and future handoff points.
- No binary patching or live kernel modification.
- No private Apple symbol acquisition beyond user-provided KDK/dSYM/local
  symbols and legally accessible artifacts.

## Browser Suite Overview

Target browser families:

- Apple Project Overview
- dyld Cache Browser
- Image/Framework Browser
- ObjC Browser
- ObjC Dispatch Browser
- C++/ObjC++ Browser
- IOKit Browser
- Swift-Adjacent Symbol Browser
- Kernel Collection Browser
- XNU Interface Browser
- KDK/dSYM Symbolication Browser
- Panic Browser
- Runtime Relationship Graph
- Runtime Problem Browser

These browsers are not separate apps. They are route-backed Workbench pages
inside the same project/session model.

## Workbench Placement

Main layout:

```text
Toolbar
  project/source status
  global search
  command palette
  analysis jobs
  sidecar/KDK status

Left navigator
  Project
  Sources
  Cache
  Images
  Functions
  Strings
  ObjC
  C++
  IOKit
  Kernel
  Panic
  Problems

Center workspace
  browser page
  assembly
  graph
  hex
  source lines
  relationship graph

Right inspector
  selected object summary
  evidence/provenance
  xrefs
  type/signature
  commands
  problems

Bottom drawers
  search results
  jobs
  activity
  problems
```

Principle:

- The user can always switch from a browser object to assembly, graph, hex, or
  source-line view without losing the selected route.

## Browser Page Pattern

Every runtime browser page should follow a shared pattern:

```text
Page Header
  title
  subtitle
  route chip
  source/image/member context
  confidence/status badges
  primary commands

Summary Strip
  counts
  warnings
  sidecar status
  analysis freshness

Main Sections
  tables
  relationship cards
  graph preview
  evidence buckets
  problems

Inspector Integration
  provenance
  xrefs
  types/signatures
  raw records
  commands
```

Shared controls:

- filter field
- scope picker
- confidence picker
- source/image/member picker
- sort menu
- columns menu
- export selection
- copy route
- show in assembly
- show in graph
- show in hex

Shared states:

- empty
- loading
- partial
- stale
- ambiguous
- sidecar missing
- unsupported
- error with recovery actions

## Apple Project Overview

### Purpose

The project overview is the first orientation surface after intake. It should
answer:

- What did I open?
- What Apple artifact type is this?
- What runtime systems were found?
- What sidecars are attached or missing?
- What analysis is complete?
- What should I do next?

### Page Header

Fields:

- project name
- artifact kind
- architecture
- platform
- OS/build where known
- source count
- active symbolication profile
- analysis state

Artifact kinds:

- standalone Mach-O
- universal Mach-O
- app bundle
- dyld shared cache
- projected cache image
- driver cache
- kernelcache
- kernel collection
- KDK bundle
- dSYM bundle
- panic log
- extracted Apple asset root

### Summary Cards

Cards:

- Sources
- Images
- Functions
- Strings
- ObjC
- C++
- Swift names
- Cache links
- Kernel members
- KDK/dSYM status
- Panic logs
- Problems
- Jobs

Each card includes:

- count
- freshness
- route
- primary command
- warning badge where needed

### Suggested Actions

Examples:

- Analyze selected image
- Build runtime indexes
- Open ObjC browser
- Open C++ browser
- Open cache image map
- Attach dSYM
- Attach KDK
- Attach panic log
- Import local symbols
- Show unresolved runtime targets
- Export project summary

### API

```text
GET /api/projects/{project_id}/apple/overview
```

Response fields:

- project summary
- artifact summary
- browser availability
- counts
- sidecar state
- active jobs
- top problems
- suggested commands

### Performance

Budget:

- overview shell under 300 ms for warm project
- source/artifact classification under 500 ms for normal local files
- summary counts incrementally update as jobs finish
- no overview card waits on deep ObjC/C++/kernel analysis

## dyld Cache Browser

### Purpose

The cache browser turns a dyld shared cache into a navigable OS image map. It
should avoid the "one enormous file" mental model and present images,
dependencies, imports, providers, ObjC objects, strings, and analysis status as
first-class objects.

### Cache Home

Header fields:

- cache UUID
- architecture
- platform
- member count
- image count
- mapping count
- symbols sidecar availability
- local symbols status
- source graph route

Summary sections:

- top frameworks
- images with ObjC metadata
- images with many imports
- images with unresolved providers
- images with analysis errors
- subcache/member map

### Image List

Columns:

- install name
- basename
- image id
- base cache vmaddr
- image size
- member/subcache
- dependency count
- dependent count
- export count
- import count
- ObjC class count
- selector count
- analysis status
- problem count

Filters:

- basename/install name
- framework path prefix
- member/subcache
- has ObjC
- has Swift names
- has C++ names
- has local symbols
- has unresolved imports
- analyzed/not analyzed

Commands:

- Open Image
- Analyze Image
- Show Dependencies
- Show Dependents
- Show Imports
- Show Exports
- Show ObjC
- Show Provider Chain
- Project Image
- Compare With Other Cache
- Copy Install Name
- Copy Route

### Image Detail Page

Sections:

- Identity
- Segments and sections
- Dependencies
- Dependents
- Imports
- Exports
- Reexports
- Provider/importer edges
- ObjC classes/categories/selectors
- C++ symbols/vtables where known
- Strings
- Functions
- Analysis jobs
- Problems

### Provider/Importer View

Questions answered:

- Which image provides this symbol?
- Which images import it?
- Is it reexported?
- What chain resolves the symbol?
- Can I jump to implementation?
- Is the provider only a candidate?

Rows:

- symbol
- importing image
- target dylib
- provider image
- provider member
- reexport path
- resolved route
- confidence
- problem state

### API

```text
GET /api/projects/{project_id}/cache/summary
GET /api/projects/{project_id}/cache/images
GET /api/projects/{project_id}/cache/images/{image_id}
GET /api/projects/{project_id}/cache/images/{image_id}/dependencies
GET /api/projects/{project_id}/cache/images/{image_id}/dependents
GET /api/projects/{project_id}/cache/images/{image_id}/imports
GET /api/projects/{project_id}/cache/images/{image_id}/exports
GET /api/projects/{project_id}/cache/symbol-providers
GET /api/projects/{project_id}/cache/symbol-importers
GET /api/projects/{project_id}/cache/reexports
```

Paging:

- all large lists require cursor pagination
- list responses include result version
- filters are explicit query fields, not ad hoc client-side scans

### Current Damsel Integration

Current assets:

- shared-cache inventory
- projected images
- provider/importer/reexport commands
- `DisassemblyCacheLinkUse`
- cache symbolication result types

Migration:

1. Reuse existing cache query engine for read-only browser pages.
2. Add route ids to images, imports, exports, providers, and reexports.
3. Persist cache source/member/image records in project database.
4. Convert current CLI outputs into paged project API responses.
5. Add Workbench cache browser backed by the same APIs.

## Image And Framework Browser

### Purpose

An image page should be the bridge between source/container context and
language/runtime context.

For standalone Mach-O:

- image is the primary source

For dyld cache:

- image is a member view over shared source bytes

For kernel collection:

- member/kext is a kernel image-like source

### Image Overview

Header:

- image name
- install name or bundle id
- architecture
- source kind
- vmaddr range
- file/member offset range
- UUID
- platform
- analysis state

Sections:

- load commands
- segments/sections
- imports/exports
- function starts
- ObjC metadata
- Swift names
- C++ symbols
- strings
- entrypoints
- warnings

### Runtime Readiness

The image page should show which runtime browsers are meaningful:

```text
ObjC: 421 classes, 13,022 selectors, index ready
C++: 74 demangled classes, vtable scan pending
Swift: names demangled, metadata scan disabled
Kernel: not applicable
Debug info: dSYM missing
```

### Commands

- Analyze Image
- Build Function Index
- Build ObjC Index
- Build C++ Index
- Build String Index
- Attach dSYM
- Export Header Draft
- Open In Assembly
- Open In Graph
- Open In Hex

## ObjC Browser

### Purpose

The ObjC browser should make classes, categories, protocols, selectors,
methods, and message-send callsites primary navigation objects.

It should not simply render current `ObjcMetadata` arrays. It should build
searchable, route-backed runtime indexes and expose confidence for dynamic
dispatch.

### ObjC Home

Header:

- source scope
- image scope
- class count
- category count
- protocol count
- selector count
- method count
- message-send candidate count
- unresolved message-send count
- type-encoding parse status

Cards:

- Classes
- Categories
- Protocols
- Selectors
- Methods
- Message Sends
- Unresolved Dispatch
- Header Drafts
- Problems

Quick filters:

- current image
- all cache images
- userland only
- frameworks only
- categories only
- protocols only
- unresolved methods
- missing type encodings
- dynamic dispatch candidates

### Class List

Columns:

- class name
- image/source
- superclass
- category count
- protocol count
- instance method count
- class method count
- property count
- ivar count
- message sender count
- problem count
- confidence

Sorts:

- name
- source/image
- method count
- caller count
- problem severity
- analysis freshness

### Class Detail

Sections:

- identity
- superclass chain
- subclass candidates
- adopted protocols
- categories
- instance methods
- class methods
- properties
- ivars
- class refs
- selector refs
- message sends
- IMP functions
- generated header preview
- provenance
- problems

Class header fields:

- class route
- metadata address
- class object pointer
- metaclass pointer
- superclass pointer/name
- source/image
- symbolication profile
- name source
- confidence

### Category Detail

Sections:

- category name
- target class
- source/image
- runtime-list backing status
- symbol-synthesized status
- methods
- class methods
- properties
- adopted protocols
- provenance
- problems

Important distinction:

- runtime-list categories are stronger evidence
- symbol-synthesized categories are useful but lower-confidence

### Protocol Detail

Sections:

- protocol name
- adopted-by classes
- required instance methods
- required class methods
- optional instance methods
- optional class methods
- properties
- references
- type encodings
- provenance

### Selector Detail

Purpose:

- answer "what is this selector, who implements it, and who sends it?"

Sections:

- selector text
- string/ref locations
- implementers grouped by class/category/protocol/image
- direct metadata references
- likely message-send callsites
- ambiguous callsites
- manual bindings
- related selectors
- type-encoding candidates
- provenance
- problems

Implementer row fields:

- owner class/category/protocol
- method kind
- image/source
- IMP route
- type encoding
- signature candidate
- confidence
- provenance

Caller row fields:

- instruction route
- function/procedure
- image/source
- receiver evidence
- selector evidence
- resolved target candidate
- confidence
- unresolved reason

### Method Detail

Sections:

- owner
- selector
- class/instance method
- implementation address
- function/procedure route
- type encoding
- parsed signature
- callers
- overrides/category context
- references
- graph preview
- source-line info if available
- comments/tags
- provenance

### Message Send Detail

Purpose:

- explain dynamic dispatch instead of pretending every target is known.

Fields:

- instruction route
- callee import or stub
- selector candidate
- receiver candidate
- receiver source
- selector source
- target implementer candidates
- confidence bucket
- unresolved reason
- manual binding state

Confidence buckets:

- exact metadata call
- strong receiver and selector
- selector known, receiver candidate
- receiver known, selector unknown
- import-only `objc_msgSend`
- weak pattern
- unresolved

Commands:

- Show Selector
- Show Implementers
- Show Senders
- Bind Target
- Mark Unresolved Reason
- Add Comment
- Open Function
- Open Graph
- Copy Dispatch Summary

### Header Draft View

Purpose:

- make ObjC metadata useful without claiming perfect source recovery.

Sections:

- generated interface
- known methods
- properties
- ivars
- protocols
- categories
- unknown types
- conflicts
- source facts

Rules:

- header draft is evidence, not source recovery
- unknown types remain explicit
- conflicting type encodings are shown
- user edits are transactions
- export includes provenance summary

### ObjC API

```text
GET /api/projects/{project_id}/objc/summary
GET /api/projects/{project_id}/objc/classes
GET /api/projects/{project_id}/objc/classes/{class_id}
GET /api/projects/{project_id}/objc/categories
GET /api/projects/{project_id}/objc/protocols
GET /api/projects/{project_id}/objc/selectors
GET /api/projects/{project_id}/objc/selectors/{selector_id}
GET /api/projects/{project_id}/objc/methods/{method_id}
GET /api/projects/{project_id}/objc/message-sends
GET /api/projects/{project_id}/objc/message-sends/{message_id}
POST /api/projects/{project_id}/objc/message-sends/{message_id}/bind
GET /api/projects/{project_id}/objc/header-drafts/{class_id}
```

### Current Damsel Integration

Current seeds:

- `ObjcMetadata.class_names`
- `ObjcMetadata.selector_names`
- `ObjcMetadata.method_names`
- `ObjcPointerRef`
- `ObjcClassRecord`
- `ObjcProtocolRecord`
- `ObjcCategoryRecord`
- `ObjcMethodRecord`
- `ObjcPropertyRecord`
- `ObjcIvarRecord`
- `DisassemblyObjcUse`
- `DisassemblyObjcReferenceSummary`

Needed refactors:

- move from per-image arrays to project-wide runtime object ids
- normalize selectors across images
- link methods to procedure/function records
- convert disassembly ObjC uses into indexed callsite records
- add confidence/provenance to callsites
- add route ids to every object
- add paged list APIs
- add Workbench browser models

## ObjC Dispatch Browser

### Why Separate From ObjC Browser

The ObjC browser describes metadata objects. The dispatch browser describes
callsite evidence. Keeping them separate prevents Damsel from overstating
message targets.

### Dispatch Home

Cards:

- total `objc_msgSend` calls
- selector-known calls
- receiver-known calls
- target-bound calls
- ambiguous calls
- unresolved calls
- manual bindings
- top unresolved selectors

### Dispatch Table

Columns:

- instruction
- function
- image/source
- selector
- receiver
- target candidates
- confidence
- evidence
- unresolved reason
- manual binding

Filters:

- selector text
- class candidate
- current image
- unresolved only
- exact only
- manually bound
- weak pattern

### Evidence Panel

For each callsite:

- register/value facts
- immediate load facts
- selector ref source
- class ref source
- import/stub target
- surrounding instructions
- recovered receiver candidate
- candidate implementers
- rejected candidates
- missing evidence

### Manual Binding

Manual binding transaction:

- target callsite route
- selected method route
- reason
- confidence override
- optional note

Undo:

- removes binding
- restores analyzer-derived state
- invalidates affected call graph facts

## C++ And ObjC++ Browser

### Purpose

The C++ browser should expose Apple Clang C++ and ObjC++ structure:

- mangled/demangled names
- namespaces
- classes
- constructors
- destructors
- methods
- RTTI
- vtables
- virtual callsites
- thunks
- inheritance candidates
- ObjC bridge/context where present

This is crucial for IOKit, WebKit-style C++, system frameworks, and ObjC++
glue code.

### C++ Home

Cards:

- demangled symbols
- class candidates
- vtables
- RTTI objects
- constructors
- destructors
- virtual callsites
- thunks
- unresolved virtual calls
- ObjC++ bridge evidence
- IOKit candidates
- problems

Scopes:

- current image
- all project
- all cache images
- kernel members
- userland only
- kernel only

### Demangled Symbol List

Columns:

- demangled name
- mangled name
- kind
- namespace
- class
- function route
- source/image
- symbol source
- signature candidate
- confidence

Kinds:

- function
- method
- constructor
- destructor
- thunk
- vtable symbol
- typeinfo symbol
- typeinfo name
- guard variable
- static data
- unknown C++ symbol

### Class Detail

Sections:

- class identity
- namespaces
- source/image
- vtables
- typeinfo
- base candidates
- derived candidates
- methods
- constructors/destructors
- virtual methods
- virtual callsites
- thunks
- fields/placeholders
- ObjC bridge evidence
- IOKit overlay
- provenance
- problems

### Vtable Detail

Header:

- vtable route
- owner candidate
- address
- source/image/member
- typeinfo pointer
- address point
- slot count
- confidence

Slot table columns:

- slot index
- offset
- raw pointer
- stripped pointer if PAC applies
- function route
- demangled target
- thunk adjustment
- pure/deleted/null state
- xref count
- confidence

Commands:

- Open Slot Target
- Open Typeinfo
- Show Xrefs To Slot
- Rename Class
- Bind Owner Class
- Mark Slot Pure
- Copy Vtable Summary

### RTTI Detail

Sections:

- typeinfo object
- typeinfo name
- mangled name
- demangled name
- base-class records
- vtable links
- dynamic-cast references where detected
- provenance
- problems

### Virtual Callsite Detail

Fields:

- instruction route
- function/procedure
- receiver register/value evidence
- vtable load evidence
- slot index evidence
- candidate vtables
- candidate functions
- thunk adjustments
- confidence
- unresolved reason

Commands:

- Show Candidate Targets
- Bind Virtual Target
- Open Vtable
- Open Class
- Add Comment

### ObjC++ Bridge View

Purpose:

- show places where ObjC metadata and C++ structure overlap.

Examples:

- ObjC method IMP calls a C++ method
- C++ method references ObjC selector/class
- ObjC class owns C++ ivar-like pointer patterns
- block/lambda bridge names appear near ObjC dispatch
- IOKit C++ class appears in user client reachable from ObjC API

Rows:

- ObjC object route
- C++ object route
- evidence
- source/image
- confidence
- commands

### C++ API

```text
GET /api/projects/{project_id}/cxx/summary
GET /api/projects/{project_id}/cxx/symbols
GET /api/projects/{project_id}/cxx/classes
GET /api/projects/{project_id}/cxx/classes/{class_id}
GET /api/projects/{project_id}/cxx/vtables
GET /api/projects/{project_id}/cxx/vtables/{vtable_id}
GET /api/projects/{project_id}/cxx/rtti/{rtti_id}
GET /api/projects/{project_id}/cxx/virtual-calls
GET /api/projects/{project_id}/cxx/virtual-calls/{call_id}
POST /api/projects/{project_id}/cxx/virtual-calls/{call_id}/bind
GET /api/projects/{project_id}/cxx/objc-bridges
```

### Current Damsel Integration

Current seeds:

- symbol table names
- imports/exports
- function starts
- disassembly recovered values
- indirect targets
- jump tables
- arm64/arm64e decoding

Needed new subsystems:

- demangler wrapper
- symbol classifier
- RTTI parser
- vtable scanner
- pointer/fixup/PAC target integration
- virtual callsite recognizer
- thunk recognizer
- class hierarchy candidate builder
- browser indexes

## IOKit Browser

### Purpose

IOKit is a core reason Damsel should specialize in Apple system binaries. The
browser should identify likely IOKit classes, service/client relationships,
external method tables, user-client entrypoints, and vtable methods without
pretending every driver convention is fully solved.

### IOKit Home

Cards:

- class candidates
- service/provider candidates
- user client candidates
- external method tables
- method dispatch candidates
- kernel collection members with IOKit evidence
- unresolved patterns

### Class Candidate Detail

Sections:

- C++ class link
- source/member/kext
- inheritance candidates
- vtables
- lifecycle methods
- external methods
- user client relationships
- strings/properties
- xrefs
- confidence
- evidence

Lifecycle method hints:

- init
- free
- start
- stop
- open
- close
- newUserClient
- externalMethod
- getTargetAndMethodForIndex

### External Method Table View

Rows:

- selector/index
- function route
- scalar input/output counts
- struct input/output sizes
- flags where recoverable
- owning user client
- confidence
- source evidence

Commands:

- Open Handler
- Show User Client
- Export Method Table
- Add Manual Method Name

### IOKit API

```text
GET /api/projects/{project_id}/iokit/summary
GET /api/projects/{project_id}/iokit/classes
GET /api/projects/{project_id}/iokit/classes/{class_id}
GET /api/projects/{project_id}/iokit/user-clients
GET /api/projects/{project_id}/iokit/external-methods
GET /api/projects/{project_id}/iokit/external-methods/{method_id}
```

### Confidence Policy

Confidence buckets:

- exact symbol/type evidence
- KDK type evidence
- C++ vtable plus known method name
- method-table structure evidence
- string/protocol hints only
- weak heuristic

Weak heuristic results should never appear as definitive names. They appear as
candidates with evidence.

## Swift-Adjacent Browser

### Purpose

Damsel should recognize Swift-related symbols and names without promising full
Swift metadata reconstruction early.

### Scope

First browser version:

- demangled Swift symbol names where available
- Swift-related sections and strings
- method/function symbol grouping by module/type name
- bridge evidence between Swift symbols and ObjC selectors where visible
- unresolved Swift metadata problems

The dedicated
[Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec](swift-runtime-metadata-implementation-spec.md)
expands this placeholder into the full parser, object model, Workbench store,
API, CLI, diagnostics, and validation plan. This browser should consume those
Swift project facts rather than implementing a separate Swift-only UI model.

Out of first version:

- full Swift metadata graph
- full generic type reconstruction
- complete Swift decompilation

### Swift Home

Cards:

- demangled names
- modules
- nominal type names
- functions
- ObjC bridge references
- strings
- unresolved symbols

### Swift Symbol Detail

Fields:

- mangled name
- demangled name
- module
- entity kind
- function route
- source/image
- signature candidate
- related ObjC/C++ objects
- confidence
- problems

### API

```text
GET /api/projects/{project_id}/swift/summary
GET /api/projects/{project_id}/swift/symbols
GET /api/projects/{project_id}/swift/symbols/{symbol_id}
GET /api/projects/{project_id}/swift/modules
```

## Kernel Collection Browser

### Purpose

The kernel browser should make XNU-style artifacts feel like structured
systems, not anonymous Mach-O blobs.

It should support:

- kernelcache
- kernel collections
- fileset-style members
- kexts
- driver cache where available
- KDK/dSYM enrichment
- IOKit overlays
- panic route mapping

### Kernel Home

Header:

- source kind
- architecture
- OS/build
- UUID
- collection/member count
- KDK status
- dSYM status
- symbolication profile
- analysis status

Cards:

- Members
- Kexts
- IOKit Classes
- Syscalls
- Mach Traps
- MIG Routines
- Panic Logs
- KDK Symbols
- Problems

### Member List

Columns:

- member name
- bundle id
- UUID
- vmaddr range
- size
- symbol status
- KDK match status
- function count
- IOKit class count
- problem count

Filters:

- member name
- bundle id
- has KDK symbols
- has IOKit
- has panic frames
- unresolved symbols
- problem severity

### Member Detail

Sections:

- identity
- address ranges
- source/member mapping
- segments and sections
- symbols
- functions
- strings
- C++ classes
- IOKit classes
- syscalls/traps/MIG where relevant
- panic frames
- KDK/dSYM status
- problems

### Kext Detail

Sections:

- bundle id
- version
- executable/member route
- dependencies
- IOKit classes
- exported symbols
- imported symbols
- entitlements/signing metadata when available
- panic frames
- problems

### Address Conversion Panel

Kernel work requires explicit address spaces.

Panel fields:

- global kernel vmaddr
- member-local vmaddr
- file offset
- slid runtime address
- panic frame address
- KDK symbol address
- source route

Commands:

- Convert Address
- Copy Conversion
- Jump To Converted Route
- Set Panic Slide
- Explain Address Mapping

### Kernel API

```text
GET /api/projects/{project_id}/kernel/summary
GET /api/projects/{project_id}/kernel/collections
GET /api/projects/{project_id}/kernel/members
GET /api/projects/{project_id}/kernel/members/{member_id}
GET /api/projects/{project_id}/kernel/kexts
GET /api/projects/{project_id}/kernel/kexts/{kext_id}
POST /api/projects/{project_id}/kernel/address/convert
```

## XNU Interface Browser

### Purpose

The XNU interface browser should expose recognized kernel interface families:

- BSD syscalls
- Mach traps
- MIG routines
- IOKit user-client methods
- panic-related routines
- security and entitlement checks where evidence exists

### Syscall Browser

Rows:

- syscall number
- name
- handler route
- table source
- member/kext
- signature candidate
- KDK type source
- xrefs
- confidence

### Mach Trap Browser

Rows:

- trap number
- name
- handler route
- table source
- signature candidate
- confidence

### MIG Browser

Rows:

- subsystem
- routine id
- routine name
- handler route
- request/reply type evidence
- source file if KDK/DWARF available
- confidence

### API

```text
GET /api/projects/{project_id}/xnu/summary
GET /api/projects/{project_id}/xnu/syscalls
GET /api/projects/{project_id}/xnu/mach-traps
GET /api/projects/{project_id}/xnu/mig-routines
GET /api/projects/{project_id}/xnu/interfaces/{interface_id}
```

### Source Strategy

Recognizer sources:

- KDK symbols
- XNU open-source names
- table shape recognizers
- string references
- known section/data patterns
- manual user annotations

Rules:

- open-source XNU source names are recognizer hints, not proof for a specific
  shipped kernel unless matched by build/symbol evidence
- KDK/dSYM profile facts outrank generic open-source pattern hints
- table entries with weak evidence remain candidates

## KDK/dSYM Symbolication Browser

### Purpose

The symbolication browser should make sidecar status visible and actionable.
Users should not have to infer whether a kernel member, panic frame, or ObjC/C++
type is enriched by local symbols, dSYM, DWARF, or KDK.

### Symbolication Home

Cards:

- active profile
- attached dSYMs
- attached KDKs
- local symbols
- DWARF units
- source files
- line rows
- type records
- mismatches
- privacy state

### Candidate Matching View

Rows:

- candidate path
- source kind
- UUID
- build
- architecture
- match state
- mismatch reason
- import command

Match states:

- exact
- compatible
- suspicious
- mismatch
- missing UUID
- unsupported

### Profile Detail

Sections:

- active sources
- precedence order
- conflicts
- imported symbols
- imported types
- source files
- privacy policy
- invalidation impact
- problems

### API

```text
GET /api/projects/{project_id}/symbolication/summary
GET /api/projects/{project_id}/symbolication/profiles
GET /api/projects/{project_id}/symbolication/profiles/{profile_id}
GET /api/projects/{project_id}/symbolication/candidates
POST /api/projects/{project_id}/symbolication/attach-dsym
POST /api/projects/{project_id}/symbolication/attach-kdk
POST /api/projects/{project_id}/symbolication/profiles/{profile_id}/activate
```

### UX Rules

- matching explanations must be visible
- mismatched KDK/dSYM data is never silently used
- source paths are redacted in diagnostics by default
- long imports are cancellable jobs
- changing profile shows affected route counts

## Panic Browser

### Purpose

The panic browser should turn panic logs and crash-like kernel evidence into
route-backed analysis sessions.

### Panic Home

Cards:

- attached panic logs
- symbolicated frames
- unresolved frames
- detected kernel slide
- detected OS/build
- involved kexts
- top routines
- missing sidecars

### Panic Detail

Sections:

- raw panic summary
- parsed metadata
- kernel slide
- kext list
- frame table
- symbolication state
- unresolved frames
- related source routes
- export report

### Frame Table

Columns:

- frame index
- raw address
- unslid address
- member/kext
- symbol
- function route
- source line
- confidence
- unresolved reason

Frame commands:

- Open Function
- Open Assembly
- Open Graph
- Open Source Line
- Show Address Conversion
- Mark Frame Important
- Copy Frame Evidence

### Unresolved Frame Reasons

Reasons:

- no slide
- no matching member
- no KDK/dSYM
- symbolication profile inactive
- address outside known ranges
- PAC/stripped pointer unresolved
- source changed

### API

```text
GET /api/projects/{project_id}/panic/summary
GET /api/projects/{project_id}/panic/logs
GET /api/projects/{project_id}/panic/logs/{panic_id}
POST /api/projects/{project_id}/panic/attach
POST /api/projects/{project_id}/panic/{panic_id}/set-slide
GET /api/projects/{project_id}/panic/{panic_id}/frames
GET /api/projects/{project_id}/panic/{panic_id}/report
```

## Runtime Relationship Graph

### Purpose

The relationship graph should answer how Apple runtime objects relate:

- selector -> implementers -> callers
- class -> methods -> functions -> xrefs
- vtable -> slots -> functions -> callers
- user client -> external methods -> handlers
- cache image -> imports -> providers -> implementations
- panic frame -> member -> function -> source line

### Graph Modes

Modes:

- ObjC selector graph
- ObjC class graph
- C++ class/vtable graph
- IOKit user-client graph
- cache provider graph
- kernel member graph
- panic trail graph

### Node Types

- source
- image
- function/procedure
- ObjC class
- ObjC selector
- ObjC method
- C++ class
- vtable
- virtual callsite
- IOKit class
- external method
- kernel member
- kext
- syscall/trap/MIG routine
- panic frame
- source line
- problem

### Edge Types

- implements
- sends
- calls
- references
- inherits
- overrides
- owns
- contains
- imports
- provides
- reexports
- symbolicates
- resolves_to
- conflicts_with
- manually_bound

### Graph Requirements

- graph nodes are route-backed
- edge labels include confidence
- graph layout jobs are cancellable
- large graphs start with neighborhood view
- user can expand one hop at a time
- graph exports include provenance
- accessibility fallback is a list/tree view

### API

```text
GET /api/projects/{project_id}/runtime-graph
POST /api/projects/{project_id}/runtime-graph/layout
GET /api/projects/{project_id}/runtime-graph/layouts/{layout_id}
```

## Runtime Problem Browser

### Purpose

Runtime analysis creates ambiguity. The problem browser should make ambiguity
manageable instead of hiding it.

### Problem Kinds

ObjC:

- selector ref unresolved
- class ref unresolved
- method type encoding parse failed
- category target unresolved
- message receiver unknown
- message selector unknown
- multiple implementers

C++:

- demangle failed
- vtable owner ambiguous
- typeinfo pointer missing
- virtual call slot unknown
- thunk adjustment unresolved
- RTTI class hierarchy conflict

Kernel:

- KDK mismatch
- dSYM mismatch
- member address conversion failed
- syscall table unresolved
- MIG routine ambiguous
- panic frame unresolved

Cache:

- provider ambiguous
- import unresolved
- reexport cycle
- image projection failed
- local symbols missing

### Problem Page

Fields:

- problem id
- severity
- route
- source/image/member
- analyzer
- evidence
- expected data
- observed data
- recovery commands
- suppression state

### Recovery Commands

Examples:

- Attach KDK
- Attach dSYM
- Import local symbols
- Run C++ vtable scan
- Build ObjC dispatch index
- Bind message target
- Choose provider image
- Set panic slide
- Suppress problem
- Export evidence

## Data Model Additions

### Browser Summary Records

Each browser needs summary records separate from full object records:

```text
RuntimeBrowserSummary {
  project_id
  browser_kind
  scope
  result_version
  counts
  status
  stale_state
  top_problem_ids
  suggested_command_ids
}
```

### Runtime List Rows

Rows should be lightweight:

```text
RuntimeListRow {
  id
  route
  title
  subtitle
  source_context
  badges
  counts
  confidence
  problem_count
  sort_keys
}
```

### Runtime Detail Records

Detail records can include:

- object identity
- related object summaries
- paged child lists
- provenance summary
- command availability
- problem ids
- graph seed

Do not inline every child object into every detail response.

### Browser Query Scope

```text
RuntimeQueryScope {
  project_id
  source_id?
  image_id?
  member_id?
  symbolication_profile_id?
  route_context?
  confidence_filter?
  problem_filter?
}
```

## Workbench Swift Architecture

### New Stores

Suggested stores:

```text
AppleRuntimeStore
CacheBrowserStore
ImageBrowserStore
ObjCBrowserStore
ObjCDispatchStore
CxxBrowserStore
IOKitBrowserStore
SwiftSymbolStore
KernelBrowserStore
SymbolicationBrowserStore
PanicBrowserStore
RuntimeGraphStore
RuntimeProblemStore
```

Shared dependencies:

- `RouteStore`
- `SelectionStore`
- `NavigationStore`
- `CommandDispatcher`
- `JobStore`
- `ProblemStore`
- `ActivityStore`
- `DamselAPIClient`

### Store Rules

- browser stores own paged query state
- route store owns active route
- selection store owns selected runtime object
- command dispatcher owns commands
- job store owns background jobs
- problem store owns global problem drawer
- browser stores do not own project transactions directly

### Swift Models

Add models:

```text
RuntimeBrowserSummary
RuntimeListPage
RuntimeListRow
RuntimeObjectRoute
RuntimeBadge
RuntimeConfidence
RuntimeProblemSummary
ObjCClassDetail
ObjCSelectorDetail
ObjCMessageSendDetail
CxxClassDetail
CxxVtableDetail
KernelMemberDetail
PanicFrameDetail
SymbolicationProfileSummary
```

### View Components

Shared components:

- `RuntimeBrowserShell`
- `RuntimeSummaryStrip`
- `RuntimeListView`
- `RuntimeDetailHeader`
- `RuntimeEvidencePanel`
- `RuntimeProblemBanner`
- `RuntimeCommandBar`
- `ConfidenceBadge`
- `SourceContextChip`
- `SidecarStatusChip`
- `RouteBreadcrumbs`

Domain views:

- `ObjCBrowserView`
- `ObjCSelectorView`
- `ObjCDispatchView`
- `CxxBrowserView`
- `CxxVtableView`
- `IOKitBrowserView`
- `CacheBrowserView`
- `KernelBrowserView`
- `PanicBrowserView`
- `SymbolicationBrowserView`

## Rust Architecture

### Browser Services

New service modules:

```text
runtime_browser_service
objc_browser_service
objc_dispatch_service
cxx_browser_service
iokit_browser_service
cache_browser_service
kernel_browser_service
symbolication_browser_service
panic_browser_service
runtime_graph_service
```

Each service owns:

- query construction
- page cursor generation
- row summaries
- detail assembly
- provenance summaries
- command suggestions
- problem lookup

Services do not own low-level parsing. They consume project facts.

### Indexes

Indexes:

- selector -> implementers
- selector -> callsites
- class -> methods/categories/protocols
- method -> procedure
- function -> runtime objects
- symbol -> C++ object
- vtable -> slots
- function -> virtual callsites
- image -> imports/exports/providers
- member -> kexts
- panic frame -> route
- route -> problems

### Caching

Cache:

- browser summary counts
- paged list row materialization
- detail record expensive sections
- runtime graph neighborhoods
- symbolication candidate matches

Invalidation:

- project source changed
- analyzer version changed
- sidecar/profile changed
- user transaction changed relevant object
- new analysis job completed

## Command Integration

Command families:

```text
objc.open_browser
objc.show_class
objc.show_selector
objc.show_implementers
objc.show_senders
objc.bind_message_send
cxx.open_browser
cxx.show_class
cxx.show_vtable
cxx.show_virtual_call_targets
cxx.bind_virtual_target
iokit.open_browser
iokit.show_user_client
iokit.show_external_methods
cache.open_browser
cache.show_image
cache.jump_to_provider
kernel.open_browser
kernel.show_member
kernel.convert_address
symbolication.attach_dsym
symbolication.attach_kdk
panic.attach_log
panic.show_frame
```

Rules:

- every command takes route context
- every mutating command creates a transaction
- long commands create jobs
- disabled commands explain missing evidence
- commands appear in palette and context menus

## Performance Budgets

General:

- browser shell appears under 150 ms after route selection
- cached summary appears under 300 ms
- first page of any browser list appears under 200 ms warm
- cold list query for large project returns partial state under 500 ms
- filter keystroke to first page under 150 ms warm
- detail page header under 150 ms warm
- expensive detail sections load independently
- graph neighborhood under 500 ms for first 100 nodes
- sidecar import starts as job under 100 ms

Large dyld cache:

- cache image list is paged
- provider/importer queries are indexed
- ObjC selector search does not scan every image on UI thread
- image overview does not project every image

Large kernel collection:

- member list is paged
- KDK matching runs as job
- symbol import progress is visible
- panic frame symbolication can complete before full kernel analysis

## Empty And Partial States

Examples:

ObjC browser:

- "No Objective-C metadata found in this scope."
- "ObjC metadata exists, dispatch index not built."
- "Selector index ready, message-send analysis still running."

C++ browser:

- "No demangled C++ symbols found."
- "Vtable scan pending."
- "RTTI disabled or stripped; showing symbol-derived classes only."

Kernel browser:

- "This project is not a kernel artifact."
- "Kernel members found, KDK not attached."
- "Panic log attached, slide unresolved."

Cache browser:

- "Cache inventory ready, image projection not started."
- "Local symbols sidecar missing."

Partial states should include primary next actions.

## Privacy And Trust

Sensitive data:

- source paths
- private framework names
- proprietary symbols
- panic log content
- KDK paths
- dSYM paths
- raw strings from binaries

Rules:

- browser UI can show local private data to the user
- diagnostics redact by default
- evidence export requires explicit choice
- command activity logs store route ids and redacted source labels by default
- panic logs are treated as sensitive
- support bundles exclude raw binary data unless explicitly requested

## Implementation Phases

### Phase A: Browser Shell And Shared Components

Deliver:

- shared runtime browser shell
- runtime summary/list/detail model types
- route-backed browser selection
- browser command ids
- disabled reasons
- first Swift API client group

Validation:

- browser shell renders with synthetic summary
- command palette can open browser routes
- list pagination model snapshots
- empty/partial/error state snapshots

### Phase B: ObjC Browser From Existing Metadata

Deliver:

- ObjC summary endpoint
- class list endpoint
- selector list endpoint
- class detail endpoint
- selector detail endpoint
- method detail endpoint
- Workbench ObjC browser

Use existing:

- `ObjcMetadata`
- `ObjcClassRecord`
- `ObjcProtocolRecord`
- `ObjcCategoryRecord`
- `ObjcMethodRecord`

Validation:

- fixture ObjC class list snapshot
- selector implementer snapshot
- category source confidence snapshot
- Workbench class detail smoke

### Phase C: ObjC Dispatch Browser

Deliver:

- message-send callsite index
- confidence buckets
- evidence panel
- manual binding transaction
- caller/implementer cross-links

Use existing:

- `DisassemblyObjcUse`
- recovered value facts
- import/stub analysis

Validation:

- known selector callsite fixture
- unresolved receiver fixture
- manual binding undo/redo fixture
- Workbench dispatch detail smoke

### Phase D: Cache Browser Integration

Deliver:

- cache summary
- paged image list
- image detail
- provider/importer view
- cross-image route breadcrumbs

Use existing:

- shared-cache inventory
- provider/importer/reexport commands
- cache link analysis

Validation:

- synthetic cache image list snapshot
- provider/importer route snapshot
- Workbench image browser smoke

### Phase E: C++/ObjC++ Foundation Browser

Deliver:

- demangled symbol list
- class candidates
- vtable records
- vtable detail
- virtual callsite candidates

Validation:

- C++ fixture with vtable/RTTI
- multiple inheritance fixture
- virtual callsite fixture
- Workbench vtable detail smoke

### Phase F: IOKit Browser

Deliver:

- IOKit class candidates
- user-client candidates
- external method table candidates
- confidence/evidence panel

Validation:

- synthetic IOKit-style fixture
- KDK-backed sample if available
- weak heuristic does not present as fact

### Phase G: Kernel And KDK Browser

Deliver:

- kernel summary
- member/kext list
- member detail
- KDK/dSYM candidate matching view
- address conversion panel

Validation:

- synthetic fileset/kernel fixture
- KDK mismatch fixture
- address conversion tests
- Workbench kernel browser smoke

### Phase H: Panic Browser

Deliver:

- panic attach flow
- parsed panic summary
- frame table
- slide state
- frame-to-route mapping
- report export draft

Validation:

- synthetic panic log fixture
- missing slide fixture
- KDK matched frame fixture
- unresolved frame explanations

### Phase I: Runtime Relationship Graph

Deliver:

- graph neighborhoods for ObjC selector/class
- C++ vtable/class graph
- cache provider graph
- panic trail graph
- list fallback

Validation:

- graph JSON snapshots
- layout job cancellation
- accessibility fallback snapshots

## Acceptance Criteria

Documentation acceptance:

- browser suite is specified as a native app architecture, not just backend
  records
- ObjC, C++, ObjC++, IOKit, dyld cache, kernel, KDK, and panic workflows are
  represented
- current Damsel facts are mapped to browser migration steps
- API contracts exist for every browser family
- validation gates exist for every browser family

Implementation acceptance:

- ObjC browser works from current metadata fixtures
- selector pages show implementers and callers
- message-send details show evidence and confidence
- cache browser uses paged indexes
- C++ browser distinguishes demangled-symbol classes from RTTI/vtable facts
- IOKit browser marks weak heuristics clearly
- kernel browser shows member address conversions
- KDK/dSYM browser explains match state
- panic browser maps frames to routes when evidence exists
- every browser route can open assembly/graph/hex/source where applicable
- every mutation uses transactions
- every long operation is a job

Performance acceptance:

- first browser page loads without blocking full analysis
- large lists are paged
- cache and kernel summary pages do not scan full artifacts on UI thread
- selector and provider searches use indexes
- Workbench remains interactive during sidecar imports and graph layout

Truthfulness acceptance:

- ambiguous dispatch is labeled ambiguous
- weak IOKit heuristics are labeled weak
- missing KDK/dSYM is visible
- unmatched panic frames explain why
- Swift support is called Swift-adjacent until full metadata exists
- generated headers are drafts with provenance

## Risks And Mitigations

### Risk: Browser UI Overstates Analysis

Mitigation:

- confidence badges everywhere
- evidence panels for dynamic dispatch and virtual calls
- unresolved reasons are first-class
- weak heuristics separated from exact facts

### Risk: Browser Pages Become Slow On Huge Caches

Mitigation:

- paged APIs only
- indexed summaries
- lazy detail sections
- background jobs for expensive scans
- route shell appears before deep data

### Risk: Apple Domain Browsers Diverge From Generic Routes

Mitigation:

- every object is route-backed
- shared `RuntimeListRow`
- shared command dispatcher
- shared selection/navigation model
- shared problem and activity stores

### Risk: C++ And IOKit Analysis Becomes Guesswork

Mitigation:

- separate demangled names, RTTI, vtable, and heuristic overlays
- show evidence class
- require fixture tests for every promotion to higher confidence
- allow manual user bindings with provenance

### Risk: Kernel Work Requires Private Or Sensitive Data

Mitigation:

- sidecar attachment is explicit
- paths redacted in diagnostics
- KDK/dSYM match explanations visible
- panic logs treated as sensitive
- no automatic upload or external submission

## Summary

Damsel's strongest Hopper-competitive path is not broad format parity. It is a
native Apple systems workbench where runtime concepts are the primary
navigation units:

- ObjC selectors, classes, categories, protocols, methods, and message sends
- C++ classes, RTTI, vtables, thunks, and virtual calls
- ObjC++ bridge evidence
- IOKit classes, user clients, and external methods
- dyld cache images, providers, importers, and reexports
- kernel collection members, kexts, syscalls, Mach traps, and MIG routines
- KDK/dSYM profiles and panic frames

The current code already has enough seeds to begin: ObjC metadata records,
disassembly ObjC uses, cache links, shared-cache queries, symbols, sections,
and analysis summaries. The missing product layer is a set of route-backed,
paged, confidence-aware, command-integrated Workbench browsers that make those
facts searchable, inspectable, explainable, and fast.
