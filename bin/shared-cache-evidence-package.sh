#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/shared-cache-evidence-package.sh [options] [-- smoke-options...]

Collects or packages real shared-cache smoke evidence into a directory that can
be copied to another checkout and audited with bin/shared-cache-evidence-audit.sh.
By default it runs bin/shared-cache-real-smoke.sh --report and stores the JSONL
report, smoke log, manifest, checksums, and audit instructions together.

Options:
  --output DIR        Evidence package directory (default: ./damsel-real-cache-evidence-UTC)
  --archive PATH      Also write a .tar.gz/.tgz archive after package verification
  --from-report PATH  Package an existing JSONL report instead of running smoke
  --verify PATH       Verify an existing evidence package directory, .tar.gz archive, or artifact .zip and exit
  -h, --help          Show this help

Smoke options after -- are passed to bin/shared-cache-real-smoke.sh, for example:
  bin/shared-cache-evidence-package.sh -- --cache /path/to/dyld_shared_cache_arm64e
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

  echo "required tool not found: $tool" >&2
  return 1
}

root="$(repo_root)"
output_dir=""
archive_path=""
from_report=""
verify_dir=""
smoke_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      output_dir="$2"
      shift 2
      ;;
    --archive)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      archive_path="$2"
      case "$archive_path" in
        *.tar.gz|*.tgz) ;;
        *)
          echo "--archive path must end in .tar.gz or .tgz: $archive_path" >&2
          exit 1
          ;;
      esac
      shift 2
      ;;
    --from-report)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      from_report="$2"
      shift 2
      ;;
    --verify)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      verify_dir="$2"
      shift 2
      ;;
    --)
      shift
      smoke_args=("$@")
      break
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
      echo "unexpected positional argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

python3_bin="$(find_tool python3)"
if [[ -n "$verify_dir" ]]; then
  if [[ -n "$output_dir" || -n "$archive_path" || -n "$from_report" || ${#smoke_args[@]} -gt 0 ]]; then
    echo "--verify cannot be combined with --output, --archive, --from-report, or smoke options" >&2
    exit 1
  fi
  verify_target="$verify_dir"
  verify_tmpdir=""
  if [[ -f "$verify_target" ]]; then
    case "$verify_target" in
      *.zip)
        verify_tmpdir="$(mktemp -d)"
        trap 'rm -rf "$verify_tmpdir"' EXIT
        verify_target="$("$python3_bin" - "$verify_target" "$verify_tmpdir" <<'PY'
import pathlib
import stat
import sys
import zipfile

zip_path = pathlib.Path(sys.argv[1])
extract_root = pathlib.Path(sys.argv[2]).resolve()

def fail(message):
    print(f"evidence-package: fail: {message}", file=sys.stderr)
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
)"
        "$root/bin/shared-cache-evidence-package.sh" --verify "$verify_target"
        exit 0
        ;;
      *.tar.gz|*.tgz)
        verify_tmpdir="$(mktemp -d)"
        trap 'rm -rf "$verify_tmpdir"' EXIT
        verify_target="$("$python3_bin" - "$verify_target" "$verify_tmpdir" <<'PY'
import pathlib
import sys
import tarfile

archive_path = pathlib.Path(sys.argv[1])
extract_root = pathlib.Path(sys.argv[2]).resolve()

def fail(message):
    print(f"evidence-package: fail: {message}", file=sys.stderr)
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
)"
        ;;
      *)
        echo "evidence-package: fail: verify target is not a package directory, .tar.gz archive, or artifact .zip: $verify_target" >&2
        exit 1
        ;;
    esac
  elif [[ ! -d "$verify_target" ]]; then
    echo "evidence-package: fail: missing package directory, archive, or artifact zip: $verify_target" >&2
    exit 1
  fi

  "$python3_bin" - "$verify_target" <<'PY'
import hashlib
import json
import pathlib
import sys

package_dir = pathlib.Path(sys.argv[1])

def fail(message):
    print(f"evidence-package: fail: {message}", file=sys.stderr)
    raise SystemExit(1)

def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

if not package_dir.is_dir():
    fail(f"missing package directory: {package_dir}")

required = ["evidence.jsonl", "manifest.json", "smoke.log", "README.txt", "SHA256SUMS"]
paths = {name: package_dir / name for name in required}
for name, path in paths.items():
    if not path.is_file():
        fail(f"missing package file: {name}")

checksum_entries = {}
with paths["SHA256SUMS"].open("r", encoding="utf-8") as handle:
    for line_number, raw_line in enumerate(handle, 1):
        line = raw_line.strip()
        if not line:
            continue
        parts = line.split(maxsplit=1)
        if len(parts) != 2:
            fail(f"SHA256SUMS:{line_number}: invalid checksum line")
        checksum, name = parts
        if len(checksum) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in checksum):
            fail(f"SHA256SUMS:{line_number}: invalid sha256 digest")
        checksum_entries[name] = checksum.lower()

for name in ("evidence.jsonl", "manifest.json", "smoke.log", "README.txt"):
    expected = checksum_entries.get(name)
    if expected is None:
        fail(f"SHA256SUMS missing entry for {name}")
    actual = sha256(paths[name])
    if actual != expected:
        fail(f"checksum mismatch for {name}: expected={expected} actual={actual}")

try:
    manifest = json.loads(paths["manifest.json"].read_text(encoding="utf-8"))
except json.JSONDecodeError as error:
    fail(f"manifest.json: invalid JSON: {error}")
if not isinstance(manifest, dict):
    fail("manifest.json: root must be an object")

records = []
with paths["evidence.jsonl"].open("r", encoding="utf-8") as handle:
    for line_number, raw_line in enumerate(handle, 1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            fail(f"evidence.jsonl:{line_number}: invalid JSON: {error}")
        if not isinstance(record, dict):
            fail(f"evidence.jsonl:{line_number}: record must be an object")
        records.append(record)
if not records:
    fail("evidence.jsonl: report is empty")

host_records = [record for record in records if record.get("event") == "host"]
passed_caches = [
    record
    for record in records
    if record.get("event") == "cache" and record.get("status") == "passed"
]
failed_caches = [
    record
    for record in records
    if record.get("event") == "cache" and record.get("status") == "failed"
]
duplicate_caches = [
    record
    for record in records
    if record.get("event") == "cache" and record.get("status") == "skipped_duplicate_uuid"
]

expectations = {
    "schema_version": 1,
    "host_record_count": len(host_records),
    "passed_cache_count": len(passed_caches),
    "failed_cache_count": len(failed_caches),
    "skipped_duplicate_cache_count": len(duplicate_caches),
    "report_sha256": sha256(paths["evidence.jsonl"]),
    "smoke_log_sha256": sha256(paths["smoke.log"]),
}
for key, expected in expectations.items():
    if manifest.get(key) != expected:
        fail(f"manifest mismatch for {key}: expected={expected!r} actual={manifest.get(key)!r}")

passed_cache_uuids = sorted(
    record["cache_uuid"]
    for record in passed_caches
    if isinstance(record.get("cache_uuid"), str)
)
if manifest.get("passed_cache_uuids") != passed_cache_uuids:
    fail("manifest mismatch for passed_cache_uuids")

if len(host_records) != 1:
    fail(f"expected exactly one host record; found {len(host_records)}")
if not passed_caches:
    fail("expected at least one passed cache record")

print("evidence-package: ok")
print(f"  package: {package_dir}")
print(f"  host_product_version: {host_records[0].get('product_version')}")
print(f"  host_build_version: {host_records[0].get('build_version')}")
print(f"  passed_cache_count: {len(passed_caches)}")
for cache_uuid in passed_cache_uuids:
    print(f"  cache_uuid: {cache_uuid}")
PY
  exit 0
fi

generated_at_utc="$(date -u +"%Y-%m-%dT%H%M%SZ")"
if [[ -z "$output_dir" ]]; then
  output_dir="$root/damsel-real-cache-evidence-${generated_at_utc}"
fi
output_dir="${output_dir%/}"

if [[ -e "$output_dir" ]]; then
  echo "output directory already exists: $output_dir" >&2
  exit 1
fi
mkdir -p "$output_dir"

report_path="$output_dir/evidence.jsonl"
smoke_log="$output_dir/smoke.log"
manifest_path="$output_dir/manifest.json"
readme_path="$output_dir/README.txt"
checksums_path="$output_dir/SHA256SUMS"

if [[ -n "$from_report" ]]; then
  if [[ ${#smoke_args[@]} -gt 0 ]]; then
    echo "--from-report cannot be combined with smoke options" >&2
    exit 1
  fi
  if [[ ! -f "$from_report" ]]; then
    echo "missing report: $from_report" >&2
    exit 1
  fi
  cp "$from_report" "$report_path"
  {
    echo "packaged existing report: $from_report"
    echo "generated_at_utc: $generated_at_utc"
  } > "$smoke_log"
else
  set +e
  "$root/bin/shared-cache-real-smoke.sh" --report "$report_path" "${smoke_args[@]}" > "$smoke_log" 2>&1
  smoke_status=$?
  set -e
  cat "$smoke_log"
  if [[ "$smoke_status" -ne 0 ]]; then
    echo "shared-cache real smoke failed; partial package: $output_dir" >&2
    exit "$smoke_status"
  fi
fi

"$python3_bin" - "$root" "$generated_at_utc" "$report_path" "$manifest_path" "$smoke_log" <<'PY'
import hashlib
import json
import pathlib
import subprocess
import sys

root = pathlib.Path(sys.argv[1])
generated_at_utc = sys.argv[2]
report_path = pathlib.Path(sys.argv[3])
manifest_path = pathlib.Path(sys.argv[4])
smoke_log = pathlib.Path(sys.argv[5])

def run(args):
    try:
        return subprocess.check_output(
            args,
            cwd=root,
            stderr=subprocess.DEVNULL,
            text=True,
        ).strip()
    except Exception:
        return None

def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

records = []
with report_path.open("r", encoding="utf-8") as handle:
    for line_number, raw_line in enumerate(handle, 1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise SystemExit(f"{report_path}:{line_number}: invalid JSON: {error}")
        if not isinstance(record, dict):
            raise SystemExit(f"{report_path}:{line_number}: record must be an object")
        records.append(record)

host_records = [record for record in records if record.get("event") == "host"]
passed_caches = [
    record
    for record in records
    if record.get("event") == "cache" and record.get("status") == "passed"
]
failed_caches = [
    record
    for record in records
    if record.get("event") == "cache" and record.get("status") == "failed"
]
duplicate_caches = [
    record
    for record in records
    if record.get("event") == "cache" and record.get("status") == "skipped_duplicate_uuid"
]
summaries = [record for record in records if record.get("event") == "summary"]

cache_samples = []
for record in passed_caches:
    cache_samples.append(
        {
            "cache_uuid": record.get("cache_uuid"),
            "path": record.get("path"),
            "architecture": record.get("architecture"),
            "image_count": record.get("image_count"),
            "member_count": record.get("member_count"),
            "sample_count": record.get("sample_count"),
            "skip_count": record.get("skip_count"),
        }
    )

damsel_bin = pathlib.Path("target/debug/damsel-cli")
damsel_bin_info = None
if damsel_bin.exists():
    damsel_bin_info = {
        "path": str(damsel_bin),
        "sha256": sha256(root / damsel_bin),
    }

git_commit = run(["git", "rev-parse", "HEAD"])
git_dirty = run(["git", "status", "--short"])
workspace_version = None
cargo_toml = root / "Cargo.toml"
if cargo_toml.exists():
    in_workspace_package = False
    for raw_line in cargo_toml.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if line == "[workspace.package]":
            in_workspace_package = True
            continue
        if in_workspace_package and line.startswith("["):
            break
        if in_workspace_package and line.startswith("version"):
            workspace_version = line.split("=", 1)[1].strip().strip('"')
            break

manifest = {
    "schema_version": 1,
    "generated_at_utc": generated_at_utc,
    "workspace_version": workspace_version,
    "git_commit": git_commit,
    "git_dirty": bool(git_dirty),
    "host": host_records[0] if host_records else None,
    "host_record_count": len(host_records),
    "passed_cache_count": len(passed_caches),
    "failed_cache_count": len(failed_caches),
    "skipped_duplicate_cache_count": len(duplicate_caches),
    "passed_cache_uuids": sorted(
        record["cache_uuid"]
        for record in passed_caches
        if isinstance(record.get("cache_uuid"), str)
    ),
    "skipped_duplicate_cache_uuids": sorted(
        record["cache_uuid"]
        for record in duplicate_caches
        if isinstance(record.get("cache_uuid"), str)
    ),
    "failed_cache_uuids": sorted(
        record["cache_uuid"]
        for record in failed_caches
        if isinstance(record.get("cache_uuid"), str)
    ),
    "cache_samples": cache_samples,
    "summary": summaries[-1] if summaries else None,
    "report_sha256": sha256(report_path),
    "smoke_log_sha256": sha256(smoke_log),
    "damsel_bin": damsel_bin_info,
    "audit_command": "bin/shared-cache-evidence-audit.sh /path/to/this-package-or-archive /path/to/other-package-or-archive",
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

cat > "$readme_path" <<EOF
Damsel real shared-cache evidence package

Generated: $generated_at_utc

Files:
- evidence.jsonl: machine-readable report from bin/shared-cache-real-smoke.sh --report
- manifest.json: derived host/cache summary and checksums
- smoke.log: terminal output captured during collection or packaging
- SHA256SUMS: checksums for package files

Audit after copying this directory or the optional archive next to another
evidence package:

  bin/shared-cache-evidence-audit.sh "$output_dir" /path/to/other-package-or-archive

The default audit requires at least two independent host/release evidence keys,
two unique passed cache UUIDs, and six sampled projected images per passed cache.
EOF

(
  cd "$output_dir"
  shasum -a 256 evidence.jsonl manifest.json smoke.log README.txt > "$(basename "$checksums_path")"
)

"$root/bin/shared-cache-evidence-package.sh" --verify "$output_dir"

echo "evidence package: $output_dir"
echo "report: $report_path"
echo "manifest: $manifest_path"

if [[ -n "$archive_path" ]]; then
  archive_path="${archive_path%/}"
  if [[ -e "$archive_path" ]]; then
    echo "archive already exists: $archive_path" >&2
    exit 1
  fi

  archive_parent="$(dirname "$archive_path")"
  mkdir -p "$archive_parent"
  output_abs="$(cd "$output_dir" && pwd)"
  archive_parent_abs="$(cd "$archive_parent" && pwd)"
  case "$archive_parent_abs/" in
    "$output_abs"/*|"$output_abs/")
      echo "archive path must not be inside the evidence package: $archive_path" >&2
      exit 1
      ;;
  esac

  COPYFILE_DISABLE=1 tar -czf "$archive_path" -C "$(dirname "$output_abs")" "$(basename "$output_abs")"
  archive_listing="$(mktemp)"
  tar -tzf "$archive_path" > "$archive_listing"
  missing_archive_file=""
  for package_file in evidence.jsonl manifest.json smoke.log README.txt SHA256SUMS; do
    if ! grep -Fxq -- "$(basename "$output_abs")/$package_file" "$archive_listing"; then
      missing_archive_file="$package_file"
      break
    fi
  done
  rm -f "$archive_listing"
  if [[ -n "$missing_archive_file" ]]; then
    echo "archive missing package file: $missing_archive_file" >&2
    exit 1
  fi
  archive_sha256="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
  echo "archive: $archive_path"
  echo "archive_sha256: $archive_sha256"
fi
