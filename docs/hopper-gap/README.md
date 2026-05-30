# Hopper Gap Documentation Pack

Status: Planning and research pack. This is not an implementation claim.

Audience: Damsel maintainers deciding what to build after the current
Mach-O, dyld shared-cache, Objective-C metadata, and disassembler V2 alpha
work.

Primary question: what would Damsel need to implement to come closest to
Hopper Disassembler for the Apple-focused lane: macOS, iOS, dyld shared
caches, XNU-style system binaries, Objective-C, C++, and Objective-C++.

## Pack Map

- [Feature Gap Inventory](feature-gap-inventory.md): the missing feature
  families, grounded in Hopper's public surface and Damsel's current
  implementation.
- [Target Architecture](target-architecture.md): how the missing systems fit
  into the current Rust core, Mach-O loader, CLI, web UI, and native macOS
  workbench.
- [Feature Specifications](feature-specifications.md): concrete user-facing
  and internal contracts for the major features.
- [Implementation Guide](implementation-guide.md): subsystem-by-subsystem
  implementation notes, likely model changes, parser/analysis algorithms,
  fixtures, and validation gates.
- [Current Code Integration Map](current-code-integration-map.md): repo-grounded
  mapping from today's crates, commands, UI server, Workbench files, and tests
  into the target architecture.
- [Wave 1 Project Foundation Implementation Spec](wave-1-project-foundation-spec.md):
  concrete first-branch scope, model types, CLI/API contracts, tests, and review
  gates for project/source/address/object foundations.
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md):
  implementation-ready plan for the shared Rust project/document kernel,
  typed-byte database, object ids, facts, routes, annotations, jobs, UI server
  migration, Workbench migration, performance budgets, and test gates required
  by Hopper-level interaction.
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md):
  implementation plan for semantic assembly rows, route-backed selection,
  inspector modules, graph/hex/feature-map contracts, project search,
  navigators, Workbench store migration, local UI API endpoints, and latency
  gates.
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md):
  implementation-ready plan for Hopper-class synchronized function graphs,
  layout jobs, feature-map lanes, bounded hex/data windows, canonical route
  selection, assembly/graph/hex/inspector synchronization, graph export,
  accessibility fallbacks, performance budgets, and Apple runtime graph
  extensions.
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md):
  implementation-ready plan for the typed command registry, menu validation,
  keyboard shortcuts, command palette, focus model, route-backed selection,
  navigation stack, undo/redo transactions, activity log, job dispatch, and
  Apple-system command families that make the native Workbench feel like a
  coherent Hopper-class document app.
- [Implementation Epic Breakdown](implementation-epic-breakdown.md):
  issue-sized epics, dependencies, deliverables, risks, and validation gates for
  turning this pack into real work.
- [Architecture Decision And Risk Register](architecture-decision-risk-register.md):
  proposed decisions, deferred choices, high-risk failure modes, and mitigation
  rules for the large refactor program.
- [Integration Roadmap](integration-roadmap.md): sequencing, milestones,
  dependencies, quality gates, and what should deliberately wait.
- [Research Sources](research-sources.md): source-grounded research notes and
  why each source matters.
- [Hopper Public Research Notes](hopper-public-research.md): deeper public-web
  research on Hopper's official tutorial/homepage/FAQ/download/debugger-server
  pages plus public SDK/API/plugin evidence.
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md):
  expanded second-pass research correcting the earlier underestimation of
  Hopper's document architecture, byte-typing database, editable graph state,
  automation boundaries, Apple-runtime support, performance posture, and app
  product architecture.
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md):
  current public Hopper SDK 6.3.0 research covering loader, CPU, tool,
  document, disassembled-file, procedure, byte-type, transaction, Python API,
  MCP, and extension boundaries.
- [Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md):
  source-to-requirement audit using Hopper's official release API, changelog,
  tutorial, FAQ, SDK 6.3.0 headers, public scripts, MCP wrappers, and workflow
  evidence to map public Hopper features to Damsel subsystems.
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md):
  expanded public-web and SDK evidence pass that reconstructs Hopper as a
  document, byte-type, procedure, xref, type, transaction, architecture
  semantics, decompiler, debugger, automation, and native app architecture.
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md):
  public research on Hopper tutorial workflows, synchronized views, byte
  typing, inspector behavior, navigation, debugger surface, automation, package
  clues, and what the current Damsel Workbench lacks by comparison.
- [Hopper Feature Parity Matrix](hopper-feature-parity-matrix.md):
  Hopper-feature-by-feature parity targets, Damsel current state, required
  subsystems, and priority.
- [Product Requirements](product-requirements.md): competitive product
  requirements, user personas, feature tiers, and product completion criteria.
- [Competitive UX And App Feature Plan](competitive-ux-app-plan.md): the
  Hopper-competitive app experience, Workbench workflows, app polish, and
  UX-specific acceptance gates.
- [End-To-End Workflow Playbooks](end-to-end-workflow-playbooks.md): concrete
  user journeys for standalone triage, dyld caches, ObjC selectors, C++/IOKit,
  kernel/KDK/panic, search, summaries, and automation.
- [Workbench Information Architecture](workbench-information-architecture.md):
  target app layout, navigators, menus, routes, object browsers, and migration
  from the current three-pane Workbench.
- [Performance And Responsiveness Architecture](performance-responsiveness-architecture.md):
  latency budgets, background analysis, paging, caching, instrumentation, and
  performance refactors.
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md):
  implementation plan for path-based source registration, mmap-backed source
  access, job scheduling, analysis priorities, indexed paging, event streams,
  Workbench store splits, metrics, diagnostics, and large Apple artifact
  performance gates.
- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md):
  implementation plan for Damsel analysis snapshots, Apple-system-aware
  diffs, object fingerprints, dyld/cache/kernel/ObjC/C++ comparison, evidence
  packages, report exports, Workbench diff UI, privacy redaction, and future
  BinExport-like interoperability.
- [Corpus, Benchmark, And Regression Plan](corpus-benchmark-regression-plan.md):
  fixture tiers, real-artifact evidence, benchmark classes, regression gates,
  and corpus expansion for Hopper-gap work.
- [UX Latency And Polish Measurement Plan](ux-latency-polish-measurement-plan.md):
  measurable Workbench latency, keyboard, cancellation, error, and large-list
  quality bars.
- [Analysis Database Blueprint](analysis-database-blueprint.md): concrete
  storage, schema, indexing, provenance, migration, and invalidation plan for
  the Hopper-like workspace core.
- [API And Data Contracts](api-data-contracts.md): project, source, job, query,
  function, xref, graph, ObjC, C++, kernel, cache, annotation, and metrics API
  contracts.
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md): shared
  route/object/search/index model for navigators, xrefs, feature map, command
  palette, scripting, MCP, and debugger mapping.
- [Native App Polish Plan](native-app-polish-plan.md): precise expectations for
  macOS document behavior, menus, keyboarding, command palette, accessibility,
  errors, and interaction finish.
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md):
  concrete migration plan for splitting the current Workbench store, adding
  project sessions, route/navigation state, command registry, synchronized
  assembly/graph/hex/feature-map surfaces, jobs, problems, transactions, and
  native Apple runtime browsers.
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md):
  implementation plan for signed/notarized distribution, hardened-runtime and
  debugger entitlement policy, source access grants, local helper security,
  diagnostic redaction, crash/hang recovery, update channels, plugin trust, and
  Workbench trust UI.
- [Apple Systems Specialization Plan](apple-systems-specialization-plan.md):
  how Damsel's dyld-cache, kernel, ObjC/C++/Swift, arm64e, KDK, and panic
  workflows should become the product advantage.
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md):
  implementation plan for classifying, registering, expanding, projecting, and
  enriching standalone Mach-O files, universal binaries, dyld shared caches,
  driver caches, kernel collections, KDKs, dSYMs, panic logs, app bundles, and
  IPSW/OTA-derived Apple artifacts.
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md):
  implementation-ready runtime object model for ObjC, dispatch, dyld shared
  cache, C++/ObjC++, Swift-adjacent facts, arm64e/PAC/chained fixups, kernel
  collections, KDK/dSYM profiles, and panic-log routing.
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md):
  implementation-ready native Workbench browser plan for ObjC classes,
  selectors, message-send evidence, C++/ObjC++ classes, vtables, IOKit
  overlays, dyld cache images/providers, kernel collection members, KDK/dSYM
  symbolication state, XNU interfaces, panic frames, relationship graphs,
  runtime problems, paged APIs, commands, and performance gates.
- [Type And Symbolication System Plan](type-symbolication-system-plan.md):
  provenance-aware names, types, dSYM/KDK/DWARF, ObjC encodings, C++ RTTI,
  Swift demangling, user overrides, and symbolication profiles.
- [Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md):
  implementation-ready plan for Hopper-class type records, signatures,
  calling conventions, local variables, debug-info imports, KDK/dSYM profiles,
  ObjC encodings, C++ demangling/RTTI, Swift name facts, Workbench editing,
  CLI/API contracts, automation, and validation.
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md):
  implementation-ready plan for debug-source discovery, dSYM/KDK matching,
  DWARF compile units, source files, line tables, inline callsites, variables,
  type DIE import, symbolication profiles, Workbench source-line UX, privacy,
  performance, CLI/API contracts, and validation.
- [Decompiler And IR Strategy](decompiler-ir-strategy.md): staged path from
  function summaries through low-level IR, SSA, type/data-flow, Apple runtime
  overlays, and honest pseudocode.
- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md):
  implementation plan for persistent procedure records, seeds, ranges, basic
  blocks, call graph, local labels, register aliases, stack locals, conflicts,
  and runtime ownership.
- [Decompiler, Function Recovery, And IR Implementation Spec](decompiler-function-ir-implementation-spec.md):
  implementation-ready sequence from procedure seeds through architecture
  semantics, low-level IR, SSA, stack/local recovery, ABI modeling, runtime
  overlays, control-structure recovery, and pseudocode.
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md):
  implementation-ready plan for turning decompiler output into a native
  Workbench representation with pseudocode routes, line/token models, source
  maps, confidence, issue records, user correction transactions, Apple runtime
  overlays, jobs, caches, exports, and responsiveness gates.
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md):
  safe sequencing for scripting, MCP/AI tools, plugins/SDK boundaries, LLDB
  bridging, and future patching.
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md):
  implementation architecture for CLI JSON automation, batch operations,
  read-only local MCP, audited annotation mutation, capabilities, privacy,
  activity logs, Workbench integration, and future script bindings.
- [Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md):
  implementation architecture for Damsel loader, sidecar, architecture
  semantics, analyzer, renderer/exporter, and Workbench command extension
  boundaries without prematurely freezing unstable Rust internals.
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md):
  implementation architecture for synchronized hex/data views, patch preview,
  assembler/NOP generation, copy-on-write source revisions, patch
  transactions, invalidation, export, Workbench UX, and safety policy.
- [Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md):
  implementation architecture for local debugger sessions, runtime module
  matching, route-backed breakpoints, current-PC overlays, registers, frames,
  read-only memory, LLDB helper boundaries, and platform failure handling.
- [Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md):
  implementation-ready native debugger Workbench plan for launch/attach,
  debug targets, module matching, route-backed breakpoints, stop events,
  stepping, registers, frames, memory windows, ObjC/C++/arm64e runtime
  interpretation, dyld shared-cache runtime mapping, LLDB helper isolation,
  permissions, diagnostics, exports, and validation gates.
- [Refactor And Subsystem Plan](refactor-subsystem-plan.md): the large
  subsystems and deep refactors required for a real app, not just small feature
  patches.
- [Quality And Validation Plan](quality-validation-plan.md): documentation,
  model, analysis, UI, performance, and real-evidence gates.
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md):
  requirement-to-evidence map for future completion audits and implementation
  readiness checks.

## Current Damsel Baseline

Damsel is already a static Mach-O analysis CLI focused on Apple Silicon
binaries. The root README currently lists standalone Mach-O parsing, arm64 and
arm64e coverage, structured binary views, AArch64 disassembly, and Wave 1/2/3
dyld shared-cache inventory, symbolication, projected-image analysis, and
cache-native linkage exploration as implemented.

The current docs say the planned next frontier is not basic decoding. It is
graph and trace workflows, deeper system-framework spelunking, cross-image
navigation, and eventual debugger integration. That matters: this pack assumes
that the current decoder/cache base is real and should be lifted into a richer
interactive analysis system instead of rewritten from scratch.

Important current building blocks:

- `damsel-core::BinaryImage` is the per-image analysis object.
- `damsel-core::ObjcMetadata` already records classes, protocols, categories,
  methods, properties, ivars, selector refs, class refs, and method IMPs.
- `damsel-core::DyldMetadata` already records imports, rpaths, exports,
  function starts, chained-fixup presence, bindings, stubs, and helpers.
- `damsel-core::DisassemblyAnalysis` already records basic blocks, control-flow
  edges, imports, ObjC references, cache links, recovered values, jump tables,
  and summary metrics.
- `damsel-core::SharedCache` and `damsel-macho::SharedCacheSession` already
  provide first-class dyld shared-cache loading, projection, symbolication, and
  dependency/provider/importer/reexport queries.
- `damsel-cli cache disasm --analysis` is currently the natural place where a
  projected cache image and live cache session coexist, which makes it the
  existing path for cache-aware enrichment.
- The native Damsel Workbench intentionally launches the existing CLI UI server
  instead of rewriting the decoder stack.

## Hopper-Distance Thesis

Hopper is not just a decoder. Its public surface combines:

- native macOS interaction
- automatic analysis when a file opens
- byte typing into code, data, strings, procedures, and unknown regions
- symbol and string browsers
- contextual inspector
- assembly, CFG, pseudo-code, and hex representations
- Objective-C and Swift awareness
- references and reverse references
- annotations, tags, labels, comments, and colors
- debugger integration
- scripting and SDK extension points
- binary modification tools
- AI/MCP integration as a document automation surface

Damsel should not chase generic file-format breadth first. Hopper supports
Mach-O, PE, ELF, multiple CPU families, decompilation, debugging, patching, and
extension SDKs. Damsel's credible advantage is narrower: be unusually strong
for Apple system binaries and Apple language/runtime metadata.

The closest Apple-focused path is:

1. Add a persistent analysis database and whole-image auto-analysis.
2. Turn local disassembly summaries into navigable function, xref, data, and
   graph indexes.
3. Deepen Objective-C recovery from metadata display into dispatch-aware
   navigation.
4. Add C++ and Objective-C++ recovery, especially vtables, RTTI, constructors,
   destructors, virtual dispatch, and IOKit idioms.
5. Add XNU kernelcache, kernel collection, KDK/dSYM, kext, IOKit, syscall,
   Mach trap, and MIG-aware workflows.
6. Add arm64e pointer-auth and chained-fixup semantics as first-class analysis
   facts.
7. Only then build a decompiler-grade IR and pseudocode surface.
8. Defer patching and live debugging until static navigation is excellent and
   the read-only policy has been explicitly revisited.

## Scope

In scope:

- macOS arm64 and arm64e userland binaries
- iOS arm64 and arm64e userland Mach-O binaries when legally available in
  decrypted/analyzable form
- dyld shared caches and projected images
- XNU-style kernelcaches, kernel collections, kexts, DriverKit-adjacent
  metadata, KDK/dSYM symbol packs, and panic/backtrace symbolication
- Objective-C runtime metadata and message-send analysis
- C++ and Objective-C++ ABI-driven recovery
- Swift only where it touches the Apple systems analysis lane: demangled symbol
  navigation, runtime metadata placeholders, and interop with ObjC/C++ surfaces

Out of scope for the first Hopper-gap push:

- Windows PE and ELF parity
- non-Apple CPU expansion
- binary mutation as a default workflow
- live process injection
- jailbreak-only workflows as a product requirement
- source-code recovery claims that imply recompilable output

## Vocabulary

`Hopper parity` in this pack means close to Hopper in user workflow, not a
feature-for-feature clone. Damsel can be closer to Hopper in Apple systems
workflows while still omitting PE/ELF, patching, or general CPU SDK surfaces.

`Analysis database` means the persistent, queryable project state that records
what Damsel knows about every byte, address, symbol, function, basic block,
cross-reference, runtime object, type, and user annotation.

`Fact` means a record derived from a source with provenance and confidence:
Mach-O load command, section, symbol table, export trie, chained fixup, ObjC
metadata, Capstone instruction detail, inferred CFG edge, KDK symbol, DWARF
type, user label, or manual override.

`Projection` means the existing Damsel concept of materializing a cache image
into per-image analysis while preserving shared-cache provenance.

`Kernel collection` is used broadly here for XNU-style kernelcache containers
including modern fileset-based kernels and collections. Exact file naming and
layout varies by OS generation and should be represented by typed container
models rather than string heuristics.

## Quality Bar

The docs in this pack are written as buildable architecture, not generic
reverse-engineering inspiration. Each proposed feature should eventually have:

- a model addition or an explicit reason it can reuse an existing model
- a loader/parser responsibility
- an analysis responsibility
- CLI output contract
- UI/workbench behavior
- fixture strategy
- real-system evidence strategy
- failure modes and typed errors
- performance expectation
- acceptance criteria

## Non-Negotiable Design Constraints

- Keep Apple focus unless a separate product decision broadens the project.
- Preserve existing standalone Mach-O behavior.
- Preserve current shared-cache point-query commands while adding higher-level
  indexes on top.
- Keep read-only analysis as the default. Any patching or process mutation must
  be separately designed.
- Keep outputs deterministic and JSON-stable.
- Keep ambiguous analysis explicit instead of inventing certainty.
- Use provenance everywhere. A symbol from KDK DWARF, an export trie, an ObjC
  method list, and a user label are different strengths of evidence.
- Treat real dyld shared-cache and kernelcache coverage as opt-in evidence
  until the environment actually provides those corpora.
