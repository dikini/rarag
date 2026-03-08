# Service Porcelain Follow-up Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Task Registry ID: `2026-03-08-service-porcelain-followup`

Goal: Close correctness gaps in `rarag service install` so generated user units are portable across install methods and config locations.
Architecture: Keep the existing `rarag service` surface and systemd user model, but change unit generation inputs from hardcoded paths to resolved runtime values. Use explicit integration tests for resolved binary/config path propagation and preserve current managed-file safety semantics.
Tech Stack: Rust 1.93+, edition 2024, existing CLI parser and service orchestration module, current daemon_cli_mcp test harness.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Keep existing `rarag service` command names and flag shape.
- Preserve managed-file overwrite safeguards.
- Fix unit generation inputs without widening runtime surface.

## Task Update Contract

- New instructions must be mapped to this plan before continuing execution.
- If priority conflicts exist, apply Instruction Priority and document the resolution.
- Do not silently drop accepted requirements.

## Completion Gate

- Completion requires path-resolution behavior proven in tests.
- Completion requires updates to docs/task/changelog and passing fast-feedback checks.

## Model Compatibility Notes

- Keep critical constraints in plain language.
- Avoid relying on XML-only delimiters for required behavior.

---

### Task 1: Resolve Binary and Config Paths for Service Install

**Files:**

- Modify: `crates/rarag/src/main.rs`
- Modify: `crates/rarag/src/services.rs`
- Modify: `crates/rarag/src/cli.rs` (if needed for carrying resolved config path metadata)
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`
- Modify: `docs/ops/systemd-user.md`
- Modify: `INSTALL.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**

- Current unit templates hardcode `%h/.cargo/bin/{raragd,rarag-mcp}`.
- Current unit templates hardcode `%h/.config/rarag/rarag.toml`.

**Invariants**

- Existing porcelain command names remain unchanged.
- Install remains idempotent for matching managed units.
- Unmanaged units remain protected from overwrite.

**Postconditions**

- Generated units point to resolved executable paths for installed binaries.
- Generated units use resolved config path when one is explicitly provided or discovered.
- Dry-run output and JSON report surface the resolved unit write behavior.

**Tests (must exist before implementation)**

Unit:
- `services::tests::install_uses_resolved_binary_paths`
- `services::tests::install_uses_resolved_config_path`

Invariant:
- `daemon_cli_mcp::cli_supports_service_install_dry_run`
- `daemon_cli_mcp::cli_rejects_service_reload_for_mcp_target`

Integration:
- `daemon_cli_mcp::cli_supports_service_install_dry_run` with explicit `--config` should include that path in planned unit content/report.

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test daemon_cli_mcp cli_supports_service_install_dry_run -- --nocapture`
Expected: failing assertions once new path-specific expectations are added and before implementation wiring is complete.

**Implementation Steps**

1. Add failing tests for resolved binary/config path propagation into generated units.
2. Thread resolved config source path and executable paths into service install generation.
3. Keep managed-file protection and lifecycle behavior unchanged.
4. Update operator/install docs to remove hardcoded-path assumptions.

**Green Phase (required)**

Command: `cargo test -p rarag -- --nocapture && cargo test -p rarag-core --test daemon_cli_mcp -- --nocapture`
Expected: service porcelain unit/invariant/integration tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag/src/**`, directly related docs
Re-run: `cargo test -p rarag -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- `scripts/check-fast-feedback.sh` passing
- `CHANGELOG.md` and `docs/tasks/tasks.csv` updated
