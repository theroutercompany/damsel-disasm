# Hopper Public-Web Feature Architecture Audit, 2026

Status: deeper source-to-requirement research audit. This is not an
implementation claim, and it is not a claim about Hopper's private source code.

Research date: 2026-05-26.

Purpose: turn the public Hopper Disassembler evidence into a sharper
architecture model for Damsel. The previous docs already identified major
feature families. This audit focuses on the part that is easy to undercount:
how Hopper's public features imply a layered analysis application architecture,
and what Damsel would have to implement to feel close to Hopper for macOS, iOS,
dyld shared caches, XNU-style system binaries, Objective-C, C++, and
Objective-C++.

## Source Reliability Model

Official product sources:

- Homepage: <https://www.hopperapp.com/index.html>
- Tutorial: <https://www.hopperapp.com/tutorial.html>
- FAQ: <https://www.hopperapp.com/faq.html>
- Debugger server page: <https://www.hopperapp.com/HopperDebuggerServer/>
- Release API:
  <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- Changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Public SDK archive:
  <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>

Public SDK files inspected from Hopper SDK 6.3.0:

- `include/Hopper/HPDocument.h`
- `include/Hopper/HPDisassembledFile.h`
- `include/Hopper/HPSegment.h`
- `include/Hopper/HPSection.h`
- `include/Hopper/HPProcedure.h`
- `include/Hopper/HPBasicBlock.h`
- `include/Hopper/HPTypeDesc.h`
- `include/Hopper/CPUDefinition.h`
- `include/Hopper/CPUContext.h`
- `include/Hopper/FileLoader.h`
- `include/Hopper/HopperPlugin.h`
- `include/Hopper/HopperTool.h`
- sample loader, CPU, and tool projects

Public automation and workflow sources:

- Python API stubs:
  <https://raw.githubusercontent.com/Fi5t/hopper-disassembler-stubs/master/src/hopper_api-stubs/__init__.pyi>
- Public script collection:
  <https://github.com/droe/hopper-scripts>
- Public script stash:
  <https://github.com/phracker/HopperScripts>
- Public procedure-name export workflow:
  <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- Third-party Hopper MCP wrapper:
  <https://glama.ai/mcp/servers/dflatline/HopperPyMCP>

Third-party and author-context sources:

- 2015 MacGeneration interview:
  <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>
- Mike Ash walkthrough:
  <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- Desktop Insights package page:
  <https://desktopinsights.vercel.app/apps/hopper-disassembler>

Confidence labels used below:

- Official fact: directly stated by Hopper's site, FAQ, tutorial, debugger
  page, changelog, release API, or public SDK.
- SDK surface: directly visible in the Hopper SDK 6.3.0 headers or samples.
- Public workflow evidence: visible in public scripts, public writeups, or
  third-party MCP wrappers. These sources show real user expectations, but not
  necessarily official product intent.
- Third-party package evidence: external app analysis. Useful but lower
  confidence than official sources.
- Inference: a Damsel architecture requirement derived from multiple public
  surfaces.

## Current Public Version Anchor

Official release API findings on 2026-05-26:

- Mac application: Hopper `6.3.0`, published `2026-05-19 16:58:15`, deployment
  target `macOS 12.4`, universal package, SHA1
  `c4126b9fedbbe2cc5f5088f6a64a8ea5fdfed9d6`.
- SDK: Hopper SDK `6.3.0`, published `2026-05-19 16:58:15`, deployment target
  `macOS 12.4`, SHA1 `a99628912f3b3dfa3dbe09ce22f1a741f6b0a3bc`.
- Linux packages: Hopper `6.3.3`, published `2026-05-22 19:20:00`, available
  as DEB, RPM, and Arch packages.

Official changelog themes from 2021-2026:

- Hopper 6.0 added a new decompiler, MCP server, improved switch detection,
  improved Swift string/demangling support, authenticated Objective-C selector
  trampoline support, and macOS 26 design work.
- Hopper 6.2.x repeatedly improved MCP reliability, large database load time,
  memory footprint, and document discovery.
- Hopper 6.2.5 rewrote the MCP server in C++ for shared macOS/Linux code.
- Hopper 6.1.0 added quick linear segment disassembly and driver-cache dylib
  reading.
- Hopper 6.0.12 improved method navigation for files extracted from the dyld
  shared cache.
- Hopper 5.x repeatedly mentions dyld shared cache support, kernel cache
  support, Swift metadata, Objective-C header export, chained fixups, PAC,
  decompiler memory, argument analysis, stack variables, large-binary
  analysis, Python compatibility, debugger behavior, patch-before-launch, and
  UI responsiveness.

Interpretation:

- Hopper is still moving quickly in exactly the Apple-system areas relevant to
  Damsel.
- Hopper's public roadmap pressure is not only "better decompiler". It is also
  large document performance, MCP/tool reliability, dyld/driver/kernel cache
  workflows, Swift/Objective-C metadata, PAC patterns, and native app polish.
- Damsel should treat these as baseline competitiveness requirements, not late
  extras.

## Corrected Architecture Model

The public evidence points to Hopper as a layered product:

```text
Native document application
  |
  +-- document/session controller
  |     +-- current address and selection
  |     +-- route/navigation stack
  |     +-- background process state
  |     +-- wait/cancel state
  |     +-- UI update and user messages
  |
  +-- mutable disassembled-file database
  |     +-- source/container identity
  |     +-- address spaces, segments, sections
  |     +-- typed byte map
  |     +-- names, labels, comments, inline comments
  |     +-- operand formats, colors, tags, bookmarks
  |     +-- procedures, basic blocks, stack facts
  |     +-- xrefs, callers, callees
  |     +-- type database, signatures, calling conventions
  |     +-- problems and analysis warnings
  |     +-- undo/redo transactions
  |
  +-- architecture semantics
  |     +-- instruction decode
  |     +-- CPU modes and address normalization
  |     +-- prolog, epilog, padding, NOP detection
  |     +-- branch and switch analysis
  |     +-- thunk and memory-reference analysis
  |     +-- rendering, assembler, decompiler cooperation
  |
  +-- analysis projections
  |     +-- assembly view
  |     +-- CFG view
  |     +-- pseudocode view
  |     +-- synchronized hex view
  |     +-- feature/navigation map
  |     +-- symbol/string/tag/procedure browsers
  |     +-- contextual inspector modules
  |     +-- debugger overlays
  |
  +-- automation and extension surfaces
        +-- Python scripting
        +-- local MCP server
        +-- tool plugins
        +-- loader plugins
        +-- CPU plugins
```

Damsel can be narrower in file and CPU breadth, but it cannot be shallow in the
architecture. To feel close to Hopper for Apple binaries, Damsel needs the same
kind of project/document spine.

## Layer 1: Native Document Application

Public evidence:

- The homepage markets Hopper as a fast native macOS disassembler with a
  polished Cocoa interface, keyboard shortcuts, and native macOS integration.
- The FAQ says Hopper's macOS direction is tied to Apple technologies and user
  experience.
- The tutorial assumes a native app with left browsers, center
  representations, right inspector, navigation stack, toolbar modes, menu
  commands, undo/redo, and direct editing.
- The release API exposes a signed universal Mac package with macOS deployment
  target.
- Third-party package analysis identifies Hopper as a Swift macOS app and lists
  code signing, notarization, Sparkle, Protocol Buffers, and debugger-related
  entitlements. Treat this as lower-confidence package evidence, not as an
  official architecture claim.

Architecture inference:

- Hopper's app shell is not a neutral viewer. Its native command routing,
  keyboard feel, window/document behavior, inspector focus, progress
  management, and persistence are part of the product.
- The app likely has a durable document/session layer above the disassembled
  file, because cursor, selection, navigation stack, background job state, and
  multiple representations all need shared state.

Damsel requirement:

- Promote the native Workbench from a CLI UI launcher to a document
  application with project documents, persisted UI routes, command registry,
  navigation history, autosave, undo/redo surfaces, job status, cancellation,
  inspector routing, and native error presentation.
- Keep the browser UI useful for experiments, but do not mistake it for the
  target Hopper-grade workbench.

## Layer 2: Document And Session Controller

Public evidence:

- `HPDocument` exposes current disassembled file, current segment, current
  section, current address, selected address range, cursor movement, `goto`,
  navigation-stack pop, background-process waiting/stopping, waiting state,
  logging, alerts, virtual-address reads and writes, UI refresh, and window
  access.
- Public scripts call into Hopper's current document.
- The tutorial describes jump navigation and backtracking as ordinary workflow,
  not a special command.

Architecture inference:

- Hopper separates UI session state from the mutable file database.
- Background analysis and cancellation are document-scoped.
- Address navigation is first-class and shared by all representations.

Damsel requirement:

- Add a `ProjectSession` or equivalent document controller distinct from
  `AnalysisProject`.
- Make route stack, current selection, selected source, selected procedure,
  current representation, background jobs, problems, and pending transactions
  first-class session state.
- Make CLI, Workbench, MCP, and future scripts use the same route resolver so
  that automation cannot create addresses the UI cannot navigate.

## Layer 3: Disassembled-File Database

Public evidence:

- `HPDisassembledFile` exposes CPU family/subfamily, CPU definition, syntax
  index, original path, address width, integer width, base address,
  entrypoints, potential procedures, segments, sections, names, nearest names,
  comments, inline comments, byte types, operand formats, procedures, colors,
  tags, problem records, assembler, NOP generation, typed reads, address
  parsing, undo/redo transactions, primitive types, pointers, arrays,
  structures, unions, enums, type database import/export, and structure
  placement.

Architecture inference:

- Hopper's central database is not just a list of decoded instructions. It
  holds analysis facts, user facts, display choices, type records, and
  transactions.
- Names, comments, colors, tags, operand formats, types, and byte
  classifications are not UI-only state.

Damsel requirement:

- Add a persistent analysis database with typed tables for sources, address
  spaces, segments, sections, byte classifications, procedures, basic blocks,
  xrefs, names, annotations, operand formats, runtime facts, types, problems,
  jobs, and transactions.
- Persist provenance on every fact: loader, Mach-O command, dyld cache, KDK,
  dSYM, ObjC metadata, C++ ABI analysis, architecture semantics, user edit,
  script, MCP tool, or plugin.
- Make invalidation explicit. Byte mutation, procedure creation/removal, type
  placement, and manual xrefs must be able to stale or recompute affected
  facts.

## Layer 4: Source, Loader, Segment, And Container Model

Public evidence:

- Hopper supports Mach-O, PE, and ELF.
- Hopper can disassemble iOS binaries when the input is not still FairPlay
  encrypted.
- `FileLoader` can detect file types from bytes and paths, create segments and
  sections, set CPU family/subfamily, report loading progress, load debug data,
  fix up rebased files, extract inner files from composite containers, and
  adjust the final file after extraction.
- `HPSegment` exposes file offset, file length, flags, read/write/execute
  permissions, mapped data, sections, procedures, and xrefs.
- The changelog repeatedly mentions dyld shared caches, files extracted from
  dyld shared caches, driver cache dylibs, kernel caches, FAT binaries, and
  malformed or malicious Mach-O files.

Architecture inference:

- Hopper treats container extraction and address mapping as first-class loader
  problems.
- Composite sources can produce extracted files while preserving enough
  original-container context to navigate offsets, base addresses, and rebases.

Damsel requirement:

- Model source artifacts separately from projected analysis targets:
  standalone Mach-O, universal slice, dyld shared cache, projected image,
  driver cache member, kernel collection member, kext, dSYM/KDK sidecar, panic
  log, and future IPSW asset bundle.
- Preserve both file offset and virtual address route forms, including original
  container offset and projected image offset where they differ.
- Build loader progress, cancellation, detected-type evidence, and load
  problems into the project kernel.

## Layer 5: Typed Byte Map And Feature Map

Public evidence:

- The tutorial says Hopper's core concept is transforming binary bytes into
  readable structure by assigning a type to each byte.
- The tutorial says automatic analysis starts when a file is loaded.
- The tutorial's navigation bar color-codes code, procedures, ASCII strings,
  data, and undefined bytes.
- The tutorial says manual hex editing can destroy the current instruction and
  procedure and roll the affected type back to undefined.
- `HPDisassembledFile` exposes byte type query/mutation, type modifiability,
  code tests, CPU mode at address, and "find next address of type".

Architecture inference:

- Byte typing is the shared index behind the feature map, hex view, procedure
  recovery, search, and patch invalidation.
- Type changes are commands, not cosmetic changes.

Damsel requirement:

- Implement `TypedByteRange` and `ByteClassification` as foundational project
  objects.
- Support code, procedure, data widths, ASCII, Unicode, pointer, external
  address, external function, alignment, structure, Objective-C metadata, C++
  metadata, Swift-adjacent metadata, PAC/chained-fixup pointer, kernel table,
  MIG descriptor, and unknown.
- Track confidence, source, owner, revision, and invalidation dependencies.
- Render the feature map from byte facts, not from sections alone.

## Layer 6: Procedure, CFG, And Graph State

Public evidence:

- The tutorial says most display modes require procedures because procedures
  contain basic blocks and stack usage.
- The tutorial describes a CFG mode with editable labels and comments, keyboard
  movement between blocks, a minimap, viewport movement, grouping of graph
  nodes, node background colors, and editable printed text.
- `HPProcedure` exposes frame/stack facts, basic block access, local labels,
  register renames, callers, callees, variable names, method signatures,
  calling conventions, and tags.
- `HPBasicBlock` exposes start/end, procedure owner, index, predecessors,
  successors, and tags.
- The changelog repeatedly mentions switch/case detection, no-return function
  detection, procedure creation using PAC, argument analysis, stack variables,
  variable liveness, local symbol naming, and decompiler hangs on weird
  procedures.

Architecture inference:

- Hopper's procedures are durable semantic objects, not transient CFGs.
- Graph edits need their own durable state or transaction layer.
- Procedure local facts are the bridge from disassembly to decompilation.

Damsel requirement:

- Implement persistent `ProcedureRecord`, `BasicBlockRecord`,
  `ControlFlowEdge`, `StackFrameFact`, `LocalVariableFact`,
  `RegisterAlias`, `LocalLabel`, `CallingConventionFact`,
  `ProcedureSignature`, and `ProcedureLayoutOverride`.
- Allow user-provided switch hints, no-return hints, local labels, and register
  aliases with transaction history.
- Separate architecture-derived CFG facts from user graph layout and grouping
  state.

## Layer 7: Architecture Semantics

Public evidence:

- `CPUDefinition` exposes CPU families/subfamilies, address width, integer
  width, endianess, syntax variants, CPU modes, registers, stack/frame/program
  counter recognition, NOP generation, assembler capability, and decompiler
  capability.
- `CPUContext` exposes address adjustment, CPU mode guessing, failed-decode
  fallback, NOP/padding detection, prolog detection, analysis lifecycle hooks,
  single-instruction decode, halt detection, procedure/basic-block analysis,
  direct memory reference filtering, load-address distinction, float use,
  target CPU-mode changes, switch-statement detection, branch analysis,
  instruction-specific analysis, thunk detection, rendering, decompiler hooks,
  and raw assembler output.
- The MacGeneration interview says instruction decoding is the easier part and
  that Hopper's value is deeper automatic structure reconstruction.

Architecture inference:

- Hopper's CPU layer is a semantic analysis participant, not just a decoder.
- Capstone-like decoding is a dependency, but not the architecture layer.

Damsel requirement:

- Add an `ArchitectureSemantics` trait above Capstone output.
- For AArch64/AArch64e, implement canonical instruction effects, branch/call
  classification, return detection, no-return propagation, switch candidates,
  thunk recognition, ADRP/LDR literal resolution, stack frame effects, PAC
  strip/sign/auth facts, authenticated Objective-C selector trampolines, NOP
  generation, and assembler integration later.
- Make unsupported or partial semantics explicit so downstream analysis can
  degrade honestly.

## Layer 8: Type, Signature, And Data Modeling

Public evidence:

- Hopper's homepage and older public product descriptions mention a type
  editor with structures, unions, and enums.
- `HPTypeDesc` exposes primitive, pointer, array, structure, union, enum, and
  function-pointer shape.
- `HPDisassembledFile` exposes primitive type factories, pointer/array builders,
  structure placement, type database enumeration, type import, and type export.
- `HPProcedure` exposes method signatures, calling conventions, and variable
  names.
- The changelog mentions C type parser fixes, enum base types, field ordering,
  argument ordering, signature editor improvements, C++ mangled name signature
  computation, and Objective-C header export improvements.

Architecture inference:

- Hopper's type model is central to decompilation, operand formatting,
  Objective-C header export, C++ recovery, and analyst annotation.

Damsel requirement:

- Implement a provenance-aware type system with primitive, pointer, array,
  struct, union, enum, function pointer, Objective-C encoded method type, block
  signature, C++ method signature, vtable slot type, kernel struct profile, and
  symbolication profile types.
- Support user-edited names and fields without overwriting imported KDK/dSYM or
  inferred facts.
- Make type import/export stable enough for fixtures and regression tests.

## Layer 9: Synchronized Views And Inspector

Public evidence:

- The tutorial describes a three-area UI: left symbols/strings list, center
  representations, and right contextual inspector.
- Hopper can show assembly, CFG, pseudocode, and hex views.
- Hex mode is synchronized with assembly and selects bytes belonging to the
  current instruction.
- Hopper 6.0.2 added a view synchronization toggle.
- The inspector has instruction encoding, operand format, comments, colors and
  tags, references, and procedure modules.
- The homepage says the inspector adapts to context.

Architecture inference:

- View synchronization requires shared route identity across assembly rows,
  graph blocks, pseudocode statements, byte ranges, symbols, strings, tags,
  xrefs, and debugger program counters.
- The inspector is a set of route-aware modules, not a fixed metadata panel.

Damsel requirement:

- Implement stable `Route`, `Selection`, `AddressRangeRoute`,
  `ProcedureRoute`, `BasicBlockRoute`, `RuntimeObjectRoute`,
  `TypeRoute`, `ProblemRoute`, `DebugFrameRoute`, and `PatchRoute` objects.
- Drive assembly, graph, hex, feature map, runtime browsers, search results,
  command palette, inspector modules, MCP responses, and debugger overlays from
  those routes.
- Add module-level inspector contracts for encoding, format, comments, tags,
  xrefs, procedure facts, runtime facts, type facts, patch preview, and
  debugger state.

## Layer 10: Objective-C, Swift, Dyld, And Apple Runtime Specialization

Public evidence:

- The homepage says Hopper is specialized for macOS and iOS and can decode
  Swift names and extract Objective-C selectors and messages.
- The tutorial says Hopper auto-tags Objective-C class and category
  implementations during loading.
- The changelog references Objective-C stubs, new retain/release functions,
  stripped Objective-C declarations, Objective-C ivar display, Objective-C
  header export, Objective-C protocols in dyld caches, authenticated
  Objective-C selector trampolines, Swift metadata, Swift strings, Swift
  demangling, Swift demangler crashes, dyld shared cache path discovery, dyld
  shared cache symbols, files extracted from dyld caches, driver cache dylibs,
  and iOS/macOS kernel caches.
- Public script ecosystems include class-dump and CFString/comment workflows.

Architecture inference:

- Apple runtime support is not a side panel. It affects names, tags,
  decompilation, selector navigation, string/comment rendering, method
  navigation, type export, and cache projection.

Damsel requirement:

- Treat Objective-C classes, categories, protocols, methods, selectors, ivars,
  properties, message sends, retain/release/autorelease idioms, blocks,
  CFStrings, chained fixups, dyld exports/imports, Swift-demangled names,
  Swift-adjacent metadata, and arm64e/PAC facts as routeable project objects.
- Add XNU/kernel-specific objects: kernel collection member, kext, IOKit
  class, vtable, OSObject method family, syscall, Mach trap, MIG routine,
  panic frame, KDK profile, dSYM profile, and DriverKit-adjacent metadata.
- Use auto-tags for Apple runtime and kernel object classes so search and
  navigation become fast for system-binary work.

## Layer 11: Decompiler Architecture

Public evidence:

- The homepage markets C-like pseudocode and automatic function-parameter
  identification.
- The tutorial says pseudocode is functionally equivalent but may be imperfect,
  with unreachable-code issues and a user-facing mitigation toggle.
- The FAQ says Hopper does not fully recover source and pseudocode is usually
  not directly compilable.
- The changelog shows major v6 decompiler work, large-function improvements,
  memory-leak fixes, memory-footprint reductions, switch/case improvements,
  dead-code removal, stack-variable handling, variable liveness, AST
  optimizations, argument analysis, no-return handling, and Objective-C ivar
  display fixes.
- `CPUContext` exposes decompiler hooks but says the AST/decompiler classes are
  not publicly exposed for third-party decompiler creation.

Architecture inference:

- Hopper's decompiler is a deep internal subsystem built on procedure,
  architecture semantics, types, signatures, variables, switch recovery, and
  runtime overlays.
- Decompiler extensibility is deliberately more private than loader/CPU/tool
  plugin extensibility.

Damsel requirement:

- Build decompiler capability in stages: function summaries, semantic
  instruction rows, low-level IR, SSA, stack/local recovery, ABI models,
  type propagation, runtime overlays, control-structure recovery, then
  pseudocode.
- Do not claim Hopper-grade decompilation until large functions, switch/case,
  Objective-C dispatch, PAC thunks, C++ virtual calls, kernel idioms, and
  type-driven output are validated on corpus fixtures.

## Layer 12: Debugger Integration

Public evidence:

- The homepage markets dynamic analysis with LLDB and GDB support.
- The debugger server page says remote debugging server is legacy unless
  remote debugging is needed, and that users should use the local debugger for
  local debugging.
- The tutorial says clicking a red margin column sets a breakpoint and a blue
  column sets a bookmark.
- The changelog references debugger attach, hardened runtime detection,
  debugger panel behavior, patch application before launching debugger,
  debugger environment variables, program-counter highlighting on conditional
  jumps, hex column display when PC/RIP is outside the main binary, and GDB
  remote target fixes.

Architecture inference:

- Hopper maps dynamic state onto static document routes. Breakpoints, current
  PC, process modules, patches, and runtime memory need to align with the same
  address/procedure/hex/graph model.

Damsel requirement:

- Start debugger integration as explicit local LLDB helper sessions with
  read-mostly memory and route-backed breakpoints.
- Model process modules, slides, mapped shared cache ranges, thread frames,
  registers, current PC, breakpoint locations, and debugger problems as
  overlays on project routes.
- Keep process writes and remote debugging out of early automation.

## Layer 13: Patching, Hex Editing, And Transactions

Public evidence:

- The tutorial describes synchronized hex editing, byte modification, undo/redo,
  assembler, and NOP region commands.
- The FAQ says Hopper can modify binaries with the hex editor, assembler, or
  Python byte writes.
- `HPDocument` exposes virtual-address writes.
- `HPDisassembledFile` exposes assembler, NOP data generation, and undo/redo
  transaction methods.
- The changelog mentions BSS modifications and patches being applied before
  launching the debugger.
- Public scripts include byte export, NOP insertion, JMP insertion, conditional
  branch inversion, and return-value patch helpers.

Architecture inference:

- Patching is coupled to byte types, procedure invalidation, assembler
  semantics, undo/redo, export, and debugger launch behavior.

Damsel requirement:

- Implement read-only synchronized hex first, but design a copy-on-write source
  revision model early.
- Add patch transactions with source revision, original bytes, proposed bytes,
  route, actor, reason, affected facts, invalidation plan, code-signing warning,
  and export policy.
- Keep in-place mutation of dyld caches, kernel collections, and signed system
  binaries outside the safe product path.

## Layer 14: Automation, MCP, And Scripting

Public evidence:

- The homepage markets Python scripting and MCP/AI integration.
- The FAQ says MCP configuration lives in Hopper Help and Python can modify
  individual bytes.
- The changelog shows multiple MCP iterations, including a C++ rewrite of the
  MCP server and document-discovery fixes.
- The changelog also shows repeated Python compatibility fixes.
- Public Python stubs expose document, segment, procedure, byte, comment,
  reference, name, decompile, local label, and mutation APIs.
- Public scripts automate shellcode annotation, Yara annotation, byte export,
  selection export, import fixing, class dump, procedure creation, pseudocode
  generation, branch patching, NOPs, strings, and annotation import/export.
- Third-party MCP wrappers expose documents, segments, address info, search,
  strings, disassembly, decompilation, call graphs, demangling, comments,
  names, and data type marking.

Architecture inference:

- Hopper automation is document automation. Scripts and MCP tools are useful
  because documents have stable objects and mutation operations.
- Third-party wrappers reveal demand for machine-readable access to Hopper's
  object graph, but also show why typed APIs are safer than ad hoc UI scraping.

Damsel requirement:

- Build CLI JSON and batch project APIs before an embedded scripting language.
- Add a local read-only MCP server over the same route/fact/query APIs.
- Add annotation mutation only through transactions with actor identity,
  capability checks, activity logs, inverse operations where possible, and
  Workbench visibility.
- Do not provide an LLM-only back door or raw project-wide byte dump.

## Layer 15: Plugin And SDK Boundaries

Public evidence:

- The homepage markets an extensible SDK for file format parsers and CPU
  backends.
- `HopperPlugin` defines SDK version, UUID, plugin type, display metadata,
  version, and command-line identifiers.
- SDK plugin classes include file loaders, CPU definitions, and tools.
- `HopperTool` exposes menu descriptions and selectors invoked against the
  current document.
- `FileLoader` handles file detection, loading, debug data, rebasing, composite
  extraction, and post-extraction setup.
- `CPUDefinition` and `CPUContext` expose architecture semantics above decoding.
- SDK docs use plugin install paths split by loader, CPU, and tool plugin
  type, and macOS plugins require signing on modern macOS.

Architecture inference:

- Hopper's extension surface is layered: loader, CPU, and tool extensions have
  different contracts and trust requirements.
- Decompiler internals are not fully public. Public SDK breadth does not imply
  every subsystem should be externally extensible.

Damsel requirement:

- Start with internal traits and stable schemas, not a public Rust ABI.
- Add manifest-driven helper-process extensions for loaders, sidecar importers,
  analyzers, exporters, and Workbench commands after the project API is stable.
- Record plugin id, version, schema version, capability set, provenance, and
  invalidation dependencies on plugin-produced facts.

## Layer 16: Performance, Scale, And Responsiveness

Public evidence:

- Hopper changelog repeatedly mentions large database loading time, large
  binary loading time, memory footprint, analysis time for very large binaries,
  decompiler memory efficiency, memory leaks, deadlocks, UI glitches, address
  list responsiveness, large document display, export deadlocks, and large
  function decompilation.
- Public MCP wrapper documentation says string API access can be slow enough
  that a string cache taking minutes per document is recommended. This is
  third-party evidence, but it is useful as a product warning.

Architecture inference:

- Hopper-class interaction requires paging, indexes, background jobs,
  cancellation, stable snapshots, and performance instrumentation.
- "It works on a small binary" is not a useful acceptance bar.

Damsel requirement:

- Add benchmark fixtures for standalone Mach-O, large userland binaries, dyld
  shared caches, projected cache images, kernel collections, KDK/dSYM-enriched
  projects, Objective-C-heavy binaries, Swift-heavy binaries, and C++/IOKit
  binaries.
- Track load time, first useful paint, route jump latency, search latency,
  graph expansion latency, inspector latency, hex window latency, MCP response
  latency, memory high-water mark, cancellation latency, and stale-fact
  cleanup.

## Source-To-Requirement Trace

| Public evidence | Hopper capability | Architecture inference | Damsel requirement |
| --- | --- | --- | --- |
| Tutorial byte typing | code/data/string/procedure/undefined map | byte map is central state | persistent typed-byte database |
| Tutorial undoable transformations | manual analysis edits | edits are commands with rollback | annotation and analysis transactions |
| Tutorial synchronized hex | assembly and bytes share selection | route identity spans views | shared route/selection model |
| Tutorial CFG minimap/group/color | graph is editable product state | graph layout is persisted separately | graph layout and override store |
| Tutorial inspector references | xrefs are contextual facts | inspector is route-aware | xref store and inspector modules |
| Tutorial Objective-C auto-tags | runtime facts affect navigation | runtime objects need tags/routes | ObjC object model and auto-tags |
| FAQ MCP configuration | official local tool surface | automation is product surface | local MCP over project APIs |
| FAQ Python byte modification | script mutation exists | mutation needs policy | capability-gated byte writes later |
| FAQ CPU/file formats | broad loader and CPU support | extension boundaries matter | loader and architecture traits |
| Debugger server page | local and remote debugging split | dynamic state overlays static routes | local LLDB helper first |
| Changelog large databases | project scale is product concern | persistence and indexes matter | analysis database performance gates |
| Changelog MCP rewrite | MCP has real maintenance cost | avoid duplicated tool APIs | one API for CLI/UI/MCP |
| Changelog dyld navigation | shared-cache extraction affects UX | container routes matter | cache image route model |
| Changelog driver cache | Apple system containers are first-class | loader needs Apple variants | driver cache source type |
| Changelog kernel cache | XNU binaries are in product scope | kernel objects need models | kernel/kext/KDK pipeline |
| Changelog Swift demangling | names and metadata affect UX | demangler is not enough | Swift-adjacent fact model |
| Changelog ObjC trampolines | arm64e patterns matter | architecture and runtime interact | PAC/ObjC selector analysis |
| SDK `HPDocument` | session, cursor, background, writes | document layer exists | `ProjectSession` |
| SDK `HPDisassembledFile` | mutable file database | document facts are rich | `AnalysisProject` fact store |
| SDK `FileLoader` | container/debug/rebase loading | source model is layered | `SourceArtifact` and sidecars |
| SDK `CPUContext` | semantic analysis contract | decoder is insufficient | `ArchitectureSemantics` |
| SDK `HPProcedure` | stack, variables, callers, signatures | procedure DB is semantic | persistent procedure database |
| SDK `HPTypeDesc` | structured type database | types are first-class | provenance-aware type system |
| Public scripts | automation fills UI gaps | users need scriptable objects | stable JSON and MCP APIs |
| HopperPyMCP | AI tools want same objects | object API must be typed | MCP tool schema fixtures |
| Desktop package evidence | signed native app, debugger entitlements | distribution/security affect design | signing and entitlement plan |

## Damsel Implementation Delta

Minimal architecture slices required before Damsel should claim
Hopper-adjacent closeness:

1. Project/document kernel:
   - `AnalysisProject`
   - `ProjectSession`
   - source registry
   - address-space registry
   - route resolver
   - job scheduler
   - problem store

2. Typed byte map:
   - byte classifications
   - provenance
   - confidence
   - invalidation dependencies
   - feature map API
   - transformation commands

3. Procedure database:
   - procedure seeds
   - basic blocks
   - CFG edges
   - stack facts
   - local labels
   - register aliases
   - callers/callees
   - signatures and calling conventions

4. Architecture semantics:
   - AArch64/AArch64e semantic instruction records
   - branch/call/return classification
   - switch candidates
   - thunks
   - NOP and assembler later
   - PAC/chained-fixup interpretation

5. Apple runtime object model:
   - Objective-C objects
   - Swift-adjacent names and metadata
   - C++ ABI objects
   - dyld shared-cache objects
   - kernel collection objects
   - KDK/dSYM profiles
   - panic-frame routing

6. Native Workbench:
   - project documents
   - route stack
   - synchronized assembly/graph/hex/feature map
   - symbol/string/tag/procedure browsers
   - contextual inspector modules
   - command registry
   - activity log

7. Automation boundary:
   - CLI JSON
   - batch queries
   - read-only MCP
   - audited annotation mutation
   - explicit capabilities
   - privacy and byte-read policy

8. Extension boundary:
   - internal traits first
   - manifest-driven helper processes later
   - loader/sidecar/analyzer/exporter/workbench command separation
   - versioned schemas and provenance

9. Patching and debugger foundations:
   - read-only hex
   - source revisions
   - patch preview transactions
   - route-backed LLDB sessions
   - static-dynamic module matching

10. Performance and quality gates:
    - large corpus fixtures
    - route latency budgets
    - memory budgets
    - cancellation tests
    - UI responsiveness tests
    - MCP contract snapshots

## Anti-Patterns This Research Rules Out

Do not treat Hopper closeness as:

- a flat disassembly table plus a graph
- a decompiler button bolted onto current output
- a syntax-highlighting exercise
- a browser UI facelift
- a scripting layer that bypasses the UI model
- an MCP server over prose summaries
- a plugin ABI exposing unstable Rust internals
- a debugger that does not understand static routes
- a hex editor that mutates bytes without invalidation
- an ObjC browser disconnected from selectors, IMPs, calls, tags, and xrefs
- a dyld shared-cache extractor that loses original-container routes
- a kernel-cache view without KDK/dSYM and panic-frame workflows
- a performance claim based only on toy fixtures

## Strongest Damsel Opportunity

Hopper's public feature set is broader than Damsel should chase immediately:
Mach-O, PE, ELF, multiple CPU families, decompilation, debugging, patching,
Python, MCP, and SDK plugins.

Damsel's plausible advantage is depth, not breadth:

- dyld shared-cache and driver-cache workflows
- projected image routes that preserve cache context
- Objective-C selector/class/category/method navigation
- Objective-C++ and C++ ABI recovery for Apple frameworks and IOKit
- arm64e PAC and chained-fixup-aware analysis
- kernel collections, kexts, KDK/dSYM profiles, IOKit classes, MIG, Mach
  traps, syscalls, and panic logs
- auditable evidence packages for Apple system binary research

But depth only matters if the workbench feels like a real interactive
disassembler. The architecture must first become Hopper-like: persistent,
typed, routeable, incremental, undoable, automatable, and responsive.

## Acceptance Criteria For This Research Thread

This research pass is useful only if later implementation planning can point
back to concrete requirements:

- every major Hopper feature family maps to a Damsel subsystem
- every Damsel subsystem has a source-grounded reason to exist
- source-derived facts are separated from inference
- current Hopper version facts are anchored to the official release API
- SDK-derived architecture claims name the relevant public object family
- public automation evidence informs Damsel APIs without copying fragile
  wrapper designs
- Apple-specific feature work remains first-class rather than hidden behind
  generic disassembler terms

