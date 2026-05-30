# Hopper Deep Public-Web Research Addendum

Status: deeper public-web research pass, written after the first Hopper-gap
research was judged too shallow.

Date of research pass: 2026-05-26.

Purpose: reconstruct Hopper Disassembler as a full product and architecture from
public sources, not as a vague checklist of "disassembly, graph, pseudocode."
This document deliberately over-indexes on workflow, data model, app behavior,
and extension boundaries because those are the parts easiest to underestimate.

## Source Set And Confidence

Primary Hopper sources:

- Homepage: <https://www.hopperapp.com/index.html>
- Tutorial: <https://www.hopperapp.com/tutorial.html>
- FAQ: <https://www.hopperapp.com/faq.html>
- Download page: <https://www.hopperapp.com/download.html>
- Changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Debugger Server page: <https://www.hopperapp.com/HopperDebuggerServer/>
- 2015 MacGeneration interview with Hopper's author:
  <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>

Public SDK and automation evidence:

- Unofficial Hopper SDK v4 mirror:
  <https://github.com/makigumo/HopperSDK-v4>
- Raw SDK header example for `HPDisassembledFile`:
  <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/HPDisassembledFile.h>
- Raw SDK header example for `CPUDefinition`:
  <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUDefinition.h>
- Xcode templates for Hopper plugin projects:
  <https://github.com/makigumo/hopper-xcode-templates>
- Public Hopper script collection:
  <https://github.com/droe/hopper-scripts>
- Public Swift demangling plugin:
  <https://git.hubp.de/keith/hopper-swift-demangle>
- Public procedure-name export workflow:
  <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- Public IL2CPP import script workflow:
  <https://iosgods.com/topic/182321-unity-il2cppdumper-for-hopper-disassembler-on-macos/>
- Public third-party MCP notes:
  <https://www.mcp.pizza/mcp-server/CL7N/HopperMCP>

Third-party product and usage evidence:

- Desktop Insights app package page:
  <https://desktopinsights.vercel.app/apps/hopper-disassembler>
- Mike Ash 2012 article:
  <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- 0xabad1dea decompiler article:
  <https://abad1dea.tumblr.com/post/23487860422/analyzing-binaries-with-hoppers-decompiler>
- State of Security 2012 review:
  <https://stateofsecurity.com/tool-review-hopper-disassembler-for-os-x/>
- J. Viotti private-framework workflow:
  <https://www.jviotti.com/2023/11/20/exploring-macos-private-frameworks.html>

Confidence labels used below:

- Official fact: stated by Hopper's own site, changelog, FAQ, tutorial, download
  page, debugger page, or the author interview.
- Public API evidence: visible in SDK headers, public scripts, plugin templates,
  or public plugin repos. This is strong evidence for the exposed object model,
  but not a promise about private internals.
- Community workflow evidence: public user workflows and scripts. This is useful
  for product expectations, but should not be treated as official product scope.
- Third-party package evidence: external app/package analysis. This can inform
  architecture guesses but should remain lower confidence than official sources.

## Executive Correction

The earlier shorthand "Hopper has disassembly, CFG, pseudocode, hex, debugger,
scripts, SDK" is directionally true but architecturally weak. The public sources
show a more demanding product:

- Hopper is a document-centric interactive analysis database.
- Every byte can have a type and those types can be inferred, manually changed,
  reverted, and reflected in a feature map.
- Procedures are not only instruction ranges. They own basic blocks, stack
  facts, local labels, register names, callers, callees, signatures, calling
  conventions, tags, and graph state.
- Assembly is not a flat string dump. The rendering layer knows names, local
  names, registers, comments, strings, CoreFoundation strings, Objective-C
  classes, selectors, protocols, addresses, numbers, and operands.
- The inspector is a modular live object inspector, not a sidebar of static
  metadata.
- The graph is editable product state with labels, comments, colors, grouping,
  keyboard movement, minimap, and export.
- Hopper's scripting, SDK, and MCP features all orbit the same document object
  model. That makes document object IDs and mutation transactions central.
- Hopper's recent public changelog shows active work on performance,
  decompiler memory, large binaries, Python, MCP, Swift, dyld shared cache
  navigation, driver cache dylibs, kernel caches, switch analysis, and arm64e
  Objective-C selector trampolines.
- The author has publicly described the hard part as automatic analysis and
  structure reconstruction after instruction decoding, with instruction decode
  itself treated as the simpler part.

Damsel must therefore think in terms of a persistent, incremental, mutable
analysis workspace with Apple-runtime overlays. A better decoder and a graph
view alone would not bring it close to Hopper.

## Product Surface Reconstructed From Official Sources

### Native Mac Product, Not A Viewer

Official facts:

- Hopper calls itself a fast native macOS disassembler.
- The homepage emphasizes a polished native interface, keyboard shortcuts, and
  macOS integration.
- The FAQ says Hopper v3 used Apple's technologies and ARC; the author ties the
  platform choice to user experience and speed.
- The MacGeneration interview says Hopper was developed on OS X, for OS X, and
  that users feel the difference compared with a ported tool.

Architecture implication:

- The app shell is not ornamental. It is part of the reverse-engineering
  throughput. Keyboard routing, undo/redo, command palette or menu parity,
  inspector focus, large-list behavior, document save/restore, and native
  window behavior are competitive features.

Damsel requirement:

- Workbench must become a first-class macOS document app. A native shell wrapped
  around transient CLI responses cannot match Hopper's interaction model. It
  needs project documents, persisted analysis state, route restoration, menu
  commands, keyboard command routing, autosave, undo/redo, and native error
  surfaces.

### Document And Session Model

Official and community facts:

- Mike Ash describes Hopper as having documents separate from binaries; saved
  documents preserve comments and annotations across sessions.
- Hopper's tutorial repeatedly talks about cursor location, navigation stack,
  view synchronization, labels, comments, tags, colors, and undo/redo.
- Public scripts call `Document.getCurrentDocument()`, iterate segments, inspect
  named addresses, ask for files, set names, set comments, and write output.
- SDK headers expose document/file concepts, current UI updates, read/write by
  virtual address, background-process state, waiting/cancellation UI, and
  undo/redo transaction APIs.

Architecture implication:

- Hopper's product is built around a mutable project/document object. The binary
  is an input. The saved analysis document is the artifact the user works in.
- User edits are not loose decorations. They are structured facts on addresses,
  procedures, blocks, operands, tags, references, types, and byte ranges.

Damsel requirement:

- Add `AnalysisProject` as the center of the system. It must own source files,
  cache projections, address spaces, analysis jobs, byte facts, names, xrefs,
  annotations, routes, and UI state.
- Persist all analysis and user facts in an analysis database, with enough
  provenance to explain whether a value came from a Mach-O command, dyld cache,
  Capstone, ObjC metadata, a KDK/dSYM, a user edit, or an inference.

### Automatic Analysis Pipeline

Official facts:

- Hopper's tutorial says automatic analysis starts as soon as a file is loaded.
- The tutorial frames the core concept as associating types to bytes: data,
  ASCII, code, procedure, and undefined.
- The MacGeneration interview describes the disassembly stage as turning byte
  sequences into instructions, then says Hopper performs deeper analysis by
  following code paths, determining possible execution paths, rebuilding method
  structure, splitting methods into basic blocks, and separating code from data.
- The same interview says decompilation relies on heuristics built over years,
  and that instruction decoding is the simplest part compared with automated
  structure reconstruction.
- The changelog repeatedly references hangs, infinite loops, switch/case
  analysis, no-return detection, argument analysis, stack variable handling,
  dead code removal, large-function decompilation, and large-binary analysis
  time.

Architecture implication:

- Hopper is best modeled as a staged analysis engine:
  1. Load file/container.
  2. Establish address spaces, segments, sections, and entrypoints.
  3. Decode instructions.
  4. Classify bytes.
  5. Discover procedures.
  6. Build CFGs.
  7. Recover cross-references, direct memory references, and switch tables.
  8. Infer stack, variables, arguments, no-return behavior, and calls.
  9. Apply runtime overlays such as Objective-C selectors, Swift symbols, dyld
     cache metadata, chained fixups, and PAC-aware patterns.
  10. Render multiple synchronized views and optionally decompile.

Damsel requirement:

- Do not conflate `DisassemblyAnalysis` with the analysis database. It is a
  useful output today, but Hopper-like behavior needs durable intermediate
  stages, incremental invalidation, progress, cancellation, and partial
  results.
- Treat "make procedure" and "make data/string/code" as commands that update
  byte facts and enqueue reanalysis, not as one-off UI toggles.

### Byte Typing As The Core Model

Official facts:

- Hopper's tutorial explicitly lists byte types and explains manual
  transformation of bytes to data, ASCII, code, procedure, or undefined.
- The tutorial says data typing can cycle through byte, 16-bit integer, 32-bit
  integer, and so on.
- Hex editing can invalidate the instruction and procedure under the cursor and
  return the associated type to undefined.
- The navigation bar color-codes code, procedures, ASCII strings, data, and
  undefined regions.
- SDK headers expose byte type query/mutation and type-modifiability checks.

Architecture implication:

- The typed-byte map is not a view. It is a substrate for navigation,
  rendering, invalidation, patching, analysis diagnostics, and user correction.

Damsel requirement:

- Add a `ByteClassification` or `RangeFact` store covering file offsets and VM
  addresses with source, confidence, user override state, and invalidation
  relationship.
- Use it to drive feature-map rendering, hex selection, string discovery, data
  views, procedure creation, reanalysis scheduling, and problem reporting.
- Extend Hopper's categories with Apple-specific overlays: ObjC metadata,
  selector refs, class refs, method IMPs, chained fixups, PAC-stripped pointer
  targets, C++ vtables, RTTI, kernel tables, syscall/Mach trap tables, MIG
  descriptors, kext boundaries, and cache-image boundaries.

### Synchronized Representations

Official facts:

- Hopper's homepage says users can look at assembly, CFG, and pseudocode for a
  procedure.
- The tutorial says display modes include assembly, CFG, pseudocode, and hex.
- The tutorial states that most representations require procedure construction
  because procedures contain basic blocks, stack usage, and other structure.
- Hex mode is synchronized with assembly and selects instruction bytes.

Architecture implication:

- Views must be projections of the same underlying function, byte, and route
  model. If the assembly view, graph view, pseudocode view, hex view, and
  inspector each own their own state, the product will drift and feel weak.

Damsel requirement:

- Define a canonical `Route` and `Selection` model that can point to file
  ranges, VM addresses, functions, basic blocks, instructions, operands,
  ObjC/C++/Swift objects, cache images, and kernel objects.
- All views should subscribe to route/selection changes and pull semantic data
  from the project object store.

### Assembly Listing

Official facts:

- Hopper's assembly view shows address, mnemonic, operands, and optional
  instruction encoding.
- Branch arrows are shown in the margin.
- The margin supports breakpoints and bookmarks.
- Comments, inline comments, and names can be added from the listing.
- Public SDK rendering concepts include mnemonics, jump mnemonics, registers,
  names, local names, variable names, comments, strings, CoreFoundation strings,
  classes, selectors, protocols, addresses, formatted numbers, and operands.

Architecture implication:

- The assembly row model is semantic and addressable. It is not an opaque
  preformatted string.

Damsel requirement:

- Replace or wrap plain instruction strings with `AssemblyRow` and
  `AssemblySpan` records:
  address span, bytes span, mnemonic span, operand spans, target route,
  symbol/name span, selector/class/protocol span, type span, comment span,
  branch cue, xref badges, problem badges, and optional encoding.
- Keep render preferences separate from facts so operand formatting can change
  without recomputing analysis.

### Control-Flow Graph

Official facts:

- Hopper's graph mode represents procedures structurally.
- Users can edit comments and labels from the graph.
- Cursor movement can traverse basic blocks by keyboard.
- The inspector includes a graph minimap for the current procedure.
- Nodes can be grouped.
- Nodes can receive custom background colors.
- Node text can be edited.
- The homepage says the CFG is interactive and supports PDF export.

Architecture implication:

- A Hopper-like graph is a work surface, not just a diagram.
- Graph layout, grouping, color, edited text, selection, route synchronization,
  and export must be persisted or reproducible.

Damsel requirement:

- Add a `GraphViewState` and `BasicBlockPresentation` layer separate from CFG
  facts. CFG facts answer "what is the control flow"; presentation facts answer
  "how has the user arranged and annotated it."
- Add block-level commands for labels, comments, colors, grouping/collapse,
  focus, predecessor/successor inspection, and export.
- Make large graphs virtualized and cancellable. Hopper's changelog shows graph
  and large-function performance are real product maintenance concerns.

### Pseudocode And Decompiler

Official facts:

- Hopper markets a C-like decompiler.
- The tutorial says pseudocode is intended to be functionally equivalent but
  imperfect, and warns that unreachable/dead code may disappear.
- The FAQ says pseudocode is usually not directly compilable and should not be
  treated as full source recovery.
- The author interview says decompilation tries to recover information lost
  during compilation and optimization through many heuristics.
- The changelog shows continuing decompiler work: new v6 decompiler,
  large-function improvements, stack-variable handling, variable liveness,
  AST optimizations, dead-code removal, lowering memory footprint, Objective-C
  ivar display, conditional handling, and hangs/crashes.
- Public tutorial articles show decompiler failure modes around variadic calls,
  argument count confusion, globals, inlining, optimized code, and orphaned
  variables.

Architecture implication:

- Decompilation is not one feature. It is a family of IR, dataflow, type
  recovery, control-structure recovery, stack modeling, calling-convention
  modeling, language runtime overlays, and UI caveats.

Damsel requirement:

- Keep the staged decompiler plan honest:
  1. Function summary cards.
  2. Call/xref/dataflow narrative.
  3. Low-level IR with explicit unresolved facts.
  4. Stack frame and argument model.
  5. Type propagation and runtime overlays.
  6. Structured control recovery.
  7. Pseudocode with uncertainty markers.
- Never claim recompilable output. Expose "why this is uncertain" directly in
  pseudocode and inspector.

### Hex, Patching, Assembler, And Undo

Official facts:

- Hopper has a synchronized hex editor.
- Double-clicking bytes can modify them.
- Modifying instruction bytes can destroy the instruction/procedure state and
  fall back to undefined.
- Hopper provides undo/redo.
- Hopper has an embedded assembler and can NOP a selected region.
- FAQ says Python can modify individual bytes.
- SDK headers expose assembler and NOP-data APIs plus undo/redo transactions.

Architecture implication:

- Patching is an analysis-database transaction with invalidation, not just
  writing bytes.
- Even read-only tools should model byte spans and transaction boundaries early,
  otherwise adding patching later will destabilize every view.

Damsel requirement:

- Implement read-only synchronized hex/data view before write support.
- Add a patch-journal design now even if patching is deferred.
- Treat each hypothetical patch as a new source revision with reanalysis,
  invalidated byte facts, changed symbols/xrefs/problems, and export metadata.

### Search, Navigation, Tags, And Feature Map

Official facts:

- Hopper's left pane contains symbols and strings.
- The list can be filtered by tags and text.
- The tutorial describes fuzzy-style symbol filtering.
- Tags can apply to addresses, basic blocks, or procedures.
- Hopper auto-generates tags, including entrypoint tags and Objective-C
  class/category tags.
- Jumping to an address or symbol pushes the previous cursor onto a navigation
  stack.
- The navigation bar is a color-coded byte map with cursor indicator.
- Recent changelog entries mention command palette, selector navigation,
  method navigation for dyld-cache-extracted files, address copy shortcuts,
  and list selection annoyances.

Architecture implication:

- Search and navigation are core analysis infrastructure. They tie together
  byte typing, object IDs, user annotations, runtime metadata, and route stack.

Damsel requirement:

- Build a global search index over symbols, functions, strings, ObjC classes,
  selectors, protocols, C++ names, vtables, imports, exports, xrefs, comments,
  tags, bookmarks, cache images, kernel objects, problems, and source ranges.
- Build route history as a central service. Xrefs, search, graph, selector
  browser, cache providers/importers, type references, and debugger addresses
  must all use it.
- Build a feature map from byte classifications and overlays, not from a
  one-off section summary.

### Contextual Inspector

Official facts:

- Hopper's inspector adapts to current context.
- Tutorial modules include instruction encoding, operand format, comments,
  colors/tags, references, procedure info, basic block predecessor/successor
  details, and switch/case hints.
- The homepage says the inspector automatically adapts its content according to
  the context.
- SDK headers expose problems, refs, operand formatting, type database,
  names, comments, colors, tags, procedures, blocks, and analysis objects that
  naturally feed an inspector.

Architecture implication:

- Inspector modules are the user's local explanation layer. They reveal "what
  Damsel thinks this is, why, what else points here, and what can I change."

Damsel requirement:

- Define inspector module contracts by selected object type:
  instruction, operand, address, range, function, basic block, string, data,
  ObjC method, selector, class, protocol, C++ vtable, symbol, import, export,
  dyld binding, chained fixup, cache image, kernel object, and problem.
- Each module should expose facts, provenance, xrefs, commands, and confidence.

### Objective-C, Swift, And Apple Runtime Surface

Official facts:

- Hopper homepage highlights Objective-C and Swift support.
- It says Hopper decodes mangled Swift names and extracts Objective-C selectors
  and messages.
- Tutorial says Objective-C implementations are tagged with class/category
  names.
- FAQ confirms iOS binary support subject to FairPlay DRM limitations.
- Changelog mentions stripped Objective-C declarations, Objective-C ivar
  display, Objective-C stubs, retain/release functions, Objective-C header
  export, protocols from dyld shared cache, Swift metadata, Swift demangling,
  Swift strings, authenticated Objective-C selector trampolines, and dyld-cache
  method navigation.
- Public Swift demangling plugins and scripts show user demand for better Swift
  symbol readability.
- Public Apple private-framework workflows show Hopper being used to inspect
  dyld shared cache frameworks and export Objective-C headers.

Architecture implication:

- Hopper's Apple advantage is language/runtime semantic lifting, not only Mach-O
  parsing. It maps low-level code to selectors, classes, categories, protocols,
  Swift names, message sends, ivars, stubs, and cache context.

Damsel requirement:

- Make ObjC metadata first-class in the app:
  class browser, category browser, selector browser, method IMP routes,
  protocol conformance, property/ivar details, message-send candidates,
  retain/release patterns, stub/trampoline recognition, and cache-image owner.
- Add a Swift lane that starts with demangling and runtime metadata placeholders,
  then grows into protocol/type/conformance indexing where public formats and
  test corpus allow it.
- Make arm64e selector trampolines, pointer authentication stripping, and
  chained fixup resolution visible as facts rather than hidden string
  substitutions.

### dyld Shared Cache, Driver Cache, And Kernel Cache

Official facts:

- Hopper changelog repeatedly mentions dyld shared caches: extraction,
  compatibility with iOS/macOS releases, symbol reading, Objective-C header
  export, protocol reading, path discovery, files extracted from cache, method
  navigation, and crashes/hangs on malformed or old caches.
- Version 6.1.0 added a way to read dylibs from the driver cache.
- Version 5.10.0 added iOS and macOS kernel cache support for non-encrypted
  caches.
- Version 5.3.1 mentioned procedure creation using PAC.
- Version 5.2.0 handled iOS 15/macOS 12 chained fixup pointers.

Architecture implication:

- Hopper treats Apple system containers as living product targets that need
  continuous compatibility work, not as ordinary Mach-O files.

Damsel requirement:

- This is Damsel's most credible edge:
  cache-native source registry, cache-wide search, image projection, cross-image
  xrefs, provider/importer/reexport graphs, ObjC class/selector ownership,
  local symbols, fixups, stub islands, kernel collections, KDK/dSYM attachment,
  IOKit/C++ models, driver-cache source kind, and panic/backtrace mapping.
- Track OS-version-specific parser compatibility explicitly. Apple's system
  binary formats drift, so Damsel needs fixture tiers by macOS/iOS version and
  build provenance.

### C++ And Objective-C++

Official and API facts:

- Hopper changelog mentions improved method signature computation from C++
  mangled names.
- SDK headers expose signatures, calling conventions, type databases, structured
  types, local variables, stack variable names, and procedure metadata.
- Hopper's Apple workflows necessarily encounter Objective-C++ in IOKit,
  private frameworks, and system binaries even where public marketing says less
  about C++.

Architecture implication:

- C++ is not optional for an Apple-system-binary-focused competitor. Kernel and
  IOKit analysis depend on vtables, RTTI, constructors, destructors, virtual
  dispatch, and symbolication.

Damsel requirement:

- Add Itanium ABI demangling, vtable and RTTI discovery, constructor/destructor
  detection, virtual call candidate xrefs, namespace/class grouping, type
  provenance, KDK/dSYM type import, and Objective-C++ mixed ownership views.

### Debugger And Dynamic Analysis

Official facts:

- Hopper homepage says it supports LLDB and GDB dynamic analysis.
- The tutorial describes breakpoint interaction in the assembly margin.
- The Debugger Server page says remote debugging server is legacy for remote
  debugging, local debugger should be used when debugging on the same computer,
  and the server requires Xcode on the target machine.
- Download page says the demo disables the debugger backend.
- Changelog mentions debugger fixes, GDB remote target handling, environment
  variables in debugger, hardened runtime detection, attaching dialogs, patches
  before launching debugger, highlighting conditional path under the program
  counter, and debugger panel issues.
- Desktop Insights reports debugger-related entitlements in the app package,
  although that is third-party package evidence.

Architecture implication:

- Hopper's debugger is tied to static routes. Breakpoints, PC display, live
  register/memory state, branch highlighting, attach/launch, hardened runtime,
  and rebasing all need to map back into the analysis document.

Damsel requirement:

- Defer live debugging until project source identity, address-space mapping,
  rebasing, byte ranges, function objects, and route stack are reliable.
- When implemented, create `DebugSession`, `RuntimeImage`, `Breakpoint`, and
  `LiveThreadFrame` models that map runtime addresses back to static objects.
- Avoid patch/debug feature work until read-only analysis has transaction and
  provenance foundations.

### Python Scripting

Official and community facts:

- Hopper homepage markets Python scripting.
- FAQ says Python can modify bytes.
- Changelog repeatedly fixes Python compatibility and Python API issues.
- Public scripts annotate shellcode, stack strings, and YARA matches; copy bytes
  as Python/YARA; add file-offset comments; fix imports by ordinal; carve byte
  blobs; save selections; and use wrapper APIs around Hopper's procedural API.
- The 2025 procedure-name export workflow uses the current document, segments,
  named addresses, and names.
- The IL2CPP workflow imports an external JSON file and applies names/comments
  into Hopper.

Architecture implication:

- Scripting is not just batch mode. It is document automation over the active
  analysis database and selection.

Damsel requirement:

- Expose stable project APIs before embedding a scripting runtime.
- Start with external API/MCP-style tools and CLI JSON over object IDs, then
  add in-app scripting only after the mutation/undo/audit model exists.
- Include safe classes of mutation: names, comments, tags, colors, manual refs,
  type annotations, and problem suppressions. Defer byte patching.

### SDK, Plugins, And Extension Boundary

Official and public API facts:

- Hopper homepage says the SDK can add custom file format parsers and CPU
  architecture backends.
- Download page exposes an SDK download.
- Xcode templates exist for CPU, loader, and tool plugins.
- Public SDK mirror includes core document/file/segment/section/procedure/block
  concepts plus loader, CPU, and tool plugin protocols.
- CPU headers expose CPU families/subfamilies, CPU modes, syntax variants,
  registers, frame pointer/stack/program-counter recognition, NOP generation,
  assembler capability, and decompiler capability flags.
- Public plugins include Swift demangling and third-party CPU/loader examples.

Architecture implication:

- Hopper's extension story has at least three boundaries:
  loader plugins, CPU semantics plugins, and tool plugins. Those are different
  from scripts, which automate the document.

Damsel requirement:

- Do not expose unstable Rust internals as a plugin API. First stabilize:
  source loaders, address-space model, decoded instruction model, semantic
  facts, object IDs, query APIs, and mutation transactions.
- Long-term plugin categories should be loader, analyzer, runtime metadata
  importer, symbol/type provider, renderer/exporter, and report/tool command.

### MCP And AI Surface

Official and community facts:

- Hopper homepage says the product is enhanced with AI through an integrated
  MCP server.
- FAQ says MCP configuration instructions live in Hopper Help.
- Changelog entries from v6.0 onward mention MCP server addition, reliability,
  launch behavior, document discovery, tool-access efficiency, and a rewrite in
  C++ to avoid a Swift dependency and share code on macOS/Linux.
- Public third-party MCP wrappers expose address/segment inspection, names,
  strings, comments, data types, procedures, xrefs, and decompiled/disassembled
  code through tool calls, while noting that some access required private
  interfaces in that third-party wrapper.

Architecture implication:

- AI integration is not a chatbot bolted onto the UI. It is an automation and
  query surface over the document model.

Damsel requirement:

- Build a local project query API with explicit object IDs, provenance, paging,
  permission categories, mutation audit, and read-only default tools.
- MCP tools should call the same APIs as Workbench and CLI. Any separate AI data
  model will become a product fork and break trust.

### Performance And Responsiveness

Official facts:

- Changelog entries in 2026 mention large-binary loading time, big-binary
  analysis time, large database loading time, memory footprint, decompiler
  memory leaks, decompiler memory efficiency, large-function decompilation,
  hangs, deadlocks, and responsiveness of address lists.
- SDK headers include background process state, wait messages, and cancellation
  concepts.
- The author interview stresses that automated analysis and heuristic
  reconstruction are large bodies of work.

Architecture implication:

- Hopper-level polish includes performance maintenance as a continuous product
  lane. Large binaries, dyld caches, kernel caches, and decompiler passes all
  stress memory, scheduling, UI responsiveness, and cancellation.

Damsel requirement:

- Add performance budgets to every subsystem:
  open/import latency, first meaningful paint, symbol search latency, route jump
  latency, graph layout latency, inspector query latency, cache image projection
  latency, reanalysis latency, memory ceilings, cancellation response time, and
  partial-result behavior.
- Make analysis jobs incremental and pageable. Avoid "load everything into a
  single response" APIs for Workbench.

### App Package And Security Posture Clues

Third-party package evidence:

- Desktop Insights reports Hopper v6.2.5 demo as a Swift app, 71 MB, universal
  Apple Silicon and Intel, code signed, notarized, hardened runtime enabled,
  using Sparkle and Protocol Buffers, and holding code-signing/debugger-related
  entitlements.

Confidence caveat:

- This is not official Hopper documentation and should be treated as external
  app-package observation.

Architecture implication:

- Even a commercial native reverse-engineering app needs packaging, updates,
  notarization, entitlements, hardened runtime exceptions, and a clear security
  posture because debugger/plugin/JIT-like features may require unusual
  permissions.

Damsel requirement:

- Damsel Workbench should plan packaging and entitlements early if it will ever
  debug, load plugins, run scripts, or expose local automation servers.
- Security UX should explain when Damsel is read-only, when it writes project
  state, when it launches local servers, and when future debugger/patch features
  require elevated or unusual permissions.

## Architecture Reconstruction

This is inference from the public product surface, SDK headers, scripts, and
changelog. It is not a claim about Hopper's private implementation.

### Layer 1: Source And Container Layer

Probable responsibilities:

- Read executable/container input.
- Recognize file type.
- Establish CPU family/subfamily, CPU mode, address width, integer width,
  endian behavior, base address, and entrypoints.
- Create segments and sections.
- Map file offsets to virtual addresses.
- Support Mach-O, PE, ELF, dyld caches, driver caches, kernel caches, FAT
  binaries, and plugin-defined loaders.

Damsel analogue:

- `SourceRecord`, `AddressSpace`, `RangeMap`, `SegmentRecord`,
  `SectionRecord`, `CacheImageRecord`, `KernelCollectionRecord`,
  `ProjectedImageRecord`, and `LoaderFact`.

### Layer 2: Typed Byte Database

Probable responsibilities:

- Store byte classifications and confidence.
- Distinguish code, procedure, string, data, undefined, and user-overridden
  ranges.
- Reflect manual make-code/data/string/procedure commands.
- Drive navigation map and hex synchronization.
- Invalidate affected procedures and xrefs after mutation.

Damsel analogue:

- `ByteFact`, `RangeFact`, `ByteClassification`, `FactProvenance`,
  `FactConfidence`, `UserOverride`, and `InvalidationEdge`.

### Layer 3: Architecture Semantics

Probable responsibilities:

- Decode instructions.
- Provide syntax variants and CPU modes.
- Recognize registers, stack pointer, frame pointer, program counter, side
  effect registers, NOP bytes, assembler support, and branch behavior.
- Feed procedure analysis hooks, direct references, switch detection, and
  decompiler eligibility.

Damsel analogue:

- `InstructionRecord`, `OperandRecord`, `ArchitectureSemantics`,
  `RegisterSemantics`, `BranchSemantics`, `MemoryReferenceCandidate`,
  `SwitchCandidate`, and `CallSemantics`.

### Layer 4: Procedure And Graph Database

Probable responsibilities:

- Create and remove procedures.
- Recursively analyze from entrypoints and potential procedure starts.
- Store basic blocks, predecessors, successors, exits, callers, callees, stack
  offsets, local labels, register aliases, variable names, signatures, calling
  conventions, and tags.
- Support graph layout and editable graph presentation.

Damsel analogue:

- `FunctionRecord`, `BasicBlockRecord`, `ControlFlowEdgeRecord`,
  `CallEdgeRecord`, `StackFrameRecord`, `LocalLabelRecord`,
  `RegisterAliasRecord`, `FunctionSignatureRecord`, `CallingConventionRecord`,
  and `GraphPresentationRecord`.

### Layer 5: Name, Type, Annotation, And Xref Store

Probable responsibilities:

- Store global and local names.
- Store comments and inline comments.
- Store colors and tags.
- Store operand formatting preferences.
- Store manual and inferred references.
- Store problem markers.
- Store structured types, enums, unions, pointers, arrays, imports, and exports.
- Keep undo/redo transaction boundaries.

Damsel analogue:

- `NameRecord`, `CommentRecord`, `TagRecord`, `ColorRecord`,
  `OperandFormatRecord`, `ReferenceRecord`, `ProblemRecord`, `TypeRecord`,
  `TypeSourceRecord`, `AnnotationTransaction`, and `UndoJournal`.

### Layer 6: Runtime Metadata Overlays

Probable responsibilities:

- Lift Objective-C metadata to class/category/protocol/selector/message views.
- Demangle Swift and attach symbol/type metadata where possible.
- Resolve dyld-cache images, imports, providers, local symbols, protocols, and
  Objective-C headers.
- Interpret chained fixups, PAC patterns, authenticated selector trampolines,
  and system container quirks.
- Lift C++ signatures, vtables, RTTI, and Objective-C++ patterns where possible.

Damsel analogue:

- `ObjcClassRecord`, `ObjcMethodRecord`, `ObjcSelectorRecord`,
  `ObjcMessageCandidate`, `SwiftNameRecord`, `SwiftMetadataRecord`,
  `CxxTypeRecord`, `VtableRecord`, `RttiRecord`, `FixupRecord`,
  `PacFact`, `DyldCacheGraph`, `KernelSymbolicationProfile`, and
  `SystemContainerFact`.

### Layer 7: Representation And UI Services

Probable responsibilities:

- Render semantic assembly.
- Render CFG, minimap, and graph presentation.
- Render pseudocode/decompiler output.
- Render hex/data views.
- Provide contextual inspector modules.
- Provide navigators, search, feature map, route stack, bookmarks, tags,
  exports, and command palette.

Damsel analogue:

- `Route`, `Selection`, `AssemblyRow`, `GraphViewModel`, `DecompilerViewModel`,
  `HexViewModel`, `InspectorModule`, `SearchIndex`, `FeatureMapTile`,
  `NavigationStack`, `BookmarkRecord`, `ExportJob`, and `CommandRecord`.

### Layer 8: Automation, Debugger, And Extension Services

Probable responsibilities:

- Python scripting over the document.
- SDK plugin loading.
- MCP server over document tools.
- Local and remote debugger bridges.
- Breakpoints, runtime mapping, register/memory state, attach/launch, and
  dynamic branch highlighting.

Damsel analogue:

- `ProjectToolApi`, `ScriptExecution`, `PluginManifest`, `PluginCapability`,
  `McpTool`, `AutomationAuditRecord`, `DebugSession`, `RuntimeImageMap`,
  `BreakpointRecord`, and `LiveStateRecord`.

## What Damsel Must Add To Be "Close To Hopper" In The Apple Lane

### Foundation Must-Haves

- Persistent project documents.
- Source registry for standalone Mach-O, dyld cache image, driver cache image,
  kernel collection, extracted framework, dSYM/KDK, and future patched revision.
- Address-space/range model that can express file offsets, VM addresses, cache
  addresses, projected image addresses, and runtime slide-adjusted addresses.
- Typed-byte map with user overrides.
- Fact provenance and confidence.
- Analysis jobs with progress, cancellation, partial results, and problem list.
- Undo/redo transaction journal for annotation and manual analysis edits.

### Interaction Must-Haves

- Global route stack.
- Global search and fuzzy filtering.
- Feature map.
- Contextual inspector.
- Semantic assembly rows.
- Interactive CFG with minimap.
- Symbol, string, function, ObjC, C++, cache, kernel, import/export, and
  problem navigators.
- Persistent comments, inline comments, names, colors, tags, bookmarks, manual
  references, operand formats, and local labels.

### Apple Edge Must-Haves

- Cache-native cross-image xrefs.
- Provider/importer/reexport navigation.
- Objective-C class/category/protocol/selector/message browser.
- Swift demangle search and source grouping.
- arm64e PAC and authenticated selector-trampoline facts.
- Chained-fixup pointer target database.
- C++ vtable/RTTI/virtual-call recovery.
- IOKit class and user-client workflows.
- Kernel collection/fileset browser.
- KDK/dSYM symbolication profiles.
- Panic/backtrace route mapping.

### Deferred But Architecturally Reserved

- Decompiler IR and pseudocode.
- Read-write patching.
- Assembler and NOP region commands.
- Local LLDB bridge.
- Remote debug server.
- Embedded scripting runtime.
- Public plugin API.
- MCP server.

The important point is that these can be deferred as user-visible features, but
their data model constraints cannot be ignored. A project store without object
IDs, provenance, undo, transactions, and route mapping will block them later.

## Comparison Against The Earlier Research Pack

What the earlier docs got right:

- Hopper has an interactive Mac app, disassembly, CFG, pseudocode, hex,
  inspector, annotations, ObjC/Swift, debugger, scripts, SDK, MCP, and patching.
- Damsel should specialize in Apple system binaries rather than chase PE/ELF
  breadth.
- Damsel needs an analysis database, graph/xref indexes, ObjC navigation,
  C++/ObjC++ recovery, XNU/kernel collection workflows, arm64e semantics,
  decompiler staging, and performance work.

What was underweighted:

- Byte typing is the foundation, not a small feature.
- User type mutation plus undo/redo imply a transaction system.
- Hopper's document model is more central than file loading.
- The feature map depends on byte classifications.
- Graph presentation is editable, not just computed.
- Procedure-local facts are rich: local labels, register aliases, stack
  variables, signatures, calling conventions, tags, callers, callees, exits.
- Operand formatting is persistent user state.
- Manual references are part of the xref model.
- Problem lists and background analysis controls are first-class.
- Scripting is active-document automation, not just external batch commands.
- SDK plugins separate loader, CPU, and tool extension categories.
- MCP is a shipping Hopper surface, not a vague future AI idea.
- Recent Hopper work is heavily about performance, memory, large binaries,
  Python compatibility, MCP reliability, Swift/ObjC, dyld cache, and arm64e.
- The author publicly frames instruction decoding as the simpler part and
  automated structure reconstruction as the real value.

## Roadmap Adjustments For Damsel

### Add A Project Kernel Before More Views

Build the project/document kernel before adding another ad hoc Workbench view.
It should contain source registry, object IDs, address spaces, byte facts,
names, xrefs, annotations, jobs, problems, route stack, and persisted UI state.

Why:

- Every Hopper-equivalent feature depends on this.
- It prevents the graph, inspector, search, automation, and debugger from
  inventing separate object identity systems.

### Treat Byte Typing As Wave 1

Move byte classification into the first serious Hopper-gap implementation wave.

Minimum:

- Inferred ranges from Mach-O segments/sections.
- Code ranges from decoded instructions.
- Procedure ranges from function discovery.
- CString ranges from section and scanner facts.
- Data ranges from sections and fixups.
- Undefined ranges as explicit negative space.
- User override records.

Apple extensions:

- ObjC metadata ranges.
- Chained fixup ranges.
- PAC-authenticated pointer ranges.
- Cache image boundaries.
- Kernel/fileset/kext ranges.

### Make Semantic Assembly The Rendering Contract

Stop treating instruction rendering as a final string in the UI boundary.
Expose spans and semantic roles so search, selection, inspector, xrefs,
comments, type overrides, and accessibility can work correctly.

Minimum span roles:

- address
- bytes
- mnemonic
- register
- immediate
- memory operand
- symbol
- local label
- selector
- class name
- protocol name
- string
- CFString
- branch target
- call target
- comment
- problem
- xref badge

### Add Procedure-Local State

Hopper's exposed procedure model includes much more than blocks and edges.
Damsel should add durable records for:

- local labels
- register aliases
- stack slots
- argument candidates
- local variable names
- frame facts
- call edges
- caller/callee sets
- exits
- function signatures
- calling conventions
- function-level tags
- graph presentation state

### Split Analysis Facts From Presentation Facts

CFG facts should be reproducible. User graph layout, colors, groups, and edited
text should be presentation facts. Pseudocode facts should be separate from
rendered text. Assembly render preferences should be separate from instruction
semantics.

Why:

- It allows recomputation without destroying user work.
- It lets Damsel diff analysis revisions.
- It makes export and scripting predictable.

### Make The Inspector A Platform

Add an inspector module registry. Each selected object type should have modules
with stable IDs, query inputs, output models, commands, and latency budgets.

Early modules:

- instruction
- operand
- byte range
- function
- basic block
- xrefs
- comments/tags
- ObjC object
- dyld fixup/binding
- cache image
- symbol/source provenance
- problem

Later modules:

- C++ type/vtable
- Swift metadata
- kernel object
- live debugger state
- patch preview
- decompiler explanation

### Build Automation APIs From The Same Model

Every public script/MCP/plugin example reinforces that automation must talk to
the same current document and object model as the UI.

Minimum Damsel API shape:

- list sources
- list address spaces
- resolve route
- read bytes by source/range
- query symbols
- query names
- query strings
- query functions
- query blocks
- query instructions
- query xrefs
- query ObjC objects
- set name
- set comment
- set tag
- set color
- add manual ref
- create problem note
- export selection

Safety:

- read-only by default
- explicit mutation tools
- project-local audit log
- no binary patch writes in the first automation release

## Acceptance Tests Implied By This Research

### Hopper-Comparable Document Test

Open a Mach-O, let staged analysis run, add a name, comment, tag, color, manual
reference, local label, bookmark, and operand format; close/reopen project; all
facts persist, render in assembly/graph/inspector/search, and undo/redo behaves
correctly within the current session.

### Typed-Byte Feature Map Test

Open a binary with known code, data, cstring, ObjC metadata, chained fixups, and
undefined padding. Feature map should show each category, route to the correct
view, update after manual type changes, and preserve source provenance.

### Procedure Object Test

Choose a function with branches, calls, stack slots, and an indirect jump.
Damsel should expose:

- function record
- block list
- edges
- callers
- callees
- exits
- stack facts
- jump-table candidate or problem
- local labels
- graph presentation state
- route from every row and block to inspector

### Apple Shared Cache Navigation Test

Open a dyld shared cache or projected image. Search for an Objective-C class,
jump to a method implementation, inspect selectors and class ownership, follow a
provider/importer edge to another image, navigate back, and see route history
preserved.

### arm64e Runtime Semantics Test

Use a fixture with chained fixups, authenticated pointers, and selector
trampolines. Damsel should show raw pointer facts, stripped targets, auth
context where recoverable, selector/message candidates, and confidence.

### Kernel/IOKit Test

Open a kernel collection or KDK-backed fixture. Damsel should show fileset or
kext boundaries, symbolication provenance, C++ vtable candidates, IOKit class
relationships where recoverable, and panic/backtrace address routing.

### Performance Test

Open a large cache/project. The UI should first paint quickly, analysis should
remain cancellable, symbol search should stay interactive, large lists should be
paged, graph layout should have progress/cancel, and memory growth should be
measured.

### Automation Test

Run an external tool or future MCP command to list segments, functions, names,
strings, and xrefs; set a comment; set a name; add a tag; then verify the UI,
CLI, and project database all see the same facts.

## Competitive Thesis After The Deeper Research

Damsel should not claim to be close to Hopper until it has a real document
model, typed-byte database, semantic assembly, route stack, search/index layer,
contextual inspector, persistent annotations, interactive CFG, and Apple
runtime overlays. Those are Hopper's core product shape.

Damsel can still beat Hopper in a narrower lane if it focuses on:

- dyld-cache-native workflows instead of treating caches as odd containers
- ObjC selector/class/category/message navigation across cache images
- C++/IOKit and kernel/KDK symbolication as first-class views
- arm64e PAC/chained-fixup semantics shown transparently
- provenance and confidence for every recovered fact
- script/MCP APIs designed around safe local facts from day one
- measurable responsiveness on large Apple system binaries

The practical engineering conclusion is blunt: the next work should be a
project analysis kernel and semantic object model, not another isolated parser
feature. Hopper's public surface is the surface of an analysis database with a
native app around it.
