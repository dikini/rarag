#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
range=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-references.sh --changed
  scripts/check-references.sh --all
  scripts/check-references.sh --range <git-range>
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed)
      mode="changed"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    --range)
      mode="range"
      shift
      [[ $# -gt 0 ]] || {
        echo "refs-check: --range requires a value" >&2
        exit 2
      }
      range="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "refs-check: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

files_in_scope() {
  case "$mode" in
    all)
      git ls-files
      ;;
    range)
      git diff --name-only "$range"
      ;;
    changed)
      {
        git diff --name-only
        git diff --cached --name-only
        git ls-files --others --exclude-standard
      } | sort -u
      ;;
  esac
}

normalize_url() {
  local url="$1"
  while [[ "$url" =~ [\)\]\>\,\.\;\'\"]$ ]]; do
    url="${url%?}"
  done
  printf '%s\n' "$url"
}

url_reachable() {
  local url="$1"
  if curl -fsSLI --max-time 20 "$url" >/dev/null 2>&1; then
    return 0
  fi
  curl -fsSL --max-time 20 --range 0-0 "$url" -o /dev/null >/dev/null 2>&1
}

action_ref_exists() {
  local repo="$1"
  local ref="$2"
  git ls-remote --exit-code "https://github.com/${repo}.git" "$ref" >/dev/null 2>&1
}

mapfile -t files < <(files_in_scope | sed '/^$/d' | sort -u)
if [[ "${#files[@]}" -eq 0 ]]; then
  echo "refs-check: no files in scope"
  exit 0
fi

declare -A seen_urls=()
declare -A seen_actions=()
failures=0

for file in "${files[@]}"; do
  [[ -f "$file" ]] || continue
  if ! grep -Iq . "$file" 2>/dev/null; then
    continue
  fi

  while IFS= read -r raw_url; do
    [[ -n "$raw_url" ]] || continue
    url="$(normalize_url "$raw_url")"
    [[ -n "$url" ]] || continue
    if [[ -n "${seen_urls[$url]:-}" ]]; then
      continue
    fi
    seen_urls[$url]=1
    if ! url_reachable "$url"; then
      echo "refs-check: unreachable URL: $url (from $file)" >&2
      failures=$((failures + 1))
    fi
  done < <(rg -oN 'https?://[^[:space:]<>"'\'')]]+' "$file" || true)

  if [[ "$file" == .github/workflows/*.yml || "$file" == .github/workflows/*.yaml ]]; then
    while IFS= read -r uses_line; do
      uses_value="$(sed -E 's/^[[:space:]]*uses:[[:space:]]*//; s/[[:space:]]+#.*$//' <<<"$uses_line")"
      [[ -n "$uses_value" ]] || continue
      if [[ "$uses_value" == ./* || "$uses_value" == docker://* ]]; then
        continue
      fi
      if [[ "$uses_value" != *@* ]]; then
        echo "refs-check: workflow uses missing @ref: $uses_value (in $file)" >&2
        failures=$((failures + 1))
        continue
      fi
      action_path="${uses_value%@*}"
      action_ref="${uses_value##*@}"
      owner_repo="$(awk -F/ '{print $1 "/" $2}' <<<"$action_path")"
      if [[ "$owner_repo" != */* ]]; then
        echo "refs-check: invalid workflow uses target: $uses_value (in $file)" >&2
        failures=$((failures + 1))
        continue
      fi
      key="${owner_repo}@${action_ref}"
      if [[ -n "${seen_actions[$key]:-}" ]]; then
        continue
      fi
      seen_actions[$key]=1
      if ! action_ref_exists "$owner_repo" "$action_ref"; then
        echo "refs-check: unresolved action ref: $key (in $file)" >&2
        failures=$((failures + 1))
      fi
    done < <(rg -N '^[[:space:]]*uses:[[:space:]]*[^[:space:]]+' "$file" || true)
  fi
done

if [[ "$failures" -gt 0 ]]; then
  echo "refs-check: FAILED ($failures issue(s))" >&2
  exit 1
fi

echo "refs-check: OK"
