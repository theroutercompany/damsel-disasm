#!/usr/bin/env bash
# ELF evidence package builder (RE Power Wave 5, evidence row 5.13).
#
# Runs the damsel analyst loop over ELF targets and writes a portable,
# self-verifying evidence package:
#   <outdir>/report.jsonl      — one JSON line per command per target
#   <outdir>/manifest.sha256   — package hashes
#
# Usage:
#   bin/elf-evidence-package.sh <outdir> [targets...]
#     No targets  → synthetic mode: generates deterministic fixtures via
#                   a rust test helper (runs anywhere, CI included).
#     With targets → real mode: analyze each real ELF path.
#   bin/elf-evidence-package.sh --verify <outdir>
#     Recomputes the manifest and checks the recorded self-verification.
set -euo pipefail
cd "$(dirname "$0")/.."

VERIFY=0
if [[ "${1:-}" == "--verify" ]]; then
  VERIFY=1
  shift
fi
OUTDIR="${1:?usage: elf-evidence-package.sh <outdir> [targets...] | --verify <outdir>}"
shift || true

if [[ "$VERIFY" == 1 ]]; then
  [[ -f "$OUTDIR/manifest.sha256" ]] || { echo "no manifest in $OUTDIR" >&2; exit 1; }
  (cd "$OUTDIR" && shasum -a 256 -c manifest.sha256 --status) \
    && echo "elf-evidence verify: OK ($OUTDIR)" \
    || { echo "elf-evidence verify: FAILED" >&2; exit 1; }
  exit 0
fi

DAMSEL="${DAMSEL:-target/debug/damsel-cli}"
if [[ ! -x "$DAMSEL" ]]; then
  echo "damsel binary not found at $DAMSEL (build with cargo build -p damsel-cli or set DAMSEL)" >&2
  exit 1
fi
DAMSEL="$(cd "$(dirname "$DAMSEL")" && pwd)/$(basename "$DAMSEL")"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUTDIR"
REPORT="$OUTDIR/report.jsonl"
: > "$REPORT"

commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "{\"kind\":\"meta\",\"commit\":\"$commit\",\"damsel\":\"$("$DAMSEL" doctor --format json >/dev/null 2>&1 && echo runnable || echo present)\"}" >> "$REPORT"

if [[ $# -eq 0 ]]; then
  # Synthetic mode: fixtures from the rust test-support builders.
  cargo test -p damsel-elf --test loader_matrix --quiet --no-run >/dev/null
  BIN="$(ls -t target/debug/deps/loader_matrix-* | grep -v '\.d$' | awk 'NR==1')"
  TARGETS=("$WORK/libfake.so.1")
  env DAMSEL_SYNTH_OUT="$WORK/libfake.so.1" "$BIN" elf_evidence_write_fixture \
    || { echo "fixture helper failed (run via cargo test env)" >&2; exit 1; }
else
  TARGETS=("$@")
fi

for target in "${TARGETS[@]}"; do
  name="$(basename "$target")"
  project="$WORK/$name.damsel"
  analyze_json="$("$DAMSEL" analyze "$target" --refresh --format json)"
  echo "{\"kind\":\"analyze\",\"target\":\"$name\",\"result\":$analyze_json}" >> "$REPORT"
  functions_json="$("$DAMSEL" functions "$target" --limit 5 --format json)"
  echo "{\"kind\":\"functions\",\"target\":\"$name\",\"result\":$functions_json}" >> "$REPORT"
  strings_json="$("$DAMSEL" strings "$target" --format json)"
  echo "{\"kind\":\"strings\",\"target\":\"$name\",\"result\":$strings_json}" >> "$REPORT"

  # Self-verification: queries are deterministic — run them twice.
  functions_again="$("$DAMSEL" functions "$target" --limit 5 --format json)"
  strings_again="$("$DAMSEL" strings "$target" --format json)"
  if [[ "$functions_json" != "$functions_again" || "$strings_json" != "$strings_again" ]]; then
    echo "elf-evidence self-verification FAILED for $name (nondeterministic queries)" >&2
    exit 1
  fi
  echo "{\"kind\":\"selfcheck\",\"target\":\"$name\",\"deterministic\":true}" >> "$REPORT"
done

# Manifest (relative paths inside the package).
(cd "$OUTDIR" && shasum -a 256 report.jsonl > manifest.sha256)
echo "elf-evidence package written: $OUTDIR ($(wc -l < "$REPORT") report lines)"
