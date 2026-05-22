#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
audit="${root}/bin/shared-cache-evidence-audit.sh"
package_script="${root}/bin/shared-cache-evidence-package.sh"
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

append_cache() {
  local path="$1"
  local uuid="$2"
  local sample_count="$3"
  local status="${4:-passed}"
  printf '{"event":"cache","status":"%s","cache_uuid":"%s","sample_count":%s}\n' \
    "${status}" "${uuid}" "${sample_count}" >>"${path}"
}

write_report() {
  local path="$1"
  local product_version="$2"
  local build_version="$3"
  local host="$4"
  local uuid="$5"
  local sample_count="${6:-6}"
  local status="${7:-passed}"
  printf '{"event":"host","product_version":"%s","build_version":"%s","uname":"Darwin %s 25.0.0 Darwin Kernel Version 25.0.0: root:xnu/RELEASE_ARM64 arm64"}\n' \
    "${product_version}" "${build_version}" "${host}" >"${path}"
  append_cache "${path}" "${uuid}" "${sample_count}" "${status}"
}

write_artifact_zip() {
  local artifact_zip="$1"
  local archive_path="$2"

  python3 - "${artifact_zip}" "${archive_path}" <<'PY'
import pathlib
import sys
import zipfile

artifact_zip = pathlib.Path(sys.argv[1])
archive_path = pathlib.Path(sys.argv[2])
with zipfile.ZipFile(artifact_zip, "w") as archive:
    archive.write(archive_path, archive_path.name)
PY
}

expect_success() {
  local label="$1"
  shift
  local stdout_file="${tmpdir}/${label}.stdout"
  local stderr_file="${tmpdir}/${label}.stderr"
  "${audit}" "$@" >"${stdout_file}" 2>"${stderr_file}"
  grep -Fq "evidence-audit: ok" "${stdout_file}"
  if [[ -s "${stderr_file}" ]]; then
    echo "expected empty stderr for ${label}" >&2
    cat "${stderr_file}" >&2
    exit 1
  fi
}

expect_failure() {
  local label="$1"
  local expected="$2"
  shift 2
  local stdout_file="${tmpdir}/${label}.stdout"
  local stderr_file="${tmpdir}/${label}.stderr"
  set +e
  "${audit}" "$@" >"${stdout_file}" 2>"${stderr_file}"
  local status=$?
  set -e
  if [[ "${status}" -eq 0 ]]; then
    echo "expected failure for ${label}" >&2
    cat "${stdout_file}" >&2
    cat "${stderr_file}" >&2
    exit 1
  fi
  if ! grep -Fq -- "${expected}" "${stdout_file}" "${stderr_file}"; then
    echo "missing expected failure text for ${label}: ${expected}" >&2
    cat "${stdout_file}" >&2
    cat "${stderr_file}" >&2
    exit 1
  fi
}

report_a="${tmpdir}/host-a.jsonl"
report_b="${tmpdir}/host-b.jsonl"
write_report "${report_a}" "26.5" "25F71" "alpha-host" "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" 6
write_report "${report_b}" "25.7" "24G90" "beta-host" "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" 6
package_a="${tmpdir}/host-a-package"
package_b="${tmpdir}/host-b-package"
archive_a="${tmpdir}/host-a-package.tar.gz"
archive_b="${tmpdir}/host-b-package.tar.gz"
artifact_a="${tmpdir}/host-a-artifact.zip"
artifact_b="${tmpdir}/host-b-artifact.zip"
"${package_script}" --from-report "${report_a}" --output "${package_a}" --archive "${archive_a}" >"${tmpdir}/package-a.out"
"${package_script}" --from-report "${report_b}" --output "${package_b}" --archive "${archive_b}" >"${tmpdir}/package-b.out"
write_artifact_zip "${artifact_a}" "${archive_a}"
write_artifact_zip "${artifact_b}" "${archive_b}"

expect_success "two_independent_reports" "${report_a}" "${report_b}"
expect_success "two_independent_packages" "${package_a}" "${package_b}"
expect_success "two_independent_archives" "${archive_a}" "${archive_b}"
expect_success "two_independent_artifact_zips" "${artifact_a}" "${artifact_b}"
expect_success "threshold_override" \
  --min-independent-runs 1 \
  --min-unique-cache-uuids 1 \
  --min-samples 1 \
  "${report_a}"

same_host="${tmpdir}/same-host.jsonl"
write_report "${same_host}" "26.5" "25F71" "alpha-host" "cccccccccccccccccccccccccccccccc" 6
expect_failure "same_host" \
  "independent host/release evidence is insufficient" \
  "${report_a}" "${same_host}"

same_uuid="${tmpdir}/same-uuid.jsonl"
write_report "${same_uuid}" "25.7" "24G90" "beta-host" "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" 6
expect_failure "same_uuid" \
  "unique passed cache UUID evidence is insufficient" \
  "${report_a}" "${same_uuid}"

low_sample="${tmpdir}/low-sample.jsonl"
write_report "${low_sample}" "26.5" "25F71" "alpha-host" "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" 5
expect_failure "low_sample" \
  "passed cache records below sample threshold 6" \
  "${low_sample}" "${report_b}"

failed_cache="${tmpdir}/failed-cache.jsonl"
write_report "${failed_cache}" "26.5" "25F71" "alpha-host" "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" 6
append_cache "${failed_cache}" "dddddddddddddddddddddddddddddddd" 6 "failed"
expect_failure "failed_cache" \
  "failed cache records present" \
  "${failed_cache}" "${report_b}"

no_host="${tmpdir}/no-host.jsonl"
append_cache "${no_host}" "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" 6
expect_failure "no_host" \
  "expected exactly one host record in" \
  "${no_host}" "${report_b}"

bad_sample="${tmpdir}/bad-sample.jsonl"
printf '{"event":"host","product_version":"26.5","build_version":"25F71","uname":"Darwin alpha-host 25.0.0 Darwin Kernel Version 25.0.0: root:xnu/RELEASE_ARM64 arm64"}\n' >"${bad_sample}"
printf '{"event":"cache","status":"passed","cache_uuid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","sample_count":"not-a-number"}\n' >>"${bad_sample}"
expect_failure "bad_sample" \
  "passed cache record has invalid sample_count" \
  "${bad_sample}" "${report_b}"

bad_uuid="${tmpdir}/bad-uuid.jsonl"
printf '{"event":"host","product_version":"26.5","build_version":"25F71","uname":"Darwin alpha-host 25.0.0 Darwin Kernel Version 25.0.0: root:xnu/RELEASE_ARM64 arm64"}\n' >"${bad_uuid}"
printf '{"event":"cache","status":"passed","sample_count":6}\n' >>"${bad_uuid}"
expect_failure "bad_uuid" \
  "passed cache record must include a non-empty cache_uuid" \
  "${bad_uuid}" "${report_b}"

expect_failure "bad_threshold" \
  "--min-samples must be an integer" \
  --min-samples not-a-number "${report_a}"
expect_failure "missing_threshold_value" \
  "--min-samples requires a value" \
  --min-samples

tampered_package="${tmpdir}/tampered-package"
cp -R "${package_a}" "${tampered_package}"
printf '\n# tampered\n' >>"${tampered_package}/evidence.jsonl"
expect_failure "tampered_package" \
  "checksum mismatch for evidence.jsonl" \
  "${tampered_package}" "${package_b}"

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
expect_failure "unsafe_archive" \
  "unsafe archive entry" \
  "${unsafe_archive}" "${package_b}"

unsafe_zip="${tmpdir}/unsafe.zip"
python3 - "${unsafe_zip}" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1], "w") as archive:
    archive.writestr("../evil.tar.gz", b"nope")
PY
expect_failure "unsafe_artifact_zip" \
  "unsafe artifact zip entry" \
  "${unsafe_zip}" "${package_b}"

echo "shared-cache-evidence-audit tests passed"
