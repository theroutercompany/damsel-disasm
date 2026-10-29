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

### Next units (queue order)

- 4.3 `damsel analyze` CLI — DONE (see above)
- 4.4b recovery worker v1 (bounds/blocks from CFG walk) + audit gate.
- 4.5 xref harvesting — DONE (see above)
- 4.6 strings pass + `strings` command; search index + `search`.
- 4.7 budgets on `stripped-30mb` fixture (corpus bench rows 4.10).

## Wave 5 — ELF + x86_64 (parallelizable from 5A)

Not started. First step: 5A semantics producer behind differential
oracles (arm64), then `arch_x86` feature flip + x86 producer (spike-proven
sufficient; syscall intrinsic table required).

## Lab notes

- Linux lab: `hetzner-ubuntu` SSH alias (Tailscale) — verify access when
  the ELF fixture lane (5C) needs it; not yet exercised.
- Local toolchain: rustup stable-aarch64 (1.96.0) at
  `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` (not on default
  PATH in this shell).
