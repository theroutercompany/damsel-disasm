#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/render-release-notes.sh \
  --template <path> \
  --output <path> \
  --channel <nightly|beta> \
  --display-version <version> \
  --release-tag <tag> \
  --artifact-name <name.tar.gz> \
  --commit-sha <sha> \
  --run-date-utc <timestamp> \
  --signing-state <unsigned|signed-notarized>
EOF
}

template=""
output=""
channel=""
display_version=""
release_tag=""
artifact_name=""
archive_root=""
commit_sha=""
run_date_utc=""
signing_state=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --template) template="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    --channel) channel="${2:-}"; shift 2 ;;
    --display-version) display_version="${2:-}"; shift 2 ;;
    --release-tag) release_tag="${2:-}"; shift 2 ;;
    --artifact-name) artifact_name="${2:-}"; shift 2 ;;
    --archive-root) archive_root="${2:-}"; shift 2 ;;
    --commit-sha) commit_sha="${2:-}"; shift 2 ;;
    --run-date-utc) run_date_utc="${2:-}"; shift 2 ;;
    --signing-state) signing_state="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

for required in "$template" "$output" "$channel" "$display_version" "$release_tag" "$artifact_name" "$archive_root" "$commit_sha" "$run_date_utc" "$signing_state"; do
  if [[ -z "$required" ]]; then
    echo "missing required argument" >&2
    usage >&2
    exit 1
  fi
done

if [[ ! -f "$template" ]]; then
  echo "template not found: $template" >&2
  exit 1
fi

mkdir -p "$(dirname "$output")"
signing_label="$signing_state"
if [[ "$signing_state" == "unsigned" ]]; then
  signing_label="unsigned (Apple secrets unavailable or disabled)"
elif [[ "$signing_state" == "signed-notarized" ]]; then
  signing_label="signed and notarization-submitted"
fi

sed \
  -e "s|__CHANNEL__|$channel|g" \
  -e "s|__DISPLAY_VERSION__|$display_version|g" \
  -e "s|__RELEASE_TAG__|$release_tag|g" \
  -e "s|__ARTIFACT_NAME__|$artifact_name|g" \
  -e "s|__ARCHIVE_ROOT__|$archive_root|g" \
  -e "s|__COMMIT_SHA__|$commit_sha|g" \
  -e "s|__RUN_DATE_UTC__|$run_date_utc|g" \
  -e "s|__SIGNING_LABEL__|$signing_label|g" \
  "$template" >"$output"
