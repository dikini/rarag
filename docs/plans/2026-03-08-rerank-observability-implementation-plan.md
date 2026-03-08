# Rerank Observability Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Add configurable heuristic reranking, opt-in retrieval observations, and safe live config reload without changing the repository-assistance retrieval surface.
Architecture: Keep the current retrieval pipeline and daemon topology. Add shared config sections for rerank and observability, thread resolved config into retrieval and observation sinks, and expose a small admin reload control through the daemon, CLI, MCP, and `SIGHUP`.
Tech Stack: Rust 1.93+, edition 2024, existing `serde`/`toml` config loader, Turso metadata store, `tokio`, Unix sockets, and current `rarag-core` retrieval modules.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless the user explicitly requests plan-only output.

## Output Contract

- Keep task definitions concrete and local to exact files.
- Preserve existing retrieval query contracts except for the reload admin control.
- Keep observability disabled by default and preserve current ranking behavior when no config overrides are present.

## Task Update Contract

- New tuning fields must be added to shared config before retrieval code consumes them.
- Observation fields must be justified by current rerank or neighborhood logic; avoid speculative data capture.
- Reload behavior must remain validate-then-swap and non-disruptive.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are satisfied.
- Plan completion requires verification evidence, changelog updates, and task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Finalize Docs and Registry

**Files:**

- Modify: `docs/specs/repository-rag-architecture.md`
- Create: `docs/plans/2026-03-08-rerank-observability-design.md`
- Create: `docs/plans/2026-03-08-rerank-observability-implementation-plan.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**

- Approved design covers configurable reranking, observation capture, and safe reload.

**Invariants**

- Canonical spec remains the source of truth.
- Task registry stays aligned with active work.

**Postconditions**

- Spec, design, plan, and task registry all describe the same feature.

**Tests (must exist before implementation)**

Unit:
- `doc-lint rerank-observability docs`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the new docs satisfy the strict profile and registry requirements.

**Implementation Steps**

1. Align the architecture spec and design note.
2. Add the implementation plan.
3. Register the task in `docs/tasks/tasks.csv`.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`
Expected: both commands pass for docs-only changes.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/repository-rag-architecture.md`, `docs/plans/2026-03-08-rerank-observability-*.md`, `docs/tasks/tasks.csv`
Re-run: `scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing

### Task 2: Add Shared Config for Rerank, Observability, and Reload

**Files:**

- Modify: `crates/rarag-core/src/config.rs`
- Modify: `crates/rarag-core/src/config_loader.rs`
- Modify: `crates/rarag-core/tests/config_snapshot.rs`
- Modify: `crates/rarag-core/tests/config_loader.rs`
- Modify: `crates/rarag-core/tests/config_binary_entrypoints.rs`

**Preconditions**

- Shared config loader is the only TOML resolution path.

**Invariants**

- Defaults preserve current behavior.
- Missing config files remain non-fatal.
- Observability defaults to off.

**Postconditions**

- Shared config can express rerank weights, neighborhood weights, and observability settings.
- Resolved config exposes reload-safe, cloneable runtime values.

**Tests (must exist before implementation)**

Unit:
- `config_snapshot::builds_default_app_config`
- `config_snapshot::parses_rerank_and_observability_sections`

Invariant:
- `config_loader::missing_config_uses_code_defaults`
- `config_loader::toml_overrides_rerank_and_observability`

Integration:
- `config_binary_entrypoints::example_toml_matches_resolved_shape`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test config_snapshot --test config_loader --test config_binary_entrypoints -- --nocapture`
Expected: fail on the new rerank/observability config expectations.

**Implementation Steps**

1. Add typed config structs and defaults.
2. Extend TOML partial config parsing and merge logic.
3. Update config snapshot and entrypoint tests.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test config_snapshot --test config_loader --test config_binary_entrypoints -- --nocapture`
Expected: all config tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/config.rs`, `crates/rarag-core/src/config_loader.rs`, related config tests
Re-run: `cargo test -p rarag-core --test config_snapshot --test config_loader --test config_binary_entrypoints -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing

### Task 3: Add Safe Daemon Config Reload Controls

**Files:**

- Modify: `crates/rarag-core/src/daemon.rs`
- Modify: `crates/raragd/src/server.rs`
- Modify: `crates/rarag/src/main.rs`
- Modify: `crates/rarag/src/cli.rs`
- Modify: `crates/rarag-mcp/src/tools.rs`
- Modify: `crates/rarag-core/tests/daemon_transport.rs`
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`

**Preconditions**

- Config loading can be rerun from a known config path.

**Invariants**

- Reload is an admin operation, not a retrieval mode.
- Failed reload preserves the active configuration.
- In-flight requests are not invalidated by reload.

**Postconditions**

- Daemon supports `ReloadConfig` requests and `SIGHUP`.
- CLI and MCP expose reload as an admin command/tool.

**Tests (must exist before implementation)**

Unit:
- `daemon_transport::serializes_reload_request`

Invariant:
- `daemon_transport::reload_failure_keeps_old_config`

Integration:
- `daemon_cli_mcp::cli_and_mcp_support_reload_config`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test daemon_transport --test daemon_cli_mcp -- --nocapture`
Expected: fail on missing reload request and admin surface behavior.

**Implementation Steps**

1. Extend daemon request/response types with reload semantics.
2. Add daemon state/config swapping and `SIGHUP` handling.
3. Expose reload through CLI and MCP contracts.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test daemon_transport --test daemon_cli_mcp -- --nocapture`
Expected: all reload transport and contract tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: daemon request/response and reload wiring files only
Re-run: `cargo test -p rarag-core --test daemon_transport --test daemon_cli_mcp -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing

### Task 4: Make Heuristic Reranking Configurable

**Files:**

- Modify: `crates/rarag-core/src/retrieval/rerank.rs`
- Modify: `crates/rarag-core/src/retrieval/neighborhood.rs`
- Modify: `crates/rarag-core/src/retrieval/mod.rs`
- Modify: `crates/rarag-core/tests/retrieval_modes.rs`
- Modify: `crates/rarag-core/tests/semantic_fixture.rs`

**Preconditions**

- Shared config exposes typed rerank and neighborhood weights.

**Invariants**

- Default weights reproduce current ordering behavior.
- Reranking remains deterministic for a fixed config and candidate set.

**Postconditions**

- Rerank and neighborhood scores come from config-backed weights rather than hardcoded constants.
- Tests prove both default compatibility and override behavior.

**Tests (must exist before implementation)**

Unit:
- `retrieval_modes::default_rerank_weights_preserve_symbol_priority`
- `retrieval_modes::override_rerank_weights_change_rank_order`

Invariant:
- `retrieval_modes::results_never_cross_snapshot_boundary`
- `retrieval_modes::caps_neighborhood_size_by_mode`

Integration:
- `semantic_fixture::bounded_refactor_uses_impl_and_test_edges`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test retrieval_modes --test semantic_fixture -- --nocapture`
Expected: fail on the new override-based ranking assertions.

**Implementation Steps**

1. Define score components and weight accessors from config.
2. Thread resolved weights through neighborhood assembly and reranking.
3. Add override-driven ranking tests without changing external query contracts.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test retrieval_modes --test semantic_fixture -- --nocapture`
Expected: all rerank and neighborhood tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/retrieval/**`, related retrieval tests
Re-run: `cargo test -p rarag-core --test retrieval_modes --test semantic_fixture -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing

### Task 5: Add Observation Records and Structured Retrieval Logs

**Files:**

- Modify: `crates/rarag-core/src/metadata/schema.sql`
- Modify: `crates/rarag-core/src/metadata/mod.rs`
- Modify: `crates/rarag-core/src/metadata/store.rs`
- Modify: `crates/rarag-core/src/retrieval/mod.rs`
- Modify: `crates/rarag-core/src/retrieval/rerank.rs`
- Modify: `crates/raragd/src/server.rs`
- Modify: `crates/rarag-core/tests/turso_snapshot_store.rs`
- Modify: `crates/rarag-core/tests/retrieval_modes.rs`

**Preconditions**

- Config and daemon reload plumbing are in place.

**Invariants**

- Observation capture is opt-in.
- Observation capture does not alter retrieval ranking or outputs.
- Existing lightweight query audit remains intact.

**Postconditions**

- Retrieval can emit structured summary and detailed observation logs.
- Metadata store persists query and candidate observations suitable for offline eval generation.

**Tests (must exist before implementation)**

Unit:
- `turso_snapshot_store::records_and_loads_query_observations`

Invariant:
- `retrieval_modes::observation_capture_does_not_change_ranked_results`

Integration:
- `retrieval_modes::detailed_observation_captures_candidate_features`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test turso_snapshot_store --test retrieval_modes -- --nocapture`
Expected: fail on missing observation schema and candidate-feature capture.

**Implementation Steps**

1. Add observation tables and metadata store methods.
2. Capture candidate features and score breakdowns during retrieval.
3. Emit structured daemon logs according to observability verbosity.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test turso_snapshot_store --test retrieval_modes -- --nocapture`
Expected: observation persistence and retrieval tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: metadata schema/store, retrieval observation plumbing, daemon logging
Re-run: `cargo test -p rarag-core --test turso_snapshot_store --test retrieval_modes -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing

### Task 6: Add Evaluation Fixtures, Documentation, and Final Verification

**Files:**

- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `crates/rarag-core/tests/retrieval_modes.rs`
- Modify: `crates/rarag-core/tests/semantic_fixture.rs`
- Modify: any new fixture files required under `crates/rarag-core/tests/fixtures/`

**Preconditions**

- Rerank configuration and observation capture work end-to-end.

**Invariants**

- Documentation describes only implemented behavior.
- Eval fixtures remain deterministic and repository-local.

**Postconditions**

- README documents tuning and observability usage.
- Changelog records the completed feature.
- Fixture-based tests cover useful eval-set generation inputs.

**Tests (must exist before implementation)**

Unit:
- `retrieval_modes::eval_fixture_records_expected_candidate_features`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-fast-feedback.sh`

Integration:
- `cargo test --workspace`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test retrieval_modes --test semantic_fixture -- --nocapture`
Expected: fail on the new eval-fixture assertions.

**Implementation Steps**

1. Add deterministic eval fixtures and assertions around recorded features.
2. Update README, task registry, and changelog.
3. Run full verification, review, and fix any issues found.

**Green Phase (required)**

Command: `cargo test --workspace && scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`
Expected: full workspace tests and policy checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: docs, fixtures, final naming cleanup
Re-run: `cargo test --workspace && scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
