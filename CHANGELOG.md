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

### Changed
- Release/operator documentation was added at the repo root.

## Beta and Nightly Notes Policy

- Beta releases should summarize user-visible behavior changes since the previous beta.
- Nightly releases are intentionally mutable and should point readers back to the current commit SHA and this changelog.
- Once public releases are enabled, stable version sections will be added below `Unreleased`.
