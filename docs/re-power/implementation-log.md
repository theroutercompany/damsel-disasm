# RE Power Implementation Log

Status: Active work log for the accepted RE Power program
([roadmap](roadmaps/re-power-roadmap.md), decision log D-1…D-5). Not an
implementation claim by itself — every "done" below cites its proof.

Verification rule: a unit is done only when its evidence-matrix rows pass
(`cargo test`, benches, or packaged evidence) on the current tree.

## Wave 4 — Analysis Kernel

### Unit 4.1 — `damsel-project` crate scaffold + SQLite store — DONE (branch `re-power/wave-4-kernel`)

Scope delivered:

- `damsel-project` crate: `ProjectId`, `SCHEMA_VERSION` (1),
  `AnalysisProjectMeta`, `AnalysisSourceKind` (macho_image /
  shared_cache_projected_image / elf_object-reserved / external_symbol_map),
  `ObjectId` (`kind:key`, parse/display), `Confidence` (7-value blueprint
  vocabulary), `FactProvenance`, `FactKind`, `AnalysisFact`,
  `AddressSpaceId` + reserved names (image_vmaddr, cache_vmaddr, elf_vaddr,
  file_offset).
- SQLite store (D-3): bundled rusqlite 0.41.1, WAL, `foreign_keys=ON`,
  `PRAGMA user_version` migrations (v1), sources/objects/facts/events
  schema, single-writer at the type level (`&mut` mutations), typed
  `StoreError` taxonomy (`UnsupportedSchema`, `SourceNotFound`,
  `ProjectNotFound`, `AlreadyExists`), explicit `Create::New/Existing`
  open semantics (no silent overwrites).
- Canonical export (`snapshot.rs`): BTree-ordered JSON, no timestamps,
  events excluded by design; the determinism artifact per D-3.

Proof: `cargo test -p damsel-project` — 6/6 pass (round trip; WAL/FK/
user_version pragmas; canonical fact ordering; idempotent upsert;
byte-deterministic export across independent DBs with shuffled insertion
order; typed refusals incl. future-schema). Full workspace: 33 suites, 0
failures. Evidence rows: 4.1 ✓, 4.3 ✓, 4.4 partial (typed upgrade path
exists; N-1 real-db fixture lands with the first schema bump).

### Unit 4.2 — `BinaryImage → project facts` ingest (`ingest_v1`) — DONE

- `damsel-macho::analysis::ingest`: source registration (macho_image,
  image_vmaddr space), section `byte_class` ranges, rank-1 symbol-table
  seeds, rank-2 function-starts seeds, rank-3 ObjC IMP seeds
  (classes+categories, deduped), entry-point seed, import facts,
  project-level ingest marker. Canonical payload formats documented in the
  module header; `ObjectId` keys may contain `:` (ObjC symbols) — kinds may
  not.
- Proof: `cargo test -p damsel-macho --test analysis_ingest` 3/3
  (sections/symbols/imports truth; ObjC IMP seeds; byte-identical exports
  across two independent ingests) + damsel-project 6/6.

### Unit 4.3 — `damsel analyze` + `damsel project show/export` CLI — DONE

- Commands: `analyze <target> [--refresh]`, `project show <project>`,
  `project export <project> [--out]` (stdout = raw canonical bytes).
  Project dir = `<target>.damsel/` with `project.db`; resolution accepts
  both the dir and adjacent-target forms. Typed errors: `project_exists`
  (suggests --refresh), `project_not_found`, `stale_schema`,
  `store_error`, `ingest_error`. Text + JSON output, sorted keys.
- Proof: `cargo test -p damsel-cli --test project_cli` 3/3 (round trip
  incl. export canonical-order prefix assertion; typed refusal +
  refresh rebuild; unknown-project typed error). Workspace: 35 suites,
  0 failures.

### Unit 4.4a — function summaries query + `functions` command — DONE (read side)

- `damsel_project::function_summaries`: groups `function:<addr>` objects
  with their seed facts; deterministic best-name selection (smallest
  symbol_table payload); sorted by address. CLI `functions <project>
  [--name PAT] [--limit N]` (text/JSON, `count`/`truncated` contract).
  Seed fusion already observable: `_main` =
  entry_point+function_starts+symbol_table.
- Proof: analysis_ingest 4/4 (grouping/sorting/multi-seed), project_cli
  4/4 (JSON listing + name filter). Workspace 36 suites, 0 failures.
- Recovery worker (bounds/blocks/conflicts, rank-6..8 seeds) is Unit 4.4b.

### Unit 4.5 — xref harvest (`xref_v1`) + `xrefs --to/--from` command — DONE

- `damsel_macho::harvest_xrefs`: disassembles each seeded function
  window (seed → next seed / section end), promotes engine references
  into `xref` facts on `instruction:<addr>` objects with canonical
  payloads `<kind>|to=<object>` (call/branch/data/import/stub/…);
  `analyze` now runs it and reports windows/xref counts. CLI `xrefs
  <project> --to|--from <addr> [--kind] [--limit]` with
  nearest-below function attribution (v1) and `count`/`truncated`
  contract. Hex-only address parsing (decimal-parse bug caught in
  smoke, fixed + regression-covered).
- Proof: analysis_ingest 5/5 (round trip incl. name-bearing import
  targets), project_cli 5/5 (direction + kind filter + typed
  invalid_args). Workspace 35 suites, 0 failures.

### Unit 4.6 — strings pass (`strings_v1`) + `strings` command — DONE

- `damsel_macho::harvest_strings`: walks string-classified sections
  (UTF-8 via existing `read_c_string_at_address`, UTF-16LE for
  `__ustring`), min length 4, emits `string:<addr>` objects + `string`
  facts (`utf8|len=N`) and inferred `string_class` facts
  (url/path/format/selector_like). Values stay out of the project
  (privacy design); `strings` CLI reads them best-effort from the
  adjacent target with `values_available` honesty. Two bugs caught by
  smoke + regression tests: cursor-advance address off-by-len, and
  64-byte read overflowing small sections.
- Proof: analysis_ingest 6/6 (address-is-start + value-readback
  regression), project_cli 6/6 (values, name filter). Workspace 35
  suites, 0 failures.

### Unit 4.6a — unified `search` command — DONE

- `search <project> <query> [--kind function|symbol|import|section|string]
  [--limit]`: ranked (exact → prefix → substring, then kind priority
  function>symbol>import>section>string, then name), deterministic,
  paged contract (`count`/`truncated`). String search requires the
  target binary on disk — reported honestly via `strings_searchable`.
- Proof: project_cli 7/7. Workspace 35 suites, 0 failures. Wave 4 CLI
  surface complete: analyze, project show/export, functions, xrefs,
  strings, search.

### Unit 4.4b — recovery worker (`recovery_v1`) + audit gate — DONE

- `damsel_macho::recover_functions`: linear-decode bounds to first hard
  terminator (`return`/`trap`), stop reasons (`return|trap|seed_boundary|
  section_end`), rank-6 call-target seeding with stub-section exclusion
  (stub FP caught in smoke), overlap `function_problem` facts (ambiguous,
  never silent merges). `analyze` reports bounds/rank6/problems;
  `functions` shows bounds. `IngestOptions.include_symbol_seeds` enables
  stripped-mode ingest for auditing.
- Audit gate: `bin/function-recovery-audit.sh` → **PASS on current
  fixtures: truth=3 recovered=3 recall=1.000 fp=0.000** (evidence row
  4.5; thresholds recall≥95% fp≤2%). Note: gate tightens when the
  planned `xref-rich`/`data-in-text` fixtures land (queue 4.7).
- Proof: analysis_ingest 8/8; workspace 35 suites, 0 failures.

### Next units (queue order)

- 4.3 `damsel analyze` CLI — DONE (see above)
- 4.4b recovery worker v1 (bounds/blocks from CFG walk) + audit gate.
- 4.5 xref harvesting — DONE (see above)
- 4.7 budget fixtures (stripped-30mb) + bench wiring.
- 4.7 budgets on `stripped-30mb` fixture (corpus bench rows 4.10).

## Wave 5 — ELF + x86_64 (parallelizable from 5A)

### Unit 5B.1 — `damsel-elf` crate + synthetic ELF fixtures — DONE

- `damsel-elf`: typed ELF64-LE loader on the `object` backend
  (`NotAnElf`/`UnsupportedLayout`/`Malformed` taxonomy; ELF32/BE are typed
  rejections). `ElfImage` model: architecture, PIE flag, build-id (hex),
  SONAME, segments (X/W flags), sections (exec flags + kind), symbols
  (dynsym-first, defined/text/import flags), dynamic entries
  (Needed/Soname/Rpath/Runpath).
- Synthetic fixture builders (`tests/support/synth.rs`): deterministic
  in-process ELF64 bytes — static ET_EXEC (two functions with a real
  call rel32, symtab, GNU build-id note) and shared ET_DYN (PT_LOAD +
  PT_DYNAMIC, .dynsym with exported + undefined import, DT_SONAME/
  DT_NEEDED/DT_STRTAB/DT_SYMTAB). Three fixture bugs caught by the
  probe before landing (sh_link indices, code-size math, containment
  bounds).
- Proof: loader_matrix 4/4 (sections/symbols/build-id truth; dynamic
  linkage incl. import/soname/needed; typed truncation/class/endianness
  failures; file-path parity). Workspace 37 suites, 0 failures.

Next 5B steps: PT_DYNAMIC walker for section-less ELF (the
`elf-noseconds` gap), versioned symbols (.gnu.version), relocations →
binding facts, then the Wave 4 `FormatBackend` seam so `damsel analyze`
works on ELF end to end.

### Unit 5A.1 — `damsel-core::semantics` module (arm64 control-flow) — DONE

- `ControlFlowClass` vocabulary + `semantics::arm64` predicates ported
  verbatim from the engine's private helpers (`is_return_mnemonic`,
  `is_unconditional_transfer_mnemonic`, `terminator_kind` ordering).
  Engine now delegates (`terminator_kind`,
  `is_unconditional_transfer_mnemonic`); behavior identical — the
  existing disasm/stub/matrices are the differential oracle (Wave 5
  spec §5A.2 step 1). Unit tests cover every family + precedence
  (eret→Return, br→IndirectBranch, blr→IndirectCall, b.ne ≠ b).
- Proof: damsel-core lib tests incl. 3 semantics suites; workspace 35
  suites, 0 failures (all engine matrices green post-delegation).

### Unit 5A.2 — remaining pure-mnemonic classifiers delegated — DONE

- `is_return_mnemonic`, `is_terminal_trap_mnemonic`,
  `is_conditional_branch_mnemonic`, `is_call_mnemonic`,
  `is_cfg_boundary_mnemonic`, `indirect_control_kind` now delegate to
  `semantics::arm64` (new: `is_cfg_boundary`, `indirect_control_kind`
  returning &'static str). Behavior identical; matrices green.
- Proof: semantics unit tests extended (boundary set + PAC kinds);
  workspace 35 suites, 0 failures.

### Unit 5A.3 — `arch_x86` feature flip + x86_64 semantics classifier — DONE

- Workspace capstone features now include `arch_x86` (research-verified
  one-line change; `full` ≠ architectures). Arm64 decoder made explicit
  about non-arm64 detail (typed rejection instead of assumed
  exhaustiveness).
- `semantics::x86_64` v1: control-flow classifier with `Transfer`
  (Direct/Indirect) — directness is an operand property on x86; full
  jcc/loop/jcxz family; `syscall` deliberately `Other` (returns);
  `endbr64` `Other` (not a terminator).
- In-workspace spike replication: `damsel-core/tests/x86_decode_smoke`
  decodes the spike window via capstone x86 and asserts implicit
  `rflags` writes (add/cmp) + `rip` reads (lea rip-rel) — proving the
  producer prerequisite inside the workspace, not just standalone.
- Proof: damsel-core lib 6/6, x86_decode_smoke 1/1; workspace 36
  suites, 0 failures.

### Unit 5A.4 — register-write family delegated — DONE

- `destination_register`, `register_operand_name`,
  `written_destination_registers`, `instruction_writes_destination`,
  and the four write-shape tables (store-exclusive status, atomic RMW
  second-operand, casp pair, ldp/ldnp/ldxp pair) ported verbatim to
  `semantics::arm64` (public); engine delegates; legacy copies deleted
  (including the two remaining pair-table call sites in
  `pair_memory_access_width`). Unit tests: write-shape tables, single/
  pair/atomic destination extraction.
- Proof: damsel-core lib 7/7; workspace 36 suites, 0 failures.

### Unit 5A.5 — x86_64 decode mapper — DONE

- `damsel_core::decode_x86_64{,_with_limit}`: Capstone x86 (Mode64,
  detail) mapped onto the shared `DecodedInstruction` vocabulary —
  operands (Register/Immediate/Label/Memory incl. index scale), Call/
  Branch references with operand-derived directness, IndirectCall/
  IndirectBranch via register or memory expression, RIP-relative
  memory → absolute `Data` references (next-ip + disp). Documented
  contract: garbage decodes to an empty stream (capstone skip
  semantics), variable lengths via existing `DisassemblyLimit`.
- Proof: x86_decode_mapper 4/4 (two-function blob with call target at
  0x4F; rip-relative lea → Data 0x223B — test arithmetic bug caught and
  fixed; call rax → IndirectCall via register; limit + garbage
  contract). Workspace 40 suites, 0 failures.

Remaining 5A: value-flow dispatch + jump-table recognizers behind the
oracle; ELF engine integration (FormatBackend analysis path) unlocks
xrefs/recovery/strings for ELF. First step: 5A semantics producer behind differential
oracles (arm64), then `arch_x86` feature flip + x86 producer (spike-proven
sufficient; syscall intrinsic table required).

## Wave 6 — Decompiler

### Unit 6.0 — `damsel-decomp` crate, D0 function summaries — DONE

- `summarize_function`/`summarize_named`: fact-derived D0 reports —
  bound, seeds, call sites (with unresolved flags for
  indirect/register targets), imports, string joins, problems.
  Format-neutral by construction: one summary surface for Mach-O and
  ELF. CLI `summary <project> --function <name|0xaddr>` (text/JSON,
  typed `function_not_found`/`ambiguous_name`).
- Proof: d0_summary 3/3 (Mach-O `_main` incl. `_puts` import + string
  join; ELF `exported_func` call to `0xf7`; typed selector errors);
  project_cli 9/9. Workspace 43 suites, 0 failures.

### Unit 6.1 — D1 low-level IR + arm64 lift — DONE

- `damsel-decomp::ir`: typed, serde-serializable D1 vocabulary
  (IrValue/IrExpr/IrStatement/IrTerminator/IrBlock/IrFunction) per the
  decompiler-ir-strategy shape. `lift_arm64`: strategy floor set
  (mov/movz/movk, add/sub/and/orr/eor/lsl/lsr, ldr/ldur/str/stur,
  bl→resolved Call, terminators ret/trap/b/b.cond via semantics
  predicates); everything else lifts as visible `Unknown` with honest
  `unknown_count` accounting; empty windows lift to zero blocks.
- Proof: d1_lift 3/3 (`_main`: return-terminated block + Call to
  0x100000460; unknown-count invariant; deterministic serialize
  round-trip; empty window). Workspace 44 suites, 0 failures.

### Unit 6.2 — D2 data-flow v1 (versioning, CFG, phi candidates) — DONE

- `damsel-decomp::ssa`: block CFG from terminators (branch/cond/
  window-fallthrough), register def/use counts, and phi candidates
  (multi-def registers read in multi-predecessor blocks). Honest v1:
  linear counting, no dominance frontiers yet — flagged in the module
  docs as the next D2 slice.
- Proof: d2_ssa 3/3 (defs/uses on `_main` + deterministic serialize;
  empty function; hand-built merge IR flags x0 as phi candidate with
  2 preds, x1 excluded). Workspace 45 suites, 0 failures.

### Unit 6.3 — stack-frame facts — DONE

- Lift gained `stp`/`ldp` paired Store/Load semantics (base+d /
  base+d+8; writeback sp-adjust stays explicit via sub/add sp —
  documented). `analyze_stack_frame`: frame size + epilogue restore
  from `sub/add sp,sp,#N`, saved registers (fp/lr/x29/x30/x19–x28)
  from frame-base stores, local slot displacements from other
  frame-base accesses.
- Proof on the REAL `_main` prologue: frame 0x20, restore 0x20, saved
  [fp, lr], local at fp−4 (`stur wzr,[fp,#-4]`). d3_stack_frame 2/2.
  Workspace 46 suites, 0 failures.

### Unit 6.4 — D5 pseudocode renderer v1 + `decompile` CLI — DONE

- `render_pseudocode`: line model with per-line source addresses
  (block start + statement sites), honest goto-form control flow until
  D4, visible `/* unknown */` comments (adrp/cset on `_main`), signed
  negative slots. CLI `decompile <project> --function <name|0x>` with
  frame facts + unknown_count in JSON; address-gutter text mode.
- **First damsel pseudocode** (real `_main`): frame ops, both calls
  resolved, honest unknowns — nothing fabricated.
- Proof: d5_render 2/2, project_cli 10/10. Workspace 47 suites,
  0 failures.

### Unit 6.5 — x86_64 lift + cross-format decompile — DONE

- `lift_x86_64`: floor set (mov family reg↔imm/reg/mem, lea, binops,
  call direct/indirect→Unknown target, ret/jmp/jcc/trap terminators
  via semantics_x86_64); indexed memory stays visible `Unknown` (scale
  models in D2); push/pop visible unknowns v1. CLI `decompile` branches
  on ELF: resolves the function + recovered bound from project facts,
  decodes via the x86_64 mapper, lifts, renders.
- **Cross-format decompilation proven**: `exported_func` on the
  synthetic ELF renders with resolved `sub_f7()` call.
- Proof: d5_x86_lift 2/2, project_cli 11/11. Workspace 48 suites,
  0 failures.

### Unit 6.6 — flag statements (cmp/test) — DONE

- x86 lift models `cmp`/`test` as assignments to a synthetic `flags`
  register (Sub/And) — visible, typed, and the substrate for D4 jcc
  condition resolution (top unknowns from the glibc run eliminated).
- Proof: d5_x86_lift 3/3. Workspace 48 suites, 0 failures.

### Unit 6.7 — D4 structuring v1 — DONE

- `structure.rs`: condition resolution from synthetic `flags`
  assignments (Sub → relational incl. unsigned variants; And → mask
  zero/nonzero; missing flags stay `Unresolved` — never invented);
  diamond detection (cond + two arms reconverging on one merge); the
  renderer emits structured `if (a != b) { … } else { … }` for
  diamonds with arm provenance preserved and gotos dropped inside the
  diamond.
- Proof: d4_structure 5/5 (relational/mask/unresolved resolution,
  diamond detection, structured render without gotos). Workspace 49
  suites, 0 failures.

### Unit 6.8 — loop detection (back edges) — DONE

- `find_loops`: back edges (Branch/CondBranch to earlier-or-equal
  layout, incl. self-loops — boundary bug caught by the test);
  conditions resolved via the flags substrate; renderer annotates
  `goto …; /* loop back */`. while/do-while reconstruction needs
  dominance (later slice, logged).
- Proof: d4_structure 6/6. Workspace 49 suites, 0 failures.

### Unit 6.9 — arm64 condition substrate — DONE

- arm64 lift: `cmp`/`cmn`/`tst` → flags assignments (Sub/Add/And);
  resolver accepts `b.eq/ne/lt/le/gt/ge/lo/ls/hi/hs` (+ cmn equals).
  Regression caught: my refactor briefly made plain `sub` lift as
  Unknown (frame_size regression in the CLI test) — binary-op path
  restored; the suite caught it before commit.
- Proof: d4_structure 8/8. Workspace 49 suites, 0 failures (incl.
  restored Mach-O decompile frame check).

### Unit 6.10 — indexed-memory modeling (scale) — DONE

- `IrExpr::IndexedAddr { base, index, scale, displacement }`; x86
  memory operands parse `[base+idx*scale+disp]` (incl. scale-1 form);
  renderer prints `&rbx[rcx * 0x8 + 0x10]`; SSA reads count index
  registers. Kills the last blanket-unknown memory class from the
  glibc run.
- Proof: d5_x86_lift 4/4 (authored `mov rax,[rbx+rcx*8+0x10]`).
  Workspace 49 suites, 0 failures.

### Unit 6.11 — natural loops + do-while rendering — DONE

- `find_natural_loops`: v1 dominance approximation (header's
  predecessors all inside the header..source range); self-loop
  CondBranch renders `} while (cond);`; other back edges annotate
  their specific goto in place (`goto …; /* loop back */`). Goto-parse
  bug caught by tests (hex letters in "flags:jne" matched as targets)
  — rewritten to exact `sub_` runs.
- Proof: d4_structure 9/9. Workspace 49 suites, 0 failures.

### Unit 6.12 — subs/adds/ands flags emission — DONE

- Register-destination flag-setters (subs/adds/ands) emit flags from
  operands [1],[2] AND the destination write; compare-only forms keep
  [0],[1]. `_main`'s `subs w8, w0, #0xb` now yields both statements
  (was dest-only).
- Proof: d1_lift 4/4. Workspace 49 suites, 0 failures.

### Unit 6.13 — adrp/adr materialization — DONE

- arm64 lift resolves page materializations (decoder Page/Data refs)
  into constant assignments. Real `_main` renders the string address
  chain (`x0 = 0x100000000; x0 = (x0 + 0x500);`) — adrp no longer in
  unknowns.
- Proof: workspace 49 suites, 0 failures.

### Unit 6.14 — cset modeling — DONE

- `cset`/`cinc` lift as conditional-set assignments (`cond:` unknowns
  carrying the condition suffix) — D4 resolves precisely. `_main` now
  decompiles with zero blanket unknowns; expectations updated.
- Proof: workspace 49 suites, 0 failures.

### Lab evidence run 5 (2026-10-29) — delta run 3 — DONE

- Real glibc `__GI___libc_malloc` now: **unknown_count 12 → 5 → 3**,
  6 structured lines. Trajectory across the D-series: cmp/test (−7),
  indexed memory (−1), remaining 3 = segment-ops/push-pop class.

### Unit 6.15 — delta run 3 recorded — DONE

### Unit 6.16 — push/pop/nop modeling — DONE

- push = Store [&sp−8], pop = Load [&sp]; nop/nopl/endbr64 = no-op
  statements. The synthetic blob now lifts with **zero** blanket
  unknowns; real-binary remaining unknowns are only genuinely exotic
  (segment ops, simd).
- Proof: workspace 49 suites, 0 failures (blob expectation fixed to
  the stronger zero-unknown contract).

### Unit 6.17 — D2 register renaming — DONE

- `rename_registers`: per-block versioned renaming (`x0_v1`, `x0_v2`
  on redefinition); uses resolve to the latest def; AddrOf /
  IndexedAddr bases and indices rename; Store value renames. Phi-true
  cross-block SSA is the next D2 slice.
- Proof: d2_ssa 4/4 (sequential defs version; use resolves latest).
  Workspace 49 suites, 0 failures.

Next 6.x: type seeds, Wave 5A value-flow dispatch port, cross-block
phi-SSA.

## Lab notes

- Linux lab: `nixos-rex` (Tailscale, NixOS x86_64, 12 cores / 62 GB,
  nix 2.34.9). The older `hetzner-ubuntu` alias is stale (DNS gone);
  `nixos-rex` is the working builder. Remote shell is **fish** — wrap
  commands in `bash -c`.

### Lab evidence run 2 (2026-10-29) — row 5.13 artifact — DONE

- `bin/elf-evidence-package.sh` real mode on `nixos-rex` against
  `/tmp/lab/ls` + `/tmp/lab/libc.so.6`: 9-line JSONL package
  (meta/analyze/functions/strings/selfcheck per target), sha256
  manifest, `--verify` OK (manifest-only path — verify needs no
  damsel binary). Recorded results: ls = 4,941 facts / 1,798 bounds /
  78,142 xrefs; libc.so.6 = 10,337 / 3,953 / 80,765.
- Script bugs caught in flight: SIGPIPE from `head` under pipefail;
  binary check blocked `--verify`. Both fixed with tests/re-runs.

### Lab evidence run 1 (2026-10-29) — first real-ELF proof — DONE

- Branch shipped via `git bundle` + scp (no GitHub auth needed on the
  box); built with the repo's own flake (`nix build .#default`) — the
  full workspace compiles clean on NixOS out of the box.
- **`damsel analyze` on real GNU coreutils `ls` (NixOS, 1.7 MB):**
  2,242 facts · 1,798 function bounds · **78,142 xrefs** · 3,678
  strings — dynsym-only (stripped symtab), zero typed failures.
- **`damsel analyze` on real glibc `libc.so.6` (2.4 MB):** 10,187
  facts · 3,953 bounds · 80,765 xrefs · 4,739 strings; allocator
  internals readable (`_int_malloc`, `sysmalloc`, `__ptmalloc_init`)
  with honest stop reasons (mix of `return` and `seed_boundary`).
- Query surface on real targets: `functions --name malloc` (23
  matches), `xrefs --to <main>` resolves real call sites.
- Linux test parity: `nix develop -c cargo test` for damsel-core/
  macho/elf/project — all suites green on NixOS (store contract,
  loader matrix, ingest, mapper tests included).
- Evidence status: ad-hoc run recorded here; a repeatable
  `bin/elf-evidence-package.sh` (evidence matrix row 5.13) is the next
  hardening step.
- Local toolchain: rustup stable-aarch64 (1.96.0) at
  `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` (not on default
  PATH in this shell).
