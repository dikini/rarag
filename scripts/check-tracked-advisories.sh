#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

registry_path="docs/security/advisories.toml"
audit_json=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-tracked-advisories.sh [--registry <path>] [--audit-json <path>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --registry)
      registry_path="$2"
      shift 2
      ;;
    --audit-json)
      audit_json="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "tracked-advisories: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

[[ -f "$registry_path" ]] || {
  echo "tracked-advisories: missing registry file: $registry_path" >&2
  exit 1
}

if [[ -z "$audit_json" ]]; then
  cargo audit --version >/dev/null 2>&1 || {
    echo "tracked-advisories: cargo-audit is required" >&2
    exit 1
  }
  audit_json="$(mktemp)"
  trap 'rm -f "$audit_json"' EXIT
  cargo audit --json >"$audit_json"
fi

[[ -f "$audit_json" ]] || {
  echo "tracked-advisories: missing audit json: $audit_json" >&2
  exit 1
}

python3 - "$registry_path" "$audit_json" <<'PY'
import datetime as dt
import json
import pathlib
import sys
import tomllib

registry_path = pathlib.Path(sys.argv[1])
audit_path = pathlib.Path(sys.argv[2])

with registry_path.open("rb") as handle:
    registry = tomllib.load(handle)

tracked = registry.get("tracked", [])
if not tracked:
    raise SystemExit("tracked-advisories: registry has no [[tracked]] entries")

tracked_by_id = {}
for item in tracked:
    advisory_id = item.get("id")
    if not advisory_id:
        raise SystemExit("tracked-advisories: each entry requires id")
    if advisory_id in tracked_by_id:
        raise SystemExit(f"tracked-advisories: duplicate id in registry: {advisory_id}")
    tracked_by_id[advisory_id] = item

with audit_path.open("r", encoding="utf-8") as handle:
    report = json.load(handle)

warnings = report.get("warnings", {})
warning_items = []
for group in warnings.values():
    if isinstance(group, list):
        warning_items.extend(group)

present_ids = set()
for item in warning_items:
    advisory = item.get("advisory") or {}
    advisory_id = advisory.get("id")
    if advisory_id:
        present_ids.add(advisory_id)

untracked = sorted(present_ids - set(tracked_by_id.keys()))
if untracked:
    ids = ", ".join(untracked)
    raise SystemExit(
        f"tracked-advisories: untracked cargo-audit warnings present: {ids}. "
        f"Add entries to {registry_path.as_posix()}."
    )

today = dt.date.today()
for advisory_id, item in sorted(tracked_by_id.items()):
    if advisory_id not in present_ids:
        raise SystemExit(
            f"tracked-advisories: tracked advisory {advisory_id} is not present anymore; "
            f"remove/update {registry_path.as_posix()}."
        )

    review_raw = item.get("review_by")
    owner = item.get("owner", "unassigned")
    tracking_task = item.get("tracking_task", "n/a")
    via = item.get("introduced_via", "n/a")

    if not review_raw:
        raise SystemExit(f"tracked-advisories: {advisory_id} missing review_by")

    review_date = dt.date.fromisoformat(review_raw)
    if review_date < today:
        raise SystemExit(
            f"tracked-advisories: {advisory_id} review_by {review_raw} has passed "
            f"(today={today.isoformat()}); re-evaluate or refresh due date."
        )

    print(
        "::warning title=Tracked advisory::"
        f"{advisory_id} remains in dependency graph (owner={owner}, "
        f"review_by={review_raw}, task={tracking_task}, via={via})"
    )

print(
    f"tracked-advisories: OK ({len(present_ids)} tracked advisory warning(s) present and reviewed)"
)
PY
