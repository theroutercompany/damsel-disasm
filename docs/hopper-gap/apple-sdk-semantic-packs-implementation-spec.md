# Apple SDK, Header, And Semantic Packs Implementation Spec

Status: implementation-grade architecture spec.

Date of public-source pass: 2026-05-30.

Purpose: define the versioned semantic-pack subsystem that lets Damsel turn
Apple SDK headers, open-source Apple runtime/kernel/dyld material, user-provided
KDK/dSYM/source roots, and local framework metadata into searchable,
route-backed facts for macOS, iOS, dyld-cache, Objective-C, C++, ObjC++, Swift,
IOKit, DriverKit, and XNU analysis.

This is a companion to:

- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec](swift-runtime-metadata-implementation-spec.md)
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
- [Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md)
- [Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec](investigation-casebook-session-timeline-implementation-spec.md)

## Why This Exists

Hopper publicly presents a strong Apple-oriented reverse-engineering product:
native macOS UI, Objective-C and Swift awareness, control-flow graph,
decompiler, debugger, Python scripting, MCP, and SDK extensibility. Hopper's
tutorial also shows a document model that understands symbols, strings, tags,
comments, references, segments, sections, byte types, procedures, and
context-sensitive inspector panels.

Damsel's Apple niche should go beyond matching that general shape. The product
can become especially strong for system-framework and XNU-style work if it
understands the semantic material Apple already publishes or ships locally:

- SDK headers and module maps
- Objective-C runtime concepts and type encodings
- open-source `objc4` runtime structures
- open-source `xnu` headers, build macros, syscall/Mach/MIG/IOKit definitions
- open-source `dyld` shared-cache and kernel-collection code
- Swift ABI and metadata documentation
- KDK and dSYM debug sidecars
- local framework bundle metadata
- user-provided private headers or team-specific type packs

Existing Damsel docs already specify a type store, debug-info imports, runtime
object models, and Workbench browsers. This document specifies the missing
producer layer: how semantic knowledge is acquired, normalized, versioned,
indexed, invalidated, licensed, surfaced, and safely used without pretending
that a header declaration proves a fact about a specific stripped binary.

## Product Thesis

Damsel should treat Apple semantic packs as first-class project sidecars.

Opening a system framework, dyld shared-cache image, driver cache, kernel
collection, app binary, or panic log should not only parse bytes. It should
also answer:

- which SDK, KDK, Xcode, platform, and OS build best match this artifact?
- which public/private headers might describe functions, structs, Objective-C
  classes, C++ classes, enums, constants, syscalls, MIG routines, and IOKit
  user-client methods near this route?
- which declarations are directly matched to symbols, selectors, type
  encodings, debug info, or binary evidence?
- which declarations are only hints because the binary is stripped, optimized,
  version-skewed, or privately patched?
- which semantic packs are missing, stale, untrusted, restricted by license, or
  incompatible with the current source?
- how does a declaration influence search, inspector display, decompiler
  comments, AI context, reports, and automation?

The right competitive target is not "import headers once." The target is a
version-aware Apple semantic knowledge layer that can explain itself and stay
responsive on large SDKs and KDKs.

## Public Source Set

Hopper public sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page and public SDK listing:
  <https://www.hopperapp.com/download>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper Python type stubs:
  <https://pypi.org/project/types-hopper-disassembler/>
- Public Hopper scripts:
  <https://github.com/droe/hopper-scripts>

Apple and platform primary sources:

- Apple Open Source portal:
  <https://opensource.apple.com/>
- Apple `xnu` open-source distribution:
  <https://github.com/apple-oss-distributions/xnu>
- Apple `objc4` open-source distribution:
  <https://github.com/apple-oss-distributions/objc4>
- Apple `dyld` open-source distribution:
  <https://github.com/apple-oss-distributions/dyld>
- Apple Objective-C Runtime documentation:
  <https://developer.apple.com/documentation/objectivec/objective-c-runtime>
- Swift ABI type metadata documentation:
  <https://github.com/swiftlang/swift/blob/main/docs/ABI/TypeMetadata.rst>
- Swift ABI metadata header:
  <https://github.com/swiftlang/swift/blob/main/include/swift/ABI/Metadata.h>

Implementation technology references:

- Clang libclang tutorial:
  <https://clang.llvm.org/docs/LibClang.html>
- Clang AST introduction:
  <https://clang.llvm.org/docs/IntroductionToTheClangAST.html>
- libclang C API reference:
  <https://clang.llvm.org/doxygen/group__CINDEX.html>

## Public Evidence Summary

### Hopper Evidence

Hopper's public homepage emphasizes a native macOS app, Objective-C and Swift
support, Python scripting, MCP, and SDK extensibility. This implies that a
competitive Damsel cannot keep semantic knowledge trapped in one view or one
CLI command. Types, names, selectors, scripts, and extensions need common
project facts.

Hopper's tutorial shows the app behaving as a mutable analysis document:

- symbols and strings live in the left pane
- tags and text filter navigation
- the inspector changes with context
- byte types can be edited
- undo/redo is part of analysis
- graph nodes can carry comments, labels, colors, and grouping
- references can be inspected and user-added
- Objective-C class/category tags are auto-generated during loading

The semantic-pack implication is that external declarations must become
document facts with routes, provenance, and confidence. A header type shown in
the inspector should be linked to the declaration source, target artifact,
matching evidence, and invalidation status.

Hopper's FAQ says Hopper supports Mach-O Mac/iOS binaries and processor
families including Intel, ARM, ARM64, and PowerPC, and can be extended to
other CPUs through the SDK. It also says iOS binaries can be disassembled when
the user supplies legally readable, non-FairPlay-encrypted inputs. Damsel's
semantic packs should therefore model unavailable, encrypted, missing, or
user-provided sources explicitly instead of hiding them behind a generic import
failure.

### Apple Source Evidence

Apple Open Source is a primary distribution channel for code that defines many
Apple platform internals. The `xnu` repository documents header installation
locations including Kernel.framework, IOKit.framework, DriverKit include
paths, System.framework PrivateHeaders, and user include roots. It also
documents preprocessor guards such as `PRIVATE`, `KERNEL_PRIVATE`,
`BSD_KERNEL_PRIVATE`, `MACH_KERNEL_PRIVATE`, `XNU_KERNEL_PRIVATE`, `KERNEL`,
`DRIVERKIT`, `EXCLAVEKIT`, and `MODULES_SUPPORTED`. Those macros are not
incidental build details. They are semantic boundaries Damsel must preserve
when importing declarations into public/private/kernel/user/DriverKit contexts.

The `xnu` tree also makes clear that XNU has separate BSD, Mach, VM, IOKit,
DriverKit, kernel-extension, module-map, and private-header domains. A useful
Damsel pack format must keep those domains explicit so a syscall name, Mach
trap, IOKit class, private VM struct, and user-level constant do not collapse
into one unqualified global namespace.

The `objc4` distribution and Apple's Objective-C runtime documentation make
Objective-C runtime concepts primary: classes, selectors, methods, method
implementations, ivars, properties, protocols, and low-level runtime APIs.
Damsel already extracts some binary metadata; semantic packs should add the
declaration and framework side of that world.

The `dyld` distribution includes shared-cache, cache-builder, kernel-collection
builder, Mach-O, `libdyld`, and shared-cache runtime code. Damsel already has a
dyld shared-cache lane; semantic packs should help identify dyld concepts,
cache-building artifacts, fixups, closure/prebuilt-loader related names, and
kernel-collection relationships.

Swift ABI documentation explains that runtime metadata records carry type
kind, nominal type descriptors, field offsets, function metadata, protocol
metadata, and dynamic state. The Swift semantic pack should not replace the
Swift runtime metadata parser, but it should provide ABI reference entities and
known Swift runtime entrypoints that help classify static and dynamic evidence.

Clang documentation establishes a practical implementation route: libclang can
parse C, C++, Objective-C, and Objective-C++ source into an AST, traverse
declarations, expose source locations, and support development-tool style
indexing. Damsel should use structured compiler frontends for headers and
Objective-C/C++ declarations rather than inventing a string parser.

## Current Damsel Baseline

Verified from current repo docs and earlier code maps:

- Damsel is already Apple-focused and Mach-O-first.
- It can parse standalone Mach-O files and dyld shared caches.
- It exposes structured sections, symbols, imports, relocations, dyld
  metadata, Objective-C metadata, and AArch64 disassembly.
- The Hopper-gap pack already includes plans for a persistent project
  database, runtime browsers, type/signature store, debug info import, Swift
  metadata, decompiler, automation, AI, casebooks, and native Workbench.

Missing semantic-pack layer:

- no versioned SDK/header pack registry
- no declaration provenance model for headers/source packages
- no build-setting/profile model for parsing Apple headers
- no structured AST import pipeline for C, C++, Objective-C, and ObjC++
- no XNU macro-domain model
- no public/private/kernel/user/DriverKit semantic boundary model
- no declaration-to-binary matching service
- no semantic-pack browser or Workbench status UI
- no pack compatibility diagnostics
- no pack-driven search facets
- no pack-driven decompiler/type overlays
- no pack-driven report/export provenance

## Design Goals

1. Make semantic packs versioned, inspectable project sidecars.
2. Keep declaration facts separate from binary evidence.
3. Preserve source, license, platform, SDK, build, macro, and privacy
   provenance.
4. Support public SDKs, local Xcode SDKs, KDKs, dSYMs, open-source Apple
   packages, and user-provided private packs without merging their trust levels.
5. Use Clang/Swift-aware structured parsers wherever possible.
6. Keep import and indexing off the UI thread.
7. Make incomplete and ambiguous semantic knowledge useful without pretending
   it is certain.
8. Let the type store, runtime browsers, decompiler, automation, AI assistant,
   casebook, and reports all reference the same pack facts.
9. Support per-project pack pinning so research remains reproducible.
10. Keep pack data redaction-aware for private headers, local paths, comments,
    and team annotations.

## Non-Goals

- No automatic download of private Apple materials.
- No DRM bypass, FairPlay bypass, or device-protection bypass.
- No redistribution of proprietary SDK/private headers inside Damsel docs or
  default fixtures.
- No claim that an open-source header exactly matches a closed-source system
  binary unless binary/debug-info evidence proves the match.
- No decompiler correctness guarantee from header import alone.
- No general Linux/Windows SDK knowledge-base product in the first lane.
- No source-code browsing replacement for Xcode.

## Core Concepts

### Semantic Pack

A semantic pack is a versioned set of declarations and semantic hints imported
from one source family.

Examples:

- `macos-sdk-15.5-public`
- `iphoneos-sdk-19.0-public`
- `xnu-12345.1.2-open-source`
- `objc4-999.1-open-source`
- `dyld-1340-open-source`
- `kdk-26.0-25A123`
- `team-private-iokit-headers-2026-05`
- `damsel-builtin-objc-runtime-minimal`

Fields:

```text
SemanticPack {
  pack_id
  display_name
  source_kind
  source_uri?
  source_root_id?
  imported_at
  imported_by
  platform
  os_version?
  build_version?
  sdk_version?
  xcode_version?
  kdk_version?
  source_hash
  content_manifest_hash
  license_profile
  trust_level
  privacy_class
  parse_profile_id
  status
  problem_count
  fact_count
}
```

`source_kind` enum:

- local_xcode_sdk
- local_kdk
- local_dsym
- apple_open_source_archive
- apple_oss_git_checkout
- user_header_root
- generated_header_export
- builtin_minimal_pack
- plugin_provided_pack
- imported_json_pack

`trust_level` enum:

- builtin
- official_local
- official_open_source
- user_provided
- generated
- plugin_provided
- unknown

Important invariant:

- pack trust is not binary match confidence

An official SDK header may be trustworthy as a declaration source while still
being a poor match for a particular binary build.

### Parse Profile

A parse profile records how a header tree was parsed.

Fields:

```text
SemanticParseProfile {
  parse_profile_id
  language_mode
  target_triple
  sysroot_id?
  include_roots
  framework_roots
  modulemap_roots
  defines
  undefines
  clang_args
  swift_args?
  xnu_domain
  visibility_domain
  allow_private_headers
  allow_system_private_headers
  created_at
  diagnostics_hash
}
```

`language_mode` enum:

- c
- cxx
- objective_c
- objective_cxx
- modulemap
- swift_interface
- mig
- syscall_table
- plist
- generated

`xnu_domain` enum:

- none
- user
- kernel
- bsd_kernel_private
- mach_kernel_private
- xnu_kernel_private
- driverkit
- iokit_user_client
- exclavekit
- exclavecore

The XNU domain is not just metadata. It controls preprocessor definitions and
determines where declarations are valid.

### Declaration Fact

A declaration fact is a normalized declaration from a semantic pack.

Shared fields:

```text
DeclarationFact {
  decl_id
  pack_id
  source_file_id
  source_range
  USR?
  name
  qualified_name
  kind
  language
  availability
  visibility
  access
  namespace_path
  module_name?
  framework_name?
  owning_type_id?
  signature_id?
  type_id?
  value?
  macro_conditions
  declaration_hash
  doc_comment_hash?
  privacy_class
  confidence
}
```

`kind` enum:

- c_function
- c_global
- c_typedef
- c_struct
- c_union
- c_enum
- c_enum_case
- c_macro_constant
- c_macro_function
- objc_class
- objc_category
- objc_protocol
- objc_method
- objc_property
- objc_ivar
- objc_selector
- cpp_namespace
- cpp_class
- cpp_struct
- cpp_union
- cpp_enum
- cpp_method
- cpp_constructor
- cpp_destructor
- cpp_virtual_method
- cpp_static_method
- cpp_field
- cpp_vtable_hint
- iokit_class
- iokit_user_client_method
- mig_subsystem
- mig_routine
- mach_trap
- bsd_syscall
- driverkit_class
- swift_symbol_hint
- swift_runtime_entrypoint
- dyld_runtime_entrypoint
- availability_record
- entitlement_key
- plist_key
- unknown

### Semantic Match

A semantic match links a declaration fact to a project object.

Fields:

```text
SemanticMatch {
  match_id
  project_id
  source_id
  decl_id
  target_route
  target_kind
  match_kind
  evidence
  confidence
  conflict_set_id?
  created_by
  created_at
  invalidation_state
}
```

`match_kind` enum:

- exact_symbol_name
- exact_export_name
- exact_import_name
- objc_selector_name
- objc_class_name
- objc_type_encoding
- swift_demangled_name
- cxx_mangled_name
- debug_info_reference
- kdk_symbol_reference
- syscall_table_pattern
- mach_trap_pattern
- mig_routine_pattern
- iokit_vtable_pattern
- user_confirmed
- script_confirmed
- ai_suggested_unreviewed
- heuristic_name_similarity
- heuristic_callsite_shape

Matching rules:

- exact symbol/debug-info matches can be high confidence
- selector/class name matches are useful but not complete method identity
- open-source XNU names are hints unless build/source/version evidence agrees
- AI suggestions are never accepted matches until reviewed or proven by another
  evidence channel
- conflicting matches are stored, not overwritten

### Semantic Overlay

An overlay is a projection of pack facts onto a Workbench route.

Examples:

- function signature panel populated from SDK headers
- Objective-C selector page enriched with framework method declarations
- IOKit class page showing likely external methods and user-client selectors
- decompiler pseudocode line showing a typed call target
- XNU syscall browser showing known syscall names and argument types
- panic frame page showing KDK/dSYM plus XNU source declaration hints
- report citation to a header declaration and matched binary evidence

Overlay fields:

```text
SemanticOverlay {
  overlay_id
  route
  overlay_kind
  pack_ids
  match_ids
  display_title
  confidence_summary
  stale_state
  privacy_class
  actions
}
```

## Pack Families

### Apple SDK Header Pack

Input sources:

- Xcode platform SDK roots
- Command Line Tools SDK roots
- user-registered SDK roots
- generated minimal fixture SDKs for tests

Primary imports:

- C function declarations
- C typedefs
- structs, unions, enums
- macro constants
- Objective-C classes, categories, protocols, methods, properties, ivars
- C++ and ObjC++ declarations where headers expose them
- availability macros
- framework/module ownership
- framework umbrella headers
- module maps

Key challenges:

- SDKs contain many macros and availability gates
- private headers may not be present or legal to redistribute
- one header may expose different declarations under platform macros
- Objective-C lightweight generics and nullability need preservation
- module maps and umbrella headers can duplicate declarations
- parsing all headers at once may be too expensive

Implementation rule:

- import declarations through parse profiles and build a pack manifest; do not
  copy raw headers into project files unless the user explicitly creates a
  private handoff package that includes them

### Objective-C Runtime Pack

Input sources:

- Apple Objective-C runtime docs
- local SDK Objective-C headers
- `objc4` open-source distribution
- Damsel built-in minimal runtime names

Primary imports:

- runtime API functions
- runtime struct names where legally available
- selector/class/function conventions
- method type encoding grammar references
- known runtime helper calls
- retain/release/autorelease family names
- message-send variants
- block runtime entrypoints, if available from headers

Uses:

- classify runtime calls
- explain `objc_msgSend` callsites
- enrich selector and class browsers
- seed retain/release semantics
- feed decompiler annotations
- distinguish Objective-C runtime facts from framework API facts

### Swift ABI Pack

Input sources:

- Swift ABI docs
- Swift ABI headers
- local Swift runtime library symbols
- Damsel Swift metadata parser results

Primary imports:

- metadata kind names
- nominal descriptor concepts
- value witness table concepts
- function metadata concepts
- protocol/existential metadata concepts
- metadata accessor entrypoint names
- runtime function names
- bridge concepts between Swift and Objective-C

Uses:

- classify Swift runtime calls
- explain Swift metadata records
- annotate witness/accessor patterns
- support Swift Browser field labels
- provide AI/decompiler reference context without claiming full type recovery

Boundary:

- Swift ABI pack facts are reference semantics; actual Swift type facts come
  from the Swift runtime metadata subsystem.

### XNU Open Source Pack

Input sources:

- Apple `xnu` source checkout or archive
- selected XNU header roots
- build-generated syscall/Mach/MIG tables when available
- user-provided matching source package

Primary imports:

- BSD syscall names and numbers
- Mach trap names and numbers
- MIG subsystem/routine names
- kernel structs and enums
- VM, BSD, Mach, IOKit, DriverKit, and kernel-extension headers
- public/private/kernel macro boundaries
- module maps
- known kernel function declarations
- known constants, flags, option sets, and error codes

XNU-specific fields:

```text
XnuDeclarationDomain {
  decl_id
  xnu_component
  domain
  install_scope
  macro_gate
  exported_to_sdk
  available_to_user
  available_to_kernel
  available_to_driverkit
}
```

`xnu_component` enum:

- bsd
- osfmk
- iokit
- libkern
- security
- vm
- ipc
- net
- driverkit
- tests
- tools
- unknown

Crucial warning:

- open-source XNU source is often not an exact match for the user's local
  kernel binary. Treat it as versioned reference material unless KDK/dSYM/build
  evidence proves alignment.

### IOKit And DriverKit Pack

Input sources:

- SDK IOKit headers
- XNU IOKit headers
- DriverKit SDK headers
- KDK/private headers, if user-provided
- C++ ABI and runtime analysis facts

Primary imports:

- `IOService`-style class hierarchy hints
- user-client method names
- external method struct shapes
- class lifecycle method names
- matching dictionary keys where available
- DriverKit class declarations
- C++ virtual method declarations
- known vtable slot names where declaration order permits weak hints

Uses:

- enrich IOKit Browser
- improve C++/ObjC++ class pages
- label external method dispatch tables
- route from user client selectors to implementation candidates
- help reports explain user-client attack surface without overclaiming

### dyld Semantic Pack

Input sources:

- Apple `dyld` open-source distribution
- local dyld shared-cache metadata
- existing Damsel cache parsers

Primary imports:

- dyld runtime function names
- shared-cache builder concepts
- cache mapping/fixup concept names
- kernel-collection builder concepts
- closure/prebuilt-loader concept names where visible
- `libdyld` API names
- Mach-O helper declarations

Uses:

- classify dyld cache artifacts
- explain shared-cache links and fixups
- enrich dyld cache browser
- improve diff reports between OS builds
- provide source-grounded labels for dyld-internal routines where matched

### Private Team Pack

Input sources:

- user-provided headers
- internal symbol maps
- internal type databases
- exported Hopper scripts/results
- generated Damsel JSON packs

Rules:

- private packs default to private privacy class
- raw content is never included in diagnostics by default
- reports cite pack id/hash, not raw header text, unless user opts in
- pack sharing requires redaction preview
- automation and AI need explicit permission to read private declarations

## Architecture Overview

```text
Source Registration
  |
  +-- SDK/KDK/header roots
  +-- Apple OSS archives/checkouts
  +-- user private packs
  +-- generated packs
        |
        v
Semantic Pack Importer
  |
  +-- manifest builder
  +-- parse profile resolver
  +-- clang worker
  +-- Swift ABI/reference importer
  +-- XNU table/MIG importer
  +-- dyld/objc reference importer
  +-- diagnostics collector
        |
        v
Semantic Pack Store
  |
  +-- declarations
  +-- types/signatures
  +-- constants/macros
  +-- domains/availability
  +-- source ranges
  +-- provenance/license/privacy
        |
        v
Matching And Overlay Services
  |
  +-- symbol/class/selector matcher
  +-- C++/Swift name matcher
  +-- debug-info/KDK matcher
  +-- syscall/Mach/MIG recognizers
  +-- IOKit recognizers
  +-- conflict/stale-state manager
        |
        v
Consumers
  |
  +-- Workbench semantic pack browser
  +-- runtime browsers
  +-- type/signature store
  +-- decompiler overlays
  +-- search facets
  +-- automation/MCP
  +-- AI assistant evidence context
  +-- casebook/report exports
```

Ownership boundaries:

- `damsel-core` owns pack models, declaration records, semantic matches,
  provenance, privacy, and project APIs.
- `damsel-macho` consumes semantic matches when mapping Mach-O, ObjC, Swift,
  C++, dyld, and kernel facts.
- `damsel-import` or a future semantic-import crate owns Clang/Swift/XNU/dyld
  import workers.
- `damsel-cli` exposes import, list, match, diagnose, and export commands.
- `damsel-app` owns pack browser UX, status surfaces, permission prompts, and
  overlay presentation.
- Plugin/automation surfaces can request pack reads through capability gates.

## Storage Model

Tables:

- `semantic_packs`
- `semantic_pack_sources`
- `semantic_parse_profiles`
- `semantic_source_files`
- `semantic_parse_diagnostics`
- `semantic_declarations`
- `semantic_declaration_names`
- `semantic_types`
- `semantic_signatures`
- `semantic_constants`
- `semantic_availability`
- `semantic_macro_conditions`
- `semantic_xnu_domains`
- `semantic_iokit_hints`
- `semantic_mig_routines`
- `semantic_syscalls`
- `semantic_mach_traps`
- `semantic_matches`
- `semantic_match_conflicts`
- `semantic_overlays`
- `semantic_pack_jobs`
- `semantic_pack_metrics`

Indexes:

- by `pack_id`
- by `qualified_name`
- by lowercase/fuzzy name
- by declaration kind
- by framework/module
- by platform/SDK/build
- by XNU domain and component
- by selector string
- by C++ mangled/demangled name
- by Swift demangled name
- by syscall/trap/routine number
- by target route
- by privacy class

Storage rules:

- Store normalized declaration facts and source ranges.
- Store raw source only when the user explicitly imports a pack with raw-source
  retention enabled.
- Store doc comments as hashes by default; allow opt-in extracted summaries for
  public packs.
- Store macro conditions structurally, not as raw preprocessor text.
- Store local paths through source ids and redacted display names.
- Store pack manifests with content hashes for reproducibility.

## Import Pipeline

### Phase 1: Source Discovery

Inputs:

- Xcode SDK path
- Command Line Tools SDK path
- KDK path
- Apple OSS checkout
- header directory
- module map
- pack manifest JSON
- zipped pack archive

Discovery steps:

1. classify source kind
2. compute source root hash
3. detect platform and SDK version
4. detect Xcode/KDK/build version where possible
5. list header/module/source candidates
6. classify public/private/kernel/user/DriverKit roots
7. build parse profile candidates
8. preview import scope

Workbench UX:

- show source kind
- show detected platform/version
- show privacy class
- show estimated declarations
- show problematic roots
- ask before retaining raw source
- ask before indexing private headers

### Phase 2: Manifest Build

Manifest fields:

```text
SemanticPackManifest {
  schema_version
  pack_id
  roots
  source_files
  module_maps
  parse_profiles
  expected_domains
  license_profile
  privacy_class
  content_hashes
  import_policy
}
```

Manifest requirements:

- deterministic ordering
- stable hashes
- no absolute paths in export by default
- source-root redaction map
- explicit language/domain assignment

### Phase 3: Structured Parse

Use Clang/libclang or a dedicated helper for:

- C
- C++
- Objective-C
- Objective-C++
- module maps where practical

Use dedicated importers for:

- MIG definitions
- syscall tables
- Mach trap definitions
- Swift ABI reference metadata
- dyld concept/reference import
- generated JSON pack imports

Clang worker requirements:

- out-of-process optional worker for crash isolation
- timeouts per translation unit
- cancellation
- diagnostic capture
- include graph capture
- target triple and sysroot pinned in parse profile
- parallel import with bounded CPU and memory
- AST traversal emits typed records
- parser never writes project facts directly; it writes import results that are
  committed as a transaction

### Phase 4: Normalization

Normalize:

- names
- qualified names
- selector spellings
- type encodings
- C function signatures
- C++ method signatures
- Objective-C method families
- availability attributes
- macro guards
- typedef chains
- enum values
- constants
- source locations
- framework/module ownership

Do not normalize away:

- language
- namespace
- overload set identity
- macro condition
- platform condition
- public/private/kernel domain
- source pack id

### Phase 5: Pack Commit

Commit rules:

- all-or-recorded-partial
- manifest stored first
- declarations stored in chunks
- diagnostics stored even on partial failure
- pack status is `ready`, `partial`, `failed`, or `stale`
- project receives a timeline event when casebook is enabled

### Phase 6: Matching

Matching jobs run after pack commit or project source changes.

Matchers:

- exact symbol/import/export matcher
- Objective-C class/selector/method matcher
- Objective-C type-encoding matcher
- C++ mangled/demangled matcher
- Swift demangled-name matcher
- debug-info declaration matcher
- KDK symbol matcher
- syscall/trap/MIG matcher
- IOKit class/vtable/user-client matcher
- dyld runtime name matcher
- user-confirmed overlay matcher

Matching output must be reviewable in the Workbench.

### Phase 7: Overlay Build

Overlay jobs build route-specific projections:

- function declaration overlays
- selector overlays
- class overlays
- vtable overlays
- syscall/MIG overlays
- decompiler call overlays
- inspector cards
- search facets
- report citations

Overlay output should be paged and cached. It is derived data and may be
invalidated without losing pack declarations.

## API Contracts

### List Semantic Packs

```text
GET /api/projects/{project_id}/semantic-packs
```

Response:

```json
{
  "packs": [
    {
      "pack_id": "macos-sdk-15.5-public",
      "display_name": "macOS SDK 15.5",
      "source_kind": "local_xcode_sdk",
      "platform": "macos",
      "sdk_version": "15.5",
      "trust_level": "official_local",
      "privacy_class": "local_path_redacted",
      "status": "ready",
      "fact_count": 1280000,
      "problem_count": 12
    }
  ]
}
```

### Import Preview

```text
POST /api/projects/{project_id}/semantic-packs/import-preview
```

Request:

```json
{
  "source_kind": "local_xcode_sdk",
  "source_root": "source-root-1",
  "platform": "macos",
  "retain_raw_source": false,
  "include_private_headers": false
}
```

Response:

```json
{
  "detected": {
    "platform": "macos",
    "sdk_version": "15.5",
    "xcode_version": "17.0"
  },
  "parse_profiles": [
    "objective-c-public",
    "c-public",
    "cxx-public"
  ],
  "estimated_files": 4200,
  "privacy_class": "local_path_redacted",
  "warnings": [
    {
      "code": "private_headers_excluded",
      "message": "PrivateHeaders were detected but will not be indexed."
    }
  ]
}
```

### Start Import

```text
POST /api/projects/{project_id}/semantic-packs/import
```

Response:

```json
{
  "job_id": "job-semantic-import-1",
  "pack_id": "macos-sdk-15.5-public"
}
```

### Pack Detail

```text
GET /api/projects/{project_id}/semantic-packs/{pack_id}
```

Includes:

- manifest
- source roots
- parse profiles
- import diagnostics
- counts by declaration kind
- counts by framework/module
- privacy summary
- compatibility summary
- match summary

### Query Declarations

```text
GET /api/projects/{project_id}/semantic-packs/{pack_id}/declarations?kind=objc_method&q=initWith
```

Response:

```json
{
  "items": [
    {
      "decl_id": "decl-objc-method-1",
      "kind": "objc_method",
      "qualified_name": "NSView initWithFrame:",
      "framework_name": "AppKit",
      "signature": "- (instancetype)initWithFrame:(NSRect)frame",
      "availability": "macos",
      "privacy_class": "public",
      "route": "semantic-decl:decl-objc-method-1"
    }
  ],
  "next_cursor": "cursor-2"
}
```

### Semantic Overlay For Route

```text
GET /api/projects/{project_id}/routes/{route_id}/semantic-overlays
```

Response:

```json
{
  "route": "function:source-1:0x100003f80",
  "overlays": [
    {
      "overlay_id": "overlay-1",
      "overlay_kind": "function_signature",
      "decl_id": "decl-c-func-42",
      "pack_id": "macos-sdk-15.5-public",
      "confidence": "high",
      "match_kind": "exact_symbol_name",
      "display_title": "CGColorCreate",
      "actions": ["open_declaration", "apply_signature", "cite_in_casebook"]
    }
  ]
}
```

### Match Review

```text
GET /api/projects/{project_id}/semantic-matches?confidence=weak
POST /api/projects/{project_id}/semantic-matches/{match_id}/accept
POST /api/projects/{project_id}/semantic-matches/{match_id}/reject
```

Acceptance creates a semantic edit transaction and a casebook timeline event.

### Compatibility Check

```text
POST /api/projects/{project_id}/semantic-packs/{pack_id}/compatibility
```

Checks:

- platform
- architecture
- OS build
- SDK build
- KDK UUID/build
- source UUID
- binary UUID
- deployment target
- symbol/debug-info evidence
- stale route count

## CLI Contracts

```text
damsel semantic-pack list <project> --json
damsel semantic-pack discover <project> --source-root <path> --json
damsel semantic-pack import <project> --source-root <path> --kind local-xcode-sdk --json
damsel semantic-pack import <project> --apple-oss-root <path> --family xnu --json
damsel semantic-pack status <project> <pack-id> --json
damsel semantic-pack declarations <project> <pack-id> --kind objc_method --query init --json
damsel semantic-pack match <project> <pack-id> --source <source-id> --json
damsel semantic-pack overlays <project> <route> --json
damsel semantic-pack compatibility <project> <pack-id> --json
damsel semantic-pack export <project> <pack-id> --profile redacted --out pack.damselpack
damsel semantic-pack problems <project> <pack-id> --json
```

CLI rules:

- no raw private header text in JSON unless explicitly requested
- local paths redacted by default
- import jobs return deterministic ids in fixture mode
- all commands include schema version
- compatibility failures are structured problems, not stderr prose only

## Workbench UX

### Semantic Pack Center

Add a project-level "Semantic Packs" view.

Sections:

- installed packs
- suggested packs for current project
- missing packs
- import status
- compatibility
- declaration search
- match review
- privacy/redaction
- problems

Rows:

- pack display name
- source kind
- platform/build
- status
- privacy icon
- match count
- stale count
- problem count

Actions:

- Import SDK
- Import KDK
- Import Apple OSS Checkout
- Import Header Folder
- Import Pack Archive
- Match Current Source
- Review Weak Matches
- Export Redacted Pack Manifest
- Remove Pack From Project

### Open-Time Suggestions

When a source is opened, Damsel should suggest packs:

- standalone app binary -> matching macOS/iOS SDK
- dyld cache -> matching platform SDK and dyld semantic pack
- kernel collection -> matching KDK and XNU semantic pack
- panic log -> matching KDK, dSYM, and XNU semantic pack
- Objective-C heavy binary -> SDK ObjC headers and Objective-C runtime pack
- Swift heavy binary -> Swift ABI pack and Swift metadata analysis
- IOKit evidence -> IOKit/DriverKit pack

UI rule:

- suggestions are actionable but non-blocking

### Declaration Browser

Navigation facets:

- framework/module
- language
- kind
- platform
- availability
- XNU component/domain
- privacy class
- matched/unmatched
- confidence

Declaration detail:

- signature
- source pack
- source file path, redacted if needed
- macro/availability conditions
- matched routes
- conflicts
- use in decompiler/type overlays
- casebook citations

### Match Review Drawer

Review weak or conflicting matches.

Columns:

- route
- binary evidence
- declaration candidate
- match kind
- confidence reason
- conflicts
- actions

Actions:

- accept
- reject
- open route
- open declaration
- create note
- suppress matcher

### Inspector Integration

Inspector cards:

- Header Declaration
- SDK Signature
- ObjC Method Declaration
- C++ Declaration
- IOKit User Client Hint
- XNU Syscall/MIG Hint
- Swift ABI Reference
- Pack Compatibility

Each card must show:

- pack id
- confidence
- evidence
- privacy class
- stale state

### Search Integration

Global search facets:

- declaration
- selector
- framework API
- syscall
- MIG routine
- Mach trap
- IOKit class
- DriverKit class
- macro constant
- enum case
- type
- pack source

Search result rules:

- declaration-only results route to semantic declaration pages
- matched results route to binary objects
- weak matches show confidence inline
- private pack results obey privacy mode

## Matching And Confidence Model

Confidence levels:

- proven
- high
- medium
- weak
- rejected
- stale

Evidence channels:

- exact symbol/export/import
- debug info
- KDK/dSYM
- Objective-C metadata
- Swift metadata
- C++ RTTI/vtable
- callsite ABI shape
- syscall/Mach/MIG table structure
- IOKit external method table structure
- route/user selection
- automation review

Confidence rules:

- `proven` requires binary or debug-info evidence that uniquely identifies the
  declaration.
- `high` may come from exact symbol, selector, or type-encoding matches when
  ambiguity is low.
- `medium` means useful but not enough to drive type propagation without review.
- `weak` means display as a hint only.
- `stale` means pack/source/project changed after match creation.
- `rejected` remains stored to avoid repeated suggestions.

Never:

- apply a weak header hint to decompiler output as if it were true
- hide conflict sets
- silently prefer public SDK over KDK/dSYM evidence
- silently merge same-named declarations from different frameworks

## Privacy And Licensing

Privacy classes:

- public_builtin
- public_open_source
- official_local_path_redacted
- private_user_header
- private_team_pack
- generated_from_private_binary
- restricted_do_not_export

Rules:

- Store license profile per pack.
- Redact absolute source paths by default.
- Do not include raw private header text in diagnostics or reports by default.
- Do not include doc comments from private headers in AI context unless
  explicitly enabled.
- Do not export pack archives containing proprietary headers unless user opts in.
- Handoff packages include pack id/hash and missing-pack instructions by
  default.
- AI and MCP tools require `semantic_pack.read` capability and stronger
  permission for private packs.

Report behavior:

- cite pack id, source kind, version, hash, and declaration id
- include matched binary evidence
- label declaration-only hints as hints
- include redaction summary

## Performance Budgets

Import budgets:

- import preview first response: under 500 ms for path classification
- manifest for typical SDK root: progress visible under 2 seconds
- parse worker heartbeat: every 1 second
- cancel import: visible cancellation under 1 second
- Workbench remains interactive during import

Query budgets:

- declaration search warm first page: under 150 ms
- route overlay fetch warm: under 75 ms
- match review first page: under 150 ms
- pack detail summary: under 200 ms
- compatibility check cached: under 100 ms

Large-pack rules:

- never load all declarations into Swift arrays
- page every list
- cache counts separately from rows
- keep raw AST output out of the project database
- use background indexing with progress
- support pack compaction and removal

Metrics:

- files discovered
- files parsed
- parse failures
- declarations imported by kind
- declarations indexed by framework
- matches by confidence
- overlay cache hit/miss
- query latency
- memory high-water mark
- cancellation latency

## Integration With Existing Specs

### Type And Signature Store

Semantic packs supply candidate declarations, types, signatures, constants,
and availability records. The type store owns the canonical project type facts
after a declaration is matched, imported, or user-accepted.

Rule:

- declaration fact remains immutable; project type fact may be edited or
  overridden

### Debug Info And KDK/dSYM

KDK/dSYM evidence outranks header hints when matching a specific binary. A KDK
semantic pack can still expose declaration context, but symbolication profiles
own source-line and debug-object truth.

### Runtime Browsers

Runtime browsers consume semantic overlays:

- ObjC Browser shows SDK declarations beside runtime metadata.
- C++ Browser shows header declarations beside RTTI/vtables.
- IOKit Browser shows user-client and lifecycle hints.
- XNU Interface Browser shows syscalls, Mach traps, and MIG declarations.
- Swift Browser shows ABI reference labels beside parsed Swift metadata.

### Decompiler

Decompiler overlays may use accepted semantic matches to:

- name calls
- type arguments
- label return values
- annotate syscalls/traps/MIG calls
- mark IOKit external methods
- improve Objective-C method signatures
- show confidence and source pack citations

Decompiler must not silently consume weak semantic hints as facts.

### Automation, MCP, And AI

Automation tools:

- list packs
- search declarations
- fetch overlays for route
- explain match confidence
- request match review

AI assistant:

- may read public pack declarations by default if enabled
- must ask before private pack context
- must cite declaration ids and binary evidence separately
- may draft match suggestions but cannot accept them

### Casebook And Reports

Casebook events:

- pack imported
- compatibility checked
- match accepted/rejected
- private pack accessed by automation/AI
- pack removed
- report cited pack declaration

Reports:

- include semantic pack manifest summary
- include declaration citations
- include match confidence
- include stale/missing pack warnings
- include redaction summary

## Data Validation Matrix

Fixtures:

- tiny public C SDK fixture
- tiny Objective-C framework header fixture
- tiny ObjC++/C++ fixture with overloads and virtual methods
- tiny XNU-like header fixture with public/private/kernel macros
- tiny MIG fixture
- tiny syscall table fixture
- tiny IOKit user-client fixture
- tiny Swift ABI reference fixture
- synthetic KDK pack fixture
- private header redaction fixture

Tests:

- source discovery classification
- manifest deterministic hash
- parse profile snapshot
- Objective-C method declaration import
- C++ overload identity import
- macro-condition preservation
- XNU domain preservation
- syscall/trap/MIG import
- IOKit declaration import
- dyld reference import
- Swift ABI reference import
- exact symbol match
- ObjC selector match with conflict
- weak heuristic does not apply type automatically
- accepted match creates transaction
- stale pack invalidates overlays
- private pack redaction
- AI/MCP private pack capability denial
- report citation includes pack id and confidence
- large declaration query paging
- import cancellation

## Migration Slices

### S1: Pack Registry And Manifest

Deliver:

- `SemanticPack` model
- pack source registration
- manifest builder
- CLI `semantic-pack discover`
- Workbench pack list placeholder
- deterministic fixture tests

### S2: Public Header Import MVP

Deliver:

- Clang worker for C and Objective-C declarations
- parse profile model
- declaration tables
- diagnostics
- CLI `semantic-pack import`
- declaration query API
- tiny SDK fixture

### S3: Objective-C Runtime And SDK Overlay

Deliver:

- ObjC class/protocol/method/property import
- selector matcher
- type encoding matcher
- ObjC Browser declaration overlay
- match review drawer for selectors

### S4: XNU Domain Import

Deliver:

- XNU macro/domain model
- public/private/kernel/DriverKit parse profiles
- syscall/trap/MIG importers
- XNU Interface Browser overlays
- compatibility diagnostics

### S5: C++/ObjC++ And IOKit

Deliver:

- C++ class/method/field import
- overload and namespace identity
- IOKit class/user-client hints
- vtable/user-client weak match model
- IOKit Browser overlays

### S6: dyld And Swift Reference Packs

Deliver:

- dyld reference importer
- Swift ABI reference importer
- Swift runtime entrypoint labels
- dyld cache browser overlays
- Swift Browser reference cards

### S7: Matching Review And Transactions

Deliver:

- semantic match table
- conflict sets
- accept/reject/suppress commands
- semantic edit transaction bridge
- casebook timeline events

### S8: Reports, Automation, And Redaction

Deliver:

- semantic pack citations in report builder
- redacted pack manifest export
- automation/MCP tools
- private pack capability gates
- AI context rules

### S9: Performance Hardening

Deliver:

- large SDK import benchmark
- pack compaction
- overlay cache
- query indexes
- cancellation and memory budgets

## Risk Register

### Risk: Header Hints Become False Truth

Problem:

- A declaration from a nearby SDK version can look plausible while being wrong
  for a stripped binary.

Mitigation:

- separate declaration facts from matches
- show confidence and evidence
- require accepted matches before type propagation
- prefer KDK/dSYM/debug evidence

### Risk: Importing SDKs Freezes The Workbench

Problem:

- SDK and KDK trees are large.

Mitigation:

- background jobs
- progress heartbeat
- chunked commits
- paged queries
- no giant Swift arrays

### Risk: Private Header Leakage

Problem:

- user-provided headers may contain private or proprietary material.

Mitigation:

- privacy class defaults
- redacted paths
- no raw source export by default
- AI/MCP capability gates
- report redaction preview

### Risk: Clang Integration Becomes A Build-System Project

Problem:

- Apple headers need target triples, sysroots, macros, module maps, and
  conditional domains.

Mitigation:

- parse profiles as first-class objects
- import preview diagnostics
- fixture SDKs
- bounded language modes before broad module imports

### Risk: Namespaces Collapse

Problem:

- same names exist across frameworks, C++ namespaces, Swift modules, XNU
  domains, and private/public headers.

Mitigation:

- fully qualified names
- framework/module/domain fields
- overload ids
- no global name-only matching

### Risk: Open Source Apple Material Is Treated As Exact

Problem:

- Apple OSS source may not match the user's local binary build.

Mitigation:

- pack compatibility checks
- build/version/UUID fields
- stale warnings
- route evidence required for high confidence

## Acceptance Criteria

Documentation acceptance:

- this spec is linked from the Hopper-gap README
- feature inventory names semantic packs as a separate gap
- implementation epics include a semantic-pack epic
- API/data contracts mention pack import/query/match/overlay routes
- evidence matrix lists semantic-pack validation gates
- research sources cite Hopper and Apple/Clang sources
- type, runtime, Swift, debug-info, AI, casebook, and report specs link back

Implementation acceptance for first milestone:

- user can register a tiny SDK/header root as a semantic pack
- manifest is deterministic and redacts local paths
- C and Objective-C declarations import into the project database
- declaration search is paged
- exact symbol and Objective-C selector matches are recorded
- route overlays show pack id, confidence, and evidence
- private pack raw text is not exported by default

Implementation acceptance for Hopper-competitive lane:

- SDK, KDK, XNU, objc4, dyld, Swift ABI, IOKit, DriverKit, and private/team
  packs can be represented with version, provenance, privacy, and compatibility
  metadata
- weak/medium/high/proven match confidence is enforced across Workbench,
  decompiler, automation, AI, and reports
- runtime browsers and decompiler overlays consume accepted semantic matches
- XNU public/private/kernel/DriverKit domains are preserved
- large SDK queries remain responsive
- redacted reports and handoff packages cite semantic packs safely

## First Implementation Recommendation

Start with `S1` and `S2`:

1. Add the `SemanticPack`, `SemanticParseProfile`, `SemanticSourceFile`, and
   `DeclarationFact` models.
2. Add `damsel semantic-pack discover` and `damsel semantic-pack import` for a
   tiny fixture header root.
3. Use a helper-driven Clang parse path for C and Objective-C declarations.
4. Store only normalized declarations and redacted source locations.
5. Add paged declaration query API and a minimal Workbench pack list.
6. Add exact symbol and selector matching only after import snapshots are
   stable.

This gives Damsel a durable semantic-pack spine without immediately taking on
full SDK, XNU, C++, IOKit, Swift, KDK, report, and AI integration. Every later
pack family can extend the same manifest, provenance, declaration, match, and
overlay contracts.
