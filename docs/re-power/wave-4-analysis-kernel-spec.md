# Wave 4 — Analysis Kernel Implementation Spec

Status: Proposed implementation spec for Wave 4 of the
[Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md).

## Why This Wave Exists

Every current Damsel surface is a point query over a freshly loaded image.
Proprietary RE sessions run for hours or days; without memory — persisted
functions, xrefs, strings, and analyst knowledge — every question restarts
from zero. Wave 4 builds that memory once, format-neutrally, so Waves 5–8
(and the Apple track) all land on the same foundation.

## Scope

In scope:

- `damsel-project` crate: project kernel (project/source/address space/
  object ids/facts/byte map/problems/jobs) per the existing
  [project kernel spec](../hopper-gap/project-kernel-byte-map-implementation-spec.md).
- Whole-image analysis: function discovery, xref harvesting, string
  inventory, name indexing — promoted from existing window-local analysis
  into stored facts.
- CLI commands: `analyze`, `functions`, `xrefs`, `strings`, `search`, plus
  `project` introspection commands.
- UI/Workbench: function list, xref navigation, string browsing over the
  same routes.

Out of scope (hard):

- ELF/x86_64 (Wave 5), decompilation (Wave 6), batch/MCP (Wave 7),
  user annotation editing (semantic-editing spec lands it on this kernel;
  Wave 4 ships the transaction model but a minimal label/comment subset),
  patching.

## Consumed Specifications (Normative)

This spec does not duplicate these documents; it binds them and adds deltas:

- [Analysis database blueprint](../hopper-gap/analysis-database-blueprint.md)
  — entities, indexes, invalidation, privacy, budgets, storage stages.
  **Supersession (Decision D-3, 2026-06-12):** the blueprint's staged
  storage rollout (Stage 0 in-memory → Stage 1 log file → Stage 2 embedded
  store) is superseded for this program: the embedded store ships from the
  first implementation PR. The blueprint's schema-module, index, migration,
  and privacy rules remain normative.
- [Project kernel and typed byte map spec](../hopper-gap/project-kernel-byte-map-implementation-spec.md)
  — Rust modules, model contracts, ingest mapping, query API shape (its
  store staging is superseded by D-3 above).
- [Function recovery and procedure database plan](../hopper-gap/function-recovery-and-procedure-database-plan.md)
  — Procedure/ProcedureRange/ProcedureOwner/ProcedureStatus records, seed
  collection pipeline, recovery worker, boundary and conflict policy, call
  graph records.
- [Current code integration map](../hopper-gap/current-code-integration-map.md)
  — where existing types feed the kernel.

Deltas added by this spec (net-new decisions):

1. Function discovery seed fusion with explicit provenance ranking and a
   measurable precision/recall gate (§5).
2. `StringRecord` model and string harvesting pass (§6) — not previously
   specified anywhere in the repo.
3. Unified search index and ranking contract (§7) — the blueprint names the
   index; this spec defines its first shippable shape.
4. Corpus-scale determinism and budget gates (§9–10).
5. Cross-format readiness constraints: every Wave 4 type must name its
   format through the source model, never through Mach-O-specific types
   (§3.1).

## 1. Architecture

New crate layout:

```text
damsel-project/
  src/
    lib.rs          // public query facade
    project.rs      // AnalysisProject, profiles, schema version
    source.rs       // AnalysisSource kinds (macho, shared_cache_projected, [elf: W5])
    address.rs      // AddressSpace ids and typed conversions
    object.rs       // ObjectId, routes
    facts.rs        // AnalysisFact, provenance, confidence
    byte_map.rs     // classification range table
    functions.rs    // seed fusion, ProcedureRecords store, call graph
    xrefs.rs        // XrefRecord store, forward/reverse indexes
    strings.rs      // StringRecord store + harvesting pass driver
    search.rs       // ranked search index
    jobs.rs         // analysis pass scheduling, cancellation, progress
    snapshot.rs     // canonical export serializer (determinism artifact)
    store/
      mod.rs        // store trait (tests + future backends)
      sqlite.rs     // the project store (rusqlite, bundled SQLite, WAL)
      migrations.rs // versioned schema migrations (PRAGMA user_version)
```

`damsel-macho` gains `analysis/whole_image.rs`: the pass driver that runs
the existing engine over discovered functions and emits facts. The engine
itself is not rewritten in Wave 4; Wave 5A refactors its internals.

## 2. Model Deltas (Beyond The Kernel Spec)

```rust
pub struct StringRecord {
    pub id: ObjectId,
    pub source: SourceId,
    pub address: AddressIn<ImageVmaddr>,
    pub length: u32,
    pub encoding: StringEncoding,      // Utf8, Utf16Le, AsciiSubset
    pub section: Option<SectionRef>,
    pub classification: StringClass,  // Literal pool, Format, Path, Url, MangledName, SelectorLike, Unknown
    pub xrefs: Vec<ObjectId>,         // joined during harvest; indexed both ways
}

pub enum XrefKind {
    DirectCall, TailCall, Branch, FallthroughData,
    DataLoad, AddressMaterialization, String, ImportSlot,
    ExportSlot, ObjcSelector, ObjcClass, Relocation, VtableSlot, // W5
}

pub struct XrefRecord {
    pub id: ObjectId,
    pub from: ObjectId,               // instruction (or relocation) that references
    pub to: ObjectId,                 // function, string, symbol, section, import slot…
    pub kind: XrefKind,
    pub provenance: FactProvenance,   // pass, analyzer version, confidence
}
```

Function facts reuse the kernel spec's `Procedure*` records verbatim; this
spec adds only `discovery_provenance` payloads (seed kind + merge decision)
to `ProcedureStatus`.

## 3. Cross-Format Readiness Rules

1. No Wave 4 type may reference `MachO`, `DyldMetadata`, or `ObjcMetadata`
   directly; platform detail enters only as fact payloads and source kinds.
2. Address spaces are named per source (image vmaddr, cache vmaddr, and —
   reserved now — elf image vaddr), using the blueprint's address model.
3. The whole-image pass driver is parameterized by a `FormatBackend` trait
   (implemented for Mach-O in Wave 4, ELF in Wave 5B) exposing: executable
   ranges, seed candidates, string-candidate ranges, and symbol/import
   tables.
4. `doctor` gains no new capability rows in Wave 4 (nothing new is required
   of hosts).

## 4. Whole-Image Analysis Pipeline

Staged passes (each a cancellable job, each emitting facts + problems):

1. **Ingest** — sections/segments/symbols/imports/exports/dyld/ObjC become
   seed facts and byte-classification ranges (kernel spec passes 1–2).
2. **String harvest** — candidate extraction over string-typed ranges (§6),
   merged with ObjC selector/class names and symbol names already known.
3. **Function seeding + recovery** — §5; recursive-descent expansion per the
   function-recovery plan's recovery worker.
4. **Per-function analysis** — run the existing
   `analyze_disassembly_with_image` over each recovered function; promote
   its `DisassemblyAnalysis` outputs (references, targets, indirect
   controls, jump tables, ObjC uses, recovered values, cache links) into
   xref/fact records.
5. **Xref completion** — join data/address-materialization references to
   string/section/symbol objects; emit dangling references as problems, not
   silent drops.
6. **Index build** — name index, xref forward/reverse indexes, string index
   (§7).

Incremental behavior: byte-classification overrides, function-bound edits,
and annotation transactions invalidate per the blueprint's invalidation
scopes; reanalysis jobs must report invalidated ranges (kernel spec). The
first shipped subset: function-bound edits and label/comment transactions.

## 5. Function Discovery — Seed Fusion

Seed sources, in provenance order (all become `ProcedureOwner`-tagged facts):

| Rank | Seed | Confidence | Notes |
| --- | --- | --- | --- |
| 1 | Symbol table text symbols | exact | includes local symbols when present |
| 2 | Function-starts opcodes (Mach-O) / FDE ranges (ELF, W5) | exact | |
| 3 | ObjC method IMPs from metadata | exact | owner = class/category |
| 4 | Cache exports/local symbols (projected images) | exact | existing Wave 2/3 data |
| 5 | Init/fini/entry points, stub helpers | exact | |
| 6 | Direct call targets observed during expansion | strong | |
| 7 | Prologue templates (arm64: `pacibsp`/`stp x29, x30`+frame setup; W5 adds x86_64: `endbr64; push rbp; mov rbp,rsp`) | inferred | never expands without corroboration |
| 8 | Unreferenced executable sweep | weak | flagged `inferred`, hidden from default views until xrefs exist |

Fusion rules:

- Overlapping ranges from ranks 1–5 merge at the outermost exact bound;
  conflicts produce problem records (function-recovery plan conflict classes).
- Rank 6–8 seeds open a candidate procedure only if expansion produces ≥ 1
  valid terminator; otherwise the range stays byte-map `code_candidate` with
  a problem.
- Tail-call handling, stop conditions, and continue conditions follow the
  function-recovery plan verbatim.

Acceptance measurement (new gate): against the symbolized fixture rebuilt
stripped, function recovery recall ≥ 95 % of ground-truth text symbols and
false-positive rate ≤ 2 % on data-in-text fixtures; both measured by a
`bin/function-recovery-audit.sh` harness against fixtures with known truth
(symbols hidden from the analyzer, compared afterward).

## 6. String Harvesting

- Candidate ranges: sections classified string/literal (Mach-O `__cstring`,
  `__cfstring` constants, `__os_log`, `__ustring`; W5: `.rodata`, `.data`-based
  string islands on demand) plus, optionally, zero-terminated ASCII/UTF-16
  runs in generic data ranges behind a flag.
- Extraction: zero-terminated UTF-8 and UTF-16LE runs, minimum length 4
  (configurable per profile), bounded per-section; malformed tails become
  classification notes, never failures.
- Classification heuristics (first pass, cheap, provenance-tagged
  `inferred`): URL/path/format-string/mangled-name/selector-like; never
  renamed automatically — classification is display/filter metadata.
- Join: every materialization/load reference resolved against string ranges
  during pass 5 becomes a `String` xref; strings with zero xrefs are kept
  (they are evidence too) and are searchable.
- ObjC/Swift integration: selector/class/method names already in metadata
  link to the same string objects when byte ranges coincide — one object,
  not duplicates.

## 7. Search

First shippable shape (blueprint's ranking signals, reduced):

- Index: sorted name index over symbols, functions (incl. analyzer labels),
  strings, ObjC selectors/classes/methods (when present), import/export
  names, demangled names (when Wave 5 adds demanglers), section names.
- Matching: exact → prefix → substring → subsequence, with object-kind
  priority and source-scope boost; no fuzzy edit-distance in this wave.
- Contract: paged, stable ordering (rank, then kind priority, then name,
  then address), deterministic across runs; `search` CLI and
  `GET /api/projects/{id}/search` share it.

## 8. CLI Contracts

```text
damsel analyze <path> [--profile default|deep|strings-only]
                         [--refresh] [--jobs N]
damsel functions <project> [--name PAT] [--sort addr|name|size]
                           [--status exact|inferred|conflict] [--limit N]
damsel xrefs <project> --to <addr|symbol|function-id>
                       [--kind call|data|string|...] [--limit N]
damsel xrefs <project> --from <addr|function-id> [--limit N]
damsel strings <project> [--name PAT] [--class literal|path|url|...]
                         [--min-len N] [--xrefs] [--limit N]
damsel search <project> <query> [--kind ...] [--limit N]
damsel project <show|facts|problems|jobs> <project>
damsel project export <project> [--out file]     # canonical deterministic export
```

- `analyze` default profile: ingest + strings + functions + xrefs + indexes;
  `deep` adds the unreferenced sweep; progress on stderr, JSON Lines events
  with `--format json`.
- `<project>` accepts either the project directory (`foo.damsel/`, which
  contains the SQLite store) or the target path with an adjacent
  `foo.damsel/` directory; `--db <path>` overrides. The store is opened
  explicitly, never implicitly (no hidden state files).
- Text and JSON output follow repo conventions (stable key ordering,
  snapshots, typed errors such as `project_not_found`, `stale_schema`,
  `analysis_incomplete`).

Illustrative `xrefs --to` JSON (shape only; final key order fixed by
contract tests):

```json
{
  "target": {"kind": "function", "id": "fn:0x100005a40", "name": "sub_100005a40"},
  "xrefs": [
    {"from": "0x100004f18", "function": "fn:0x100004ec0", "kind": "direct_call",
     "confidence": "exact", "provenance": {"pass": "per_function_v1"}},
    {"from": "0x1000060a4", "function": "fn:0x100006000", "kind": "data_load",
     "confidence": "strong", "provenance": {"pass": "per_function_v1"}}
  ],
  "truncated": false
}
```

## 9. Storage, Determinism, And Migration

Decision D-3 (2026-06-12): build the real embedded store from day one. The
staged rollout is superseded; the design below is the shipping design.

**The store is SQLite** (via `rusqlite` with bundled SQLite, MIT):

- WAL mode, `foreign_keys = ON`, single-writer discipline (all writes go
  through the job runner or annotation transactions); readers never block
  on background analysis — this is what makes the scheduler and UI server
  viable.
- Schema versioning via `PRAGMA user_version` with explicit, tested
  migrations (`store/migrations.rs`). Every migration answers the
  blueprint's five questions (from-version, annotation impact, derived-fact
  drops, reanalysis need, reversibility).
- The blueprint's append-only event log lands as an `events` table inside
  the database (not a separate file stage): every mutation records actor,
  origin, transaction id, and timestamp — audit and undo read the same
  table.
- Schema modules follow the blueprint's logical modules (`project`,
  `sources`, `objects`, `facts`, `functions`, `references`, `strings`, …);
  the Rust API never exposes SQL — domain objects only.
- Derived, rebuildable payloads (disassembly row caches, string/xref shards
  for very large artifacts) may live in sidecar shards keyed by source hash
  + analyzer version (blueprint Stage 3 direction); the database row set is
  the source of truth.

**Determinism moves to exports.** The `.db` file is *not* byte-deterministic
across SQLite builds, and that is fine:

- `damsel project export` emits a canonical export (schema-versioned,
  BTree-ordered serialization, no timestamps, no map-iteration order) that
  *is* byte-deterministic and doubles as the CI fixture and diff format.
- Determinism gate: two `analyze` runs over identical bytes produce
  byte-identical canonical exports (CI-enforced). The database is
  rebuildable from source at any time, so exports and dbs are always
  reconcilable.

**Migration gate:** a real N−1 database fixture is checked in; opening it
under N offers a typed upgrade; user annotations survive (blueprint
migration rules). Migration tests are mandatory in every schema-bump PR.

**Why SQLite and not a pure-Rust KV store** (recorded for the future
reader): the workload is relational and paged (name search, xref joins,
range queries); WAL gives concurrent reads during writes; migrations and
`EXPLAIN QUERY PLAN` make performance work inspectable. This matches the
blueprint's own Stage 2 reasoning; D-3 only removes the deferral.

## 10. Fixtures, Tests, And Budget Gates

New fixtures (checked in with drift manifests, rebuilt by the existing
macOS lane):

- `xref-rich`: deliberate call/data/string reference web with source kept in
  `fixtures/src` for truth.
- `strings-corpus`: mixed UTF-8/UTF-16, format strings, paths, selectors.
- `deep-recursive`: tail calls, switch tables, cold/split functions to
  exercise boundary policy.
- `data-in-text`: embedded data inside `__TEXT` to punish naive sweeps
  (false-positive gate).
- `stripped-30mb`: generated large fixture for budget gates (not fully
  checked in; built by script, hash-manifested).

Test matrix (all JSON-contract + snapshot + typed-failure covered):

- ingest truth vs fixture truth (sections/symbols/imports)
- seed fusion precision/recall audit (§5 gate)
- xref round-trip: every reference discoverable from both endpoints
- string harvest encodings/classification/xref join
- search ranking stability across runs
- determinism (§9) and schema migration
- invalidation subset: function-bound edit invalidates only dependents
- paged/limit/truncation contracts
- UI API: function list, function detail, xrefs, strings, search endpoints
  with image-id→project migration kept compatible with current upload mode

Budget gates (CI, corpus bench tier): 30 MB stripped index build ≤ 30 s;
point queries ≤ 50 ms p95; search first page ≤ 150 ms p95; warm reopen
≤ 1 s; memory bounded by paged stores (no whole-image row materialization).

## 11. Workbench Integration

- Workbench project open (path-based registration preferred over upload for
  projects; upload mode remains for compatibility).
- Function list with virtualization; selection routes to existing
  instruction surface; inspector gains function facts (bounds, status,
  provenance) and xref panes (incoming/outgoing).
- Strings browser with filters and jump-to-xref.
- No new editing surfaces beyond label/comment transactions (minimal
  subset) — the semantic-editing spec owns that UX.

## 12. Risks

- Over-claiming function bounds — mitigated by provenance-ranked seeds,
  conflict problems, and the audit gate.
- Schema churn in the SQLite store — mitigated by versioned migrations
  from the first commit, mandatory N−1 migration fixtures in every
  schema-bump PR, and canonical exports decoupling the determinism gate
  from the store file format.
- Whole-image pass performance — mitigated by parallel per-function
  execution within the jobs scheduler and the budget gates; engine-level
  parallelism deepens in Wave 7.
- Scope creep into editing/ELF — mitigated by the hard out-of-scope list.

## 13. Open Questions

1. Whether `strings` belongs in the default profile (cost on huge data
   sections) or stays opt-in — decide from budget-gate numbers.
2. Minimum viable annotation subset shipping in Wave 4 vs deferring all
   editing to the semantic-editing spec's own milestone — default: labels +
   comments only.

## 14. References

- [Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md) §5 Wave 4
- [Analysis database blueprint](../hopper-gap/analysis-database-blueprint.md)
- [Project kernel spec](../hopper-gap/project-kernel-byte-map-implementation-spec.md)
- [Function recovery plan](../hopper-gap/function-recovery-and-procedure-database-plan.md)
- [Corpus benchmark regression plan](../hopper-gap/corpus-benchmark-regression-plan.md)
- [Current state audit](current-state-audit.md)
