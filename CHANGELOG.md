# Changelog

This project is still private. Public release notes are not published yet.

GitHub prerelease conventions:
- `nightly` is a mutable private prerelease lane used for continuous internal validation.
- `beta` releases are semver-tagged private prereleases such as `v0.1.0-beta.1`.
- This changelog tracks user-visible changes that should survive beyond a single nightly.

## [Unreleased]

### Added
- Cross-platform analysis: ELF64 loading (Linux) alongside Mach-O (macOS).
- x86_64 disassembly and analysis with a shared pipeline for both ISAs.
- Project database (`damsel-project`): SQLite-backed analysis persistence
  with byte-deterministic canonical exports.
- Whole-image function recovery with provenance-ranked seeds and bounds.
- Cross-reference index queryable from both directions (`xrefs --to/--from`).
- String inventory with xref joins and classification heuristics.
- Unified search across functions, symbols, imports, and strings.
- Decompiler pipeline: D0 summaries, D1 IR (arm64+x86_64), D2 SSA,
  D3 stack frames, D4 structuring (diamonds, loops), D5 pseudocode.
- `damsel summary` and `damsel decompile` commands with Markdown export.
- Binary recon triage: entropy profiles, packer signatures, anti-analysis
  indicators, CFG-flattening detection, opaque-predicate detection.
- Binary diffing with function-level matching (symbol + address strategies).
- Batch analysis with JSONL output and per-target typed failures.
- MCP server (`damsel serve`) with 7 typed tools for AI agents.
- Web UI project functions panel and pseudocode viewer.
- Evidence packaging (`bin/elf-evidence-package.sh`) with self-verification.
- Function recovery audit gate (`bin/function-recovery-audit.sh`).
- Versioned symbols (.gnu.version) on both section-ful and segment-only paths.
- Dynamic relocations (PLT/GOT) as typed binding facts.
- Segment-only PT_DYNAMIC walker for hard-stripped ELF binaries.
- Build-id resolution on both parsing paths.
- ARM64 semantics module shared between the engine and the decompiler.
- ABI type seeds (arm64 AAPCS64 + x86_64 SysV argument/return roles).
- Cross-block phi-SSA register merging.
- Stack-frame recovery (frame size, saved registers, local slots).
- Indexed-memory modeling (base + index * scale + displacement).
- Segment-register TLS access (fs:/gs:).
- Memory-destination arithmetic as read-modify-write.
- xchg, imul, movzx/movsx, cdq, leave, movs, stos instruction modeling.
- Private GitHub release hardening with nightly and beta lanes.
- Release validation, packaging, checksum, and notes-generation helpers.
- Disassembler validation harnesses for LLVM fixture comparison and local
  real-cache smoke coverage.
- One-command disassembler v2 alpha validation sweep for local operators.
- The alpha validator can write a real-cache evidence archive with
  `--real-cache-archive`.
- Manual GitHub-hosted macOS real-cache evidence collection workflow for
  producing downloadable beta evidence archives.
- Manual Blacksmith macOS real-cache evidence workflow for independent hosted
  Apple Silicon macOS 15/26 evidence collection.
- Public-beta evidence package auditing is integrated into the alpha validation
  sweep via repeatable `--beta-evidence` inputs.
- Portable self-test for the disassembler v2 alpha validation sweep.
- Machine-readable JSON Lines reporting for real-cache smoke evidence.
- Real-cache evidence audit helper for public-beta breadth checks.
- Evidence packaging and verification helper for moving real-cache smoke reports
  between hosts.
- Evidence packages now self-verify after creation and can emit a transfer
  archive with `--archive`.
- Real-cache evidence verification and audit accept verified package archives,
  package directories, raw JSONL reports, and downloaded GitHub artifact zip
  wrappers.
- Portable self-tests for the real-cache evidence packaging and audit gates.

### Changed
- Release/operator documentation was added at the repo root.
- Real shared-cache smoke validation now samples multiple projected images per
  unique cache UUID instead of only the first image.

### Fixed
- Mach-O indirect-symbol attribution now treats library ordinals as one-based,
  handles self-prefixed library lists, and tolerates duplicate same-dylib import
  rows in projected dyld-cache images.

## Beta and Nightly Notes Policy

- Beta releases should summarize user-visible behavior changes since the previous beta.
- Nightly releases are intentionally mutable and should point readers back to the current commit SHA and this changelog.
- Once public releases are enabled, stable version sections will be added below `Unreleased`.
