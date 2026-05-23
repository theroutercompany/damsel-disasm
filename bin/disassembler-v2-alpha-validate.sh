#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: bin/disassembler-v2-alpha-validate.sh [options]

Runs the fixture-backed disassembler v2 alpha validation sweep. Real dyld
shared-cache validation remains opt-in because cache paths and runtime cost vary
by host.

Options:
  --skip-workspace-tests  Skip cargo test --workspace
  --skip-fixtures         Skip fixture hash and parity checks
  --skip-bench            Skip decode bench compile
  --skip-ui               Skip UI JavaScript syntax check
  --skip-external         Skip LLVM tool disassembly comparison
  --require-external      Fail if llvm-objdump or llvm-otool is unavailable
  --real-cache-output DIR  Run real-cache evidence packaging into DIR and verify it
  --real-cache-archive PATH
                           Also write a .tar.gz/.tgz archive for --real-cache-output
  --beta-evidence PATH     Add a real-cache evidence report, package, archive, or artifact zip to the public-beta audit
  -h, --help              Show this help
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

  return 1
}

run_step() {
  local label="$1"
  shift

  printf '\n==> %s\n' "$label"
  "$@"
  printf 'ok: %s\n' "$label"
}

skip_workspace_tests=false
skip_fixtures=false
skip_bench=false
skip_ui=false
skip_external=false
require_external=false
real_cache_output=""
real_cache_archive=""
beta_evidence=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-workspace-tests)
      skip_workspace_tests=true
      shift
      ;;
    --skip-fixtures)
      skip_fixtures=true
      shift
      ;;
    --skip-bench)
      skip_bench=true
      shift
      ;;
    --skip-ui)
      skip_ui=true
      shift
      ;;
    --skip-external)
      skip_external=true
      shift
      ;;
    --require-external)
      require_external=true
      shift
      ;;
    --real-cache-output)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      real_cache_output="$2"
      shift 2
      ;;
    --real-cache-archive)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      real_cache_archive="$2"
      case "$real_cache_archive" in
        *.tar.gz|*.tgz) ;;
        *)
          echo "--real-cache-archive path must end in .tar.gz or .tgz: $real_cache_archive" >&2
          exit 1
          ;;
      esac
      shift 2
      ;;
    --beta-evidence)
      if [[ $# -lt 2 || "${2:-}" == -* ]]; then
        echo "$1 requires a value" >&2
        usage >&2
        exit 1
      fi
      beta_evidence+=("$2")
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

if [[ "$skip_external" == true && "$require_external" == true ]]; then
  echo "--skip-external and --require-external cannot be combined" >&2
  exit 1
fi

root="$(repo_root)"

if [[ -z "$real_cache_output" && -n "$real_cache_archive" ]]; then
  case "$real_cache_archive" in
    *.tar.gz)
      real_cache_output="${real_cache_archive%.tar.gz}"
      ;;
    *.tgz)
      real_cache_output="${real_cache_archive%.tgz}"
      ;;
  esac
fi

script_syntax=(
  "$root/bin/disasm-external-compare.sh"
  "$root/bin/disassembler-v2-alpha-validate.sh"
  "$root/bin/release-package.sh"
  "$root/bin/release-validate.sh"
  "$root/bin/render-release-notes.sh"
  "$root/bin/shared-cache-evidence-audit.sh"
  "$root/bin/shared-cache-evidence-audit-test.sh"
  "$root/bin/shared-cache-evidence-package.sh"
  "$root/bin/shared-cache-evidence-package-test.sh"
  "$root/bin/shared-cache-real-smoke.sh"
)

printf 'disassembler-v2-alpha validation root: %s\n' "$root"

for script in "${script_syntax[@]}"; do
  run_step "bash syntax $(basename "$script")" bash -n "$script"
done

run_step "shared-cache evidence package self-test" "$root/bin/shared-cache-evidence-package-test.sh"
run_step "shared-cache evidence audit self-test" "$root/bin/shared-cache-evidence-audit-test.sh"
run_step "nightly release metadata validation" "$root/bin/release-validate.sh" --channel nightly
workspace_version="$(
  awk '/^\[workspace\.package\]/{flag=1;next}/^\[/{flag=0} flag && $1 == "version" {gsub(/"/, "", $3); print $3; exit}' "$root/Cargo.toml"
)"
run_step "beta release metadata validation" "$root/bin/release-validate.sh" --channel beta --tag "v${workspace_version}-beta.1"

if [[ "$skip_workspace_tests" != true ]]; then
  run_step "workspace tests" cargo test --workspace --manifest-path "$root/Cargo.toml"
else
  printf '\nskip: workspace tests\n'
fi

if [[ "$skip_fixtures" != true ]]; then
  run_step "fixture hash check" "$root/fixtures/build-fixtures.sh" --check
  run_step "fixture parity probes" sh "$root/fixtures/tests/build-fixtures-parity.sh"
else
  printf '\nskip: fixture checks\n'
fi

if [[ "$skip_ui" != true ]]; then
  run_step "UI JavaScript syntax" node --check "$root/damsel-cli/ui/app.js"
else
  printf '\nskip: UI JavaScript syntax\n'
fi

if [[ "$skip_bench" != true ]]; then
  run_step "decode bench compile" cargo bench -p damsel-macho --manifest-path "$root/Cargo.toml" --bench decode_bench --no-run
else
  printf '\nskip: decode bench compile\n'
fi

if [[ "$skip_external" != true ]]; then
  if find_tool llvm-objdump >/dev/null && find_tool llvm-otool >/dev/null; then
    run_step "LLVM disassembly comparison" "$root/bin/disasm-external-compare.sh"
  elif [[ "$require_external" == true ]]; then
    echo "llvm-objdump and llvm-otool are required but unavailable" >&2
    exit 1
  else
    printf '\nskip: LLVM disassembly comparison (llvm-objdump or llvm-otool unavailable)\n'
  fi
else
  printf '\nskip: LLVM disassembly comparison\n'
fi

if [[ -n "$real_cache_output" ]]; then
  package_args=(--output "$real_cache_output")
  if [[ -n "$real_cache_archive" ]]; then
    package_args+=(--archive "$real_cache_archive")
  fi
  run_step "real shared-cache evidence package" "$root/bin/shared-cache-evidence-package.sh" "${package_args[@]}"
  run_step "real shared-cache evidence package verify" "$root/bin/shared-cache-evidence-package.sh" --verify "$real_cache_output"
  if [[ -n "$real_cache_archive" ]]; then
    run_step "real shared-cache evidence archive verify" "$root/bin/shared-cache-evidence-package.sh" --verify "$real_cache_archive"
    beta_evidence+=("$real_cache_archive")
  else
    beta_evidence+=("$real_cache_output")
  fi
else
  printf '\nskip: real shared-cache evidence packaging (pass --real-cache-output DIR or --real-cache-archive PATH to run)\n'
fi

if [[ ${#beta_evidence[@]} -gt 0 ]]; then
  run_step "public-beta evidence audit" "$root/bin/shared-cache-evidence-audit.sh" "${beta_evidence[@]}"
else
  printf '\nskip: public-beta evidence audit (pass --beta-evidence PATH, --real-cache-output DIR, or --real-cache-archive PATH to run)\n'
fi

run_step "diff whitespace check" git -C "$root" diff --check

printf '\ndisassembler-v2-alpha validation passed\n'
