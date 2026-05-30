# Apple Systems Specialization Plan

Status: Product and architecture plan. This document describes how Damsel should
compete by being unusually good at Apple system binaries rather than becoming a
generic Hopper clone.

Purpose: define the niche. Hopper is broad: many file formats, architectures,
decompiler, debugger, scripting, SDK. Damsel's credible competitive path is
depth on macOS/iOS system artifacts: dyld shared caches, driver caches, kernel
collections, Objective-C, C++, Objective-C++, Swift symbols where relevant,
arm64e pointer authentication, chained fixups, KDK/dSYM symbolication, and
panic/backtrace workflows.

Implementation companion:
[Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
turns this product thesis into concrete object records, analysis passes, API
routes, CLI commands, Workbench surfaces, tests, and acceptance criteria.

Workbench browser companion:
[Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
turns the same Apple-specialization thesis into concrete native app browser
pages, paged lists, detail views, command families, relationship graphs,
confidence displays, runtime problem surfaces, and performance gates for ObjC,
C++/ObjC++, dyld cache, IOKit, kernel, KDK/dSYM, and panic workflows.

Artifact intake companion:
[Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
defines how the app should classify, register, expand, project, and enrich the
actual Apple artifacts that feed this specialization: dyld caches, driver
caches, kernel collections, KDKs, dSYMs, panic logs, app bundles, and
IPSW/OTA-derived asset roots.

Research companion:
[Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
grounds this plan in the current public Hopper SDK, release API, changelog, and
automation surface.

## Product Thesis

Damsel should make Apple system analysis feel native to the tool:

- opening a dyld shared cache should not feel like opening one giant file
- opening a kernel collection should reveal members/kexts immediately
- ObjC classes/selectors/methods should be navigation primitives
- C++ vtables/typeinfo should be first-class for IOKit and ObjC++
- KDK/dSYM attachment should be a core workflow
- panic logs should map back to kexts/functions/source when possible
- arm64e/PAC/chained fixups should affect xrefs and displayed targets

The advantage is not "supports Mach-O." The advantage is that Damsel knows the
Apple runtime and system-binary ecosystem well enough to answer questions a
generic disassembler leaves scattered across scripts and manual notes.

## User Workflows

### dyld Shared Cache Framework Investigation

User goal:

- inspect a system framework in a dyld shared cache
- follow imports to provider implementations
- inspect ObjC classes/selectors
- understand cross-image call relationships

Ideal flow:

1. Open shared cache.
2. Damsel shows image list quickly.
3. User searches for `Foundation` or a selector.
4. Image overview shows dependencies, dependents, exports, imports, ObjC
   classes, strings, and analysis status.
5. User opens a method/function.
6. Assembly, graph, inspector, xrefs, and feature map synchronize.
7. "Follow provider" jumps through import/reexport chains to implementation.
8. Cross-image breadcrumbs show where the user came from.

Required systems:

- shared-cache source registry
- image projection without duplicating bytes
- cache-wide symbol provider/importer index
- cache-wide ObjC selector/class index
- cross-image route model
- cache address/image address conversions
- partial analysis jobs per image

### Objective-C Message Investigation

User goal:

- answer "what implementations of this selector exist, and who calls them?"

Ideal flow:

1. Search selector.
2. Selector page lists implementers by class/category/image.
3. Caller list separates direct metadata refs, likely `objc_msgSend` callsites,
   super dispatch, and ambiguous candidates.
4. User opens a caller.
5. Inspector shows receiver evidence, selector evidence, confidence, and
   unresolved alternatives.
6. User can add a manual receiver/selector binding if analysis cannot prove it.

Required systems:

- selector object store
- method IMP to function links
- message-send recognizer
- recovered-value propagation
- callsite confidence model
- manual binding annotation
- caller/implementer xref index

### ObjC++ / IOKit C++ Investigation

User goal:

- understand C++ virtual dispatch and class relationships in Apple frameworks
  or kernel extensions.

Ideal flow:

1. Open binary/cache/kernel collection.
2. Damsel discovers vtables, typeinfo, demangled names, and likely inheritance.
3. C++ browser shows classes, vtables, methods, constructors, destructors, and
   virtual callsites.
4. IOKit recognizer overlays service/provider/client style names where
   evidence exists.
5. Virtual callsite inspector lists possible target slots and confidence.

Required systems:

- Itanium ABI demangler
- C++ symbol classifier
- RTTI parser
- vtable scanner
- constructor/destructor recognizers
- virtual callsite recognizer
- inheritance graph
- IOKit-specific heuristics
- type database integration

### Kernel Collection Triage

User goal:

- inspect an XNU-style kernel artifact, identify members/kexts, attach symbols,
  and navigate system interfaces.

Ideal flow:

1. Open kernel collection or kernelcache.
2. Damsel identifies fileset/kernel collection structure.
3. Project overview shows members, kexts, UUID/build, architecture, and KDK
   status.
4. User attaches KDK/dSYM.
5. Damsel verifies UUID/build match.
6. Symbols/types/source lines enrich the project.
7. User browses kexts, syscalls, Mach traps, MIG routines, IOKit classes, and
   panic-related symbols.

Required systems:

- fileset/kernel collection loader
- member address spaces
- global/member address conversion
- kext identity model
- KDK/dSYM discovery and UUID/build matching
- kernel symbol/type import
- syscall/trap/MIG recognizers
- IOKit integration

### Panic Log Symbolication

User goal:

- map a panic/backtrace to functions, kexts, and source context.

Ideal flow:

1. Attach panic log to project.
2. Damsel extracts kernel slide, frames, kext list, build metadata, and panic
   string.
3. Damsel matches frames to kernel collection members/functions.
4. KDK/dSYM symbols and line info enrich frames if available.
5. User can click a frame to jump to function/assembly/graph.
6. Damsel lists unresolved frames with specific missing evidence.

Required systems:

- panic log parser
- slide extraction
- frame parser
- kext UUID/build matcher
- address-space converter
- symbolication engine
- route generation

## Artifact Model

### dyld Shared Cache

Model:

- cache source
- cache UUID/architecture/platform/build
- subcache parts
- images
- install names
- dependencies/dependents
- exports/imports/reexports
- ObjC metadata
- local symbols sidecar, when present
- projected image views

Competitive features:

- image overview
- dependency graph
- provider/importer queries
- cache-wide selector/class search
- cross-image xrefs
- image diff later

### Driver Cache

Public Hopper changelog now mentions reading dylibs from driver cache. Damsel
should treat this as a sign that Apple system artifacts beyond the traditional
dyld cache matter.

Model:

- driver-cache source kind
- contained dylibs/members
- relation to kernel/system version
- symbol and dependency indexes

Initial scope:

- recognize and inventory if format support is tractable
- expose as source kind
- avoid pretending full analysis exists until fixtures prove it

### Kernel Collection / Fileset

Model:

- collection source
- fileset members
- global address space
- member address spaces
- kext identity
- segment/section maps
- symbol sources
- KDK/dSYM attachment
- debug/type import status

Competitive features:

- member/kext browser
- global/member address toggle
- KDK match status
- IOKit class browser
- syscall/trap/MIG browser
- panic log integration

### KDK And dSYM

Model:

- sidecar source
- UUID/build metadata
- matched target source/member
- symbols imported
- types imported
- line tables imported
- mismatch problems

Rules:

- never silently attach mismatched KDK/dSYM data
- exact UUID/build match wins
- user can attach mismatched source only in explicit exploratory mode
- imported names/types do not overwrite user annotations

## Runtime Object Stores

### Objective-C Store

Objects:

- class
- metaclass
- category
- protocol
- method
- property
- ivar
- selector
- method type encoding
- class ref
- selector ref
- message-send callsite

Indexes:

- class name -> class
- selector -> implementers
- selector -> callers
- class -> methods
- class -> categories
- protocol -> adopters
- IMP -> function
- function -> ObjC methods/selectors

Views:

- class hierarchy
- category overlays
- selector page
- method page
- callsite confidence inspector
- type encoding decoder

### C++ / ObjC++ Store

Objects:

- demangled symbol
- namespace/class/function
- constructor/destructor
- RTTI typeinfo
- vtable
- vtable slot
- virtual callsite
- inheritance edge
- template-ish demangled display

Indexes:

- demangled name -> object
- vtable address -> class
- function -> method
- callsite -> candidate slots
- typeinfo -> vtables
- class -> subclasses/superclasses

Views:

- class browser
- vtable view
- virtual callsite inspector
- inheritance graph
- mixed ObjC++ object page where ObjC and C++ facts meet

### Swift Boundary Store

Swift should not become the center of this pack, but Hopper's public work on
Swift demangling means Damsel should handle the obvious Apple-system cases.

Initial scope:

- Swift demangled symbol display
- Swift symbol search
- Swift-related function names in summaries
- Swift metadata placeholders where parser evidence exists

Non-goals for first pass:

- full Swift type metadata reconstruction
- generic Swift decompiler
- source-level Swift recovery

## arm64e, PAC, And Chained Fixups

Apple Silicon system analysis requires pointer semantics.

Required facts:

- raw pointer value
- interpreted target
- authentication/signing mode when known
- stripped target
- fixup source
- binding/import relation
- confidence
- failure reason when unresolved

PAC-aware views:

- call/branch target display
- ObjC IMP/selector trampoline display
- C++ vtable slot display
- fixup inspector
- problem list for unresolved authenticated targets

Rules:

- never silently erase raw pointer context
- display raw and interpreted values where useful
- PAC/fixup interpretation feeds xrefs, not only UI text

## Symbolication Profiles

Apple system analysis is version-sensitive. Damsel needs explicit profiles.

Profile fields:

- OS name/version/build
- architecture
- dyld cache UUID
- kernel collection UUID
- KDK path
- dSYM paths
- local symbols sidecar paths
- source availability
- imported type/symbol versions

Behavior:

- project can have one active profile per artifact family
- profile mismatch creates problems
- switching profile marks derived facts stale
- user annotations survive profile changes

## Apple-Specific Problem Types

Problems:

- missing split cache part
- local symbols sidecar missing
- provider image not found
- reexport chain unresolved
- ObjC metadata pointer outside mapped source
- selector string missing
- method IMP target invalid
- PAC target unresolved
- chained fixup unsupported
- kernel collection member overlap
- KDK UUID mismatch
- panic frame outside known member
- vtable target not executable
- Swift demangle failed

Recovery actions:

- attach sidecar
- choose symbolication profile
- mark function start
- add manual ref
- bind ObjC callsite
- attach KDK
- enter slide
- suppress known benign problem

## API Surface Needed

Source:

- list cache images
- list kernel members
- attach sidecar
- get symbolication profile
- set active profile

ObjC:

- list classes/selectors/methods
- get selector implementers
- get selector callers
- get class hierarchy
- bind message send

C++:

- list types/vtables
- get vtable
- get virtual callsite candidates
- get inheritance graph

Kernel:

- list members/kexts
- list syscalls/traps/MIG
- symbolicate panic
- get KDK status

PAC/fixups:

- get pointer fact
- get fixup refs
- list unresolved pointer problems

## Workbench Surfaces

### Apple Project Overview

Panels:

- sources
- artifact type
- OS/build/architecture
- analysis status
- sidecar status
- major counts
- top problems
- suggested next actions

Suggested next actions:

- Analyze selected image
- Attach local symbols
- Attach KDK
- Import dSYM
- Open ObjC browser
- Open kernel browser
- Symbolicate panic

### Cache Browser

Columns:

- install name
- image base
- size
- dependencies count
- dependents count
- exports/imports count
- ObjC classes/selectors count
- analysis status

Actions:

- open image
- show dependencies
- show dependents
- find provider
- show importers
- analyze image

### Kernel Browser

Columns:

- member/kext
- bundle id
- version
- address range
- symbol status
- IOKit classes
- problems

Actions:

- open member
- attach KDK
- show IOKit classes
- show syscalls/traps/MIG
- symbolicate panic

### Runtime Browser

Tabs:

- ObjC
- C++
- Swift symbols
- Kernel/IOKit

Rules:

- runtime browser filters should be fast
- each object has a route
- each object links to code/data/xrefs
- every ambiguous relationship exposes confidence

## Implementation Sequence

1. Generalize source registry for cache, driver cache, kernel collection,
   sidecars, and panic logs.
2. Add explicit address-space conversion service.
3. Lift current ObjC metadata into runtime object store.
4. Add cache-wide class/selector/image indexes.
5. Add symbolication profile model.
6. Add KDK/dSYM attachment and mismatch problems.
7. Add kernel collection/fileset inventory.
8. Add panic log parser and route generation.
9. Add C++ demangler and vtable/typeinfo discovery.
10. Add IOKit recognizers.
11. Add PAC/fixup pointer fact integration.
12. Add Workbench Apple project overview and runtime browsers.

## Acceptance Gates

Damsel is genuinely specialized for Apple systems when:

- dyld shared-cache workflows are not forced through standalone-image UI
- cross-image provider/importer navigation is primary and fast
- ObjC selector/class/method navigation is first-class
- kernel collection opening shows members/kexts before deep analysis finishes
- KDK/dSYM matching is explicit and safe
- panic frames route to functions/kexts when evidence exists
- C++/IOKit vtables and virtual callsites are inspectable
- PAC/chained fixups influence xrefs and target display
- unresolved Apple-system ambiguity appears as problems with recovery actions
- every Apple-system object has stable routes and API object ids
