#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/shared-cache-real-smoke.sh [--cache PATH]... [--include-duplicates] [--report PATH]

Runs the opt-in real shared-cache smoke against local dyld cache roots. Cache
UUIDs are detected before tests run; duplicate UUIDs are skipped by default so
operator notes do not accidentally count duplicate paths as breadth.

When --report is supplied, the script also writes JSON Lines evidence containing
host metadata, cache UUIDs, sample rows, duplicate-skip decisions, and summary
counts. Use this for cross-host or cross-release beta evidence collection.

Environment:
  DAMSEL_BIN=/path/to/damsel-cli   Use a prebuilt CLI for cache info probing.
  DAMSEL_REAL_DYLD_SHARED_CACHE_IMAGE_SAMPLE_LIMIT=6
  DAMSEL_REAL_DYLD_SHARED_CACHE_INSTRUCTION_LIMIT=8
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
include_duplicates="false"
report_path=""
caches=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cache)
      caches+=("${2:-}")
      shift 2
      ;;
    --include-duplicates)
      include_duplicates="true"
      shift
      ;;
    --report)
      report_path="${2:-}"
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

if [[ ${#caches[@]} -eq 0 ]]; then
  candidates=(
    "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e"
    "/System/Volumes/Preboot/Cryptexes/OS/System/DriverKit/System/Library/dyld/dyld_shared_cache_arm64e"
    "/System/Volumes/Preboot/Cryptexes/Incoming/OS/System/Library/dyld/dyld_shared_cache_arm64e"
    "/System/Volumes/Preboot/Cryptexes/Incoming/OS/System/DriverKit/System/Library/dyld/dyld_shared_cache_arm64e"
  )
  for candidate in "${candidates[@]}"; do
    if [[ -f "$candidate" ]]; then
      caches+=("$candidate")
    fi
  done
fi

if [[ ${#caches[@]} -eq 0 ]]; then
  echo "no dyld shared-cache roots found; pass --cache PATH to validate a custom root" >&2
  exit 1
fi
if [[ -n "$report_path" ]]; then
  mkdir -p "$(dirname "$report_path")"
fi

python3_bin="$(find_tool python3)"
tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/damsel-real-cache-smoke.XXXXXX")"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM
seen_uuid_file="$tmpdir/seen-uuids.txt"
: > "$seen_uuid_file"
report_tmp="$tmpdir/report.jsonl"
if [[ -n "$report_path" ]]; then
  : > "$report_tmp"
fi

sync_report() {
  if [[ -n "$report_path" ]]; then
    cp "$report_tmp" "$report_path"
  fi
}

append_host_report() {
  if [[ -z "$report_path" ]]; then
    return
  fi

  local product_name=""
  local product_version=""
  local build_version=""
  local uname_value=""
  local generated_at_utc=""

  generated_at_utc="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  uname_value="$(uname -a 2>/dev/null || true)"
  if command -v sw_vers >/dev/null 2>&1; then
    product_name="$(sw_vers -productName 2>/dev/null || true)"
    product_version="$(sw_vers -productVersion 2>/dev/null || true)"
    build_version="$(sw_vers -buildVersion 2>/dev/null || true)"
  fi

  "$python3_bin" - "$report_tmp" "$generated_at_utc" "$product_name" "$product_version" "$build_version" "$uname_value" "$DAMSEL_REAL_DYLD_SHARED_CACHE_IMAGE_SAMPLE_LIMIT" "$DAMSEL_REAL_DYLD_SHARED_CACHE_INSTRUCTION_LIMIT" <<'PY'
import json
import sys

(
    report_path,
    generated_at_utc,
    product_name,
    product_version,
    build_version,
    uname_value,
    sample_limit,
    instruction_limit,
) = sys.argv[1:9]
record = {
    "event": "host",
    "generated_at_utc": generated_at_utc,
    "product_name": product_name or None,
    "product_version": product_version or None,
    "build_version": build_version or None,
    "uname": uname_value or None,
    "sample_limit": int(sample_limit),
    "instruction_limit": int(instruction_limit),
}
with open(report_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True) + "\n")
PY
  sync_report
}

append_duplicate_report() {
  if [[ -z "$report_path" ]]; then
    return
  fi

  local cache_path="$1"
  local cache_uuid="$2"
  local cache_arch="$3"
  local image_count="$4"
  local member_count="$5"

  "$python3_bin" - "$report_tmp" "$cache_path" "$cache_uuid" "$cache_arch" "$image_count" "$member_count" <<'PY'
import json
import sys

report_path, cache_path, cache_uuid, cache_arch, image_count, member_count = sys.argv[1:7]
record = {
    "event": "cache",
    "status": "skipped_duplicate_uuid",
    "path": cache_path,
    "cache_uuid": cache_uuid,
    "architecture": cache_arch,
    "image_count": int(image_count),
    "member_count": int(member_count),
}
with open(report_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True) + "\n")
PY
  sync_report
}

append_cache_report() {
  if [[ -z "$report_path" ]]; then
    return
  fi

  local cache_path="$1"
  local cache_uuid="$2"
  local cache_arch="$3"
  local image_count="$4"
  local member_count="$5"
  local exit_code="$6"
  local output_path="$7"

  "$python3_bin" - "$report_tmp" "$cache_path" "$cache_uuid" "$cache_arch" "$image_count" "$member_count" "$exit_code" "$output_path" <<'PY'
import json
import re
import sys

(
    report_path,
    cache_path,
    cache_uuid,
    cache_arch,
    image_count,
    member_count,
    exit_code,
    output_path,
) = sys.argv[1:9]

sample_re = re.compile(
    r"^sample image=(?P<image>.*?) section=(?P<section>.*?) "
    r"instructions=(?P<instructions>\d+) blocks=(?P<blocks>\d+) "
    r"edges=(?P<edges>\d+) exports=(?P<exports>\d+) "
    r"dependencies=(?P<dependencies>\d+) reexports=(?P<reexports>\d+)$"
)
skip_re = re.compile(r"^skip image=(?P<image>.*?) reason=(?P<reason>.*)$")
samples = []
skips = []
with open(output_path, "r", encoding="utf-8", errors="replace") as handle:
    for raw_line in handle:
        line = raw_line.strip()
        sample_match = sample_re.match(line)
        if sample_match:
            item = sample_match.groupdict()
            for key in ("instructions", "blocks", "edges", "exports", "dependencies", "reexports"):
                item[key] = int(item[key])
            samples.append(item)
            continue
        skip_match = skip_re.match(line)
        if skip_match:
            skips.append(skip_match.groupdict())

exit_code = int(exit_code)
record = {
    "event": "cache",
    "status": "passed" if exit_code == 0 else "failed",
    "path": cache_path,
    "cache_uuid": cache_uuid,
    "architecture": cache_arch,
    "image_count": int(image_count),
    "member_count": int(member_count),
    "sample_count": len(samples),
    "skip_count": len(skips),
    "samples": samples,
    "skips": skips,
}
with open(report_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True) + "\n")
PY
  sync_report
}

append_summary_report() {
  if [[ -z "$report_path" ]]; then
    return
  fi

  local validated_count="$1"
  local unique_uuid_count="$2"

  "$python3_bin" - "$report_tmp" "$validated_count" "$unique_uuid_count" <<'PY'
import json
import sys

report_path, validated_count, unique_uuid_count = sys.argv[1:4]
record = {
    "event": "summary",
    "validated_unique_cache_count": int(validated_count),
    "seen_unique_cache_uuid_count": int(unique_uuid_count),
}
with open(report_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True) + "\n")
PY
  sync_report
}

run_damsel_cache_info() {
  local cache_path="$1"

  if [[ -n "${DAMSEL_BIN:-}" ]]; then
    "$DAMSEL_BIN" --format json cache info "$cache_path"
  else
    cargo run -q -p damsel-cli --manifest-path "$root/Cargo.toml" -- \
      --format json cache info "$cache_path"
  fi
}

: "${DAMSEL_REAL_DYLD_SHARED_CACHE_IMAGE_SAMPLE_LIMIT:=6}"
: "${DAMSEL_REAL_DYLD_SHARED_CACHE_INSTRUCTION_LIMIT:=8}"
export DAMSEL_REAL_DYLD_SHARED_CACHE_IMAGE_SAMPLE_LIMIT
export DAMSEL_REAL_DYLD_SHARED_CACHE_INSTRUCTION_LIMIT

append_host_report
validated=0

for cache_path in "${caches[@]}"; do
  if [[ -z "$cache_path" ]]; then
    echo "empty cache path" >&2
    exit 1
  fi
  if [[ ! -f "$cache_path" ]]; then
    echo "missing cache: $cache_path" >&2
    exit 1
  fi

  info_path="$tmpdir/cache-info-$validated.json"
  run_damsel_cache_info "$cache_path" > "$info_path"
  info_line="$("$python3_bin" - "$info_path" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    payload = json.load(handle)

header = payload["data"]["header"]
print(
    "\t".join(
        [
            header["cache_uuid"],
            header["architecture"],
            str(header["image_count"]),
            str(header["member_count"]),
        ]
    )
)
PY
)"
  IFS=$'\t' read -r cache_uuid cache_arch image_count member_count <<< "$info_line"

  if [[ "$include_duplicates" != "true" ]] && grep -Fqx "$cache_uuid" "$seen_uuid_file"; then
    echo "skip duplicate cache UUID $cache_uuid: $cache_path"
    append_duplicate_report "$cache_path" "$cache_uuid" "$cache_arch" "$image_count" "$member_count"
    continue
  fi
  printf '%s\n' "$cache_uuid" >> "$seen_uuid_file"

  echo "smoke cache UUID $cache_uuid arch=$cache_arch images=$image_count members=$member_count sample_limit=$DAMSEL_REAL_DYLD_SHARED_CACHE_IMAGE_SAMPLE_LIMIT instruction_limit=$DAMSEL_REAL_DYLD_SHARED_CACHE_INSTRUCTION_LIMIT"
  echo "  path: $cache_path"
  output_path="$tmpdir/cache-test-output-$validated.txt"
  set +e
  DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT="$cache_path" \
    cargo test -p damsel-macho --manifest-path "$root/Cargo.toml" --test shared_cache_real_env -- --nocapture 2>&1 | tee "$output_path"
  test_status="${PIPESTATUS[0]}"
  set -e
  append_cache_report "$cache_path" "$cache_uuid" "$cache_arch" "$image_count" "$member_count" "$test_status" "$output_path"
  if [[ "$test_status" -ne 0 ]]; then
    exit "$test_status"
  fi
  validated=$((validated + 1))
done

if [[ "$validated" -eq 0 ]]; then
  echo "no unique cache UUIDs validated" >&2
  exit 1
fi

unique_uuid_count="$(wc -l < "$seen_uuid_file" | tr -d ' ')"
append_summary_report "$validated" "$unique_uuid_count"
echo "validated $validated unique shared-cache UUID(s)"
if [[ -n "$report_path" ]]; then
  echo "report: $report_path"
fi
