#!/usr/bin/env bash
# Function-recovery audit gate (RE Power Wave 4, evidence row 4.5).
#
# Measures recovery recall / false-positive rate against hidden
# symbol-table ground truth on the checked-in fixtures and enforces the
# roadmap thresholds: recall >= 95%, FP <= 2%.
#
# Usage: bin/function-recovery-audit.sh [--no-capture]
set -euo pipefail

cd "$(dirname "$0")/.."

EXTRA=()
if [[ "${1:-}" == "--no-capture" ]]; then
  EXTRA=(-- --nocapture)
fi

echo "function-recovery audit: running recovery_audit tests (evidence row 4.5)"
if cargo test -p damsel-macho --test analysis_ingest recovery_audit "${EXTRA[@]+"${EXTRA[@]}"}"; then
  echo "function-recovery audit: PASS (thresholds recall>=95%, fp<=2% held)"
else
  echo "function-recovery audit: FAIL" >&2
  exit 1
fi
