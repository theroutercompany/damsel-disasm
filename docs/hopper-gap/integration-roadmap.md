# Integration Roadmap

Status: Sequenced roadmap for implementing the Hopper-gap documentation pack.

## Roadmap Principles

- Build durable analysis state before expensive UI.
- Build xrefs before decompiler.
- Build ObjC/C++/kernel domain intelligence before generic scripting or
  patching.
- Treat byte typing, undoable user edits, feature-map navigation, and
  procedure-local names as core infrastructure, not UI polish.
- Treat performance, memory footprint, and cancellation as product features.
- Keep existing point-query commands stable.
- Add features behind new commands and API routes.
- Treat real-cache and kernel evidence as opt-in unless actually run.

The roadmap is intentionally split across companion docs:

- [Analysis Database Blueprint](analysis-database-blueprint.md) owns
  persistence, schema, indexing, migrations, invalidation, and transactions.
- [Native App Polish Plan](native-app-polish-plan.md) owns macOS product fit,
  menus, keyboarding, command palette, accessibility, and error/progress UX.
- [Apple Systems Specialization Plan](apple-systems-specialization-plan.md)
  owns the Damsel-specific Apple systems workflows.
- [End-To-End Workflow Playbooks](end-to-end-workflow-playbooks.md) shows how
  those systems combine into actual reverse-engineering sessions.
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md) owns the
  route, search, navigator, feature-map, and xref backbone.
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
  turns Hopper-style synchronized assembly, graph, hex, feature map, inspector,
  selection, layout, accessibility, export, and Apple runtime graph behavior
  into implementation-ready contracts.
- [Type And Symbolication System Plan](type-symbolication-system-plan.md) owns
  names, types, dSYM/KDK/DWARF, ObjC encodings, C++ RTTI, Swift demangling, and
  symbolication profiles.
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md)
  turns that plan into concrete type records, signature/calling-convention
  stores, local variable records, debug importers, KDK/dSYM profile handling,
  Workbench editors, CLI/API contracts, and validation gates.
- [Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec](swift-runtime-metadata-implementation-spec.md)
  expands Swift from demangled-name facts into Swift section inventory,
  modules, nominal types, fields, protocols, conformances, metadata accessors,
  runtime calls, Swift-ObjC bridge records, Workbench browsers, and large-artifact
  validation.
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
  breaks the debug-info work into source discovery, dSYM/KDK matching,
  compile-unit and line-table import, inline callsites, variable locations,
  type DIE translation, source-line UX, privacy, performance, and validation.
- [Decompiler And IR Strategy](decompiler-ir-strategy.md) owns the long-range
  decompiler path and explains what must be true before pseudocode is credible.
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)
  owns the scripting, MCP, plugin, LLDB, and patching boundaries.
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
  turns Hopper's public Python/MCP evidence into Damsel CLI JSON, batch query,
  read-only local MCP, audited annotation mutation, capability, privacy, and
  Workbench activity-log requirements.
- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md)
  turns Hopper's loader/CPU/tool SDK boundaries into Damsel internal traits,
  versioned manifests, helper-process isolation, and provenance requirements.
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
  owns the safe path from read-only synchronized hex to patch preview,
  assembler/NOP generation, source revisions, invalidation, copy-on-write
  export, and patch transactions.
- [Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
  owns the local debugger path: runtime module matching, static-dynamic route
  mapping, route-backed breakpoints, current-PC overlays, registers, frames,
  read-only memory, helper boundaries, and platform failure handling.
- [Implementation Epic Breakdown](implementation-epic-breakdown.md) translates
  the roadmap into issue-sized epics with dependencies and validation gates.
- [Current Code Integration Map](current-code-integration-map.md) ties those
  epics to the current crates, modules, commands, API server, Workbench files,
  and tests.
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md) defines
  what proof is needed before any milestone can be called complete.
- [Corpus, Benchmark, And Regression Plan](corpus-benchmark-regression-plan.md)
  owns fixture growth, real-artifact evidence, benchmark classes, and regression
  policy.
- [UX Latency And Polish Measurement Plan](ux-latency-polish-measurement-plan.md)
  owns measurable native app responsiveness and polish gates.
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
  turns Hopper's large-binary, large-database, memory, deadlock, and UI
  responsiveness evidence into concrete Damsel source-registration, scheduler,
  indexing, paging, event, cache, metrics, diagnostics, and Workbench store
  requirements.
- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
  turns Hopper's BinDiff/export surfaces and Damsel's Apple-system niche into
  analysis snapshots, object fingerprints, OS-build diffs, evidence packages,
  report exports, redaction policy, and Workbench diff flows.
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
  turns Hopper's signed native app, update, debugger, SDK signing, crash/hang,
  MCP reliability, Apple Gatekeeper, file-access, and Sparkle evidence into
  Damsel's source grants, authenticated helper sessions, diagnostic redaction,
  crash/hang recovery, signing/notarization gates, update state, plugin trust,
  and debugger entitlement readiness.
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
  turns Hopper's dyld extraction, driver-cache, kernel-cache, FAT/local-offset,
  SDK loader, and iOS FairPlay-boundary evidence into Damsel intake sessions,
  artifact candidates, source registration plans, source graphs, projection and
  extraction recipes, KDK/dSYM/panic sidecar matching, and Workbench artifact
  overview flows.
- [Wave 1 Project Foundation Implementation Spec](wave-1-project-foundation-spec.md)
  turns the first implementation branch into concrete model, CLI, and test
  contracts.
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
  expands the first-system implementation plan into project/document kernel,
  typed byte map, object id, route, annotation, local UI server, Workbench, and
  test-gate details.
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
  sequences the semantic assembly, inspector, graph, hex, feature-map, search,
  navigator, route-stack, local API, and Workbench migration layer above the
  project kernel.
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
  sequences the command registry, menu validation, keyboard and command
  palette layer, focus and selection stores, navigation stack, project
  transaction bridge, activity log, job command model, and Apple-system command
  families that should be built before the app surface grows further.
- [Architecture Decision And Risk Register](architecture-decision-risk-register.md)
  records major decisions and risks that must be reviewed before each wave.
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
  provides the current SDK-grounded evidence for document, loader, CPU, tool,
  byte-type, procedure, transaction, scripting, MCP, and extension boundaries.
- [Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
  provides the latest source-to-requirement audit tying Hopper's public release
  API, changelog, tutorial, FAQ, SDK, script, MCP, and workflow evidence to
  Damsel subsystems.
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
  expands the Hopper evidence into an architecture model covering document
  state, typed bytes, procedure records, xrefs, type/signature stores,
  architecture semantics, decompiler boundaries, debugger mapping, automation,
  and performance.
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
  provides the public product and interaction evidence for synchronized views,
  byte typing, navigation, debugger mapping, automation, packaging, and native
  app responsiveness.
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)
  provides the implementation object model and pass pipeline for the ObjC,
  C++/ObjC++, Swift-adjacent, arm64e/PAC, dyld-cache, kernel, KDK/dSYM, and
  panic-log specialization lane.
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
  sequences the native browser layer for ObjC classes/selectors/message sends,
  C++/ObjC++ classes/vtables, IOKit overlays, dyld cache images/providers,
  kernel members/kexts, XNU interfaces, KDK/dSYM symbolication status, panic
  frames, runtime relationship graphs, and runtime problem surfaces.
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
  provides the concrete app migration path for project sessions, store splits,
  command registry, route stack, synchronized representations, jobs, problems,
  transactions, and native Apple runtime browsers.
- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
  provides the persistent procedure database plan needed by graph navigation,
  xrefs, summaries, decompiler-lite output, automation, debugger mapping, and
  runtime owner views.
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md)
  provides the staged implementation path from seeds and CFG normalization
  through architecture semantics, IR, SSA, ABI/type/runtime enrichment, and
  pseudocode.

## Milestone 0: Decision And Scope Lock

### Goal

Confirm that the next major Damsel direction is Apple systems analysis depth,
not generic Hopper clone breadth.

### Deliverables

- accepted docs pack
- explicit non-goals
- issue breakdown
- dependency spike list
- fixture plan

### Exit Criteria

- The project agrees to keep Apple-only focus for this push.
- The project agrees patching/debugger are later.
- The project agrees analysis database is the next foundation.

## Milestone 1: Analysis Project Foundation

### Goal

Create a durable model for projects, sources, address spaces, facts, byte
classifications, functions, references, and annotations.

### Work Items

- Add `AnalysisProject` model.
- Add `AnalysisSource` model.
- Add `AddressSpace` model.
- Add `AnalysisFact` and provenance/confidence model.
- Add in-memory `AnalysisDatabase`.
- Add deterministic JSON snapshot support.
- Add project creation from standalone `BinaryImage`.
- Add project creation from projected shared-cache image.
- Add CLI project manifest/status/facts commands.

### Integration Points

- Reuse `BinaryImage`.
- Reuse `SharedCache` and `ProjectedImageProvenance`.
- Reuse existing serde/output patterns.

### Validation

- fixture project snapshot
- duplicate analysis prevention
- stale analyzer version test
- stable CLI JSON snapshots

### Risks

- Model churn.
- Large JSON snapshots becoming unwieldy.
- Premature persistence backend choice.

## Milestone 1A: Hopper-Style Document Primitives

### Goal

Add the document primitives that Hopper exposes publicly as everyday workflow:
byte classifications, feature map data, user edits, undo/redo, background
analysis state, problems, and semantic rendering hooks.

### Work Items

- Add byte-classification range table.
- Add inferred/user provenance for classifications.
- Add make-code/make-data/make-string/make-procedure/undefined commands.
- Add annotation transaction and undo/redo model.
- Add problem list model.
- Add background job state and cancellation model.
- Add semantic operand/span model for assembly rows.
- Add route-backed feature-map query.
- Add first Workbench feature-map strip and problems panel.

### Integration Points

- Reuse `AnalysisProject`, `AnalysisSource`, and `AddressSpace`.
- Reuse section/symbol/string/function-start metadata as initial
  classification seeds.
- Reuse current Workbench row selection and status surfaces for route sync.

### Validation

- byte-classification fixture snapshot
- manual type conversion undo/redo fixture
- problem-list fixture for ambiguous indirect branch
- feature-map API shape test
- Workbench smoke for route selection and feature-map cursor

### Risks

- Letting mutation APIs become binary patching too early.
- Coarse invalidation that reruns expensive whole-image analysis for small user
  edits.
- Treating feature-map colors as section colors instead of typed byte facts.

## Milestone 1B: Performance Runtime Foundation

### Goal

Replace the alpha upload-and-point-query performance shape with a project
runtime foundation that can scale to large Apple artifacts.

### Work Items

- Add path-based project source registration.
- Add source identity and availability records.
- Add mmap-backed source access where practical.
- Add bounded byte-window service.
- Add job scheduler with priorities and cancellation tokens.
- Add job progress API and event/polling surface.
- Add first index status records.
- Add response-size and query-duration metrics.
- Keep current upload/disasm API as compatibility mode.

### Integration Points

- Reuse `AnalysisProject`, `AnalysisSource`, and `AddressSpace`.
- Reuse current upload path for small compatibility workflows.
- Reuse Workbench request-id stale result protection.
- Reuse `NSTableView` instruction rendering while moving data behind paged
  query stores.

### Validation

- source registration fixture
- missing source fixture
- byte-window fixture
- job lifecycle fixture
- cancellation fixture
- metrics endpoint fixture
- upload compatibility test

### Risks

- Hiding a large architecture change behind a cosmetic performance label.
- Registering local paths without a privacy and diagnostics policy.
- Treating cancellation as UI-only state instead of analyzer checkpoints.
- Leaving Workbench on large unpaged payloads after the backend gets indexes.

## Milestone 1C: Native Trust And Diagnostics Foundation

### Goal

Make the native app trust boundary explicit before broad Workbench
distribution, automation, plugin, update, or debugger work depends on it.

### Work Items

- Add app build identity, helper identity, and trust policy models.
- Add Workbench-mode local server session token.
- Add helper protocol-version and loopback binding validation.
- Add trust status and helper identity endpoints.
- Add source access grant and security bookmark record models.
- Add diagnostics policy, redaction policy, and diagnostic bundle summary.
- Add helper crash/hang summary records.
- Add Trust and Permissions Workbench panel skeleton.
- Define signing, notarization, hardened-runtime, update, plugin, and debugger
  trust states without enabling all those capabilities yet.

### Integration Points

- Reuse `AnalysisProject`, `AnalysisSource`, `ProblemRecord`, `JobRecord`, and
  redaction policy from project/performance milestones.
- Reuse `DamselServerProcess` for initial helper lifecycle changes.
- Reuse `DamselAPIClient` for token-authenticated requests.
- Keep current upload API as compatibility mode while Workbench project mode
  moves to source grants.

### Validation

- missing-token denial fixture
- wrong-token denial fixture
- helper identity fixture
- source grant snapshot
- diagnostics redaction fixture
- helper crash summary fixture
- helper startup timeout fixture

### Risks

- Treating helper authentication as optional because the helper binds locally.
- Leaking raw paths, strings, comments, panic text, or bytes in diagnostics.
- Adding debugger/plugin entitlements to ordinary analysis builds too early.
- Claiming signing or notarization status without validating a real artifact.

## Milestone 1D: Artifact Intake And Source Registration

### Goal

Make opening real Apple artifacts a project-backed workflow rather than a
terminal-preprocessed file upload.

### Work Items

- Add artifact intake session and input models.
- Add artifact candidate and source registration plan models.
- Add source graph and artifact identity records.
- Add lightweight Mach-O/universal classification.
- Add dyld cache root/subcache/symbols source registration.
- Add container manifest records.
- Add projection and extraction recipe records.
- Add KDK/dSYM sidecar candidate and symbolication profile records.
- Add panic log source kind and attachment plan.
- Add app-bundle and extracted-asset-root classification.
- Add artifact overview route and Workbench intake review surface.

### Integration Points

- Reuse `damsel-macho::loader::load` and slice selection.
- Reuse `SharedCacheSession` and projected cache images.
- Reuse source access grants from the trust milestone.
- Reuse job/progress infrastructure from the performance milestone.
- Reuse problem/provenance models from the project foundation.

### Validation

- standalone Mach-O intake snapshot
- universal Mach-O slice snapshot
- unknown input snapshot
- synthetic dyld split-cache source graph
- missing subcache problem
- projected cache image provenance snapshot
- synthetic fileset/kernel collection manifest
- dSYM/KDK mismatch fixture
- panic log attachment fixture
- Workbench artifact overview smoke

### Risks

- Treating artifact intake as a UI file picker instead of a durable project
  source graph.
- Losing original-container routes when projecting or extracting members.
- Trusting mismatched KDK/dSYM data.
- Suggesting iOS app support without clearly rejecting encrypted/protected
  inputs.

## Milestone 2: Function And Xref Index

### Goal

Lift local disassembly analysis into persistent whole-image function and
reference indexes.

### Work Items

- Implement function seed collector.
- Implement recursive-descent function builder.
- Implement persistent procedure records, ranges, owners, and recovery status.
- Implement basic-block and edge records as project facts.
- Implement function overlap/conflict handling.
- Implement xref table.
- Implement caller/callee index.
- Implement string/data ref index.
- Implement local labels, register aliases, stack-local placeholders, and
  procedure problem records.
- Add `project functions`.
- Add `project xrefs`.
- Add function detail endpoint.

### Integration Points

- Reuse `DisassemblyAnalysis`.
- Reuse existing branch/call/data/import/reference annotations.
- Reuse cache link enrichment where cache context exists.

### Validation

- branch fixture function graph
- jump-table fixture
- import-rich fixture
- ObjC IMP seed fixture
- CLI JSON snapshots

### Risks

- Over-claiming function bounds.
- Treating tail calls as ordinary calls.
- Bad performance on large executable ranges.

## Milestone 3: Graph Navigation

### Goal

Make function CFGs and xref/call graphs queryable and visible.

Workbench integration details for graph, hex, assembly selection sync,
feature-map behavior, command routing, and app responsiveness live in
[Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md).

### Work Items

- Add graph records.
- Add CFG graph builder from function records.
- Add call graph builder from xrefs.
- Add graph JSON/DOT export.
- Add UI API for graph data.
- Add Workbench graph/list-mode viewer.
- Add graph annotations for labels, comments, tags, colors, and grouped nodes.
- Add minimap route state.

### Integration Points

- Reuse function/block/edge records.
- Reuse Workbench instruction selection and jump behavior.

### Validation

- graph fixture snapshots
- UI API graph shape test
- Workbench smoke with branch fixture

### Risks

- Layout complexity distracting from data correctness.
- Giant graphs becoming unusable.

## Milestone 4: Objective-C Explorer

### Goal

Turn ObjC metadata into navigable runtime analysis.

Implementation details for normalized selectors, classes, categories,
protocols, methods, ivars, properties, message-send candidates, dispatch
bindings, and cache-wide selector indexes live in
[Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md).

### Work Items

- Normalize ObjC metadata objects into project database.
- Add selector/class/method indexes.
- Add type-encoding decoder.
- Add selector implementer/caller queries.
- Add method IMP to function links.
- Implement message-send pass.
- Add CLI and API ObjC explorer commands.
- Add Workbench ObjC browser.

### Integration Points

- Reuse `ObjcMetadata`.
- Reuse `DisassemblyObjcUse`.
- Reuse recovered values and import/stub refs.
- Reuse cache-aware analysis for shared-cache ObjC frameworks.

### Validation

- ObjC fixture class/method snapshots
- selector caller snapshots
- type encoding snapshots
- message-send target fixture

### Risks

- Dynamic dispatch ambiguity.
- Swizzling and categories.
- False receiver-class inference.

## Milestone 5: C++ And Objective-C++ Explorer

### Goal

Add ABI-aware C++ recovery for Objective-C++ and XNU/IOKit workflows.

### Work Items

- Choose demangling crate.
- Add demangled name records.
- Add C++ symbol classifier.
- Add vtable discovery.
- Add typeinfo discovery.
- Add virtual callsite recognizer.
- Add inheritance graph.
- Add IOKit recognizers.
- Add CLI/API/Workbench C++ views.

### Integration Points

- Reuse function index.
- Reuse xref index.
- Reuse pointer-auth facts where needed.
- Reuse type/debug-info system when available.

### Validation

- C++ vtable fixture
- virtual call fixture
- multiple inheritance fixture
- IOKit-style fixture

### Risks

- ABI edge cases.
- Stripped RTTI.
- Multiple inheritance complexity.
- Kernel-specific object patterns.

## Milestone 6: Kernel Collection And XNU Support

### Goal

Support XNU-style system binaries as first-class Damsel projects.

### Work Items

- Add fileset/kernel collection loader.
- Add member address spaces.
- Add kext/member identity model.
- Add kext dependency graph.
- Add KDK/dSYM matching.
- Add kernel symbol import.
- Add IOKit class recovery integration.
- Add syscall/Mach trap/MIG recognizers.
- Add panic symbolication CLI.
- Add Workbench kernel browser.

### Integration Points

- Reuse Mach-O loader per fileset member.
- Reuse function/xref/index database.
- Reuse C++/IOKit analysis.
- Reuse debug-info integration.

### Validation

- synthetic fileset fixture
- KDK mismatch fixture
- panic text fixture
- real KDK/kernel collection opt-in smoke

### Risks

- OS-version drift.
- KDK mismatch.
- Hard-to-create public fixtures.
- Large binary performance.

## Milestone 7: Pointer Authentication And Chained Fixups

### Goal

Make arm64e pointer-auth and chained fixup semantics reusable across ObjC,
C++, shared-cache, and kernel analyses.

### Work Items

- Add pointer fact model.
- Add pointer-auth format parser/interpreter.
- Add chained-fixup materialization pass.
- Integrate pointer facts into ObjC metadata.
- Integrate pointer facts into C++ vtables.
- Integrate pointer facts into kernel tables.
- Add display and JSON contracts.

### Validation

- arm64e fixture
- chained fixup fixture
- ObjC signed IMP fixture if possible
- C++ signed vtable fixture if possible

### Risks

- Pointer format variation.
- Over-stripping pointers.
- Confusing raw and interpreted addresses.

## Milestone 8: Type And Debug-Info Integration

### Goal

Use dSYM, DWARF, KDK, unwind, and line tables to improve function, type, and
source context.

### Work Items

- Choose DWARF parsing strategy.
- Add type/name/signature/debug-info model types.
- Add provenance, precedence, stale, conflict, and transaction records.
- Decode ObjC method, ivar, and property encodings into type facts.
- Import C++ demangled names and signature candidates.
- Import Swift demangled name facts.
- Model calling conventions and hidden Apple/ObjC/C++ arguments.
- Add stack frame, local variable, register alias, and local label records.
- Add dSYM UUID matching.
- Add KDK discovery/config.
- Import line tables.
- Import function symbols.
- Import types.
- Import function signatures.
- Import source files, compile units, inline callsites, and variables where
  available.
- Attach debug info to functions and runtime objects.
- Add project type database for structs, unions, enums, arrays, pointers, and
  imported debug/KDK types.
- Add user-authored type definitions and export/import snapshots.
- Add Workbench type navigator, signature editor, local variables panel,
  symbolication profile panel, and source panel.
- Add type/signature/profile CLI JSON and local UI API contracts.

### Validation

- ObjC type encoding snapshot
- C++ demangle signature snapshot
- Swift demangle fact snapshot
- calling convention snapshot
- stack/local/register alias snapshot
- dSYM fixture
- UUID mismatch test
- line table snapshot
- type import snapshot
- type conflict snapshot
- type import/export round trip

### Risks

- Dependency size and complexity.
- Slow parsing.
- Debug info conflicts with runtime names.
- Weak demangler facts look exact.
- Type editor changes invalidate too much analysis.

## Milestone 9: Decompiler-Lite Summaries

### Goal

Provide high-level function summaries that are useful before full pseudocode.

### Work Items

- Add function summary pass.
- Add stack/argument/return summaries.
- Add ObjC message summary rendering.
- Add C++ virtual call summary rendering.
- Add switch/jump-table summaries.
- Add unresolved-fact section.
- Add UI summary pane.

### Validation

- semantic-switch summary
- import-rich summary
- ObjC message summary
- C++ virtual call summary

### Risks

- Summary sounding more certain than evidence.
- Premature pseudocode expectations.

## Milestone 10: Read-Only Automation

### Goal

Expose scripting/automation over the analysis database without binary mutation.

### Work Items

- Implement structured CLI JSON and batch mode first.
- Add read-only query APIs.
- Add a local read-only MCP surface over the same route/object schemas.
- Add annotation mutation APIs only after capability checks and transactions
  exist.
- Add audit log and Workbench-visible automation activity.
- Add graph/report export helpers.
- Shape the query surface so CLI, UI, MCP, and future scripts expose the same
  stable object ids, routes, facts, xrefs, problems, and annotations.

### Validation

- script/query fixture
- audit log fixture
- report export fixture
- MCP-shaped JSON contract fixture
- permission-denial fixture
- privacy/redaction fixture

### Risks

- API stabilization burden.
- Accidental write/mutation scope creep.
- AI/MCP tools bypassing the same permission and undo model used by UI/CLI.
- Tool responses become prose summaries instead of route-backed evidence.

## Milestone 10A: Extension Boundary

### Goal

Define controlled extension points without exposing unstable Rust internals or
running untrusted code inside the Workbench process.

### Work Items

- Define internal loader, sidecar importer, analyzer, renderer/exporter, and
  Workbench command traits.
- Define versioned extension manifests.
- Define capability groups.
- Define fact/problem emission schemas.
- Define plugin provenance fields.
- Define cancellation and metrics hooks.
- Define helper-process isolation for future external extensions.
- Spike one toy analyzer extension through the manifest path after the project
  API is stable.

### Validation

- manifest validation fixture
- incompatible-version fixture
- capability-denial fixture
- extension cancellation fixture
- helper crash isolation fixture
- plugin-produced fact provenance snapshot

### Risks

- Freezing APIs before the project database is stable.
- Letting extensions write facts without provenance or invalidation rules.
- Treating loader, CPU/architecture semantics, analyzer, exporter, and UI tool
  extensions as one generic plugin class.

## Milestone 10B: Diffing, Export, And Evidence Packages

### Goal

Make Damsel's analysis portable and comparable, with special strength for
Apple OS build archaeology.

### Work Items

- Add analysis snapshot manifests.
- Add export manifest and redaction policy.
- Add object fingerprints for functions, runtime objects, types, xrefs, dyld
  images, and kernel members.
- Add diff project model.
- Add exact/runtime/fuzzy matching passes.
- Add diff findings and ambiguity records.
- Add paged diff findings API.
- Add Markdown/HTML/JSON report exports.
- Add Objective-C surface export.
- Add dyld cache image/dependency diff.
- Add kernel member/KDK/panic diff.
- Add Workbench diff workspace and evidence inspector.

### Validation

- snapshot manifest fixture
- no-raw-bytes default export fixture
- function added/removed/changed fixture
- ObjC selector/class change fixture
- C++ vtable change fixture
- dyld provider/importer/reexport diff fixture
- kernel member/KDK mismatch diff fixture
- evidence package redaction fixture
- Workbench diff navigation smoke

### Risks

- Fuzzy matching creates false confidence.
- Reports leak private strings, comments, paths, or bytes.
- BinExport compatibility pressures Damsel into lossy generic models before
  Apple-domain evidence is mature.
- Diff UI eagerly computes whole-cache matches instead of paging and
  prioritizing selected routes.

## Milestone 11: Debugger Bridge

### Goal

Bridge live LLDB state to Damsel's static database.

### Work Items

- LLDB transport spike.
- Process/session model.
- Runtime module model.
- Runtime slide mapping.
- Static-to-dynamic route resolver.
- Project route to runtime address resolver.
- Breakpoint support.
- Current-PC overlay model.
- Register/memory read.
- Current frame to function mapping.
- Trace capture.
- Platform failure problem codes for SIP, hardened runtime, missing debugger
  tooling, source mismatch, and dyld-cache mismatch.

### Validation

- local fixture executable debug smoke
- address mapping test
- breakpoint test where environment permits
- register/backtrace route mapping test
- read-only memory window test
- mismatch problem fixture

### Risks

- SIP and entitlements.
- Environment-specific failures.
- Debugger UX overshadowing static analysis.
- Runtime address mapping accidentally treating path matches as stronger than
  UUID/hash/cache identity.

## Milestone 12: Patching And Binary Mutation

### Goal

Add explicit, opt-in patching only if product direction requires it.

### Work Items

- policy/design doc
- read-only synchronized hex/data view
- patchability model
- source revision model
- patch preview model
- assembler integration spike
- AArch64 NOP generation
- byte patch model
- diff preview
- patch transaction model with undo/redo over project state
- invalidation and reanalysis plan
- copy-on-write export path
- code-signing warning system
- no shared-cache/kernel mutation by default

### Validation

- byte-window fixture
- patch preview fixture
- fixture patch round-trip
- assembler diagnostic fixture
- source hash precondition test
- explicit write-mode tests
- no accidental mutation tests

### Risks

- Data loss.
- Code-signing breakage.
- User surprise.
- Patching a projected cache or kernel source as if it were an ordinary file.

## Parallel Workstreams

Some work can run in parallel after Milestone 1:

- Workstream A: function/xref/graph.
- Workstream B: ObjC explorer.
- Workstream C: C++ ABI spike.
- Workstream D: kernel/fileset spike.
- Workstream E: Workbench navigation shell.

Do not parallelize by letting every workstream invent its own model. The
analysis database and fact/provenance vocabulary must be shared.

## Suggested First Issues

1. Add `docs/hopper-gap` pack to cross references.
2. Add `AnalysisSource` and `AddressSpace` models.
3. Add `AnalysisFact` with source/confidence.
4. Add in-memory `AnalysisDatabase`.
5. Add `ByteClassification` range table and snapshot.
6. Add `UserAnnotation` transaction model with undo/redo.
7. Add standalone `project create` fixture snapshot.
8. Add function seed collector from symbols/exports/function starts/ObjC IMPs.
9. Add xref table from existing `DisassemblyAnalysis`.
10. Add `project xrefs --to/--from` command.
11. Add feature-map API over byte classifications.
12. Add ObjC selector implementer index.
13. Spike C++ demangler crate.
14. Spike fileset loader using a small fixture.

## Completion Definition For The Hopper-Gap Push

The push is not complete when docs exist or when a graph renders once. It is
complete only when:

- Damsel can create and reopen a project database.
- Damsel can analyze a representative standalone Mach-O into durable functions
  and xrefs.
- Damsel can analyze a projected shared-cache image with cache-aware links.
- Damsel can browse ObjC classes/selectors/methods and selector callers.
- Damsel can recover basic C++ vtables/typeinfo in fixtures.
- Damsel can open a kernel/fileset fixture and represent members/kexts.
- Workbench can navigate function list, assembly, graph, inspector, and xrefs.
- CLI JSON contracts exist for each major object.
- Fixture, snapshot, and opt-in real-evidence gates document what actually ran.
