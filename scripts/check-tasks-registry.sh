#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
CSV="docs/tasks/tasks.csv"
# Task Registry ID: 2026-03-08-ci-ripgrep-fallback

allowed_statuses='planned deferred in_progress done cancelled'

[[ -f "$CSV" ]] || {
  echo "tasks-registry: missing $CSV" >&2
  exit 1
}

header='id,type,title,source,status,blocked_by,notes'
actual_header="$(head -n1 "$CSV")"
if [[ "$actual_header" != "$header" ]]; then
  echo "tasks-registry: invalid header in $CSV" >&2
  echo "tasks-registry: expected: $header" >&2
  exit 1
fi

failures=0
fail() {
  echo "tasks-registry: $1" >&2
  failures=$((failures + 1))
}

contains_id_in_source() {
  local id="$1"
  local source="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -n -F "$id" "$source" >/dev/null 2>&1
  else
    grep -n -F "$id" "$source" >/dev/null 2>&1
  fi
}

while IFS=',' read -r id type title source status _blocked_by _notes; do
  [[ -z "$id" ]] && continue
  if [[ "$id" == "id" ]]; then
    continue
  fi

  [[ -n "$id" ]] || fail "row with empty id"
  [[ -n "$type" ]] || fail "$id missing type"
  [[ -n "$title" ]] || fail "$id missing title"
  [[ -n "$source" ]] || fail "$id missing source"
  [[ -n "$status" ]] || fail "$id missing status"

  if ! grep -Eq "(^| )$status( |$)" <<<"$allowed_statuses"; then
    fail "$id has invalid status '$status'"
  fi

  if [[ ! -f "$source" ]]; then
    fail "$id source file missing: $source"
    continue
  fi

  if ! contains_id_in_source "$id" "$source"; then
    fail "$id not referenced in source: $source"
  fi
done <"$CSV"

if [[ "$failures" -gt 0 ]]; then
  echo "tasks-registry: FAILED ($failures issue(s))" >&2
  exit 1
fi

echo "tasks-registry: OK"
