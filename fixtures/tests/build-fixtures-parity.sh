#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
SCRIPT="$ROOT/fixtures/build-fixtures.sh"

mktemp_dir() {
  if command -v mktemp >/dev/null 2>&1; then
    mktemp -d "${TMPDIR:-/tmp}/damsel-fixtures-parity.XXXXXX"
    return
  fi
  fallback="${TMPDIR:-/tmp}/damsel-fixtures-parity.$$"
  mkdir -p "$fallback"
  echo "$fallback"
}

assert_contains() {
  file="$1"
  needle="$2"
  if ! grep -Fq "$needle" "$file"; then
    echo "expected output to contain: $needle" >&2
    echo "actual output:" >&2
    cat "$file" >&2
    exit 1
  fi
}

tmpdir="$(mktemp_dir)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

"$SCRIPT" --help >"$tmpdir/help.txt"
assert_contains "$tmpdir/help.txt" "portable drift check"
assert_contains "$tmpdir/help.txt" "manifest display"
assert_contains "$tmpdir/help.txt" "macOS-only rebuild"

"$SCRIPT" --manifest >"$tmpdir/manifest.txt"
assert_contains "$tmpdir/manifest.txt" "arm64-symbolized"

"$SCRIPT" --manifest-corpus >"$tmpdir/manifest-corpus.txt"
assert_contains "$tmpdir/manifest-corpus.txt" "README.md"

"$SCRIPT" --manifest-all >"$tmpdir/manifest-all.txt"
assert_contains "$tmpdir/manifest-all.txt" "# fixtures"
assert_contains "$tmpdir/manifest-all.txt" "# export-trie-corpus"
assert_contains "$tmpdir/manifest-all.txt" "arm64-symbolized"
assert_contains "$tmpdir/manifest-all.txt" "README.md"

mkdir -p "$tmpdir/fakebin"
cat >"$tmpdir/fakebin/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Linux"
    ;;
  -m)
    echo "x86_64"
    ;;
  *)
    echo "Linux"
    ;;
esac
EOF
chmod +x "$tmpdir/fakebin/uname"

set +e
PATH="$tmpdir/fakebin:$PATH" "$SCRIPT" >"$tmpdir/rebuild.stdout" 2>"$tmpdir/rebuild.stderr"
status=$?
set -e

if [ "$status" -ne 2 ]; then
  echo "expected non-macOS rebuild path to fail with exit code 2, got $status" >&2
  cat "$tmpdir/rebuild.stdout" >&2
  cat "$tmpdir/rebuild.stderr" >&2
  exit 1
fi

assert_contains "$tmpdir/rebuild.stderr" "fixture rebuild is macOS-only (host: Linux/x86_64); use '--check' or '--manifest' on this host"

echo "fixtures parity probes passed"
