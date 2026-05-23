#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_script="${root}/bin/shared-cache-evidence-package.sh"
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

report="${tmpdir}/evidence-source.jsonl"
package_dir="${tmpdir}/package"
archive_path="${tmpdir}/package.tar.gz"

cat >"${report}" <<'JSONL'
{"build_version":"25F71","event":"host","generated_at_utc":"2026-05-22T00:00:00Z","instruction_limit":8,"product_name":"macOS","product_version":"26.5","sample_limit":6,"uname":"Darwin alpha-host 25.0.0 Darwin Kernel Version 25.0.0: root:xnu/RELEASE_ARM64 arm64"}
{"architecture":"arm64e","cache_uuid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","event":"cache","image_count":100,"member_count":2,"path":"/System/Library/dyld/dyld_shared_cache_arm64e","sample_count":6,"samples":[{"blocks":1,"dependencies":1,"edges":0,"exports":1,"image":"/usr/lib/libalpha.dylib","instructions":8,"reexports":0,"section":"__text"}],"skip_count":0,"skips":[],"status":"passed"}
{"architecture":"arm64e","cache_uuid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","event":"cache","image_count":100,"member_count":2,"path":"/System/Library/dyld/dyld_shared_cache_arm64e.duplicate","status":"skipped_duplicate_uuid"}
{"event":"summary","seen_unique_cache_uuid_count":1,"validated_unique_cache_count":1}
JSONL

package_output="${tmpdir}/package.out"
"${package_script}" --from-report "${report}" --output "${package_dir}" --archive "${archive_path}" >"${package_output}"

test -s "${package_dir}/evidence.jsonl"
test -s "${package_dir}/manifest.json"
test -s "${package_dir}/smoke.log"
test -s "${package_dir}/README.txt"
test -s "${package_dir}/SHA256SUMS"
test -s "${archive_path}"
cmp "${report}" "${package_dir}/evidence.jsonl"
grep -Fq "evidence-package: ok" "${package_output}"
grep -Fq "archive: ${archive_path}" "${package_output}"
tar -tzf "${archive_path}" >"${tmpdir}/archive-list.txt"
grep -Fxq "package/evidence.jsonl" "${tmpdir}/archive-list.txt"
grep -Fxq "package/manifest.json" "${tmpdir}/archive-list.txt"
grep -Fxq "package/smoke.log" "${tmpdir}/archive-list.txt"
grep -Fxq "package/README.txt" "${tmpdir}/archive-list.txt"
grep -Fxq "package/SHA256SUMS" "${tmpdir}/archive-list.txt"

(
  cd "${package_dir}"
  shasum -a 256 -c SHA256SUMS >/dev/null
)
verify_output="${tmpdir}/verify.out"
"${package_script}" --verify "${package_dir}" >"${verify_output}"
grep -Fq "evidence-package: ok" "${verify_output}"
archive_verify_output="${tmpdir}/archive-verify.out"
"${package_script}" --verify "${archive_path}" >"${archive_verify_output}"
grep -Fq "evidence-package: ok" "${archive_verify_output}"
artifact_zip="${tmpdir}/artifact.zip"
python3 - "${artifact_zip}" "${archive_path}" <<'PY'
import pathlib
import sys
import zipfile

artifact_zip = pathlib.Path(sys.argv[1])
archive_path = pathlib.Path(sys.argv[2])
with zipfile.ZipFile(artifact_zip, "w") as archive:
    archive.write(archive_path, archive_path.name)
PY
artifact_verify_output="${tmpdir}/artifact-verify.out"
"${package_script}" --verify "${artifact_zip}" >"${artifact_verify_output}"
grep -Fq "evidence-package: ok" "${artifact_verify_output}"

python3 - "${package_dir}/manifest.json" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    manifest = json.load(handle)

assert manifest["schema_version"] == 1
assert manifest["host_record_count"] == 1
assert manifest["passed_cache_count"] == 1
assert manifest["failed_cache_count"] == 0
assert manifest["skipped_duplicate_cache_count"] == 1
assert manifest["passed_cache_uuids"] == ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]
assert manifest["summary"]["validated_unique_cache_count"] == 1
assert manifest["cache_samples"][0]["sample_count"] == 6
assert manifest["report_sha256"]
assert manifest["smoke_log_sha256"]
assert "shared-cache-evidence-audit.sh" in manifest["audit_command"]
PY

repeat_output="${tmpdir}/repeat.out"
if "${package_script}" --from-report "${report}" --output "${package_dir}" >"${repeat_output}" 2>&1; then
  echo "expected existing output directory to fail" >&2
  exit 1
fi
grep -Fq "output directory already exists" "${repeat_output}"

broken_dir="${tmpdir}/broken-package"
cp -R "${package_dir}" "${broken_dir}"
printf '\n# tampered\n' >>"${broken_dir}/evidence.jsonl"
broken_output="${tmpdir}/broken.out"
if "${package_script}" --verify "${broken_dir}" >"${broken_output}" 2>&1; then
  echo "expected tampered package verification to fail" >&2
  exit 1
fi
grep -Fq "checksum mismatch for evidence.jsonl" "${broken_output}"

combo_output="${tmpdir}/combo.out"
if "${package_script}" --verify "${package_dir}" --from-report "${report}" >"${combo_output}" 2>&1; then
  echo "expected --verify combination to fail" >&2
  exit 1
fi
grep -Fq -- "--verify cannot be combined" "${combo_output}"

inner_archive_output="${tmpdir}/inner-archive.out"
if "${package_script}" --from-report "${report}" --output "${tmpdir}/package-with-inner-archive" --archive "${tmpdir}/package-with-inner-archive/evidence.tar.gz" >"${inner_archive_output}" 2>&1; then
  echo "expected archive inside package to fail" >&2
  exit 1
fi
grep -Fq "archive path must not be inside the evidence package" "${inner_archive_output}"

existing_archive_output="${tmpdir}/existing-archive.out"
if "${package_script}" --from-report "${report}" --output "${tmpdir}/package-with-existing-archive" --archive "${archive_path}" >"${existing_archive_output}" 2>&1; then
  echo "expected existing archive to fail" >&2
  exit 1
fi
grep -Fq "archive already exists" "${existing_archive_output}"

bad_archive_extension_output="${tmpdir}/bad-archive-extension.out"
if "${package_script}" --from-report "${report}" --output "${tmpdir}/package-with-bad-archive" --archive "${tmpdir}/evidence.zip" >"${bad_archive_extension_output}" 2>&1; then
  echo "expected bad archive extension to fail" >&2
  exit 1
fi
grep -Fq -- "--archive path must end in .tar.gz or .tgz" "${bad_archive_extension_output}"

unsafe_archive="${tmpdir}/unsafe.tar.gz"
python3 - "${unsafe_archive}" <<'PY'
import io
import sys
import tarfile

with tarfile.open(sys.argv[1], "w:gz") as archive:
    payload = b"nope"
    entry = tarfile.TarInfo("../evil")
    entry.size = len(payload)
    archive.addfile(entry, io.BytesIO(payload))
PY
unsafe_archive_output="${tmpdir}/unsafe-archive.out"
if "${package_script}" --verify "${unsafe_archive}" >"${unsafe_archive_output}" 2>&1; then
  echo "expected unsafe archive verification to fail" >&2
  exit 1
fi
grep -Fq "unsafe archive entry" "${unsafe_archive_output}"

unsafe_zip="${tmpdir}/unsafe.zip"
python3 - "${unsafe_zip}" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1], "w") as archive:
    archive.writestr("../evil.tar.gz", b"nope")
PY
unsafe_zip_output="${tmpdir}/unsafe-zip.out"
if "${package_script}" --verify "${unsafe_zip}" >"${unsafe_zip_output}" 2>&1; then
  echo "expected unsafe artifact zip verification to fail" >&2
  exit 1
fi
grep -Fq "unsafe artifact zip entry" "${unsafe_zip_output}"

multi_archive_zip="${tmpdir}/multi-archive.zip"
python3 - "${multi_archive_zip}" "${archive_path}" <<'PY'
import pathlib
import sys
import zipfile

archive_path = pathlib.Path(sys.argv[2])
with zipfile.ZipFile(sys.argv[1], "w") as archive:
    archive.write(archive_path, "first.tar.gz")
    archive.write(archive_path, "second.tar.gz")
PY
multi_archive_zip_output="${tmpdir}/multi-archive-zip.out"
if "${package_script}" --verify "${multi_archive_zip}" >"${multi_archive_zip_output}" 2>&1; then
  echo "expected multi-archive artifact zip verification to fail" >&2
  exit 1
fi
grep -Fq "expected exactly one evidence archive in artifact zip" "${multi_archive_zip_output}"

echo "shared-cache-evidence-package tests passed"
