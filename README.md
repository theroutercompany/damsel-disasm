# damsel

`damsel` is a static Mach-O analysis CLI focused on Apple Silicon binaries.

Current analysis scope:
- Implemented today: standalone Mach-O parsing plus Wave 1/2/3 dyld shared-cache inventory, symbolication, projected-image analysis, and cache-native linkage exploration.
- Thin and universal Mach-O coverage for `arm64` and `arm64e`.
- Structured views for binary info, sections, symbols, imports, relocations, slices, dyld metadata, Objective-C metadata, and AArch64 disassembly.
- Shared-cache coverage is container-first and exposed through a dedicated `cache` command family:
  - `cache info`
  - `cache images`
  - `cache image`
  - `cache exports`
  - `cache lookup-address`
  - `cache resolve-symbol`
  - `cache sections`
  - `cache symbols`
  - `cache imports`
  - `cache dyld`
  - `cache objc`
  - `cache disasm`
  - `cache image-deps`
  - `cache dependents`
  - `cache symbol-providers`
  - `cache symbol-importers`
  - `cache reexports`
- A compatibility-oriented `doctor` command that reports host/tool readiness and supports CI/operator threshold checks.

Planned next:
- richer graph/trace workflows on top of the current point-query cache explorer
- deeper system-framework spelunking and cross-image navigation
- any future live-debugger integration remains deferred and will be documented separately
- Contract and architecture for those next waves are documented in:
  - [docs/specs/dyld-shared-cache-v1.md](./docs/specs/dyld-shared-cache-v1.md)
  - [docs/architecture/apple-runtime-analysis-architecture.md](./docs/architecture/apple-runtime-analysis-architecture.md)
  - [docs/roadmaps/apple-runtime-analysis-roadmap.md](./docs/roadmaps/apple-runtime-analysis-roadmap.md)

The project is still private and is not being released publicly yet. This repo currently ships private GitHub prereleases only.

## Support Matrix

| Host | Analysis | Fixture rebuild | Drift check | Bench compile | Bench runtime |
| --- | --- | --- | --- | --- | --- |
| macOS (`arm64` / `x86_64`) | Supported | Supported | Supported | Supported | Supported with degraded features |
| Linux `x86_64` | Supported | Unsupported | Supported | Supported | Supported with degraded features |
| Linux `arm64` | Supported | Unsupported | Supported | Supported | Supported |
| Windows `x86_64` | Supported with degraded features | Unsupported | Supported | Supported with degraded features | Supported with degraded features |
| Unknown hosts | Supported with degraded features | Unsupported | Supported | Supported with degraded features | Supported with degraded features |

Important compatibility notes:
- Fixture rebuild is intentionally macOS-only and requires Xcode tooling.
- Throughput-oriented bench runtime proof is intentionally Linux `arm64`-only.
- Windows and unknown hosts are intentionally treated as degraded, not primary supported release targets.

## Private Release Channels

The release pipeline prepares one private artifact target in this pass:
- `damsel` macOS arm64 archive

Release lanes:
- `nightly`: one mutable private prerelease named `nightly`, updated by schedule or manual recovery.
- `beta`: semver prerelease tags such as `v0.1.0-beta.1`, published as private GitHub prereleases.

No public/stable release, package-manager integration, or crates.io publishing is enabled in this pass.

## Installing From a Private macOS arm64 Release

1. Open the private GitHub Release for the desired channel.
2. Download the archive named like `damsel-nightly-macos-arm64.tar.gz` or `damsel-v0.1.0-beta.1-macos-arm64.tar.gz`, plus the matching `.sha256` file.
3. Extract it:

```sh
shasum -a 256 -c damsel-nightly-macos-arm64.tar.gz.sha256
tar -xzf damsel-nightly-macos-arm64.tar.gz
cd damsel-nightly-macos-arm64
```

4. Smoke-test the packaged binary:

```sh
./damsel --help
./damsel doctor --help
./damsel cache --help
```

The packaged binary inside the archive is named `damsel`.

## Local Development

Useful verification commands:

```sh
cargo test --workspace
./fixtures/build-fixtures.sh --check
./fixtures/tests/build-fixtures-parity.sh
cargo bench -p damsel-macho --bench decode_bench --no-run
```

Optional local real-cache validation:

```sh
DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT=/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e \
  cargo test -p damsel-macho --test shared_cache_real_env -- --nocapture
```

For fixture details and host/tool parity notes, see [fixtures/README.md](./fixtures/README.md).

For Apple runtime direction beyond Wave 1, see [docs/](./docs/).

## Release Operations

Operator-facing release details live in [RELEASE.md](./RELEASE.md).

Public licensing is intentionally deferred until the repo is ready for a public release.
