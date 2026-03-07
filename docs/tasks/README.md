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
