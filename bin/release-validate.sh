#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/release-validate.sh --channel <nightly|beta> [--tag <tag>]

Validates private release inputs and emits key=value metadata lines to stdout for
GitHub Actions consumption.
EOF
}

repo_root() {
  CDPATH= cd -- "$(dirname "$0")/.." && pwd
}

workspace_version() {
  awk '
    /^\[workspace\.package\]/ { in_block = 1; next }
    /^\[/ { in_block = 0 }
    in_block && $1 == "version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "$(repo_root)/Cargo.toml"
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "required file missing: $path" >&2
    exit 1
  fi
}

channel=""
tag=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --channel)
      channel="${2:-}"
      shift 2
      ;;
    --tag)
      tag="${2:-}"
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

if [[ -z "$channel" ]]; then
  echo "--channel is required" >&2
  usage >&2
  exit 1
fi

case "$channel" in
  nightly|beta)
    ;;
  *)
    echo "unsupported channel: $channel" >&2
    exit 1
    ;;
esac

root="$(repo_root)"
version="$(workspace_version)"

require_file "$root/README.md"
require_file "$root/CHANGELOG.md"
require_file "$root/RELEASE.md"
require_file "$root/damsel-cli/Cargo.toml"
require_file "$root/bin/release-package.sh"
require_file "$root/bin/render-release-notes.sh"
require_file "$root/.github/release-notes/nightly.md.tmpl"
require_file "$root/.github/release-notes/beta.md.tmpl"

artifact_platform="macos-arm64"
binary_source="target/release/damsel-cli"
binary_name="damsel"

if [[ "$channel" == "beta" ]]; then
  if [[ -z "$tag" ]]; then
    echo "--tag is required for beta releases" >&2
    exit 1
  fi
  expected_regex="^v${version//./\\.}-beta\\.[0-9]+$"
  if [[ ! "$tag" =~ $expected_regex ]]; then
    echo "beta tag must match v${version}-beta.N; got: $tag" >&2
    exit 1
  fi
  display_version="${tag#v}"
  release_tag="$tag"
  release_name="damsel ${tag}"
  artifact_name="damsel-${tag}-${artifact_platform}.tar.gz"
  template_path=".github/release-notes/beta.md.tmpl"
  prerelease="true"
  mutable_release="false"
else
  if [[ -n "$tag" ]]; then
    echo "nightly releases must not supply --tag" >&2
    exit 1
  fi
  display_version="nightly"
  release_tag="nightly"
  release_name="damsel nightly"
  artifact_name="damsel-nightly-${artifact_platform}.tar.gz"
  template_path=".github/release-notes/nightly.md.tmpl"
  prerelease="true"
  mutable_release="true"
fi

checksum_name="${artifact_name}.sha256"
archive_root="${artifact_name%.tar.gz}"

cat <<EOF
channel=$channel
workspace_version=$version
display_version=$display_version
release_tag=$release_tag
release_name=$release_name
artifact_name=$artifact_name
checksum_name=$checksum_name
archive_root=$archive_root
template_path=$template_path
binary_source=$binary_source
binary_name=$binary_name
artifact_platform=$artifact_platform
prerelease=$prerelease
mutable_release=$mutable_release
EOF
