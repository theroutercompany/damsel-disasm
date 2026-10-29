# Wave 5 — ELF And x86_64 Implementation Spec

Status: Proposed implementation spec for Wave 5 of the
[Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md).

## Why This Wave Exists

Proprietary targets straddle macOS and Linux. Userspace Linux binaries —
daemons, agents, `.so` plugins, static executables — carry the same RE
questions as Mach-O: what are the functions, what do they reference, what
changed between versions. Damsel cannot answer them today: the format model
has a single variant (`MachO`) and the analysis engine is arm64-shaped.

Wave 5 delivers the ELF lane on x86_64 and aarch64 **without** forking the
analysis pipeline. The mechanism is a typed semantic-opcode layer (Wave 5A)
that both ISAs feed, and an ELF loader (Wave 5B) that plugs into the Wave 4
kernel through its `FormatBackend` seam.

## Scope

In scope:

- **5A:** `InstructionSemantics` model; arm64 and x86_64 producers; analysis
  engine migrated from mnemonic-string matching to semantic queries; zero
  behavior change on arm64 (proven by the existing matrices).
- **5B:** `damsel-elf` crate (ELF64 LE/BE, ET_EXEC/ET_DYN, static and
  dynamic, stripped); dynamic-linkage model (DT_NEEDED/SONAME/RPATH,
  versioned symbols, PLT/GOT variants, IFUNC); relocations into the existing
  binding vocabulary; `.eh_frame` FDE function seeds; build-id identity.
- x86_64 decode in `damsel-core` via Capstone; analysis generalization
  (variable-length windows, RIP-relative addressing, x86 jump tables,
  `endbr64` prologues).
- Cross-platform C++ ABI recovery: Itanium demangling, RTTI/typeinfo, vtable
  discovery, virtual-call candidates (consumes hopper-gap M5).
- Linux fixture lane: clang/gcc-built corpus with drift manifests, LLVM
  external comparison extended to ELF, real-ELF evidence sweep.
- `doctor` rows: `elf-analysis`, `elf-fixture-rebuild`.

Out of scope (hard):

- PE/COFF, 32-bit ELF, big-endian beyond parser tolerance, Android/Bionic
  semantics (ART, linker namespaces), Go/Rust runtime metadata recovery
  (Rust symbol demangling is in scope as a name source only), live-process
  analysis, Linux `.ko` kernels (gated stretch, §12), decompilation (Wave 6).

## Consumed Specifications (Normative)

- Wave 4 spec (§3 `FormatBackend` seam) — the ELF lane plugs in there.
- [ADR-0003](../adr/ADR-0003-cross-platform-re-scope.md) — the scope
  decision this wave executes.
- [Processor semantics and Apple ABI spec](../hopper-gap/processor-semantics-apple-abi-implementation-spec.md)
  — the existing thinking on instruction semantics and ABI facts; Wave 5A
  is its cross-ISA generalization.
- [C++ and ObjC++ explorer milestone](../hopper-gap/integration-roadmap.md)
  Milestone 5 — vtable/typeinfo work, generalized to Itanium.
- [Current state audit](current-state-audit.md) §3–4 — the mnemonic
  coupling inventory that 5A must retire.

## Part 5A — Semantic Opcodes

### 5A.1 Motivation And Shape

The engine's 51 mnemonic-taking classifier functions currently match strings
(`is_return_mnemonic`, `is_conditional_branch_mnemonic`, `terminator_kind`,
written-register analysis, value-flow dispatch). They are correct but
arm64-shaped. Rather than teaching each helper x86 spelling, decoders emit a
typed classification the engine consumes. This keeps one engine for two
ISAs and makes the Wave 6 IR lift read typed semantics instead of strings.

```rust
// damsel-core/src/semantics.rs (new)
pub struct InstructionSemantics {
    pub class: OpClass,
    pub flags: SemFlags,               // SetsFlags, IsTerminator, IsPrologue…
    pub operands: SemanticOperands,    // typed, ISA-normalized
}

pub enum OpClass {
    DirectCall     { target: BranchTarget },
    IndirectCall   { via: RegRef, auth: Option<AuthKind> },
    DirectBranch   { target: BranchTarget, condition: Option<Condition> },
    IndirectBranch { via: RegRef },
    Return         { via: Option<RegRef> },
    Trap           { kind: TrapKind },
    Load           { width: u8, dst: RegRef, addr: AddrExpr, signed: bool },
    Store          { width: u8, src: ValRef, addr: AddrExpr },
    AddrMaterialize{ dst: RegRef, expr: AddrExpr },   // adrp/add, lea, adr
    Arith          { op: ArithOp, dst: RegRef, lhs: ValRef, rhs: ValRef },
    Move           { dst: RegRef, src: ValRef },
    Compare        { lhs: ValRef, rhs: ValRef },      // cmp/tst/ccmp, cmp/test
    PtrAuth        { op: PtrAuthOp, signed: ValRef, key: AuthKey, disc: Option<u16> },
    Fixup          { kind: FixupKind },               // chained fixup/binding markers
    Unknown,
}

pub enum BranchTarget { PcRelative(i64), Absolute(u64), PltSlot(u32) }
pub enum AddrExpr { Register(RegRef), PcRelative { base: RegRef, disp: i64 },
                    BaseIndex { base: RegRef, index: RegRef, scale: u8, disp: i64 },
                    Absolute(u64) }
```

The precise migration inventory: 51 production functions in `disasm.rs` take
`mnemonic: &str` as a parameter (52 including one test helper); the full
list is regenerated at the first PR with a multi-line-signature-aware scan
(single-line `grep -n 'mnemonic: &str'` undercounts wrapped signatures).

Notes:

- `RegRef` carries ISA-neutral identity (`x0`; `rdi`), width (`w0` vs `x0`;
  `eax` vs `rax`), and a canonical-form rule so aliasing is explicit (the
  IR strategy's `xN`/`wN` rule generalized).
- `PcRelative` covers RIP-relative x86 addressing and `adr`/`adrp`+`add`
  page math uniformly; the arm64 page-immediate resolution stays in the
  arm64 producer (it already exists as `page_reference_target`).
- Everything else from the existing annotation/recovery machinery (page
  target resolution, stub lookup, jump-table slot evidence) stays where it
  is; 5A changes *how classification is derived*, not what is derived.

### 5A.2 Migration Mechanics

1. Producers: `damsel-core` gains `semantics::arm64::classify(&DecodedInstruction)`
   and `semantics::x86_64::classify(&DecodedInstruction)`. The arm64
   producer is a faithful translation of today's helpers (one-to-one
   mapping table kept in the spec's appendix, §5A.5).
2. `DecodedInstruction` gains `semantics: Option<InstructionSemantics>` (or
   a decode option `emit_semantics`), populated at decode time.
3. Engine migration order (each a separate PR, each keeping every matrix
   green): (a) terminator/edge classification, (b) return/indirect-control
   identification, (c) written-register and call-clobber analysis, (d)
   value-flow dispatch, (e) jump-table detection, (f) stub/import patterns.
4. The mnemonic classifiers remain as `#[cfg(test)]` oracles: a
   differential test asserts semantic classification equals the legacy
   classifier on the full fixture corpus for arm64. The oracles are deleted
   only after Wave 6 consumes semantics exclusively.

### 5A.3 x86_64 Producer Requirements

- Decode: Capstone x86 with `detail(true)`. **Prerequisite (verified
  2026-10-28 at the pinned tag):** add `arch_x86` to the workspace
  `capstone` features — capstone-rs `full` means non-Diet mode, not
  all-architectures, so the x86 backend is not compiled in today's build.
  One workspace line; no new native dependency. First migration PR flips
  it and asserts an x86 decode smoke test.
- Classify: `call rel/rax/[mem]`, `jcc`, `jmp`, `ret`, `syscall`, `int3`,
  `endbr64`, `lea`, `mov`/`movsx`/`movzx`/`movabs`, `cmp`/`test`, PLT/GOT
  patterns, `lock`-prefixed atomics as `Arith`+`SemFlags::Atomic`.
- Windows: variable length; `DisassemblyLimit::Bytes` semantics already
  exist; add alignment tolerance for `nop`/`int3` padding runs.
- Prologue recognition seeds: `endbr64; push rbp; mov rbp, rsp`;
  `push rbx` chains; CET prologue variants; tail-call `jmp` to function.

### 5A.4 Differential Gates

- Classification parity (arm64): semantic class == legacy mnemonic oracle
  over every fixture instruction; any divergence is a reviewed fix or an
  explicit exemption with rationale.
- Reference parity: degraded-engine metrics (`DisassemblyMetrics` counts)
  identical before/after each migration PR.
- Performance: no decode-throughput regression beyond 5 % (decode bench).

### 5A.5 Helper Mapping Appendix (abridged)

| Legacy helper | Semantic query |
| --- | --- |
| `is_return_mnemonic` | `class == Return` |
| `is_conditional_branch_mnemonic` | `DirectBranch{condition: Some(_)}` |
| `is_unconditional_transfer_mnemonic` | `DirectBranch{condition: None} \| IndirectBranch` |
| `terminator_kind` | derives from `OpClass` + `SemFlags::IsTerminator` |
| `is_call_mnemonic` | `DirectCall \| IndirectCall` |
| `indirect_control_register` | `via` on `IndirectCall/IndirectBranch/Return` |
| `written_destination_registers` | `dst` on `Load/Arith/Move/AddrMaterialize` |
| `synthesize_*_value` dispatch | match on `Arith.op` + operand kinds |
| jump-table recognizers | `AddrExpr::BaseIndex` + `Load{width: 4\|8}` chain |

The full table is generated during the first migration PR from a
`grep -n 'mnemonic'` inventory anchored in the audit (§4) and kept in this
document's appendix on landing.

## Part 5B — `damsel-elf`

### 5B.1 Crate And Loading

```text
damsel-elf/
  src/
    lib.rs         // load/load_bytes, ElfImage
    loader.rs      // goblin or object-based parse; identity + layout
    dynamic.rs     // DT_* tables, versions, PLT/GOT layout detection
    relocs.rs      // R_X86_64_*/R_AARCH64_* → ImportBindingRecord vocabulary
    unwind.rs      // .eh_frame/.eh_frame_hdr FDE seeds (+ compact unwind absent)
    cpp.rs         // RTTI/typeinfo/vtable discovery over relocations
    symbols.rs     // symtab/dynsym, GNU hash, version join, demangle hooks
```

- Parse with `object` (already a dep; mirrors the dyld-cache precedent) and
  fall back to `goblin` where `object` is lossy. Single-file ELF objects
  only — executables, PIE binaries, and shared objects are all in scope
  (ADR-0003); fileset/archive containers are out of scope.
- **Segment-only parsing path (research finding, Round 3):** both `object`
  and `goblin` resolve dynamic symbols from `SHT_DYNSYM` sections — and
  `object`'s own source carries a TODO for the no-sections case — so
  hard-stripped binaries (section headers zeroed, a common proprietary
  protection) would parse to nothing. `damsel-elf` therefore owns a
  `DynamicInfo` walker: PT_DYNAMIC → `DT_SYMTAB`/`DT_STRTAB`/
  `DT_GNU_HASH`/`DT_RELA*`, with vaddr→file-offset resolution via
  `PT_LOAD`. The section path remains the fast path when headers exist;
  the walker is the correctness floor and a fixture requirement
  (`elf-nosections`, built by zeroing the section-header table).
- `ElfImage` mirrors `BinaryImage`'s role: segments (PT_LOAD with flags),
  sections, symbols (+ versioned names), imports, relocations, dynamic
  entries, notes.
- Identity: `.note.gnu.build-id` (primary), plus SONAME (`DT_SONAME`) where
  present and `ET_DYN`-vs-`ET_EXEC`; used by the source model and later
  diffing.
- Static stripped: no dynsym; rely on `.eh_frame` FDEs, entry point,
  `.init_array`/`.fini_array`, and heuristics — the Wave 4 seed-fusion
  ordering already ranks these.

### 5B.2 Dynamic Linkage Model

| ELF concept | Wave 4/5 model |
| --- | --- |
| `DT_NEEDED` | source dependency edges (mirrors cache image-deps) |
| `SONAME` | source identity field |
| `RPATH`/`RUNPATH` | provenance-tagged strings + dependency-resolution hints |
| `DT_GNU_HASH`/`DT_HASH` | symbol index acceleration |
| `.gnu.version` / `.gnu.version_r` | `symbol_version: Option<SymbolVersion{ name, hash, is_default }>` on `Import`/`Symbol` |
| `.rela.dyn`/`.rela.plt` | `Relocation` (existing typed kind/encoding ids) + `ImportBindingRecord` |
| `R_X86_64_JUMP_SLOT`/`R_AARCH64_JUMP_SLOT` | stub entries with pointer slots (mirrors `StubEntry`) |
| `R_*_IRELATIVE` (IFUNC) | `ImportBindingKind::Ifunc` + problem "resolver at runtime" |
| `DT_INIT/FINI`, `init_array` | function seeds (exact) |
| PLT forms: `.plt`, `.plt.sec` (full RELRO/IBT), `.plt.got` | `StubKind` variants `PltClassic`, `PltSec`, `PltGot` — the dual `.plt`/`.plt.sec` layout and `-z ibtplt`/`relro` semantics verified from GNU ld docs and LLVM lld source (Research Sources §0) |
| TLS (`__tls_get_addr` thunks) | imports flagged `Tls`, seeds where resolvable |
| `DT_FLAGS_1 PIE` | source flags influencing address-space base semantics |

Rules:

- Zero new address vocabulary: ELF uses `AddressIn<ImageVaddr>`, the
  reserved space from Wave 4. `ET_DYN` loads at 0 in static analysis;
  runtime slide is a later, separate concern.
- A missing `DT_NEEDED` target is a *problem*, not an error (matches the
  Mach-O dangling-import discipline).
- Symbol versioning is preserved on display (`memcpy@GLIBC_2.14`) and in
  JSON as structured fields, never flattened into the name string.

### 5B.3 Relocation → Facts

Relocations of interest (both architectures):

- absolute pointer relocs in `.data.rel.ro`/`.data` → `DataLoad` xrefs into
  the pointed-to object (function/string/symbol) — the ELF analogue of
  chained fixups, and the backbone of vtable and C++ recovery.
- jump-slot relocs → stub entries + import bindings (existing vocabulary).
- `R_*_RELATIVE` → intra-image pointers (function pointers, string refs).
- `R_*_IRELATIVE`/`R_*_GLOB_DAT` → bindings; unresolved ones produce
  problems.
- Every relocated pointer produces a `RelocationEvidence` fact with
  provenance `elf_reloc_v1`, mirroring the Mach-O evidence shape so the
  same inspector and diff code paths apply.

### 5B.4 C++ Recovery (Cross-Platform)

- Itanium demangling as name facts (provenance `demangle_v1`), never
  overwriting exact symbols; Rust v0 demangling for `_ZN…`-shaped Rust
  symbols and `rust_begin_unwind`-style anchors.
- `__cxxabiv1` typeinfo/vtable structures: locate via `.data.rel.ro`
  relocations to `_ZTV*`/`_ZTI*`/`_ZTS*`; emit C++ type facts, vtable slot
  records, and virtual-call candidate xrefs (Wave 4 `VtableSlot` kind).
- Virtual dispatch sites: indirect call through a vtable slot load; emit
  `IndirectTargetReason::FunctionPointer`-style candidates with confidence
  `inferred` and problem records when the target set is ambiguous.
- ObjC++ is not an ELF concern; this part serves Linux C++ and provides the
  generalized model the Apple C++ explorer milestone reuses.

### 5B.5 Integration With The Kernel

`FormatBackend` implementation (`damsel-elf/src/backend.rs`):

- executable ranges: `PT_LOAD` + `PF_X`
- seeds: dynsym/symtab, FDEs, init/fini, entries
- string ranges: `.rodata`, `.data.rel.ro`, string-flagged sections
- import/export tables: dynsym + versions, GNU hash for lookup
- linkage facts: dependencies/versions/bindings

Same pass driver, same fact vocabulary, same CLI commands — `damsel analyze
./libfoo.so` produces the same object shapes as on Mach-O. Doctor rows are
`elf-analysis` (parse/xref/index readiness) and optionally
`elf-fixture-rebuild` (Linux toolchain presence), both enforced in CI where
the capability is claimed.

## 5C. Linux Fixture And Evidence Lane

Checked-in corpus (built by `fixtures/build-elf-fixtures.sh` on Linux,
hash-manifested like the Mach-O lane; parse-only, never executed):

| Fixture | Built with | Exercises |
| --- | --- | --- |
| `elf-hello-stripped` / `-symbolized` | gcc/clang, `-O2` | baseline, symbol vs stripped diff |
| `elf-nosections` | `objcopy`/script zeroing the section-header table | segment-only dynamic parsing (the `object`-TODO gap; Round 3 finding) |
| `elf-cpp-vtables` | g++ | RTTI, vtables, virtual calls, MI |
| `elf-versioned` | custom linker script + `.symver` | versioned imports/exports |
| `elf-ifunc` | glibc `ifunc` | IRELATIVE relocs, resolver problems |
| `elf-static-stripped` | `-static -s` | FDE-only function seeding |
| `elf-pie-fullrelro` | `-Wl,-z,relro,-z,now` | `.plt.sec` stubs, GOT facts |
| `elf-rust` | rustc | Rust demangling, panic anchors |
| `elf-tls` | `__thread` | TLS imports/thunks |
| `elf-malformed-*` | truncated/edited | typed failure taxonomy, mirroring Mach-O |

Validation additions:

- `bin/disasm-external-compare.sh` gains ELF mode: `llvm-objdump -d`,
  `readelf -Ws/-r/-V` comparisons, gated on tool presence (as today).
- `bin/shared-cache-evidence-*.sh` pattern reused for `bin/elf-evidence-*.sh`:
  a real-ELF sweep (distro binaries with known ground truth: e.g. `ls`,
  `bash`, a versioned `libssl`) packaged, self-verified, audited.
- Cross-ISA semantic parity test: fixture sources built for arm64 and
  x86_64 produce equivalent semantic-fact sets (same function count
  ± tolerance, same import/call graph) — a strong guard that one engine
  serves two ISAs.
- Malformed matrix: every new ELF parser exit path has a typed error and a
  no-panic test, mirroring `x86_rejection_matrix.rs` style coverage.

## 5D. Doctor And Docs

- `doctor` rows with honest host rules: `elf-analysis` supported on all
  hosts (pure parsing); `elf-fixture-rebuild` linux-only; bench impact
  documented per the existing support matrix style.
- README support matrix gains an ELF column; `README` "Planned next" list
  updated truthfully at wave exit (repo rule).

## 6. Sequencing

1. 5A.1–5A.2 arm64 producer + differential oracles (pure refactor).
2. 5A.3 x86_64 producer + decode fixtures.
3. 5A.4 engine migration PRs (a–f), each green.
4. 5B.1 loader + identity; 5B.2 dynamic tables; 5B.3 relocations.
5. 5B.5 backend wiring; `analyze` on ELF works end-to-end.
6. 5B.4 C++ recovery.
7. 5C fixtures/evidence lane; 5D doctor/docs.

Parallelizable: 5B loader work can start once ADR-0003 is accepted, without
waiting for 5A (the kernel seam is independent).

## 7. Budgets

| Budget | Target |
| --- | --- |
| ELF parse (30 MB `.so`) | ≤ 500 ms |
| x86_64 decode throughput | ≥ 60 % of arm64 decode throughput |
| Whole-image index on ELF 30 MB | ≤ 30 s (roadmap budget) |
| FDE-based function seed recovery on static stripped | ≥ 85 % of ground truth |
| Semantic parity drift (arm64 fixtures) | 0 classification divergences |

## 8. Risks

| Risk | Mitigation |
| --- | --- |
| x86_64 analysis complexity explodes | semantic layer keeps engine ISA-neutral; x86_64 decompiler coverage is preview-level at Wave 6 exit, required by Wave 7 exit (D-4) |
| Capstone x86 detail gaps (e.g. implicit operands) | classify from detail + `eflags`/write-back info; explicit `Unknown` class with problems, never guesses |
| ELF loader divergence from `object`'s model | own `ElfImage` types; `object` is a parse backend, not the contract |
| Fixture rebuild on Linux adds CI coupling | same hash-manifest pattern as macOS; drift check is portable |
| Scope creep (Android, kernels) | hard non-goals; `.ko` only as gated stretch (§12) |

## 9. Open Questions

1. ~~Whether implicit-operand/flag info from Capstone x86 is sufficient for
   `SemFlags::SetsFlags` without a second semantics table~~ **Answered
   empirically 2026-10-28** (Research Sources §0 Round 3 spike): sufficient
   — `regs_read`/`regs_write` expose `rflags` and `rip` implicit effects;
   the one gap is syscall/trap-class ops (capstone reports no effects for
   `syscall`), which get a small curated intrinsic table. iced-x86's
   per-bit rflags model serves as the differential oracle for the producer
   tests and the flags-metadata source for Wave 6.
2. `object` vs `goblin` split for ELF: start with `object`, add `goblin`
   only where a specific gap is proven.
3. Where demanglers live: `damsel-core` name service vs a thin
   `damsel-demangle` crate shared by both format crates — decide in the
   first C++ PR. Verified candidates as of 2026-10-28: `cpp_demangle` 0.5.1
   (MIT/Apache-2.0, maintained), `rustc-demangle` 0.1.28 (active), new
   `itanium-demangler` 0.1.0, and `swift-demangler` 0.2.0 for the Swift
   lane — evaluate quality in a spike; `cxxfilt` does not exist as a crate.

## 10. Gated Stretch — Linux Kernel Modules (Not This Wave)

A read-only `.ko` lane (sections, symbols, relocations, `.modinfo`, vermagic)
is technically adjacent but expands scope to kernel ABI and module loaders.
Gate: only after Wave 7 exits, driven by real user demand. Recorded here so
it is decided, not accidental.

## 11. References

- [Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md) §5 Wave 5
- [ADR-0003: Cross-Platform RE Scope](../adr/ADR-0003-cross-platform-re-scope.md)
- [Current state audit](current-state-audit.md)
- [Processor semantics spec](../hopper-gap/processor-semantics-apple-abi-implementation-spec.md)
- [Hopper-gap Milestone 5 (C++)](../hopper-gap/integration-roadmap.md)
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md)
- [Research Sources](research-sources.md) §2 (ELF/x86/ABI/format sources)
