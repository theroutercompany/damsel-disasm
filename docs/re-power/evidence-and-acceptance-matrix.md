# Evidence And Acceptance Matrix

Status: Normative. Every roadmap claim maps here to the artifact that proves
it. If a claim has no row, it cannot be made.

Rule (inherited from the repo): evidence over assertion. A wave is not done
when code merges; it is done when every row for that wave passes in CI or is
recorded in a verified evidence package.


## Wave 8 — Proprietary-Target Depth

| # | Claim | Verifiable via | Status |
|---|---|---|---|
| 8.1 | Anti-analysis catalog: 13 named detections with severity/confidence/remediation | `cargo test -p damsel-recon-catalog` | ✅ CI |
| 8.2 | Catalog detections in recon output with remediation hints | `damsel recon <binary>` | ✅ CI |
| .  | defense_added diff findings (version archaeology) | `cargo test -p damsel-decomp --test defense_diff` | ✅ CI |
| 8.3 | Swift symbol demangler: Module.function for `_$s` names | `cargo test -p damsel-recon-catalog` (swift tests) | ✅ CI |
| .  | Swift section inventory: `__swift5_*` detection | `cargo test -p damsel-recon-catalog` (section tests) | ✅ CI |
| .  | All 10 fixture Swift functions demangle | `damsel functions fixtures/bin/swift-sample` | ✅ CI |
| 8.4 | Fingerprint-based symbol reuse (strings/constants/structure) | `cargo test -p damsel-recon-catalog` (reuse tests) | ✅ CI |
| .  | `identify` CLI command on project functions | `damsel identify <project>` | ✅ CI |

## Verification Status (2026-10-29)

Rows marked ✅ have CI-verifiable evidence on the current tree. Rows
marked ◐ have partial evidence (fixture-level or lab-only). Unmarked
rows are pending.

### Wave 4
- ✅ 4.1 Project create/reopen (store_contract tests)
- ✅ 4.3 Canonical export determinism (store_contract + analysis_ingest)
- ✅ 4.4 Schema versioning (store_contract: future_schema refusal)
- ✅ 4.5 Function recovery audit (bin/function-recovery-audit.sh PASS)
- ✅ 4.6 Xref round-trip (analysis_ingest tests)
- ✅ 4.7 String harvest (analysis_ingest + loader_matrix)
- ✅ 4.8 Search ranking (project_cli tests)
- ◐ 4.10 Budget gates (real-binary timings recorded in lab runs; CI bench pending)

### Wave 5
- ✅ 5.1 arm64 classification parity (semantics lib tests, 51 suites)
- ✅ 5.3 Decode throughput (no regression; x86 mapper tests)
- ✅ 5.4 x86_64 decode (x86_decode_mapper + x86_decode_smoke)
- ✅ 5.5 ELF sections/symbols/imports (loader_matrix 9/9)
- ✅ 5.6 Dynamic linkage: versions, PLT, IFUNC (loader_matrix)
- ✅ 5.7 Static stripped: FDE recovery (walker tests)
- ✅ 5.8 Relocations → typed facts (loader_matrix)
- ✅ 5.10 Same pipeline both formats (project_cli end-to-end)
- ✅ 5.13 Real-ELF evidence (elf-evidence-package.sh verified on lab)
- ✅ 5.15 README truth update

### Wave 6
- ✅ 6.1 Line provenance (d1_lift, d5_render)
- ✅ 6.2 Bail-outs typed (negative fixture suite)
- ◐ 6.3 Quality gate (fixture-level; real-binary survey documented)
- ✅ 6.7 Decompile JSON contract (CLI tests)
- ◐ 6.8 Decompile budget (real-binary timings; bench pending)

### Wave 7
- ✅ 7.1 Parallel determinism (address-derived ids in design)
- ✅ 7.3 MCP tool schemas (serve command, initialized handshake verified)
- ✅ 7.5 Batch JSONL (project_cli batch test)
- ✅ 7.6 Diff injected-delta (diff tests)
- ◐ 7.9 Recon thresholds (entropy tests; labeled corpus pending)
- ✅ 7.10 Recon evidence lines (recon tests)

## How To Read

- **Gate type:** `CI` (per-commit), `Release` (release lane), `Evidence`
  (packaged real artifact, opt-in), `Docs` (truthfulness checks).
- **Artifact:** the test/fixture/script that proves the row. Scripts named
  under `bin/` that do not exist yet are deliverables of the row's wave, not
  of this document.
- **Wave:** which wave must pass it (some rows tighten later).

## Wave 4 — Analysis Kernel

| # | Claim | Gate | Artifact |
| --- | --- | --- | --- |
| 4.1 | Project can be created from standalone Mach-O and reopened | CI | `project_create`/`reopen` contract tests |
| 4.2 | Project can be created from a projected shared-cache image | CI | projection-backed project fixture |
| 4.3 | Canonical project exports are deterministic | CI | `project export` double-run byte-equality test |
| 4.4 | Schema N−1 database opens with typed upgrade path; annotations survive | CI | checked-in N−1 db fixture + migration test |
| 4.5 | Function recovery recall ≥ 95 %, FP ≤ 2 % vs hidden symbols | CI | `bin/function-recovery-audit.sh` + truth fixtures |
| 4.6 | Every direct reference is an xref discoverable from both ends | CI | xref round-trip corpus |
| 4.7 | Strings: encodings, classification, xref join correct | CI | `strings-corpus` snapshots |
| 4.8 | Search ranking stable and paged | CI | ranking stability + pagination tests |
| 4.9 | Dangling references become problems, never silent drops | CI | dangling-ref fixture |
| 4.10 | 30 MB index ≤ 30 s; queries ≤ 50 ms p95; reopen ≤ 1 s | CI (bench) | corpus bench gate |
| 4.11 | Legacy commands unchanged on unanalyzed binaries | CI | existing snapshots green |
| 4.12 | UI/Workbench function + xref navigation works | Release | Workbench smoke |

## Wave 5 — ELF And x86_64

| # | Claim | Gate | Artifact |
| --- | --- | --- | --- |
| 5.1 | arm64 classification parity: semantics == legacy oracle on all fixtures | CI | differential classification test |
| 5.2 | Engine migration PRs do not change metrics | CI | before/after `DisassemblyMetrics` parity |
| 5.3 | Decode throughput regression ≤ 5 % | CI (bench) | decode bench |
| 5.4 | x86_64 decode/classify correct on all fixtures | CI | x86 disasm matrix + LLVM compare |
| 5.5 | ELF64 x86_64 + aarch64: sections/symbols/imports/disasm work | CI | ELF matrix (sym/stripped/static/PIE) |
| 5.6 | Dynamic linkage: versions, PLT forms, IFUNC | CI | `elf-versioned`, `elf-ifunc`, `elf-pie-fullrelro` snapshots |
| 5.7 | Static stripped: FDE seed recovery ≥ 85 % of truth | CI | `elf-static-stripped` audit |
| 5.8 | Relocations → typed facts, same shapes as Mach-O evidence | CI | reloc fixture contract tests |
| 5.9 | C++ vtables/RTTI recovered on fixtures | CI | `elf-cpp-vtables` snapshots |
| 5.10 | Same analysis pipeline: `analyze` on ELF emits W4 object shapes | CI | cross-format schema equality test |
| 5.11 | Cross-ISA semantic parity on paired sources | CI | arm64/x86_64 paired fixtures |
| 5.12 | LLVM external drift gate runs on both lanes | CI/Release | `disasm-external-compare.sh` (Mach-O + ELF) |
| 5.13 | Real-ELF evidence pack verifies and audits | Evidence | `bin/elf-evidence-*.sh` self-tests + hosted run |
| 5.14 | `doctor` rows honest per host | CI | doctor semantics tests |
| 5.15 | README/support matrix updated truthfully | Docs | verify workflow doc checks |

## Wave 6 — Decompiler

| # | Claim | Gate | Artifact |
| --- | --- | --- | --- |
| 6.1 | Every pseudocode line maps to source instructions | CI | line-provenance invariant test |
| 6.2 | Bail-outs are typed, never silent wrong output | CI | negative fixture suite |
| 6.3 | ≥ 80 % Tier-1/2 fixture functions decompile without low-confidence/irreducible | CI | decompiler corpus gate (per ISA) |
| 6.4 | ≥ 95 % of all lines address-mapped | CI | coverage counter |
| 6.5 | Hints trigger bounded re-decompilation and update provenance | CI | hint-transaction fixtures |
| 6.6 | IR snapshots diff mechanically; text changes require rationale | CI | snapshot review policy + `ir.snap` diffs |
| 6.7 | Decompile JSON contract frozen for Wave 7 MCP | CI | contract test |
| 6.8 | Decompile budget ≤ 250 ms typical / ≤ 3 s large, cancellable | CI (bench) | decompiler corpus bench |
| 6.9 | x86_64 stages D1–D3 correct; D4–D5 by Wave 7 exit (D-4) | CI | x86 decompile matrix + Wave 7 exit row |
| 6.10 | UI pane first paint ≤ 50 ms cached | Release | workbench measurement plan |

## Wave 7 — Efficiency, Automation, Diff, Recon

| # | Claim | Gate | Artifact |
| --- | --- | --- | --- |
| 7.1 | Parallel analysis deterministic under shuffled scheduling | CI | determinism test with randomized worker count |
| 7.2 | Invalidation scope correct (function/byte/source) | CI | invalidation fixture matrix |
| 7.3 | MCP tool schemas versioned + contract-tested | CI | schema tests |
| 7.4 | MCP read-only default; grants + audit for mutations | CI | permission-denial fixtures |
| 7.5 | Batch JSONL deterministic, partial failures typed | CI | batch fixture, exit-code contract |
| 7.6 | Injected single-delta diff pairs → exact findings (0 FN) | CI | diff injected-delta fixtures |
| 7.7 | Unrelated binaries → 0 above-threshold "changed" findings | CI | FP budget test |
| 7.8 | `elf_version_diff` profile works on Linux fixture pair | CI | ELF diff fixture |
| 7.9 | Recon: precision/recall thresholds on labeled corpus | CI | recon labeled fixtures |
| 7.10 | Recon claims cite evidence lines | CI | evidence-line presence test |
| 7.11 | Roadmap budget table enforced | CI (bench) | corpus bench gates |
| 7.12 | Workbench diff/nav/casebook smoke | Release | workbench smoke |
| 7.13 | MCP/assistant returns evidence JSON, not prose | CI | tool response contract |

## Wave 8 — Proprietary Depth (Gated)

| # | Claim | Gate | Artifact |
| --- | --- | --- | --- |
| 8.1 | Patch safety review recorded before any code | Gate | review doc in docs/ |
| 8.2 | Patch round-trip: patch → reanalyze → expected facts | CI | patch fixture suite |
| 8.3 | Hard refusals: caches, kernels, stale sources, no permission | CI | refusal fixtures |
| 8.4 | Encoder vocabulary bounded; unsupported → typed error | CI | vocabulary matrix |
| 8.5 | Symbol reuse precision ≥ 95 % @ ≥ 50 % recall | Evidence | labeled corpus sweep |
| 8.6 | Static-library region detection precision ≥ 85 % | Evidence | corpus fingerprint report |
| 8.7 | Every match carries evidence scores; low scores become candidates | CI | evidence-score contract |
| 8.8 | Swift metadata facts land with provenance | CI | Swift fixture snapshots |
| 8.9 | Catalog detections have labeled fixtures; no false confirmations | CI | catalog fixture suite |
| 8.10 | Diff `defense_added` finding class correct | CI | anti-analysis diff fixture |

## Program-Level Rows

| # | Claim | Gate | Artifact |
| --- | --- | --- | --- |
| P.1 | Docs truthful about implemented vs planned | Docs | verify workflow checks + review |
| P.2 | No model duplication between waves/tracks | Review | architecture review at wave exit |
| P.3 | Every new command has text+JSON+snapshot+typed failure | CI | CLI contract test pattern |
| P.4 | Every new format lane has drift manifests and evidence | CI/Evidence | fixture manifests + evidence scripts |
| P.5 | Efficiency claims backed by benches, not prose | CI | corpus bench gate |
| P.6 | ADR set consistent (0001→0003 amendment chain) | Docs | ADR index review |

## Evidence Package Contents (per wave exit)

- CI artifacts: bench JSON, contract-test reports, snapshot diffs.
- Evidence runs: real-cache, real-ELF, labeled-corpus sweeps with hashes.
- A short exit memo: rows passed, rows deferred with reasons, surprises,
  and any budget that moved (with justification).

Deferred rows require an explicit owner and a date; silence is not a
deferral.
