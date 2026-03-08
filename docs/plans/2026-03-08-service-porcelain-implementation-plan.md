# Service Porcelain Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Add a user-friendly `rarag service` porcelain for user-systemd install and lifecycle operations.
Architecture: Extend `rarag` CLI parsing with a service command family and execute those operations via a dedicated local systemd orchestration module. Keep daemon and MCP request contracts unchanged while updating docs and policy artifacts to reflect the new command surface.
Tech Stack: Rust 1.93+, edition 2024, std fs/path/process APIs, existing integration tests.
Template-Profile: tdd-strict-v1

Task id: `2026-03-08-service-porcelain`

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Add `rarag service install|start|stop|restart|reload` command support.
- Keep daemon request behavior unchanged for existing commands.
- Ensure install semantics are idempotent and safe for unmanaged unit files.
- Update operator docs/spec/changelog/task registry for the new surface.

## Task Update Contract

- New instructions must be mapped to this plan before continuing execution.
- If priority conflicts exist, apply Instruction Priority and document the resolution.
- Do not silently drop accepted requirements.

## Completion Gate

- Task complete only when tests and fast-feedback checks pass on current tree state.
- Plan completion requires `CHANGELOG.md` and `docs/tasks/tasks.csv` updates.

## Model Compatibility Notes

- Keep critical constraints in plain language.
- Avoid relying on XML-only delimiters for required behavior.

---

### Task 1: Add Service Porcelain and Policy Updates

**Files:**

- Modify: `crates/rarag/src/cli.rs`
- Modify: `crates/rarag/src/main.rs`
- Create: `crates/rarag/src/services.rs`
- Modify: `crates/rarag/Cargo.toml`
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`
- Modify: `docs/specs/repository-rag-architecture.md`
- Modify: `INSTALL.md`
- Modify: `docs/ops/systemd-user.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**

- `systemctl --user` is the target runtime for service operations.
- Existing daemon/CLI/MCP contracts and tests are available.

**Invariants**

- Existing daemon request and MCP tool contracts remain compatible.
- `service reload` targets daemon HUP only.
- Unmanaged unit files are not overwritten by install.

**Postconditions**

- CLI exposes `service` porcelain with install and lifecycle operations.
- Dry-run output provides script-visible operation commands.
- Docs/spec/changelog/tasks reflect the shipped command surface.

**Tests (must exist before implementation)**

Unit:
- `cli::tests::parses_service_install_force`
- `cli::tests::parses_service_start_default_target_all`
- `cli::tests::rejects_reload_for_mcp_target`

Invariant:
- `daemon_cli_mcp::cli_rejects_service_reload_for_mcp_target`

Integration:
- `daemon_cli_mcp::cli_supports_service_reload_dry_run`
- `daemon_cli_mcp::cli_supports_service_start_all_dry_run`
- `daemon_cli_mcp::cli_supports_service_install_dry_run`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test daemon_cli_mcp service_reload -- --nocapture`
Expected: service porcelain tests fail because `rarag service ...` is not implemented.

**Implementation Steps**

1. Add failing integration tests for service reload/start/install dry-run and reload target validation.
2. Refactor CLI command representation to support daemon and service action branches.
3. Implement service orchestration module for install/start/stop/restart/reload.
4. Add parser unit tests in `rarag`.
5. Update help and documentation/spec/task/changelog artifacts.

**Green Phase (required)**

Command: `cargo test -p rarag -- --nocapture && cargo test -p rarag-core --test daemon_cli_mcp -- --nocapture`
Expected: all listed unit/invariant/integration tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag/src/**`
Re-run: `cargo test -p rarag -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- `scripts/check-fast-feedback.sh` passing
- `CHANGELOG.md` updated
