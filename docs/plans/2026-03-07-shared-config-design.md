# Shared Config Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Define the approved shared configuration model for `rarag`, `raragd`, and `rarag-mcp` before implementation work begins.
Architecture: Keep defaults in code inside `rarag-core`, then layer an optional shared TOML config file and binary-local overrides on top. Use one shared config resolver so CLI, daemon, and MCP do not drift in path, socket, storage, or embedding semantics.
Tech Stack: Rust 1.93+, edition 2024, `serde`, `toml`, standard filesystem APIs, existing `rarag-core` config types.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design document's configuration and verification contracts.
4. Explicit updates recorded in this file.

## Execution Mode

- Mode: plan-only
- Default: plan-only for this design document.

## Output Contract

- Keep the config model shared across binaries unless a setting is genuinely binary-local.
- Preserve code defaults as the first layer.
- Keep secrets out of checked-in config examples and out of config parsing errors.

## Task Update Contract

- New config requirements must be reflected in the shared model before binary-local handling is added.
- Changes to override precedence must update both the spec and the implementation plan.
- Binary-specific config handling must remain consistent with the shared resolver contract.

## Completion Gate

- The design is complete only when shared config structure, override order, search paths, binary-local sections, and secret-handling rules are explicit.
- Completion also requires doc verification evidence and task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

## Design Summary

### Recommended Option

Use one shared optional TOML file, `rarag.toml`, for all local binaries.

- `rarag-core` owns:
  - config defaults
  - TOML parsing
  - file search
  - layered merge
  - resolved config types
- `rarag`, `raragd`, and `rarag-mcp` own:
  - explicit `--config` handling
  - binary-local final validation
  - CLI flag precedence

This avoids duplicating config semantics across binaries and keeps user-facing setup simple.

### Config Shape

Shared sections:

- `runtime`
- `storage.turso`
- `storage.tantivy`
- `storage.qdrant`
- `embeddings`
- `indexing`
- `retrieval`

Binary-local sections:

- `cli`
- `daemon`
- `mcp`

All sections are optional. When a field is missing, code defaults remain in effect.

### Search and Override Order

Resolver order:

1. compiled defaults
2. explicit `--config <path>`
3. `RARAG_CONFIG`
4. `$XDG_CONFIG_HOME/rarag/rarag.toml`
5. `~/.config/rarag/rarag.toml`
6. documented per-field env overrides
7. CLI flags

The first existing config file wins. Missing config files are non-fatal.

### Security Constraints

- Secrets stay in environment variables, not config files.
- Checked-in examples reference env var names only.
- Errors may name missing env vars, but must never echo secret values.
- Binaries must not require secret env vars until the relevant operation is used, unless explicitly running a readiness check.

### OpenAI-Compatible Embeddings

The shared config must support:

- `base_url`
- `endpoint_path`
- `model`
- `api_key_env`
- `dimensions`

The default OpenAI-compatible shape is:

- `base_url = "https://api.openai.com/v1"`
- `endpoint_path = "/embeddings"`
- `model = "text-embedding-3-small"`

### Binary Behavior

- `rarag` should be able to run entirely on defaults unless a command needs non-default stores or credentials.
- `raragd` should resolve the shared config and expose daemon-local socket/service settings.
- `rarag-mcp` should reuse the same runtime and socket semantics as `raragd`.

### Review Focus

Spec review:

- one canonical local config model
- defaults remain in code
- override precedence is explicit

Security review:

- no inline secrets
- no secret leakage in docs or parse errors

Code quality review:

- no duplicated config search logic across binaries
- shared resolver stays in `rarag-core`

### Task 1: Ratify Shared Config Baseline

**Files:**

- Modify: `docs/specs/repository-rag-architecture.md`
- Create: `docs/plans/2026-03-07-shared-config-design.md`
- Create: `docs/plans/2026-03-07-shared-config-implementation-plan.md`
- Modify: `docs/tasks/tasks.csv`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- The repository RAG architecture spec already exists.
- The user approved TOML as the canonical user-facing config format.

**Invariants**

- Shared config stays optional.
- Defaults stay in code.
- Secrets remain outside checked-in files.

**Postconditions**

- The config architecture is explicit in the canonical spec.
- A dedicated implementation plan exists for execution.
- The task registry records the new design/plan effort.

**Tests (must exist before implementation)**

Unit:
- `doc-lint shared-config design header check`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the new design and plan documents satisfy the strict profile.

**Implementation Steps**

1. Extend the architecture spec with the shared TOML config contract.
2. Record the approved config design and review constraints.
3. Write the execution plan and register the work in `docs/tasks/tasks.csv`.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: all updated design and planning documents pass lint.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/repository-rag-architecture.md`, `docs/plans/2026-03-07-shared-config-design.md`, `docs/plans/2026-03-07-shared-config-implementation-plan.md`, `docs/tasks/tasks.csv`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- Task registry updated
