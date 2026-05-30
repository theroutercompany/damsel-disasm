# Implementation Epic Breakdown

Status: Planning artifact for turning the Hopper-gap documentation pack into
actual work. This is intentionally issue-like and dependency-oriented.

Purpose: the full objective is too large for one implementation branch. This
document decomposes the work into epics, deliverables, dependencies, validation
gates, and "do not start before" constraints.

## Principles

- Keep the Apple systems niche central.
- Build durable project state before UI chrome that depends on it.
- Build queryable facts before pseudocode.
- Build cancellation/paging before loading huge caches in the app.
- Build annotation/undo before patching.
- Build route/object identity before scripting, MCP, debugger, or plugins.
- Preserve existing CLI commands while adding project-backed commands.

## Epic Map

| Epic | Name | Primary Outcome | Depends On |
| --- | --- | --- | --- |
| E0 | Docs acceptance and scope lock | Agreement on product direction | current docs |
| E1 | Project/source/address model | Durable project identity | E0 |
| E2 | Fact/provenance database | Queryable analysis facts | E1 |
| E3 | Byte map and annotations | Hopper-style mutable document primitives | E1/E2 |
| E4 | Function and xref index | Whole-image navigation | E2/E3 |
| E5 | Workbench project shell | Native project workflow | E1-E4 API slices |
| E6 | Graph and feature-map UX | Synchronized visual navigation | E3/E4/E5 |
| E7 | ObjC runtime explorer | Apple runtime advantage | E4 |
| E8 | Shared-cache workspace | Cache-native navigation | E4/E7 |
| E9 | C++/ObjC++/IOKit explorer | System C++ workflows | E4/E7 |
| E10 | Kernel/KDK/panic workflows | XNU analysis lane | E8/E9 |
| E11 | Performance, scheduler, and persistence | Large-project viability | E1-E10 |
| E12 | Type/debug-info integration | Better names/types/source | E2/E4/E10 |
| E13 | Decompiler-lite and IR | Honest high-level function understanding | E4/E9/E12/E22 |
| E14 | Automation Studio, MCP, scripting, and AI assistant | Scriptable document facts plus evidence-grounded AI UX | E1-E4/E11/E21 |
| E15 | Debugger bridge and live Workbench | Static/dynamic route mapping plus native debugger surface | E5/E11/E12/E19/E21 |
| E16 | Plugin/SDK boundary | Controlled extensibility | E11/E14 |
| E17 | Patch Studio and binary revision design | Future binary mutation path with native preview/export UX | E3/E5/E11/E18/E19/E21 |
| E18 | Diff/export/evidence | Apple OS build archaeology and reports | E4/E7-E12/E14 |
| E19 | Native app trust/distribution/diagnostics | Trustworthy macOS workstation distribution and recovery | E1/E5/E11 |
| E20 | Artifact intake/source registration | Apple artifact open/import/project/enrich workflow | E1/E11/E19 |
| E21 | Command/navigation/undo interaction spine | Hopper-class native control surface | E1/E3/E5/E11 |
| E22 | Processor semantics and Apple ABI | Hopper-class CPU/ABI facts for Apple binaries | E1/E2/E4/E8-E10 |
| E23 | Decompiler Workbench representation | Native synchronized pseudocode workspace | E5/E11-E13/E21/E22 |
| E24 | Decompiler quality and user-assisted recovery | Trustworthy pseudocode correction and validation loop | E12/E13/E22/E23 |
| E25 | Remote device debugging and target environments | Honest remote macOS/iOS/simulator debugger workflow | E15/E19/E20 |
| E26 | Semantic editing and reanalysis hints | Analyst-authored knowledge and correction loop | E3/E12/E13/E21/E22/E24 |
| E27 | Preferences, keymaps, layout, and theming | Native workstation personalization and recovery | E5/E11/E19/E21/E26 |
| E28 | Swift runtime metadata and Swift-ObjC bridge | First-class Swift Apple-systems analysis lane | E2/E4/E7/E8/E11/E12/E21/E22 |
| E29 | Investigation casebook, timeline, and report workspace | Evidence-driven analyst memory and handoff | E3/E5/E14/E15/E17/E18/E21/E26 |

## E0: Docs Acceptance And Scope Lock

Goal:

- confirm Damsel is pursuing Apple-focused Hopper competitiveness, not generic
  clone breadth

Deliverables:

- docs pack reviewed
- non-goals accepted
- prioritized first implementation wave
- fixture strategy accepted
- real-system opt-in policy accepted
- Hopper SDK and document architecture research reviewed
- Hopper public-web feature architecture audit reviewed
- Hopper feature architecture public evidence reviewed
- Hopper interaction, debugger, and product surface research reviewed
- Apple runtime and kernel object model spec reviewed
- native Workbench Hopper-grade implementation spec reviewed
- function recovery and procedure database plan reviewed
- decompiler/function/IR implementation spec reviewed
- automation/MCP/scripting implementation spec reviewed
- plugin/SDK extension boundary implementation spec reviewed
- patching/hex/assembly/transaction implementation spec reviewed
- debugger static-dynamic integration implementation spec reviewed
- native app trust/distribution/diagnostics implementation spec reviewed
- Apple artifact intake/source-registration implementation spec reviewed
- command/navigation/undo Workbench interaction implementation spec reviewed
- Apple runtime Workbench browser implementation spec reviewed
- Swift runtime metadata and Swift-ObjC bridge implementation spec reviewed
- investigation casebook, session timeline, and report workspace implementation
  spec reviewed

Validation:

- docs inventory
- cross-links
- unresolved work-marker scan
- source list
- completion audit checklist
- SDK/object-model cross-link check
- public source-to-requirement cross-link check
- feature-architecture evidence cross-link check
- Workbench interaction architecture cross-link check
- procedure/decompiler implementation cross-link check
- automation/plugin boundary cross-link check
- patching/debugger implementation cross-link check
- command/navigation/undo interaction cross-link check
- Apple runtime browser suite cross-link check
- Swift runtime metadata and bridge cross-link check
- investigation casebook and report workspace cross-link check

Do not leave E0 until:

- patching/debugger are explicitly later
- project database is accepted as foundation
- Apple niche remains explicit
- runtime object-model scope is accepted as the ObjC/C++/kernel target shape
- native Workbench migration is accepted as a project/session/route/command
  architecture change, not a cosmetic refresh
- Hopper is understood as document, byte-type, procedure, CPU semantics,
  transaction, automation, and native-app architecture, not just a decoder with
  several views
- patching and debugging are understood as route/source/transaction/address
  architecture work, not isolated UI features
- native app distribution, helper lifecycle, source access, diagnostics,
  updates, plugin trust, and debugger entitlements are understood as one trust
  subsystem, not scattered packaging details
- artifact opening is understood as a source graph, sidecar matching,
  projection/extraction, and Workbench overview subsystem, not a file picker
- native app commands are understood as a shared descriptor/dispatcher,
  route-selection, navigation-history, transaction, and activity-log subsystem,
  not a growing set of AppKit selectors and notification names
- Apple runtime browsers are understood as route-backed, paged,
  confidence-aware Workbench workflows over ObjC, C++/ObjC++, IOKit, dyld
  cache, kernel, KDK/dSYM, XNU interface, and panic facts, not just static
  tables over parsed metadata
- investigation casebooks are understood as evidence-pinned analyst memory,
  session replay, timeline capture, finding review, redaction, and handoff
  architecture, not a loose notes surface beside the disassembler

## E1: Project, Source, And Address Model

Goal:

- introduce the durable object identity needed by every later feature

Deliverables:

- `AnalysisProject`
- `AnalysisSource`
- `AddressSpace`
- `AnalysisAddress`
- source registry
- deterministic object id scheme
- route resolver skeleton
- project manifest snapshot

Implementation tasks:

- add pure model types in `damsel-core`
- add project creation from standalone Mach-O
- add project creation from projected shared-cache image
- record source hash/UUID/build/architecture
- support source parent/child relationships
- implement address conversion errors
- add JSON snapshot tests

Validation:

- standalone project fixture
- projected cache image fixture
- missing source fixture
- source hash mismatch fixture
- address conversion unit tests

Risks:

- raw integer addresses leak into APIs
- source paths become privacy leaks
- object ids become unstable

## E2: Fact And Provenance Database

Goal:

- store analysis as facts with confidence and provenance

Deliverables:

- `AnalysisFact`
- `FactSource`
- `FactConfidence`
- fact insertion/query APIs
- conflict model
- stale fact model
- analyzer run metadata

Implementation tasks:

- model exact/strong/inferred/ambiguous/weak/user facts
- convert existing symbols/imports/exports/ObjC/dyld facts
- add analyzer run ids
- add stale analyzer version detection
- add contradiction representation

Validation:

- fact snapshot fixture
- conflicting name fixture
- stale analyzer fixture
- user fact precedence fixture

Risks:

- facts become stringly typed
- contradictions get silently collapsed
- user annotations are mixed with analyzer facts

## E3: Byte Map, Annotation, Undo, And Problems

Goal:

- add Hopper-style document primitives

Deliverables:

- byte classification table
- feature-map data API
- annotation model
- transaction log
- undo/redo
- problem model
- user hints

Implementation tasks:

- classify mapped ranges from headers/sections/symbols/strings/functions
- add make-code/make-data/make-string/make-procedure commands
- implement comments/labels/tags/bookmarks/colors
- implement manual refs and function bounds
- implement problem creation and suppression
- implement targeted invalidation hooks

Validation:

- byte map fixture
- classification override fixture
- undo/redo fixture
- problem/hint fixture
- feature-map compact run fixture

Risks:

- mutation bypasses transaction model
- feature map becomes section coloring only
- undo exists only in Swift UI

## E4: Function And Xref Index

Goal:

- lift local disassembly into persistent whole-image navigation

Deliverables:

- function seed collector
- function builder
- persistent procedure database
- procedure ranges and owners
- basic block records
- local labels
- register aliases
- stack-local placeholders
- procedure signatures and calling conventions as placeholders for later type
  enrichment
- procedure conflict records
- xref table
- caller/callee index
- string/data ref index
- function detail endpoint/command

Implementation tasks:

- seed from symbols/exports/function starts/ObjC IMPs
- recursively decode executable ranges
- merge/split overlapping functions conservatively
- record recovery status, stale analysis state, and problem records
- persist calls/branches/data refs/import refs/runtime refs
- build reverse xrefs
- expose paged queries

Validation:

- branch fixture
- jump-table fixture
- import fixture
- ObjC IMP fixture
- overlap/conflict fixture

Risks:

- overconfident function bounds
- poor performance on large text segments
- cache context lost in generic analyzer

## E5: Workbench Project Shell

Goal:

- make the native app a project workbench, not a transient CLI viewer

Implementation companion:

- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
  defines the concrete Swift store split, project document/session model,
  route stack, command registry, navigator, jobs/problems, transactions, and
  migration phases for this epic.

Deliverables:

- project open/save UI
- source/project navigator
- route navigation stack
- command palette baseline
- jobs/progress strip
- problems drawer
- persisted UI state

Implementation tasks:

- add project API client models
- replace upload-only workflow for local project open
- add route model in Swift
- add navigator paging
- add progress/cancel UI
- add menu commands
- add keyboard shortcuts

Validation:

- Workbench project open smoke
- navigation stack test
- command palette test
- jobs/progress UI test
- missing source error test

Risks:

- Swift holds huge arrays
- UI route state diverges from server route state
- app still requires terminal mental model

## E6: Graph, Feature Map, And Representation Sync

Goal:

- make assembly, graph, hex/data, inspector, and feature map synchronized views

Primary spec:

- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)

Deliverables:

- graph API
- graph layout service
- graph record, node, edge, and layout models
- feature-map strip
- feature-map lanes and compressed ranges
- read-only hex/data view
- bounded hex-window API
- selection sync
- canonical representation selection events
- graph list-mode accessibility fallback
- inspector module contracts
- graph export JSON/DOT
- Apple runtime graph extension points

Implementation tasks:

- build CFG graph from function records
- add block selection routes
- add minimap/feature-map data
- add hex range fetches
- connect assembly row selection to graph/hex/inspector
- add export graph command
- add graph layout cache and layout job metrics
- add selection-loop prevention by origin/reason/sequence
- add list fallback for large graphs and accessibility

Validation:

- graph JSON fixture
- graph layout smoke
- feature map route fixture
- hex/assembly selection sync test
- accessibility list fallback test
- graph export snapshot
- large graph fallback test
- selection loop prevention test

Risks:

- graph rendering distracts from data correctness
- feature map query scans whole project
- selection loops between views
- layout is accidentally treated as semantic truth
- hex view materializes huge binaries in Swift

## E7: Objective-C Runtime Explorer

Goal:

- make ObjC the first major Apple-specific advantage

Implementation companion:

- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
  defines the normalized selector, class, category, protocol, method,
  message-send, dispatch-binding, and cache-wide ObjC index records for this
  epic.

Deliverables:

- ObjC runtime object store
- class browser
- selector browser
- method page
- type encoding decoder
- message-send analysis pass
- manual receiver/selector binding

Implementation tasks:

- normalize existing ObjC metadata into project facts
- index classes/categories/protocols/selectors/methods
- link method IMPs to functions
- decode type encodings
- identify `objc_msgSend` callsites
- propagate receiver/selector candidates

Validation:

- ObjC class fixture
- selector implementer fixture
- selector caller fixture
- type encoding fixture
- ambiguous receiver fixture

Risks:

- dynamic dispatch ambiguity hidden
- categories/swizzling overclaimed
- receiver inference becomes too optimistic

## E8: Shared-Cache Workspace

Goal:

- make dyld shared caches first-class projects

Deliverables:

- cache source registry
- image browser
- cache-wide symbol indexes
- provider/importer navigation
- cache-wide ObjC indexes
- cross-image xrefs

Implementation tasks:

- avoid copying full cache into Swift
- map cache/image/projected addresses
- persist image dependency graph
- persist provider/importer/reexport facts
- support per-image analysis status
- add cache project overview

Validation:

- synthetic cache fixture
- provider/importer fixture
- cross-image route fixture
- optional real-cache smoke

Risks:

- real-cache tests become mandatory in CI
- projected-image context loses cache session
- slow cache-wide indexing blocks UI

## E9: C++ / ObjC++ / IOKit Explorer

Goal:

- support Apple system C++ and ObjC++ reverse engineering

Deliverables:

- demangler integration
- C++ symbol classifier
- RTTI/typeinfo records
- vtable scanner
- virtual callsite recognizer
- inheritance graph
- IOKit recognizers

Implementation tasks:

- evaluate demangling crate
- parse common Itanium ABI names
- detect vtable ranges
- link vtable slots to functions
- detect constructors/destructors where possible
- identify virtual call patterns
- add Workbench C++ browser

Validation:

- vtable fixture
- RTTI fixture
- virtual call fixture
- multiple inheritance fixture
- IOKit-style fixture

Risks:

- stripped RTTI
- multiple inheritance complexity
- false virtual-call targets

## E10: Kernel, KDK, And Panic Workflows

Goal:

- make XNU-style system binaries first-class

Deliverables:

- fileset/kernel collection loader
- member/kext model
- KDK/dSYM attachment
- kernel symbol/type import
- syscall/trap/MIG recognizers
- panic log parser/symbolicator
- kernel browser

Implementation tasks:

- parse kernel collection members
- model global/member address spaces
- attach KDK/dSYM with UUID/build checks
- import symbols/types/line info
- parse panic frames/slides/kext lists
- map frames to functions/routes

Validation:

- synthetic fileset fixture
- KDK mismatch fixture
- panic log fixture
- optional KDK-matched smoke

Risks:

- OS-version drift
- private fixtures unavailable
- wrong KDK symbols worse than missing symbols

## E11: Performance, Persistence, And Resilience

Goal:

- make large projects usable and reopenable

Primary spec:

- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)

Deliverables:

- path-based source registration
- source identity and availability records
- mmap-backed source access where practical
- bounded byte-window service
- analysis job scheduler
- job priority model
- job progress and event/polling surface
- append-only event log
- SQLite/embedded store spike
- paged query service
- cancellable jobs
- index status records
- metrics endpoint
- diagnostics export

Implementation tasks:

- add project/source register-path API
- preserve upload API as compatibility mode
- add source access handles
- add job scheduler
- add cancellation tokens
- add job progress records
- add event stream or polling fallback
- add query paging and result versions
- add first index shards for symbols, strings, procedures, routes, and problems
- add metrics collection
- add response-size logging
- add Workbench paged query store
- add Workbench job/progress store
- add project reopen tests
- add schema migration tests
- add diagnostics redaction

Validation:

- source registration fixture
- missing source fixture
- byte-window fixture
- job lifecycle fixture
- large-list paging fixture
- cancellation fixture
- stale cursor fixture
- first-page search benchmark
- project reopen fixture
- migration fixture
- diagnostics redaction fixture
- large cache performance smoke

Risks:

- persistence schema chosen too early
- source path registration leaks private paths
- cancellation is only UI state and not analyzer checkpoints
- UI filters giant arrays
- query responses stay too large for Swift decoding
- diagnostics leak private strings

## E12: Type And Debug-Info Integration

Goal:

- enrich analysis with trustworthy external symbols/types/source context

Primary spec:

- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)

Deliverables:

- type database
- type fact and name fact stores
- function signature store
- signature candidates and creation reasons
- calling convention records, including Apple AArch64 and ObjC runtime cases
- stack frame and local variable records
- register aliases and local labels
- ObjC type encoding importer
- ObjC property attribute importer
- C++ demangle signature candidates
- Swift demangled name facts
- dSYM matching
- DWARF import
- KDK import
- debug source registry
- debug source match explanations
- symbolication profile activation/deactivation
- compile unit, source file, and line table indexes
- inline callsite import
- variable location expression records
- source root mapping and privacy redaction
- compact unwind integration where useful
- source line records
- type precedence rules
- type conflicts and mismatch problem records
- type import/export snapshots
- Workbench type navigator, signature editor, local variable panel, profile
  panel, and source panel

Implementation tasks:

- add type/name/signature/debug-info model types
- add provenance, precedence, stale, conflict, and transaction records
- decode ObjC method, ivar, and property encodings into type facts
- import C++ demangled names and signature candidates
- import Swift demangled name facts without overclaiming Swift types
- model calling conventions and hidden arguments
- add variable, stack-frame, register-alias, and local-label facts
- choose DWARF strategy
- implement UUID matching
- import function symbols
- import line tables
- import types/signatures
- import source files, compile units, inline callsites, and variables where
  available
- implement KDK profile matching and profile switching
- preserve user annotation precedence
- expose mismatch problems
- expose type/signature/profile APIs and CLI JSON
- add Workbench stores and editor surfaces

Validation:

- ObjC type encoding fixture
- C++ demangle fixture
- Swift demangle fixture
- calling convention fixture
- stack/local/register alias fixture
- dSYM fixture
- UUID mismatch fixture
- type import fixture
- line table fixture
- precedence fixture
- type conflict fixture
- type import/export round trip
- Workbench type/signature store tests

Risks:

- dependency weight
- slow imports
- debug types overwrite user knowledge
- C++/Swift demangling overclaims source-level type recovery
- Workbench loads giant type graphs eagerly

## E13: Decompiler-Lite And IR

Goal:

- provide honest high-level function understanding

Primary spec:

- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
- [Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)

Deliverables:

- function summary pass
- architecture semantics layer
- low-level IR
- SSA/value-flow
- stack frame facts
- ABI and call-effect facts
- memory and alias policy
- runtime-overlay summaries for ObjC, C++, Swift-adjacent, and kernel facts
- control-structure recovery
- decompiler-lite UI/API payloads
- pseudocode representation handoff to E23

Implementation tasks:

- add IR model
- map source instructions to IR
- implement common AArch64 semantics
- implement AArch64e/PAC-aware target normalization where evidence exists
- add register SSA
- add stack frame recovery
- add ABI/calling-convention modeling
- add ObjC/C++ semantic overlays
- add kernel semantic overlays
- add uncertainty-aware summary UI
- add user hints and reanalysis hooks

Validation:

- procedure recovery snapshots
- IR snapshots
- SSA snapshots
- stack/local snapshots
- ABI/call-effect snapshots
- summary snapshots
- unresolved-fact negative tests
- pseudocode source-map contract tests shared with E23

Risks:

- pretty output outruns evidence
- dead/unreachable code hidden
- full decompiler expectations too early

## E22: Processor Semantics And Apple ABI

Goal:

- turn decoded rows into stable architecture, CPU-mode, register, control,
  stack, ABI, PAC, ObjC/C++/Swift, and XNU facts that every Hopper-class
  surface can cite

Primary spec:

- [Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)

Deliverables:

- architecture profile model
- ABI profile model
- CPU-mode model
- register role model
- instruction semantics model
- control effect records
- stack/frame effect records
- callsite ABI interpretations
- PAC state records
- ObjC message-send semantics
- C++/ObjC++ callsite semantics
- Swift-adjacent ABI and symbol-role facts
- XNU/kernel ABI profiles
- semantic problem records
- semantic override transactions
- CLI/API semantic profile and instruction routes
- Workbench architecture, instruction, ABI, CPU-mode, and PAC inspectors

Implementation tasks:

- add `ArchitectureProfile`, `AbiProfile`, `CpuModeProfile`, and
  `RegisterProfile`
- map Mach-O slice and fileset facts to profile selection
- add arm64 register roles and Apple ABI rules
- add local arm64 control semantics for branch/call/return families
- add PC-relative value semantics for ADR, ADRP, ADD, and literal loads
- add frame and stack semantics with Apple user/kernel profile separation
- add ObjC message-send callsite interpretation
- add arm64e/PAC state tracking
- add x86_64 Darwin profile baseline
- add legacy ARM/Thumb profile baseline
- add semantic problem and override models
- project semantic facts into existing `DisassemblyAnalysis` summaries during
  migration

Validation:

- architecture profile JSON snapshots
- register-role snapshots
- CPU-mode snapshots
- instruction semantic snapshots
- control-effect snapshots
- stack/frame snapshots
- PC-relative value snapshots
- PAC state snapshots
- ObjC/C++/Swift callsite fixtures
- XNU/kernel profile negative tests
- CLI/API route snapshots
- Workbench inspector payload snapshots
- performance gates for semantic page queries

Risks:

- semantic layer grows into an unbounded decoder rewrite
- ABI facts look more certain than the evidence supports
- user-mode ABI rules leak into kernel workflows
- arm64e/PAC is flattened into raw address resolution
- semantic enrichment blocks Workbench responsiveness

## E23: Decompiler Workbench Representation

Goal:

- make pseudocode a native, synchronized, evidence-backed Workbench
  representation rather than a detached backend string

Source spec:

- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [Decompiler Quality And User-Assisted Recovery Implementation Spec](decompiler-quality-user-assisted-recovery-implementation-spec.md)

Deliverables:

- `DecompilerFunctionView` API contract
- pseudocode route and tab/split behavior
- decompiler job and cache UI
- pseudocode line/token models
- line-to-assembly, graph, hex, source, IR, and runtime source maps
- confidence and issue gutters
- decompiler-lite summary before full pseudocode
- variable rename transactions
- type/signature edit transactions
- ObjC message-send overlays
- C++ virtual-call overlays
- XNU/KDK/panic/log overlays
- large-function partial result flow
- pseudocode export and evidence package
- automation read endpoints and audited mutation hooks

Implementation tasks:

- add decompiler response models to Rust and Swift
- add `DecompilerWorkspaceStore`, `DecompilerJobStore`,
  `PseudocodeViewStore`, and edit/problem/export stores
- add command families for open, generate, re-run, cancel, rename, set type,
  mark no-return, set switch hint, toggle dead code, and export
- add source-map APIs and selection synchronization across assembly, graph,
  hex, source/debug info, runtime browsers, and inspector
- add cache invalidation dependencies for type, signature, function-bound,
  runtime overlay, debug-info profile, and user-hint changes
- add performance metrics for cached open, generation, cancellation, line
  selection sync, scroll stability, memory peak, and export size

Validation:

- pseudocode view JSON snapshots
- source-map snapshots
- confidence/issue snapshots
- Workbench route snapshots
- command validation snapshots
- rename/type undo UI tests
- cancellation and stale-cache UI tests
- ObjC/C++/kernel overlay fixtures
- large-function partial result benchmark
- evidence export/redaction tests

Risks:

- pseudocode becomes visually convincing but semantically opaque
- source maps are too weak to trust synchronized navigation
- variable/type edits mutate UI text instead of project facts
- large functions stall the native app
- Apple runtime overlays overstate weak evidence

## E24: Decompiler Quality And User-Assisted Recovery

Goal:

- make pseudocode trustworthy by exposing pass evidence, source maps,
  dead-code policy, user hints, Apple-runtime quality gates, regression cases,
  and performance/cancellation proof

Primary spec:

- [Decompiler Quality And User-Assisted Recovery Implementation Spec](decompiler-quality-user-assisted-recovery-implementation-spec.md)

Deliverables:

- decompiler run records
- decompiler pass ledger
- decompiler quality report
- coverage and confidence summaries
- source-map inspector
- decompiler issue records
- user hint and correction transaction records
- dead-code include/exclude policy
- switch and jump-table recovery workflow
- no-return, tailcall, thunk, frame, and calling-convention hint workflows
- ObjC overlay quality records
- C++/ObjC++ overlay quality records
- Swift-adjacent overlay quality records
- XNU/kernel overlay quality records
- arm64e/PAC quality records
- decompiler regression corpus
- forbidden-claim regression expectations
- Workbench quality drawer, pass ledger, and hint deck
- automation and assistant read endpoints over decompiler evidence

Implementation tasks:

- add `DecompilerRun`, `DecompilerPassRun`, `DecompilerQualityReport`,
  `DecompilerIssue`, `DecompilerSourceMap`, `DecompilerHint`,
  `DecompilerRegressionCase`, and `DecompilerPerformanceSample`
- connect source maps to IR, processor semantics, instructions, graph blocks,
  type/signature facts, runtime objects, and user hints
- add dead-code policy and reachable/excluded block records
- add switch table hint records for address, entry size, entry count,
  multiplier, and default target
- add variable rename/type correction as transactions
- add no-return, tailcall, and thunk hint transactions
- add ObjC/C++/Swift/XNU/PAC overlay quality issue categories
- add quality CLI/API routes
- add regression runner and semantic expectation schema
- add native Workbench quality drawer, pass ledger, source-map inspector, and
  hint deck payloads

Validation:

- decompiler run/pass ledger snapshots
- quality report snapshots
- line/token source-map snapshots
- dead-code inclusion/exclusion fixtures
- switch hint fixtures
- variable rename/type transaction fixtures
- no-return/tailcall/thunk fixtures
- ObjC/C++/Swift/XNU/PAC overlay quality fixtures
- forbidden-claim negative tests
- stale pseudocode marker snapshots
- cancellation and performance samples
- assistant citation validation over pseudocode/source-map routes

Risks:

- pseudocode confidence becomes a cosmetic badge instead of backed evidence
- hint edits mutate text instead of project facts
- exact pseudocode text snapshots make regression tests brittle
- Apple runtime overlays overfit tiny fixtures
- large functions make quality reports expensive

## E14: Automation Studio, MCP, Scripting, And AI Assistant

Goal:

- expose project facts safely to scripts and AI/tooling through one typed API
  plus native Automation Studio and evidence-grounded assistant surfaces

Primary spec:

- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec](automation-studio-scripting-plugin-workbench-implementation-spec.md)
- [Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md)

Deliverables:

- CLI JSON query foundation
- CLI batch operations
- native Automation Studio shell
- evidence-grounded assistant pane
- assistant context/evidence/tool/suggestion stores
- assistant model profiles and privacy policies
- recipe catalog and recipe run records
- read-only local MCP server
- MCP session panel and call log
- script package install/run plan
- plugin package visibility and compatibility state
- audited annotation mutation API
- automation activity log
- capability and permission model
- privacy/redaction policy
- MCP tool schema fixtures
- Apple-system recipe pack for ObjC, dyld cache, C++/IOKit, panic/KDK, and
  patch review workflows
- Apple-system assistant modes for ObjC dispatch, C++ virtual calls, kernel
  panic frames, patch plans, debugger stops, and artifact diffs

Implementation tasks:

- add project summary/search/route/function/xref/runtime/problem tool schemas
- add recipe catalog, input schema, output schema, and run model
- add native stores for automation studio, recipes, runs, MCP sessions,
  capability grants, script packages, plugin packages, outputs, activity, and
  diagnostics
- add assistant session, context, evidence, tool-call, suggestion, model
  profile, privacy, prompt-template, and diagnostics stores
- expose common read APIs
- add structured batch command
- add local stdio MCP server binding to a visible project/session scope
- expose audited annotation writes only through annotation transactions
- add permission denial paths
- add byte-read and export-selection policy
- add Workbench-visible automation activity records
- add MCP-shaped JSON contract fixtures
- add Apple-system recipes and evidence outputs
- add script/plugin sidecar run metadata and provenance records
- add assistant prompt templates, context review, route/fact citation
  validation, privacy gates, model-provider adapters, and reviewed suggestions

Validation:

- script query fixture
- mutation audit fixture
- permission denial fixture
- MCP schema snapshot
- MCP session/call-log UI test
- recipe catalog snapshot
- recipe run/cancel snapshots
- capability grant/revoke tests
- script timeout fixture
- plugin compatibility fixture
- Apple-system recipe fixtures
- assistant context snapshot
- assistant tool timeline snapshot
- assistant citation validation fixture
- hallucinated-route rejection fixture
- assistant privacy/redaction fixture
- assistant suggestion apply/undo fixture
- Apple-system assistant explanation fixtures
- privacy redaction fixture
- batch partial-failure fixture
- route-backed evidence fixture

Risks:

- arbitrary code execution too early
- MCP mutates without audit
- API stabilization burden
- automation returns plausible prose without route/object evidence
- automation gets a separate model from CLI and Workbench

## E15: Debugger Bridge And Live Workbench

Goal:

- bridge live LLDB state to static project routes and expose it through a
  permissioned native debugger Workbench

Deliverables:

- debug target and launch configuration model
- launch/attach sheets and debugger bar
- LLDB launch/attach spike
- debug session model
- runtime module model
- runtime address model
- module/slide mapping
- static-to-dynamic route resolver
- breakpoint route mapping
- breakpoint gutter and unresolved breakpoint problem states
- current-PC overlay model
- frame-to-function mapping
- registers/backtrace/memory panel
- ObjC/C++/arm64e runtime value interpretation
- helper-process isolation and session-token auth
- redacted debugger diagnostics and session export
- platform failure problem records
- separate future design gates for remote, iOS, and kernel debugging

Source specs:

- [Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
- [Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
- [Remote Device Debugging And Target Environment Implementation Spec](remote-device-debugging-target-environment-implementation-spec.md)

Implementation tasks:

- define debug target/session/module/breakpoint/stop-event/frame/register/memory
  API contracts
- add native debug target, session, breakpoint, module, thread, frame,
  register, memory, problem, and diagnostics stores
- evaluate LLDB transport
- define helper process boundary
- authenticate helper sessions with per-session tokens
- map runtime modules to sources
- compute slides
- resolve runtime address to project route
- resolve project route to runtime breakpoint location
- set route-backed breakpoints
- read registers/backtrace/read-only memory
- interpret registers and memory using ObjC, C++, dyld, PAC, and source-route
  evidence where available
- surface SIP/permission errors
- represent dyld shared-cache and patched-source-revision mismatches
- enforce redaction for launch environment, arguments, logs, registers, memory,
  and local paths
- gate automation capabilities for launch, attach, control, memory read, and
  breakpoint mutation

Validation:

- local fixture executable debug smoke
- UUID/slide mismatch fixture
- breakpoint route test
- current-PC overlay test
- frame mapping test
- read-only memory test
- helper crash test
- helper token rejection test
- launch/attach UI tests
- register/frame/memory snapshot tests
- ObjC/C++/arm64e stop-context fixtures
- redacted export test
- platform denial fixture

Risks:

- platform permissions
- debugger UX distracts from static core
- runtime/static mismatch confusion
- helper process or event stream freezes Workbench if not isolated

## E25: Remote Device Debugging And Target Environments

Goal:

- extend local debugger planning into explicit remote macOS, iOS/iPadOS,
  simulator, and XNU-adjacent target-environment workflows without implying
  platform bypass or unsafe hidden attach behavior

Primary spec:

- [Remote Device Debugging And Target Environment Implementation Spec](remote-device-debugging-target-environment-implementation-spec.md)

Deliverables:

- target environment registry
- target host identity records
- device identity records
- debug transport records
- debug-server instance lifecycle records
- remote debug plan and preflight model
- attach candidate model
- remote module/source match records
- device symbol source records
- runtime event stream
- reconnect/disconnect states
- target environment browser
- debug plan sheet
- remote module view
- device runtime problems drawer
- remote stop report
- iOS/simulator supported and unsupported workflow policy
- Darwin kernel placeholder and KDK/panic relation records
- redacted remote/device diagnostics
- automation capability gates for remote connect, launch, attach, control, and
  memory reads

Implementation tasks:

- add `TargetEnvironment`, `TargetHostIdentity`, `DeviceIdentity`,
  `DebugTransport`, `DebugServerInstance`, `RemoteDebugPlan`,
  `AttachCandidate`, `RuntimeModuleMatch`, `DeviceSymbolSource`, and
  `RuntimeEvent`
- add target list, connect/disconnect, preflight, attach-candidate, remote-plan,
  runtime-module, event, stop-report, reconnect, and diagnostics endpoints
- add simulator target placeholders and skipped-with-reason validation
- model Developer Mode, pairing, toolchain, Xcode, debugserver, device-support,
  and symbol-cache problem states
- model remote source/install paths separately from local project source ids
- add runtime module matching by UUID, install name, path, dyld-cache identity,
  fileset member, and source revision
- keep unsupported iOS App Store attach, jailbreak-only workflows, process
  memory writes, and live kernel debugging behind explicit blocked states
- add native target browser, plan sheet, module view, runtime problems drawer,
  and stop report payloads
- enforce redaction for device identifiers, paths, launch arguments,
  environment, logs, registers, memory, and network endpoints

Validation:

- target environment JSON snapshots
- duplicate target fixture
- missing Xcode/debugserver/toolchain fixtures
- Developer Mode disabled fixture
- unpaired device fixture
- simulator unavailable fixture
- refused transport fixture
- reconnect fixture
- source mismatch fixture
- dyld-cache image match fixture
- missing dSYM/device-symbol fixture
- unsupported App Store attach fixture
- live kernel blocked fixture
- runtime event paging fixture
- remote stop report snapshot
- redacted diagnostics snapshot
- automation permission-denial tests

Risks:

- docs imply protected-device bypass capability
- remote debugging turns into a general remote shell
- runtime and static artifacts mismatch silently
- device diagnostics leak identifiers or private paths
- network/event streams freeze the Workbench
- kernel debugging scope overwhelms static XNU analysis

## E26: Semantic Editing And Reanalysis Hints

Goal:

- make analyst-authored knowledge first-class, route-aware, undoable,
  reanalysis-aware, automation-safe, and exportable with privacy controls

Primary spec:

- [Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec](semantic-editing-reanalysis-hints-implementation-spec.md)

Deliverables:

- semantic edit schema
- semantic edit target/value/group records
- suggestion and review records
- conflict and stale-route repair records
- comment, label, tag, bookmark, color, and manual-reference edits
- byte-type and procedure hint edits
- type, signature, variable, local-label, and register-alias edits
- switch/case, dead-code, and decompiler recovery hints
- ObjC message, C++ vtable/virtual-call, IOKit, MIG/syscall/trap, KDK, and
  panic-frame binding edits
- reanalysis impact records
- Workbench inspector edit decks
- suggestion tray and conflict browser
- CLI/API edit commands
- automation capability gates
- diagnostics and evidence export redaction policies

Implementation tasks:

- add semantic edit tables to project kernel schema
- add route-target validation and stale-route repair policies
- add edit kind/state/actor/source-surface enums
- add set/clear comment, inline comment, label, tag, bookmark, and color
  transactions
- add manual xref transactions and xref-index integration
- add byte-type and procedure hint transactions with reanalysis request records
- add variable/type/signature edit transactions and debug-info conflict records
- add decompiler hint transactions and decompiler cache invalidation hooks
- add ObjC/C++/XNU runtime relationship binding transactions
- add suggestion store, evidence records, review workflow, and capability checks
- add Workbench inspector, suggestion tray, conflict browser, and stale-edit UI
- add CLI/API list, preview, apply, undo, redo, suggest, and accept commands
- add diagnostics redaction and evidence-package inclusion controls
- add performance metrics for simple edits, bulk imports, and route repair

Validation:

- semantic edit schema snapshots
- comment/label/tag/bookmark/color persistence and reopen tests
- manual xref forward/reverse query tests
- byte-type override conflict tests
- procedure hint reanalysis request tests
- variable rename/type assignment tests
- function signature edit and profile-switch conflict tests
- switch/dead-code/decompiler hint invalidation tests
- ObjC message binding undo tests
- C++ vtable/virtual-call binding undo tests
- XNU/MIG/panic binding undo tests
- suggestion accept/reject tests
- capability denial tests
- stale-route repair tests
- diagnostics redaction tests
- evidence export annotation inclusion tests
- bulk script annotation performance tests

Risks:

- comments and labels become Swift-only UI state
- user edits overwrite imported symbols or analyzer facts
- reanalysis deletes or detaches user work without visibility
- assistant suggestions become unreviewed mutations
- scripts pollute projects with untraceable edits
- Apple runtime bindings overclaim certainty
- shared-cache projects regress from full annotation scans

## E27: Preferences, Keymaps, Layout, And Theming

Goal:

- make native Workbench personalization typed, recoverable, testable,
  privacy-aware, accessible, and fast enough for daily reverse-engineering work

Primary spec:

- [Preferences, Keymaps, Layout Persistence, And Theming Implementation Spec](preferences-keymaps-layout-theming-implementation-spec.md)

Deliverables:

- typed app preference registry
- project preference API
- effective preference resolver
- renderer preferences for assembly, graph, hex, search, and pseudocode
- keymap profiles and shortcut conflict detection
- theme token model and built-in accessible themes
- workspace layout persistence, repair, and reset
- analysis profile preferences
- Apple artifact source root policies for shared caches, KDKs, dSYMs, device
  symbols, IPSW/OTA roots, and panic logs
- automation/assistant/MCP privacy preferences
- diagnostics and evidence export presets
- settings import/export bundle schema

Implementation tasks:

- add `PreferenceStore`, `ProjectPreferenceStore`, `KeymapStore`,
  `ThemeStore`, `WorkspaceLayoutStore`, and diagnostics preference state
- add Rust project tables for project preferences and workspace layouts
- add app/project/effective preference API routes
- add keymap validation and command-id binding APIs
- add theme validation and contrast checks
- add layout save/repair/reset APIs
- add instruction-byte, fixed-hex-column, and fuzzy-search preferences as
  first proof points
- add settings UI tabs for General, Appearance, Assembly, Graph, Hex, Search,
  Analysis, Apple Artifacts, Keybindings, Automation And AI, Privacy And
  Diagnostics, and Advanced
- add diagnostics redaction preset preview
- add corrupt layout fallback and reset command

Validation:

- preference schema snapshots
- effective preference snapshots
- instruction-byte toggle UI/API test
- fixed hex column UI/API test
- fuzzy search toggle UI/API test
- keymap conflict fixture
- theme token and contrast fixtures
- feature-map color distinguishability fixture
- layout restore/repair/reset tests
- corrupt layout fallback test
- analysis profile preview test
- Apple artifact root redaction test
- diagnostics preset redaction test
- preference hot-path performance samples

Risks:

- settings become unstructured Swift view state
- project files leak local paths, keymaps, comments, or diagnostics choices
- themes break byte-category meaning or accessibility
- shortcut conflicts make navigation unreliable
- layout restoration blocks first paint on large projects
- analysis profiles hide expensive work
- automation preferences become cosmetic toggles instead of enforced gates

## E28: Swift Runtime Metadata And Swift-ObjC Bridge

Goal:

- make Swift metadata, names, conformances, accessors, runtime calls, and
  Objective-C bridges first-class routeable project facts for Apple-system
  analysis

Primary spec:

- [Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec](swift-runtime-metadata-implementation-spec.md)

Deliverables:

- `SwiftMetadata` root model
- Swift section inventory
- demangler service and failure records
- Swift name facts and role classification
- Swift module/type/protocol/conformance records
- type-reference, reflection-string, and field-metadata parsers
- witness table and metadata accessor classification
- Swift runtime call classification
- Swift-ObjC bridge relation records
- Swift browser APIs and Workbench stores
- Swift problem records
- dyld shared-cache projected-image route preservation
- large-name batch performance budgets

Implementation tasks:

- add `damsel-macho::swift` parser module
- add `SwiftMetadata` models in `damsel-core`
- move current `--swift-symbol` demangler shell-out behind a reusable
  demangler service
- add Swift section JSON commands
- add Swift name-role facts and search indexes
- add relative-pointer helpers with bounded failure modes
- add typeref and reflection-string parsers
- add field metadata parser
- add protocol and conformance parsers
- add metadata accessor and runtime-call classifiers
- add Swift-ObjC bridge linker
- add Swift Browser Workbench stores and paged APIs
- add diagnostics redaction for Swift names, fields, and local paths

Validation:

- Swift section classifier snapshots
- `swift-sample` raw/demangled lookup tests
- demangler unavailable and timeout fixtures
- Swift name-role corpus
- typeref symbolic-reference rejection fixture
- malformed typeref and field metadata bounded parse tests
- field metadata fixture
- protocol and conformance fixtures
- witness accessor fixture
- metadata accessor/runtime-call fixtures
- mixed Swift/ObjC bridge fixture
- Workbench Swift Home UI smoke
- route reveal from Swift type to hex/assembly/ObjC bridge
- large demangle batch performance sample
- dyld shared-cache projected-image Swift route smoke

Risks:

- demangled strings are mistaken for real type recovery
- malformed Swift metadata crashes the parser
- SwiftUI materializes huge symbol/type lists
- Swift-ObjC bridge edges overclaim certainty
- ABI drift breaks parser assumptions
- static metadata and future debugger runtime observations are merged without
  evidence boundaries

## E29: Investigation Casebook, Session Timeline, And Report Workspace

Goal:

- make manual edits, route history, automation outputs, assistant responses,
  debugger stops, diffs, patch previews, notebook pages, and evidence exports
  part of one recoverable case record

Primary spec:

- [Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec](investigation-casebook-session-timeline-implementation-spec.md)

Deliverables:

- case and session models
- timeline event model with actor, route, source, privacy, and evidence fields
- evidence pins with route repair and stale-state diagnostics
- hypotheses, findings, review states, and confidence boundaries
- notebook pages with route/evidence embeds
- report builder with outline, section, citation, and export models
- redaction preview and handoff package export
- Workbench casebook sidebar, timeline, evidence tray, hypothesis board,
  findings list, notebook editor, and report builder stores
- API and CLI surfaces for case/session/timeline/evidence/hypothesis/finding,
  notebook, report, redaction, and handoff operations
- privacy, paging, compaction, and append-only audit rules

Implementation tasks:

- add case, session, timeline, evidence pin, hypothesis, finding, notebook,
  report, redaction, and handoff tables
- add session lifecycle commands for open, resume, pause, close, and snapshot
- add timeline append service with typed actor/event validation
- connect route history, semantic edits, annotation changes, automation runs,
  AI drafts, debugger stops, diff reviews, patch previews, and evidence exports
  to timeline emission
- add evidence pin creation from route selections, search results, diffs,
  debugger stops, patch previews, and exported artifacts
- add stale-route repair and unresolved-evidence problem reporting
- add hypothesis and finding review-state transitions with required evidence
  links before accepted findings
- add notebook route/evidence embeds and deterministic snapshot rendering
- add report outline/section builder, citation checks, export, and manifest
  generation
- add redaction preview for local paths, source roots, user text, device names,
  debugger targets, plugin names, AI prompts, notebook prose, and report text
- add handoff package export and reopen validation
- add Workbench stores and paged APIs that keep large timelines off the main
  thread

Validation:

- case/session model snapshots
- timeline actor/event fixture matrix
- route-history and semantic-edit event capture tests
- automation, AI, debugger, diff, and patch event capture tests
- evidence pin creation and stale-route repair fixtures
- hypothesis and finding transition tests
- accepted-finding requires evidence-pin test
- notebook embed rendering snapshots
- report preview/export snapshots
- redaction preview and export redaction fixtures
- handoff manifest and reopen smoke
- large timeline paging and compaction performance sample

Risks:

- casebook collapses into a loose notes app with weak links to disassembly facts
- noisy event capture hides the analyst's important decisions
- local paths, prompts, target names, or notebook text leak through reports
- AI-generated text is treated as an accepted finding without human review
- large timelines block the Workbench
- handoff packages are exportable but not reopenable with route identity intact

## E16: Plugin / SDK Boundary

Goal:

- support controlled extensibility after object contracts stabilize

Primary spec:

- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)

Deliverables:

- plugin manifest
- SDK versioning
- loader, sidecar importer, architecture semantics, analyzer, renderer,
  exporter, and Workbench command capability shapes
- helper-process isolation story
- plugin provenance and invalidation model
- extension activity and metrics model

Implementation tasks:

- define plugin capability schema
- define fact/problem emission API
- define cancellation/metrics hooks
- add incompatible-version errors
- add capability-denial errors
- record plugin id/version/schema on emitted facts
- define crash isolation and cancellation behavior
- spike one toy analyzer plugin

Validation:

- manifest validation
- SDK mismatch test
- capability denial fixture
- plugin cancellation fixture
- plugin crash isolation test
- plugin-produced fact provenance snapshot

Risks:

- exposing unstable internals
- extension API freezes too early
- plugin crashes corrupt project
- loader, CPU/architecture, analyzer, exporter, and UI tool boundaries collapse
  into one unsafe generic plugin class

## E17: Patch Studio And Binary Revision Design

Goal:

- define future binary mutation and Patch Studio UX without enabling
  accidental data loss

Deliverables:

- patch policy doc
- read-only synchronized hex/data view
- patchability model
- Patch Studio target/editor/preview/impact/export layout
- patch preview model
- source revision model
- patch transaction model
- patch history model
- assembler integration spike
- AArch64 NOP generation spike
- write-copy-only export design
- Apple artifact export policy for standalone Mach-O, universal binaries, app
  bundles, dyld shared-cache projections, kernel collections, and debug sidecars
- code-signing warning model
- evidence package export
- debugger launch handoff for patched revisions
- invalidation/reanalysis plan

Source specs:

- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
- [Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md)

Implementation tasks:

- expose byte windows through project/source/address-space/revision identity
- add patchability query APIs and Workbench status UI
- model original/new byte ranges
- model patch plans and operations
- model patch previews, impacts, revisions, exports, and history
- require source hash precondition
- add diff preview concept
- integrate architecture semantics for assembly/NOP preview
- integrate invalidation/reanalysis
- model derived source revisions
- design no-in-place-cache/kernel rule
- add code-signing, bundle, dyld cache, kernel collection, and debug sidecar
  export warnings
- add automation dry-run APIs and explicit apply/export capabilities

Validation:

- byte-window fixture
- patchability fixture
- patch preview fixture
- assembler diagnostic fixture
- NOP preview fixture
- source hash precondition test
- derived source revision test
- no in-place mutation test
- immutable original hash test
- dyld cache/kernel readonly policy tests
- code-signing warning snapshot
- patch history snapshot
- evidence export snapshot
- automation capability denial test
- warning text review

Risks:

- data loss
- code-signing breakage
- users assume patched system binaries can run

## E18: Diff, Export, And Evidence Packages

Goal:

- make Damsel project facts portable, comparable, and useful for Apple OS build
  archaeology

Primary spec:

- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)

Deliverables:

- analysis snapshot manifest
- export manifest
- redaction policy
- object fingerprint model
- diff project model
- exact/runtime/fuzzy matching passes
- diff findings and ambiguity records
- evidence package format
- Markdown/HTML/JSON report export
- Objective-C surface export
- dyld cache image/dependency diff
- kernel member/KDK/panic diff
- Workbench diff workspace

Implementation tasks:

- add snapshot creation command/API
- add export job model
- add fingerprint computation for procedures, runtime objects, types, xrefs,
  dyld images, and kernel members
- add diff job scheduler integration
- add match confidence and match-reason records
- add conflict/ambiguity records
- add finding pagination
- add match pin/reject annotation transactions
- add report generation from canonical JSON facts
- add export redaction preview and log
- add Workbench change navigator and evidence inspector

Validation:

- snapshot manifest fixture
- no-raw-bytes default export fixture
- function added/removed/changed fixture
- ambiguous match fixture
- ObjC selector/class change fixture
- C++ vtable slot change fixture
- dyld provider/importer/reexport change fixture
- kernel member/KDK mismatch fixture
- evidence package fixture
- redaction log fixture
- Workbench diff navigation smoke

Risks:

- fuzzy matching produces false confidence
- reports leak private paths, comments, strings, or bytes
- BinExport compatibility distorts the Apple-specific model too early
- diff UI starts whole-cache matching eagerly instead of paging and prioritizing
  selected routes

## E19: Native App Trust, Distribution, And Diagnostics

Goal:

- make the Workbench trustworthy as a distributed macOS reverse-engineering
  app that handles private Apple-system artifacts, local helpers, diagnostics,
  updates, plugins, and future debugger capability honestly

Primary spec:

- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)

Deliverables:

- app build identity model
- helper identity model
- local server session token model
- trust status endpoint
- source access grant model
- security bookmark record model
- diagnostics policy and redaction model
- diagnostic bundle export
- helper crash/hang summaries
- signing/notarization/hardened-runtime release gates
- update state model
- plugin trust record model
- debugger trust record model
- Trust and Permissions Workbench panel

Implementation tasks:

- add `TrustPolicy`, `AppBuildIdentity`, `HelperIdentity`,
  `LocalServerSession`, `SourceAccessGrant`, `SecurityBookmarkRecord`,
  `DiagnosticsPolicy`, `DiagnosticBundle`, `CrashReportSummary`,
  `HangReportSummary`, `UpdateState`, `PluginTrustRecord`, and
  `DebuggerTrustRecord`
- update `DamselServerProcess` to launch a token-authenticated loopback helper
- update `DamselAPIClient` to attach local session authentication
- add `/api/trust/status`, `/api/helper/identity`, and diagnostics endpoints
- migrate Workbench project mode from whole-file upload toward source grants
- add default diagnostic redaction for paths, strings, comments, and raw bytes
- record helper termination and stalled-job summaries
- define release signing/notarization validation commands before making
  distribution claims
- keep debugger and plugin capability behind build flavor and trust policy

Validation:

- missing-token denial fixture
- wrong-token denial fixture
- helper identity fixture
- loopback-only helper startup test
- source grant snapshot
- stale bookmark repair fixture
- diagnostics redaction fixture
- helper crash summary fixture
- helper hang summary fixture
- signing state validation for release artifacts when such artifacts exist
- update invalid-signature fixture when update manifests exist
- plugin trust policy fixture
- debugger trust policy fixture

Risks:

- treating helper authentication as dev-only and shipping a soft local API
- leaking source paths or strings through diagnostics
- adding broad debugger/plugin entitlements before the static app needs them
- claiming signed, notarized, or hardened-runtime status before release
  validation proves it
- preserving the whole-file upload path as the Workbench default for large
  Apple artifacts

## E20: Apple Artifact Intake And Source Registration

Goal:

- turn raw Apple artifact opening into a durable project/source workflow that
  supports Mach-O, universal binaries, dyld caches, driver caches, kernel
  collections, KDKs, dSYMs, panic logs, app bundles, and IPSW/OTA-derived asset
  roots

Primary spec:

- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)

Deliverables:

- artifact intake session model
- intake input model
- artifact candidate model
- source registration plan model
- artifact identity model
- source graph and container manifest records
- projection and extraction recipe records
- sidecar candidate and symbolication profile records
- external tool receipt model
- intake problem codes
- CLI project intake commands
- Workbench Open Artifact and artifact overview surfaces

Implementation tasks:

- add lightweight Mach-O and universal classification before full analysis
- register root sources, member sources, sidecars, and derived projections
- expose dyld cache root/subcache/symbols source graph
- add kernel collection/fileset intake path
- add KDK/dSYM scanner and match evidence model
- add panic log attachment plan
- classify app bundles and extracted Apple asset roots
- preserve local and global offsets for extracted/projected members
- keep protected/encrypted iOS app inputs as typed unsupported states
- schedule initial artifact overview and selected-member jobs

Validation:

- standalone Mach-O intake fixture
- universal Mach-O slice fixture
- unknown/protected input negative fixture
- synthetic dyld cache source graph fixture
- missing subcache fixture
- projected image provenance fixture
- synthetic fileset/kernel collection fixture
- KDK/dSYM exact and mismatch fixtures
- panic log fixture
- external tool receipt fixture
- Workbench artifact overview smoke

Risks:

- losing provenance when extracting or projecting members
- silently applying wrong sidecar symbols
- letting terminal preprocessing remain the real user workflow
- over-claiming IPSW or iOS app support before encrypted/protected boundaries
  are explicit

## First Implementation Wave Recommendation

Wave A should include:

1. E1 project/source/address model.
2. E2 fact/provenance database.
3. E3 byte map/annotations/problems.
4. E4 function/xref index.
5. Thin slices of E5 Workbench project shell.
6. Thin slices of E11 paging/jobs/metrics.

Why:

- everything else depends on stable project facts
- it creates visible Hopper-like workflows
- it avoids premature decompiler/debugger work
- it preserves the existing CLI while building the next architecture

## Completion Audit Template

For each epic, record:

- requirement
- source doc
- implementation files
- CLI/API surface
- Workbench surface
- fixture evidence
- real-system evidence, if required
- performance evidence
- remaining risks
- explicit non-goals

Mark an epic complete only when evidence covers its stated scope.
