# Current State Audit

Status: Research record. Accurate as of the audit commit; file anchors are
approximate line regions, not exact lines.

Purpose: establish the precise starting line for Waves 4–8. Every wave spec
cites this audit rather than re-deriving facts about the codebase.

## 1. Workspace Shape

| Crate/dir | Role | Size (loc) |
| --- | --- | --- |
| `damsel-core` | typed model + arm64 decoder facade | 4,873 (`model.rs` 4,379, `decode.rs` 450, `lib.rs` 44) |
| `damsel-macho` | Mach-O loading, dyld, ObjC, shared cache, analysis engine | 24,277 (`disasm.rs` 16,125, `shared_cache.rs` 2,466, `dyld.rs` 1,261, `objc.rs` 1,771, `loader.rs` 1,831, `shared_cache_query.rs` 601, `lib.rs` 157, `errors.rs` 65) |
| `damsel-cli` | CLI + renderers + UI server | 12,277 (`output.rs` 8,601, `main.rs` 3,156, `ui.rs` 520) |
| `damsel-app` | native SwiftUI Workbench (SwiftPM) | 6,754 Swift (`WorkbenchStore.swift` 1,076) |
| `fixtures/` | checked-in binary corpus + rebuild/drift scripts | 19 binaries + corpora |
| `bin/` | validation, evidence packaging/audit, release scripts | 11 scripts |
| `docs/` | ADRs, roadmaps, specs, hopper-gap pack | ~80k lines |

## 2. Model Layer (`damsel-core/src/model.rs`)

122 public types. Highlights relevant to the next phase:

- **Container:** `BinaryFormat` (single variant `MachO` — the Wave 5
  extension point), `BinarySource` (file/memory), `SliceInfo`/`SliceDescriptor`
  (universal handling), `Platform`, `Endianness` (already modeled, unused by
  Mach-O arm64).
- **Architecture:** `Arm64`, `Arm64e`, `X86_64` — x86_64 exists for slice
  inventory only; loader rejects thin x86_64 (`UnsupportedThinArchitecture`).
- **Image:** `Segment`, `Section`/`SectionKind`, `Symbol`/`SymbolKind`,
  `Import`, `Relocation` (+ typed kind/encoding ids), `ExportRecord`,
  `ExportFlags`.
- **Runtime metadata:** `DyldMetadata` (dylibs, rpaths, exports trie, classic
  and chained bindings, function starts, stubs/helpers), `ObjcMetadata`
  (classes, categories, protocols, methods, ivars, properties, pointer refs
  with typed provenance `ObjcNameSource`/`ObjcSelectorSource`).
- **Disassembly:** `DecodedInstruction` (mnemonic string, typed `Operand`
  enum, `Reference` enum with 11 variants, `Annotation` enum with 12
  variants, recovered values), `BasicBlock`, `ControlFlowEdge`, and the
  `DisassemblyAnalysis` summary family (targets, indirect control, imports,
  jump tables, ObjC refs, recovered values, cache links) with counts and
  metrics. These types are the raw material Wave 4 promotes into stored
  facts.
- **Shared cache:** container header/mappings/members with a typed
  address vocabulary (`cache_vmaddr`, `image_base_vmaddr`, `image_offset`,
  `member_file_offset`), `CacheImageId`, image/dependency/provider/importer/
  reexport records, `CacheLookupResult` with `SymbolicationMatch`.
- **Compatibility:** the `doctor` capability/policy model with host rules
  and tool requirements — the extension point for `elf-analysis` rows.

Notably absent (Wave 4/5 additions): project/source/address-space/fact
objects, function records, xref records, string records, dynamic-linkage
(ELF) records, symbol-version records, type records.

## 3. Decoder (`damsel-core/src/decode.rs`)

- Single decoder path: **Capstone arm64 with `detail(true)`** (struct is
  historically named `Bad64Decoder`; no bad64 dependency exists in the lock
  file).
- Produces `DecodedInstruction`s: mnemonic string + mapped `Operand`s +
  per-instruction `Reference` derivation (`ReferenceDerivation`) — references
  are already derived at decode time, not only in the analysis engine.
- Workspace `Cargo.toml` builds capstone with `features = ["std", "full",
  "arch_arm64"]`. **Live-verified correction (2026-10-28, at the pinned
  tag):** in capstone-rs, `full` selects non-Diet mode — it does **not**
  enable architectures. Architecture support comes from the `arch_*`
  features, and only `arch_arm64` is enabled today, so the x86 backend is
  **not** compiled in the current build. Enabling x86 is a one-line
  workspace change (`arch_x86`, which maps to `capstone-sys/arch_x86`) —
  still no new native dependency, only a decoder mapper and
  analysis-engine generalization.
- `DisassemblyLimit` (instructions/bytes/unlimited) and
  `DisassemblyOptions` are already ISA-neutral vocabulary.

## 4. Analysis Engine (`damsel-macho/src/disasm.rs`)

The engine is the phase's most refactor-sensitive asset. Structure:

- Entry points: `disassemble`, `disassemble_v2` (limit/stop-reason
  semantics), `analyze_disassembly{,_with_image}`.
- Target resolution: section/symbol/address window with symbol-boundary
  clamping (`resolve_disassembly_target_v2`).
- Pipeline: decode → `annotate_instructions` (symbol/section context,
  stubs, imports, value-flow-lite recovery: `adrp`/`add` page math, `movz`/
  `movk`, register copies, shifts, bitfields, multiply/carries, pair loads,
  scalar memory loads with table-slot evidence, jump-table candidates and
  slot resolution, PAC-aware indirect targets) → `synthesize_analysis_references`
  (call/branch/data/import/ObjC/stub/cache-link references) →
  `build_basic_blocks` + edge synthesis (`analyze_disassembly_impl`) →
  summary aggregation.
- **Mnemonic-string coupling (the 5A migration map):** 353 occurrences of
  mnemonic handling; **51 production functions take `mnemonic: &str`** as
  a classification parameter (52 including one `#[cfg(test)]` helper) —
  `is_return_mnemonic`,
  `is_conditional_branch_mnemonic`, `terminator_kind`,
  `written_destination_registers`, the
  `synthesize_*_value` ladder, jump-table recognizers, stub pattern checks.
  (The `lower` variable is always derived locally inside helpers via
  `mnemonic.to_ascii_lowercase()`, never passed as a parameter.) All are
  arm64-shaped. Each becomes a query over `InstructionSemantics`,
  one helper at a time, behind the existing test matrices
  (`disasm_matrix.rs`, `stub_matrix.rs`, `export_matrix.rs`,
  `import_binding_matrix.rs`, `arm64e_matrix.rs`).
- Value-flow state is per-window (`clear_analysis_state`,
  `store_known_value*`): correct for point queries, and the reason whole-image
  indexing (Wave 4) needs a function-scoped re-run rather than window
  stitching.

## 5. Loader And Metadata (`damsel-macho`)

- `loader.rs`: file/memory load, universal slice selection with arm64e
  subtype ranking, segments/sections/symbols/imports via `object`,
  dysymtab/indirect-symbol attribution (ordinal-backed, duplicate-dylib
  tolerant), stub helper population, import-hint merging with dyld analysis.
- `dyld.rs`: LC_DYLD_INFO bindings/exports trie, **chained fixups**
  (header, imports, per-segment chains), function starts, stub taxonomy.
- `objc.rs`: class/protocol/category/method/ivar/property extraction with
  bounded malformed-input handling and provenance sources.
- `shared_cache.rs` + `shared_cache_query.rs`: real and synthetic cache
  sessions; split-cache member discovery; image inventory; projection into
  per-image analysis (cached per image); exports/lookup/symbolication;
  dependency/provider/importer/reexport relations. Real caches parse via the
  `object` crate's dyld-cache support — a strong precedent for `damsel-elf`
  building on `object`'s ELF support.

## 6. CLI (`damsel-cli`)

- Command families: `doctor`, `info`, `sections`, `symbols`, `imports`,
  `relocs`, `dyld`, `slices`, `objc`, `disasm` (symbol/section/addr/objc
  targets, annotations/references/values/analysis flags), `ui`, and 17
  `cache` subcommands (info, images, image, exports, lookup-address,
  resolve-symbol, image-deps, dependents, symbol-providers,
  symbol-importers, reexports, sections, symbols, imports, dyld, objc,
  disasm).
- Output discipline: every command renders text and JSON with stable key
  ordering (`output.rs`), covered by insta snapshots and JSON contract tests.
  New commands must join this discipline.
- Error discipline: typed `CliRunError` mapping from `MachoError` classes.
- Absent commands (the Wave 4/7 surface): `analyze`, `functions`, `xrefs`,
  `strings`, `search`, `decompile`, `diff`, `recon`, `serve`, batch mode.

## 7. UI Server And Workbench

- `ui.rs`: axum server; endpoints `POST /api/images` (multipart upload,
  128 MB cap), `GET /api/images/{id}/symbols`, `POST /api/images/{id}/disasm`
  (target kinds section/symbol/address; options include annotations, value
  flow, analysis). In-memory `ImageStore`; no persistence.
- Web UI (`damsel-cli/ui/`): vanilla JS three-pane shell over those
  endpoints.
- Native Workbench (`damsel-app`): SwiftUI app that launches
  `damsel-cli ui --no-open --port 0` and drives the same API; virtualized
  instruction surface; command palette with route history; inspector shows
  analysis facts (blocks, edges, targets, indirect controls, jump tables,
  ObjC refs, recovered values, incoming xrefs). Stage for Wave 4/6/7 panes.

## 8. Verification Infrastructure

- Fixture corpus: 19 checked-in binaries with inline hash manifests; rebuild
  is macOS/Xcode-only with portable drift-check mode; export-trie and
  shared-cache synthetic corpora; parity self-tests for the scripts.
- Test matrices: load, decode, disasm, stub, import-binding, export,
  objc, arm64e, x86-rejection, shared-cache load/query/linkage matrices,
  typed contract tests in `damsel-core`, CLI snapshot/JSON/error tests,
  doctor semantics tests, UI API tests.
- External comparison: `bin/disasm-external-compare.sh` against
  `llvm-objdump`/`llvm-otool` where installed.
- Real-cache evidence: smoke script, evidence packaging/verification/audit
  scripts with portable self-tests; GitHub-hosted and Blacksmith macOS
  workflows; the alpha validator sweep wraps everything and can emit/accept
  evidence archives.
- Performance: criterion `decode_bench`; corpus benchmark regression plan
  (hopper-gap) defines tiers and gates this phase enforces.
- Toolchain: Nix flake pinning Rust/LLVM/Ruby/shell tooling; CI workflows
  for verify, release (nightly/beta lanes), and evidence collection.
- Release: private prereleases, macOS arm64 archive, checksums, release
  validation scripts.

## 9. What The Toolchain Cannot Do Today

Consolidated gap list (the phase's contract):

1. No persistence, project, or session memory.
2. No function model; no whole-image analysis; analysis is window-local.
3. No xref table; references exist only inside returned windows.
4. No string inventory; no unified search.
5. One format (Mach-O), one effective ISA (arm64/arm64e).
6. No decompiler, IR, or pseudocode.
7. No diffing, no obfuscation triage, no recon.
8. No automation surface (batch, MCP, scripting).
9. No patching; read-only by design.
10. No type system beyond ObjC metadata encodings and demangle-ready
    symbols.

## 10. Audit Cross-References

- Disassembler v2 plan and validation record:
  [plan](../roadmaps/disassembler-v2-alpha-plan.md),
  [validation](../validation/disassembler-v2-alpha-validation.md).
- Hopper-gap current-code integration map (Apple-track companion):
  [current-code-integration-map.md](../hopper-gap/current-code-integration-map.md).
- Architecture record:
  [apple-runtime-analysis-architecture.md](../architecture/apple-runtime-analysis-architecture.md).
