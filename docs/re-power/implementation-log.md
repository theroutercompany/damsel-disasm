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

Next 5A steps: value-flow dispatch (the `synthesize_*_value` ladder)
and jump-table recognizers behind the same oracle; then the x86_64
decode mapper in `decode.rs` (variable-length windows) and engine
migration PRs (d)–(f). First step: 5A semantics producer behind differential
oracles (arm64), then `arch_x86` feature flip + x86 producer (spike-proven
sufficient; syscall intrinsic table required).

## Lab notes

- Linux lab: `hetzner-ubuntu` SSH alias (Tailscale) — verify access when
  the ELF fixture lane (5C) needs it; not yet exercised.
- Local toolchain: rustup stable-aarch64 (1.96.0) at
  `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` (not on default
  PATH in this shell).
