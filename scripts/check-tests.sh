#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-tests.sh --workspace
  scripts/check-tests.sh --args <cargo-test-args...>
USAGE
}

if [[ $# -eq 0 ]]; then
  usage
  exit 2
fi

if [[ "$1" == "--workspace" ]]; then
  shift
  args=(--workspace "$@")
elif [[ "$1" == "--args" ]]; then
  shift
  args=("$@")
else
  echo "check-tests: unknown argument '$1'" >&2
  usage
  exit 2
fi

if cargo nextest --version >/dev/null 2>&1; then
  nextest_build_jobs="${RARAG_NEXTEST_BUILD_JOBS:-2}"
  nextest_profile="${RARAG_NEXTEST_PROFILE:-default}"
  echo "check-tests: running cargo nextest run --profile ${nextest_profile} -j ${nextest_build_jobs} ${args[*]}"
  cargo nextest run --profile "${nextest_profile}" -j "${nextest_build_jobs}" "${args[@]}"
else
  echo "check-tests: running cargo test ${args[*]}"
  cargo test "${args[@]}"
fi
