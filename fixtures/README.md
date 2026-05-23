This directory contains the small fixture corpus used by loader, CLI, and
benchmark coverage.

Fixture inventory (checked in under `fixtures/bin`):
- `arm64-symbolized`: baseline arm64 Mach-O with symbols.
- `arm64-stripped`: stripped variant of `arm64-symbolized`.
- `universal-hello`: universal Mach-O (`arm64` + `x86_64` slices).
- `arm64e-sample`: arm64e Mach-O sample used for slice-selection and arm64e parsing.
- `x86_64-only-hello`: thin x86_64 Mach-O used for unsupported-arch rejection.
- `duplicate-symbol-ordinal`: two-dylib duplicate-symbol fixture used to prove ordinal-backed attribution.
- `swift-sample`: Swift executable with stable mangled function symbols used for Swift entrypoint tests.
- `import-lazy`: helper-bearing lazy-binding fixture used for stub/helper linkage and helper-target disassembly.
- `import-rich`: arm64 Mach-O with multiple external imports/stubs.
- `export-kinds`: arm64 export-bearing fixture used for regular, weak, absolute, and thread-local export truth.
- `indirect-dispatch`: arm64 dispatch fixture used for alias-aware value-flow, table-backed function-pointer recovery, and exported-target recovery.
- `relative-dispatch`: arm64 companion dispatch fixture with 32-bit relative slot tables for relative-slot recovery into function and export targets.
- `objc-sample`: Objective-C sample used for ObjC metadata tests.
- `malformed-objc-protocol-list`: Objective-C sample variant with an inconsistent category protocol-list count for bounded parser coverage.
- `semantic-switch`: arm64 semantic-analysis fixture with literal loads, `adrp` addressing, and switch-style setup.
- `malformed-dysymtab-indirect`: intentionally broken indirect-symbol metadata fixture.
- `malformed-stub-helper-size`: malformed stub metadata fixture with invalid helper/stub sizing.
- `malformed-stub-reserved2`: malformed stub metadata fixture with broken `reserved2`/helper opcode expectations.
- `malformed-truncated`: intentionally truncated Mach-O for failure-path tests.

Sources:
- `src/hello.c` builds the symbolized/stripped/universal C fixtures.
- `src/import-rich.c` and `src/import-lazy.c` build the import/stub-heavy fixtures.
- `src/export-kinds.c` builds the export-kind fixture.
- `src/export-absolute.s` provides the absolute export companion used by `export-kinds`.
- `src/indirect-dispatch.c` builds the function-pointer and exported-target dispatch fixture.
- `src/relative-dispatch.s` builds the relative-slot dispatch fixture.
- `src/objc-sample.m` builds the Objective-C metadata fixture.
- `src/semantic-switch.c` builds the semantic-analysis fixture.
- `src/dup-alpha.c`, `src/dup-beta.c`, and `src/duplicate-symbol-main.c` build the duplicate-symbol fixture.
- `src/swift-sample.swift` builds the Swift symbol-entrypoint fixture.

Script usage:
- `fixtures/build-fixtures.sh`: rebuild fixtures (`macOS` + Xcode tooling only).
- `fixtures/build-fixtures.sh --check`: portable drift-check mode; verifies
  checked-in fixture and export-trie corpus hashes against inline manifests.
- `fixtures/build-fixtures.sh --manifest`: manifest-display mode for fixture
  hashes only.
- `fixtures/build-fixtures.sh --manifest-corpus`: manifest-display mode for
  export-trie corpus hashes only.
- `fixtures/build-fixtures.sh --manifest-all`: print both manifests with
  section headers.
- `sh fixtures/tests/build-fixtures-parity.sh`: read-only parity probes for
  `--help`, `--manifest`, `--manifest-corpus`, `--manifest-all`, `--check`,
  no-usable-hash failure, SDK probe failure, missing `clang`/`strip`,
  missing/invocation-broken `python3`, missing `nm`, and missing `swiftc`.
- `arm64e-sample` is rebuilt on a best-effort basis; if the local toolchain does
  not support `-arch arm64e`, the script preserves the checked-in binary.

Host/tool portability notes:
- Rebuild mode is intentionally macOS-only and now errors early on non-Darwin
  hosts with a clear fallback message (`--check` / manifest modes).
- Drift-check mode is host-portable and selects a SHA-256 backend from:
  `sha256sum`, `shasum`, then `openssl` (in that order), based on backend
  usability (not only command detection).
- Rebuild mode validates required tooling with a canonical probe contract:
  `xcrun` must be present, SDK resolution must succeed via
  `xcrun --show-sdk-path`, `clang`/`strip` are resolved via `xcrun --find`
  (with PATH fallback), `python3` must be invocable, `nm` must resolve to an
  executable path, and `swiftc` must resolve via `xcrun --find` or PATH.
- Temporary files/directories now use `mktemp` when available; a pid-scoped
  fallback is retained with warnings for constrained environments.

Notes:
- The checked-in binaries are parse-only artifacts used by tests and benches;
  they are never executed by the suite.
- The hash check is intended to detect fixture drift in CI and local workflows.
- CI drift and bench compatibility checks run on macOS 14, Linux x86_64, and
  Linux arm64; decode throughput runtime smoke remains Linux arm64-focused,
  while Linux x86_64 validates the explicit unsupported-host bench runtime path.
- The malformed helper fixtures are expected to fail with typed dyld metadata
  errors rather than panic or silently degrade.
- `fixtures/export-trie-corpus` contains checked-in advanced export payload
  cases (reexport/stub/malformed/unknown-bit) used for parser-proof coverage.
