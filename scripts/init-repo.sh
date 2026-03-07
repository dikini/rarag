#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode=""
force=false
config_path="project.init.yaml"
config_explicit=false
default_project_name="$(basename "$ROOT")"
cli_project_name=""
interactive_project_name=""
config_project_name=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/init-repo.sh --check
  scripts/init-repo.sh --apply [--force] [--project <name>] [--config <path>]
  scripts/init-repo.sh --dry-run [--force] [--project <name>] [--config <path>]
  scripts/init-repo.sh --interactive [--config <path>]

Options:
  --check           Validate whether top-level starter files exist.
  --apply           Create starter files from templates when missing.
  --dry-run         Show resolved values and file actions without writing files.
  --interactive     Prompt for missing config values and write config file.
  --force           Overwrite existing starter files (only with --apply).
  --project <name>  Project name used for README template replacement.
  --config <path>   Config file path (default: project.init.yaml).
USAGE
}

set_mode() {
  local requested_mode="$1"
  if [[ -n "$mode" && "$mode" != "$requested_mode" ]]; then
    echo "init-repo: exactly one mode is required (--check, --apply, --dry-run, or --interactive)" >&2
    exit 2
  fi
  mode="$requested_mode"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      set_mode "check"
      shift
      ;;
    --apply)
      set_mode "apply"
      shift
      ;;
    --dry-run)
      set_mode "dry-run"
      shift
      ;;
    --interactive)
      set_mode "interactive"
      shift
      ;;
    --force)
      force=true
      shift
      ;;
    --project)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-repo: --project requires a value" >&2
        exit 2
      }
      cli_project_name="$1"
      shift
      ;;
    --config)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-repo: --config requires a value" >&2
        exit 2
      }
      config_path="$1"
      config_explicit=true
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "init-repo: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$mode" ]]; then
  echo "init-repo: explicit mode required (--check, --apply, --dry-run, or --interactive)" >&2
  usage
  exit 2
fi

if [[ "$mode" != "apply" && "$mode" != "dry-run" && "$force" == true ]]; then
  echo "init-repo: --force is only valid with --apply or --dry-run" >&2
  exit 2
fi

if [[ "$mode" == "interactive" && -n "$cli_project_name" ]]; then
  echo "init-repo: --project cannot be used with --interactive" >&2
  exit 2
fi

readme_template="docs/templates/README.template.md"
agents_template="docs/templates/AGENTS.template.md"
readme_target="README.md"
agents_target="AGENTS.md"

[[ -f "$readme_template" ]] || {
  echo "init-repo: missing template $readme_template" >&2
  exit 1
}
[[ -f "$agents_template" ]] || {
  echo "init-repo: missing template $agents_template" >&2
  exit 1
}

read_config_project_name() {
  local cfg_path="$1"
  awk '
    /^[[:space:]]*project:[[:space:]]*$/ { in_project=1; next }
    in_project && /^[^[:space:]]/ { in_project=0 }
    in_project && /^[[:space:]]*name:[[:space:]]*/ {
      sub(/^[[:space:]]*name:[[:space:]]*/, "", $0)
      gsub(/^["'"'"']|["'"'"']$/, "", $0)
      print
      exit
    }
  ' "$cfg_path"
}

load_config() {
  if [[ -f "$config_path" ]]; then
    config_project_name="$(read_config_project_name "$config_path")"
    return 0
  fi
  if [[ "$mode" == "interactive" ]]; then
    return 0
  fi
  if [[ "$config_explicit" == true ]]; then
    echo "init-repo: config file not found: $config_path" >&2
    exit 1
  fi
}

resolve_project_name() {
  local resolved="$default_project_name"
  if [[ -n "$config_project_name" ]]; then
    resolved="$config_project_name"
  fi
  if [[ -n "$interactive_project_name" ]]; then
    resolved="$interactive_project_name"
  fi
  if [[ -n "$cli_project_name" ]]; then
    resolved="$cli_project_name"
  fi
  printf '%s' "$resolved"
}

write_config() {
  local target="$1"
  local resolved_project_name="$2"
  cat >"$target" <<EOF
version: 1
project:
  name: $resolved_project_name
EOF
}

has_required_placeholders() {
  local path="$1"
  rg -n '<project-name>|<workspace>' "$path" >/dev/null 2>&1
}

write_readme() {
  local resolved_project_name="$1"
  sed \
    -e "s|<project-name>|$resolved_project_name|g" \
    -e "s|<workspace>|$resolved_project_name|g" \
    "$readme_template" >"$readme_target"
}

write_agents() {
  cp "$agents_template" "$agents_target"
}

check_file() {
  local path="$1"
  if [[ -f "$path" ]]; then
    echo "init-repo: present $path"
    return 0
  fi
  echo "init-repo: missing $path" >&2
  return 1
}

load_config
resolved_project_name="$(resolve_project_name)"

if [[ "$mode" == "check" ]]; then
  failures=0
  check_file "$readme_target" || failures=$((failures + 1))
  check_file "$agents_target" || failures=$((failures + 1))
  if [[ "$failures" -gt 0 ]]; then
    echo "init-repo: FAILED (missing starter files)" >&2
    exit 1
  fi
  echo "init-repo: OK"
  exit 0
fi

if [[ "$mode" == "interactive" ]]; then
  prompt_default="$resolved_project_name"
  printf 'Project name [%s]: ' "$prompt_default" >&2
  IFS= read -r answer || true
  if [[ -n "$answer" ]]; then
    interactive_project_name="$answer"
  else
    interactive_project_name="$prompt_default"
  fi
  resolved_project_name="$(resolve_project_name)"
  write_config "$config_path" "$resolved_project_name"
  echo "init-repo: wrote $config_path"
  echo "init-repo: OK"
  exit 0
fi

if [[ "$mode" == "dry-run" ]]; then
  echo "init-repo: dry-run"
  echo "init-repo: resolved project name: $resolved_project_name"
  if [[ -f "$readme_target" && "$force" != true ]]; then
    echo "init-repo: would skip $readme_target (use --force to overwrite)"
  else
    echo "init-repo: would write $readme_target"
  fi
  if [[ -f "$agents_target" && "$force" != true ]]; then
    echo "init-repo: would skip $agents_target (use --force to overwrite)"
  else
    echo "init-repo: would write $agents_target"
  fi
  echo "init-repo: OK"
  exit 0
fi

if [[ -f "$readme_target" && "$force" != true ]]; then
  echo "init-repo: skip existing $readme_target (use --force to overwrite)"
else
  write_readme "$resolved_project_name"
  echo "init-repo: wrote $readme_target"
fi

if [[ -f "$agents_target" && "$force" != true ]]; then
  echo "init-repo: skip existing $agents_target (use --force to overwrite)"
else
  write_agents
  echo "init-repo: wrote $agents_target"
fi

if [[ -f "$readme_target" ]] && has_required_placeholders "$readme_target"; then
  echo "init-repo: unresolved required placeholders in $readme_target" >&2
  exit 1
fi
if [[ -f "$agents_target" ]] && has_required_placeholders "$agents_target"; then
  echo "init-repo: unresolved required placeholders in $agents_target" >&2
  exit 1
fi

echo "init-repo: OK"
