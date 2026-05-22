#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/disasm-external-compare.sh [--fixture NAME[:LIMIT]]...

Compares Damsel fixture disassembly against LLVM Mach-O tools. The check is
semantic rather than byte-for-byte: instruction addresses, mnemonics, and direct
branch/call targets must agree, while Damsel-specific annotations may differ.

Environment:
  DAMSEL_BIN=/path/to/damsel-cli   Use a prebuilt CLI instead of cargo run.
EOF
}

repo_root() {
  CDPATH= cd -- "$(dirname "$0")/.." && pwd
}

find_tool() {
  local tool="$1"

  if command -v "$tool" >/dev/null 2>&1; then
    command -v "$tool"
    return 0
  fi

  if command -v xcrun >/dev/null 2>&1; then
    if xcrun --find "$tool" >/dev/null 2>&1; then
      xcrun --find "$tool"
      return 0
    fi
  fi

  echo "required tool not found: $tool" >&2
  return 1
}

root="$(repo_root)"
fixtures=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fixture)
      fixtures+=("${2:-}")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ${#fixtures[@]} -eq 0 ]]; then
  fixtures=(
    "semantic-switch:32"
    "import-rich:24"
    "indirect-dispatch:20"
    "relative-dispatch:16"
    "objc-sample:20"
    "swift-sample:20"
    "arm64e-sample:16"
  )
fi

python3_bin="$(find_tool python3)"
objdump_bin="$(find_tool llvm-objdump)"
otool_bin="$(find_tool llvm-otool)"

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/damsel-disasm-compare.XXXXXX")"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM

run_damsel_json() {
  local fixture_path="$1"
  local limit="$2"

  if [[ -n "${DAMSEL_BIN:-}" ]]; then
    "$DAMSEL_BIN" --format json disasm "$fixture_path" --section __text --limit "$limit"
  else
    cargo run -q -p damsel-cli --manifest-path "$root/Cargo.toml" -- \
      --format json disasm "$fixture_path" --section __text --limit "$limit"
  fi
}

for fixture_spec in "${fixtures[@]}"; do
  if [[ -z "$fixture_spec" ]]; then
    echo "empty fixture spec" >&2
    exit 1
  fi

  fixture_name="${fixture_spec%%:*}"
  limit="${fixture_spec#*:}"
  if [[ "$fixture_name" == "$limit" ]]; then
    limit="24"
  fi

  fixture_path="$root/fixtures/bin/$fixture_name"
  if [[ ! -f "$fixture_path" ]]; then
    echo "missing fixture: $fixture_name ($fixture_path)" >&2
    exit 1
  fi

  prefix="$tmpdir/$fixture_name"
  run_damsel_json "$fixture_path" "$limit" > "$prefix.damsel.json"
  "$objdump_bin" --disassemble --section=__TEXT,__text --macho "$fixture_path" > "$prefix.objdump.txt"
  "$otool_bin" -tV "$fixture_path" > "$prefix.otool.txt"

  "$python3_bin" - "$fixture_name" "$prefix.damsel.json" "$prefix.objdump.txt" "$prefix.otool.txt" <<'PY'
import json
import re
import sys

fixture_name, damsel_path, objdump_path, otool_path = sys.argv[1:5]

DIRECT_BRANCHES = {
    "b",
    "bl",
    "cbz",
    "cbnz",
    "tbz",
    "tbnz",
}


def die(message):
    print(f"{fixture_name}: {message}", file=sys.stderr)
    sys.exit(1)


def normalize_mnemonic(value):
    return value.strip().lower()


def first_hex_target(value):
    match = re.search(r"#?(0x[0-9a-fA-F]+)", value)
    if match:
        return int(match.group(1), 16)
    return None


def parse_objdump(text):
    entries = {}
    for raw_line in text.splitlines():
        match = re.match(r"^\s*([0-9a-fA-F]+):\s*(.*)$", raw_line)
        if not match:
            continue
        address = int(match.group(1), 16)
        remainder = match.group(2).strip()
        parts = [part.strip() for part in remainder.split("\t") if part.strip()]
        if not parts:
            continue
        if len(parts) >= 2:
            asm = " ".join(parts[1:])
        else:
            asm = parts[0]
        mnemonic = asm.split(None, 1)[0] if asm else ""
        if not mnemonic:
            continue
        entries[address] = {
            "mnemonic": normalize_mnemonic(mnemonic),
            "text": asm,
        }
    return entries


def parse_otool(text):
    entries = {}
    for raw_line in text.splitlines():
        match = re.match(r"^\s*([0-9a-fA-F]{8,16})\s+([A-Za-z][A-Za-z0-9.]*)\b(.*)$", raw_line)
        if not match:
            continue
        address = int(match.group(1), 16)
        mnemonic = normalize_mnemonic(match.group(2))
        entries[address] = {
            "mnemonic": mnemonic,
            "text": f"{match.group(2)}{match.group(3)}".strip(),
        }
    return entries


def compare_tool(tool_name, tool_entries, damsel_instructions):
    for instruction in damsel_instructions:
        address = instruction["address"]
        mnemonic = normalize_mnemonic(instruction["mnemonic"])
        tool_entry = tool_entries.get(address)
        if tool_entry is None:
            die(f"{tool_name} missing instruction at 0x{address:x}")
        if tool_entry["mnemonic"] != mnemonic:
            die(
                f"{tool_name} mnemonic mismatch at 0x{address:x}: "
                f"damsel={mnemonic} {tool_name}={tool_entry['mnemonic']}"
            )

        is_direct_branch = mnemonic in DIRECT_BRANCHES or mnemonic.startswith("b.")
        if is_direct_branch:
            damsel_target = first_hex_target(instruction.get("rendered", ""))
            tool_target = first_hex_target(tool_entry["text"])
            if damsel_target is not None and tool_target is not None and damsel_target != tool_target:
                die(
                    f"{tool_name} branch target mismatch at 0x{address:x}: "
                    f"damsel=0x{damsel_target:x} {tool_name}=0x{tool_target:x}"
                )


with open(damsel_path, "r", encoding="utf-8") as handle:
    damsel_payload = json.load(handle)

damsel_data = damsel_payload.get("data", {})
damsel_instructions = damsel_data.get("instructions", [])
if not damsel_instructions:
    die("Damsel returned no instructions")

with open(objdump_path, "r", encoding="utf-8") as handle:
    objdump_entries = parse_objdump(handle.read())
with open(otool_path, "r", encoding="utf-8") as handle:
    otool_entries = parse_otool(handle.read())

if not objdump_entries:
    die("llvm-objdump returned no parseable instructions")
if not otool_entries:
    die("llvm-otool returned no parseable instructions")

compare_tool("llvm-objdump", objdump_entries, damsel_instructions)
compare_tool("llvm-otool", otool_entries, damsel_instructions)

first_address = damsel_instructions[0]["address"]
last_address = damsel_instructions[-1]["address"]
print(
    f"ok {fixture_name}: {len(damsel_instructions)} instructions "
    f"0x{first_address:x}..0x{last_address:x} matched llvm-objdump and llvm-otool"
)
PY
done
