# Disassembler V2 Alpha Validation

Status: operator checklist for alpha candidates.

This matrix records the checks expected before treating the disassembler v2
features as alpha-ready. The required CI path stays fixture-only and portable;
real dyld shared-cache checks are opt-in because cache paths, OS versions, and
Apple developer tooling vary by host.

## Required Fixture Matrix

Run these before cutting an alpha build:

```sh
bin/disassembler-v2-alpha-validate.sh --require-external
```

When public-beta evidence packages exist, pass them to the same validator:

```sh
bin/disassembler-v2-alpha-validate.sh --require-external \
  --beta-evidence /tmp/damsel-real-cache-evidence \
  --beta-evidence /path/to/other-host-evidence
```

Hosted independent evidence can also be collected with the manual
`real-cache evidence` GitHub Actions workflow. The workflow is intentionally
manual and read-only: it runs on a selected arm64 macOS hosted runner, writes a
verified `.tar.gz` evidence archive, and uploads it with `actions/upload-artifact`.
Downloaded artifact `.zip` wrappers can be passed directly to package
verification, `--beta-evidence`, and the evidence audit; the tools require the
zip to contain exactly one evidence `.tar.gz`/`.tgz` archive.

If the repository has the Blacksmith GitHub App installed, the manual
`blacksmith real-cache evidence` workflow runs the same archive collection path
on Blacksmith Apple Silicon macOS runners. Use `blacksmith-6vcpu-macos-15` as
the preferred independent-host evidence lane; macOS 26 Blacksmith labels are
available for same-release hosted confirmation.

The validator wraps this fixture-backed matrix:

```sh
bin/disassembler-v2-alpha-validate-test.sh
cargo test --workspace
./fixtures/build-fixtures.sh --check
sh ./fixtures/tests/build-fixtures-parity.sh
bin/shared-cache-evidence-package-test.sh
bin/shared-cache-evidence-audit-test.sh
```

Coverage expected from the checked-in corpus:

| Area | Fixture or test | Expected evidence |
| --- | --- | --- |
| CFG and summary | `semantic-switch`, `damsel-macho/tests/disasm_matrix.rs` | Multiple basic blocks, direct edges, fallthrough edges, data refs, recovered values, jump-table summary |
| Cache links | `internal-linkage-arm64.cache`, `damsel-cli/tests/cli_json.rs` | Import rows resolve to provider images and stable `cache_links` JSON |
| Value-flow safety | `damsel-macho` unit tests | Call-clobbered registers stop flowing past calls while callee-saved registers survive |
| ObjC entrypoints | `objc-sample`, `damsel-cli/tests/cli_json.rs` | `--objc-owner` plus `--objc-selector` resolves to an implementation address |
| Swift entrypoints | `swift-sample`, `damsel-cli` tests | Exact Mach-O mangled names resolve end-to-end without external tools; demangled display and ambiguity paths are deterministic |
| Text output | `damsel-cli/tests/cli_snapshots.rs` | Representative `--analysis` summary and graph text is snapshot-covered |
| UI payload | `damsel-cli/tests/ui_api.rs` | `/api/images/{id}/disasm` returns analysis and the shell exposes graph navigation code |
| Beta evidence package | `bin/shared-cache-evidence-package-test.sh` | Report packaging emits `evidence.jsonl`, `manifest.json`, `smoke.log`, `README.txt`, verified SHA-256 checksums, an optional verified transfer archive, and direct verification of downloaded artifact zip wrappers |
| Beta evidence gate | `bin/shared-cache-evidence-audit-test.sh` | Public-beta audit accepts independent evidence as reports, package directories, package archives, or downloaded artifact zips, and rejects single-host, duplicate-cache, failed-cache, low-sample, unsafe-archive, unsafe-zip, and malformed-report cases |

## Optional Real-Cache Matrix

Set `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT` to a dyld shared-cache root or member
path and run the opt-in smoke directly:

```sh
DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT=/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e \
  cargo test -p damsel-macho --test shared_cache_real_env -- --nocapture
```

The smoke must project a deterministic sample of real cache images, find a
file-backed executable section that decodes, build a graph summary from that
instruction window, resolve at least one export address through cache lookup,
and exercise dependency/reexport queries. The default sample is six
disassemblable images per unique cache UUID.

For broader local coverage, prefer the operator harness. It probes common local
macOS cache paths, records cache UUIDs before running tests, exports the sample
and instruction limits, and skips duplicate UUIDs by default:

```sh
bin/shared-cache-real-smoke.sh
```

For public-beta evidence collection on another host or macOS release, also
write a self-verified portable evidence package archive through the same
validator:

```sh
bin/disassembler-v2-alpha-validate.sh \
  --require-external \
  --real-cache-archive /tmp/damsel-real-cache-evidence.tar.gz
bin/shared-cache-evidence-package.sh --verify /tmp/damsel-real-cache-evidence.tar.gz
```

The validator derives `/tmp/damsel-real-cache-evidence` as the package directory
for this archive path, runs real-cache smoke packaging, verifies the unpacked
package, verifies the archive, and then feeds the archive into the public-beta
evidence audit. The package includes `evidence.jsonl`, `manifest.json`,
`smoke.log`, `README.txt`, and `SHA256SUMS`; package creation runs the same
verification as `--verify`, checking required files, checksums, manifest
consistency, JSONL parseability, exactly one host record, and at least one passed
cache record.
When `--verify` receives a package archive, it first rejects unsafe archive
entries such as absolute paths, parent traversal, links, or unexpected member
types, then verifies the extracted package. The report records host metadata,
cache UUIDs,
duplicate-UUID skips, sampled projected images, skipped image reasons, and
summary counts. Beta evidence should include at least one report whose host OS
release or cache UUID set is independent from the current local macOS 26.5 run.

Audit one or more packages before claiming public-beta breadth. The audit
accepts package archives, downloaded artifact zips, package directories, or raw
`evidence.jsonl` files; archives, artifact zips, and directories are verified
before their embedded reports are audited.

```sh
bin/shared-cache-evidence-audit.sh /tmp/damsel-real-cache-evidence.tar.gz /path/to/other-host-evidence.tar.gz
```

The default audit requires at least two independent host/release evidence keys,
two unique passed cache UUIDs, and six sampled projected images per passed
cache. A single current-host report is expected to fail this audit.

## External Tool Comparison

When available on the host, compare representative fixture windows against
Apple/LLVM tools:

```sh
bin/disasm-external-compare.sh
```

Acceptance is not byte-for-byte text parity. Damsel should agree on decoded
instruction addresses, mnemonics, and direct branch/call targets while adding
higher-level annotations that `llvm-objdump` and `llvm-otool` do not provide.

## Release Note Requirements

Every alpha or beta note should state:

- The exact fixture commands that passed.
- Whether `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT` was run, including OS/cache
  identity when available.
- Whether `llvm-objdump` or `llvm-otool` comparison was run.
- Any known gaps in deeper Swift metadata recovery, indirect target precision,
  or real-cache breadth.

## Current Local Evidence

Last amended: 2026-05-22 on macOS 26.5 arm64. Full fixture/workspace
evidence below was collected during the v2 alpha sweep; narrower revalidation
bullets name their exact command.

- `cargo test --workspace`: passed.
- `bin/disassembler-v2-alpha-validate.sh --require-external`: passed.
- `bin/disassembler-v2-alpha-validate.sh --skip-workspace-tests --skip-fixtures --skip-bench --skip-ui --skip-external --real-cache-output <tmp-package>`:
  passed script, package, release-metadata, and real-cache package verification,
  then failed the public-beta evidence audit for the expected single-host reason:
  `independent host/release evidence is insufficient: 1 < 2; observed=26.5/25F71/macbook`.
- `bin/disassembler-v2-alpha-validate.sh --skip-workspace-tests --skip-fixtures --skip-bench --skip-ui --skip-external --beta-evidence <tmp>.tar.gz`:
  accepted a current-host package archive through the top-level validator and
  failed for the same expected single-host public-beta evidence reason.
- `bin/disassembler-v2-alpha-validate.sh --skip-workspace-tests --skip-fixtures --skip-bench --skip-ui --skip-external --real-cache-archive <tmp>.tar.gz`:
  derived a package directory from the archive path, produced and verified both
  the current-host package and archive, audited the archive as the public-beta
  evidence input, then failed for the same expected single-host public-beta
  evidence reason.
- `.github/workflows/real-cache-evidence.yml`: added a manual hosted-runner
  collection workflow for `macos-15`/`macos-14` arm64 evidence archives; the
  release-readiness workflow contract validates the dispatch trigger, selected
  runner input, read-only permission, and artifact upload path.
- `.github/workflows/blacksmith-real-cache-evidence.yml`: added a manual
  Blacksmith macOS evidence workflow for `blacksmith-6vcpu-macos-15`,
  `blacksmith-12vcpu-macos-15`, and macOS 26/later labels; the
  release-readiness workflow contract validates the dispatch trigger, selected
  runner input, read-only permission, and artifact upload path.
- `bin/disassembler-v2-alpha-validate-test.sh`: passed, covering help output,
  fast skip-path execution, public-beta evidence audit wiring with two
  synthetic packages including a downloaded-artifact-zip wrapper, invalid
  option combinations, missing option values, and unknown arguments.
- `cargo test -p damsel-cli`: passed, including Swift symbol target unit tests,
  the checked-in `swift-sample` end-to-end test, and `disasm_analysis_snapshot`.
- `cargo test -p damsel-macho synthesize_analysis_references_clears_ambiguous_join_state -- --nocapture`:
  passed.
- `./fixtures/build-fixtures.sh --check`: passed with `swift-sample` in the
  inline fixture manifest.
- `sh ./fixtures/tests/build-fixtures-parity.sh`: passed.
- `cargo bench -p damsel-macho --bench decode_bench --no-run`: passed.
- `node --check damsel-cli/ui/app.js`: passed.
- `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT=/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e cargo test -p damsel-macho --test shared_cache_real_env -- --nocapture`:
  passed against cache UUID `46e0097ff38536c884a9a40d315a32d1`, arm64e,
  3646 images, 13 members, six sampled projected images. The sampled set
  included `libSystem.B.dylib`, `libobjc.A.dylib`, `libdispatch.dylib`,
  `CoreFoundation`, `Foundation`, and `CoreNameParser`; one code-less projected
  image was skipped with an explicit reason.
- `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT=/System/Volumes/Preboot/Cryptexes/Incoming/OS/System/Library/dyld/dyld_shared_cache_arm64e cargo test -p damsel-macho --test shared_cache_real_env -- --nocapture`:
  passed, but `cache info` reported the same cache UUID as the active OS cache,
  so this does not count as independent cache-breadth evidence.
- `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT=/System/Volumes/Preboot/Cryptexes/OS/System/DriverKit/System/Library/dyld/dyld_shared_cache_arm64e cargo test -p damsel-macho --test shared_cache_real_env -- --nocapture`:
  passed against cache UUID `78d1d79923e93777ad5f7efe2284be27`, arm64e,
  58 images, 2 members, six sampled projected images.
- `bin/shared-cache-real-smoke.sh`: passed, validating the two unique cache
  UUIDs above with six sampled images per UUID and skipping duplicate
  `Incoming/OS` cache paths by UUID.
- `bin/shared-cache-real-smoke.sh --report <tmp>.jsonl`: passed across the main
  OS and DriverKit cache UUIDs above, produced parseable JSON Lines evidence,
  and skipped duplicate `Incoming/OS` cache paths by UUID.
- `bin/shared-cache-evidence-audit.sh <current-host-report>.jsonl`: failed as
  expected because the current evidence has only one independent host/release
  key.
- `bin/shared-cache-evidence-audit.sh <current-host-report>.jsonl <synthetic-independent-report>.jsonl`:
  passed, proving the audit accepts two independent host/release evidence keys
  and two unique cache UUIDs.
- `bin/shared-cache-evidence-package-test.sh`: passed, covering package
  creation from a pre-existing report, manifest derivation, checksum
  verification, explicit package verification, transfer archive creation,
  direct archive verification, downloaded artifact zip verification, archive
  path safety, unsafe archive and unsafe zip rejection, multi-archive zip
  rejection, tamper rejection, existing-output refusal, and
  temp-directory-local scratch output.
- `bin/shared-cache-evidence-package.sh --from-report <current-host-report>.jsonl --output <tmp-package>`:
  passed; package checksums verified with `shasum -a 256 -c SHA256SUMS`, and
  auditing the packaged `evidence.jsonl` failed for the expected single-host
  reason.
- `bin/shared-cache-evidence-package.sh --from-report <current-host-report>.jsonl --output <tmp-package> --archive <tmp>.tar.gz`:
  passed on the current-host real-cache report, reverified the generated package,
  wrote a transfer archive, and confirmed the archive contains `evidence.jsonl`,
  `manifest.json`, `smoke.log`, `README.txt`, and `SHA256SUMS`.
- `bin/disassembler-v2-alpha-validate.sh --skip-workspace-tests --skip-fixtures --skip-bench --skip-ui --skip-external --real-cache-archive <tmp>.tar.gz`:
  passed real-cache smoke packaging on the current host, produced a verified
  archive containing the main OS and DriverKit cache evidence, and failed the
  public-beta audit for the expected single-host reason.
- `bin/shared-cache-evidence-package.sh --verify <tmp>.tar.gz`: passed on the
  current-host transfer archive, including safe extraction and package checksum
  verification.
- `bin/shared-cache-evidence-package.sh --verify <tmp-artifact>.zip`: passed on
  a GitHub-artifact-shaped zip wrapper around the current-host transfer archive,
  including safe zip extraction, inner archive verification, and package
  checksum verification. The sampled wrapper had SHA-256
  `d0672e5456c205a21d8fbfeb2fa0df8f43afdb735326f68c3622f93dea9a52bf`.
- `bin/shared-cache-evidence-package.sh --verify <tmp-package>`: passed on the
  current-host package, reporting two passed cache UUIDs and host build `25F71`.
- `bin/shared-cache-evidence-audit.sh <tmp-package>`: accepted the verified
  current-host package directory and failed for the expected single-host reason.
- `bin/shared-cache-evidence-audit.sh <tmp>.tar.gz`: accepted the verified
  current-host package archive and failed for the expected single-host reason.
- `bin/shared-cache-evidence-audit.sh <tmp-artifact>.zip`: accepted the
  downloaded-artifact-shaped wrapper, verified its embedded package evidence,
  and failed for the expected single-host reason.
- `bin/shared-cache-evidence-audit-test.sh`: passed, covering audit pass/fail
  behavior, direct package-directory and package-archive inputs, package tamper
  rejection, downloaded artifact zip inputs, unsafe archive and unsafe zip
  rejection, threshold overrides, malformed counts, missing host records,
  failed cache records, and missing cache UUIDs.
- `cargo test -p damsel-macho ordinal -- --nocapture`: passed, covering Mach-O
  one-based library ordinals, self-prefixed `goblin` library lists, legacy
  ordinal `0xfe`, and the duplicate-symbol ordinal fixture.
- `cargo test -p damsel-macho indirect_symbol_resolution -- --nocapture`:
  passed, covering exact ordinal-dylib matches, stale projected-cache ordinals
  with unique-name fallback, duplicate same-dylib import rows, and true
  cross-dylib ambiguity rejection.
- `bin/disasm-external-compare.sh`: passed across `semantic-switch`,
  `import-rich`, `indirect-dispatch`, `relative-dispatch`, `objc-sample`,
  `swift-sample`, and `arm64e-sample`.

## Public Beta Remaining Gates

- Current local evidence covers two distinct macOS 26.5 arm64e cache UUIDs
  (main OS plus DriverKit), but public-beta breadth still needs at least one
  `--report` evidence file from a different macOS release or an independent
  host.
- Expand Swift metadata coverage beyond symbol-table entrypoints only after
  adding dedicated metadata fixtures and parser contracts.
