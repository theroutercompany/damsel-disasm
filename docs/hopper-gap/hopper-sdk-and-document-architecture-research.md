# Hopper SDK And Document Architecture Research

Status: third public-web and public-SDK research pass.

Research date: 2026-05-26.

Purpose: ground the Hopper-gap plan in the current public Hopper SDK and
related public automation surfaces. Earlier docs correctly identified Hopper as
a document-centric analysis product, but this pass tightens the implementation
picture: which object models Hopper exposes, which analysis contracts plugins
must satisfy, what remains private, and what Damsel must build if it wants a
Hopper-like architecture instead of a pile of one-off views.

## Source Set

Primary official sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page: <https://www.hopperapp.com/download.html>
- Hopper public release API:
  <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper Debugger Server page:
  <https://www.hopperapp.com/HopperDebuggerServer/>
- Hopper SDK 6.3.0 public archive:
  <https://www.hopperapp.com:443/downloader/public/HopperSDK-6.3.0.zip>
- Hopper 2015 MacGeneration interview:
  <https://www.macg.co/os-x/2015/04/interview-hopper-le-logiciel-qui-renverse-la-securite-du-mac-88444>

Public automation and API evidence:

- PyPI type stubs for Hopper's Python API:
  <https://pypi.org/project/types-hopper-disassembler/>
- Stub repository:
  <https://github.com/Fi5t/hopper-disassembler-stubs>
- Stub raw API file:
  <https://raw.githubusercontent.com/Fi5t/hopper-disassembler-stubs/master/src/hopper_api-stubs/__init__.pyi>
- Public Hopper script collection:
  <https://github.com/droe/hopper-scripts>
- Procedure-name export workflow:
  <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- Third-party HopperMCP notes:
  <https://www.mcpnow.io/en/server/hopper-mxiris-reverse-engineering-hoppermcp>

Third-party product package evidence:

- Desktop Insights Hopper package page:
  <https://desktopinsights.vercel.app/apps/hopper-disassembler>
- Mike Ash 2012 Hopper walkthrough:
  <https://www.mikeash.com/pyblog/friday-qa-2012-01-06-the-hopper-disassembler.html>

Local inspection performed:

- Downloaded the official `HopperSDK-6.3.0.zip` to `/private/tmp`.
- Inspected `SDK Documentation.pdf`.
- Inspected public headers under `include/Hopper`.
- Inspected sample loader, CPU, and tool plugin sources included in the SDK.
- Did not inspect Hopper's application binary or private implementation.

## Executive Findings

Hopper's public SDK confirms that Hopper is organized around a mutable document
and disassembled-file model, not only around decoder output.

The SDK exposes:

- plugin identity and versioning
- three plugin families: file loader, CPU definition, tool
- public services for current document access, log messages, line construction,
  loader option UI, and call destination construction
- file loader detection, loading, rebasing, debug-data loading, container
  extraction, and post-extraction setup
- a disassembled file object with CPU metadata, segments, sections, entry
  points, labels, comments, byte types, operand formats, procedures, colors,
  tags, problems, assembler, memory reads, type records, structured type
  definitions, and undo/redo transactions
- a document object with cursor movement, navigation stack operations,
  background process control, wait/cancel UI, user-facing messages, endian-aware
  reads and writes, and UI refresh
- segment and section objects with mapped data, permissions, procedures, and
  xrefs
- procedure objects with basic blocks, stack facts, local labels, register
  renaming, call graph, variable names, signatures, calling conventions, and
  tags
- CPU contracts for instruction decode, CPU modes, prolog detection, padding,
  basic-block analysis, branch analysis, switch hints, instruction-specific
  analysis, thunk detection, rendering, assembly, and decompiler integration
- type records and method signatures as first-class objects

The SDK also confirms a boundary: public plugins can provide file formats, CPU
backends, and menu tools, but they do not get a public AST/decompiler API. The
header explicitly exposes decompiler hooks around private `ASTNode` and
`Decompiler` classes while noting that writing a decompiler is not publicly
available through the SDK.

For Damsel, the implication is direct: a Hopper-competitive plan must include a
project/document kernel, typed byte map, procedure store, semantic instruction
records, transaction log, command surface, and extension boundary. UI polish
without those models would only mimic screenshots.

## Current Public Version Facts

From the official release API inspected on 2026-05-26:

- Mac application: Hopper `6.3.0`, published `2026-05-19 16:58:15`,
  deployment target `macOS 12.4`, universal Apple Silicon and Intel package.
- SDK: Hopper SDK `6.3.0`, published `2026-05-19 16:58:15`, requires Hopper
  `6.3.0` or higher, SHA1 `a99628912f3b3dfa3dbe09ce22f1a741f6b0a3bc`.
- Linux packages: Hopper `6.3.3`, published `2026-05-22 19:20:00`, with DEB,
  RPM, and Arch tar package variants.

From the official changelog:

- `6.3.0` fixes a Swift demangler crash.
- `6.2.9` fixes Python interpreter compatibility with latest library versions.
- `6.2.7` improves large-binary load time, decompilation of large functions,
  and Intel binary analysis.
- `6.2.6` optimizes analysis time for very large binaries.
- `6.2.5` rewrites the MCP server in C++ for shared macOS/Linux code.
- `6.2.0` improves large database loading time, memory footprint, MCP
  reliability, and UI details.
- `6.1.0` adds quick linear segment disassembly and an option to read dylibs
  from the driver cache.
- `6.0.12` improves method navigation for files extracted from the dyld shared
  cache and adds base-address/first-visible-address command-line and AppleScript
  options.
- `6.0.0` introduces the new decompiler, MCP server, improved switch detection,
  Swift string/demangling support, authenticated Objective-C selector
  trampolines, and macOS 26 design work.

Planning impact:

- Hopper is actively investing in scale, document/database loading, MCP,
  Python, Swift, dyld-cache navigation, driver-cache workflows, switch
  analysis, and arm64e runtime details.
- Damsel should treat "large project responsiveness" and "Apple runtime
  overlays" as foundational competitiveness, not late enhancements.

## SDK Archive Shape

The official SDK archive contains:

- `SDK Documentation.pdf`
- `include/Hopper/*.h`
- sample projects:
  - `SampleTool`
  - `AmigaLoader`
  - `M68kCPU`
  - Linux sample build files
- a bundled Capstone copy used by the M68k sample
- Linux patches and setup script for building the Objective-C/GNUstep
  environment Hopper expects on Linux

The SDK documentation says plugins are Cocoa bundles on macOS and GNUstep-based
bundles on Linux. macOS plugins need signing on modern macOS. Plugin install
paths are:

- system scope: `/Library/Application Support/Hopper/PlugIns/v4`
- user scope: `~/Library/Application Support/Hopper/PlugIns/v4`

The `v4` path is an internal architecture path, not the Hopper product version.

Plugin subdirectories and extensions:

- file loaders: `Loaders/*.hopperLoader`
- CPU plugins: `CPUs/*.hopperCPU`
- tools: `Tools/*.hopperTool`

Planning impact:

- Hopper's extension model is locally installed, user/system scoped, and
  bundle-based. Damsel does not need to copy that packaging immediately, but it
  needs equivalent capability boundaries if it wants future extensibility.
- The internal path versioning is a good warning: extension APIs need explicit
  compatibility/version markers independent of app version.

## Common Plugin Contract

Every Hopper plugin conforms to `HopperPlugin`.

Exposed responsibilities:

- initialize with `HPHopperServices`
- return the SDK version used at compile time
- return a stable plugin UUID
- return plugin type
- return name, description, author, copyright, and version
- return command-line identifiers

Planning impact for Damsel:

- Extension identity should be explicit and stable. Plugin output facts should
  record plugin id, version, and analyzer version.
- Command-line identifiers should not be an afterthought. If Damsel gets
  plugin-like analyzers later, their CLI names should be stable and scriptable.
- UUIDs matter because documents may persist references to plugin-provided CPU
  semantics or loader decisions.

## Services Object

`HPHopperServices` exposes:

- Hopper version queries
- current document access
- logging
- UUID construction
- detected-file-type construction
- assembly-line construction helpers
- loader option UI component construction
- call-destination construction

Line construction helpers include:

- blank lines
- file headers
- segment headers
- section headers
- procedure info
- suffix lines
- raw strings
- plain strings
- names at addresses
- local names at addresses
- formatted numbers
- formatted addresses

Planning impact:

- Hopper's renderer is not just string concatenation. The SDK gives plugins
  typed line construction services so rendered assembly can carry address/name
  semantics.
- Damsel's `SemanticSpan` and `InstructionRow` model should be the first-class
  equivalent. Text ranges should point to addresses, names, selectors, registers,
  variables, comments, and xref targets.
- The service object pattern maps cleanly to a future Damsel analyzer context:
  analyzers receive project services, id builders, fact writers, route builders,
  type constructors, logger, and problem reporter.

## File Loader Architecture

Public SDK facts:

- File loaders conform to `FileLoader`.
- Detection receives bytes, length, filename, and path.
- Detection returns one or more detected file type records.
- A loader can report CPU family, CPU subfamily, address width, file
  description, options, low priority, and debug-data status.
- A loader can ask Hopper to present option controls: address field, checkbox,
  CPU selector, string list, and combo box.
- Loading receives the raw bytes, selected detected type, loader options, an
  empty disassembled file object, and a progress callback.
- The loader creates segments and sections, sets CPU family/subfamily/address
  width, marks entry points and potential procedures, labels addresses, writes
  comments, and maps bytes into memory.
- Loader code should not call endian-aware virtual-address read methods
  during load because no CPU plugin is attached yet.
- Loaders can handle debug files, rebased files, container extraction, and
  post-extraction setup.

Architectural meaning:

- Hopper separates "container/file detection" from "analysis of instructions".
- Composite containers are expected. A loader can expose multiple detected
  file choices and extract a selected object.
- A loaded object is not just bytes. It is already a segmented, sectioned,
  address-space-aware file with entry points, procedure seeds, labels, and
  comments.

Damsel requirements:

- `AnalysisSource` must distinguish container sources from extracted/projected
  child sources.
- Loaders must produce address spaces, segment/section maps, entry points,
  procedure seeds, byte classification seeds, names, comments, and problems.
- Dyld shared-cache projection and kernel fileset extraction should be modeled
  as first-class loader/extractor work, not as ad hoc command output.
- Damsel should expose loader options in CLI/API form before trying to expose
  plugin UI. Examples: base address, member/image choice, parse ObjC, parse
  Swift, parse exception data, attach sidecar symbols.

## Disassembled File Model

`HPDisassembledFile` is the most important public SDK clue. It exposes a
Hopper document's analysis substrate.

Major exposed groups:

- file UUID
- CPU family/subfamily and CPU definition
- address-space width and integer width
- 32/64-bit queries
- file base address
- entry points
- segment and section access
- CPU context factory
- all names and named addresses
- nearest-name query
- name mutation with creation reason
- address lookup by name
- comments and inline comments
- byte type query/mutation
- code/procedure query
- CPU mode at address
- search by byte type
- operand format and relative-format mutation
- formatted number rendering
- procedure creation/removal
- colors
- tags
- problem list insertion
- assembler and NOP data generation
- endian-aware integer and address reads
- C string reads
- mapped-data checks
- address-string parsing
- undo/redo transaction begin/end/discard
- primitive, pointer, array, structure, union, and enum type constructors
- structure definition at address
- type database queries
- type import/export

Creation-reason enums matter:

- names distinguish user, script, import, metadata, and automatic origins
- comments distinguish user, script, automatic, and dynamic origins
- signatures distinguish decompilation, data flow, call, database, demangling,
  and user origins

Architectural meaning:

- Hopper's document model stores not only analysis results but also user
  assertions and their origins.
- Names, comments, signatures, and types are provenance-bearing. The public API
  is not purely destructive "set text" state, even when the UI makes editing
  feel simple.
- Byte type, operand formatting, colors, tags, and problems are part of the
  analysis database.
- Undo/redo lives at the analysis-file layer, not only in the UI.

Damsel requirements:

- Add creation reasons and provenance to every mutable user-facing fact.
- Support a transaction log that can be invoked by UI, CLI, scripts, and future
  MCP tools.
- Store operand-format choices, not only raw operand text.
- Store problems next to facts. Problems are navigable project objects.
- Treat type import/export as a product capability, even before a full header
  parser exists.

## Document Model

`HPDocument` exposes the UI-facing document:

- disassembled file access
- current segment and current section
- current address
- selected address range
- cursor movement without navigation stack
- navigation-stack goto and pop
- background-process state
- request stop for background process
- wait/cancel UI
- user messages and alerts
- endian-aware reads and writes
- global UI update
- window-for-sheet access

Architectural meaning:

- Hopper distinguishes the analysis object from the active UI document. The
  active document owns cursor, selection, waiting state, navigation, messages,
  and writes.
- Background analysis and cancellation are public concepts.
- Read/write memory operations are document-level commands because they can
  change analysis state and UI state.

Damsel requirements:

- Workbench needs a `ProjectSession` above `AnalysisProject`: route stack,
  current selection, active source/member/image, visible ranges, jobs, pending
  cancellation, and UI state.
- Background jobs must be first-class API objects, not terminal spinners.
- Every long-running index or analysis operation needs progress, cancellation,
  and partial-result semantics.

## Segment And Section Model

`HPSegment` exposes:

- segment name
- file offset and file length
- flags and readable/writable/executable permissions
- mapped data
- start/end/end-mapped addresses
- sections
- next/previous segment navigation
- section creation
- procedure list and lookup
- references to and from addresses
- manual reference add/remove

`HPSection` exposes:

- section name
- file offset and file length
- flags
- segment relationship
- start/end addresses
- containment checks

Architectural meaning:

- Segment and section objects are navigable analysis objects with references and
  procedures attached.
- Xrefs are address-level document facts, not just a derived printout.

Damsel requirements:

- Store xrefs at the project level, with source address, target address/object,
  kind, provenance, and confidence.
- Allow manual xrefs later through transactions.
- Preserve source/container offsets alongside VM addresses.
- Support next/previous segment/section navigation in Workbench and CLI.

## Procedure Model

`HPProcedure` exposes:

- frame facts: base-pointer frame, saved-register size, frame-pointer offset,
  purged bytes, locals size
- basic block count and lookup
- entry point
- exit blocks
- stack pointer offset at address
- local labels
- register renaming
- callers and callees
- caller and callee procedures
- stack variable names
- method signature and signature propagation
- calling convention
- tags

Architectural meaning:

- A procedure is a structured object with CFG, stack, local naming, call graph,
  signature, calling convention, and user annotations.
- Hopper exposes register renaming and local labels because real reverse
  engineering depends on procedure-local edits, not only global names.

Damsel requirements:

- `FunctionRecord` must become `ProcedureRecord` or include procedure-local
  tables: blocks, stack slots, local labels, register aliases, call edges,
  variable names, signatures, calling conventions, tags, and user facts.
- Decompiler-lite work should not start until these procedure-local facts are
  durable.
- Objective-C methods and C++ methods should link to procedures, not replace
  them.

## CPU Plugin Architecture

`CPUDefinition` exposes:

- CPU context class and factory
- CPU families and subfamilies
- address and integer width
- endianess
- syntax variants
- CPU modes
- syntax and mode names
- register classes and register counts
- register rendering
- CPU state rendering
- stack pointer/frame pointer/program counter classification
- side-effect register classification
- frame-pointer register name
- NOP bytes
- assembler availability
- decompiler availability

`CPUContext` exposes:

- disassembly structure initialization
- code-address adjustment
- CPU mode inference
- CPU-mode-forcing checks
- failed-decode next address
- NOP detection
- procedure prolog detection
- padding detection
- analysis lifecycle callbacks
- reset disassembler
- single-instruction decode into `DisasmStruct`
- halt detection
- procedure/basic-block analysis callbacks
- direct-memory-reference filtering
- load-address-only detection
- float-manipulation detection
- branch target CPU-mode inference
- next-instruction CPU-mode inference
- switch-statement detection
- branch analysis with next address, in-procedure branches, called addresses,
  and callsites
- instruction-specific analysis
- thunk destination detection
- mnemonic and operand rendering
- decompiler hook
- assembler hook

Architectural meaning:

- Hopper's CPU contract is not just `bytes -> text`. It asks the CPU backend to
  cooperate in procedure discovery, control-flow construction, switch recovery,
  reference extraction, mode tracking, thunk resolution, rendering, assembly,
  and decompilation.
- This is why a generic decoder library is not enough to match Hopper. A
  Hopper-like tool needs an architecture semantics layer.

Damsel requirements:

- Add an `ArchitectureSemantics` trait above Capstone. Initial arm64/arm64e
  implementation should expose:
  - instruction decode result
  - branch kind
  - direct target candidates
  - fallthrough
  - callsite targets
  - return/halt/no-return hints
  - prolog/epilog candidates
  - thunk candidates
  - jump-table/switch candidates
  - ADRP/add/ldr address materialization facts
  - authenticated branch and pointer-auth facts where recoverable
  - ObjC selector trampoline recognition
  - display operands with semantic spans
- Keep architecture semantics separate from loader and project storage.
- Treat arm64e/PAC and Apple runtime patterns as core architecture-semantics
  obligations for Damsel's niche.

## DisasmStruct Implications

The SDK's `DisasmStruct` contains the public shape of the instruction semantic
record that Hopper expects CPU plugins to fill.

The important concept is not the exact field layout. The important concept is
that a decoded instruction carries:

- virtual address
- bytes
- CPU/subtype/endian context
- instruction text details
- branch type
- operand count
- operand kinds
- memory access details
- immediate values
- register classes and indexes
- special flags, including CPU-mode effects
- CPU state effects

Damsel requirement:

- `InstructionRecord` should not collapse to mnemonic plus operand strings.
  It needs a structured instruction payload with branch, operand, memory,
  register, immediate, target, and CPU-state facts.
- Renderer output should derive from structured facts, with spans over the
  rendered text.

## Tool Plugin Architecture

Tool plugins expose menu entries by returning a nested menu-description array.
Selectors are resolved at runtime, and sample tools can:

- read the current document
- read current address
- show waiting state
- display alerts
- log messages

Architectural meaning:

- Hopper's "tool" plugin is a UI/document command extension. It is different
  from a CPU or loader extension.
- Tool plugins are not necessarily headless batch analyzers. They are expected
  to interact with the active document and user.

Damsel requirements:

- Separate future extension classes:
  - source loaders
  - analyzers
  - renderers
  - document commands
  - import/export tools
  - automation tools
- Tool commands should run through the same project transaction system as UI
  commands.

## Python API And Script Surface

Public stubs and scripts show a broad Python document API. The exact stubs are
unofficial, but they match public usage patterns and are useful for product
expectations.

Public scripting capabilities visible from stubs and examples include:

- get current document
- enumerate documents
- load/save documents
- query database and executable paths
- rebase a document
- create/delete/rename segments
- enumerate segments and sections
- navigate cursor and selection
- assemble instructions
- read and write bytes/integers
- set byte types and mark code/procedure/data/undefined
- disassemble a whole segment
- set/get names and comments
- get demangled names
- get instructions
- get references from/to addresses
- add/remove references
- enumerate labels and named addresses
- enumerate procedures
- inspect basic blocks
- inspect callers and callees
- decompile procedures to strings
- get procedure signatures
- rename registers
- build and apply tags
- apply colors
- generate Objective-C headers from metadata
- produce a modified executable
- manage bookmarks
- query Hopper version

The calderpwn procedure-name export workflow demonstrates a typical need:
large iOS/macOS and Swift binaries often require exporting names/addresses for
offline comparison, documentation, or grep-driven exploration.

Planning impact:

- Damsel should expose a stable query/mutation API before exposing a scripting
  language. Scripting is only useful if the object model is rich and stable.
- The "export procedure names" use case should be built into CLI/API because it
  is common and simple.
- Damsel's future MCP tools should share the same permission model as scripts,
  especially for mutations.

## MCP And AI Surface

Official Hopper facts:

- The homepage advertises integrated MCP server support for connecting to LLMs.
- The FAQ says MCP configuration instructions live in Hopper Help.
- The changelog shows active MCP work:
  - v6.0.0 added MCP server
  - v6.0.3 reduced tool accesses needed for document queries
  - v6.2.0 improved reliability
  - v6.2.5 rewrote the server in C++ for shared macOS/Linux code
  - v6.2.4 fixed document discovery when extensions were hidden
  - v6.2.1 changed launch behavior so Hopper waits for a real request

Third-party MCP evidence suggests the same product pressure: people want tool
access to assembly, pseudocode, names, addresses, comments, and document
selection.

Planning impact:

- MCP is no longer speculative in this competitive space. It is a live Hopper
  product surface.
- Damsel should design MCP after stable project APIs exist, not before.
- MCP tools should be narrow, inspectable, and route/fact-oriented:
  `open_project`, `search_symbols`, `get_function`, `get_xrefs`,
  `get_runtime_object`, `add_comment`, `rename_object`, `explain_address`.
- Mutation tools need transactions, provenance, undo, and audit logs.

## Debugger Surface

Official Hopper facts:

- The homepage says Hopper supports LLDB and GDB dynamic analysis for Mac apps.
- The tutorial shows breakpoints in the disassembly margin.
- The Debugger Server page says the separate Hopper Debugger Server is now
  legacy unless remote debugging is needed; local debugger should be used for
  same-machine debugging.
- The Debugger Server requires Xcode on the target machine.
- The changelog includes multiple debugger fixes and attach/hardened-runtime
  improvements.

Planning impact:

- Hopper integrates static route/document state with dynamic state:
  breakpoints, current PC, conditional-jump path highlighting, environment
  variables, attach dialogs, and remote debug support.
- Damsel should defer live debugging until static project routing is strong,
  but static/dynamic mapping should be designed now:
  - file/image VM address
  - slide
  - runtime load address
  - debug target id
  - breakpoint target
  - current frame route
  - symbolication profile

## Native Package Evidence

Desktop Insights reports Hopper as a Swift macOS app, universal Apple Silicon
and Intel, code-signed, notarized, hardened runtime, with Sparkle, Protocol
Buffers, Expat, and Hopper components visible in the package.

This is third-party package evidence, not official architecture documentation.
Still, it aligns with the official changelog:

- Sparkle update in the changelog
- MCP server and protobuf-like protocol needs
- native macOS packaging and signing requirements
- debugger-related entitlements in package analysis

Damsel planning impact:

- Native app packaging, signing, entitlement planning, crash logging, update
  strategy, and local server lifecycle are not "polish only" if Damsel wants a
  real Hopper-like app.

## What Hopper Exposes Versus Keeps Private

Publicly exposed through SDK and scripts:

- file loading
- CPU decoding/analysis/rendering
- tool menu commands
- document/segment/section/procedure access
- byte type mutations
- labels, comments, tags, colors
- xrefs
- procedures and basic blocks
- signatures and calling conventions
- type database construction/import/export
- assembler hooks
- background analysis state
- decompile-to-string from Python

Not publicly exposed as a stable SDK surface:

- decompiler AST classes
- decompiler implementation internals
- full analysis scheduler internals
- private Hopper database schema
- internal UI architecture
- official MCP tool schema through public web pages
- exact Swift/ObjC metadata parser internals
- exact dyld/kernel loader internals

Planning impact:

- Damsel does not need to mimic private internals. It needs to mimic the public
  architectural affordances users depend on: document state, facts, procedures,
  navigation, mutation, automation, and Apple-runtime overlays.
- Damsel can choose a different storage and UI architecture as long as the
  surface behavior is equally fluid and explainable.

## Damsel Architectural Requirements Derived From SDK Research

### Requirement 1: Project Kernel

Implement `AnalysisProject` as the durable center:

- project id
- sources
- child sources/projections/extractions
- address spaces
- analysis runs
- facts
- byte classifications
- names
- xrefs
- procedures
- runtime objects
- types
- annotations
- problems
- jobs
- route stack state

Why:

- Hopper's SDK centers everything around a disassembled file in a document.
- Damsel's current per-command JSON outputs are too transient for Hopper-like
  editing and navigation.

### Requirement 2: Typed Byte Map

Implement a byte/range classification system with:

- kind
- source range
- address range
- file range
- provenance
- confidence
- user override marker
- invalidation links

Kinds should cover Hopper-style basics and Apple-specific overlays:

- unknown
- header
- load command
- code
- procedure
- data
- string
- ObjC metadata
- C++ metadata
- Swift metadata
- chained fixup
- PAC pointer
- dyld binding
- debug info
- kernel table
- MIG descriptor
- excluded

Why:

- Hopper's tutorial and SDK both make byte type a central document primitive.

### Requirement 3: Procedure Store

Implement durable procedure records with:

- entry
- ranges
- basic blocks
- exit blocks
- CFG edges
- stack facts
- local labels
- register aliases
- local variables
- callers/callees
- signatures
- calling convention
- tags
- comments
- confidence and problems

Why:

- Hopper exposes these as procedure-level facts. They are not optional if a
  decompiler or graph is expected to feel grounded.

### Requirement 4: Architecture Semantics Layer

Define an internal trait, initially for arm64/arm64e:

- decode instruction
- classify branch
- extract fallthrough
- extract direct targets
- extract callsites
- extract memory references
- detect prolog/epilog
- detect NOP/padding
- detect thunk
- detect switch/jump table
- detect ADRP/add/ldr target construction
- detect no-return candidates
- detect authenticated selector trampolines
- render semantic operands
- assemble later

Why:

- Hopper's CPU plugin contract expects cooperation in procedure and branch
  analysis, not only textual disassembly.

### Requirement 5: Transaction And Provenance System

Every mutable command should produce a transaction:

- name
- origin: UI, CLI, script, MCP, analyzer
- timestamp
- affected facts
- invalidation scope
- undo data when supported

Why:

- Hopper's SDK exposes undo/redo transactions at the file layer and reason
  enums for names/comments/signatures.

### Requirement 6: Object-Routed Automation

Define stable object ids before scripting or MCP:

- source
- address space
- segment
- section
- procedure
- basic block
- instruction
- xref
- selector
- class
- method
- vtable
- kernel member
- type
- problem
- job

Why:

- Hopper's Python and MCP pressure is all about querying and mutating document
  facts. Damsel needs ids and routes to make that safe.

### Requirement 7: Extension Boundary

Design extension classes even if implementation waits:

- loader
- extractor/projector
- analyzer
- architecture semantics
- type importer
- symbolicator
- renderer
- document command
- exporter

Why:

- Hopper's SDK exposes loader, CPU, and tool plugins. Damsel can begin with
  internal traits that later become plugin boundaries.

## API Contract Additions

Suggested Damsel API endpoints:

```text
GET /api/projects/{project_id}/sources
GET /api/projects/{project_id}/sources/{source_id}
GET /api/projects/{project_id}/address-spaces
GET /api/projects/{project_id}/byte-map?space_id=&start=&limit=
POST /api/projects/{project_id}/commands/classify-range
POST /api/projects/{project_id}/commands/make-procedure
POST /api/projects/{project_id}/commands/add-reference
POST /api/projects/{project_id}/commands/rename
POST /api/projects/{project_id}/commands/comment
GET /api/projects/{project_id}/procedures
GET /api/projects/{project_id}/procedures/{procedure_id}
GET /api/projects/{project_id}/procedures/{procedure_id}/blocks
GET /api/projects/{project_id}/procedures/{procedure_id}/callers
GET /api/projects/{project_id}/procedures/{procedure_id}/callees
GET /api/projects/{project_id}/instructions?space_id=&start=&count=
GET /api/projects/{project_id}/xrefs?target_id=
GET /api/projects/{project_id}/types
GET /api/projects/{project_id}/problems
GET /api/projects/{project_id}/jobs
POST /api/projects/{project_id}/jobs/{job_id}/cancel
GET /api/projects/{project_id}/transactions
POST /api/projects/{project_id}/transactions/{transaction_id}/undo
```

Suggested CLI commands:

```text
damsel project open <path>
damsel project status <project>
damsel project byte-map <project> --address ...
damsel project classify <project> --range ... --kind ...
damsel project procedure <project> --address ...
damsel project procedures <project> --filter ...
damsel project xrefs <project> --target ...
damsel project rename <project> --target ... --name ...
damsel project comment <project> --target ... --text ...
damsel project types <project>
damsel project problems <project>
damsel project transactions <project>
```

## UI Contract Additions

Workbench should gain:

- source/project navigator
- segment/section navigator
- procedure navigator
- strings/names navigator
- route stack
- feature map
- semantic assembly rows
- procedure detail inspector
- xref inspector
- byte classification inspector
- problem drawer
- jobs drawer
- command palette
- undo/redo history
- type browser
- ObjC selector/class/method browser
- C++ class/vtable browser
- kernel member/kext browser

The minimum Hopper-like behavior is not visual resemblance. It is that any
object the user sees can be routed to, inspected, renamed/commented/tagged when
allowed, queried for xrefs, and restored later.

## Risks Identified By SDK Research

### Risk: Treating Capstone As The Architecture

Capstone can decode instructions, but Hopper's CPU plugin contract shows that
procedure analysis requires architecture semantics. Damsel must not stop at
mnemonic/operand strings.

Mitigation:

- Add an arm64/arm64e semantics layer before graph and decompiler work.

### Risk: Building UI Without Transactions

If labels, comments, colors, and type changes are stored as Swift UI state,
future CLI/MCP/script mutation will diverge.

Mitigation:

- Put mutation and undo in the project kernel.

### Risk: Automation Before Stable Object Ids

MCP over a transient view API will encourage brittle prompts and accidental
mutations.

Mitigation:

- Build object ids, routes, facts, and provenance first.

### Risk: Decompiler Ambition Too Early

Hopper's public SDK hints that even its plugin API cannot expose full decompiler
construction. Damsel should not promise high-level source until procedure,
type, ABI, and dataflow foundations exist.

Mitigation:

- Deliver decompiler-lite summaries and IR after function/xref/type systems.

### Risk: Ignoring Apple-Specific Loader Complexity

Hopper's recent changelog references dyld shared cache, driver cache, kernel
cache, Swift metadata, ObjC stubs, and authenticated selector trampolines.
Damsel's edge must live exactly in those details.

Mitigation:

- Keep Apple-system artifacts in the core roadmap, not side quests.

## Implementation Sequencing Update

This research strengthens the existing roadmap order:

1. Project/source/address model.
2. Fact/provenance database.
3. Typed byte map.
4. Transactions, annotations, names, comments, tags, colors, bookmarks,
   problems.
5. Architecture semantics trait for arm64/arm64e.
6. Procedure and xref index.
7. Semantic assembly rows and feature map.
8. ObjC runtime object model.
9. Shared-cache workspace and cross-image routes.
10. C++/ObjC++/IOKit object model.
11. Kernel collection/KDK/panic workflows.
12. Scripting/MCP over stable facts.
13. Debugger route bridge.
14. Plugin/extension boundary.
15. Patching and executable production, only after explicit policy decision.

## Acceptance Criteria For This Research Item

This document should be considered useful only if future implementation reviews
can answer these questions from Damsel code:

- Where is the project/document object?
- Where are byte classifications stored?
- Where are names/comments/tags/colors/bookmarks stored?
- Where is the transaction log?
- Where are procedure-local labels, variables, signatures, and register aliases
  stored?
- Where is the arm64/arm64e architecture semantics layer?
- Where are source/container/projection relationships modeled?
- Where are plugin/analyzer/source provenance ids stored?
- Where are background jobs and cancellations modeled?
- Where are MCP/script permissions and mutation audits enforced?

If the answer is "in the view" or "in a command's temporary JSON," Damsel is
still far from Hopper's architecture.
