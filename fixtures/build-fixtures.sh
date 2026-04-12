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
e07559fadea991c21dda7e94bc064fb641974c76daa847cd8501dc9be36e3811 objc-sample
94436763cc1a25e64a6f2358936ea9046b608a370bd20d01b992635c9a7a2ae4 arm64e-sample
b200d6d6587af820c258aad12eed1b482d335a8f92f44972e08b0c4f1277d072 x86_64-only-hello
bec5bb22e25742ac9368876de4a94575d14e91232fbdd3a1ba655d29cb9b550b import-rich
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

  arm64e_tmp="$BIN/arm64e-sample.tmp"
  if "$CLANG" \
    -arch arm64e \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    "$SRC/hello.c" \
    -o "$arm64e_tmp"
  then
    mv "$arm64e_tmp" "$BIN/arm64e-sample"
  else
    rm -f "$arm64e_tmp"
    if [ -f "$BIN/arm64e-sample" ]; then
      echo "warning: arm64e build unavailable; preserving existing fixture" >&2
    else
      echo "warning: arm64e build unavailable and no existing fixture is present" >&2
    fi
  fi

  "$CLANG" \
    -arch x86_64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    "$SRC/hello.c" \
    -o "$BIN/x86_64-only-hello"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    "$SRC/import-rich.c" \
    -o "$BIN/import-rich"

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
