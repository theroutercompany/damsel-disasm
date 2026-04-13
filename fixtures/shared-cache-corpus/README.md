# Synthetic dyld shared-cache corpus

This corpus contains deterministic synthetic fixtures for Wave 1/2/3 shared-cache
parser, projection, linkage, and symbolication testing.

These files are intentionally small and portable. They are not full Apple
system cache dumps and must not be treated as production cache artifacts.

CI drift for this corpus is enforced via `./fixtures/build-fixtures.sh --check`.

Fixture inventory:

- `valid-single-arm64.cache`
  - Single-member arm64 cache set baseline.
- `valid-split-arm64.cache`
  - Split-cache root member baseline.
- `valid-split-arm64.cache.1`
  - Required numbered subcache member for split-cache tests.
- `valid-split-arm64.cache.symbols`
  - Optional local-symbol sidecar variant.
- `unsupported-arch-x86_64.cache`
  - Cache metadata declares unsupported architecture.
- `malformed-header.cache`
  - Header corruption fixture.
- `malformed-mapping-table.cache`
  - Mapping table corruption fixture.
- `malformed-image-table.cache`
  - Image table corruption fixture.
- `missing-subcache-root.cache`
  - Root that advertises a missing numbered subcache member.
- `ambiguous-basename.cache`
  - Image inventory with basename collisions for ambiguity tests.
- `internal-linkage-arm64.cache`
  - Internal dependency/importer/provider fixture for cache-native linkage queries.
- `local-symbols-present-arm64.cache`
  - Cache root where local symbols are expected to be available.
- `local-symbols-present-arm64.cache.symbols`
  - Sidecar for local-symbol-available variant.
- `local-symbols-absent-arm64.cache`
  - Cache root where no symbols sidecar is available.
- `reexport-linkage-arm64.cache`
  - Reexport-focused fixture for provider and reexport resolution tests.
