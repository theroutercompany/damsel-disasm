# Hopper Feature Parity Matrix

Status: Planning and research. This matrix translates public Hopper features
into Damsel implementation targets. It does not claim the listed Damsel targets
are implemented.

Sources: official Hopper homepage, tutorial, FAQ, download page, debugger
server page, official changelog, and public SDK/API/plugin evidence summarized
in [Hopper Public Research Notes](hopper-public-research.md) and the deeper
second-pass [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md).

## Priority Legend

- P0: required for the first credible Hopper-adjacent Apple systems workflow.
- P1: required for power-user parity once the project model and analysis
  database exist.
- P2: important long-term feature, but only after static analysis foundations
  are stable.
- Edge: a place where Damsel can exceed Hopper in the Apple-focused niche.

## Core Workspace And Analysis

| Hopper Surface | Public Evidence | Damsel Current State | Damsel Target | Required Subsystems | Priority |
| --- | --- | --- | --- | --- | --- |
| Native document workspace | Homepage emphasizes native macOS UI; tutorial describes persistent window panes; SDK exposes document/file objects | Workbench is native, but currently wraps the CLI UI server and transient disassembly responses | Project documents with source registry, route state, autosave, analysis jobs, annotations, and saved UI state | project runtime, persistent database, route model, annotation store, app shell | P0 |
| Automatic analysis on load | Tutorial says analysis starts as soon as a file loads | CLI and Workbench run explicit commands/windows | Open/import starts staged analysis with progress, cancellation, and resumable facts | scheduler, fact store, job API, progress UI | P0 |
| Byte typing | Tutorial describes byte types: data, ASCII, code, procedure, undefined | Damsel has sections, symbols, ObjC/dyld facts, and decoded instructions, but no durable byte-type layer | Store inferred and user-overridden byte classifications across mapped ranges | byte map, provenance, undo transactions, invalidation | P0 |
| Manual type conversion | Tutorial describes toolbar/keyboard conversion and undo/redo | No user mutation model | Make code/data/string/procedure/undefined commands with scoped reanalysis | command model, byte map, analysis invalidation, undo | P1 |
| Background process control | SDK exposes background-process state, wait, and cancellation methods | Workbench has loading booleans and CLI process lifecycle | Analysis jobs with explicit state, cancellation, wait messages, diagnostics, and partial results | job runtime, cancellation tokens, progress events | P0 |
| Problem list | SDK exposes `addProblemAt` | Damsel has errors/diagnostics but no project-level problem navigator | Problems panel for decode failures, ambiguous jumps, bad metadata, missing dSYMs, cache gaps | problem store, analysis provenance, UI panel | P1 |

## Representations

| Hopper Surface | Public Evidence | Damsel Current State | Damsel Target | Required Subsystems | Priority |
| --- | --- | --- | --- | --- | --- |
| Assembly listing | Tutorial describes address, mnemonic, operands, optional encoding, branch arrows | Workbench lists address, opcode, instruction | Semantic assembly rows with labels, comments, operand spans, xref badges, branch cues, byte spans, and optional encoding | semantic renderer, xref index, annotation store, byte spans | P0 |
| Control-flow graph | Tutorial describes editable CFG, minimap, keyboard block movement, grouping, colors, PDF export | Damsel records blocks/edges for decoded windows but has no interactive graph canvas | Procedure graph view backed by full-function records, minimap, selection sync, block comments/labels/colors, export | function database, graph layout service, route model, annotations | P0 |
| Pseudocode/decompiler | Homepage/FAQ market decompiler; tutorial warns about imperfect output and dead-code controls | No decompiler; summaries are possible from current semantic facts | Decompiler-lite summaries first; later IR and pseudocode with uncertainty and dead-code controls | typed IR, dataflow, type DB, UI caveats | P2 |
| Hex view | Tutorial describes synchronized hex mode and instruction-byte selection | CLI can expose bytes; Workbench has no hex/data pane | Read-only synchronized hex/data pane first; later patch transactions | byte provider, range selection sync, route model | P1 |
| Binary patching | FAQ/tutorial describe hex edits, assembler, NOP region, Python byte modification | Out of scope today | Patch transactions only after project, undo, byte typing, and invalidation are stable | patch journal, assembler, provenance, export writer | P2 |

## Navigation, Search, And Inspection

| Hopper Surface | Public Evidence | Damsel Current State | Damsel Target | Required Subsystems | Priority |
| --- | --- | --- | --- | --- | --- |
| Symbols/strings navigator | Tutorial left pane contains symbols and strings with text/tag filters | Workbench has symbol suggestions; no persistent navigator | Left navigator with symbols, functions, strings, classes, selectors, imports, tags, bookmarks | indexed query service, paged UI model | P0 |
| Fuzzy search | Tutorial describes fuzzy symbol filtering | Symbol suggestions exist but are narrow | Global search across symbols, functions, strings, ObjC, C++, xrefs, comments, and cache images | search index, ranking, paged endpoints | P0 |
| Tags and colors | Tutorial describes tags/colors on addresses, blocks, procedures; auto tags include entrypoints and ObjC classes/categories | No persistent tags/colors | Tag/color model with auto and user namespaces | annotation store, query index, UI badges | P1 |
| Navigation stack | Tutorial describes jump stack and back/forward | `jump(to:)` reruns disassembly and lacks route history | Route stack for address, function, block, metadata object, xref, cache image, and source range | route model, project object IDs | P0 |
| Navigation bar / feature map | Tutorial describes color-coded byte map for code/procedure/string/data/undefined | No byte map UI | Feature map for file/cache image with byte classifications plus Apple overlays | byte map, cache projection, UI minimap | P0 |
| Contextual inspector | Tutorial lists instruction, format, comment, colors/tags, references, procedure modules | Workbench inspector is a narrow current-instruction panel | Adaptive inspector modules keyed by selected route/object | object store, inspector contracts, annotation API | P0 |
| References and reverse references | Tutorial inspector shows refs both directions and permits manual references | Current disassembly records some refs for decoded windows | Persistent xref/reverse-xref index with provenance and manual overrides | xref graph, fact store, undo, query endpoints | P0 |
| Switch/case hint workflow | Tutorial exposes switch/case hint button for indirect jumps; changelog shows ongoing switch analysis work | Current analyzer has jump-table facts, but no user hint UI | Indirect-branch problem cards and user hints for base/table/count/stride/sign-extension | jump-table model, hint API, reanalysis | P1 |

## Procedure, Type, And Language Semantics

| Hopper Surface | Public Evidence | Damsel Current State | Damsel Target | Required Subsystems | Priority |
| --- | --- | --- | --- | --- | --- |
| Procedure model | Tutorial says procedures carry basic blocks and stack usage; SDK has `HPProcedure` | Damsel records block/edge facts for decode windows | Stable whole-image function records with blocks, exits, calls, stack facts, and summaries | function discovery, CFG builder, analysis DB | P0 |
| Local labels | SDK exposes procedure-local label APIs | No local label store | Local labels scoped to function and rendered in assembly/graph | annotation store, semantic renderer | P1 |
| Register renaming | SDK exposes register rename APIs | No user register names | Procedure-scoped register aliases for readability | procedure model, renderer, undo | P1 |
| Stack variables | SDK exposes variable naming by stack displacement; changelog shows stack-variable decompiler work | No stack frame model | Stack frame recovery, user names, argument/local rendering | dataflow, ABI model, frame model, type DB | P1 |
| Signatures/calling conventions | SDK exposes signatures and calling conventions; changelog mentions signature editor and C++ mangled-name signatures | No function signature editor | Function signatures, ABI/calling-convention model, propagation controls | type DB, ABI model, annotation store | P1 |
| Type database | SDK exposes primitive, pointer, array, struct, union, enum, import/export type APIs | No durable user type database | Project type database with C/ObjC/C++/Swift-adjacent types and KDK/dSYM imports | type store, parser/importer, symbolication | P1 |
| Operand formatting | Tutorial inspector supports signed/unsigned hex, decimal, octal, address, etc.; SDK exposes operand format APIs | Current instruction strings are preformatted | Per-operand display overrides and persistent render preferences | semantic operands, renderer, annotation store | P1 |
| ObjC and Swift awareness | Homepage highlights Swift demangling and ObjC selectors/messages; changelog includes ObjC/Swift improvements | ObjC metadata extraction is already strong; Swift is limited | ObjC class/selector/message browsers, Swift demangling/type metadata path, cache-aware method navigation | runtime object store, demangler, xref graph | P0/Edge |
| Swift runtime metadata | Changelog references Swift strings/demangling, Swift metadata loading, Swift metadata crash fixes, and Swift demangler fixes | Swift is currently executable-symbol targeting plus demangler shell-out; no `SwiftMetadata` model | Swift sections, name-role facts, modules, nominal types, fields, protocols, conformances, accessors, runtime calls, ObjC bridges, problems, and paged Workbench browser | Swift parser, demangler service, runtime object store, bridge linker, search/index, Workbench browser | Edge/P1 |
| C++ recovery | Changelog mentions C++ mangled-name signature computation | Damsel has no dedicated C++ ABI layer | Itanium ABI demangling, vtables, RTTI, virtual dispatch xrefs, ObjC++ mixed views | C++ model, demangler, data refs | P1 |

## Apple Systems Niche

| Hopper Surface | Public Evidence | Damsel Current State | Damsel Target | Required Subsystems | Priority |
| --- | --- | --- | --- | --- | --- |
| dyld shared cache support | Changelog repeatedly references dyld cache extraction, symbols, compatibility, ObjC headers, method navigation | Damsel already has first-class shared-cache loading, projection, symbolication, dependencies, importers/providers/reexports | Cache-native project with cross-image xrefs, class/selector indexes, provider/importer navigation, cache-wide search | shared-cache source registry, cache graph, projected image facts | Edge/P0 |
| Driver cache/dylib reading | 2026 changelog adds reading dylibs from driver cache | Damsel has shared-cache workflows, but no explicit driver-cache UX | Driver-cache source kind and browser if relevant to macOS/iOS system analysis | source registry, kernel/container adapters | Edge/P1 |
| Kernel caches | Changelog added iOS/macOS kernel cache support in 2023 | Damsel roadmap targets XNU/kernel collections but implementation is incomplete | Kernel collection/source kind with kext bundles, segments, symbolication, IOKit class browser, KDK/dSYM import | kernel runtime, symbolication profiles, C++/IOKit model | Edge/P0 |
| AArch64e/PAC | Changelog mentions PAC procedure creation and authenticated ObjC selector trampolines | Damsel has arm64e/chained-fixup awareness and PAC-related disassembly work | PAC-aware call/branch/value semantics and ObjC selector trampoline recovery | arm64e semantic engine, fixup resolver | Edge/P0 |
| Chained fixups | Changelog mentions iOS 15/macOS 12 chained fixups | Damsel already parses dyld/chained-fixup presence and bindings | Full fixup-aware pointer/data/xref database | fixup graph, byte map, xref index | Edge/P0 |

## Automation, Extensibility, And Dynamic Analysis

| Hopper Surface | Public Evidence | Damsel Current State | Damsel Target | Required Subsystems | Priority |
| --- | --- | --- | --- | --- | --- |
| Python scripting | Homepage/FAQ market Python scripting; third-party scripts use document/segment/name APIs | CLI is scriptable from shell; no in-document scripting model | Read-only project scripting plus controlled annotation writes | stable API, audit log, permission model | P1 |
| SDK/plugins | Homepage says SDK supports custom file format parsers and CPU backends; public SDK exposes plugin protocols | Rust crates are internal APIs | Long-term plugin boundary for loaders/analyzers/renderers/exporters, not raw internals | capability API, versioning, sandbox rules | P2 |
| AI/MCP | Homepage and FAQ mention integrated MCP; changelog shows active MCP work in 2025-2026 | No AI/MCP product surface | MCP-style document API over project facts, with read-only defaults and explicit mutation tools | query API, object IDs, permissions, audit | P2 |
| Debugger | Homepage says LLDB/GDB support; debugger server page describes remote debugging and local debugger | Roadmap mentions eventual debugger; no current bridge | Static-to-dynamic address mapping and local LLDB bridge after project/address-space model is trustworthy | process bridge, rebasing, breakpoint model | P2 |
| Export | Tutorial/homepage mention PDF CFG export; changelog mentions BinDiff export/import | Damsel has CLI output and docs tests | Export assembly, graph, function summaries, xrefs, cache graphs, and later BinExport-like data | export service, stable schemas | P1 |

## What The Matrix Changes

Earlier Damsel planning could plausibly focus on parsers, xrefs, ObjC metadata,
and graph rendering. The public Hopper surface shows several additional
foundational systems that must be designed early:

- byte classification is a central model, not a convenience UI detail
- user edits require undo/redo and reanalysis invalidation even before binary
  patching
- feature-map navigation depends on whole-file byte typing
- procedure-local facts include labels, register aliases, stack variables,
  signatures, calling conventions, tags, and graph edits
- scripting, SDK, and MCP depend on stable document/object APIs rather than CLI
  string output
- responsiveness and memory footprint are continuously maintained product
  requirements, not a one-time optimization pass
