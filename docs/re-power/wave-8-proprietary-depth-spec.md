# Wave 8 — Proprietary-Target Depth (Gated)

Status: Proposed implementation spec for Wave 8 of the
[Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md).

Gate rule: no Wave 8 work starts until Wave 7 exits. Each item below has its
own additional gate. Wave 8 is the phase's "after the loop is fast, deepen
the specialties" wave — not a backlog of unrelated wishes.

## Why This Wave Exists

Proprietary binaries fight back in specific ways: they strip symbols, vendor
whole SDKs, obfuscate Swift, abuse arm64e pointers, and stack anti-analysis
checks. Waves 4–7 give damsel the loop; Wave 8 gives it the specialized
levers that turn hours into minutes on those targets.

## Consumed Specifications (Normative)

- [Patch studio, assembler spec](../hopper-gap/patch-studio-assembler-workbench-implementation-spec.md)
  and [patching/hex/transaction spec](../hopper-gap/patching-hex-assembly-transaction-implementation-spec.md)
- [Apple SDK semantic packs spec](../hopper-gap/apple-sdk-semantic-packs-implementation-spec.md)
- [Swift runtime metadata spec](../hopper-gap/swift-runtime-metadata-implementation-spec.md)
- [Type, signature, symbolication spec](../hopper-gap/type-signature-symbolication-implementation-spec.md)
- [DWARF/dSYM/KDK spec](../hopper-gap/dwarf-dsym-kdk-debug-info-implementation-spec.md)
- Wave 4–7 specs — kernel transactions, semantics, decompile contract, recon
  evidence discipline.

## Item 8.1 — Patch Studio MVP

Gate: Wave 7 exit **and** a written safety review covering data loss,
code-signing breakage, and cache/kernel mutation refusal **and** an
explicit maintainer re-decision to leave read-only mode (Decision D-5,
2026-06-12: damsel stays read-only until that re-decision happens; this
item is fully planned but dormant).

Scope:

- Transactional byte patches over a project: preview (before/after bytes +
  disassembly re-decode), apply, undo/redo via kernel transactions,
  checksums, copy-on-write output file; source file never modified in place.
- **Bounded encoder vocabulary instead of a general assembler** (licensing
  decision, see §Risks):
  - arm64/arm64e: NOP, RET, unconditional B/BL offset rewrite, MOVZ/MOVK
    immediate materialization, conditional branch rewrite within range,
    PAC-placeholder insertion where the pattern is exact.
  - x86_64: byte patches via the `iced-x86` encoder (MIT — license and
    encoder API live-verified 2026-10-28: `Encoder`, `BlockEncoder`, and a
    `CodeAssembler` instruction assembler exist in 1.21.0) — NOP, RET, short/
    near JMP/Jcc within range, MOV imm32/64 — the same bounded vocabulary.
  - Anything outside the vocabulary: typed `patch_unsupported` error with a
    suggested manual route; never a silent wrong encoding.
- Fixture patches must round-trip: patch → reanalyze → patched fact set
  matches expectation; disassembly re-decode proves the patch.
- Code-signature warning surface: any patch on a signed Mach-O flags the
  signature invalid (no re-signing in this wave); ELF patches flag build-id
  drift.
- Hard refusals: projected shared-cache sources and kernel collections
  (read-only by design), files without write permission, and any source
  whose content hash changed since analysis (`stale_source`).
- CLI: `patch preview`, `patch apply`, `patch revert`; Workbench hex view
  is read-only in this wave except preview hover states (the full patch
  studio Workbench UX is deferred to the Apple track spec once safety work
  lands).

Out: re-signing, cache/kernel patches, in-place writes, general assembly,
hooks/trampolines, anti-cheat adversarial scenarios.

## Item 8.2 — Symbol Reuse At Scale

Gate: Wave 7 exit; evidence sweep proving name-match precision on a labeled
corpus.

Surfaces:

- **Apple SDK semantic packs** (consumes the existing spec): SDK/header/
  XNU/objc4/dyld/Swift ABI declaration sets matched against unknown imports,
  stubs, selectors, and functions; packs are versioned and permissive to
  load; matches are `inferred` facts with pack provenance.
- **Linux symbol corpora** (new): glibc/libstdc++/libcrypto versioned-symbol
  registries derived from distro debuginfo (build-id → name/type map);
  matching by import name + version, function fingerprints (structural hash
  against known implementations — e.g. reused `glibc` static code), and
  string/constant signatures.
- **Static-link reuse detection**: a proprietary static binary containing
  documented OSS libraries gets those regions named automatically (fingerprint
  matching against corpus builds), reported as `inferred` with score and
  evidence — huge for triage of stripped Linux binaries.
- Acceptance: on the labeled corpus, import/stub naming precision ≥ 95 % at
  ≥ 50 % recall of the nameable set; static-library region detection
  precision ≥ 85 %; every match carries evidence lines (score components),
  and matches below threshold are emitted as `candidate` problems, not
  names.

Out: uploading binaries anywhere; corpora are local/offline; no cloud
matching.

## Item 8.3 — Swift Runtime Metadata And Apple Deep Facts

Gate: Wave 7 exit; consume the Swift metadata spec's milestones.

Scope (delegated to the Apple spec for detail):

- Swift section inventory, demangler service, type refs, field metadata,
  protocols/conformances, witness/accessor classification, Swift↔ObjC bridge
  records.
- All facts land in the Wave 4 kernel with provenance; the decompiler (Wave 6
  pipeline) renders Swift-ish names and types where evidence exists, with
  markers.
- PAC/arm64e navigation polish on shared caches: authenticated pointer
  resolution feeding xrefs (consumes hopper-gap M7).

Out: Swift source reconstruction, SIL-level analysis, ObjC runtime
simulation.

## Item 8.4 — Anti-Analysis Catalog

Gate: Wave 7 exit; each detection has a labeled fixture and an
evidence-line report.

- A catalog crate (`damsel-recon-catalog`) of named detections: anti-debug
  (`ptrace`, `sysctl`, `task_get_exception_ports`, timing), anti-VM
  (hypervisor leaf checks, IOKit device probes), string encryption
  (XOR/add-loop over data feeding API calls), control-flow flattening,
  opaque predicates, junk-code insertion, syscall stubs, dyld-cache abuse
  (targeting system images as code caves).
- Each detection: code, description, severity, evidence extraction (address,
  instruction window, constants), confidence, and remediation hint
  ("jump table at X", "constant condition at Y").
- Catalog output integrates with `recon` (Wave 7) and with diff (new
  findings class `defense_added` when a vendor adds a check between
  versions) — the version-archaeology payoff.

## Sequencing Within The Wave

1. 8.2 symbol reuse (biggest immediate triage win, no mutation risk).
2. 8.4 anti-analysis catalog (feeds recon/diff, low risk).
3. 8.3 Swift/PAC depth (Apple track interleave).
4. 8.1 patch studio last (highest risk; safety review first).

## Budgets

| Surface | Budget |
| --- | --- |
| Symbol-corpus match, 30 MB binary | ≤ 20 s |
| Patch preview + re-decode | ≤ 500 ms |
| Catalog scan (static) | ≤ 10 s |
| Swift metadata pass (typical framework) | ≤ 2 s |

## Risks

| Risk | Mitigation |
| --- | --- |
| **Assembler licensing and weight**: Keystone is GPL-2 (license-blocked for a closed-source product); LLVM-MC is permissive but drags in a C++ toolchain dependency | bounded encoder vocabulary + iced-x86 (MIT); decision recorded here, reviewed again if scope grows |
| Patch data loss / signing breakage | transactional writes, CoW export, hard refusals, signature warnings, safety review gate |
| Symbol-corpus false names | evidence scores, thresholds, `candidate` problems, precision gates |
| Catalog false positives | labeled fixtures, wording rules ("indicated", not "confirmed"), evidence lines |
| Corpora licensing/provenance | record corpus sources and licenses in the catalog; ship only redistributable derived data |

## Open Questions

1. Corpora storage format and update cadence (versioned sidecar packs vs
   built-in): likely sidecar packs consistent with the SDK semantic packs
   precedent.
2. Whether iced-x86 replaces Capstone x86 decode later — out of scope now;
   encoder-only dependency keeps the risk small.
3. Patch studio Workbench UX ownership: this wave ships CLI + preview; the
   patch-studio spec owns the full UI (Apple track).

## References

- [Reverse-Engineering Power Roadmap](../roadmaps/re-power-roadmap.md) §5 Wave 8
- [Patch studio spec](../hopper-gap/patch-studio-assembler-workbench-implementation-spec.md)
- [Patching/hex/transaction spec](../hopper-gap/patching-hex-assembly-transaction-implementation-spec.md)
- [SDK semantic packs spec](../hopper-gap/apple-sdk-semantic-packs-implementation-spec.md)
- [Swift metadata spec](../hopper-gap/swift-runtime-metadata-implementation-spec.md)
- [Type/symbolication spec](../hopper-gap/type-signature-symbolication-implementation-spec.md)
- [DWARF/dSYM/KDK spec](../hopper-gap/dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Research Sources](research-sources.md) §3 (licensing and library facts)
