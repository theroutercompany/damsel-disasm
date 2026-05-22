#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/shared-cache-evidence-audit.sh [options] REPORT.jsonl-or-PACKAGE-DIR-or-ARCHIVE...

Audits real shared-cache smoke JSON Lines reports produced by
bin/shared-cache-real-smoke.sh --report, evidence package directories, or
.tar.gz package archives produced by bin/shared-cache-evidence-package.sh.
Downloaded GitHub artifact .zip wrappers are also accepted when they contain
exactly one evidence .tar.gz/.tgz archive. Package directories, archives, and
artifact zips are verified before their evidence.jsonl reports are audited. The
default thresholds model the public-beta breadth gate:
at least two independent host/release runs and at least two unique passed cache
UUIDs, with at least six sampled images per cache.

Options:
  --min-independent-runs N   Required unique host/release evidence keys (default: 2)
  --min-unique-cache-uuids N Required unique passed cache UUIDs (default: 2)
  --min-samples N           Required sampled images per passed cache (default: 6)
  -h, --help                Show this help
EOF
}

repo_root() {
  CDPATH= cd -- "$(dirname "$0")/.." && pwd
}

min_independent_runs=2
min_unique_cache_uuids=2
min_samples=6
reports=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --min-independent-runs)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      min_independent_runs="${2:-}"
      shift 2
      ;;
    --min-unique-cache-uuids)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      min_unique_cache_uuids="${2:-}"
      shift 2
      ;;
    --min-samples)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      min_samples="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
    *)
      reports+=("$1")
      shift
      ;;
  esac
done

if [[ ${#reports[@]} -eq 0 ]]; then
  echo "at least one report path or package directory is required" >&2
  usage >&2
  exit 1
fi

root="$(repo_root)"
audit_tmpdir="$(mktemp -d)"
trap 'rm -rf "$audit_tmpdir"' EXIT

extract_evidence_archive() {
  local archive_path="$1"
  local extract_root="$2"

  python3 - "$archive_path" "$extract_root" <<'PY'
import pathlib
import sys
import tarfile

archive_path = pathlib.Path(sys.argv[1])
extract_root = pathlib.Path(sys.argv[2]).resolve()

def fail(message):
    print(f"evidence-audit: fail: {message}", file=sys.stderr)
    raise SystemExit(1)

def safe_child(path, root):
    try:
        path.resolve().relative_to(root)
    except ValueError:
        return False
    return True

try:
    archive = tarfile.open(archive_path, "r:gz")
except (tarfile.TarError, OSError) as error:
    fail(f"invalid evidence archive: {error}")

with archive:
    members = archive.getmembers()
    if not members:
        fail("evidence archive is empty")

    top_levels = set()
    file_names = set()
    for member in members:
        path = pathlib.PurePosixPath(member.name)
        parts = path.parts
        if (
            not parts
            or member.name.startswith("/")
            or any(part in ("", ".", "..") for part in parts)
        ):
            fail(f"unsafe archive entry: {member.name}")
        if not (member.isfile() or member.isdir()):
            fail(f"unsupported archive member type: {member.name}")
        destination = extract_root.joinpath(*parts)
        if not safe_child(destination, extract_root):
            fail(f"unsafe archive entry: {member.name}")
        top_levels.add(parts[0])
        if member.isfile():
            file_names.add(member.name)

    if len(top_levels) != 1:
        rendered = ", ".join(sorted(top_levels))
        fail(f"expected exactly one top-level package directory in archive; found: {rendered}")

    package_root = next(iter(top_levels))
    required = ["evidence.jsonl", "manifest.json", "smoke.log", "README.txt", "SHA256SUMS"]
    for name in required:
        expected_name = f"{package_root}/{name}"
        if expected_name not in file_names:
            fail(f"archive missing package file: {name}")

    try:
        archive.extractall(extract_root, members=members, filter="data")
    except TypeError:
        archive.extractall(extract_root, members=members)

print(extract_root / package_root)
PY
}

extract_artifact_zip() {
  local zip_path="$1"
  local extract_root="$2"

  python3 - "$zip_path" "$extract_root" <<'PY'
import pathlib
import stat
import sys
import zipfile

zip_path = pathlib.Path(sys.argv[1])
extract_root = pathlib.Path(sys.argv[2]).resolve()

def fail(message):
    print(f"evidence-audit: fail: {message}", file=sys.stderr)
    raise SystemExit(1)

def safe_child(path, root):
    try:
        path.resolve().relative_to(root)
    except ValueError:
        return False
    return True

try:
    archive = zipfile.ZipFile(zip_path)
except (zipfile.BadZipFile, OSError) as error:
    fail(f"invalid artifact zip: {error}")

with archive:
    infos = archive.infolist()
    if not infos:
        fail("artifact zip is empty")

    evidence_archives = []
    for info in infos:
        name = info.filename
        path = pathlib.PurePosixPath(name)
        parts = path.parts
        if (
            not parts
            or name.startswith("/")
            or "\\" in name
            or any(part in ("", ".", "..") for part in parts)
        ):
            fail(f"unsafe artifact zip entry: {name}")
        mode = (info.external_attr >> 16) & 0o170000
        if mode and mode not in (stat.S_IFREG, stat.S_IFDIR):
            fail(f"unsupported artifact zip member type: {name}")
        destination = extract_root.joinpath(*parts)
        if not safe_child(destination, extract_root):
            fail(f"unsafe artifact zip entry: {name}")
        if not info.is_dir() and name.endswith((".tar.gz", ".tgz")):
            evidence_archives.append(name)

    if len(evidence_archives) != 1:
        rendered = ", ".join(sorted(evidence_archives)) or "none"
        fail(f"expected exactly one evidence archive in artifact zip; found: {rendered}")

    archive.extractall(extract_root)

print(extract_root.joinpath(*pathlib.PurePosixPath(evidence_archives[0]).parts))
PY
}

resolved_reports=()
archive_count=0
for report in "${reports[@]}"; do
  if [[ -d "$report" ]]; then
    "$root/bin/shared-cache-evidence-package.sh" --verify "$report" >/dev/null
    resolved_reports+=("$report/evidence.jsonl")
  elif [[ -f "$report" ]]; then
    case "$report" in
      *.zip)
        archive_count=$((archive_count + 1))
        zip_root="$audit_tmpdir/artifact-${archive_count}"
        extract_root="$audit_tmpdir/archive-${archive_count}"
        mkdir -p "$zip_root" "$extract_root"
        inner_archive="$(extract_artifact_zip "$report" "$zip_root")"
        package_dir="$(extract_evidence_archive "$inner_archive" "$extract_root")"
        "$root/bin/shared-cache-evidence-package.sh" --verify "$package_dir" >/dev/null
        resolved_reports+=("$package_dir/evidence.jsonl")
        ;;
      *.tar.gz|*.tgz)
        archive_count=$((archive_count + 1))
        extract_root="$audit_tmpdir/archive-${archive_count}"
        mkdir -p "$extract_root"
        package_dir="$(extract_evidence_archive "$report" "$extract_root")"
        "$root/bin/shared-cache-evidence-package.sh" --verify "$package_dir" >/dev/null
        resolved_reports+=("$package_dir/evidence.jsonl")
        ;;
      *)
        resolved_reports+=("$report")
        ;;
    esac
  else
    resolved_reports+=("$report")
  fi
done

python3 - "$min_independent_runs" "$min_unique_cache_uuids" "$min_samples" "${resolved_reports[@]}" <<'PY'
import json
import pathlib
import sys

def fail(message):
    print(f"evidence-audit: fail: {message}", file=sys.stderr)
    raise SystemExit(1)

def parse_positive_int(name, value):
    try:
        parsed = int(value)
    except (TypeError, ValueError):
        fail(f"{name} must be an integer")
    if parsed <= 0:
        fail(f"{name} must be greater than zero")
    return parsed

min_independent_runs = parse_positive_int("--min-independent-runs", sys.argv[1])
min_unique_cache_uuids = parse_positive_int("--min-unique-cache-uuids", sys.argv[2])
min_samples = parse_positive_int("--min-samples", sys.argv[3])
report_paths = [pathlib.Path(path) for path in sys.argv[4:]]

def load_report(path):
    if not path.is_file():
        fail(f"missing report: {path}")
    records = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, raw_line in enumerate(handle, 1):
            line = raw_line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as error:
                fail(f"{path}:{line_number}: invalid JSON: {error}")
            if not isinstance(record, dict):
                fail(f"{path}:{line_number}: record must be an object")
            record["_report_path"] = str(path)
            record["_line_number"] = line_number
            records.append(record)
    if not records:
        fail(f"{path}: report is empty")
    return records

def cache_uuid(record):
    value = record.get("cache_uuid")
    if not isinstance(value, str) or not value:
        fail(
            f"{record.get('_report_path')}:{record.get('_line_number')}: "
            "passed cache record must include a non-empty cache_uuid"
        )
    return value

def host_key(host_record):
    product_version = host_record.get("product_version") or "unknown-version"
    build_version = host_record.get("build_version") or "unknown-build"
    uname = host_record.get("uname") or ""
    uname_parts = uname.split()
    hostname = uname_parts[1] if len(uname_parts) > 1 else "unknown-host"
    return (product_version, build_version, hostname)

def sample_count(record):
    value = record.get("sample_count", 0)
    try:
        return int(value)
    except (TypeError, ValueError):
        fail(
            f"{record.get('_report_path')}:{record.get('_line_number')}: "
            f"passed cache record has invalid sample_count: {value!r}"
        )

all_records = []
report_count = 0
for report_path in report_paths:
    records = load_report(report_path)
    report_hosts = [record for record in records if record.get("event") == "host"]
    if len(report_hosts) != 1:
        fail(
            f"expected exactly one host record in {report_path}; "
            f"found {len(report_hosts)}"
        )
    report_passed_caches = [
        record
        for record in records
        if record.get("event") == "cache" and record.get("status") == "passed"
    ]
    if not report_passed_caches:
        fail(f"{report_path}: no passed cache records found")
    report_count += 1
    all_records.extend(records)

host_records = [record for record in all_records if record.get("event") == "host"]
passed_caches = [
    record
    for record in all_records
    if record.get("event") == "cache" and record.get("status") == "passed"
]
failed_caches = [
    record
    for record in all_records
    if record.get("event") == "cache" and record.get("status") == "failed"
]
if failed_caches:
    failures = ", ".join(
        f"{record.get('cache_uuid', '?')} from {record.get('_report_path')}"
        for record in failed_caches
    )
    fail(f"failed cache records present: {failures}")

passed_cache_rows = [
    (record, cache_uuid(record), sample_count(record))
    for record in passed_caches
]
sample_failures = [
    (record, count)
    for record, _uuid, count in passed_cache_rows
    if count < min_samples
]
if sample_failures:
    details = ", ".join(
        f"{record.get('cache_uuid', '?')} sample_count={count}"
        for record, count in sample_failures
    )
    fail(f"passed cache records below sample threshold {min_samples}: {details}")

host_keys = {host_key(record) for record in host_records}
cache_uuids = {uuid for _record, uuid, _count in passed_cache_rows}

if len(host_keys) < min_independent_runs:
    rendered = "; ".join("/".join(key) for key in sorted(host_keys))
    fail(
        f"independent host/release evidence is insufficient: "
        f"{len(host_keys)} < {min_independent_runs}; observed={rendered}"
    )
if len(cache_uuids) < min_unique_cache_uuids:
    rendered = ", ".join(sorted(cache_uuids))
    fail(
        f"unique passed cache UUID evidence is insufficient: "
        f"{len(cache_uuids)} < {min_unique_cache_uuids}; observed={rendered}"
    )

print("evidence-audit: ok")
print(f"  reports: {report_count}")
print(f"  independent_host_release_runs: {len(host_keys)}")
print(f"  unique_passed_cache_uuids: {len(cache_uuids)}")
print(f"  passed_cache_records: {len(passed_caches)}")
for product_version, build_version, hostname in sorted(host_keys):
    print(f"  host: product_version={product_version} build={build_version} host={hostname}")
for cache_uuid in sorted(cache_uuids):
    print(f"  cache_uuid: {cache_uuid}")
PY
