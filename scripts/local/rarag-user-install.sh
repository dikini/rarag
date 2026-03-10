#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

install_root="${HOME}/.local"
config_path="${HOME}/.config/rarag/rarag.toml"
config_source="$ROOT/examples/rarag.example.toml"
with_service=true
force_config=false

usage() {
  cat <<'USAGE'
Usage:
  scripts/local/rarag-user-install.sh [options]

Options:
  --install-root <path>   Cargo install root (default: ~/.local)
  --config-path <path>    Target config path (default: ~/.config/rarag/rarag.toml)
  --config-source <path>  Source TOML to copy (default: examples/rarag.example.toml)
  --force-config          Overwrite config file if it already exists
  --no-service            Skip `rarag service install` and restart
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install-root)
      install_root="$2"
      shift 2
      ;;
    --config-path)
      config_path="$2"
      shift 2
      ;;
    --config-source)
      config_source="$2"
      shift 2
      ;;
    --force-config)
      force_config=true
      shift
      ;;
    --no-service)
      with_service=false
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "rarag-user-install: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

command -v cargo >/dev/null 2>&1 || {
  echo "rarag-user-install: cargo is required" >&2
  exit 1
}

[[ -f "$config_source" ]] || {
  echo "rarag-user-install: missing config source: $config_source" >&2
  exit 1
}

echo "rarag-user-install: installing binaries into $install_root/bin"
cargo install \
  --path crates/rarag \
  --path crates/raragd \
  --path crates/rarag-mcp \
  --locked \
  --root "$install_root"

mkdir -p "$(dirname "$config_path")"
if [[ "$force_config" == true || ! -f "$config_path" ]]; then
  cp "$config_source" "$config_path"
  echo "rarag-user-install: wrote config to $config_path"
else
  echo "rarag-user-install: keeping existing config at $config_path"
fi

if [[ "$with_service" == true ]]; then
  command -v rarag >/dev/null 2>&1 || {
    echo "rarag-user-install: 'rarag' not found on PATH after install; add $install_root/bin to PATH and re-run" >&2
    exit 1
  }
  echo "rarag-user-install: installing/restarting user services"
  rarag service install
  rarag service restart --service all
fi

if [[ ":$PATH:" != *":$install_root/bin:"* ]]; then
  echo "rarag-user-install: add to PATH: export PATH=\"$install_root/bin:\$PATH\""
fi

echo "rarag-user-install: OK"
