#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/release-package.sh \
  --binary <path> \
  --artifact-name <name.tar.gz> \
  --archive-root <dir-name> \
  --output-dir <dir> \
  --channel <nightly|beta> \
  --release-tag <tag> \
  --display-version <version> \
  --signing-state <unsigned|signed-notarized> \
  --commit-sha <sha>
EOF
}

repo_root() {
  CDPATH= cd -- "$(dirname "$0")/.." && pwd
}

binary=""
artifact_name=""
archive_root=""
output_dir=""
channel=""
release_tag=""
display_version=""
signing_state=""
commit_sha=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary) binary="${2:-}"; shift 2 ;;
    --artifact-name) artifact_name="${2:-}"; shift 2 ;;
    --archive-root) archive_root="${2:-}"; shift 2 ;;
    --output-dir) output_dir="${2:-}"; shift 2 ;;
    --channel) channel="${2:-}"; shift 2 ;;
    --release-tag) release_tag="${2:-}"; shift 2 ;;
    --display-version) display_version="${2:-}"; shift 2 ;;
    --signing-state) signing_state="${2:-}"; shift 2 ;;
    --commit-sha) commit_sha="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

for required in "$binary" "$artifact_name" "$archive_root" "$output_dir" "$channel" "$release_tag" "$display_version" "$signing_state" "$commit_sha"; do
  if [[ -z "$required" ]]; then
    echo "missing required argument" >&2
    usage >&2
    exit 1
  fi
done

if [[ ! -f "$binary" ]]; then
  echo "binary not found: $binary" >&2
  exit 1
fi

root="$(repo_root)"
tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/damsel-release-package.XXXXXX")"
trap 'rm -rf "$tmpdir"' EXIT

stage_root="$tmpdir/$archive_root"
mkdir -p "$stage_root"
install -m 0755 "$binary" "$stage_root/damsel"
cp "$root/README.md" "$stage_root/README.md"
cp "$root/CHANGELOG.md" "$stage_root/CHANGELOG.md"
cp "$root/RELEASE.md" "$stage_root/RELEASE.md"

cat >"$stage_root/RELEASE-METADATA.txt" <<EOF
channel=$channel
release_tag=$release_tag
display_version=$display_version
signing_state=$signing_state
commit_sha=$commit_sha
EOF

mkdir -p "$output_dir"
archive_path="$output_dir/$artifact_name"
checksum_path="$output_dir/${artifact_name}.sha256"
rm -f "$archive_path" "$checksum_path"

COPYFILE_DISABLE=1 tar -czf "$archive_path" -C "$tmpdir" "$archive_root"
checksum="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
printf '%s  %s\n' "$checksum" "$(basename "$archive_path")" >"$checksum_path"

echo "archive_path=$archive_path"
echo "checksum_path=$checksum_path"
