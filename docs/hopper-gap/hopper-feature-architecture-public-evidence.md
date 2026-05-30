# Hopper Feature Architecture Public Evidence

Status: expanded public-web and public-SDK research pass.

Purpose: correct the remaining under-specified parts of the Hopper comparison.
This document treats Hopper Disassembler as a feature architecture, not just a
feature checklist. It reconstructs the public shape of Hopper from official
pages, the current public SDK archive, the SDK quick tour, Objective-C headers,
recent changelog entries, public workflow writeups, and public automation
projects.

This is not a claim about Hopper's private implementation. It is a planning
document for what Damsel must implement if it wants to feel close to Hopper in
the Apple systems lane.

## Source Set

Official product sources:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/download.html>
- <https://www.hopperapp.com/rss/html_changelog.php>
- <https://www.hopperapp.com/HopperDebuggerServer/>
- <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>

SDK artifacts inspected from the public Hopper SDK 6.3.0 archive:

- `SDK Documentation.pdf`
- `include/Hopper/HPDocument.h`
- `include/Hopper/HPDisassembledFile.h`
- `include/Hopper/HPSegment.h`
- `include/Hopper/HPSection.h`
- `include/Hopper/HPProcedure.h`
- `include/Hopper/HPBasicBlock.h`
- `include/Hopper/CPUDefinition.h`
- `include/Hopper/CPUContext.h`
- `include/Hopper/CommonTypes.h`
- `include/Hopper/HPTypeDesc.h`
- `include/Hopper/HPMethodSignature.h`
- `include/Hopper/HopperTool.h`
- `include/Hopper/FileLoader.h`
- sample loader, CPU, and tool plugin projects

Public workflow and community sources:

- <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>
- <https://abad1dea.tumblr.com/post/23487860422/analyzing-binaries-with-hoppers-decompiler>
- <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>
- <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- <https://github.com/droe/hopper-scripts>
- <https://glama.ai/mcp/servers/dflatline/HopperPyMCP>

Source confidence labels used below:

- Official fact: stated by Hopper's site, tutorial, FAQ, download page,
  debugger page, changelog, SDK quick tour, or current public SDK headers.
- SDK surface: visible API contract in the public Hopper SDK 6.3.0 archive.
- Public workflow evidence: observed use in public scripts, articles, or MCP
  wrappers. These show how users extend Hopper, but do not prove official
  product intent.
- Inference: architecture Damsel should implement because multiple public
  Hopper surfaces imply that shape.

## Executive Correction

The earlier shorthand "Hopper is a disassembler with CFG, pseudocode, hex,
debugger, Python, SDK, and MCP" is still too small. The public evidence points
to a larger architecture:

1. Hopper is a document application.
2. The document owns a mutable disassembled file.
3. The disassembled file owns segments, sections, byte types, names, comments,
   operand formats, procedures, tags, colors, problems, types, and transactions.
4. Procedures are first-class semantic objects with frame information, basic
   blocks, stack offsets, local labels, register renames, variables, callers,
   callees, signatures, calling conventions, and tags.
5. CPU plugins are not mere decoders. They participate in address alignment,
   CPU modes, prolog detection, padding, branch recovery, switch detection,
   thunk detection, instruction-specific analysis, rendering, assembly, and
   decompiler cooperation.
6. Hopper's UI is a synchronized projection of document state: assembly, graph,
   pseudocode, hex, feature map, symbol lists, string lists, tag filters,
   inspector modules, navigation stack, and debugger state all orbit the same
   document.
7. Hopper's automation is document automation. Python scripts, tool plugins,
   third-party MCP wrappers, and Hopper's integrated MCP server make sense
   because the document has stable objects and mutation APIs.
8. Hopper's recent release history shows continuous investment in large
   binaries, decompiler memory use, MCP reliability, dyld shared cache
   navigation, driver cache reading, Swift demangling, ObjC trampolines,
   switch/case recovery, and native app polish.

Damsel does not get close to Hopper by adding another table of decoded
instructions. The minimum credible architecture is a project/document kernel,
typed byte map, procedure database, architecture semantics layer, xref graph,
annotation transaction model, native route-synchronized workbench, and
Apple-system semantic overlays.

## Official Product Surface

### Native App And Interaction Model

Official facts:

- Hopper markets itself as a fast native macOS disassembler.
- The homepage emphasizes a polished Cocoa interface, keyboard shortcuts, and
  native macOS integration.
- Hopper for Mac currently requires macOS 12.4 or higher.
- The download page exposes Mac, Linux, and SDK downloads. The Mac build is a
  universal binary for Intel and Apple Silicon.

Architecture implication:

- Hopper's user value is partly the native document workflow, not just analysis
  algorithms.
- Damsel's native Workbench must become a real document workbench with menus,
  keyboarding, command palette, window state, progress, cancellation, route
  sync, and undo surfaces.
- A browser-style point-query shell cannot be considered Hopper-close even if
  the underlying parser is strong.

### File And Processor Breadth

Official facts:

- Hopper supports Mach-O, PE, and ELF file formats.
- Hopper can disassemble Intel 32-bit, Intel 64-bit, ARMv6, ARMv7, ARM64, and
  PowerPC binaries.
- Hopper can disassemble iOS binaries when the input is legally readable and
  not still FairPlay-encrypted.
- The SDK can extend Hopper with custom file format parsers and CPU backends.

Planning decision for Damsel:

- Damsel should not chase Hopper's generic file and CPU breadth first.
- Damsel can still be competitive if it is much deeper for macOS, iOS, dyld
  shared caches, kernel collections, KDK/dSYM symbolication, Objective-C,
  C++, Objective-C++, arm64e, PAC, and XNU-style workflows.
- Damsel's architecture must nevertheless be modular enough that loader,
  architecture semantics, procedure recovery, and rendering boundaries are
  explicit. Hopper's SDK makes those boundaries public.

### Automatic Analysis And Byte Typing

Official facts:

- The tutorial says Hopper starts automatic analysis as soon as a file is
  loaded.
- The tutorial frames Hopper's core concept as transforming bytes into
  human-readable structure by assigning a type to each byte.
- Public byte types in the tutorial include data, ASCII, code, procedure, and
  undefined.
- The SDK `CommonTypes.h` exposes byte types such as undefined, outside, next,
  integer widths, ASCII, Unicode, external address, external function, align,
  data meta-type, code, procedure, and structure.
- The SDK `HPDisassembledFile` exposes `typeForVirtualAddress`,
  `setType:atVirtualAddress:forLength:`, `typeCanBeModifiedAtAddress`,
  `hasCodeAt`, and `findNextAddress:ofTypeOrMetaType:wrapping:`.

Architecture implication:

- The byte map is not a UI decoration. It is a central analysis database.
- Making a byte code, data, string, procedure, or undefined must update
  analysis state, invalidate affected procedure/xref facts, and record user
  provenance.
- The feature map should be backed by byte classifications rather than section
  coloring.
- Damsel's first Hopper-grade implementation wave needs a typed byte map before
  graph polish, pseudocode, MCP mutation, or debugger mapping.

### Synchronized Representations

Official facts:

- Hopper displays assembly, CFG, pseudocode, and hex representations.
- The homepage says users can inspect assembly, CFG, and pseudocode for a
  procedure at the same time.
- The tutorial says most display modes require procedure construction because
  procedures contain basic blocks, stack usage, and other structure.
- Hex mode is synchronized with assembly and selects all bytes of the current
  instruction.
- Hopper 6.0.2 added a menu item to toggle view synchronization.

Architecture implication:

- Representations are not independent renderers. They are views over a shared
  procedure/address/selection model.
- Damsel needs a route and selection system that can move between assembly row,
  basic block, graph node, hex byte range, pseudocode statement, symbol,
  selector, class, xref, and debugger address.
- The Workbench store split should separate session/project state, selection
  state, representation caches, inspector data, jobs, problems, and command
  dispatch.

### Navigation, Search, Tags, And Inspector

Official facts:

- Hopper's left pane contains symbols and strings.
- Symbol search supports fuzzy matching behavior described in the tutorial.
- Tags can be applied to addresses, basic blocks, and procedures.
- Tags can filter the symbol/procedure list.
- Hopper auto-generates tags during loading, including entrypoint tags and tags
  for Objective-C class/category implementations.
- Hopper has a navigation stack for jumps and backtracking.
- Hopper has a navigation bar that color-codes byte categories and shows cursor
  position.
- Hopper's inspector adapts to context and includes instruction encoding,
  operand format controls, comments, colors/tags, references, and procedure
  details.

SDK surface:

- `HPDocument` exposes current address, selected address range, direct cursor
  movement, navigation-stack navigation, background process state, and waiting
  state.
- `HPDisassembledFile` exposes all names, named addresses, nearest names, name
  setting with reason, comments, inline comments, operand formats, colors,
  tags, and problem records.
- `HPBasicBlock` and `HPProcedure` both expose tag operations.

Architecture implication:

- Damsel needs a search/index subsystem, not a search box bolted onto the UI.
- Tags, colors, comments, labels, operand formats, and problem records must be
  project facts with transaction history.
- Inspector modules should be generated from the selected route and loaded from
  stable APIs.
- Auto-generated tags should be used for ObjC classes, categories, entrypoints,
  cache images, kernel collection members, IOKit families, MIG routines, and
  panic-related code paths.

## Public SDK Architecture

### Document Layer

SDK surface:

- `HPDocument` exposes the current disassembled file, current segment, current
  section, current address, selected range, cursor movement, `goto`, navigation
  stack pop, background process waiting/stopping, UI update, and read/write
  operations.
- It also exposes document-level messaging and alerts.

Architecture lesson:

- Hopper has a document controller layer distinct from the disassembled file.
- Background processes and cancellation are document concepts, not only global
  app concepts.
- Damsel's native Workbench should treat a project session as the document
  layer and expose project status, background jobs, cancellation, route stack,
  and user messages through that layer.

### Disassembled File Layer

SDK surface:

- `HPDisassembledFile` owns CPU family/subfamily, CPU definition, syntax index,
  original file path, address width, integer width, file base, entrypoints,
  potential procedures, segments, sections, labels, comments, byte types,
  operand formats, procedures, colors, tags, problems, assembler, NOP data,
  memory reads, undo/redo transactions, type constructors, structure
  definitions, and type database.

Architecture lesson:

- Hopper's public file object is the core mutable analysis database.
- Damsel's `BinaryImage` can remain the parsed static image, but Hopper-grade
  behavior needs a separate `AnalysisProject` or `DisassembledFile` equivalent
  that owns mutable analysis facts and user edits.
- Project snapshots must persist analysis state independently from the raw
  binary.

### Loader Architecture

SDK surface:

- The SDK quick tour says loader plugins detect types from bytes, return
  detected file type objects, expose CPU family/subfamily/address width hints,
  optionally request user loader parameters, and then create segments and
  sections in an empty disassembled file.
- The loader should create mapped data, CPU attributes, entrypoints, potential
  procedures, labels, comments, and progress callbacks.
- The SDK supports debug-data loading against an already loaded file.

Architecture lesson:

- Damsel's Mach-O and shared-cache loaders should be reframed as project source
  loaders that produce segments, sections, address spaces, entrypoints,
  potential procedure seeds, labels, comments, and provenance.
- Sidecars such as dSYM, KDK, `.symbols`, panic logs, and extracted cache images
  should attach through loader-like contracts rather than one-off command
  branches.
- Loading progress must be observable and cancellable.

### CPU And Architecture Semantics Layer

SDK surface:

- `CPUDefinition` exposes CPU families/subfamilies, address width, integer
  width, endianess, syntax variants, CPU modes, register classes, register
  names, stack/frame/program-counter recognition, NOP generation, assembler
  capability, and decompiler capability.
- `CPUContext` exposes address adjustment, CPU mode guessing, CPU mode
  heuristics, next address after failed decode, NOP/padding detection,
  procedure prolog detection, analysis lifecycle hooks, procedure lifecycle
  hooks, basic-block creation hooks, disassembler reset, single instruction
  disassembly, halt detection, procedure-analysis updates, direct-reference
  eligibility, address-load recognition, float recognition, CPU-mode changes
  through branch instructions, switch-statement eligibility, branch analysis,
  instruction-specific analysis, thunk detection, rendering, decompiler
  cooperation, and assembly.

Architecture lesson:

- Capstone-style decoding is far below Hopper's CPU contract.
- Damsel needs an AArch64/AArch64e architecture semantics trait above Capstone:
  branch semantics, call semantics, PAC-aware target normalization, ADRP/LDR
  materialization, switch-table recognition, thunk recognition, stack/frame
  effects, condition flags, prolog/epilog recovery, memory reference policy,
  and rendering spans.
- XNU-style work requires the same layer to understand kernel stubs, branch
  islands, PAC-stripped pointers, chained fixups, and dispatch idioms.

### Procedure Model

SDK surface:

- `HPProcedure` exposes frame facts, saved-register size, frame-pointer offset,
  purged bytes, locals size, basic-block count, first block, block lookup by
  start address, block lookup by instruction address, segment, entrypoint, exit
  blocks, stack-pointer offsets at addresses, local labels, register rename
  overrides, callers, callees, caller/callee procedures, variable names by
  stack displacement, signatures, calling conventions, and tags.
- `HPBasicBlock` exposes address range, parent procedure, index,
  predecessor/successor lists, and tags.

Architecture lesson:

- Hopper procedures are not just ranges. They are the unit where graph,
  decompiler, local variables, call graph, register aliases, signatures, tags,
  and stack facts meet.
- Damsel's current `BasicBlock` and edge summaries need to be lifted into a
  persistent `ProcedureRecord` and `BasicBlockRecord` database.
- Procedure creation must support seeds, conflicts, user hints, cache context,
  ObjC IMP ownership, C++ virtual owner hints, kernel owner hints, and stale
  analysis stages.

### Type And Signature Database

SDK surface:

- `HPDisassembledFile` exposes primitive, pointer, structure, union, enum,
  array, and type-database APIs.
- `HPTypeDesc` exposes size, primitive/pointer/function-pointer/array/struct
  enum classification, names, forward declarations, incomplete types,
  single-line display, fields, enum values, and function signatures.
- `HPMethodSignature` exposes return type, argument list mutation, and calling
  convention linkage through procedure signatures.
- `CommonTypes.h` exposes signature creation reasons including decompilation,
  data flow, calls, embedded database, demangling, and user input.
- `CommonTypes.h` includes calling conventions for C-style conventions,
  AAPCS, AAPCS VFP, Swift, x86_64 variants, and Objective-C runtime-specific
  AArch64 argument shifts.

Architecture lesson:

- Types and signatures are not a future decompiler-only feature. They feed
  operand formatting, variable naming, procedure facts, call effects, ObjC
  dispatch, C++ virtual calls, Swift-adjacent names, and automation.
- Damsel needs a provenance-aware type store with importers for ObjC type
  encodings, C++ RTTI/vtables, dSYM/DWARF, KDK symbols, demangled names, user
  type definitions, and eventually decompiler-inferred types.
- Objective-C calling conventions and arm64e trampolines must be explicit
  records, not hard-coded string formatting.

### Annotation, Mutation, And Transactions

Official facts:

- Hopper lets users rename addresses, comment lines, add inline comments, tag
  addresses/basic blocks/procedures, set colors, edit hex bytes, assemble
  instructions, and NOP regions.
- The FAQ says Hopper can modify binaries via hex editor, assembler, and
  Python script.
- The tutorial says changing bytes can destroy instruction/procedure structure
  and that undo/redo can roll the change back.

SDK surface:

- `HPDisassembledFile` exposes name/comment/type/format/color/tag/procedure
  mutation APIs, assembler and NOP APIs, problem creation, and undo/redo
  transaction APIs.
- `HPDocument` exposes write operations for integers and addresses.

Architecture lesson:

- Damsel should implement analysis-state transactions before binary patching.
- The first transaction scope should cover byte classifications, names,
  comments, tags, colors, manual refs, function bounds, operand formats, user
  hints, and problem suppressions.
- Binary patching should remain a later explicit policy decision.
- User edits must cause targeted invalidation and reanalysis, not silent stale
  views.

### Decompiler Boundary

Official facts:

- Hopper markets C-like pseudocode and says it can identify parameters
  automatically.
- The tutorial warns that pseudocode can be imperfect and can lose unreachable
  code.
- The FAQ says pseudocode is often not directly compilable and that Hopper
  cannot fully recover lost source code.
- Hopper 6.0.0 introduced a new decompiler with improved output.
- Recent changelog entries mention switch/case detection, stack variables,
  variable liveness, AST optimizations, decompiler memory footprint, decompiler
  hangs, aggressively optimized functions, Objective-C ivar display, Swift,
  AArch64 authenticated Objective-C selector trampolines, and large-function
  decompilation.

SDK surface:

- `CPUDefinition` can report whether a CPU plugin embeds a decompiler, but says
  the main AST class is not publicly exposed.
- `CPUContext` exposes `canDecompileProcedure`, prolog/epilog skipping, and a
  `decompileInstructionAtAddress` method returning `ASTNode`, with a note that
  `ASTNode` is not publicly exposed.

Architecture lesson:

- Hopper's public architecture treats decompilation as a late stage that uses
  procedure records, CPU semantics, stack/frame analysis, signatures, type
  facts, and private AST machinery.
- Damsel should build procedure recovery, low-level IR, SSA, stack locals, ABI
  modeling, runtime overlays, and uncertainty reporting before trying to render
  C-like pseudocode.
- A useful intermediate product is a decompiler-lite procedure summary with
  calls, recovered values, branch conditions, stack locals, runtime messages,
  confidence, and unresolved problems.

## Changelog-Driven Feature Evidence

The public changelog is unusually valuable because it shows what Hopper must
keep maintaining after the product is mature.

Recent entries as of 2026-05-26:

- 6.3.0 on 2026-05-19 fixed a Swift demangler crash.
- 6.2.9 on 2026-05-05 fixed the Python interpreter for newer library
  versions.
- 6.2.7 on 2026-04-28 improved loading time for large binaries, decompilation
  for some large functions, and Intel analysis.
- 6.2.6 on 2026-04-07 optimized analysis time for very large binaries.
- 6.2.5 on 2026-03-27 rewrote the MCP server in C++ for macOS/Linux code
  sharing and fixed a minimized-window crash with `set_document`.
- 6.2.4 on 2026-03-23 fixed a decompiler memory leak and an MCP document
  discovery issue.
- 6.1.0 on 2026-02-02 added quick linear segment disassembly and easier dylib
  reading from the driver cache.
- 6.0.12 on 2025-12-09 improved navigation between methods, especially for
  files extracted from the shared dyld cache.
- 6.0.10 on 2025-11-12 fixed a switch/case analysis hang and procedure-list
  selection annoyances.
- 6.0.0 on 2025-09-10 shipped a new decompiler, MCP server, major switch/case
  detection improvements, Swift improvements, AArch64 authenticated
  Objective-C selector trampoline support, macOS 26 design updates, procedure
  address-column controls, font-size shortcuts, ELF loading improvements, and
  many fixes.

Older but still important entries:

- 5.19.0 started a large decompiler refactoring with better stack variables,
  variable rename from right-click, better variable liveness, and AST
  optimizations.
- 5.17.x improved CFG scrolling/zooming, Objective-C header export from dyld
  shared cache, address display, switch hints, Unicode string comments, and
  copy-current-address.
- 5.16.x improved hardened runtime detection and debugger patch application.
- 5.15.x and 5.14.x include dyld shared cache compatibility, BSS modification,
  Python 3.12 compatibility, Objective-C header export, C type parser fixes,
  command palette, ARM construct detection, and decompiler assignment fixes.
- 5.13.x includes Apple AMX instruction disassembly, dead-code removal,
  no-return propagation, C++ mangled signature computation, ARMv8.5 handling,
  BinDiff exporter work, and Python API fixes.
- 5.11.x includes address-list responsiveness, background-analysis deadlock
  fixes, bookmarks in Python, debugger conditional-path highlighting, M68k
  support, Amiga executable support, and pseudo-code export deadlock fixes.
- 5.10.0 added support for iOS and macOS kernel caches that are not encrypted.
- 5.8.x and 5.7.x include Apple Silicon decompilation, dyld cache symbols and
  extraction, Objective-C program decompilation from dyld cache, Swift metadata
  loading, Objective-C section parsing, debugger environment variables, local
  and global file-offset navigation, and AArch64 vector fixes.
- 5.3.x and 5.2.x include no-return detection, switch table entry size hints,
  PAC-related procedure creation fixes, Capstone update, and iOS 15/macOS 12
  chained-fixup pointer support.

Implications for Damsel:

- Large binary performance is a permanent product surface.
- MCP/automation is not a side experiment. Hopper is maintaining it actively.
- Decompiler work is a multi-year subsystem with memory, correctness,
  performance, UI, and architecture-specific bugs.
- Apple-specific support is broad: dyld caches, driver cache, kernel caches,
  Swift, Objective-C, ObjC headers, AArch64e/PAC, chained fixups, and Apple
  Silicon decompilation.
- Procedure list responsiveness and background-analysis deadlocks are
  first-class quality issues, not cleanup.

## Public Workflow Evidence

### Mike Ash Workflow

Public workflow evidence:

- Hopper documents are separate from binaries and can preserve comments and
  annotations.
- Users can mark unexplored bytes as a procedure.
- Hopper shows control-flow arrows in assembly.
- Hopper can switch to CFG view for a procedure.
- Users can decompile a procedure into C-like output.
- Comments and names are central to making analysis readable.
- Double-click navigation and backtracking are core workflows.
- Users can follow data references to selectors and mark bytes as ASCII.
- Hopper has Python scripting and a script editor.

Planning impact:

- Damsel needs saved projects before deep annotation work matters.
- Manual procedure creation and user hints are part of the product, not edge
  cases.
- Data navigation from instruction to pointer to selector/string should be a
  first-class route chain.

### 0xabad1dea Decompiler Workflow

Public workflow evidence:

- Hopper's pseudocode helps beginners and analysts reason about functions, but
  still requires assembly knowledge.
- External library function names act as waypoints.
- Internal functions named by address require analyst naming.
- Variables are synthetic and not always typed well.
- Variadic functions, inlined functions, globals, and calling conventions can
  confuse decompilation.
- Analysts frequently move between decompiler, disassembly, xrefs, and manual
  annotations.

Planning impact:

- Damsel's decompiler-lite output should expose uncertainty and unresolved
  calling-convention facts.
- Function summaries need xrefs, global-variable clues, string references,
  import calls, and manual notes.
- "Pretty pseudocode" without nearby assembly/xref/provenance context would be
  less useful than honest structured summaries.

### Public Python Scripts

Public workflow evidence:

- Public scripts use Hopper's document, segments, selections, comments, names,
  bytes, and script menu.
- `droe/hopper-scripts` includes workflows for shellcode annotation, stack
  strings, Yara match annotation, copying selections as Python/Yara, file
  offsets, import ordinal fixes, byte carving, and selection export.
- A 2025 procedure-name export workflow uses `Document.getCurrentDocument`,
  segment lists, named addresses, and names to export procedure/symbol maps.

Planning impact:

- Automation needs stable document APIs over objects and routes.
- Damsel's first automation layer should start read-mostly: list documents,
  list sources, query names, query strings, query procedures, query xrefs,
  render assembly windows, and export structured summaries.
- Mutation tools should require transaction/provenance support first.

### Public MCP Wrappers

Public workflow evidence:

- Third-party Hopper MCP wrappers expose document selection, segment listing,
  address information, name and string search, disassembly, decompilation, call
  graph queries, demangling, comments, labels, and data-type marking.
- Some wrappers note limitations of public Hopper APIs and work around them.
- Hopper's official changelog now includes an integrated MCP server and
  reliability fixes.

Planning impact:

- Damsel can design a cleaner public automation API from the start because it
  controls the project kernel.
- MCP should not scrape UI strings. It should query typed project objects and
  expose tool annotations distinguishing safe reads from mutations.
- Damsel's Apple advantage should be visible through MCP tools: ObjC message
  graph, dyld-cache provider/importer graph, KDK match, kernel object lookup,
  panic frame resolution, C++ virtual dispatch candidates, and PAC/chained
  fixup explanations.

## Hopper Feature Families Damsel Must Account For

### 1. Document And Project Persistence

Hopper feature family:

- saved documents separate from binaries
- annotations across sessions
- mutable analysis state
- background analysis and waiting state
- project-like selection/navigation state

Damsel implementation need:

- `AnalysisProject`
- `AnalysisSource`
- `ProjectSession`
- project manifest
- analysis database
- background job model
- project open/reopen tests
- path privacy rules

### 2. Loader And Source Graph

Hopper feature family:

- file loader plugins
- segments and sections
- debug data loading
- entrypoints and potential procedures
- loader progress callbacks

Damsel implementation need:

- source registry
- address-space graph
- Mach-O source loader
- dyld shared-cache source loader
- projected image source loader
- kernel collection/fileset source loader
- dSYM/KDK sidecar loader
- panic-log source attachment
- progress and cancellation

### 3. Typed Byte Map

Hopper feature family:

- per-byte classification
- manual make-code/data/string/procedure/undefined operations
- feature-map color navigation
- byte type search
- mutation invalidation

Damsel implementation need:

- range-indexed byte classification table
- inferred and user provenance
- classification transactions
- reanalysis invalidation
- feature map API
- Workbench feature map
- conflicting classification problems

### 4. Architecture Semantics

Hopper feature family:

- CPU plugin contract
- CPU modes and syntax variants
- register model
- branch and switch analysis
- prolog/padding/thunk recognition
- assembler and NOP generation
- decompiler cooperation

Damsel implementation need:

- AArch64/AArch64e semantics trait
- register and pseudo-register model
- branch/call/return effects
- condition flag model
- ADRP/ADD/LDR materialization
- jump-table and switch-table recovery
- PAC-aware call target normalization
- thunk/branch-island detection
- stack/frame effects
- assembly rendering spans
- fixture corpus for each semantic case

### 5. Procedure Database

Hopper feature family:

- procedure creation/removal
- procedure records with frame facts
- basic block lookup
- exits and stack offsets
- local labels
- register aliases
- stack variables
- callers/callees
- signatures and calling conventions
- procedure tags

Damsel implementation need:

- `ProcedureRecord`
- `ProcedureRange`
- `BasicBlockRecord`
- `ControlFlowEdgeRecord`
- `CallSiteRecord`
- `ProcedureLocalLabel`
- `RegisterAlias`
- `StackLocal`
- `ProcedureSignature`
- `ProcedureOwner`
- conflict records
- user hints and reanalysis

### 6. Xrefs And References

Hopper feature family:

- references to and from instructions/data
- manual reference addition
- caller/callee lists
- xref inspector
- call graph automation

Damsel implementation need:

- forward and reverse xref table
- typed reference kinds
- import/string/data/runtime/cache/kernel references
- manual reference facts
- caller/callee index
- graph query API
- xref inspector module

### 7. Search, Lists, And Tags

Hopper feature family:

- symbol list
- string list
- fuzzy symbol search
- tag filtering
- command palette in recent versions
- address copy and go-to dialogs

Damsel implementation need:

- global search index
- routeable result model
- name/string/procedure/runtime/kernel filters
- tag index
- command palette
- copy address/source path commands
- Workbench keyboard navigation

### 8. Inspector And Workbench Surfaces

Hopper feature family:

- contextual inspector
- instruction encoding
- operand format selection
- comments
- colors and tags
- references
- procedure block predecessor/successor lists
- CFG minimap and graph editing

Damsel implementation need:

- inspector module API
- semantic instruction rows
- operand span model
- format override model
- graph model and minimap
- synchronized hex window
- problem drawer
- jobs drawer
- native command routing

### 9. Decompiler And IR

Hopper feature family:

- C-like pseudocode
- Objective-C-like method output
- new v6 decompiler
- stack variable and liveness work
- AST optimizations
- switch/case recovery
- memory footprint optimization
- decompiler hangs/crashes maintenance

Damsel implementation need:

- architecture semantics first
- procedure database first
- low-level IR
- def-use and SSA
- stack/local recovery
- ABI/calling-convention model
- memory/alias policy
- type enrichment
- ObjC/C++/Swift/kernel runtime overlays
- control-structure recovery
- pseudocode with uncertainty

### 10. Apple Runtime Specialization

Hopper feature family:

- Objective-C selectors and messages
- Swift demangling and Swift strings
- ObjC headers from dyld shared cache
- ObjC stubs and retain/release functions
- AArch64 authenticated Objective-C selector trampolines
- dyld shared cache navigation
- driver cache dylib reading
- kernel cache support
- chained fixup support

Damsel implementation need:

- ObjC runtime object graph
- selector/class/category/protocol/method ownership
- message-send callsite facts
- Swift-adjacent demangle/name facts
- C++ ABI and Objective-C++ bridge facts
- dyld cache cross-image graph
- driver-cache and kernel-collection source kinds
- KDK/dSYM profile matching
- panic-log route resolver
- PAC/chained-fixup explanation facts

### 11. Debugger And Static-Dynamic Mapping

Hopper feature family:

- LLDB and GDB support
- local debugger
- legacy remote debugger server
- breakpoint gutter
- debugger panel
- hardened runtime detection
- patch application before launch
- conditional-path highlighting under program counter

Damsel implementation need:

- later LLDB bridge
- static route to runtime address mapping
- slide-aware address-space conversion
- breakpoint records
- PC route highlighting
- local signed fixture executable
- hardened runtime diagnostics
- read-only static workflow first

### 12. Automation, SDK, And Plugins

Hopper feature family:

- Python scripting
- integrated MCP server
- file loader plugins
- CPU plugins
- tool plugins
- script menu
- public SDK archive

Damsel implementation need:

- stable JSON API first
- CLI and UI API share model contracts
- MCP server over project facts
- scriptable query surface
- extension boundary after project kernel stabilizes
- capability-based mutation permissions
- plugin ABI only after internal Rust traits settle

### 13. Performance And Responsiveness

Hopper feature family:

- large binary loading improvements
- very large binary analysis optimizations
- decompiler memory footprint work
- large function decompilation improvements
- address-list responsiveness fixes
- background-analysis deadlock fixes
- MCP reliability fixes
- procedure-list selection fixes

Damsel implementation need:

- large-list paging from the start
- cancellable analysis jobs
- immutable snapshots for UI reads
- bounded memory windows for bytes and rows
- graph and decompiler time budgets
- signpost/metrics instrumentation
- fixture and opt-in real-artifact benchmarks
- Workbench responsiveness tests

## Architecture Damsel Should Build Toward

The minimum Hopper-comparable architecture in Damsel's Apple lane:

```text
ProjectSession
  |
  v
AnalysisProject
  |
  +-- SourceRegistry
  +-- AddressSpaceGraph
  +-- TypedByteMap
  +-- FactStore
  +-- AnnotationTransactionLog
  +-- ProblemStore
  +-- JobStore
  +-- TypeStore
  +-- ProcedureDatabase
  +-- XrefIndex
  +-- RuntimeObjectStore
  +-- ArchitectureSemantics
  +-- IRStore
  +-- SearchIndex
  +-- RouteResolver
```

Workbench projections:

```text
Route + Selection
  |
  +-- AssemblyWindow
  +-- GraphWindow
  +-- HexWindow
  +-- FeatureMap
  +-- PseudocodeOrSummaryWindow
  +-- InspectorModules
  +-- Navigators
  +-- CommandPalette
  +-- JobsAndProblems
```

Automation projections:

```text
Project API
  |
  +-- CLI JSON
  +-- UI HTTP API
  +-- MCP tools
  +-- future scripts
  +-- future plugins
```

The important rule: every projection must read and mutate the same project
objects. Otherwise Damsel will accumulate parallel partial systems that cannot
support Hopper-grade workflows.

## Implications For Current Damsel

Current Damsel already has valuable building blocks:

- `BinaryImage`
- `ObjcMetadata`
- `DyldMetadata`
- `SharedCache`
- `SharedCacheSession`
- `ProjectedBinaryImage`
- `DisassemblyAnalysis`
- `DecodedInstruction`
- `BasicBlock`
- `ControlFlowEdge`
- cache links
- import uses
- ObjC uses
- recovered values
- jump-table uses
- indirect control-flow uses
- CLI JSON output
- local UI server
- native Workbench shell

But Hopper public evidence says the next layer is not just "more fields" on
`DisassemblyAnalysis`. Damsel needs:

- a durable project/document object
- a typed byte map separate from sections
- persistent procedure records separate from per-request analysis output
- architecture semantics above Capstone
- transactions and user edit provenance
- xrefs as an index, not only per-window annotations
- route identity across views
- object-centric inspector modules
- background jobs and cancellation
- performance budgets for giant Apple artifacts
- Apple runtime and kernel object stores that feed search, graph, xrefs, and
  decompiler-lite output

## Ordering Constraints

Do first:

- project/source/address model
- typed byte map
- fact/provenance model
- annotation transactions
- procedure database
- xref index
- route/selection model
- architecture semantics trait
- Workbench store split

Do next:

- graph and hex synchronization
- inspector modules
- search and command palette
- ObjC runtime browser
- dyld shared-cache project workspace
- C++/Objective-C++ object model
- kernel/KDK/panic object model
- performance instrumentation

Do after the above is stable:

- low-level IR
- SSA
- decompiler-lite summaries
- pseudocode
- MCP mutation tools
- LLDB bridge
- plugin ABI
- binary patching

## Completion Test Against Hopper Public Architecture

Damsel should not claim Hopper closeness until a real Apple binary session can
do all of the following:

1. Open a project document and preserve analysis state across app restarts.
2. Show a byte feature map backed by real byte classifications.
3. Navigate symbols, strings, procedures, ObjC objects, cache images, and xrefs
   through one route system.
4. Create or correct a procedure manually and record the edit as a transaction.
5. Show synchronized assembly, graph, hex, and inspector state for the same
   selection.
6. Show procedure details: blocks, edges, exits, stack facts, calls, callers,
   runtime owner, tags, comments, and problems.
7. Resolve ObjC selector/class/method relationships and dyld shared-cache
   cross-image links.
8. Produce honest decompiler-lite facts for a selected procedure with
   confidence and unresolved evidence markers.
9. Keep a large symbol/procedure/string list responsive while background
   analysis runs.
10. Expose read-only automation APIs for documents, sources, procedures, xrefs,
    runtime objects, and rendered windows.

If any of these are missing, Damsel may still be useful, but it is not close to
Hopper's public architecture.

