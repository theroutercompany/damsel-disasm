# Research Sources

Status: Source notes used by the Hopper-gap documentation pack.

This document intentionally separates external source grounding from Damsel
implementation evidence. The goal is not to copy product claims, but to anchor
planning in public tool surfaces and platform specifications.

## Hopper Public Surface

For the deeper Hopper-specific research pass, see
[Hopper Public Research Notes](hopper-public-research.md). The notes summarize
official public pages plus public SDK/API/plugin evidence and mark inferred
architecture separately from explicit public claims.

For the second-pass correction after the initial research was judged too
shallow, see
[Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md).
That addendum treats Hopper as a document-centric mutable analysis database and
uses public web evidence to map the likely source/container, typed-byte,
architecture-semantics, procedure, annotation/xref, runtime-overlay, UI,
automation, debugger, and extension layers.

For the current SDK-grounded pass, see
[Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md).
That document inspects the official public Hopper SDK 6.3.0 archive, SDK
documentation, Objective-C headers, sample plugins, Python API stubs, current
release API, and recent changelog entries.

For the source-to-requirement audit pass, see
[Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md).
That document uses Hopper's official release API, changelog, tutorial, FAQ,
debugger page, SDK 6.3.0 headers, public scripts, public MCP wrappers, and
workflow writeups to map Hopper's public feature families to concrete Damsel
subsystems and acceptance criteria.

For the type/signature/symbolication implementation pass, see
[Type, Signature, Symbolication, And Debug-Info Implementation Spec](type-signature-symbolication-implementation-spec.md).
That document uses Hopper's homepage, tutorial, FAQ, download page, changelog,
debugger page, public SDK 6.3.0 headers, public Python stubs/scripts, public
MCP wrappers, macOS dyld shared-cache workflow writeups, and external
DWARF/C++ ABI/KDK sources to define Damsel's type records, signatures, calling
conventions, local variables, dSYM/DWARF import, KDK profiles, ObjC encoding
import, C++/Swift name facts, Workbench editors, CLI/API contracts, and
validation gates.

For the dedicated debug-info implementation pass, see
[DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md).
That document uses Hopper's public SDK debug-data loader surface, Hopper's
document/procedure/type headers, LLVM source-level debug-info and `dsymutil`
documentation, DWARF standard references, Apple debugging information
documentation, Apple KDK acquisition context, and current Damsel code evidence
to define debug-source discovery, dSYM/KDK matching, compile units, source
files, line tables, inline callsites, variable locations, type DIE import,
symbolication profiles, Workbench source-line UX, privacy controls, performance
rules, and validation gates.

For the performance/scheduler/indexing implementation pass, see
[Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md).
That document uses Hopper's public changelog performance work, SDK background
process and loader progress APIs, tutorial representation model, and current
Damsel UI/server code evidence to define Damsel's path-based source
registration, job scheduler, indexing, paging, event, cache, metrics, and
Workbench responsiveness architecture.

For the decompiler/pseudocode Workbench implementation pass, see
[Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md).
That document uses Hopper's official homepage, tutorial, FAQ, and prior
changelog/source notes in this pack to separate backend decompiler work from
the native app representation work: pseudocode routes, line/token/source-map
contracts, confidence and issue display, user correction transactions,
ObjC/C++/XNU overlays, jobs, caches, performance budgets, and exports.

For the diff/export/evidence implementation pass, see
[Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md).
That document uses Hopper's BinDiff/BinExport changelog entries, assembly and
pseudocode export reliability entries, Objective-C header export entries,
public SDK export/type/document surfaces, BinExport ecosystem documentation,
and public Hopper script workflows to define Damsel's Apple-system-aware
snapshot, diff, export, evidence package, report, redaction, and Workbench diff
architecture.

For the native app trust/distribution/diagnostics implementation pass, see
[Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md).
That document uses Hopper's official product/download/FAQ/release API,
changelog, debugger server, SDK archive, third-party package inspection,
Apple platform security documentation, and Sparkle documentation to define
Damsel's signing, notarization, entitlement, source access, helper security,
diagnostic redaction, crash/hang recovery, update, plugin trust, and debugger
trust architecture.

For the Apple artifact intake/acquisition/source-registration implementation
pass, see
[Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md).
That document uses Hopper's FAQ, changelog, SDK loader architecture, Apple file
access guidance, and current Damsel loader/shared-cache code to define intake
sessions, artifact classification, source registration plans, source graphs,
projection/extraction recipes, sidecar matching, KDK/dSYM/panic attachment,
IPSW/app-bundle intake policy, and Workbench artifact overview flows.

For the expanded feature-architecture pass, see
[Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md).
That document combines official product pages, the current SDK archive, SDK
quick tour, public headers, recent changelog entries, public Python workflows,
and public MCP wrappers into an explicit Hopper architecture model.

For the interaction/product pass, see
[Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md).
That document focuses on Hopper's tutorial workflows, synchronized
representations, debugger surface, automation expectations, package evidence,
and concrete Damsel Workbench gaps.

For the patching/debugger implementation pass, see
[Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
and
[Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md).
Those documents use Hopper's public tutorial, FAQ, SDK mutation APIs, debugger
server page, and changelog entries to derive Damsel's safe byte-editing,
copy-on-write revision, patch preview, LLDB bridge, and static-dynamic route
mapping architecture.

For the Patch Studio implementation pass, see
[Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md).
That document uses Hopper's tutorial byte-editing, assembler, NOP Region, hex
view synchronization, undo/redo, FAQ mutation, public SDK mutation/assembler
surfaces, and prior changelog notes in this pack to define Damsel's future
patchability queries, byte/assembler/NOP previews, copy-on-write patch
revisions, Apple artifact export policies, code-signing warnings, reanalysis,
patch history, debugger handoff, automation gates, and evidence packages.

For the live debugger Workbench implementation pass, see
[Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md).
That document uses Hopper's homepage, tutorial breakpoint surface, Debugger
Server page, prior changelog notes in this pack, LLDB remote-debugging
documentation, and Apple debugging entitlement/hardened-runtime documentation
to define Damsel's future launch/attach UX, debug target/session/module models,
route-backed breakpoints, stop-event views, register/frame/memory panes,
helper isolation, permission diagnostics, Apple runtime live interpretation,
and redacted session exports.

For the remote/device debugger implementation pass, see
[Remote Device Debugging And Target Environment Implementation Spec](remote-device-debugging-target-environment-implementation-spec.md).
That document uses Hopper's Debugger Server page, Hopper tutorial/homepage
debugger surface, LLDB's current remote-debugging documentation, Apple's LLDB
standalone guide, Apple debugger entitlement and Developer Mode documentation,
and Apple KDK/kernel-debugging documentation to separate local debugging from
remote macOS, iOS/iPadOS device, simulator, and XNU-adjacent target
environments. It defines target registries, transport records, debug-server
lifecycle, preflight plans, remote module/source matching, device symbol
state, runtime events, Workbench target browsers, redacted diagnostics, and
honest unsupported-workflow boundaries.

For the semantic editing and reanalysis-hints implementation pass, see
[Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec](semantic-editing-reanalysis-hints-implementation-spec.md).
That document uses Hopper's tutorial evidence for byte-type edits, undo/redo,
graph comments/labels/grouping/colors/text edits, address naming, tags,
manual references, switch/case hints, hex-editor invalidation, and inspector
modules; Hopper's homepage evidence for direct CFG annotation, scripting, SDK,
MCP, and native UX; public Hopper Python API stubs for comments, inline
comments, labels, tags, colors, manual references, local labels, register
aliases, byte-type setters, document save paths, and local variables; public
script collections for analyzer-created annotations; and public workflow
discussion around local-variable type editing and Manage Types. It maps those
sources into Damsel semantic edit records, suggestions, review, conflict
handling, stale-route repair, Apple runtime bindings, reanalysis impacts,
automation capability gates, privacy, and evidence export.

For the preferences, keymaps, layout, and theming implementation pass, see
[Preferences, Keymaps, Layout Persistence, And Theming Implementation Spec](preferences-keymaps-layout-theming-implementation-spec.md).
That document uses Hopper's tutorial evidence for instruction-encoding display
preferences, fuzzy-search preferences, adaptive/fixed hex columns, pane
visibility, keyboard graph movement, navigation stack behavior, feature-map
colors, and contextual inspector modules; Hopper's homepage evidence for
native Cocoa polish, keyboard shortcuts, synchronized representations,
scripting, SDK, and MCP; public theme/plugin evidence from the Hopper plugin
index; a public Hopper teaching workflow that exposes show/hide panes and the
hex-column preference; and Apple documentation for app defaults, dark-mode
expectations, and window restoration concepts. It maps those sources into
Damsel app/project/session preference ownership, renderer settings, keymaps,
theme tokens, layout repair, analysis profiles, Apple artifact roots,
automation/privacy gates, diagnostics presets, and settings import/export.

For the Swift runtime metadata and Swift-ObjC bridge implementation pass, see
[Swift Runtime Metadata And Swift-ObjC Bridge Implementation Spec](swift-runtime-metadata-implementation-spec.md).
That document uses Hopper's homepage Objective-C and Swift positioning,
Hopper's current changelog entries for Swift demangling, Swift strings, Swift
metadata, and malformed/ciphered Swift metadata failures, Swift ABI mangling
and type-metadata documentation, Swift ABI metadata headers, LLVM Mach-O Swift
section-name constants, and public Swift reverse-engineering workflows. It maps
those sources into Damsel Swift section inventory, demangler service, name
roles, typeref/field/protocol/conformance parsing, metadata accessor and
runtime-call classification, Swift-ObjC bridge records, browser APIs,
diagnostics, static/dynamic evidence separation, and validation fixtures.

For the investigation casebook/session-timeline/report workspace implementation
pass, see
[Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec](investigation-casebook-session-timeline-implementation-spec.md).
That document uses Hopper's tutorial evidence for comments, labels, tags,
manual references, graph annotations, navigation stack, bookmarks, breakpoints,
undo/redo, and switch hints; Hopper's homepage evidence for native UI, direct
CFG annotation, PDF export, scripting, AI/MCP, debugger, and contextual
inspector; Hopper's FAQ/download evidence for scripting, save/export, and
unsupported encrypted iOS app constraints; and public Hopper script/export
workflows. It maps those sources into Damsel cases, sessions, timeline events,
evidence pins, hypotheses, findings, notebook pages, report builder,
redaction preview, handoff packages, Workbench stores, APIs, CLI commands, and
validation gates.

For the automation and SDK-boundary implementation pass, see
[Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
and
[Plugin And SDK Extension Boundary Implementation Spec](plugin-sdk-extension-boundary-implementation-spec.md).
Those documents use Hopper's public Python scripting, MCP, public Python stubs,
public scripts, official SDK plugin contracts, loader/CPU/tool headers, and
sample plugin code to derive Damsel's stable CLI JSON, batch automation, local
MCP, annotation transaction, extension manifest, helper-process, and
capability-boundary architecture.

For the Automation Studio implementation pass, see
[Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec](automation-studio-scripting-plugin-workbench-implementation-spec.md).
That document uses Hopper's homepage scripting/MCP/SDK claims, FAQ MCP and
Python mutation notes, public Hopper Python type stubs, public Hopper scripts,
and SDK/plugin evidence in this pack to define Damsel's native recipe runner,
Automation Studio, local MCP session panel, script/package model, plugin
manager, capability prompts, activity logs, extension outputs, provenance,
Apple-system automation recipes, diagnostics, and validation gates.

For the evidence-grounded AI assistant implementation pass, see
[Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md).
That document uses Hopper's homepage AI/MCP claim, Hopper's FAQ MCP guidance,
the current official MCP specification, and the MCP tools/resources/prompts
security model to define Damsel's assistant pane, context review, evidence
tray, route/fact citations, tool timelines, model profiles, privacy modes,
reviewed suggestions, Apple-system assistant modes, response validation, and
evidence exports.

### Hopper Patching And Debugger Implementation Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- <https://pypi.org/project/types-hopper-disassembler/>
- <https://github.com/droe/hopper-scripts>
- <https://modelcontextprotocol.io/specification/>
- <https://modelcontextprotocol.io/specification/2025-06-18/server/tools>
- <https://modelcontextprotocol.io/specification/2025-06-18/server/resources>
- <https://modelcontextprotocol.io/specification/2025-06-18/server/prompts>
- <https://lldb.llvm.org/use/remote.html>
- <https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.debugger>

SDK evidence inspected:

- `HPDisassembledFile` mutation, assembler, NOP, procedure, tag, color,
  comment, operand-format, problem, and undo/redo transaction APIs.
- `HPDocument` virtual-address read/write APIs and background-process state.
- `CPUDefinition` NOP and assembler capability APIs.
- `CPUContext` raw instruction assembly, rendering, and analysis hooks.

Why it matters:

- Hopper's hex editor, assembler, NOP region, Python byte mutation, debugger,
  and patch-before-launch changelog entries prove that mutation and debugging
  are real product surfaces.
- The public SDK shows that mutation belongs near document/file transactions,
  byte types, procedures, and CPU semantics rather than in a disconnected hex
  widget.
- The debugger evidence shows that live process state must overlay the same
  static routes, assembly, graph, hex, and procedure objects.
- Hopper's local/remote debugger split and Apple's debugger permission model
  imply that Damsel's live debugger needs explicit launch/attach flows, helper
  isolation, platform-denial problems, diagnostic redaction, and separate
  future tracks for remote, iOS, and kernel debugging.
- Hopper's patching surface implies that byte edits, assembler, NOP, undo/redo,
  and debugger launch must share one document/revision model rather than live in
  a disconnected hex editor.

Planning impact:

- Damsel should implement read-only synchronized hex first, but design
  patchability, patch preview, source revisions, invalidation, and transaction
  models early.
- Damsel's debugger should start as local, explicit, read-mostly, route-backed
  LLDB integration after project/source/address models are stable.
- Damsel's debugger Workbench should treat debug targets, sessions, modules,
  breakpoints, stop events, frames, registers, memory windows, and live
  Apple-runtime interpretations as structured project records rather than as a
  raw LLDB transcript.
- Damsel's Patch Studio should start with patchability, dry-run previews,
  copy-on-write source revisions, immutable originals, code-signing warnings,
  and explicit export/evidence packages before any broad binary-rewriting claim.
- In-place patching of shared caches or kernel collections remains outside the
  safe product path.

### Hopper Native App Trust, Distribution, And Diagnostics Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/download.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- <https://desktopinsights.vercel.app/apps/hopper-disassembler>
- <https://support.apple.com/guide/security/app-code-signing-process-sec3ad8e6e53/web>
- <https://support.apple.com/guide/security/gatekeeper-and-runtime-protection-sec5599b66df/web>
- <https://support.apple.com/guide/security/controlling-app-access-to-files-secddd1d86a6/web>
- <https://sparkle-project.org/documentation/>

Why it matters:

- Hopper's public release API exposes signed macOS release metadata with
  archive hash, signature, EdDSA signature, deployment target, public/beta
  state, and separate SDK package metadata.
- Hopper's changelog repeatedly treats crashes, hangs, memory leaks, large
  binary/database performance, MCP reliability, hardened-runtime detection,
  debugger attach behavior, dyld-cache compatibility, and plugin crash-loop
  recovery as product-quality work.
- Hopper's SDK documentation states that macOS plugins require proper signing
  on modern macOS, which makes extension trust a distribution and database
  provenance problem.
- Apple and Sparkle sources ground the distribution requirements in Developer
  ID signing, notarization, Gatekeeper, file-access consent, archive signing,
  appcasts, update signatures, and nested helper signing.

Planning impact:

- Damsel needs a native trust subsystem, not only UI polish: helper
  authentication, source access grants, security-scoped bookmark policy,
  diagnostic redaction, crash/hang summaries, signing/notarization gates,
  update channel modeling, plugin trust, and debugger entitlement readiness.
- The current Workbench helper launch and upload-based source access should be
  migrated behind a token-authenticated local helper and project-backed source
  grants before broad distribution claims.
- Future plugin/debugger/update work should be build-flavor and capability
  gated so the ordinary analysis app does not inherit unnecessary entitlement
  risk.

### Hopper Command, Navigation, Undo, And Native Interaction Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/download>
- <https://www.hopperapp.com/HopperDebuggerServer/>

Local evidence:

- `damsel-app/Sources/DamselWorkbench/DamselWorkbenchApp.swift`
- `damsel-app/Sources/DamselAppCore/Support/DamselCommands.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- `damsel-app/Sources/DamselAppCore/Views/InstructionSurfaceView.swift`
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)

Why it matters:

- Hopper's public tutorial makes command-driven interaction central: byte-type
  transformations, graph navigation, graph edits, inspector edits, hex edits,
  assembler/NOP operations, navigation stack behavior, tags, comments, and
  undo/redo all operate against one document context.
- Hopper's homepage emphasizes native Cocoa polish, keyboard shortcuts,
  synchronized representations, scripting, MCP, debugger support, Objective-C
  and Swift awareness, and SDK extensibility.
- Current Damsel Workbench commands are mostly one-off AppKit selectors and
  notification names, so the app needs a typed command registry, route-backed
  selection, navigation stack, activity log, and project transaction bridge
  before adding many more product surfaces.

Planning impact:

- Damsel should treat commands as structured product infrastructure shared by
  menus, toolbar, command palette, context menus, CLI JSON, MCP, plugins, and
  tests.
- Undo/redo must be backed by project transactions, not only Swift
  `UndoManager`.
- Apple-specific route kinds and command groups should exist early so ObjC,
  C++, dyld-cache, KDK, panic-log, and kernel workflows do not become bolted-on
  UI exceptions.

### Apple Runtime Workbench Browser Implementation Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://developer.apple.com/documentation/objectivec/objective-c-runtime>
- <https://developer.apple.com/documentation/foundation/objective-c-runtime-utilities>
- <https://developer.apple.com/documentation/apple-silicon/debugging-a-custom-kernel-extension>
- <https://developer.apple.com/documentation/driverkit/debugging-and-testing-system-extensions>
- <https://developer.apple.com/documentation/kernel/mach-o>
- <https://github.com/apple-oss-distributions/objc4>
- <https://github.com/apple-oss-distributions/xnu>
- <https://github.com/apple-oss-distributions/dyld>
- <https://itanium-cxx-abi.github.io/cxx-abi/abi.html>

Local evidence:

- `damsel-core/src/model.rs`
- `damsel-macho/src/objc.rs`
- `damsel-macho/src/disasm.rs`
- `damsel-app/Sources/DamselAppCore/Models/DamselModels.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)

Why it matters:

- Hopper's public product surface makes Objective-C/Swift extraction,
  synchronized representations, inspector context, native UI speed, scripting,
  debugger support, and SDK extensibility part of the expected experience.
- Apple's Objective-C runtime documentation grounds classes, selectors,
  methods, IMPs, ivars, properties, protocols, type encodings, and
  `objc_msgSend` in public API concepts that Damsel can mirror as browser
  objects.
- Apple's KDK/kernel-debugging and DriverKit documentation ground the KDK,
  panic, arm64e/PAC, entitlement, kext, and system-extension concerns that
  Damsel needs to expose honestly in kernel workflows.
- The Itanium C++ ABI grounds the vtable, RTTI, implicit `this`, thunk, and
  dynamic-cast concepts behind C++/ObjC++ and IOKit analysis.
- Current Damsel already has ObjC metadata, message-send summaries, cache link
  summaries, shared-cache queries, and Swift disassembly models, but it lacks
  paged native runtime browsers over those facts.

Planning impact:

- Damsel needs native Workbench browsers for ObjC, ObjC dispatch, C++/ObjC++,
  IOKit, dyld cache, kernel collection, XNU interfaces, KDK/dSYM
  symbolication, panic logs, runtime relationship graphs, and runtime
  problems.
- Browser pages must be route-backed, paged, confidence-aware, command-driven,
  and built over project indexes rather than transient disassembly responses.
- The Apple runtime browser suite is the clearest way for Damsel to express its
  niche beyond generic Hopper parity.

### Hopper Apple Artifact Intake And Source Registration Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- <https://support.apple.com/guide/security/controlling-app-access-to-files-secddd1d86a6/web>
- <https://developer.apple.com/download/>

Local code evidence inspected:

- `damsel-macho/src/loader.rs`
- `damsel-macho/src/shared_cache.rs`

Why it matters:

- Hopper's FAQ says iOS binaries are supported when not still protected by
  FairPlay-style App Store encryption, which means unsupported/protected input
  classification belongs in the loader/intake layer.
- Hopper's changelog repeatedly mentions dyld shared-cache extraction,
  extracted-file navigation, local/global offsets, iOS 16 cache compatibility,
  newer macOS cache compatibility, driver-cache dylibs, and unencrypted iOS and
  macOS kernel caches.
- Hopper's SDK loader architecture supports multiple detected file types,
  loader-specific user parameters, segment/section construction, debug-data
  loading, and progress callbacks.
- Current Damsel can load Mach-O files and inspect/project shared caches, but
  it lacks a durable project intake session, source registration plan, sidecar
  matching workflow, app-bundle/IPSW-root intake, KDK/dSYM/panic attachment
  implementation, and Workbench artifact overview.

Planning impact:

- Damsel should implement artifact classification and source registration as a
  first-class pre-analysis layer.
- dyld cache roots, subcaches, symbols sidecars, projected images, driver-cache
  members, kernel fileset members, KDKs, dSYMs, panic logs, app bundles, and
  extracted IPSW roots should all become source graph nodes with provenance.
- Workbench should show artifact identity, member lists, sidecar status,
  problems, and initial jobs quickly rather than requiring terminal
  preprocessing.

### Hopper Public-Web Feature Architecture Audit, 2026

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- <https://raw.githubusercontent.com/Fi5t/hopper-disassembler-stubs/master/src/hopper_api-stubs/__init__.pyi>
- <https://github.com/droe/hopper-scripts>
- <https://github.com/phracker/HopperScripts>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://glama.ai/mcp/servers/dflatline/HopperPyMCP>
- <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>
- <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- <https://desktopinsights.vercel.app/apps/hopper-disassembler>

Why it matters:

- Anchors current Hopper version facts as of 2026-05-26: Mac app and SDK
  `6.3.0`, Linux packages `6.3.3`, and SDK archive SHA1.
- Converts public Hopper feature evidence into explicit architecture layers:
  document/session, disassembled-file database, loader/container, typed byte
  map, procedure/CFG, architecture semantics, type/signature, synchronized UI,
  Apple runtime specialization, decompiler, debugger, patching, automation,
  SDK boundaries, and performance.
- Provides a source-to-requirement table that future completion audits can use
  to verify whether every major Hopper feature family maps to a Damsel
  subsystem.

Planning impact:

- Confirms that Hopper closeness requires persistent, typed, routeable,
  incremental, undoable, automatable, and responsive project state.
- Makes Damsel's Apple-specialized advantage concrete without underestimating
  Hopper's product architecture.
- Strengthens the requirement that automation, plugin, debugger, patching, and
  Workbench views share one project/document object model.

### Hopper Performance, Scheduler, And Indexing Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>

Local code evidence inspected:

- `damsel-cli/src/ui.rs`
- `damsel-app/Sources/DamselAppCore/Services/DamselAPIClient.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- `damsel-app/Sources/DamselAppCore/Views/VirtualizedInstructionTable.swift`

Why it matters:

- Hopper's changelog repeatedly treats large binary load time, large database
  load time, memory footprint, decompiler memory, hangs, deadlocks, large
  documents, dyld cache extraction, driver cache reading, and address-list
  responsiveness as product work.
- The SDK exposes background-process state, cancellation/wait UI, and loader
  progress callbacks.
- Current Damsel has useful alpha foundations, but its upload-based in-memory
  UI server and point-query payload model cannot scale to Hopper-class Apple
  system workflows.

Planning impact:

- Damsel needs a path-based source registry, mmap-backed source access, job
  scheduler, priority model, paged indexes, event stream, hot cache policy,
  metrics endpoint, diagnostics export, and Workbench store split before
  claiming large-artifact responsiveness.

### Hopper Diffing, Export, And Evidence Pass

Source set:

- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- <https://github.com/google/binexport>
- <https://github.com/droe/hopper-scripts>
- <https://github.com/phracker/HopperScripts>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>

SDK evidence inspected:

- `HPDisassembledFile` names, comments, byte types, procedures, tags, problems,
  structured types, import/export types, and transactions.
- `HPProcedure` basic blocks, callers, callees, local labels, register aliases,
  variables, signatures, and calling conventions.
- `HPSegment` references to/from addresses.
- `HopperTool` document command surface.

Why it matters:

- Hopper's public changelog includes BinDiff/BinExport export/import,
  Objective-C header export, assembly/pseudocode export reliability, BinDiff
  exporter memory work, and large export crash fixes.
- The BinExport ecosystem shows that disassembly export is a common
  interoperability requirement across disassemblers.
- Public Hopper scripts show analysts often need custom export paths beyond
  fixed UI reports.

Planning impact:

- Damsel should implement native analysis snapshots, stable fingerprints,
  Apple-runtime/kernel-aware diffing, evidence packages, redaction-by-default,
  and report exports before attempting full BinExport compatibility.
- Damsel's niche should be OS-build archaeology: dyld cache images,
  provider/importer/reexport edges, ObjC selectors/classes, C++/IOKit ABI,
  kernel collections, KDK/dSYM profiles, and panic-route diffs.

### Hopper Automation, MCP, And SDK Boundary Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- <https://raw.githubusercontent.com/Fi5t/hopper-disassembler-stubs/master/src/hopper_api-stubs/__init__.pyi>
- <https://github.com/droe/hopper-scripts>
- <https://github.com/phracker/HopperScripts>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://glama.ai/mcp/servers/dflatline/HopperPyMCP>

SDK evidence inspected:

- `HopperPlugin`, `HopperTool`, `FileLoader`, `CPUDefinition`, `CPUContext`,
  `HPDocument`, `HPDisassembledFile`, `HPProcedure`, `HPBasicBlock`,
  `HPSegment`, `HPTypeDesc`, and SDK sample plugins.

Why it matters:

- Hopper's public automation surface is document automation, not a separate
  model.
- Hopper's SDK separates loader, CPU, and tool plugin responsibilities.
- Public scripts and MCP wrappers show user demand for stable object access,
  comments, labels, byte reads, procedure data, decompilation, patch helpers,
  and AI/tool integration.

Planning impact:

- Damsel should implement CLI JSON and read-only local MCP over the same
  project APIs before embedding a scripting language.
- Damsel should expose future mutation only through audited transactions.
- Damsel should grow extension points through internal traits and
  manifest-driven helper processes before any public ABI.

### Hopper Feature Architecture Public Evidence

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/download.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- <https://abad1dea.tumblr.com/post/23487860422/analyzing-binaries-with-hoppers-decompiler>
- <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://github.com/droe/hopper-scripts>
- <https://glama.ai/mcp/servers/dflatline/HopperPyMCP>

Why it matters:

- Treats Hopper as a feature architecture rather than a feature checklist.
- Uses official SDK 6.3.0 headers to map public document, disassembled-file,
  segment, procedure, basic-block, CPU, type, signature, transaction, and
  plugin surfaces.
- Uses the changelog to show ongoing investment in decompiler correctness,
  large-binary performance, MCP reliability, dyld shared-cache navigation,
  driver-cache support, Swift demangling, ObjC trampolines, and kernel cache
  work.
- Separates official facts, SDK surfaces, public workflow evidence, and
  inference.

Planning impact:

- Confirms that Damsel needs project/document state, typed bytes, procedure
  records, xrefs, architecture semantics, transactions, route-synchronized
  Workbench views, and automation APIs before claiming Hopper closeness.
- Makes the decompiler path more explicit: procedure database, architecture
  semantics, low-level IR, SSA, ABI/type/runtime overlays, then pseudocode.
- Reinforces Damsel's Apple-system specialization as the competitive lane, but
  also shows that the internal architecture must be Hopper-grade.

### Hopper Interaction, Debugger, And Product Surface Research

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/download.html>
- <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.hopperapp.com:443/downloader/public/HopperSDK-6.3.0.zip>
- <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- <https://stateofsecurity.com/tool-review-hopper-disassembler-for-os-x/>
- <https://github.com/droe/hopper-scripts>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://desktopinsights.vercel.app/apps/hopper-disassembler>

Why it matters:

- Connects Hopper's public product behavior to concrete app architecture
  requirements: routes, navigation stack, synchronized views, feature map,
  inspector modules, problem surfaces, jobs, command palette, debugger mapping,
  and packaging reliability.
- Grounds the Workbench migration plan in both public Hopper behavior and
  current Damsel Swift files.

Planning impact:

- Damsel Workbench must migrate from a point-query decode shell to a native
  project document application.
- The current `WorkbenchStore` should be split before adding route stack,
  project state, transactions, jobs, and runtime browsers.

For the graph/feature-map/hex representation pass, see
[Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md).
That document uses Hopper's tutorial/homepage/FAQ surface, Hopper SDK
`HPBasicBlock`, `HPProcedure`, `HPDisassembledFile`, `HPASMLine`, and
`CPUContext` headers, plus current Damsel `DisassemblyAnalysis`, API, and
Workbench evidence to define route-backed graph records, layout jobs, feature
map lanes, bounded hex windows, selection synchronization, graph export,
accessibility fallbacks, performance budgets, and Apple-runtime graph
extensions.

### Hopper SDK And Document Architecture Research

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/download.html>
- <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.hopperapp.com:443/downloader/public/HopperSDK-6.3.0.zip>
- <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>
- <https://pypi.org/project/types-hopper-disassembler/>
- <https://github.com/Fi5t/hopper-disassembler-stubs>
- <https://raw.githubusercontent.com/Fi5t/hopper-disassembler-stubs/master/src/hopper_api-stubs/__init__.pyi>
- <https://github.com/droe/hopper-scripts>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://www.mcpnow.io/en/server/hopper-mxiris-reverse-engineering-hoppermcp>
- <https://desktopinsights.vercel.app/apps/hopper-disassembler>
- <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>

Why it matters:

- Confirms Hopper's public architecture is centered on a mutable document and
  disassembled-file model, not only decoder output.
- Shows that public plugins cover loader, CPU, and tool surfaces, while
  decompiler AST internals remain private.
- Exposes concrete SDK objects for byte types, names, comments, references,
  tags, colors, problems, procedures, basic blocks, signatures, calling
  conventions, types, undo/redo transactions, and background analysis state.
- Shows that Hopper's CPU contract expects architecture semantics for branch
  analysis, switch recovery, prolog detection, thunk recognition, rendering,
  assembly, and decompiler cooperation.
- Grounds the Damsel plan for project kernel, typed byte map, procedure store,
  architecture semantics trait, transaction log, and extension boundaries.

Planning impact:

- Treat Capstone as an instruction decoder, not as the full architecture layer.
- Build transactions and provenance before scripting or MCP mutation.
- Add object ids and routeable facts before automation.
- Keep Apple runtime and XNU-style object models in the core roadmap because
  Hopper is actively investing in the same public areas.

### Hopper Deep Public-Web Research Addendum

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/download.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>
- <https://github.com/makigumo/HopperSDK-v4>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/HPDisassembledFile.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUDefinition.h>
- <https://github.com/makigumo/hopper-xcode-templates>
- <https://github.com/droe/hopper-scripts>
- <https://git.hubp.de/keith/hopper-swift-demangle>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://iosgods.com/topic/182321-unity-il2cppdumper-for-hopper-disassembler-on-macos/>
- <https://www.mcp.pizza/mcp-server/CL7N/HopperMCP>
- <https://desktopinsights.vercel.app/apps/hopper-disassembler>
- <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- <https://abad1dea.tumblr.com/post/23487860422/analyzing-binaries-with-hoppers-decompiler>
- <https://stateofsecurity.com/tool-review-hopper-disassembler-for-os-x/>
- <https://www.jviotti.com/2023/11/20/exploring-macos-private-frameworks.html>

Why it matters:

- Corrects the shallow interpretation of Hopper as only a disassembler plus CFG
  and pseudocode.
- Adds author-interview evidence that Hopper's hard value is automatic
  structure reconstruction after instruction decoding.
- Uses public SDK headers to identify document/file/procedure/block/type/tag,
  undo, assembler, reference, and CPU-semantics concepts.
- Adds package and changelog evidence for native app, MCP, Python, large
  database, memory, driver-cache, dyld-cache, kernel-cache, Swift, ObjC, and
  arm64e concerns.

Planning impact:

- Damsel should prioritize a persistent project kernel, typed-byte range map,
  semantic assembly rows, procedure-local facts, route stack, inspector module
  system, and safe automation API before attempting broad Hopper-like polish.
- Damsel's strongest competitive lane remains Apple system binaries, but the
  implementation must still match Hopper's document and interaction
  architecture.

### Processor Semantics And Apple ABI Implementation Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUDefinition.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUContext.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/HPDisassembledFile.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/FileLoader.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CommonTypes.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/DisasmStruct.h>
- <https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms>
- <https://developer.apple.com/documentation/security/preparing-your-app-to-work-with-pointer-authentication>
- <https://clang.llvm.org/docs/PointerAuthentication.html>
- <https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst>
- <https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/EXTERNAL_HEADERS/mach-o/loader.h>
- <https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/EXTERNAL_HEADERS/mach-o/fat.h>
- <https://github.com/swiftlang/swift/blob/main/docs/ABI/Mangling.rst>

Repo evidence inspected:

- `damsel-core/src/model.rs`
- `damsel-macho/src/disasm.rs`
- `Cargo.toml`

Document produced:

- [Processor Semantics And Apple ABI Implementation Spec](processor-semantics-apple-abi-implementation-spec.md)

Why it matters:

- Hopper's public SDK shape shows CPU support as an analysis contract, not only
  a disassembly-printer contract: CPU plugins expose mode selection, address
  adjustment, prolog/epilog detection, branch and switch analysis, thunk
  recovery, assembler support, and decompiler hooks.
- Hopper's tutorial and changelog make CPU mode, switch hints, AArch64,
  AArch64e/PAC, ARMv7, Intel, Swift, ObjC, dyld cache, driver cache, and kernel
  cache concerns user-visible product behavior.
- Apple and Clang ABI sources make arm64e/PAC, `x18`, `x29`, stack alignment,
  Objective-C runtime pointers, C++ vtables, Swift mangling, and XNU filesets
  central to Damsel's Apple-system niche.
- Current Damsel code has `Architecture`, `Operand`, `DecodedInstruction`, and
  `DisassemblyAnalysis`, but lacks routeable architecture profiles, register
  roles, instruction semantics, ABI profiles, and semantic problem records.

Planning impact:

- Damsel should implement a durable processor/ABI semantics layer before
  claiming Hopper-class decompiler, debugger, patching, ObjC/C++/Swift/XNU
  browser, MCP, or AI assistant parity.
- Capstone can remain the initial decoder, but Damsel needs its own
  provenance-rich semantic facts above decoder output.
- Apple-system advantage depends on separating user-mode, dyld-cache, and
  XNU/kernel ABI profiles rather than applying one generic arm64 rule set.

### Decompiler Quality And User-Assisted Recovery Implementation Pass

Source set:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUContext.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUDefinition.h>
- <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/HPDisassembledFile.h>
- <https://github.com/melomac/Hopper>
- <https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms>
- <https://clang.llvm.org/docs/PointerAuthentication.html>
- <https://github.com/swiftlang/swift/blob/main/docs/ABI/Mangling.rst>

Repo evidence inspected:

- `docs/hopper-gap/decompiler-function-ir-implementation-spec.md`
- `docs/hopper-gap/decompiler-pseudocode-workbench-implementation-spec.md`
- `docs/hopper-gap/decompiler-ir-strategy.md`
- `damsel-core/src/model.rs`
- `damsel-macho/src/disasm.rs`

Document produced:

- [Decompiler Quality And User-Assisted Recovery Implementation Spec](decompiler-quality-user-assisted-recovery-implementation-spec.md)

Why it matters:

- Hopper's tutorial frames pseudocode as useful but imperfect, with dead-code
  visibility and switch/case hints as user-facing recovery tools.
- Hopper's FAQ explicitly says pseudocode is usually not directly compilable
  and should be used for understanding rather than source recovery.
- Hopper's changelog shows decompiler quality is an ongoing subsystem:
  decompiler rewrite, stack variables, variable rename, liveness, AST
  optimizations, switch/case, no-return, dead code, C++ signatures,
  Objective-C ivars/stubs/trampolines, Swift demangling/strings, hangs, memory
  footprint, and large function behavior.
- Hopper's SDK exposes CPU decompiler hooks while keeping `ASTNode` private,
  which reinforces that Damsel needs its own internal quality model rather than
  a superficial pseudocode renderer.

Planning impact:

- Damsel should treat pseudocode as an evidence-backed Workbench surface with
  pass ledgers, source maps, quality reports, and user hint transactions.
- Regression tests should validate semantic expectations and forbidden claims,
  not only exact pseudocode text.
- Apple-system pseudocode quality must separately gate ObjC, C++/ObjC++,
  Swift-adjacent, XNU/kernel, dyld, and arm64e/PAC overlays.

### Hopper Homepage

Source: <https://www.hopperapp.com/index.html>

Why it matters:

- Establishes Hopper as a macOS-native reverse-engineering product.
- Publicly positions Hopper around disassembly, decompilation, debugging, and
  interactive inspection.
- Supports treating Hopper as a workflow target, not merely an instruction
  decoder benchmark.

Planning impact:

- Damsel needs project/workspace behavior.
- Damsel needs multiple synchronized representations.
- Damsel needs native interaction quality in Workbench.
- Damsel eventually needs decompiler/debugger/scripting discussions, even if
  they are not first.

### Hopper FAQ

Source: <https://www.hopperapp.com/faq.html>

Why it matters:

- Useful for understanding what Hopper exposes publicly around licensing,
  platform support, and expected user workflows.

Planning impact:

- Reinforces that Damsel should define a narrower Apple systems niche instead
  of copying all broad product surfaces.

### Hopper Changelog

Source: <https://www.hopperapp.com/rss/html_changelog.php>

Why it matters:

- Shows that Hopper's public evolution is still active as of 2026, including
  MCP reliability, Swift demangling, Python compatibility, large-binary loading,
  memory footprint, dyld-cache method navigation, switch/case analysis,
  authenticated Objective-C selector trampolines, and driver-cache workflows.

Planning impact:

- Damsel should plan performance and responsiveness as ongoing first-class
  engineering lanes.
- Damsel should not treat MCP/AI as purely speculative, because Hopper now
  exposes it publicly.
- Driver-cache, dyld-cache, Swift/ObjC, and AArch64e details strengthen the
  Apple-systems specialization thesis.

### Hopper Tutorial

Source: <https://www.hopperapp.com/tutorial.html>

Why it matters:

- Public tutorial materials show the kind of interactive workflow users expect:
  navigation, analysis, names, procedures, references, and multiple views.

Planning impact:

- Damsel needs persistent names/xrefs and graph navigation before pseudocode can
  feel useful.

### Public Hopper Scripting Workflows

Sources:

- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://github.com/droe/hopper-scripts>
- <https://glama.ai/mcp/servers/dflatline/HopperPyMCP>

Why they matter:

- They show real users extending Hopper around document, segment, procedure,
  named-address, byte selection, comments, string search, annotation, and
  data-typing APIs.
- The MCP wrapper shows the same pressure moving toward AI/tool access over
  document facts, not only traditional Python scripts.

Planning impact:

- Damsel should expose stable object ids and fact APIs before promising
  scripting, plugins, or MCP.
- Annotation and data-typing mutations need the same audit/undo model whether
  they are initiated by UI, CLI, scripts, or MCP tools.

## Apple Mach-O And dyld

### Apple Mach-O Overview

Source:
<https://developer.apple.com/library/archive/documentation/Performance/Conceptual/CodeFootprint/Articles/MachOOverview.html>

Why it matters:

- Apple's overview explains the basic Mach-O organization into segments and
  sections, including executable text, C strings, stubs, and symbol pointer
  sections.

Planning impact:

- Byte classification should start from Mach-O segments/sections.
- Stub and symbol pointer sections should become reference and import facts.
- String/data browsers should be section-aware.

### LLVM `llvm-objdump`

Source: <https://llvm.org/docs/CommandGuide/llvm-objdump.html>

Why it matters:

- Documents baseline object-file disassembly capabilities such as disassembly,
  symbol scoping, address limits, relocations, source context, and unwind info.

Planning impact:

- Damsel should continue comparing disassembly output against LLVM where useful.
- Damsel's next layer should exceed objdump by adding persistent xrefs and
  runtime-aware navigation, not merely duplicate flags.

### LLVM `llvm-otool`

Source: <https://llvm.org/docs/CommandGuide/llvm-otool.html>

Why it matters:

- Documents Mach-O-specific inspection surfaces such as chained fixups, dyld
  info, indirect symbols, Objective-C sections, text sections, opcodes, and
  symbolic operands.

Planning impact:

- Damsel should treat chained fixups and ObjC sections as first-class analysis
  inputs.
- `llvm-otool` remains a comparison target for many Mach-O-specific views.

### LLVM Mach-O Header Definitions

Source: <https://llvm.org/docs/doxygen/BinaryFormat_2MachO_8h_source.html>

Why it matters:

- Provides public source-level constants and structures for Mach-O file types,
  load commands, and fileset-related vocabulary.

Planning impact:

- XNU/kernel collection support should model fileset entries explicitly instead
  of relying on filename heuristics.

## XNU And Kernel Binaries

### Apple XNU Open Source

Source: <https://github.com/apple-oss-distributions/xnu>

Why it matters:

- Provides public XNU source history and kernel-facing definitions useful for
  semantic recognizers, although shipping binaries and KDKs may not match source
  releases exactly.

Planning impact:

- Kernel analysis should include build/UUID provenance.
- Damsel should avoid claiming source/binary equivalence unless a specific
  build and symbol source prove it.
- Recognizers for syscalls, Mach traps, MIG, and IOKit should be version-aware.

### Apple Kernel Programming Guide Archive

Source:
<https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/KernelProgramming/>

Why it matters:

- Provides Apple documentation around kernel extension concepts and Darwin
  kernel development vocabulary.

Planning impact:

- Kernel docs and UI should use Apple vocabulary: kernel extensions, I/O Kit,
  Mach, BSD, and kernel/user boundaries.

### Binary Ninja Kernelcache Documentation

Source: <https://docs.binary.ninja/guide/kernelcache.html>

Why it matters:

- This is not a primary Apple spec, but it is useful competitor/context
  evidence that kernelcache analysis is its own specialized workflow.

Planning impact:

- Damsel should treat kernelcache/kernel collection support as a dedicated
  workflow, not a normal Mach-O open with extra symbols.

## Objective-C Runtime

### Apple Objective-C Runtime Source

Source: <https://github.com/apple-oss-distributions/objc4>

Why it matters:

- Provides public Objective-C runtime implementation source, including runtime
  metadata concepts that map to classes, methods, protocols, categories,
  selectors, properties, and ivars.

Planning impact:

- Damsel should build on its existing `ObjcMetadata` parser and normalize
  runtime objects for navigation.
- Type encodings, categories, protocols, and dispatch should be modeled as
  first-class analysis records.

### Apple Objective-C Runtime Reference

Source: <https://developer.apple.com/documentation/objectivec/objective-c_runtime>

Why it matters:

- Public API-level runtime documentation helps align Damsel's user-facing
  vocabulary with Apple's terms.

Planning impact:

- UI and CLI should use terms users know: class, metaclass, protocol, category,
  selector, method, IMP, ivar, property.

## C++ And Objective-C++

### Itanium C++ ABI

Source: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html>

Why it matters:

- Apple's C++ ABI surface for Mach-O uses Itanium-family concepts for name
  mangling, vtables, RTTI, constructors, destructors, and exception handling.

Planning impact:

- Damsel's C++ recovery should start with demangling, vtables, RTTI, class
  hierarchy, thunks, and virtual calls.
- XNU/IOKit analysis depends on ABI recovery because IOKit is heavily C++.

## arm64e Pointer Authentication

### Clang Pointer Authentication

Source: <https://clang.llvm.org/docs/PointerAuthentication.html>

Why it matters:

- Documents pointer authentication concepts that affect arm64e code generation,
  function pointers, vtables, and ABI behavior.

Planning impact:

- Damsel must distinguish raw stored pointers from interpreted targets.
- Pointer-auth facts should be reusable across ObjC IMPs, C++ vtables, and
  kernel tables.

### LLVM Pointer Authentication

Source: <https://llvm.org/docs/PointerAuth.html>

Why it matters:

- LLVM documentation is useful for object-level and compiler-level pointer-auth
  representation, especially when comparing against LLVM tools.

Planning impact:

- Damsel should avoid treating pointer-auth as merely a mnemonic-printing issue.

## Swift Runtime Boundary

### Swift ABI Mangling

Source:
<https://github.com/swiftlang/swift/blob/main/docs/ABI/Mangling.rst>

Why it matters:

- Swift symbol mangling is relevant to Damsel's existing Swift symbol target
  support and future runtime metadata exploration.

Planning impact:

- Swift is not the center of this pack, but demangled Swift symbols should
  coexist with ObjC/C++ names in the name system.

## Damsel Repo Evidence

### Root README

Source: `README.md`

Why it matters:

- Establishes the current implemented scope: static Mach-O analysis, Apple
  Silicon, dyld shared-cache Wave 1/2/3 work, projected-image analysis, and
  cache-native linkage exploration.

Planning impact:

- The docs should not recommend redoing the decoder or cache container work.

### Apple Runtime Architecture

Source: `docs/architecture/apple-runtime-analysis-architecture.md`

Why it matters:

- Defines the current shared-cache architecture and explicitly labels live
  debugger integration and cache mutation as excluded.

Planning impact:

- Hopper-gap work should remain additive and read-only by default.

### Disassembler V2 Alpha Plan

Source: `docs/roadmaps/disassembler-v2-alpha-plan.md`

Why it matters:

- Shows that Damsel already has a seven-track alpha direction: CFG/trace,
  cross-image cache navigation, value flow, function summary, ObjC/Swift
  entrypoints, interactive UI navigation, and real-cache validation.

Planning impact:

- The Hopper-gap roadmap should build above this alpha layer.

### Core Model

Source: `damsel-core/src/model.rs`

Why it matters:

- Contains the existing ObjC metadata, dyld metadata, disassembly analysis, CFG,
  recovered value, cache link, and shared-cache model types.

Planning impact:

- New architecture should extend these types with project/fact/index layers
  instead of replacing them.

### Mach-O Disassembler

Source: `damsel-macho/src/disasm.rs`

Why it matters:

- Contains the current disassembly analysis pass that builds local blocks,
  edges, imports, ObjC refs, recovered values, jump tables, and summary metrics.

Planning impact:

- Whole-image function analysis should lift this current local analysis into
  persistent function records.

### Objective-C Parser

Source: `damsel-macho/src/objc.rs`

Why it matters:

- Contains current Objective-C metadata extraction for classes, protocols,
  categories, selectors, methods, pointer refs, properties, and ivars.

Planning impact:

- Objective-C explorer work should normalize and index existing parser output
  before writing a second parser.

### Shared Cache Session

Source: `damsel-macho/src/shared_cache.rs`

Why it matters:

- Contains current dyld shared-cache loading, projection, symbolication, and
  dependency/provider/importer/reexport support.

Planning impact:

- Cache-aware Hopper-gap features should use `SharedCacheSession` rather than
  bolting cache context onto generic image analysis without provenance.

### Damsel Workbench

Source: `damsel-app/README.md`

Why it matters:

- Explains that Workbench is a native shell over the existing CLI UI server.

Planning impact:

- Future native UI should stay a client of Rust APIs rather than become a
  separate analysis implementation.
