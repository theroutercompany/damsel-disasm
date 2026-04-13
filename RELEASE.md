# Release Runbook

This repo is currently configured for **private GitHub Releases only**.

What is enabled in this pass:
- one macOS arm64 CLI release artifact
- one mutable nightly prerelease named `nightly`
- semver-tagged beta prereleases such as `v0.1.0-beta.1`
- focused release-time verification
- checksum generation
- signing/notarization scaffolding that is conditional on Apple secrets

What is intentionally not enabled yet:
- public/stable releases
- crates.io publishing
- Windows or Linux binary release targets
- Homebrew, installers, or package-manager integrations
- notarization/signing enforcement as a hard requirement
- a public license and public support commitments

## Channel Behavior

### Nightly

- Triggered by GitHub Actions `schedule` and `workflow_dispatch`.
- Publishes or updates a single mutable private prerelease tagged `nightly`.
- Intended for continuous internal validation and manual recovery builds.
- Nightly release notes must include:
  - commit SHA
  - UTC build time
  - artifact name
  - signing state (`unsigned` unless release secrets are configured)
  - support matrix reminder

### Beta

- Triggered by pushing a tag that matches `vX.Y.Z-beta.N`.
- Tag must match the workspace version declared in the root `Cargo.toml`:
  - workspace version `0.1.0`
  - valid beta tags `v0.1.0-beta.1`, `v0.1.0-beta.2`, etc.
- Each beta tag produces a new immutable private prerelease.
- Beta release notes must include:
  - tag version
  - commit SHA
  - macOS arm64 install instructions
  - host/degraded-support notes

## Operator Flow

### Create a beta release

1. Confirm the root workspace version in `Cargo.toml`.
2. Create a matching beta tag, for example:

```sh
git tag v0.1.0-beta.1
git push origin v0.1.0-beta.1
```

3. Wait for `.github/workflows/release.yml` to:
   - validate the tag/version contract
   - run focused verification
   - build the macOS arm64 artifact
   - package archive + checksum
   - publish the private prerelease

### Trigger a nightly release manually

1. Open the `release` workflow in GitHub Actions.
2. Use `Run workflow` on the desired branch or recovery ref.
3. The workflow will update the existing `nightly` prerelease instead of creating a new permanent release.

## Apple Signing / Notarization Secrets

The release workflow is designed to publish **unsigned** artifacts when Apple secrets are absent. When the full Apple secret set is present, the workflow can sign the packaged binary and perform notarization submission.

Expected secret names:
- `APPLE_CERTIFICATE_P12_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_APP_SPECIFIC_PASSWORD`
- `APPLE_TEAM_ID`

Behavior:
- if the required secret set is missing, release publishing continues with an unsigned artifact and the release notes explicitly say so
- if the secret set is present, the workflow imports the signing certificate into a temporary keychain, signs the `damsel` binary, prepares a notarization submission container, submits via `notarytool`, waits for the result, and republishes the signed archive

Current limitation:
- the current private release artifact is a `tar.gz` archive containing a raw CLI binary
- that format is acceptable for internal/private distribution, but it is not the ideal long-term container for a strict public notarization/stapling policy
- a later public-release pass may switch to a DMG/PKG or add an additional notarization-friendly container if enforced stapling becomes required

## Focused Release Verification

The release workflow must run this verification subset before publishing:

```sh
cargo test -p damsel-cli
cargo test -p damsel-core
cargo test -p damsel-macho --tests
./fixtures/build-fixtures.sh --check
./fixtures/tests/build-fixtures-parity.sh
```

Release-time smoke checks on the built/package binary:

```sh
./damsel --help
./damsel doctor --help
./damsel cache --help
```

Wave 1 note:
- `cache --help` and `cache info fixtures/shared-cache-corpus/valid-single-arm64.cache` are mandatory release smokes.
- release smoke also runs one projected-image command against the synthetic corpus:
  - `cache sections fixtures/shared-cache-corpus/valid-single-arm64.cache /usr/lib/libobjc.A.dylib --exec`
- Release verification keeps Wave 1 scope narrow:
  - read-only inspection/query only
  - Apple Silicon shared-cache sets only
  - no debugger transport
  - no cache mutation or rebuilding
- The implementation contract and follow-on roadmap live in:
  - `docs/specs/dyld-shared-cache-v1.md`
  - `docs/architecture/apple-runtime-analysis-architecture.md`
  - `docs/roadmaps/apple-runtime-analysis-roadmap.md`

## Future Toggle Points

These are intentionally deferred:
- stable public semver tags and public GitHub Releases
- notarization/signing as a mandatory gate
- additional binary targets (Linux / Windows)
- crates.io publish metadata and publish workflow
- package-manager and installer distribution
- final public license decision
