# ADR-0003: Cross-Platform RE Scope (macOS + Linux)

- Status: Accepted 2026-06-12 (implementation not yet started)
- Date: 2026-06-12
- Decision record: accepted by the maintainer together with program decisions
  D-1 through D-5, logged in
  [Roadmap §10 Decision Log](../roadmaps/re-power-roadmap.md).
- Supersedes: the scope lock in
  [ADR-0001: Apple Runtime Focus](ADR-0001-apple-runtime-focus.md). ADR-0001's
  implementation record remains historically accurate; only its
  "no ELF / depth-not-breadth" decision is amended.
- Implemented by: [Wave 5 of the Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md),
  specified in [Wave 5 — ELF And x86_64](../re-power/wave-5-elf-x86-spec.md).

## Context

ADR-0001 locked `damsel` to Apple-only analysis when the tool was a
Mach-O point-query CLI. Since then:

- Waves 1–3 landed a typed core model, a dyld shared-cache container with
  cross-image linkage, ObjC metadata, and a window-local arm64/arm64e
  analysis engine with value recovery, plus a native Workbench and mature
  verification/evidence infrastructure.
- The next phase's highest-value infrastructure (project database,
  functions, xrefs, strings, decompiler, diff, MCP automation) is format-
  and ISA-neutral; only the loaders and decoders are platform-specific.
- Proprietary targets straddle macOS and Linux: the same vendors ship macOS
  daemons/frameworks and Linux x86_64/aarch64 ELF binaries, drivers, and
  agents. A macOS-only tool leaves half the work uncovered.
- Grounding facts verified in the [current state audit](../re-power/current-state-audit.md):
  - Capstone is the existing decoder dependency; the x86 backend requires
    only the `arch_x86` feature on the existing `capstone` dependency
    (one workspace line — verified 2026-10-28 that `full` alone does not
    enable architectures).
  - `goblin` and `object` (already workspace dependencies) parse ELF.
  - `Architecture::X86_64` and `Endianness` already exist in the core model.
  - The analysis engine's mnemonic-string matching is the single structural
    blocker to a second ISA; the Wave 5A semantic-opcode refactor removes it
    with a tested migration path.

## Decision

`damsel` becomes a two-platform, static-first reverse-engineering workbench:

- Formats: Mach-O (including dyld shared caches) and ELF64.
- ISAs: arm64, arm64e, x86_64 (ELF aarch64 reuses the arm64 decoder).
- ELF scope: userspace executables, shared libraries, and daemons; static
  stripped binaries are in scope; the Linux kernel module lane is an
  explicitly gated stretch.
- All Apple-specific depth work continues unchanged; nothing Apple-only is
  de-prioritized by this decision.

Depth discipline (what prevents breadth from becoming shallowness):

1. Format-neutral infrastructure lands before the second format
   (Wave 4 kernel and Wave 5A semantics precede Wave 5B ELF).
2. Each platform lane must satisfy the same verification discipline:
   drift-manifested fixtures, LLVM/readelf external comparison, packaged
   evidence runs, budget gates.
3. No feature may be platform-specific in its model; only its loader,
   decoder producer, and fixture lane.

## Consequences

- One analysis pipeline serves both ISAs; the semantic-opcode refactor is a
  hard prerequisite for x86_64 analysis, not optional debt.
- Fixture/CI/evidence infrastructure gains an ELF lane (Linux rebuild lane,
  LLVM external comparison, real-ELF evidence sweep) with the same drift
  discipline.
- `doctor` and the README support matrix gain honest rows
  (`elf-analysis`, `elf-fixture-rebuild`).
- C++ ABI recovery becomes cross-platform (Itanium) instead of Apple-only,
  which strengthens rather than dilutes the Apple ObjC++ story.
- PE/COFF, 32-bit ELF, Android/Bionic semantics, and live-process analysis
  remain out of scope.

## Alternatives Considered

1. **Stay Apple-only, add x86_64 Mach-O only** — rejected: leaves the Linux
   half of proprietary targets uncovered; most cross-platform RE value
   (indexing, decompiler, diff, automation) is wasted on a single platform.
2. **Rewrite on a generic multi-format framework** — rejected: discards the
   typed-model, shared-cache, and evidence investments that differentiate
   damsel; breadth-first was already rejected in ADR-0001 and remains wrong.
3. **Ship ELF via shelling out to external tools (objdump/readelf)** —
   rejected: violates deterministic typed output, cannot power
   xrefs/decompiler/diff, and breaks offline evidence workflows.
4. **Adopt a third-party analysis framework (e.g., riding Ghidra's loader
   or P-code)** — rejected: heavy dependency, licensing/Java coupling, and
   it would replace rather than extend the repo's typed model and CLI
   contracts.

## Risks

| Risk | Mitigation |
| --- | --- |
| x86_64 variable-length decode complicates windowing and analysis | semantic opcodes; x86_64 decompiler coverage is preview-level at Wave 6 exit and required by Wave 7 exit (D-4); paired fixtures |
| Splitting attention across formats | format-neutral waves scheduled first; per-lane evidence discipline |
| ELF breadth creep (Android, 32-bit, kernels) | explicit non-goals; `.ko` behind a gate |
| Fixture/CI coupling on Linux runners | portable drift checks (existing pattern), manifests alongside macOS |
| Divergent fact models per format | Wave 4 cross-format readiness rules; cross-format schema equality test |

## Status History

- 2026-06-12: Proposed alongside the RE Power program documentation pack.
- 2026-06-12: Accepted (D-1). Wave 5B unblocked; Wave 4 and Wave 5A start
  in parallel per roadmap §7.
