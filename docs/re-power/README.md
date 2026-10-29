# RE Power Documentation Pack

Status: Planning and research pack for Waves 4–8 of the RE Power program
(program accepted 2026-06-12; implementation not started). This pack is not
an implementation claim.

Audience: Damsel maintainers executing
[Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md)
(Waves 4–8), whose goal is to make Damsel genuinely powerful at
reverse-engineering proprietary macOS and Linux binaries efficiently.

Program status: **accepted 2026-06-12** (roadmap §10 Decision Log, D-1
through D-5). Implementation has not started. The two consequential
decision overrides versus the original pack draft: the embedded SQLite
project store ships from day one (D-3, supersedes the staged rollout), and
decompiler quality is a standing workstream with x86_64 pseudocode required
by Wave 7 exit (D-4). Damsel remains read-only until an explicit
re-decision (D-5).

Relationship to the [Hopper Gap pack](../hopper-gap/README.md): that pack is
the Apple-only program (twelve milestones with implementation specs). This
pack sequences and generalizes its format-neutral core to two platforms and
two ISAs, and adds the surfaces the Hopper-gap pack does not cover: a Linux
ELF lane, x86_64, strings/search as products, obfuscation triage, corpus
batch automation, and measured efficiency gates. The consumption map in the
roadmap (§4) is normative: no workstream invents a second model.

## Pack Map

- [Current State Audit](current-state-audit.md): precise inventory of what
  exists today, with file anchors, including the engine-level audit that
  grounds the Wave 5A semantic-opcode refactor.
- [Competitive Landscape](competitive-landscape.md): researched comparison
  against Hopper, Ghidra, IDA, Binary Ninja, rizin/radare2, and CLI baselines;
  differentiation strategy and honest gaps.
- [Wave 4 — Analysis Kernel Spec](wave-4-analysis-kernel-spec.md): project
  database, function discovery, xrefs, strings, search.
- [Wave 5 — ELF And x86_64 Spec](wave-5-elf-x86-spec.md): semantic-opcode
  refactor, `damsel-elf`, x86_64 decode, cross-platform C++ recovery.
- [Wave 6 — Decompiler Spec](wave-6-decompiler-spec.md): scheduling the
  decompiler IR strategy D0–D6 across two ISAs with honesty gates.
- [Wave 7 — Efficiency, Automation, Diff Spec](wave-7-efficiency-automation-spec.md):
  performance engineering, batch/MCP automation, binary diffing, recon
  triage, workbench integration.
- [Wave 8 — Proprietary-Depth Spec](wave-8-proprietary-depth-spec.md): patch
  studio, symbol reuse corpora, Swift/PAC depth, anti-analysis catalog.
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md): every
  roadmap claim mapped to the artifact that proves it.
- [Research Sources](research-sources.md): external sources with
  why-it-matters notes and verification status.

## Reading Order

1. Roadmap (the program, budgets, sequencing).
2. Current State Audit (the starting line).
3. The wave spec you are about to implement.
4. Evidence And Acceptance Matrix (your definition of done).
5. Research Sources (when you need the external grounding).

## Conventions

- Every spec carries: scope, contracts (model/CLI/API), algorithms,
  fixtures and test matrices, budgets, risks, open questions, references.
- Rust sketches are illustrative type shapes, not committed APIs; final names
  follow repo conventions and review.
- JSON examples are illustrative; shipped contracts must have stable key
  ordering and JSON contract tests like the existing command family.
- Status language: "Proposed" means decided in this pack but not implemented;
  implemented work is marked in the roadmap and README, per the repo's
  honesty rule.
- External facts used by decisions (crate capabilities, licenses, tool
  surfaces) are recorded with verification status in Research Sources;
  unverified items must be confirmed before depending on them.
