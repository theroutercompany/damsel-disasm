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

assert_exit_code() {
  actual="$1"
  expected="$2"
  context="$3"
  if [ "$actual" -ne "$expected" ]; then
    echo "expected exit code $expected for $context, got $actual" >&2
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
assert_contains "$tmpdir/manifest.txt" "swift-sample"

"$SCRIPT" --manifest-corpus >"$tmpdir/manifest-corpus.txt"
assert_contains "$tmpdir/manifest-corpus.txt" "README.md"

"$SCRIPT" --manifest-all >"$tmpdir/manifest-all.txt"
assert_contains "$tmpdir/manifest-all.txt" "# fixtures"
assert_contains "$tmpdir/manifest-all.txt" "# export-trie-corpus"
assert_contains "$tmpdir/manifest-all.txt" "arm64-symbolized"
assert_contains "$tmpdir/manifest-all.txt" "README.md"

"$SCRIPT" --check >"$tmpdir/check.stdout" 2>"$tmpdir/check.stderr"
assert_exit_code "$?" 0 "--check"
if [ -s "$tmpdir/check.stderr" ]; then
  echo "expected empty stderr for --check success" >&2
  cat "$tmpdir/check.stderr" >&2
  exit 1
fi

mkdir -p "$tmpdir/fakehash"
cat >"$tmpdir/fakehash/sha256sum" <<'EOF'
#!/bin/sh
exit 1
EOF
cat >"$tmpdir/fakehash/shasum" <<'EOF'
#!/bin/sh
exit 1
EOF
cat >"$tmpdir/fakehash/openssl" <<'EOF'
#!/bin/sh
exit 1
EOF
chmod +x "$tmpdir/fakehash/sha256sum" "$tmpdir/fakehash/shasum" "$tmpdir/fakehash/openssl"

set +e
PATH="$tmpdir/fakehash:$PATH" "$SCRIPT" --check >"$tmpdir/check-nohash.stdout" 2>"$tmpdir/check-nohash.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "--check without usable hash backend"
assert_contains "$tmpdir/check-nohash.stderr" "missing usable hash tool for drift checks (need one of: sha256sum, shasum, openssl)"

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

assert_exit_code "$status" 2 "non-macOS rebuild"
assert_contains "$tmpdir/rebuild.stderr" "fixture rebuild is macOS-only (host: Linux/x86_64); use '--check' or '--manifest' on this host"

mkdir -p "$tmpdir/fake-sdk-fail"
cat >"$tmpdir/fake-sdk-fail/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Darwin"
    ;;
  -m)
    echo "arm64"
    ;;
  *)
    echo "Darwin"
    ;;
esac
EOF
cat >"$tmpdir/fake-sdk-fail/xcrun" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --show-sdk-path)
    exit 1
    ;;
  *)
    exit 1
    ;;
esac
EOF
chmod +x "$tmpdir/fake-sdk-fail/uname" "$tmpdir/fake-sdk-fail/xcrun"

set +e
PATH="$tmpdir/fake-sdk-fail:$PATH" "$SCRIPT" >"$tmpdir/sdkfail.stdout" 2>"$tmpdir/sdkfail.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "sdk probe failure"
assert_contains "$tmpdir/sdkfail.stderr" "unable to resolve macOS SDK path via 'xcrun --show-sdk-path'"

mkdir -p "$tmpdir/fake-clang-missing"
cat >"$tmpdir/fake-clang-missing/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Darwin"
    ;;
  -m)
    echo "arm64"
    ;;
  *)
    echo "Darwin"
    ;;
esac
EOF
cat >"$tmpdir/fake-clang-missing/dirname" <<'EOF'
#!/bin/sh
value="${1:-.}"
case "$value" in
  */*)
    printf '%s\n' "${value%/*}"
    ;;
  *)
    echo "."
    ;;
esac
EOF
cat >"$tmpdir/fake-clang-missing/xcrun" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --show-sdk-path)
    echo "/tmp/mock-sdk"
    exit 0
    ;;
  --find)
    case "${2:-}" in
      clang)
        exit 1
        ;;
      strip)
        echo "__FAKE_DIR__/strip"
        exit 0
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF
cat >"$tmpdir/fake-clang-missing/strip" <<'EOF'
#!/bin/sh
exit 0
EOF
sed "s#__FAKE_DIR__#$tmpdir/fake-clang-missing#g" "$tmpdir/fake-clang-missing/xcrun" >"$tmpdir/fake-clang-missing/xcrun.tmp"
mv "$tmpdir/fake-clang-missing/xcrun.tmp" "$tmpdir/fake-clang-missing/xcrun"
chmod +x "$tmpdir/fake-clang-missing/uname" "$tmpdir/fake-clang-missing/dirname" "$tmpdir/fake-clang-missing/xcrun" "$tmpdir/fake-clang-missing/strip"

set +e
PATH="$tmpdir/fake-clang-missing" "$SCRIPT" >"$tmpdir/clangmissing.stdout" 2>"$tmpdir/clangmissing.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "clang missing"
assert_contains "$tmpdir/clangmissing.stderr" "unable to locate clang via xcrun or PATH"

mkdir -p "$tmpdir/fake-strip-missing"
cat >"$tmpdir/fake-strip-missing/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Darwin"
    ;;
  -m)
    echo "arm64"
    ;;
  *)
    echo "Darwin"
    ;;
esac
EOF
cat >"$tmpdir/fake-strip-missing/dirname" <<'EOF'
#!/bin/sh
value="${1:-.}"
case "$value" in
  */*)
    printf '%s\n' "${value%/*}"
    ;;
  *)
    echo "."
    ;;
esac
EOF
cat >"$tmpdir/fake-strip-missing/xcrun" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --show-sdk-path)
    echo "/tmp/mock-sdk"
    exit 0
    ;;
  --find)
    case "${2:-}" in
      clang)
        echo "__FAKE_DIR__/clang"
        exit 0
        ;;
      strip)
        exit 1
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF
cat >"$tmpdir/fake-strip-missing/clang" <<'EOF'
#!/bin/sh
exit 0
EOF
sed "s#__FAKE_DIR__#$tmpdir/fake-strip-missing#g" "$tmpdir/fake-strip-missing/xcrun" >"$tmpdir/fake-strip-missing/xcrun.tmp"
mv "$tmpdir/fake-strip-missing/xcrun.tmp" "$tmpdir/fake-strip-missing/xcrun"
chmod +x "$tmpdir/fake-strip-missing/uname" "$tmpdir/fake-strip-missing/dirname" "$tmpdir/fake-strip-missing/xcrun" "$tmpdir/fake-strip-missing/clang"

set +e
PATH="$tmpdir/fake-strip-missing" "$SCRIPT" >"$tmpdir/stripmissing.stdout" 2>"$tmpdir/stripmissing.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "strip missing"
assert_contains "$tmpdir/stripmissing.stderr" "unable to locate strip via xcrun or PATH"

mkdir -p "$tmpdir/fake-python-missing"
cat >"$tmpdir/fake-python-missing/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Darwin"
    ;;
  -m)
    echo "arm64"
    ;;
  *)
    echo "Darwin"
    ;;
esac
EOF
cat >"$tmpdir/fake-python-missing/dirname" <<'EOF'
#!/bin/sh
value="${1:-.}"
case "$value" in
  */*)
    printf '%s\n' "${value%/*}"
    ;;
  *)
    echo "."
    ;;
esac
EOF
cat >"$tmpdir/fake-python-missing/xcrun" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --show-sdk-path)
    echo "/tmp/mock-sdk"
    exit 0
    ;;
  --find)
    case "${2:-}" in
      clang)
        echo "__FAKE_DIR__/clang"
        exit 0
        ;;
      strip)
        echo "__FAKE_DIR__/strip"
        exit 0
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF
cat >"$tmpdir/fake-python-missing/clang" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-python-missing/strip" <<'EOF'
#!/bin/sh
exit 0
EOF
sed "s#__FAKE_DIR__#$tmpdir/fake-python-missing#g" "$tmpdir/fake-python-missing/xcrun" >"$tmpdir/fake-python-missing/xcrun.tmp"
mv "$tmpdir/fake-python-missing/xcrun.tmp" "$tmpdir/fake-python-missing/xcrun"
chmod +x "$tmpdir/fake-python-missing/uname" "$tmpdir/fake-python-missing/dirname" "$tmpdir/fake-python-missing/xcrun" "$tmpdir/fake-python-missing/clang" "$tmpdir/fake-python-missing/strip"

set +e
PATH="$tmpdir/fake-python-missing" "$SCRIPT" >"$tmpdir/pythonmissing.stdout" 2>"$tmpdir/pythonmissing.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "python3 missing"
assert_contains "$tmpdir/pythonmissing.stderr" "fixture rebuild requires python3"

mkdir -p "$tmpdir/fake-python-fail"
cat >"$tmpdir/fake-python-fail/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Darwin"
    ;;
  -m)
    echo "arm64"
    ;;
  *)
    echo "Darwin"
    ;;
esac
EOF
cat >"$tmpdir/fake-python-fail/xcrun" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --show-sdk-path)
    echo "/tmp/mock-sdk"
    exit 0
    ;;
  --find)
    case "${2:-}" in
      clang)
        echo "__FAKE_DIR__/clang"
        exit 0
        ;;
      strip)
        echo "__FAKE_DIR__/strip"
        exit 0
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF
cat >"$tmpdir/fake-python-fail/clang" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-python-fail/strip" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-python-fail/python3" <<'EOF'
#!/bin/sh
exit 1
EOF
sed "s#__FAKE_DIR__#$tmpdir/fake-python-fail#g" "$tmpdir/fake-python-fail/xcrun" >"$tmpdir/fake-python-fail/xcrun.tmp"
mv "$tmpdir/fake-python-fail/xcrun.tmp" "$tmpdir/fake-python-fail/xcrun"
chmod +x "$tmpdir/fake-python-fail/uname" "$tmpdir/fake-python-fail/xcrun" "$tmpdir/fake-python-fail/clang" "$tmpdir/fake-python-fail/strip" "$tmpdir/fake-python-fail/python3"

set +e
PATH="$tmpdir/fake-python-fail:$PATH" "$SCRIPT" >"$tmpdir/pythonfail.stdout" 2>"$tmpdir/pythonfail.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "python3 negative probe"
assert_contains "$tmpdir/pythonfail.stderr" "fixture rebuild requires invocable python3 (python3 -c 'import sys; sys.exit(0)' failed)"

mkdir -p "$tmpdir/fake-nm-missing"
cat >"$tmpdir/fake-nm-missing/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Darwin"
    ;;
  -m)
    echo "arm64"
    ;;
  *)
    echo "Darwin"
    ;;
esac
EOF
cat >"$tmpdir/fake-nm-missing/dirname" <<'EOF'
#!/bin/sh
value="${1:-.}"
case "$value" in
  */*)
    printf '%s\n' "${value%/*}"
    ;;
  *)
    echo "."
    ;;
esac
EOF
cat >"$tmpdir/fake-nm-missing/xcrun" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --show-sdk-path)
    echo "/tmp/mock-sdk"
    exit 0
    ;;
  --find)
    case "${2:-}" in
      clang)
        echo "__FAKE_DIR__/clang"
        exit 0
        ;;
      strip)
        echo "__FAKE_DIR__/strip"
        exit 0
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF
cat >"$tmpdir/fake-nm-missing/clang" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-nm-missing/strip" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-nm-missing/python3" <<'EOF'
#!/bin/sh
exit 0
EOF
sed "s#__FAKE_DIR__#$tmpdir/fake-nm-missing#g" "$tmpdir/fake-nm-missing/xcrun" >"$tmpdir/fake-nm-missing/xcrun.tmp"
mv "$tmpdir/fake-nm-missing/xcrun.tmp" "$tmpdir/fake-nm-missing/xcrun"
chmod +x "$tmpdir/fake-nm-missing/uname" "$tmpdir/fake-nm-missing/dirname" "$tmpdir/fake-nm-missing/xcrun" "$tmpdir/fake-nm-missing/clang" "$tmpdir/fake-nm-missing/strip" "$tmpdir/fake-nm-missing/python3"

set +e
PATH="$tmpdir/fake-nm-missing" "$SCRIPT" >"$tmpdir/nmmissing.stdout" 2>"$tmpdir/nmmissing.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "nm missing"
assert_contains "$tmpdir/nmmissing.stderr" "fixture rebuild requires nm"

mkdir -p "$tmpdir/fake-swiftc-missing"
cat >"$tmpdir/fake-swiftc-missing/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
  -s)
    echo "Darwin"
    ;;
  -m)
    echo "arm64"
    ;;
  *)
    echo "Darwin"
    ;;
esac
EOF
cat >"$tmpdir/fake-swiftc-missing/dirname" <<'EOF'
#!/bin/sh
value="${1:-.}"
case "$value" in
  */*)
    printf '%s\n' "${value%/*}"
    ;;
  *)
    echo "."
    ;;
esac
EOF
cat >"$tmpdir/fake-swiftc-missing/xcrun" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --show-sdk-path)
    echo "/tmp/mock-sdk"
    exit 0
    ;;
  --find)
    case "${2:-}" in
      clang)
        echo "__FAKE_DIR__/clang"
        exit 0
        ;;
      strip)
        echo "__FAKE_DIR__/strip"
        exit 0
        ;;
      swiftc)
        exit 1
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF
cat >"$tmpdir/fake-swiftc-missing/clang" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-swiftc-missing/strip" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-swiftc-missing/python3" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmpdir/fake-swiftc-missing/nm" <<'EOF'
#!/bin/sh
if [ "$1" = "--version" ]; then exit 0; fi
exit 1
EOF
sed "s#__FAKE_DIR__#$tmpdir/fake-swiftc-missing#g" "$tmpdir/fake-swiftc-missing/xcrun" >"$tmpdir/fake-swiftc-missing/xcrun.tmp"
mv "$tmpdir/fake-swiftc-missing/xcrun.tmp" "$tmpdir/fake-swiftc-missing/xcrun"
chmod +x "$tmpdir/fake-swiftc-missing/uname" "$tmpdir/fake-swiftc-missing/dirname" "$tmpdir/fake-swiftc-missing/xcrun" "$tmpdir/fake-swiftc-missing/clang" "$tmpdir/fake-swiftc-missing/strip" "$tmpdir/fake-swiftc-missing/python3" "$tmpdir/fake-swiftc-missing/nm"

set +e
PATH="$tmpdir/fake-swiftc-missing" "$SCRIPT" >"$tmpdir/swiftcmissing.stdout" 2>"$tmpdir/swiftcmissing.stderr"
status=$?
set -e
assert_exit_code "$status" 2 "swiftc missing"
assert_contains "$tmpdir/swiftcmissing.stderr" "unable to locate swiftc via xcrun or PATH"

echo "fixtures parity probes passed"
