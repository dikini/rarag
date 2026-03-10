#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

binary_root="$ROOT/target/release"
base_config="$ROOT/examples/rarag.example.toml"
fixtures="$ROOT/tests/fixtures/eval/tasks.json"
workspace_root="$ROOT/tests/fixtures/compat_repo"
repo_root="$ROOT/tests/fixtures/compat_repo"
worktree="$ROOT/tests/fixtures/compat_repo"
output_root="$ROOT/docs/ops/optimization-runs/generated"
runtime_root="${RARAG_OPT_RUNTIME_ROOT:-/tmp/rarag-opt-runtime}"
run_id="$(date +%Y-%m-%d-run-%H%M%S)"
cycles=3
experiments_per_cycle=10
history_max_nodes=4
limit=10
git_sha=""
include_history=0
method="heuristic"

usage() {
  cat <<'EOF'
Usage:
  scripts/run-optimization-experiments.sh [options]

Options:
  --binary-root <path>          Root containing prebuilt rarag and raragd binaries
  --base-config <path>          Baseline TOML config to mutate per experiment
  --fixtures <path>             Eval fixture JSON
  --workspace-root <path>       Workspace root for index build
  --repo-root <path>            Repo root for index build
  --worktree <path>             Worktree path for eval replay
  --git-sha <sha>               Pinned revision; defaults to single revision in fixtures
  --output-root <path>          Parent directory for generated run artifacts
  --run-id <id>                 Stable run identifier directory name
  --cycles <n>                  Number of optimization cycles
  --experiments-per-cycle <n>   Number of candidate experiments per cycle
  --history-max-nodes <n>       Eval replay history max nodes
  --limit <n>                   Eval replay result limit
  --include-history             Include history retrieval during replay
  --method <name>               Candidate method: heuristic | random-jitter
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary-root)
      binary_root="$2"
      shift 2
      ;;
    --base-config)
      base_config="$2"
      shift 2
      ;;
    --fixtures)
      fixtures="$2"
      shift 2
      ;;
    --workspace-root)
      workspace_root="$2"
      shift 2
      ;;
    --repo-root)
      repo_root="$2"
      shift 2
      ;;
    --worktree)
      worktree="$2"
      shift 2
      ;;
    --git-sha)
      git_sha="$2"
      shift 2
      ;;
    --output-root)
      output_root="$2"
      shift 2
      ;;
    --run-id)
      run_id="$2"
      shift 2
      ;;
    --cycles)
      cycles="$2"
      shift 2
      ;;
    --experiments-per-cycle)
      experiments_per_cycle="$2"
      shift 2
      ;;
    --history-max-nodes)
      history_max_nodes="$2"
      shift 2
      ;;
    --limit)
      limit="$2"
      shift 2
      ;;
    --include-history)
      include_history=1
      shift
      ;;
    --method)
      method="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "run-optimization-experiments: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

rarag_bin="$binary_root/rarag"
raragd_bin="$binary_root/raragd"

[[ -x "$rarag_bin" ]] || {
  echo "run-optimization-experiments: missing executable $rarag_bin" >&2
  exit 1
}
[[ -x "$raragd_bin" ]] || {
  echo "run-optimization-experiments: missing executable $raragd_bin" >&2
  exit 1
}
[[ -f "$base_config" ]] || {
  echo "run-optimization-experiments: missing base config $base_config" >&2
  exit 1
}
[[ -f "$fixtures" ]] || {
  echo "run-optimization-experiments: missing fixtures $fixtures" >&2
  exit 1
}

if [[ -z "$git_sha" ]]; then
  git_sha="$(
    python3 - "$fixtures" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    tasks = json.load(handle)
revisions = sorted({task["revision"] for task in tasks})
if len(revisions) != 1:
    raise SystemExit("fixture revisions must contain exactly one unique value")
print(revisions[0])
PY
  )"
fi

mkdir -p "$output_root"
run_dir="$output_root/$run_id"
mkdir -p "$run_dir"

python3 - "$base_config" "$run_dir/base-params.json" <<'PY'
import json
import sys
import tomllib

with open(sys.argv[1], "rb") as handle:
    config = tomllib.load(handle)

rules = {rule["path_glob"]: rule["weight"] for rule in config["document_sources"]["rules"]}
params = {
    "document_specs_weight": rules["docs/specs/**"],
    "document_ops_weight": rules["docs/ops/**"],
    "document_plans_weight": rules["docs/plans/**"],
    "document_changelog_weight": rules["CHANGELOG.md"],
    "document_tasks_weight": rules["docs/tasks/tasks.csv"],
    "blast_radius_test_like": config["retrieval"]["rerank"]["blast_radius_test_like"],
    "blast_radius_other": config["retrieval"]["rerank"]["blast_radius_other"],
    "implement_adjacent_body_region": config["retrieval"]["rerank"]["implement_adjacent_body_region"],
    "bounded_refactor_test_like": config["retrieval"]["rerank"]["bounded_refactor_test_like"],
    "bounded_refactor_other": config["retrieval"]["rerank"]["bounded_refactor_other"],
    "find_examples_example_like": config["retrieval"]["rerank"]["find_examples_example_like"],
    "find_examples_other": config["retrieval"]["rerank"]["find_examples_other"],
    "worktree_diff_blast_radius": config["retrieval"]["rerank"]["worktree_diff_blast_radius"],
    "worktree_diff_implement_adjacent": config["retrieval"]["rerank"]["worktree_diff_implement_adjacent"],
    "worktree_diff_bounded_refactor": config["retrieval"]["rerank"]["worktree_diff_bounded_refactor"],
    "worktree_diff_find_examples": config["retrieval"]["rerank"]["worktree_diff_find_examples"],
    "text_reference_implement_adjacent": config["retrieval"]["neighborhood"]["text_reference_implement_adjacent"],
    "text_reference_bounded_refactor": config["retrieval"]["neighborhood"]["text_reference_bounded_refactor"],
    "text_reference_find_examples": config["retrieval"]["neighborhood"]["text_reference_find_examples"],
    "text_reference_blast_radius": config["retrieval"]["neighborhood"]["text_reference_blast_radius"],
    "semantic_reference_implement_adjacent": config["retrieval"]["neighborhood"]["semantic_reference_implement_adjacent"],
    "semantic_reference_bounded_refactor": config["retrieval"]["neighborhood"]["semantic_reference_bounded_refactor"],
    "semantic_reference_find_examples": config["retrieval"]["neighborhood"]["semantic_reference_find_examples"],
    "semantic_reference_blast_radius": config["retrieval"]["neighborhood"]["semantic_reference_blast_radius"],
    "semantic_impl_implement_adjacent": config["retrieval"]["neighborhood"]["semantic_impl_implement_adjacent"],
    "semantic_impl_bounded_refactor": config["retrieval"]["neighborhood"]["semantic_impl_bounded_refactor"],
    "semantic_impl_find_examples": config["retrieval"]["neighborhood"]["semantic_impl_find_examples"],
}
with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(params, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

generate_candidates() {
  local cycle_number="$1"
  local output_json="$2"
  local previous_summary="${3:-}"
  python3 - "$cycle_number" "$experiments_per_cycle" "$run_dir/base-params.json" "$output_json" "$previous_summary" "$method" <<'PY'
import json
import random
import sys

cycle = int(sys.argv[1])
count = int(sys.argv[2])
with open(sys.argv[3], "r", encoding="utf-8") as handle:
    base = json.load(handle)
output_path = sys.argv[4]
summary_path = sys.argv[5] if len(sys.argv) > 5 and sys.argv[5] else None
method = sys.argv[6]

PARAM_LIMITS = {
    "document_specs_weight": (0.1, 3.0),
    "document_ops_weight": (0.1, 3.0),
    "document_plans_weight": (0.0, 2.0),
    "document_changelog_weight": (0.0, 2.0),
    "document_tasks_weight": (0.1, 3.0),
    "blast_radius_test_like": (0.0, 3.0),
    "blast_radius_other": (0.0, 3.0),
    "implement_adjacent_body_region": (0.0, 3.0),
    "bounded_refactor_test_like": (0.0, 3.0),
    "bounded_refactor_other": (0.0, 3.0),
    "find_examples_example_like": (0.0, 3.0),
    "find_examples_other": (0.0, 3.0),
    "worktree_diff_blast_radius": (0.0, 3.0),
    "worktree_diff_implement_adjacent": (0.0, 3.0),
    "worktree_diff_bounded_refactor": (0.0, 3.0),
    "worktree_diff_find_examples": (0.0, 3.0),
    "text_reference_implement_adjacent": (0.0, 12.0),
    "text_reference_bounded_refactor": (0.0, 12.0),
    "text_reference_find_examples": (0.0, 12.0),
    "text_reference_blast_radius": (0.0, 12.0),
    "semantic_reference_implement_adjacent": (0.0, 12.0),
    "semantic_reference_bounded_refactor": (0.0, 12.0),
    "semantic_reference_find_examples": (0.0, 12.0),
    "semantic_reference_blast_radius": (0.0, 12.0),
    "semantic_impl_implement_adjacent": (0.0, 12.0),
    "semantic_impl_bounded_refactor": (0.0, 12.0),
    "semantic_impl_find_examples": (0.0, 12.0),
}

def clamp(value, key):
    low, high = PARAM_LIMITS[key]
    return round(min(max(value, low), high), 3)

def apply_delta(source, delta):
    updated = dict(source)
    for key, shift in delta.items():
        updated[key] = clamp(updated[key] + shift, key)
    return updated

def candidate(hypothesis, delta):
    return {"hypothesis": hypothesis, "params": apply_delta(base if cycle == 1 else best_params, delta)}

def random_jitter_candidates(seed_source, center, count):
    rng = random.Random(seed_source)
    keys = sorted(PARAM_LIMITS.keys())
    candidates = []
    for index in range(1, count + 1):
        params = dict(center)
        changed = []
        sample_keys = rng.sample(keys, k=min(6, len(keys)))
        for key in sample_keys:
            low, high = PARAM_LIMITS[key]
            span = high - low
            step = span * rng.uniform(-0.18, 0.18)
            params[key] = clamp(params[key] + step, key)
            if params[key] != center[key]:
                changed.append(f"{key}={params[key]}")
        candidates.append({
            "experiment_id": f"experiment-{index:02d}",
            "hypothesis": "Random jitter around best-known region" if cycle > 1 else "Random jitter around baseline",
            "params": params,
            "changed": changed,
        })
    return candidates

if method == "random-jitter":
    if cycle == 1:
        candidates = random_jitter_candidates(seed_source=cycle * 991, center=base, count=count)
    else:
        with open(summary_path, "r", encoding="utf-8") as handle:
            summary = json.load(handle)
        best_params = summary["best_params"]
        candidates = random_jitter_candidates(seed_source=cycle * 991, center=best_params, count=count)
elif cycle == 1:
    deltas = [
        ("Favor normative docs over plans/changelog", {
            "document_specs_weight": 0.3,
            "document_ops_weight": 0.2,
            "document_plans_weight": -0.2,
            "document_changelog_weight": -0.1,
        }),
        ("Push harder toward specs and ops", {
            "document_specs_weight": 0.6,
            "document_ops_weight": 0.5,
            "document_plans_weight": -0.4,
            "document_changelog_weight": -0.2,
        }),
        ("Strongly penalize future-plan evidence", {
            "document_specs_weight": 0.8,
            "document_plans_weight": -0.6,
            "document_changelog_weight": -0.3,
        }),
        ("Prefer ops evidence for reload archaeology", {
            "document_ops_weight": 0.8,
            "document_plans_weight": -0.5,
            "document_changelog_weight": -0.4,
        }),
        ("Boost task registry alongside spec evidence", {
            "document_tasks_weight": 0.4,
            "document_specs_weight": 0.2,
            "document_plans_weight": -0.4,
            "document_changelog_weight": -0.2,
        }),
        ("Lower generic blast-radius noise and raise diff locality", {
            "blast_radius_other": -0.15,
            "worktree_diff_blast_radius": 0.4,
            "document_plans_weight": -0.4,
        }),
        ("Lift implement-adjacent code locality", {
            "implement_adjacent_body_region": 0.25,
            "worktree_diff_implement_adjacent": 0.3,
            "text_reference_implement_adjacent": 0.4,
            "semantic_reference_implement_adjacent": 0.3,
        }),
        ("Lift bounded-refactor code+test relevance", {
            "bounded_refactor_test_like": 0.25,
            "bounded_refactor_other": -0.1,
            "worktree_diff_bounded_refactor": 0.3,
            "text_reference_bounded_refactor": 0.35,
            "semantic_impl_bounded_refactor": 0.3,
        }),
        ("Lift find-examples precision", {
            "find_examples_example_like": 0.25,
            "find_examples_other": -0.1,
            "worktree_diff_find_examples": 0.2,
            "text_reference_find_examples": 0.35,
            "semantic_impl_find_examples": 0.25,
        }),
        ("Reward test-like blast-radius hits while suppressing plan noise", {
            "blast_radius_test_like": 0.25,
            "blast_radius_other": -0.1,
            "document_plans_weight": -0.5,
        }),
        ("Raise blast-radius neighborhood references", {
            "text_reference_blast_radius": 0.5,
            "semantic_reference_blast_radius": 0.3,
            "document_plans_weight": -0.4,
        }),
        ("Blend normative docs with stronger blast-radius neighborhoods", {
            "document_specs_weight": 0.4,
            "document_ops_weight": 0.4,
            "text_reference_blast_radius": 0.4,
            "document_plans_weight": -0.5,
        }),
        ("Aggressively bias toward current docs and diff context", {
            "document_specs_weight": 0.3,
            "document_ops_weight": 0.6,
            "worktree_diff_blast_radius": 0.5,
            "document_plans_weight": -0.3,
            "document_changelog_weight": -0.2,
        }),
    ]
    candidates = [candidate(h, d) for h, d in deltas[:count]]
else:
    with open(summary_path, "r", encoding="utf-8") as handle:
        summary = json.load(handle)
    best_params = summary["best_params"]
    trend_delta = summary["trend_delta"]
    best_metrics = summary["best_metrics"]
    candidates = []
    scales = [0.5, 1.0, 1.5]
    candidates.append({"hypothesis": "Repeat prior best candidate", "params": best_params})
    for scale in scales:
        delta = {key: round(value * scale, 3) for key, value in trend_delta.items()}
        candidates.append({
            "hypothesis": f"Follow top-trend direction at scale {scale}",
            "params": apply_delta(best_params, delta),
        })
    if best_metrics["distractor_task_hit_rate"] > 0:
        candidates.append(candidate("Reduce distractor-prone plan and changelog evidence further", {
            "document_plans_weight": -0.25,
            "document_changelog_weight": -0.15,
            "bounded_refactor_other": -0.1,
            "find_examples_other": -0.1,
        }))
        candidates.append(candidate("Reduce generic blast-radius other score", {
            "blast_radius_other": -0.15,
            "document_plans_weight": -0.15,
        }))
    else:
        candidates.append(candidate("Keep low distractors while testing tighter result counts", {
            "document_plans_weight": -0.1,
            "worktree_diff_blast_radius": 0.15,
        }))
        candidates.append(candidate("Probe smaller neighborhood with stronger normative docs", {
            "document_specs_weight": 0.15,
            "text_reference_blast_radius": -0.2,
        }))
    if best_metrics["acceptable_task_hit_rate"] < 1.0:
        candidates.append(candidate("Raise acceptable evidence via specs and ops", {
            "document_specs_weight": 0.2,
            "document_ops_weight": 0.2,
            "implement_adjacent_body_region": 0.15,
        }))
        candidates.append(candidate("Raise acceptable evidence via blast-radius locality", {
            "worktree_diff_blast_radius": 0.2,
            "text_reference_blast_radius": 0.25,
            "semantic_reference_blast_radius": 0.15,
        }))
        candidates.append(candidate("Raise acceptable evidence via code-mode locality", {
            "worktree_diff_implement_adjacent": 0.2,
            "worktree_diff_bounded_refactor": 0.2,
            "worktree_diff_find_examples": 0.15,
            "text_reference_implement_adjacent": 0.25,
            "text_reference_bounded_refactor": 0.2,
            "text_reference_find_examples": 0.2,
        }))
    else:
        candidates.append(candidate("Hold acceptable hits and trim ops inflation", {
            "document_ops_weight": -0.1,
            "document_specs_weight": 0.1,
        }))
        candidates.append(candidate("Hold acceptable hits and trim broad neighborhoods", {
            "text_reference_blast_radius": -0.15,
            "semantic_reference_blast_radius": -0.1,
        }))
    candidates.append(candidate("Explore stronger task-registry support", {
        "document_tasks_weight": 0.25,
        "document_plans_weight": -0.1,
    }))
    candidates.append(candidate("Explore stronger ops plus diff locality", {
        "document_ops_weight": 0.2,
        "worktree_diff_blast_radius": 0.2,
    }))
    candidates = candidates[:count]

if method != "random-jitter":
    for index, item in enumerate(candidates, start=1):
        item["experiment_id"] = f"experiment-{index:02d}"

with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(candidates, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

write_config() {
  local destination="$1"
  local runtime_json="$2"
  local params_json="$3"
  python3 - "$base_config" "$destination" "$runtime_json" "$params_json" <<'PY'
import json
import sys
import tomllib

def toml_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    raise TypeError(f"unsupported value: {value!r}")

def emit_table(lines, prefix, table):
    scalars = []
    child_tables = []
    array_tables = []
    for key, value in table.items():
        if isinstance(value, dict):
            child_tables.append((key, value))
        elif isinstance(value, list) and value and all(isinstance(item, dict) for item in value):
            array_tables.append((key, value))
        else:
            scalars.append((key, value))
    if prefix:
        lines.append(f"[{prefix}]")
    for key, value in scalars:
        lines.append(f"{key} = {toml_value(value)}")
    if prefix and (child_tables or array_tables):
        lines.append("")
    for index, (key, value) in enumerate(child_tables):
        child_prefix = f"{prefix}.{key}" if prefix else key
        emit_table(lines, child_prefix, value)
        if index != len(child_tables) - 1 or array_tables:
            lines.append("")
    for table_index, (key, values) in enumerate(array_tables):
        table_prefix = f"{prefix}.{key}" if prefix else key
        for value_index, item in enumerate(values):
          lines.append(f"[[{table_prefix}]]")
          for item_key, item_value in item.items():
              lines.append(f"{item_key} = {toml_value(item_value)}")
          if value_index != len(values) - 1 or table_index != len(array_tables) - 1:
              lines.append("")

with open(sys.argv[1], "rb") as handle:
    config = tomllib.load(handle)
runtime = json.loads(sys.argv[3])
with open(sys.argv[4], "r", encoding="utf-8") as handle:
    params = json.load(handle)

config["runtime"]["socket_path"] = runtime["socket_path"]
config["runtime"]["state_root"] = runtime["state_root"]
config["runtime"]["cache_root"] = runtime["cache_root"]
config["turso"]["database_url"] = f'file:{runtime["state_root"]}/metadata.db'
config["tantivy"]["index_root"] = f'{runtime["cache_root"]}/tantivy'
config["lancedb"]["db_root"] = f'{runtime["state_root"]}/lancedb'
config["daemon"]["socket_path"] = runtime["socket_path"]
config["mcp"]["socket_path"] = runtime["mcp_socket_path"]
config["observability"]["enabled"] = False
config["observability"]["verbosity"] = "off"

config["retrieval"]["rerank"]["blast_radius_test_like"] = params["blast_radius_test_like"]
config["retrieval"]["rerank"]["blast_radius_other"] = params["blast_radius_other"]
config["retrieval"]["rerank"]["implement_adjacent_body_region"] = params["implement_adjacent_body_region"]
config["retrieval"]["rerank"]["bounded_refactor_test_like"] = params["bounded_refactor_test_like"]
config["retrieval"]["rerank"]["bounded_refactor_other"] = params["bounded_refactor_other"]
config["retrieval"]["rerank"]["find_examples_example_like"] = params["find_examples_example_like"]
config["retrieval"]["rerank"]["find_examples_other"] = params["find_examples_other"]
config["retrieval"]["rerank"]["worktree_diff_blast_radius"] = params["worktree_diff_blast_radius"]
config["retrieval"]["rerank"]["worktree_diff_implement_adjacent"] = params["worktree_diff_implement_adjacent"]
config["retrieval"]["rerank"]["worktree_diff_bounded_refactor"] = params["worktree_diff_bounded_refactor"]
config["retrieval"]["rerank"]["worktree_diff_find_examples"] = params["worktree_diff_find_examples"]
config["retrieval"]["neighborhood"]["text_reference_implement_adjacent"] = params["text_reference_implement_adjacent"]
config["retrieval"]["neighborhood"]["text_reference_bounded_refactor"] = params["text_reference_bounded_refactor"]
config["retrieval"]["neighborhood"]["text_reference_find_examples"] = params["text_reference_find_examples"]
config["retrieval"]["neighborhood"]["text_reference_blast_radius"] = params["text_reference_blast_radius"]
config["retrieval"]["neighborhood"]["semantic_reference_implement_adjacent"] = params["semantic_reference_implement_adjacent"]
config["retrieval"]["neighborhood"]["semantic_reference_bounded_refactor"] = params["semantic_reference_bounded_refactor"]
config["retrieval"]["neighborhood"]["semantic_reference_find_examples"] = params["semantic_reference_find_examples"]
config["retrieval"]["neighborhood"]["semantic_reference_blast_radius"] = params["semantic_reference_blast_radius"]
config["retrieval"]["neighborhood"]["semantic_impl_implement_adjacent"] = params["semantic_impl_implement_adjacent"]
config["retrieval"]["neighborhood"]["semantic_impl_bounded_refactor"] = params["semantic_impl_bounded_refactor"]
config["retrieval"]["neighborhood"]["semantic_impl_find_examples"] = params["semantic_impl_find_examples"]

for rule in config["document_sources"]["rules"]:
    path_glob = rule["path_glob"]
    if path_glob == "docs/specs/**":
        rule["weight"] = params["document_specs_weight"]
    elif path_glob == "docs/ops/**":
        rule["weight"] = params["document_ops_weight"]
    elif path_glob == "docs/plans/**":
        rule["weight"] = params["document_plans_weight"]
    elif path_glob == "CHANGELOG.md":
        rule["weight"] = params["document_changelog_weight"]
    elif path_glob == "docs/tasks/tasks.csv":
        rule["weight"] = params["document_tasks_weight"]

lines = []
emit_table(lines, "", config)
with open(sys.argv[2], "w", encoding="utf-8") as handle:
    handle.write("\n".join(line for line in lines if line is not None).rstrip() + "\n")
PY
}

write_metadata() {
  local destination="$1"
  local label="$2"
  local cycle_number="$3"
  local socket_path="$4"
  local config_path="$5"
  local daemon_log="$6"
  local report_path="$7"
  local hypothesis="$8"
  python3 - "$destination" "$label" "$cycle_number" "$socket_path" "$config_path" "$daemon_log" "$report_path" "$fixtures" "$worktree" "$git_sha" "$hypothesis" <<'PY'
import json
import sys
from datetime import datetime, timezone

payload = {
    "label": sys.argv[2],
    "cycle": int(sys.argv[3]),
    "socket_path": sys.argv[4],
    "config_path": sys.argv[5],
    "daemon_log": sys.argv[6],
    "report_path": sys.argv[7],
    "fixtures_path": sys.argv[8],
    "worktree": sys.argv[9],
    "git_sha": sys.argv[10],
    "hypothesis": sys.argv[11],
    "recorded_at": datetime.now(timezone.utc).isoformat(),
}
with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

run_single_experiment() {
  local label="$1"
  local cycle_number="$2"
  local experiment_dir="$3"
  local params_json="$4"
  local hypothesis="$5"

  local runtime_label="$label"
  if [[ "$label" == experiment-* ]]; then
    runtime_label="e${label#experiment-}"
  elif [[ "$label" == "baseline" ]]; then
    runtime_label="b"
  fi
  local runtime_dir="$runtime_root/$run_id/c${cycle_number}/${runtime_label}"
  local state_root="$runtime_dir/state"
  local cache_root="$runtime_dir/cache"
  local socket_path="$runtime_dir/raragd.sock"
  local mcp_socket_path="$runtime_dir/rarag-mcp.sock"
  local config_path="$experiment_dir/config.toml"
  local report_path="$experiment_dir/report.json"
  local manifest_path="$experiment_dir/manifest.json"
  local daemon_log="$experiment_dir/daemon.log"

  mkdir -p "$state_root" "$cache_root"

  local runtime_json
  runtime_json="$(
    python3 - <<PY
import json
print(json.dumps({
  "socket_path": "$socket_path",
  "state_root": "$state_root",
  "cache_root": "$cache_root",
  "mcp_socket_path": "$mcp_socket_path"
}))
PY
  )"
  write_config "$config_path" "$runtime_json" "$params_json"

  "$raragd_bin" serve --config "$config_path" --socket "$socket_path" \
    --test-deterministic-embeddings --test-memory-vector-store \
    >"$daemon_log" 2>&1 &
  local daemon_pid=$!
  sleep 0.2

  local status=0
  if ! "$rarag_bin" index build \
    --config "$config_path" \
    --workspace-root "$workspace_root" \
    --repo-root "$repo_root" \
    --worktree "$worktree" \
    --git-sha "$git_sha" \
    --max-body-bytes 120 \
    --socket "$socket_path" \
    --json >"$experiment_dir/index.json"; then
    status=$?
  else
    replay_cmd=(
      "$rarag_bin" eval replay
      --config "$config_path"
      --fixtures "$fixtures"
      --worktree "$worktree"
      --history-max-nodes "$history_max_nodes"
      --limit "$limit"
      --socket "$socket_path"
      --json
    )
    if [[ $include_history -eq 1 ]]; then
      replay_cmd+=(--include-history)
    fi
    if ! "${replay_cmd[@]}" >"$report_path"; then
      status=$?
    fi
  fi

  kill "$daemon_pid" >/dev/null 2>&1 || true
  wait "$daemon_pid" >/dev/null 2>&1 || true

  write_metadata "$manifest_path" "$label" "$cycle_number" "$socket_path" "$config_path" "$daemon_log" "$report_path" "$hypothesis"

  if [[ $status -ne 0 ]]; then
    echo "run-optimization-experiments: $label failed" >&2
    return "$status"
  fi
}

mark_interesting() {
  local baseline_report="$1"
  local report_path="$2"
  local experiment_dir="$3"
  python3 - "$baseline_report" "$report_path" "$experiment_dir" <<'PY'
import json
import pathlib
import shutil
import sys

def load(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)

baseline = load(sys.argv[1])
report = load(sys.argv[2])
experiment_dir = pathlib.Path(sys.argv[3])

def aggregate(task_list):
    warnings = sorted({warning for task in task_list for warning in task.get("warnings", [])})
    classes = sorted({clazz for task in task_list for clazz in task.get("evidence_class_coverage", [])})
    result_count = round(sum(task.get("result_count", 0) for task in task_list) / len(task_list), 4) if task_list else 0
    return warnings, classes, result_count

baseline_warnings, baseline_classes, baseline_result_count = aggregate(baseline.get("tasks", []))
warnings, classes, result_count = aggregate(report.get("tasks", []))

interesting = (
    baseline["acceptable_task_hit_rate"] != report["acceptable_task_hit_rate"]
    or baseline["distractor_task_hit_rate"] != report["distractor_task_hit_rate"]
    or baseline["ideal_task_hit_rate"] != report["ideal_task_hit_rate"]
    or baseline_warnings != warnings
    or baseline_classes != classes
    or baseline_result_count != result_count
)

interesting_dir = experiment_dir / "interesting"
daemon_log = experiment_dir / "daemon.log"
if interesting:
    interesting_dir.mkdir(exist_ok=True)
    shutil.copy2(daemon_log, interesting_dir / "daemon.log")
else:
    if interesting_dir.exists():
        shutil.rmtree(interesting_dir)
PY
}

build_cycle_summary() {
  local cycle_dir="$1"
  local cycle_number="$2"
  local previous_summary="${3:-}"
  python3 - "$cycle_dir" "$cycle_number" "$run_dir/base-params.json" "$previous_summary" <<'PY'
import json
import pathlib
import statistics
import sys

cycle_dir = pathlib.Path(sys.argv[1])
cycle_number = int(sys.argv[2])
base_params_path = pathlib.Path(sys.argv[3])
previous_summary = pathlib.Path(sys.argv[4]) if len(sys.argv) > 4 and sys.argv[4] else None

with base_params_path.open("r", encoding="utf-8") as handle:
    base_params = json.load(handle)
with (cycle_dir / "baseline" / "report.json").open("r", encoding="utf-8") as handle:
    baseline_report = json.load(handle)

experiments = []
for experiment_dir in sorted(path for path in cycle_dir.iterdir() if path.name.startswith("experiment-")):
    with (experiment_dir / "report.json").open("r", encoding="utf-8") as handle:
        report = json.load(handle)
    with (experiment_dir / "params.json").open("r", encoding="utf-8") as handle:
        params = json.load(handle)
    with (experiment_dir / "manifest.json").open("r", encoding="utf-8") as handle:
        manifest = json.load(handle)
    tasks = report.get("tasks", [])
    avg_result_count = round(sum(task.get("result_count", 0) for task in tasks) / len(tasks), 4) if tasks else 0
    warning_task_rate = round(
        sum(1 for task in tasks if task.get("warnings")) / len(tasks), 4
    ) if tasks else 0
    evidence_union = sorted({clazz for task in tasks for clazz in task.get("evidence_class_coverage", [])})
    warning_union = sorted({warning for task in tasks for warning in task.get("warnings", [])})
    objective = (
        report["acceptable_task_hit_rate"]
        - report["distractor_task_hit_rate"]
        + 0.1 * report["ideal_task_hit_rate"]
        - 0.01 * avg_result_count
        - 0.02 * warning_task_rate
    )
    experiments.append({
        "experiment_id": experiment_dir.name,
        "objective": round(objective, 4),
        "acceptable_task_hit_rate": report["acceptable_task_hit_rate"],
        "distractor_task_hit_rate": report["distractor_task_hit_rate"],
        "ideal_task_hit_rate": report["ideal_task_hit_rate"],
        "avg_result_count": avg_result_count,
        "warning_task_rate": warning_task_rate,
        "warnings": warning_union,
        "evidence_class_coverage": evidence_union,
        "hypothesis": manifest.get("hypothesis", ""),
        "params": params,
    })

experiments.sort(key=lambda item: (-item["objective"], -item["acceptable_task_hit_rate"], item["distractor_task_hit_rate"], item["avg_result_count"], item["warning_task_rate"], item["experiment_id"]))
best = experiments[0]
top = experiments[: min(3, len(experiments))]
trend_delta = {}
for key, value in base_params.items():
    average = statistics.mean(item["params"][key] for item in top)
    trend_delta[key] = round(average - value, 3)

analysis_lines = [
    f"# Cycle {cycle_number} Analysis",
    "",
    f"best_candidate: {best['experiment_id']}",
    f"best_objective: {best['objective']}",
    "",
    "## Baseline Metrics",
    "",
    f"- ideal_task_hit_rate: {baseline_report['ideal_task_hit_rate']}",
    f"- acceptable_task_hit_rate: {baseline_report['acceptable_task_hit_rate']}",
    f"- distractor_task_hit_rate: {baseline_report['distractor_task_hit_rate']}",
    "",
    "## Candidate Results",
    "",
]
for item in experiments:
    analysis_lines.extend([
        f"- {item['experiment_id']}: acceptable={item['acceptable_task_hit_rate']}, distractor={item['distractor_task_hit_rate']}, ideal={item['ideal_task_hit_rate']}, avg_result_count={item['avg_result_count']}, warning_task_rate={item['warning_task_rate']}, objective={item['objective']}",
        f"  hypothesis: {item['hypothesis']}",
        f"  evidence_class_coverage: {', '.join(item['evidence_class_coverage']) or 'none'}",
        f"  warnings: {', '.join(item['warnings']) or 'none'}",
    ])
    changed = []
    for key, baseline_value in base_params.items():
        experiment_value = item["params"][key]
        if experiment_value != baseline_value:
            changed.append(f"{key}={experiment_value}")
    if changed:
        analysis_lines.append(f"  changed_params: {', '.join(changed)}")
    else:
        analysis_lines.append("  changed_params: none")
analysis_lines.extend([
    "",
    "## Trend Summary",
    "",
    "- top-three average parameter movement relative to baseline:",
])
for key, value in trend_delta.items():
    analysis_lines.append(f"  {key}: {value:+.3f}")

analysis_lines.extend([
    "",
    "## Next-Cycle Hypotheses",
    "",
])

if best["acceptable_task_hit_rate"] < 1.0:
    analysis_lines.append("- Acceptable hits remain below ceiling, so the next cycle should keep pushing specs/ops and blast-radius locality.")
else:
    analysis_lines.append("- Acceptable hits are already at ceiling for the current fixture set, so the next cycle should focus on keeping that level while trimming noise.")

if best["distractor_task_hit_rate"] > 0.0:
    analysis_lines.append("- Distractors remain present, so the next cycle should continue penalizing plan/changelog evidence and generic blast-radius weights.")
else:
    analysis_lines.append("- Distractors are suppressed in the best candidate, so the next cycle can refine around lower result counts without relaxing the current plan/changelog penalties.")

if previous_summary and previous_summary.exists():
    with previous_summary.open("r", encoding="utf-8") as handle:
        previous = json.load(handle)
    analysis_lines.append(f"- Prior cycle best was {previous['best_experiment_id']}; compare whether the new best preserves or improves that tradeoff.")

analysis_path = cycle_dir / "analysis.md"
analysis_path.write_text("\n".join(analysis_lines) + "\n", encoding="utf-8")

summary = {
    "cycle": cycle_number,
    "best_experiment_id": best["experiment_id"],
    "best_params": best["params"],
    "best_metrics": {
        "ideal_task_hit_rate": best["ideal_task_hit_rate"],
        "acceptable_task_hit_rate": best["acceptable_task_hit_rate"],
        "distractor_task_hit_rate": best["distractor_task_hit_rate"],
        "avg_result_count": best["avg_result_count"],
        "warning_task_rate": best["warning_task_rate"],
    },
    "trend_delta": trend_delta,
    "baseline_metrics": {
        "ideal_task_hit_rate": baseline_report["ideal_task_hit_rate"],
        "acceptable_task_hit_rate": baseline_report["acceptable_task_hit_rate"],
        "distractor_task_hit_rate": baseline_report["distractor_task_hit_rate"],
    },
    "experiments": experiments,
}
with (cycle_dir / "cycle-summary.json").open("w", encoding="utf-8") as handle:
    json.dump(summary, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

for ((cycle_number = 1; cycle_number <= cycles; cycle_number++)); do
  cycle_dir="$run_dir/cycle-$cycle_number"
  mkdir -p "$cycle_dir/baseline"

  baseline_params="$cycle_dir/baseline/params.json"
  cp "$run_dir/base-params.json" "$baseline_params"
  run_single_experiment "baseline" "$cycle_number" "$cycle_dir/baseline" "$baseline_params" "Baseline config for comparison"

  previous_summary=""
  if ((cycle_number > 1)); then
    previous_summary="$run_dir/cycle-$((cycle_number - 1))/cycle-summary.json"
  fi
  candidates_json="$cycle_dir/candidates.json"
  generate_candidates "$cycle_number" "$candidates_json" "$previous_summary"

  while IFS= read -r experiment_id; do
    experiment_dir="$cycle_dir/$experiment_id"
    mkdir -p "$experiment_dir"
    params_path="$experiment_dir/params.json"
    hypothesis="$(
      python3 - "$candidates_json" "$experiment_id" "$params_path" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    candidates = json.load(handle)
match = next(item for item in candidates if item["experiment_id"] == sys.argv[2])
with open(sys.argv[3], "w", encoding="utf-8") as handle:
    json.dump(match["params"], handle, indent=2, sort_keys=True)
    handle.write("\n")
print(match["hypothesis"])
PY
    )"
    run_single_experiment "$experiment_id" "$cycle_number" "$experiment_dir" "$params_path" "$hypothesis"
    mark_interesting "$cycle_dir/baseline/report.json" "$experiment_dir/report.json" "$experiment_dir"
  done < <(
    python3 - "$candidates_json" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    for candidate in json.load(handle):
        print(candidate["experiment_id"])
PY
  )

  build_cycle_summary "$cycle_dir" "$cycle_number" "$previous_summary"
done

python3 - "$run_dir" "$cycles" "$experiments_per_cycle" <<'PY'
import json
import pathlib
import sys

run_dir = pathlib.Path(sys.argv[1])
cycles = int(sys.argv[2])
experiments_per_cycle = int(sys.argv[3])

summary = {
    "run_dir": str(run_dir),
    "cycles": [],
}

for cycle in range(1, cycles + 1):
    cycle_dir = run_dir / f"cycle-{cycle}"
    with (cycle_dir / "cycle-summary.json").open("r", encoding="utf-8") as handle:
        cycle_summary = json.load(handle)
    summary["cycles"].append({
        "cycle": cycle,
        "best_experiment_id": cycle_summary["best_experiment_id"],
        "best_metrics": cycle_summary["best_metrics"],
    })
    experiment_dirs = sorted(path.name for path in cycle_dir.iterdir() if path.name.startswith("experiment-"))
    if len(experiment_dirs) != experiments_per_cycle:
        raise SystemExit(f"expected {experiments_per_cycle} experiments in {cycle_dir}, found {len(experiment_dirs)}")
    required = [
        cycle_dir / "baseline" / "config.toml",
        cycle_dir / "baseline" / "report.json",
        cycle_dir / "analysis.md",
        cycle_dir / "cycle-summary.json",
    ]
    for path in required:
        if not path.exists():
            raise SystemExit(f"missing required artifact {path}")

with (run_dir / "run-summary.json").open("w", encoding="utf-8") as handle:
    json.dump(summary, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

echo "run-optimization-experiments: artifacts written to $run_dir"
