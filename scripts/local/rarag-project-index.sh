#!/usr/bin/env bash
set -euo pipefail

subcommand="${1:-}"
if [[ -n "$subcommand" ]]; then
  shift
fi

worktree=""
socket_path=""
json_output=false
debounce_seconds=90
exclude_regex='(^|/)(\.git|target|\.direnv|\.idea|\.vscode|node_modules)(/|$)'

usage() {
  cat <<'USAGE'
Usage:
  scripts/local/rarag-project-index.sh <add|reindex|watch> [options]

Subcommands:
  add       Add project index entry (same operation as reindex)
  reindex   Rebuild index for the selected worktree
  watch     Reindex after filesystem quiet period (debounced inotify)

Options:
  --worktree <path>          Worktree/repo root (default: git top-level or current dir)
  --socket <path>            Explicit daemon socket path
  --json                     Emit JSON response from rarag index build
  --debounce-seconds <n>     Quiet period before reindex in watch mode (default: 90)
  --exclude-regex <regex>    inotify exclude regex in watch mode
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --worktree)
      worktree="$2"
      shift 2
      ;;
    --socket)
      socket_path="$2"
      shift 2
      ;;
    --json)
      json_output=true
      shift
      ;;
    --debounce-seconds)
      debounce_seconds="$2"
      shift 2
      ;;
    --exclude-regex)
      exclude_regex="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "rarag-project-index: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

case "$subcommand" in
  add | reindex | watch) ;;
  *)
    echo "rarag-project-index: missing or unsupported subcommand '$subcommand'" >&2
    usage
    exit 2
    ;;
esac

command -v rarag >/dev/null 2>&1 || {
  echo "rarag-project-index: rarag CLI is required on PATH" >&2
  exit 1
}

if [[ -z "$worktree" ]]; then
  if git rev-parse --show-toplevel >/dev/null 2>&1; then
    worktree="$(git rev-parse --show-toplevel)"
  else
    worktree="$PWD"
  fi
fi

worktree="$(cd "$worktree" && pwd)"

git_sha="$(git -C "$worktree" rev-parse HEAD 2>/dev/null || true)"
if [[ -z "$git_sha" ]]; then
  echo "rarag-project-index: failed to resolve git HEAD for worktree: $worktree" >&2
  exit 1
fi

run_index() {
  local cmd=(
    rarag index build
    --workspace-root "$worktree"
    --repo-root "$worktree"
    --worktree "$worktree"
    --git-sha "$git_sha"
  )
  if [[ -n "$socket_path" ]]; then
    cmd+=(--socket "$socket_path")
  fi
  if [[ "$json_output" == true ]]; then
    cmd+=(--json)
  fi
  "${cmd[@]}"
}

if [[ "$subcommand" == "add" || "$subcommand" == "reindex" ]]; then
  run_index
  exit 0
fi

command -v inotifywait >/dev/null 2>&1 || {
  echo "rarag-project-index: watch mode requires inotifywait (inotify-tools package)" >&2
  exit 1
}

echo "rarag-project-index: initial index for $worktree"
run_index

echo "rarag-project-index: watching $worktree (debounce=${debounce_seconds}s)"
dirty=0
while true; do
  status=0
  inotifywait \
    -q \
    -r \
    -e close_write \
    -e create \
    -e delete \
    -e moved_to \
    -e moved_from \
    --exclude "$exclude_regex" \
    -t "$debounce_seconds" \
    "$worktree" >/dev/null 2>&1 || status=$?

  if [[ $status -eq 0 ]]; then
    dirty=1
    continue
  fi

  if [[ $status -eq 2 ]]; then
    if [[ $dirty -eq 1 ]]; then
      git_sha="$(git -C "$worktree" rev-parse HEAD)"
      echo "rarag-project-index: filesystem quiet, reindexing git_sha=$git_sha"
      run_index
      dirty=0
    fi
    continue
  fi

  echo "rarag-project-index: inotifywait failed with status=$status" >&2
  exit 1
done
