#!/usr/bin/env bash
set -euo pipefail

root_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
workspace_root="$root_dir/.worktrees/repository-rag-phase1"
if [[ ! -d "$workspace_root" ]]; then
  workspace_root="$root_dir"
fi

required_vars=(OPENAI_API_KEY RARAG_LIVE_QDRANT_ENDPOINT)
for var_name in "${required_vars[@]}"; do
  if [[ -z "${!var_name:-}" ]]; then
    echo "missing required environment variable: $var_name" >&2
    exit 2
  fi
done

openai_base_url=${OPENAI_BASE_URL:-https://api.openai.com/v1}
openai_model=${RARAG_LIVE_OPENAI_MODEL:-text-embedding-3-small}
qdrant_collection=${RARAG_LIVE_QDRANT_COLLECTION:-rarag_live_chunks}
fixture_root="$workspace_root/tests/fixtures/mini_repo"
worktree_root=${RARAG_LIVE_WORKTREE_ROOT:-/repo/.worktrees/live-premerge}
git_sha=${RARAG_LIVE_GIT_SHA:-live-premerge}

runtime_dir=$(mktemp -d)
state_dir=$(mktemp -d)
cache_dir=$(mktemp -d)
config_path="$runtime_dir/rarag.toml"
daemon_socket="$runtime_dir/raragd.sock"
mcp_socket="$runtime_dir/rarag-mcp.sock"

cleanup() {
  status=$?
  if [[ -n "${mcp_pid:-}" ]] && kill -0 "$mcp_pid" 2>/dev/null; then
    kill "$mcp_pid" 2>/dev/null || true
    wait "$mcp_pid" 2>/dev/null || true
  fi
  if [[ -n "${daemon_pid:-}" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    python - "$daemon_socket" <<'PY' >/dev/null 2>&1 || true
import json, socket, sys
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(sys.argv[1])
sock.sendall(json.dumps({"kind": "shutdown"}).encode())
sock.shutdown(socket.SHUT_WR)
while sock.recv(65536):
    pass
sock.close()
PY
    wait "$daemon_pid" 2>/dev/null || true
  fi
  rm -rf "$runtime_dir" "$state_dir" "$cache_dir"
  exit "$status"
}
trap cleanup EXIT

cat >"$config_path" <<TOML
[runtime]
socket_path = "$daemon_socket"
state_root = "$state_dir"
cache_root = "$cache_dir"

[qdrant]
endpoint = "$RARAG_LIVE_QDRANT_ENDPOINT"
collection = "$qdrant_collection"

[embeddings]
base_url = "$openai_base_url"
endpoint_path = "/embeddings"
model = "$openai_model"
api_key_env = "OPENAI_API_KEY"
dimensions = 1536

[daemon]
socket_path = "$daemon_socket"

[mcp]
socket_path = "$mcp_socket"
TOML

(
  cd "$workspace_root"
  cargo build -q -p raragd -p rarag -p rarag-mcp
)

XDG_RUNTIME_DIR="$runtime_dir" XDG_STATE_HOME="$state_dir" XDG_CACHE_HOME="$cache_dir" \
  "$workspace_root/target/debug/raragd" serve --config "$config_path" --socket "$daemon_socket" \
  >"$runtime_dir/daemon.out" 2>"$runtime_dir/daemon.err" &
daemon_pid=$!
for _ in $(seq 1 200); do
  [[ -S "$daemon_socket" ]] && break
  sleep 0.05
done
[[ -S "$daemon_socket" ]]

XDG_RUNTIME_DIR="$runtime_dir" XDG_STATE_HOME="$state_dir" XDG_CACHE_HOME="$cache_dir" \
  "$workspace_root/target/debug/rarag-mcp" serve --config "$config_path" --socket "$mcp_socket" --daemon-socket "$daemon_socket" \
  >"$runtime_dir/mcp.out" 2>"$runtime_dir/mcp.err" &
mcp_pid=$!
for _ in $(seq 1 200); do
  [[ -S "$mcp_socket" ]] && break
  sleep 0.05
done
[[ -S "$mcp_socket" ]]

(
  cd "$workspace_root"
  "$workspace_root/target/debug/rarag" index \
    --config "$config_path" \
    --socket "$daemon_socket" \
    --workspace-root "$fixture_root" \
    --repo-root /repo \
    --worktree-root "$worktree_root" \
    --git-sha "$git_sha" \
    --json >"$runtime_dir/index.json"

  "$workspace_root/target/debug/rarag" query \
    --config "$config_path" \
    --socket "$daemon_socket" \
    --worktree-root "$worktree_root" \
    --mode understand-symbol \
    --text example_sum \
    --symbol-path mini_repo::example_sum \
    --json >"$runtime_dir/cli-query.json"

  "$workspace_root/target/debug/rarag" blast-radius \
    --config "$config_path" \
    --socket "$daemon_socket" \
    --worktree-root "$worktree_root" \
    --text Data \
    --symbol-path mini_repo::Data \
    --changed-path src/lib.rs \
    --json >"$runtime_dir/blast-radius.json"
)

python - "$mcp_socket" "$worktree_root" >"$runtime_dir/mcp-query.json" <<'PY'
import json, socket, sys
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(sys.argv[1])
sock.sendall(json.dumps({
    "kind": "call_tool",
    "name": "query_context",
    "arguments": {
        "worktree_root": sys.argv[2],
        "mode": "implement-adjacent",
        "text": "Data incremented helper",
        "symbol_path": "mini_repo::impl::Data",
        "limit": "4"
    }
}).encode())
sock.shutdown(socket.SHUT_WR)
chunks = []
while True:
    chunk = sock.recv(65536)
    if not chunk:
        break
    chunks.append(chunk)
sock.close()
print(json.dumps(json.loads(b"".join(chunks)), indent=2))
PY

python - "$runtime_dir" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
index = json.loads((root / 'index.json').read_text())
cli = json.loads((root / 'cli-query.json').read_text())
blast = json.loads((root / 'blast-radius.json').read_text())
mcp = json.loads((root / 'mcp-query.json').read_text())
assert index['chunk_count'] > 0, index
assert cli['items'], cli
assert blast['items'], blast
assert mcp['kind'] == 'call_result', mcp
assert mcp['result']['items'], mcp
print(json.dumps({
    'chunk_count': index['chunk_count'],
    'cli_top_symbol': cli['items'][0]['chunk']['symbol_path'],
    'blast_top_symbol': blast['items'][0]['chunk']['symbol_path'],
    'mcp_top_symbol': mcp['result']['items'][0]['chunk']['symbol_path'],
}, indent=2))
PY
