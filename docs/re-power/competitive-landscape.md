# Competitive Landscape

Status: Research record for the RE Power program. Core external claims were
**live-verified 2026-10-28** by direct primary-source fetching (see
[Research Sources](research-sources.md) §0: two corrections found and
applied — capstone feature semantics and MCP transports — plus verified
licenses, encoder/demangler APIs, and blocked-host records). Web search was
not used; direct fetches were. Remaining unverified claims carry explicit
status and must be refreshed before public positioning.

Purpose: define what "genuinely powerful at reverse-engineering proprietary
macOS/Linux binaries" means competitively, where Damsel should and should
not fight, and what each wave must therefore deliver.

## Method

- Compare on the axes that matter to the phase's personas (§2 of the
  roadmap): whole-image navigation speed, recovered semantics quality,
  platform depth, automation surface, honesty/provenance, licensing/cost.
- Baseline axes intentionally excluded from competition: IDE-style team
  collaboration, cloud analysis, firmware supply-chain breadth, and
  debugger-first workflows (explicitly deferred in this program).

## Competitor Profiles

### Hopper Disassembler (macOS/Linux, commercial)

- Strengths: native macOS app polish; fast interactive analysis of large
  Mach-O files; procedural/byte-typing document model exposed through a
  Python SDK; pseudocode marketed with honest caveats; bundled
  debugger-server for local/remote LLDB-style sessions.
- Weaknesses: single-analyst document model; closed ecosystem; automation
  beyond the embedded Python SDK is limited; shared-cache and Apple-runtime
  depth is thinner than its app polish suggests (per the repo's own
  Hopper-gap research).
- Verification: high (the hopper-gap pack holds extensive public-web
  research: `hopper-public-research.md`, `hopper-deep-public-web-research.md`,
  `hopper-sdk-and-document-architecture-research.md`).

### Ghidra (cross-platform, free, Apache-2.0)

- Strengths: the default serious free workbench; SLEIGH/P-code processor
  breadth; mature decompiler; scripting in Java/Python; active ecosystem.
- Weaknesses: heavyweight UX; import/analysis latency on large binaries;
  Apple-runtime-specific knowledge (dyld caches, ObjC/Swift, PAC) is
  shallower than its ISA breadth; determinism and CI-native operation are
  not product surfaces.
- Verification: high (long-stable public product).

### IDA Pro (commercial, gold standard)

- Strengths: best-in-class analysis heuristics, FLIRT signature matching,
  Hex-Rays decompiler quality, debugger suite, ecosystem maturity.
- Weaknesses: cost; closed; Linux/macOS Apple-runtime depth (shared caches,
  ObjC/Swift) relies on plugins; scripting is powerful but heavy to
  automate reproducibly.
- Verification: medium (feature surface stable; version-specific claims not
  re-verified).

### Binary Ninja (commercial)

- Strengths: API-first architecture; fast MLIL/HLIL layered IR; excellent
  headless/automation story; growing decompiler quality.
- Weaknesses: native Apple-runtime depth limited; UI app less polished for
  Mac-first analysts; IL quality varies by ISA.
- Verification: medium-high.

### rizin / radare2 (+ Cutter GUI)

- Strengths: free, scriptable, huge command surface, good format breadth
  including Mach-O/ELF; rizin refactors the r2 API toward cleanliness.
- Weaknesses: steep UX; analysis determinism/quality varies; Apple-depth
  (shared cache, ObjC provenance) partial; docs uneven.
- Verification: medium-high.

### CLI baselines (llvm-objdump, otool, readelf, nm)

- The repo already treats LLVM as the external comparison oracle. These
  tools are exact but point-query oriented: no persistence, no xrefs beyond
  symbols, no semantics. Damsel's CLI discipline (typed JSON, deterministic
  output, typed failures) is directly more automatable.
- Verification: high.

### Programmatic frameworks (angr, capstone-based tooling)

- angr (and its lifter/VEX IR) defines the programmatic-analysis bar for
  ELF: symbolic execution, CFG recovery, pipelining in Python. Heavyweight;
  not a workbench; Apple support weak.
- Verification: medium.

## Feature Comparison

Axis-by-axis view (D = Damsel today, D′ = Damsel after this phase):

| Axis | Hopper | Ghidra | IDA | BinJa | rizin | D | D′ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Whole-image function/xref index | yes | yes | yes | yes | yes | no | **yes** (W4) |
| Deterministic typed JSON for every view | partial | partial | partial | good | partial | **yes** | **yes** |
| Dyld shared-cache as first-class container | partial | partial | plugin | partial | partial | **yes** | **yes** |
| ObjC/Swift/PAC Apple depth | good | partial | plugin | partial | partial | good→**deep** | deep (W8) |
| ELF + Linux proprietary lane | partial | yes | yes | yes | yes | no | **yes** (W5) |
| x86_64 + arm64 peer ISAs | yes | yes | yes | yes | yes | arm64 only | **yes** (W5) |
| Pseudocode/decompiler | marketed, caveated | mature | best | good | partial | no | MVP, honest (W6) |
| Version-to-version binary diff | no | via plugins | via BinDiff | via plugins | partial | no | **yes, core** (W7) |
| Obfuscation/packing recon | no | partial | partial | partial | partial | no | **yes** (W7) |
| Headless batch + agent/MCP server | weak | scripting | scripting | good | scripting | no | **yes, core** (W7) |
| Provenance/confidence on every fact | no | partial | partial | partial | no | partial (analysis) | **yes, core** (W4+) |
| Native macOS workbench app | **best** | Java UI | Qt | Qt | Cutter (Qt) | early | strong (W7) |
| Patching | yes | yes | yes | yes | yes | no | gated MVP (W8; dormant until re-decision, D-5) |

## Differentiation Strategy

**Fight on:**

1. **Apple-runtime depth as the wedge** — shared cache, ObjC/Swift/PAC,
   dyld semantics. No competitor treats the dyld cache as a first-class,
   navigable, diffable container. This remains the flagship even as the ELF
   lane lands.
2. **Determinism and typed evidence as a product surface** — every view is
   typed JSON with stable ordering, every fact carries provenance, every
   claim ships with an evidence artifact. This is what CI pipelines, audits,
   and AI agents actually need; no competitor centers it.
3. **Agent-native operation (W7)** — MCP tools returning route-backed,
   paged, provenance-carrying facts rather than prose. First-mover position
   for binary-analysis agents on macOS/Linux proprietary targets.
4. **Version archaeology (W7 diff)** — the analyst workflow "vendor shipped
   1.2.3, what changed?" executed as a first-class command on both
   platforms, including stripped binaries.
5. **Proprietary-binary triage (W7 recon)** — packing, flattening, opaque
   predicates, anti-analysis indicators at first touch, with proofs where
   the value-flow engine can establish them.

**Do not fight on:**

- ISA breadth (SLEIGH-scale processor support), debugger suites, team
  collaboration, decompiler output quality versus Hex-Rays, FLIRT-scale
  signature libraries, or cloud services.
- Being free: Ghidra/rizin own that position; Damsel's value is depth,
  determinism, and automation, not price.

**Honest post-phase gaps to state publicly:** decompiler maturity behind
Ghidra/IDA; no live debugging; no team features; ELF lane is userspace-only;
patching is a bounded MVP.

## Implications Per Wave

- Wave 4 must reach "boring reliability": the function/xref index is table
  stakes for every competitor; our differentiator is determinism +
  provenance, so facts-not-strings is the contract from day one.
- Wave 5 should copy nothing from competitors except the fixture discipline;
  the semantic-opcode layer is our structural answer to what BinJa solved
  with ILs and Ghidra with P-code — smaller, typed, testable.
- Wave 6 ships behind honesty gates because we cannot out-Hex-Rays Hex-Rays
  in one phase; summary quality and provenance lines are the credibility
  play.
- Wave 7 is where Damsel becomes category-unique: MCP + batch + diff + recon
  as first-class surfaces competitors treat as afterthoughts.
- Wave 8 patches only what the recon/diff/index loop proves analysts need.

## Sources And Verification Status

See [Research Sources](research-sources.md) §1 for the competitor source
list. Basis: maintainer knowledge of public product surfaces as of early
2026 plus this repo's own Hopper-gap research pack (which contains
link-level evidence for the Hopper column). Refresh cadence: before any
external-facing claim, and at each wave's exit review.
