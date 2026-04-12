#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
SRC="$ROOT/src"
BIN="$ROOT/bin"

print_manifest() {
  cat <<'EOF'
f1550c7234dbef3fb1218e3b3edbf8b8d9f17c9d742109463f8517569f1530c9 arm64-symbolized
037a16d7b82f24dd0ce4dffae59a374ab7e5ca826d5660cd3fd131c0ff932ca7 arm64-stripped
d02c320d54e579378b3781c73f0322605873f0c9d681371fe635691cd1199c87 universal-hello
c04a9535969d0f1132d158dd5e9380f0e6dc4fc1835e2bceb206f66993ab71e6 objc-sample
fd33e4f22bf94f6f75b9bb33e2b99d5c3888a1a7fdc907dd13638f2aba816b66 malformed-truncated
EOF
}

sha256_file() {
  file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    echo "missing sha256 tool (sha256sum or shasum)" >&2
    exit 2
  fi
}

check_fixtures() {
  mkdir -p "$BIN"
  manifest_file="${TMPDIR:-/tmp}/damsel-fixture-manifest.$$"
  trap 'rm -f "$manifest_file"' EXIT INT TERM
  print_manifest > "$manifest_file"

  status=0
  while read -r expected_hash fixture_name; do
    [ -n "$expected_hash" ] || continue
    fixture_path="$BIN/$fixture_name"
    if [ ! -f "$fixture_path" ]; then
      echo "missing fixture: $fixture_name" >&2
      status=1
      continue
    fi
    actual_hash="$(sha256_file "$fixture_path")"
    if [ "$actual_hash" != "$expected_hash" ]; then
      echo "hash mismatch: $fixture_name" >&2
      echo "  expected: $expected_hash" >&2
      echo "  actual:   $actual_hash" >&2
      status=1
    fi
  done < "$manifest_file"

  rm -f "$manifest_file"
  trap - EXIT INT TERM
  exit "$status"
}

build_fixtures() {
  if ! command -v xcrun >/dev/null 2>&1; then
    echo "xcrun is required to build fixtures" >&2
    exit 2
  fi

  SDKROOT="$(xcrun --show-sdk-path)"
  CLANG="/usr/bin/clang"
  STRIP="$(xcrun --find strip)"

  mkdir -p "$BIN"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    "$SRC/hello.c" \
    -o "$BIN/arm64-symbolized"

  cp "$BIN/arm64-symbolized" "$BIN/arm64-stripped"
  "$STRIP" -S -x "$BIN/arm64-stripped"

  "$CLANG" \
    -arch arm64 \
    -arch x86_64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    "$SRC/hello.c" \
    -o "$BIN/universal-hello"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -framework Foundation \
    "$SRC/objc-sample.m" \
    -o "$BIN/objc-sample"

  head -c 256 "$BIN/arm64-symbolized" > "$BIN/malformed-truncated"
}

case "${1:-}" in
  --check)
    check_fixtures
    ;;
  --manifest)
    print_manifest
    ;;
  --help|-h)
    cat <<'EOF'
Usage:
  fixtures/build-fixtures.sh            Build fixture binaries (macOS + Xcode).
  fixtures/build-fixtures.sh --check    Verify checked-in fixture hashes.
  fixtures/build-fixtures.sh --manifest Print fixture hash manifest.
EOF
    ;;
  "")
    build_fixtures
    ;;
  *)
    echo "unknown argument: $1" >&2
    exit 2
    ;;
esac
