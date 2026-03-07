# Shared Config Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Add shared TOML config loading for `rarag`, `raragd`, and `rarag-mcp` while preserving code defaults and explicit override precedence.
Architecture: `rarag-core` owns config defaults, TOML parsing, path resolution, and layered merging. Binaries consume the shared resolved config and apply binary-local CLI overrides without reimplementing config semantics.
Tech Stack: Rust 1.93+, edition 2024, `serde`, `toml`, standard filesystem APIs, existing `rarag-core` config types.
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

- Keep task definitions concrete: exact files, commands, and expected outcomes.
- Use Red/Green checkpoints as hard gates before claiming task completion.
- Record unresolved risks instead of silently skipping checks.

## Task Update Contract

- New instructions must be mapped to affected tasks before continuing execution.
- If priority conflicts exist, apply Instruction Priority and document the resolution.
- Do not silently drop prior accepted requirements.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are all satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints must be restated in plain language for model-robust adherence.

### Task 1: Extend Shared Config Types and Defaults

**Files:**

- Modify: `crates/rarag-core/src/config.rs`
- Modify: `crates/rarag-core/tests/config_snapshot.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/config_snapshot.rs`

**Preconditions**

- The current config types compile and are used by existing tests.

**Invariants**

- Defaults remain defined in code.
- Embedding provider settings remain explicit and serializable.
- New binary-specific config sections are optional.

**Postconditions**

- Shared config types cover `cli`, `daemon`, and `mcp`.
- A default-resolved config can be produced with no config file.

**Tests (must exist before implementation)**

Unit:
- `config_snapshot::builds_default_app_config`
- `config_snapshot::binary_sections_are_optional`

Invariant:
- `config_snapshot::defaults_preserve_openai_embedding_endpoint_shape`

Integration:
- `config_snapshot::snapshot_key_roundtrips_to_json`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test config_snapshot -- --nocapture`
Expected: failing tests for this task only because defaults and binary-specific config sections do not exist yet.

**Implementation Steps**

1. Add optional `cli`, `daemon`, and `mcp` config sections.
2. Add code-defined defaults and a helper to build resolved config.
3. Update config tests and changelog.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test config_snapshot -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/config.rs`, `crates/rarag-core/tests/config_snapshot.rs`
Re-run: `cargo test -p rarag-core --test config_snapshot -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Add TOML File Parsing and Config Search Resolution

**Files:**

- Modify: `crates/rarag-core/Cargo.toml`
- Create: `crates/rarag-core/src/config_loader.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Create: `crates/rarag-core/tests/config_loader.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/config_loader.rs`

**Preconditions**

- Shared config types and defaults exist.

**Invariants**

- Missing config files are non-fatal.
- Search order is deterministic.
- No secret values are required in config files.

**Postconditions**

- The core crate can discover, parse, and merge `rarag.toml`.
- Explicit path, env path, and XDG/default paths are supported.

**Tests (must exist before implementation)**

Unit:
- `config_loader::prefers_explicit_config_path`
- `config_loader::falls_back_to_xdg_config_path`

Invariant:
- `config_loader::missing_config_uses_code_defaults`

Integration:
- `config_loader::toml_overrides_default_embedding_endpoint`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test config_loader -- --nocapture`
Expected: failing tests for this task only because the config loader module does not exist yet.

**Implementation Steps**

1. Add `toml` dependency and a config loader module.
2. Implement config path discovery and TOML deserialization.
3. Merge parsed config onto code defaults.
4. Update exports and changelog.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test config_loader -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/config_loader.rs`, `crates/rarag-core/tests/config_loader.rs`
Re-run: `cargo test -p rarag-core --test config_loader -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Wire Shared Config Consumption into Binaries and Examples

**Files:**

- Modify: `crates/rarag/src/main.rs`
- Modify: `crates/raragd/src/main.rs`
- Modify: `crates/rarag-mcp/src/main.rs`
- Modify: `README.md`
- Modify: `examples/rarag.openai.example.json`
- Create: `examples/rarag.example.toml`
- Create: `crates/rarag-core/tests/config_binary_entrypoints.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/config_binary_entrypoints.rs`

**Preconditions**

- Shared config loading exists in `rarag-core`.

**Invariants**

- Binaries keep code defaults when no config file exists.
- Config overrides are consistent across binaries.
- Example config contains no secrets.

**Postconditions**

- Each binary can resolve shared config.
- The checked-in example uses TOML and documents override behavior.

**Tests (must exist before implementation)**

Unit:
- `config_binary_entrypoints::cli_uses_default_config_without_file`
- `config_binary_entrypoints::daemon_accepts_explicit_config_path`

Invariant:
- `config_binary_entrypoints::mcp_and_daemon_share_socket_override_semantics`

Integration:
- `config_binary_entrypoints::example_toml_matches_resolved_shape`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test config_binary_entrypoints -- --nocapture`
Expected: failing tests for this task only because binary config consumption and TOML example files do not exist yet.

**Implementation Steps**

1. Add minimal binary config entrypoint helpers.
2. Add TOML example config and README guidance.
3. Verify consistent socket/path override behavior.
4. Update changelog.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test config_binary_entrypoints -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag/src/main.rs`, `crates/raragd/src/main.rs`, `crates/rarag-mcp/src/main.rs`, `examples/**`
Re-run: `cargo test -p rarag-core --test config_binary_entrypoints -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
