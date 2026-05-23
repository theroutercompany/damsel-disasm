# Changelog

This project is still private. Public release notes are not published yet.

GitHub prerelease conventions:
- `nightly` is a mutable private prerelease lane used for continuous internal validation.
- `beta` releases are semver-tagged private prereleases such as `v0.1.0-beta.1`.
- This changelog tracks user-visible changes that should survive beyond a single nightly.

## [Unreleased]

### Added
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
