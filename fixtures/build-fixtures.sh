#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
SRC="$ROOT/src"
BIN="$ROOT/bin"
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
