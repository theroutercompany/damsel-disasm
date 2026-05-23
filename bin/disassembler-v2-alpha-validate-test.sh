#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validator="${root}/bin/disassembler-v2-alpha-validate.sh"
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

expect_failure() {
  local label="$1"
  local expected="$2"
  shift 2
  local output="${tmpdir}/${label}.out"
  set +e
  "$validator" "$@" >"${output}" 2>&1
  local status=$?
  set -e
  if [[ "${status}" -eq 0 ]]; then
    echo "expected failure for ${label}" >&2
    cat "${output}" >&2
    exit 1
  fi
  if ! grep -Fq -- "${expected}" "${output}"; then
    echo "missing expected failure text for ${label}: ${expected}" >&2
    cat "${output}" >&2
    exit 1
  fi
}

help_output="${tmpdir}/help.out"
"${validator}" --help >"${help_output}"
grep -Fq "Usage: bin/disassembler-v2-alpha-validate.sh" "${help_output}"
grep -Fq -- "--real-cache-output DIR" "${help_output}"
grep -Fq -- "--real-cache-archive PATH" "${help_output}"
grep -Fq -- "--beta-evidence PATH" "${help_output}"

fast_output="${tmpdir}/fast.out"
"${validator}" \
  --skip-workspace-tests \
  --skip-fixtures \
  --skip-bench \
  --skip-ui \
  --skip-external \
  >"${fast_output}"
grep -Fq "skip: workspace tests" "${fast_output}"
grep -Fq "skip: fixture checks" "${fast_output}"
grep -Fq "skip: UI JavaScript syntax" "${fast_output}"
grep -Fq "skip: decode bench compile" "${fast_output}"
grep -Fq "skip: LLVM disassembly comparison" "${fast_output}"
grep -Fq "pass --real-cache-output DIR or --real-cache-archive PATH to run" "${fast_output}"
grep -Fq "pass --beta-evidence PATH, --real-cache-output DIR, or --real-cache-archive PATH to run" "${fast_output}"
grep -Fq "disassembler-v2-alpha validation passed" "${fast_output}"

report_a="${tmpdir}/host-a.jsonl"
report_b="${tmpdir}/host-b.jsonl"
package_a="${tmpdir}/host-a-package"
package_b="${tmpdir}/host-b-package"
archive_a="${tmpdir}/host-a-package.tar.gz"
artifact_a="${tmpdir}/host-a-artifact.zip"
cat >"${report_a}" <<'JSONL'
{"build_version":"25F71","event":"host","generated_at_utc":"2026-05-22T00:00:00Z","instruction_limit":8,"product_name":"macOS","product_version":"26.5","sample_limit":6,"uname":"Darwin alpha-host 25.0.0 Darwin Kernel Version 25.0.0: root:xnu/RELEASE_ARM64 arm64"}
{"architecture":"arm64e","cache_uuid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","event":"cache","image_count":100,"member_count":2,"path":"/System/Library/dyld/dyld_shared_cache_arm64e","sample_count":6,"samples":[],"skip_count":0,"skips":[],"status":"passed"}
{"event":"summary","seen_unique_cache_uuid_count":1,"validated_unique_cache_count":1}
JSONL
cat >"${report_b}" <<'JSONL'
{"build_version":"24G90","event":"host","generated_at_utc":"2026-05-22T00:00:00Z","instruction_limit":8,"product_name":"macOS","product_version":"25.7","sample_limit":6,"uname":"Darwin beta-host 24.0.0 Darwin Kernel Version 24.0.0: root:xnu/RELEASE_ARM64 arm64"}
{"architecture":"arm64e","cache_uuid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","event":"cache","image_count":100,"member_count":2,"path":"/System/Library/dyld/dyld_shared_cache_arm64e","sample_count":6,"samples":[],"skip_count":0,"skips":[],"status":"passed"}
{"event":"summary","seen_unique_cache_uuid_count":1,"validated_unique_cache_count":1}
JSONL
"${root}/bin/shared-cache-evidence-package.sh" --from-report "${report_a}" --output "${package_a}" --archive "${archive_a}" >"${tmpdir}/package-a.out"
"${root}/bin/shared-cache-evidence-package.sh" --from-report "${report_b}" --output "${package_b}" >"${tmpdir}/package-b.out"
python3 - "${artifact_a}" "${archive_a}" <<'PY'
import pathlib
import sys
import zipfile

artifact_zip = pathlib.Path(sys.argv[1])
archive_path = pathlib.Path(sys.argv[2])
with zipfile.ZipFile(artifact_zip, "w") as archive:
    archive.write(archive_path, archive_path.name)
PY

beta_output="${tmpdir}/beta.out"
"${validator}" \
  --skip-workspace-tests \
  --skip-fixtures \
  --skip-bench \
  --skip-ui \
  --skip-external \
  --beta-evidence "${artifact_a}" \
  --beta-evidence "${package_b}" \
  >"${beta_output}"
grep -Fq "ok: public-beta evidence audit" "${beta_output}"

expect_failure "external_conflict" \
  "--skip-external and --require-external cannot be combined" \
  --skip-external --require-external
expect_failure "missing_real_cache_output" \
  "--real-cache-output requires a value" \
  --real-cache-output
expect_failure "missing_real_cache_archive" \
  "--real-cache-archive requires a value" \
  --real-cache-archive
expect_failure "bad_real_cache_archive_extension" \
  "--real-cache-archive path must end in .tar.gz or .tgz" \
  --real-cache-archive "${tmpdir}/evidence.zip"
expect_failure "missing_beta_evidence" \
  "--beta-evidence requires a value" \
  --beta-evidence
expect_failure "unknown_argument" \
  "unknown argument: --definitely-not-real" \
  --definitely-not-real

echo "disassembler-v2-alpha-validate tests passed"
