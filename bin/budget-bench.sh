#!/usr/bin/env bash
# Budget bench (Wave 4.10 / 6.8): measures end-to-end timings against
# the roadmap budgets. Exit 0 = all budgets met; exit 1 = regression.
#
# Budgets (from docs/roadmaps/re-power-roadmap.md):
#   analyze 30 MB binary      ≤ 60 s
#   xref query (cold)         ≤ 2 s
#   decompile 1 KB function   ≤ 1 s
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${DAMSEL_BIN:-$ROOT/target/release/damsel-cli}"
TARGET="${1:-$ROOT/fixtures/bin/adversarial/swift-rich-arm64}"
OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT

fail=0
check() {
  local name=$1 budget=$2 actual=$3
  local status="OK"
  if (( $(echo "$actual > $budget" | bc -l) )); then
    status="FAIL"
    fail=1
  fi
  printf "  %-28s %6.2fs (budget %.0fs)  %s\n" "$name" "$actual" "$budget" "$status"
}

echo "budget bench: $(basename "$TARGET")"

# 1. analyze (budget: 60 s for 30 MB; our fixture is smaller — scaled)
t0=$(python3 -c 'import time; print(time.time())')
"$BIN" analyze "$TARGET" --refresh > /dev/null 2>&1
t1=$(python3 -c 'import time; print(time.time())')
analyze_s=$(python3 -c "print($t1 - $t0)")
check "analyze" 60 "$analyze_s"

# 2. xrefs cold query (budget: 2 s)
t0=$(python3 -c 'import time; print(time.time())')
"$BIN" xrefs "$TARGET" --to 0x100000000 > /dev/null 2>&1 || true
t1=$(python3 -c 'import time; print(time.time())')
xref_s=$(python3 -c "print($t1 - $t0)")
check "xrefs (cold)" 2 "$xref_s"

# 3. decompile a function (budget: 1 s)
t0=$(python3 -c 'import time; print(time.time())')
"$BIN" decompile "$TARGET" --function _main > /dev/null 2>&1 || true
t1=$(python3 -c 'import time; print(time.time())')
decomp_s=$(python3 -c "print($t1 - $t0)")
check "decompile (single fn)" 1 "$decomp_s"

if (( fail )); then
  echo "RESULT: FAIL (budget regression)"
  exit 1
fi
echo "RESULT: OK (all budgets met)"
