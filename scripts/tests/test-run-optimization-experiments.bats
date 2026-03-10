#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_DIR="$(mktemp -d)"
  BIN_DIR="$TMP_DIR/bin"
  ARTIFACT_ROOT="$TMP_DIR/artifacts"
  FIXTURES_PATH="$TMP_DIR/tasks.json"
  mkdir -p "$BIN_DIR" "$ARTIFACT_ROOT"

  cat > "$FIXTURES_PATH" <<'EOF'
[
  {
    "task_id": "reload-archaeology",
    "revision": "abc123",
    "query_mode": "blast-radius",
    "query_text": "archaeology: changed doc_example_sum behavior in daemon server",
    "symbol_path": "compat_repo::doc_example_sum",
    "ideal": ["history:h1", "docs/specs/current-behavior.md"],
    "acceptable": ["docs/ops/reload.md", "compat_repo::doc_example_sum"],
    "distractors": ["docs/plans/future-work.md"]
  }
]
EOF

  cat > "$BIN_DIR/raragd" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
log_root="${TEST_LOG_ROOT:?}"
printf 'raragd %s\n' "$*" >> "$log_root/calls.log"
trap 'exit 0' TERM INT
while true; do
  sleep 1
done
EOF
  chmod +x "$BIN_DIR/raragd"

  cat > "$BIN_DIR/rarag" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
log_root="${TEST_LOG_ROOT:?}"
printf 'rarag %s\n' "$*" >> "$log_root/calls.log"

config_path=""
for ((i=1; i<=$#; i++)); do
  if [[ "${!i}" == "--config" ]]; then
    next=$((i + 1))
    config_path="${!next}"
  fi
done

if [[ "$1" == "index" && "$2" == "build" ]]; then
  printf '{"status":"indexed"}\n'
  exit 0
fi

if [[ "$1" == "eval" && "$2" == "replay" ]]; then
  case "$config_path" in
    *baseline/config.toml)
      cat <<'JSON'
{
  "fixtures_path": "tasks.json",
  "task_count": 1,
  "ideal_task_hit_rate": 1.0,
  "acceptable_task_hit_rate": 0.0,
  "distractor_task_hit_rate": 1.0,
  "tasks": [
    {
      "task_id": "reload-archaeology",
      "query_mode": "blast-radius",
      "query_text": "archaeology",
      "result_count": 10,
      "ideal_hits": 1,
      "acceptable_hits": 0,
      "distractor_hits": 1,
      "matched_ideal": ["history:h1"],
      "matched_acceptable": [],
      "matched_distractors": ["docs/plans/future-work.md"],
      "evidence_class_coverage": ["history"],
      "warnings": ["history selector requested but no history candidates were found"]
    }
  ]
}
JSON
      ;;
    *cycle-1/experiment-01/config.toml)
      cat <<'JSON'
{
  "fixtures_path": "tasks.json",
  "task_count": 1,
  "ideal_task_hit_rate": 1.0,
  "acceptable_task_hit_rate": 1.0,
  "distractor_task_hit_rate": 0.0,
  "tasks": [
    {
      "task_id": "reload-archaeology",
      "query_mode": "blast-radius",
      "query_text": "archaeology",
      "result_count": 8,
      "ideal_hits": 1,
      "acceptable_hits": 1,
      "distractor_hits": 0,
      "matched_ideal": ["history:h1"],
      "matched_acceptable": ["docs/ops/reload.md"],
      "matched_distractors": [],
      "evidence_class_coverage": ["history", "document"],
      "warnings": []
    }
  ]
}
JSON
      ;;
    *)
      cat <<'JSON'
{
  "fixtures_path": "tasks.json",
  "task_count": 1,
  "ideal_task_hit_rate": 1.0,
  "acceptable_task_hit_rate": 0.0,
  "distractor_task_hit_rate": 1.0,
  "tasks": [
    {
      "task_id": "reload-archaeology",
      "query_mode": "blast-radius",
      "query_text": "archaeology",
      "result_count": 10,
      "ideal_hits": 1,
      "acceptable_hits": 0,
      "distractor_hits": 1,
      "matched_ideal": ["history:h1"],
      "matched_acceptable": [],
      "matched_distractors": ["docs/plans/future-work.md"],
      "evidence_class_coverage": ["history"],
      "warnings": ["history selector requested but no history candidates were found"]
    }
  ]
}
JSON
      ;;
  esac
  exit 0
fi

echo "unsupported fake rarag invocation: $*" >&2
exit 1
EOF
  chmod +x "$BIN_DIR/rarag"
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "run-optimization-experiments preserves baseline and candidate artifacts" {
  run env \
    TEST_LOG_ROOT="$TMP_DIR" \
    "$ROOT/scripts/run-optimization-experiments.sh" \
    --binary-root "$BIN_DIR" \
    --base-config "$ROOT/examples/rarag.example.toml" \
    --fixtures "$FIXTURES_PATH" \
    --workspace-root "$ROOT/tests/fixtures/compat_repo" \
    --repo-root "$ROOT/tests/fixtures/compat_repo" \
    --worktree "$ROOT/tests/fixtures/compat_repo" \
    --cycles 1 \
    --experiments-per-cycle 2 \
    --output-root "$ARTIFACT_ROOT" \
    --run-id test-run
  [ "$status" -eq 0 ]

  [ -f "$ARTIFACT_ROOT/test-run/cycle-1/baseline/config.toml" ]
  [ -f "$ARTIFACT_ROOT/test-run/cycle-1/baseline/report.json" ]
  [ -f "$ARTIFACT_ROOT/test-run/cycle-1/experiment-01/config.toml" ]
  [ -f "$ARTIFACT_ROOT/test-run/cycle-1/experiment-01/report.json" ]
  [ -f "$ARTIFACT_ROOT/test-run/cycle-1/experiment-02/report.json" ]
  [ -f "$ARTIFACT_ROOT/test-run/cycle-1/analysis.md" ]
}

@test "run-optimization-experiments keeps interesting diagnostics for metric-changing runs" {
  run env \
    TEST_LOG_ROOT="$TMP_DIR" \
    "$ROOT/scripts/run-optimization-experiments.sh" \
    --binary-root "$BIN_DIR" \
    --base-config "$ROOT/examples/rarag.example.toml" \
    --fixtures "$FIXTURES_PATH" \
    --workspace-root "$ROOT/tests/fixtures/compat_repo" \
    --repo-root "$ROOT/tests/fixtures/compat_repo" \
    --worktree "$ROOT/tests/fixtures/compat_repo" \
    --cycles 1 \
    --experiments-per-cycle 2 \
    --output-root "$ARTIFACT_ROOT" \
    --run-id interesting-run
  [ "$status" -eq 0 ]

  [ -f "$ARTIFACT_ROOT/interesting-run/cycle-1/experiment-01/interesting/daemon.log" ]
  [ ! -d "$ARTIFACT_ROOT/interesting-run/cycle-1/experiment-02/interesting" ]

  run rg '^best_candidate: experiment-01$' "$ARTIFACT_ROOT/interesting-run/cycle-1/analysis.md"
  [ "$status" -eq 0 ]
}
