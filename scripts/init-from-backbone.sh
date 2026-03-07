#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

dest=""
project_name=""
force=false

usage() {
  cat <<'USAGE'
Usage:
  scripts/init-from-backbone.sh [--dest <path>] [--project <name>] [--force]

Options:
  --dest <path>     Destination directory for the new project repository (optional).
  --project <name>  Project name used during starter file initialization.
                    Required when --dest is omitted; default destination becomes ./<project>.
  --force           Allow initialization into an existing destination by replacing its contents.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dest)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-from-backbone: --dest requires a value" >&2
        exit 2
      }
      dest="$1"
      shift
      ;;
    --project)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-from-backbone: --project requires a value" >&2
        exit 2
      }
      project_name="$1"
      shift
      ;;
    --force)
      force=true
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "init-from-backbone: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$dest" ]]; then
  if [[ -z "$project_name" ]]; then
    echo "init-from-backbone: --project is required when --dest is omitted" >&2
    usage
    exit 2
  fi
  dest="./$project_name"
fi

source_abs="$(realpath "$SOURCE_ROOT")"
dest_abs="$(realpath -m "$dest")"

if [[ "$dest_abs" == "$source_abs" || "$dest_abs" == "$source_abs/"* ]]; then
  echo "init-from-backbone: destination cannot be inside source repository" >&2
  exit 1
fi

if [[ -e "$dest_abs" ]]; then
  if [[ ! -d "$dest_abs" ]]; then
    echo "init-from-backbone: destination exists and is not a directory: $dest_abs" >&2
    exit 1
  fi
  if [[ "$force" != true ]]; then
    echo "init-from-backbone: destination already exists: $dest_abs (use --force to replace contents)" >&2
    exit 1
  fi
  if find "$dest_abs" -mindepth 1 -print -quit | grep -q .; then
    find "$dest_abs" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
  fi
else
  mkdir -p "$dest_abs"
fi

if command -v rsync >/dev/null 2>&1; then
  rsync -a \
    --exclude '.git' \
    --exclude '.tools' \
    --exclude 'target' \
    "$source_abs"/ "$dest_abs"/
else
  tar -C "$source_abs" -cf - --exclude='.git' --exclude='.tools' --exclude='target' . | tar -C "$dest_abs" -xf -
fi

reset_governance_artifacts() {
  local target_root="$1"
  local changelog_template="$target_root/docs/templates/CHANGELOG.template.md"

  if [[ -f "$changelog_template" ]]; then
    cp "$changelog_template" "$target_root/CHANGELOG.md"
  fi

  rm -rf "$target_root/docs/specs" "$target_root/docs/plans"
  mkdir -p "$target_root/docs/specs" "$target_root/docs/plans"

  mkdir -p "$target_root/docs/tasks"
  cat >"$target_root/docs/tasks/tasks.csv" <<'EOF'
id,type,title,source,status,blocked_by,notes
EOF

  cat >"$target_root/docs/tasks/README.md" <<'EOF'
# Task Registry

This directory provides deterministic task state listing for planning and deferred work.

## Format

- Registry file: `docs/tasks/tasks.csv`
- Header columns:
  - `id,type,title,source,status,blocked_by,notes`
- Status enum:
  - `planned`
  - `deferred`
  - `in_progress`
  - `done`
  - `cancelled`

## Commands

- List all: `scripts/tasks.sh`
- Summary: `scripts/tasks.sh --summary`
- Upsert task row: `scripts/tasks.sh --upsert <id> --status <status> [--type ... --title ... --source ... --blocked-by ... --notes ...]`
- Validate registry: `scripts/check-tasks-registry.sh`
- Validate sync gating (changed files): `scripts/check-tasks-sync.sh --changed`
- Bootstrap toolchain/deps: `scripts/bootstrap-dev.sh --apply`
- Canonical task runner entrypoint: `just verify`
EOF
}

reset_governance_artifacts "$dest_abs"

if [[ ! -d "$dest_abs/.git" ]]; then
  git -C "$dest_abs" init -q
fi

if [[ -z "$project_name" ]]; then
  project_name="$(basename "$dest_abs")"
fi

(cd "$dest_abs" && scripts/init-repo.sh --apply --force --project "$project_name")

echo "init-from-backbone: initialized repository at $dest_abs"
