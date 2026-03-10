#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_DIR="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "tracked-advisories passes when all warning IDs are tracked and current" {
  cat >"$TMP_DIR/registry.toml" <<'EOF'
[[tracked]]
id = "RUSTSEC-2024-0436"
owner = "repo-maintainers"
review_by = "2099-01-01"
tracking_task = "task-1"
introduced_via = "transitive"
notes = "tracked"
EOF

  cat >"$TMP_DIR/audit.json" <<'EOF'
{
  "warnings": {
    "unmaintained": [
      {
        "kind": "unmaintained",
        "advisory": {"id": "RUSTSEC-2024-0436"}
      }
    ]
  }
}
EOF

  run "$ROOT/scripts/check-tracked-advisories.sh" --registry "$TMP_DIR/registry.toml" --audit-json "$TMP_DIR/audit.json"
  [ "$status" -eq 0 ]
  run rg 'tracked-advisories: OK' <<<"$output"
  [ "$status" -eq 0 ]
}

@test "tracked-advisories fails when warning IDs are not tracked" {
  cat >"$TMP_DIR/registry.toml" <<'EOF'
[[tracked]]
id = "RUSTSEC-2024-0436"
owner = "repo-maintainers"
review_by = "2099-01-01"
tracking_task = "task-1"
introduced_via = "transitive"
notes = "tracked"
EOF

  cat >"$TMP_DIR/audit.json" <<'EOF'
{
  "warnings": {
    "unsound": [
      {
        "kind": "unsound",
        "advisory": {"id": "RUSTSEC-2026-0002"}
      }
    ]
  }
}
EOF

  run "$ROOT/scripts/check-tracked-advisories.sh" --registry "$TMP_DIR/registry.toml" --audit-json "$TMP_DIR/audit.json"
  [ "$status" -eq 1 ]
  run rg 'untracked cargo-audit warnings present' <<<"$output"
  [ "$status" -eq 0 ]
}

