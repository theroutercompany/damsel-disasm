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
7aae1dd9625bba604dbd81aabd1e1fa442bc98bf01e1e3de43279804f3a9080d objc-sample
94436763cc1a25e64a6f2358936ea9046b608a370bd20d01b992635c9a7a2ae4 arm64e-sample
b200d6d6587af820c258aad12eed1b482d335a8f92f44972e08b0c4f1277d072 x86_64-only-hello
bec5bb22e25742ac9368876de4a94575d14e91232fbdd3a1ba655d29cb9b550b import-rich
b34622e74db24f3829cb2bda8040e80c02eff8fe479339bd654be438a7a701bc import-lazy
4cece37e15eaed6ebbc8a8961f8d2e000e0eadf3aa43fc5c9b6ebf3aad259f7f semantic-switch
678eb57e8b45c8be5904a2c1301fa2b6315252e68a7fe579ada23ffcfdb0ec1e export-kinds
5c459bac25fae382836bee6ff5a0ecde1de8f9594dc7fa7d7be65cef3ff58fc8 indirect-dispatch
06664157d92032c782a70335800dd6bdf49d9919b05696538eebd95b36b03ffb relative-dispatch
3ad866e5b98bbaebee790d46d1963319f38d6cc96aeda4e1e3f2f305abb2736c malformed-objc-protocol-list
5d46560896550803303f6f92027d1c8e622c18761e20cc1b39d7a64b57e2b5dc malformed-dysymtab-indirect
fd33e4f22bf94f6f75b9bb33e2b99d5c3888a1a7fdc907dd13638f2aba816b66 malformed-truncated
61ac976ddaf21d6d427c202dfab484a9557ebe7fbb04509443726abc9e95de8d malformed-stub-helper-size
18ccbc63820072b0572926d566e99a375f2671685f2a7bfcaedf2457c8d42952 malformed-stub-reserved2
39449cc22b5b090af7abd3b0b811e299827bc36bf4267ca8f32472616291d6a4 duplicate-symbol-ordinal
EOF
}

print_export_trie_corpus_manifest() {
  cat <<'EOF'
5f19c8c7420dcd5151aa2d185731a66729de10489244fdd4ef33f420dbdfe6b5 README.md
103280ecc521fe027b8302b4ce490cf845e598208981deeafdf6634dd94b025a reexport-same-name.toml
e4592ad7d2b9a314ef355d6616725fe3a8adfe648bf07b46f2bf440dc7f634d9 reexport-renamed-symbol.toml
47603b12003ef5de1456da025a1d1eec2c4326a3730a4fa51e5e836de8968f3c stub-and-resolver.toml
24607c88435b123c540dd6287f2fe709ef1c1d37146ba7f74ba503115b27826f malformed-reexport-ordinal.toml
5f26b0fc7bc466d7c9c90aa44fc889f8840e549fbbb1c0a5d327f0ba14717ae5 malformed-stub-resolver-offsets.toml
03a07548d5878513a7ee4ca6e6e88d81fa43f1d4823a4fc0a9a105f34e108e2b unknown-flag-bits-regular.toml
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
  corpus_manifest_file="${TMPDIR:-/tmp}/damsel-export-trie-corpus-manifest.$$"
  trap 'rm -f "$manifest_file" "$corpus_manifest_file"' EXIT INT TERM
  print_manifest > "$manifest_file"
  print_export_trie_corpus_manifest > "$corpus_manifest_file"

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

  corpus_root="$ROOT/export-trie-corpus"
  while read -r expected_hash corpus_name; do
    [ -n "$expected_hash" ] || continue
    corpus_path="$corpus_root/$corpus_name"
    if [ ! -f "$corpus_path" ]; then
      echo "missing export-trie corpus file: $corpus_name" >&2
      status=1
      continue
    fi
    actual_hash="$(sha256_file "$corpus_path")"
    if [ "$actual_hash" != "$expected_hash" ]; then
      echo "hash mismatch: export-trie-corpus/$corpus_name" >&2
      echo "  expected: $expected_hash" >&2
      echo "  actual:   $actual_hash" >&2
      status=1
    fi
  done < "$corpus_manifest_file"

  rm -f "$manifest_file"
  rm -f "$corpus_manifest_file"
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
    -Wl,-no_fixup_chains \
    "$SRC/import-rich.c" \
    -o "$BIN/import-lazy"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -O2 \
    "$SRC/semantic-switch.c" \
    -o "$BIN/semantic-switch"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -O2 \
    "$SRC/export-kinds.c" \
    "$SRC/export-absolute.s" \
    -Wl,-exported_symbol,_exported_regular \
    -Wl,-exported_symbol,_exported_weak \
    -Wl,-exported_symbol,_exported_tls \
    -Wl,-exported_symbol,_exported_absolute \
    -o "$BIN/export-kinds"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -O2 \
    -Wl,-no_fixup_chains \
    -Wl,-exported_symbol,_exported_gamma \
    "$SRC/indirect-dispatch.c" \
    -o "$BIN/indirect-dispatch"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -O2 \
    -Wl,-no_fixup_chains \
    -Wl,-exported_symbol,_relative_exported_gamma \
    "$SRC/relative-dispatch.s" \
    -o "$BIN/relative-dispatch"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -framework Foundation \
    "$SRC/objc-sample.m" \
    -o "$BIN/objc-sample"

  ROOT_BIN="$BIN" python3 - <<'PY'
from pathlib import Path
import os
import struct
import subprocess

MH_MAGIC_64 = 0xfeedfacf
LC_SEGMENT_64 = 0x19

root = Path(os.environ["ROOT_BIN"])
src = root / "objc-sample"
out = root / "malformed-objc-protocol-list"
data = bytearray(src.read_bytes())

def find_symbol_address(path: Path, name: str) -> int:
    output = subprocess.check_output(["nm", "-a", str(path)], text=True)
    for line in output.splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[-1] == name:
            return int(parts[0], 16)
    raise SystemExit(f"missing symbol: {name}")

def va_to_file_offset(buf: bytearray, address: int) -> int:
    magic, = struct.unpack_from("<I", buf, 0)
    if magic != MH_MAGIC_64:
        raise SystemExit("unexpected Mach-O magic")
    ncmds, = struct.unpack_from("<I", buf, 16)
    offset = 32
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from("<II", buf, offset)
        if cmd == LC_SEGMENT_64:
            nsects, = struct.unpack_from("<I", buf, offset + 64)
            section_offset = offset + 72
            for _ in range(nsects):
                addr, size = struct.unpack_from("<QQ", buf, section_offset + 32)
                file_offset, = struct.unpack_from("<I", buf, section_offset + 48)
                if addr <= address < addr + size:
                    return file_offset + (address - addr)
                section_offset += 80
        offset += cmdsize
    raise SystemExit(f"address not mapped: {address:#x}")

symbol = "__OBJC_CLASS_PROTOCOLS_$_Speaker(Diagnostics)"
symbol_address = find_symbol_address(src, symbol)
protocol_list_offset = va_to_file_offset(data, symbol_address)
struct.pack_into("<Q", data, protocol_list_offset, 0x200)
out.write_bytes(data)
PY

  dup_tmp="$BIN/.dup-build"
  rm -rf "$dup_tmp"
  mkdir -p "$dup_tmp"

  "$CLANG" \
    -arch arm64 \
    -dynamiclib \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -Wl,-install_name,@rpath/libdupalpha.dylib \
    "$SRC/dup-alpha.c" \
    -o "$dup_tmp/libdupalpha.dylib"

  "$CLANG" \
    -arch arm64 \
    -dynamiclib \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    -Wl,-install_name,@rpath/libdupbeta.dylib \
    "$SRC/dup-beta.c" \
    -o "$dup_tmp/libdupbeta.dylib"

  "$CLANG" \
    -arch arm64 \
    -isysroot "$SDKROOT" \
    -mmacosx-version-min=13.0 \
    "$SRC/duplicate-symbol-main.c" \
    -L"$dup_tmp" \
    -ldupalpha \
    -ldupbeta \
    -Wl,-rpath,@loader_path \
    -o "$BIN/duplicate-symbol-ordinal.base"

  head -c 256 "$BIN/arm64-symbolized" > "$BIN/malformed-truncated"

  ROOT_BIN="$BIN" python3 - <<'PY'
from pathlib import Path
import struct
import os

root = Path(os.environ["ROOT_BIN"])
src = root / "import-lazy"
base = bytearray(src.read_bytes())

MH_MAGIC_64 = 0xfeedfacf
LC_SEGMENT_64 = 0x19
LC_DYSYMTAB = 0xB

def parse_layout(buf):
    magic, = struct.unpack_from("<I", buf, 0)
    if magic != MH_MAGIC_64:
        raise SystemExit("unexpected Mach-O magic")

    ncmds, = struct.unpack_from("<I", buf, 16)
    offset = 32
    dysymtab = None
    sections = {}
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from("<II", buf, offset)
        if cmd == LC_DYSYMTAB:
            indirectsymoff = struct.unpack_from("<I", buf, offset + 56)[0]
            dysymtab = {"offset": offset, "indirectsymoff": indirectsymoff}
        elif cmd == LC_SEGMENT_64:
            segname = bytes(buf[offset + 8:offset + 24]).split(b"\x00", 1)[0].decode("ascii", "ignore")
            nsects, = struct.unpack_from("<I", buf, offset + 64)
            section_offset = offset + 72
            for _ in range(nsects):
                sectname = bytes(buf[section_offset:section_offset + 16]).split(b"\x00", 1)[0].decode("ascii", "ignore")
                size, = struct.unpack_from("<Q", buf, section_offset + 40)
                sections[(segname, sectname)] = {"offset": section_offset, "size": size}
                section_offset += 80
        offset += cmdsize
    if dysymtab is None:
        raise SystemExit("LC_DYSYMTAB not found")
    return dysymtab, sections

dysymtab, sections = parse_layout(base)

broken_indirect = bytearray(base)
struct.pack_into("<I", broken_indirect, dysymtab["indirectsymoff"], 0xFFFFFFFE)
(root / "malformed-dysymtab-indirect").write_bytes(broken_indirect)

helper_key = ("__TEXT", "__stub_helper")
if helper_key not in sections:
    raise SystemExit("missing __TEXT,__stub_helper section")
helper = sections[helper_key]
broken_helper = bytearray(base)
struct.pack_into("<Q", broken_helper, helper["offset"] + 40, 8)
(root / "malformed-stub-helper-size").write_bytes(broken_helper)

stub_key = ("__TEXT", "__stubs")
if stub_key not in sections:
    raise SystemExit("missing __TEXT,__stubs section")
stub = sections[stub_key]
broken_stub = bytearray(base)
struct.pack_into("<I", broken_stub, stub["offset"] + 68, 1)
(root / "malformed-stub-reserved2").write_bytes(broken_stub)
PY

  ROOT_BIN="$BIN" python3 - <<'PY'
from pathlib import Path
import struct
import os

root = Path(os.environ["ROOT_BIN"])
base_path = root / "duplicate-symbol-ordinal.base"
out_path = root / "duplicate-symbol-ordinal"
data = bytearray(base_path.read_bytes())

MH_MAGIC_64 = 0xfeedfacf
LC_SYMTAB = 0x2
N_UNDF = 0x0
N_EXT = 0x1

magic, = struct.unpack_from("<I", data, 0)
if magic != MH_MAGIC_64:
    raise SystemExit("unexpected Mach-O magic")

ncmds, = struct.unpack_from("<I", data, 16)
offset = 32
symtab = None
for _ in range(ncmds):
    cmd, cmdsize = struct.unpack_from("<II", data, offset)
    if cmd == LC_SYMTAB:
        symoff, nsyms, stroff, strsize = struct.unpack_from("<IIII", data, offset + 8)
        symtab = (symoff, nsyms, stroff, strsize)
        break
    offset += cmdsize

if symtab is None:
    raise SystemExit("LC_SYMTAB not found")

symoff, nsyms, stroff, strsize = symtab

def symbol_name(n_strx):
    if n_strx >= strsize:
        return ""
    start = stroff + n_strx
    end = data.find(b"\x00", start, stroff + strsize)
    if end < 0:
        end = stroff + strsize
    return data[start:end].decode("utf-8", "ignore")

alpha = None
beta = None
for index in range(nsyms):
    entry = symoff + index * 16
    n_strx, n_type = struct.unpack_from("<IB", data, entry)
    n_type_bits = n_type & 0x0E
    is_external = (n_type & N_EXT) != 0
    if not is_external or n_type_bits != N_UNDF:
        continue
    name = symbol_name(n_strx)
    if name == "_dup_shared_alpha":
        alpha = (entry, n_strx)
    elif name == "_dup_shared_beta":
        beta = (entry, n_strx)

if alpha is None or beta is None:
    raise SystemExit("expected undefined symbols not found")

struct.pack_into("<I", data, beta[0], alpha[1])
out_path.write_bytes(data)
PY

  rm -f "$BIN/duplicate-symbol-ordinal.base"
  rm -rf "$dup_tmp"
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
