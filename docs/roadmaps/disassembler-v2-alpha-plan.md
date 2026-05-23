# Disassembler V2 Alpha Plan

Status: Alpha candidate implemented; see
`docs/validation/disassembler-v2-alpha-validation.md` for the current
verification matrix.

This plan turns the completed v1 disassembler into an alpha-testable Apple
runtime exploration layer. V1 remains the compatibility baseline: standalone
Mach-O commands, cache projection, cache-native point queries, deterministic
JSON, and read-only behavior must keep working while the v2 features land.

## Source Grounding

Primary external references used for this plan:

- Apple Mach-O overview: segments contain sections, `__TEXT,__text` stores
  compiled machine code, `__cstring` stores literal strings, and stub/symbol
  pointer sections are dyld-facing runtime structures.
  <https://developer.apple.com/library/archive/documentation/Performance/Conceptual/CodeFootprint/Articles/MachOOverview.html>
- Apple Mach-O ABI reference: Mach-O files are organized as headers, load
  commands, segments/sections, and link-edit tables such as symbol/string
  tables and relocations.
  <https://leopard-adc.pepas.com/documentation/DeveloperTools/Conceptual/MachORuntime/Mach-O_File_Format.pdf>
- LLVM `llvm-objdump`: baseline disassembly UX includes executable-section
  disassembly, symbol-scoped disassembly, start/stop addresses, source
  interleaving, relocations, unwind info, and architecture selection.
  <https://llvm.org/docs/CommandGuide/llvm-objdump.html>
- LLVM `llvm-otool`: Mach-O-specific inspection includes chained fixups,
  dyld info, indirect symbols, Objective-C segments, opcode bytes, text
  sections, and symbolized disassembled operands.
  <https://llvm.org/docs/CommandGuide/llvm-otool.html>
- Capstone: Damsel's decoder dependency exposes instruction details,
  architecture groups, and implicit register semantics that can support
  higher-level analysis.
  <https://github.com/capstone-engine/capstone>
- Arm A64 reference: branch, call, compare-and-branch, test-and-branch,
  pointer-authenticated branch, and load/store families define the control-flow
  and value-flow surface Damsel must model.
  <https://documentation-service.arm.com/static/6245e8f0f7d10f7540e0c054>
- LLVM pointer-auth documentation: arm64e authenticated relocations and
  pointer-auth instructions are object-level concepts, not just printed
  mnemonics.
  <https://llvm.org/docs/PointerAuth.html>
- Swift ABI mangling documentation: Swift symbols use a stable mangled-name
  surface that can be resolved exactly in Mach-O symbol tables before deeper
  metadata recovery is available.
  <https://github.com/swiftlang/swift/blob/main/docs/ABI/Mangling.rst>

## Seven Feature Tracks

### 1. CFG and Trace Layer

Goal: derive basic blocks, direct edges, indirect edges, call edges, returns,
and trace-friendly summaries from the existing instruction stream.

Implementation:

- Add stable core model types for basic blocks and control-flow edges.
- Build analysis from `DisassemblyResultV2` after current annotation/value-flow
  synthesis.
- Keep the first pass local to one decoded window; add cross-function and
  cache-wide graph composition later.
- Expose JSON with stable key order and text output suitable for snapshots.

Acceptance:

- Branch-heavy fixtures produce multiple blocks with direct branch and
  fallthrough edges.
- Indirect dispatch fixtures preserve unresolved and resolved indirect edges.
- Empty or tiny decode windows return an empty graph without panics.

### 2. Cross-Image Cache Navigation

Goal: connect disassembly references to cache provider/importer/reexport
records so users can follow framework boundaries directly from code.

Implementation:

- Resolve import/stub references through existing shared-cache query APIs.
- Add cache-disassembly analysis that emits `provider_image`,
  `provider_install_name`, and reexport-chain metadata where available.
- Preserve image-local analysis when no cache context is available.

Acceptance:

- Synthetic cache fixtures prove import-to-provider and reexport traversal.
- Ambiguous providers stay typed and deterministic.
- Standalone `disasm` output is unchanged unless analysis is requested.

### 3. Basic-Block-Aware Value Flow

Goal: move beyond the current linear register-state pass into block-aware,
path-safe value propagation.

Implementation:

- Keep the current linear recovered-value pass as the v2 seed.
- Add block-local value state with provenance and confidence.
- Merge only equivalent incoming values at joins; otherwise mark unknown.
- Add explicit invalidation for calls, writes, and hard control-flow
  boundaries.

Acceptance:

- Existing jump-table and relative-slot tests continue passing.
- New branch-join fixtures prove no false constant is propagated across
  divergent paths.
- JSON exposes confidence/provenance without removing existing fields.

### 4. Function Summary Mode

Goal: answer "what does this function/window do?" without forcing the user to
read every instruction.

Implementation:

- Summarize calls, imports, data references, recovered constants, ObjC
  selectors/classes, jump tables, block count, edge count, and stop reason.
- Support standalone and projected cache images.
- Text output should be compact; JSON should be stable and complete.

Acceptance:

- `semantic-switch` reports jump-table evidence and branch/data references.
- `import-rich` reports import calls/stubs.
- `objc-sample` reports ObjC selector/class references when present.

### 5. ObjC and Swift-Aware Entrypoints

Goal: navigate from runtime metadata to implementation code.

Implementation:

- Add Objective-C method-target resolution from class/category/protocol method
  records to IMP addresses.
- Add CLI targets such as method owner plus selector.
- Add Swift symbol targets that resolve exact Mach-O mangled names without
  external tools and demangled queries through the local Swift demangler when
  available.
- Anchor Swift entrypoint coverage in a checked-in `swift-sample` fixture with
  stable mangled function symbols.
- Keep deeper Swift metadata recovery as a planned subtrack beyond symbol-table
  entrypoints.

Acceptance:

- ObjC fixtures can disassemble a method implementation by owner/selector.
- Swift exact mangled symbols resolve to address targets without needing
  Xcode, while demangled lookups produce typed toolchain errors if
  `swift-demangle` is unavailable.
- The checked-in Swift fixture proves end-to-end CLI resolution for at least one
  real Swift function symbol.
- Missing method targets produce typed errors, not address-not-mapped guesses.

### 6. Interactive UI Navigation

Goal: make the local UI a real exploration surface for the new graph and
summary data.

Implementation:

- Return analysis from the existing `/api/images/{image_id}/disasm` endpoint
  when requested.
- Add summary, block list, edge list, and clickable target navigation.
- Avoid loading cache-wide indexes in the browser; keep heavy analysis on the
  Rust side.

Acceptance:

- UI tests cover upload, disassembly, analysis payload shape, and target jumps.
- Manual alpha smoke proves no overlapping controls and usable navigation on a
  laptop viewport.

### 7. Real-Cache Validation Matrix

Goal: make alpha confidence evidence-led rather than anecdotal.

Implementation:

- Promote optional real-cache smoke into a documented operator matrix.
- Compare selected outputs against `llvm-objdump`, `llvm-otool`, and known
  Damsel fixture expectations where those tools are available.
- Keep real system cache tests opt-in to avoid CI/environment coupling.

Acceptance:

- `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT` tests report graph/summary smoke
  coverage when configured.
- Release validation documents which fixtures, host classes, and external-tool
  comparisons were actually run.

## Alpha Quality Gates

All seven tracks are alpha-ready only when:

- `cargo test --workspace` passes.
- Fixture drift and parity checks pass.
- JSON contract tests cover every new public object shape.
- Text snapshots cover representative summary/graph output.
- Existing disassembly output remains unchanged unless new flags are used.
- At least one opt-in real-cache smoke run has been performed and documented,
  or the release notes explicitly state that real-cache validation is pending.
- The UI endpoint can return analysis without breaking the current UI flow.

Public beta requires a stricter gate: broad real-cache coverage across at least
two OS cache sets, no known crashing parser cases in the expanded fixture
matrix, and no unresolved correctness issues in branch/indirect-target summary
semantics.
