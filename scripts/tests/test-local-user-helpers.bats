#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_DIR="$(mktemp -d)"
  BIN_DIR="$TMP_DIR/bin"
  mkdir -p "$BIN_DIR"
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "rarag-user-install installs to chosen root and writes config" {
  cat >"$BIN_DIR/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >> "${TEST_LOG_ROOT}/calls.log"
exit 0
EOF
  chmod +x "$BIN_DIR/cargo"

  run env \
    PATH="$BIN_DIR:$PATH" \
    TEST_LOG_ROOT="$TMP_DIR" \
    "$ROOT/scripts/local/rarag-user-install.sh" \
    --install-root "$TMP_DIR/install" \
    --config-path "$TMP_DIR/config/rarag.toml" \
    --config-source "$ROOT/examples/rarag.example.toml" \
    --no-service
  [ "$status" -eq 0 ]

  [ -f "$TMP_DIR/config/rarag.toml" ]
  run rg -- '--root '"$TMP_DIR/install" "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
}

@test "rarag-project-index reindex passes inferred roots and git sha" {
  mkdir -p "$TMP_DIR/worktree"
  cat >"$BIN_DIR/rarag" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >> "${TEST_LOG_ROOT}/calls.log"
exit 0
EOF
  cat >"$BIN_DIR/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "-C" && "$3" == "rev-parse" && "$4" == "HEAD" ]]; then
  echo "abc123"
  exit 0
fi
echo "unexpected git invocation: $*" >&2
exit 1
EOF
  chmod +x "$BIN_DIR/rarag" "$BIN_DIR/git"

  run env \
    PATH="$BIN_DIR:$PATH" \
    TEST_LOG_ROOT="$TMP_DIR" \
    "$ROOT/scripts/local/rarag-project-index.sh" reindex --worktree "$TMP_DIR/worktree" --json
  [ "$status" -eq 0 ]

  run rg 'index build' "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
  run rg -- '--workspace-root '"$TMP_DIR/worktree" "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
  run rg -- '--repo-root '"$TMP_DIR/worktree" "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
  run rg -- '--worktree '"$TMP_DIR/worktree" "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
  run rg -- '--git-sha abc123' "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
}

@test "rarag-project-index watch debounces before reindex" {
  mkdir -p "$TMP_DIR/worktree"
  cat >"$BIN_DIR/rarag" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >> "${TEST_LOG_ROOT}/calls.log"
exit 0
EOF
  cat >"$BIN_DIR/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "-C" && "$3" == "rev-parse" && "$4" == "HEAD" ]]; then
  echo "def456"
  exit 0
fi
echo "unexpected git invocation: $*" >&2
exit 1
EOF
  cat >"$BIN_DIR/inotifywait" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
counter_file="${TEST_LOG_ROOT}/inotify-count"
count=0
if [[ -f "$counter_file" ]]; then
  count="$(cat "$counter_file")"
fi
count=$((count + 1))
echo "$count" > "$counter_file"
if [[ "$count" -eq 1 ]]; then
  exit 0
fi
if [[ "$count" -eq 2 ]]; then
  exit 2
fi
exit 1
EOF
  chmod +x "$BIN_DIR/rarag" "$BIN_DIR/git" "$BIN_DIR/inotifywait"

  run env \
    PATH="$BIN_DIR:$PATH" \
    TEST_LOG_ROOT="$TMP_DIR" \
    "$ROOT/scripts/local/rarag-project-index.sh" watch --worktree "$TMP_DIR/worktree" --debounce-seconds 1
  [ "$status" -eq 1 ]

  # One initial index plus one debounced reindex.
  run bash -lc "rg 'index build' '$TMP_DIR/calls.log' | wc -l"
  [ "$status" -eq 0 ]
  [ "${output//[[:space:]]/}" -eq 2 ]
}
