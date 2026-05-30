# Hopper Public Research Notes

Status: Public-web research notes. This document is deliberately Hopper-specific
and separates explicit public claims from architecture inferred from public UI,
tutorial, scripting, and SDK surfaces.

Purpose: correct the common underestimation of Hopper as "a disassembler with a
CFG and pseudocode." Public materials show a much broader application model:
document state, byte typing, procedure reconstruction, multiple synchronized
representations, annotations, navigation stack, inspector modules, graph
editing, binary modification, scripting, plugins, debugger integration, and now
AI/MCP integration.

Follow-up note: [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)
is the stricter second-pass correction. It adds public evidence from the
MacGeneration author interview, SDK headers, public scripts/plugins, package
observations, and additional workflow writeups, then reconstructs Hopper as a
document-centric analysis database with native app, automation, debugger, and
Apple-runtime overlays.

## Source Priority

Primary public sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page: <https://www.hopperapp.com/download.html>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper Debugger Server page:
  <https://www.hopperapp.com/HopperDebuggerServer/>

Secondary public sources:

- unofficial Hopper SDK mirror:
  <https://github.com/makigumo/HopperSDK-v4>
- raw public SDK headers in that mirror, including `HPDocument`,
  `HPDisassembledFile`, `HPSegment`, `HPProcedure`, `HPBasicBlock`, `HPASMLine`,
  and `HopperPlugin`
- public Hopper plugin examples, such as Swift demangling and custom CPU/loader
  plugins
- public Python API stubs and scripts
- public third-party writeups that show real user workflows
- public third-party MCP wrappers around Hopper scripting

Important caveat:

- Secondary SDK mirrors and community scripts are not official Hopper product
  documentation. They are still useful public evidence of the object model and
  automation/plugin expectations, but Damsel docs should label architectural
  conclusions from them as inference.

## Hopper Feature Surface From Official Public Pages

### Current Public Version And Release Direction

Public changelog facts as of 2026-05-26:

- The latest public changelog entry observed during this research is Hopper
  6.3.0, dated 2026-05-19, with a Swift demangler crash fix.
- Hopper 6.2.9, dated 2026-05-05, fixed the Python interpreter for newer
  library versions.
- Hopper 6.2.7 and 6.2.6, dated 2026-04-28 and 2026-04-07, improved large
  binary loading and analysis time.
- Hopper 6.2.5 and 6.2.0, dated 2026-03-27 and 2026-03-10, improved MCP
  server reliability and reduced memory footprint for large databases.
- Hopper 6.1.0, dated 2026-02-02, added a quick linear segment disassembly
  option and an option to read dylibs from the driver cache.
- Hopper 6.0.12, dated 2025-12-09, improved method navigation for files
  extracted from the dyld shared cache.
- Hopper 6.0.0, dated 2025-09-10, introduced a new decompiler, MCP server,
  improved switch/case detection, better Swift support, support for
  authenticated Objective-C selector trampolines, macOS 26 design changes, and
  additional UI customization.

Damsel implication:

- Hopper is not a static target frozen at its tutorial feature set. Recent
  public work clusters around large-binary responsiveness, memory footprint,
  MCP/AI, decompiler quality, Swift/ObjC, switch/case recovery, dyld-cache
  navigation, and Apple-specific AArch64e constructs.
- Damsel's plan must reserve explicit engineering lanes for performance,
  responsiveness, and product fit-and-finish. Treating those as cleanup after
  feature work would miss how Hopper appears to evolve publicly.
- Driver-cache and authenticated selector-trampoline support are especially
  relevant to Damsel's Apple systems niche.

### Native Mac Application

Public claim:

- Hopper positions itself as a fast native macOS disassembler with a polished
  Cocoa interface, keyboard shortcuts, and native macOS integration.

Damsel implication:

- Workbench cannot feel like a thin diagnostic panel forever. It needs native
  document behavior, menus, keyboard-first navigation, command palette, state
  restoration, autosave, contextual inspectors, and smooth large-list behavior.

### Automatic Analysis And Byte Typing

Tutorial facts:

- Hopper transforms binary bytes into human-readable structure.
- It performs automatic analysis as soon as a file is loaded.
- It associates types to bytes.
- Public tutorial byte categories include data, ASCII strings, code,
  procedures, and undefined regions.
- Users can manually change types from toolbar buttons or keyboard shortcuts.
- Data typing can be cycled through byte, 16-bit, 32-bit, and other
  interpretations.
- Undo/redo applies to these transformations.

Damsel implication:

- Damsel needs a byte-classification database, not only section metadata.
- Manual "make code/data/string/procedure" operations are part of competitive
  UX, even if binary mutation stays out of scope.
- The analysis database should represent both inferred and user-modified byte
  classifications.

### Multiple Representations

Tutorial and homepage facts:

- Hopper displays assembly, CFG, pseudocode, and hex representations.
- Hopper can show assembly, CFG, and pseudocode for a procedure.
- Many representations require procedure construction because procedures carry
  basic blocks, stack usage, and other structural information.

Damsel implication:

- Damsel must make `FunctionRecord` central. Assembly, graph, summary, and
  future pseudocode should all be views of a procedure/function object.
- A one-off disassembly response is not enough for Hopper-level UX.

### Assembly Mode

Tutorial facts:

- Assembly mode is a linear listing.
- It displays instruction addresses, mnemonics, operands, and optionally
  instruction encoding.
- The margin includes colored arrows for jump destinations.
- The margin supports breakpoint and bookmark interactions.

Damsel implication:

- Damsel's assembly view should include address, opcode toggle, labels,
  comments, branch cues, xref badges, bookmark gutter, and debugger gutter
  later.
- Branch arrows are not decorative. They are part of the local flow
  comprehension model.

### Control Flow Graph Mode

Tutorial and homepage facts:

- CFG mode represents a procedure structurally.
- Comments and labels can still be modified in graph mode.
- Cursor movement can traverse basic blocks with arrow keys.
- The inspector contains a CFG minimap for the current procedure.
- The minimap indicates current block and current viewport.
- Nodes can be grouped.
- Nodes can receive custom background colors.
- Node printed text can be edited.
- Homepage mentions interactive CFG and PDF export.

Damsel implication:

- Damsel's graph view should support editable labels/comments/colors, minimap,
  keyboard block navigation, group/collapse later, and export.
- Graph UX needs block-level selection and route identity, not only a rendered
  SVG or static image.

### Pseudocode / Decompiler

Tutorial facts:

- Hopper produces pseudocode intended to be functionally equivalent to CPU
  instructions and more or less like an Objective-C method.
- Hopper explicitly warns pseudocode is imperfect and may lose parts when code
  is considered unreachable/dead.
- The tutorial mentions a control to mitigate dead-code disappearance.

Homepage/FAQ facts:

- Hopper markets decompilation to C-like pseudocode.
- FAQ says decompiled output is not fully recovered source code and is usually
  not directly compilable.

Damsel implication:

- Damsel should not overpromise decompilation.
- A competitive path is decompiler-lite summaries first, then IR, then
  pseudocode with visible unresolved/dead-code caveats.
- Dead/unreachable-code handling needs UI affordances and analysis controls.

### Hex Mode And Binary Modification

Tutorial/FAQ facts:

- Hopper has a hex mode synchronized with assembly.
- Cursor selection in hex can expand to the bytes of the current instruction.
- Double-clicking bytes can modify them.
- Modifying instruction bytes can destroy the instruction/procedure structure
  and fall back to undefined type.
- Undo/redo can roll back changes.
- Hopper includes an embedded assembler.
- Hopper can NOP a selected region.
- FAQ says Hopper can modify binaries through hex editor, assembler, and Python
  scripts.

Damsel implication:

- Even if Damsel stays read-only first, it should model byte ranges, instruction
  byte spans, and undoable user edits from the start.
- Future patching requires a transaction system, not a direct write helper.
- Hex/data view should be synchronized with assembly before patching exists.

### Segments And Sections Navigation

Tutorial facts:

- Hopper exposes segment and section list navigation.
- It explains executable mapping in memory via segments split into sections.

Damsel implication:

- Damsel already has segments/sections, but Workbench needs a polished segment
  and section navigator with feature-map integration.

### Symbols, Strings, Tags, And Fuzzy Filtering

Tutorial facts:

- Left pane contains symbols and strings.
- Symbol list can be filtered by tags and text.
- Hopper supports naming addresses from the cursor.
- Symbol filtering includes fuzzy-style behavior.
- Tags can apply to addresses, basic blocks, or procedures.
- Hopper automatically generates some tags during loading.
- Entry points receive entrypoint tags.
- Objective-C implementations are tagged with class or category names.
- Strings can be displayed in a string-oriented mode.

Damsel implication:

- Damsel needs a name service, string index, tag index, and fuzzy search.
- ObjC analysis should automatically create class/category tags or equivalent
  structured filters.
- Tags must apply to multiple target kinds, not only addresses.

### Navigation Stack

Tutorial facts:

- Jumping to an address or symbol pushes the old cursor location on a stack.
- Users can pop back with Escape/backspace or navigation toolbar items.

Damsel implication:

- Route-based back/forward navigation is P0, not polish.
- Every jump from xref, graph, search, ObjC, C++, cache, or kernel views should
  use the same navigation stack.

### Navigation Bar / Feature Map

Tutorial facts:

- Hopper has a navigation bar above assembly.
- It color-codes code, procedures, ASCII strings, data, and undefined bytes.
- It indicates current cursor position.

Damsel implication:

- Damsel needs a feature map backed by byte classifications.
- Damsel's feature map should extend Hopper's categories for Apple systems:
  ObjC metadata, C++ metadata, fixups, kernel tables, cache image boundaries,
  and problems.

### Inspector

Tutorial facts:

- Inspector adapts to current cursor context.
- Modules include instruction encoding, operand format, comments, colors/tags,
  references, and procedure information.
- References are bidirectional and can include manual references.
- Procedure inspector shows basic block predecessors and successors.
- Procedure inspector includes a switch/case hint button for indirect jumps.

Homepage fact:

- The inspector panel automatically adapts to context.

Damsel implication:

- Inspector must become a modular context system, not a static dump.
- Manual references, operand formatting, comments, tags, procedure CFG facts,
  and switch/jump-table hints need first-class models.
- Damsel's existing recovered jump-table work should surface as a user-correctable
  switch/case workflow.

### Objective-C And Swift

Homepage/FAQ/tutorial facts:

- Hopper is specialized for macOS/iOS development.
- It decodes mangled Swift names.
- It extracts Objective-C selectors and messages.
- Tutorial says ObjC implementations receive generated tags by class/category.
- FAQ confirms Hopper can disassemble iOS binaries, with a FairPlay DRM caveat.

Damsel implication:

- Damsel's ObjC and Swift surfaces must be app-visible, not only CLI details.
- Damsel can be more specialized than Hopper by deeply linking ObjC selectors,
  class/category ownership, cache images, and message-send candidates.

### Debugger

Homepage/FAQ facts:

- Hopper supports dynamic analysis with LLDB and GDB.
- Users can debug Mac applications while analyzing structure.
- Hopper Debugger Server is legacy for remote debugging; local debugger is now
  preferred for non-remote cases.
- Demo limitations disable debugger backend.

Damsel implication:

- Damsel needs a debugger bridge eventually, but static address-space and
  project models must come first.
- Debugger UI should map live runtime addresses to project routes and preserve
  static provenance.

### Scriptable Python

Homepage/FAQ/community evidence:

- Hopper markets powerful Python scripting.
- FAQ says Python can modify individual bytes.
- Public scripts use a current document, segments, named addresses, comments,
  Yara matches, stack-string annotations, file offsets, import fixes, byte
  carving, and selection export.
- Public type stubs mention `Document`, `GlobalInformation`, segment lists, and
  typed scripting support.
- A 2025 public workflow exports procedure/named-address data by getting the
  current document, iterating segments, and reading names at addresses.
- Public script collections include workflows for shellcode annotation, stack
  string annotation, YARA match annotation, copying selected bytes as Python or
  YARA, recording file offsets as comments, fixing import names, and saving byte
  ranges.

Damsel implication:

- Damsel should plan automation around the same conceptual objects: project,
  source, segment/section, address, name, comment, type, ref, procedure,
  selection, and current route.
- Read-only scripting plus annotation writes should precede patching scripts.
- String search and large-list APIs need caches and paging; public wrappers
  work around slow string APIs with their own caches, which is a warning for
  Damsel's API design.

### Extensible SDK And Plugins

Homepage/FAQ/SDK mirror facts:

- Hopper SDK supports custom file format parsers and CPU architecture backends.
- Public SDK mirror contains plugin protocols, document/file/segment/section,
  procedure/basic-block, ASM line rendering, tags, types, references, assembler,
  and CPU analysis hooks.
- Public plugin examples include Swift demangling and custom CPU/loader plugins.

Damsel implication:

- Long-term extensibility requires stable internal object contracts.
- Damsel should not expose raw parser internals as a plugin API too early.
- The plugin boundary should eventually target loaders, analyzers, renderers,
  importers/exporters, and reports.

### AI / MCP

Homepage/FAQ facts:

- Hopper homepage mentions AI capabilities through an integrated MCP server.
- FAQ says MCP configuration instructions are in Hopper Help.
- A public third-party Hopper MCP wrapper exposes document management, address
  and segment inspection, name/string search, comments, naming, data typing,
  procedure analysis, xrefs, and graph-style queries through tool calls.

Damsel implication:

- Damsel should eventually expose local analysis facts through a structured local
  tool/API surface if AI integration is desired.
- This should be local, provenance-aware, and privacy-preserving by default.
- MCP must use the same route/object/annotation contracts as the app and CLI,
  otherwise AI tooling will fork the product's data model.

## Inferred Hopper Architecture From Public SDK/API Surface

This section is inference from public headers and scripts, not a claim about
private Hopper internals.

### Document-Centric Model

Public API concepts:

- current document
- current segment
- current section
- current address and selected range
- cursor movement with or without navigation stack
- background process state
- waiting/cancellation UI
- logging and alerts
- read/write by virtual address
- UI update

Inference:

- Hopper's core UX is document-centric. The document mediates current location,
  background analysis state, file access, and UI synchronization.

Damsel target:

- `AnalysisProject` plus `WorkbenchRoute` should play this role.

### Disassembled File Model

Public API concepts:

- file UUID
- CPU family/subfamily
- CPU definition
- address-space width
- integer width
- file base address
- entrypoints and potential procedures
- segments and sections
- labels/names
- comments and inline comments
- byte type query and mutation
- operand formatting
- procedure creation/removal/recursive construction
- colors and tags
- problem list
- assembler and NOP data
- reading file values
- undo/redo transactions
- type database and structured types

Inference:

- Hopper stores a rich mutable analysis database behind each loaded file.

Damsel target:

- Damsel needs an equivalent project analysis database, not only immutable parse
  results.

### Segment And Section Model

Public API concepts:

- segment name
- file offset/length
- readable/writable/executable flags
- mapped data
- virtual address containment
- sections
- procedures
- references to/from address
- section code/data/cstring flags
- previous/next section

Inference:

- Hopper treats segments and sections as navigable containers with procedure and
  xref ownership.

Damsel target:

- Damsel's `Section` model should be lifted into project range indexes and
  navigator surfaces.

### Procedure And Basic Block Model

Public API concepts:

- procedure frame properties
- basic block count
- first/basic blocks by address/index
- exit blocks
- stack pointer offset
- local labels
- register renaming
- callers and callees
- variable names by stack displacement
- signatures and calling conventions
- tags on procedures
- basic block predecessors and successors
- tags on blocks

Inference:

- Hopper has a durable procedure graph and frame/variable/signature model.

Damsel target:

- Damsel's current `DisassemblyAnalysis` must become persistent `FunctionRecord`,
  `BasicBlockRecord`, `StackFrameRecord`, `CallGraphRecord`, and
  `SignatureRecord` objects.

### ASM Rendering Model

Public API concepts:

- attributed assembly line construction
- mnemonics and jump mnemonics
- registers
- names and local names
- variable names
- comments
- strings/CFStrings
- classes/selectors/protocols
- addresses and formatted numbers
- operands

Inference:

- Hopper's rendered assembly is semantic. It knows operand roles and runtime
  object kinds at rendering time.

Damsel target:

- Assembly rows should not be plain rendered strings only. They need structured
  semantic spans for names, selectors, classes, comments, numbers, registers,
  addresses, and operands.

### CPU Plugin Analysis Hooks

Public SDK snippets include:

- disassemble single instruction
- instruction halt behavior
- procedure analysis hooks
- branch analysis hooks
- switch statement detection
- direct memory reference extraction control

Inference:

- Hopper delegates architecture-specific semantics to CPU modules while the app
  orchestrates procedure and reference analysis.

Damsel target:

- Damsel currently uses Capstone for arm64. If it ever adds extensibility or
  deeper semantics, it needs an architecture semantics layer between raw decode
  and higher-level analysis.

## Hopper Features Damsel Docs Must Track Explicitly

P0 competitive UX:

- document/project model
- automatic analysis
- byte typing
- user type changes
- undo/redo
- symbols and strings pane
- fuzzy filtering
- tags
- navigation stack
- feature map
- contextual inspector
- assembly view with branch cues
- CFG view with minimap
- comments/labels/colors
- refs and reverse refs
- procedure details
- switch/case hints

P1 app power:

- synchronized hex view
- manual references
- local labels
- register renaming
- stack variable names
- procedure signatures
- calling conventions
- type database
- structured types
- problem list
- graph node grouping
- graph export

P2 parity or later:

- pseudocode/decompiler
- debugger bridge
- assembler
- NOP region
- binary byte editing
- Python scripting
- plugin/SDK surface
- MCP/AI integration

Apple niche overlays:

- dyld shared-cache browser
- cache image projection and cross-image jumps
- ObjC class/category/selector/message browser
- Swift demangle and symbol navigation
- C++/IOKit type/vtable/virtual-call browser
- XNU kernel collection/fileset browser
- KDK/dSYM symbol/type attachment
- panic symbolication
- arm64e pointer-auth facts

## Gaps In The Existing Damsel Docs After This Research

The first Damsel Hopper-gap docs correctly identified many major systems, but
they underweighted several Hopper details:

- byte typing and user type mutation
- feature map/navigation bar
- graph node grouping, coloring, and text editing
- operand display format controls
- local labels
- register renaming
- stack variable naming
- procedure signatures and calling conventions
- type database and structured types
- manual references
- switch/case hint workflow
- problem list
- background process/cancel/wait UI
- scripting as document automation, not only external CLI automation
- SDK/plugin architecture as a first-class long-term boundary
- hex editor synchronization and patch transaction implications
- AI/MCP as a public Hopper feature

These should be reflected in Damsel's product requirements, API contracts,
implementation roadmap, and validation plan.
