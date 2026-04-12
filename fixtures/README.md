This directory contains the small fixture corpus used by loader, CLI, and
benchmark coverage.

Fixture inventory (checked in under `fixtures/bin`):
- `arm64-symbolized`: baseline arm64 Mach-O with symbols.
- `arm64-stripped`: stripped variant of `arm64-symbolized`.
- `universal-hello`: universal Mach-O (`arm64` + `x86_64` slices).
- `arm64e-sample`: arm64e Mach-O sample used for slice-selection and arm64e parsing.
- `x86_64-only-hello`: thin x86_64 Mach-O used for unsupported-arch rejection.
- `duplicate-symbol-ordinal`: two-dylib duplicate-symbol fixture used to prove ordinal-backed attribution.
- `import-lazy`: helper-bearing lazy-binding fixture used for stub/helper linkage and helper-target disassembly.
- `import-rich`: arm64 Mach-O with multiple external imports/stubs.
- `objc-sample`: Objective-C sample used for ObjC metadata tests.
- `semantic-switch`: arm64 semantic-analysis fixture with literal loads, `adrp` addressing, and switch-style setup.
- `malformed-dysymtab-indirect`: intentionally broken indirect-symbol metadata fixture.
- `malformed-stub-helper-size`: malformed stub metadata fixture with invalid helper/stub sizing.
- `malformed-stub-reserved2`: malformed stub metadata fixture with broken `reserved2`/helper opcode expectations.
- `malformed-truncated`: intentionally truncated Mach-O for failure-path tests.

Sources:
- `src/hello.c` builds the symbolized/stripped/universal C fixtures.
- `src/import-rich.c` and `src/import-lazy.c` build the import/stub-heavy fixtures.
- `src/objc-sample.m` builds the Objective-C metadata fixture.
- `src/semantic-switch.c` builds the semantic-analysis fixture.
- `src/dup-alpha.c`, `src/dup-beta.c`, and `src/duplicate-symbol-main.c` build the duplicate-symbol fixture.

Script usage:
- `fixtures/build-fixtures.sh`: rebuild fixtures on macOS with Xcode installed.
- `fixtures/build-fixtures.sh --check`: verify checked-in fixture hashes against
  the inline manifest in the script (drift check).
- `fixtures/build-fixtures.sh --manifest`: print the current inline fixture
  manifest.
- `arm64e-sample` is rebuilt on a best-effort basis; if the local toolchain does
  not support `-arch arm64e`, the script preserves the checked-in binary.

Notes:
- The checked-in binaries are parse-only artifacts used by tests and benches;
  they are never executed by the suite.
- The hash check is intended to detect fixture drift in CI and local workflows.
- The malformed helper fixtures are expected to fail with typed dyld metadata
  errors rather than panic or silently degrade.
