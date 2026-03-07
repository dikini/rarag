# Repository RAG Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Build a simple Rust-first repository assistance RAG with hybrid retrieval, worktree-aware snapshots, and local CLI plus MCP access over Unix sockets.
Architecture: A single Rust workspace contains `rarag-core`, `raragd`, `rarag`, and `rarag-mcp`. `rarag-core` owns chunking, indexing, retrieval, and workflow-aware neighborhood assembly; the daemon owns warm state and a Unix-socket API; CLI and MCP are thin clients.
Tech Stack: Rust 1.93+, edition 2024, `turso`, `tantivy`, `qdrant-client`, `ra_ap_syntax`, `ra_ap_ide`, `tokio`, `serde`, `clap`, `tracing`, Unix domain sockets, OpenAI-compatible embeddings over HTTP.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: plan-only
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

## Prerequisites

### User-Run System Package Setup

The agent cannot run `sudo` on this system. If these tools are missing, the user must install them.

Recommended batch for Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler pkg-config libssl-dev
```

Verify after install:

```bash
protoc --version
pkg-config --version
```

### Rust Toolchain Setup

These commands do not require `sudo` and can be run by the user or the agent during implementation:

```bash
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
cargo install cargo-nextest --locked
```

## Phase 1: Workspace Foundation

### Task 1: Create the Rust Workspace Skeleton

**Files:**

- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.cargo/config.toml`
- Create: `crates/rarag-core/Cargo.toml`
- Create: `crates/rarag-core/src/lib.rs`
- Create: `crates/rarag-core/tests/workspace_bootstrap.rs`
- Create: `crates/raragd/Cargo.toml`
- Create: `crates/raragd/src/main.rs`
- Create: `crates/rarag/Cargo.toml`
- Create: `crates/rarag/src/main.rs`
- Create: `crates/rarag-mcp/Cargo.toml`
- Create: `crates/rarag-mcp/src/main.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/workspace_bootstrap.rs`

**Preconditions**

- Rust toolchain is installed.
- Required system packages are present if Qdrant or gRPC-related dependencies need `protoc`.

**Invariants**

- All crates use edition `2024`.
- The workspace builds on Rust `1.93+`.
- Binary crates remain thin wrappers; core logic stays in `rarag-core`.

**Postconditions**

- The repository has a compiling Rust workspace with four crates.
- The workspace has baseline linting and build configuration.

**Tests (must exist before implementation)**

Unit:
- `config::tests::workspace_defaults_parse`

Invariant:
- `workspace_bootstrap::all_workspace_members_build`

Integration:
- `workspace_bootstrap::binaries_start_with_help`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test workspace_bootstrap workspace_bootstrap::all_workspace_members_build -- --exact`
Expected: failing tests for this task only because the workspace and test target do not exist yet.

**Implementation Steps**

1. Add the root workspace manifest, toolchain file, and crate manifests.
2. Add minimal `main.rs` and `lib.rs` stubs plus the first workspace bootstrap test.
3. Add baseline shared dependencies only where required; keep binaries thin.
4. Update `CHANGELOG.md` for the new workspace foundation milestone.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test workspace_bootstrap -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `crates/rarag-*`
Re-run: `cargo test -p rarag-core --test workspace_bootstrap -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Add Config and Snapshot Key Types

**Files:**

- Create: `crates/rarag-core/src/config.rs`
- Create: `crates/rarag-core/src/snapshot.rs`
- Create: `crates/rarag-core/tests/config_snapshot.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/config_snapshot.rs`

**Preconditions**

- Workspace exists and builds.

**Invariants**

- Snapshot keys include repo root, worktree root, git SHA, cargo target, feature set, and cfg profile.
- Runtime paths and embedding config are explicit and serializable.

**Postconditions**

- Config parsing and snapshot identity types exist in `rarag-core`.
- Snapshot ids are deterministic from normalized inputs.

**Tests (must exist before implementation)**

Unit:
- `config_snapshot::parses_runtime_paths`
- `config_snapshot::rejects_incomplete_embedding_config`

Invariant:
- `config_snapshot::snapshot_identity_changes_when_worktree_changes`

Integration:
- `config_snapshot::snapshot_key_roundtrips_to_json`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test config_snapshot -- --nocapture`
Expected: failing tests for this task only because config and snapshot types are not implemented yet.

**Implementation Steps**

1. Define config structs for runtime paths, Turso, Tantivy, Qdrant, and embedding provider settings.
2. Define normalized snapshot-key structs and deterministic id helpers.
3. Export the new modules from `rarag-core`.
4. Update `CHANGELOG.md` with snapshot/config coverage.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test config_snapshot -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/config.rs`, `crates/rarag-core/src/snapshot.rs`, `crates/rarag-core/src/lib.rs`
Re-run: `cargo test -p rarag-core --test config_snapshot -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

## Phase 2: Metadata and Structural Indexing

### Task 3: Implement the Turso Metadata Schema and Store

**Files:**

- Create: `crates/rarag-core/src/metadata/mod.rs`
- Create: `crates/rarag-core/src/metadata/schema.sql`
- Create: `crates/rarag-core/src/metadata/store.rs`
- Create: `crates/rarag-core/tests/turso_snapshot_store.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/turso_snapshot_store.rs`

**Preconditions**

- Config and snapshot types exist.
- Turso dependency is available in the workspace.

**Invariants**

- Metadata schema is append-friendly.
- Snapshot rows are immutable after creation.
- Chunk and edge rows reference valid snapshot ids.

**Postconditions**

- Turso-backed schema creation and snapshot CRUD APIs exist.
- Indexing runs and query-audit records have concrete schema definitions.

**Tests (must exist before implementation)**

Unit:
- `turso_snapshot_store::normalizes_feature_sets_before_insert`

Invariant:
- `turso_snapshot_store::same_build_world_reuses_snapshot_identity`

Integration:
- `turso_snapshot_store::create_and_load_snapshot`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test turso_snapshot_store -- --nocapture`
Expected: failing tests for this task only because the Turso schema and store do not exist yet.

**Implementation Steps**

1. Write the metadata schema SQL and embed it in the core crate.
2. Implement snapshot creation, lookup, and indexing-run recording.
3. Add the Turso-backed integration test using a local database.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test turso_snapshot_store -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/metadata/**`
Re-run: `cargo test -p rarag-core --test turso_snapshot_store -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 4: Build the `ra_ap_syntax` Structural Chunker

**Files:**

- Create: `crates/rarag-core/src/chunking/mod.rs`
- Create: `crates/rarag-core/src/chunking/types.rs`
- Create: `crates/rarag-core/src/chunking/rust.rs`
- Create: `crates/rarag-core/tests/chunker_fixture.rs`
- Create: `tests/fixtures/mini_repo/Cargo.toml`
- Create: `tests/fixtures/mini_repo/src/lib.rs`
- Create: `tests/fixtures/mini_repo/src/nested.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/chunker_fixture.rs`

**Preconditions**

- Snapshot metadata is available.
- Fixture repository files exist for chunker tests.

**Invariants**

- Source spans map back to original text exactly.
- Body-region splits preserve owning symbol headers.
- Test and example chunks are emitted as first-class chunks.

**Postconditions**

- Structural chunking exists for crate, module, symbol, body-region, and test/example chunks.
- Chunk output includes canonical symbol path, source span, and parent relationships.

**Tests (must exist before implementation)**

Unit:
- `chunker_fixture::extracts_symbol_chunks_from_fixture`
- `chunker_fixture::preserves_symbol_header_on_body_split`

Invariant:
- `chunker_fixture::span_text_matches_source_slice`

Integration:
- `chunker_fixture::indexes_fixture_workspace_structurally`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test chunker_fixture -- --nocapture`
Expected: failing tests for this task only because chunker modules do not exist yet.

**Implementation Steps**

1. Define chunk and span types.
2. Implement Rust structural chunk extraction with `ra_ap_syntax`.
3. Add fixture-repo tests covering nested modules and oversized bodies.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test chunker_fixture -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/chunking/**`, `tests/fixtures/mini_repo/**`
Re-run: `cargo test -p rarag-core --test chunker_fixture -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

## Phase 3: Hybrid Indexing and Retrieval

### Task 5: Add Tantivy, Qdrant, and Real Embeddings Ingestion

**Files:**

- Create: `crates/rarag-core/src/indexing/mod.rs`
- Create: `crates/rarag-core/src/indexing/tantivy_store.rs`
- Create: `crates/rarag-core/src/indexing/qdrant_store.rs`
- Create: `crates/rarag-core/src/embeddings.rs`
- Create: `crates/rarag-core/tests/index_pipeline.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/index_pipeline.rs`

**Preconditions**

- Chunking output exists.
- Turso metadata store exists.
- Embedding provider configuration is available.

**Invariants**

- Metadata, lexical, and vector ids stay consistent per snapshot.
- Embedding failures do not mark indexing runs as complete.
- The system uses a real embedding provider.

**Postconditions**

- Chunk metadata, Tantivy docs, and Qdrant vectors can be ingested together.
- Query embeddings can be produced with the same configured provider.

**Tests (must exist before implementation)**

Unit:
- `index_pipeline::maps_chunk_to_tantivy_document`
- `index_pipeline::builds_openai_compatible_embedding_request`

Invariant:
- `index_pipeline::metadata_lexical_and_vector_counts_match`

Integration:
- `index_pipeline::reindexes_fixture_repository`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test index_pipeline -- --nocapture`
Expected: failing tests for this task only because indexing adapters and embedding provider do not exist yet.

**Implementation Steps**

1. Add Tantivy and Qdrant adapters with stable `chunk_id` mapping.
2. Implement an OpenAI-compatible embedding client behind a trait.
3. Wire indexing-run state through Turso metadata.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test index_pipeline -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/indexing/**`, `crates/rarag-core/src/embeddings.rs`
Re-run: `cargo test -p rarag-core --test index_pipeline -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 6: Implement Query Modes and Neighborhood Assembly

**Files:**

- Create: `crates/rarag-core/src/retrieval/mod.rs`
- Create: `crates/rarag-core/src/retrieval/query.rs`
- Create: `crates/rarag-core/src/retrieval/rerank.rs`
- Create: `crates/rarag-core/src/retrieval/neighborhood.rs`
- Create: `crates/rarag-core/tests/retrieval_modes.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/retrieval_modes.rs`

**Preconditions**

- Hybrid stores are queryable.
- Chunk edges and workflow enums exist.

**Invariants**

- Neighborhood expansion is bounded by mode.
- Exact symbol matches outrank approximate vector matches for the same snapshot.
- Workflow phase affects reranking and context assembly.

**Postconditions**

- Retrieval supports all required query modes.
- Responses include ranking evidence and missing-data warnings.

**Tests (must exist before implementation)**

Unit:
- `retrieval_modes::prioritizes_exact_symbol_match`
- `retrieval_modes::caps_neighborhood_size_by_mode`

Invariant:
- `retrieval_modes::results_never_cross_snapshot_boundary`

Integration:
- `retrieval_modes::bounded_refactor_returns_tests_and_references`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test retrieval_modes -- --nocapture`
Expected: failing tests for this task only because retrieval modules do not exist yet.

**Implementation Steps**

1. Define workflow phase and query mode enums.
2. Implement hybrid candidate search, bounded graph expansion, and reranking.
3. Implement neighborhood assembly payloads with ranking evidence.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test retrieval_modes -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/retrieval/**`
Re-run: `cargo test -p rarag-core --test retrieval_modes -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

## Phase 4: Semantic Enrichment and Worktree Awareness

### Task 7: Add Rust Analyzer Enrichment and Worktree Diff Biasing

**Files:**

- Create: `crates/rarag-core/src/semantic/mod.rs`
- Create: `crates/rarag-core/src/semantic/rust_analyzer.rs`
- Create: `crates/rarag-core/src/worktree.rs`
- Create: `crates/rarag-core/tests/semantic_fixture.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/semantic_fixture.rs`

**Preconditions**

- Structural retrieval works without semantic enrichment.
- Fixture repository is available for analyzer-backed tests.

**Invariants**

- Enrichment is additive and optional.
- Worktree-local diff biasing never escapes snapshot boundaries.
- Missing analyzer data is surfaced explicitly.

**Postconditions**

- Definitions, references, implementations, type hints, and test links can enrich chunks.
- Reranking can incorporate worktree-local diff signals.

**Tests (must exist before implementation)**

Unit:
- `semantic_fixture::maps_reference_results_to_chunk_edges`
- `semantic_fixture::falls_back_when_analysis_unavailable`

Invariant:
- `semantic_fixture::enrichment_never_rewrites_chunk_source_spans`

Integration:
- `semantic_fixture::bounded_refactor_uses_impl_and_test_edges`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test semantic_fixture -- --nocapture`
Expected: failing tests for this task only because semantic enrichment and diff biasing do not exist yet.

**Implementation Steps**

1. Add a semantic-enrichment facade around `rust-analyzer` APIs.
2. Map analyzer outputs into snapshot-scoped edges.
3. Add worktree diff helpers and rerank bias inputs.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test semantic_fixture -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/semantic/**`, `crates/rarag-core/src/worktree.rs`
Re-run: `cargo test -p rarag-core --test semantic_fixture -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

## Phase 5: Clients and End-to-End Workflow Support

### Task 8: Implement the Unix-Socket Daemon API

**Files:**

- Create: `crates/raragd/src/config.rs`
- Create: `crates/raragd/src/server.rs`
- Create: `crates/raragd/src/transport.rs`
- Modify: `crates/raragd/src/main.rs`
- Create: `crates/rarag-core/tests/daemon_transport.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/daemon_transport.rs`

**Preconditions**

- Core retrieval payloads are stable.
- Runtime path config exists.

**Invariants**

- The daemon API is snapshot-aware.
- Requests and responses are serializable over Unix sockets.
- The daemon remains the single owner of open indexes.

**Postconditions**

- A local daemon can serve index, query, and blast-radius requests over a Unix socket.
- The daemon surfaces warnings for stale or partial enrichment data.

**Tests (must exist before implementation)**

Unit:
- `daemon_transport::serializes_unix_socket_requests`

Invariant:
- `daemon_transport::requests_require_snapshot_or_unambiguous_worktree`

Integration:
- `daemon_transport::daemon_roundtrip_serves_query_payload`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test daemon_transport -- --nocapture`
Expected: failing tests for this task only because daemon transport types and server glue do not exist yet.

**Implementation Steps**

1. Define daemon request and response types.
2. Implement the Unix-socket transport and server loop.
3. Wire core retrieval operations into daemon handlers.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test daemon_transport -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/raragd/src/**`, `crates/rarag-core/tests/daemon_transport.rs`
Re-run: `cargo test -p rarag-core --test daemon_transport -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 9: Deliver the CLI and MCP Clients

**Files:**

- Create: `crates/rarag/src/cli.rs`
- Create: `crates/rarag/src/client.rs`
- Modify: `crates/rarag/src/main.rs`
- Create: `crates/rarag-mcp/src/server.rs`
- Create: `crates/rarag-mcp/src/tools.rs`
- Modify: `crates/rarag-mcp/src/main.rs`
- Create: `crates/rarag-core/tests/daemon_cli_mcp.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/daemon_cli_mcp.rs`

**Preconditions**

- Daemon API exists.
- Core retrieval payloads are stable.

**Invariants**

- CLI and MCP use the same daemon contract.
- CLI remains shell-script friendly with `--json`.
- MCP tools expose repository-assistance semantics, not generic chat endpoints.

**Postconditions**

- The CLI can build indexes, inspect status, query context, retrieve examples, and compute blast radius.
- The MCP server can expose equivalent tools over a Unix socket.

**Tests (must exist before implementation)**

Unit:
- `daemon_cli_mcp::cli_parses_phase_and_mode_flags`
- `daemon_cli_mcp::mcp_tool_names_match_contract`

Invariant:
- `daemon_cli_mcp::cli_and_mcp_observe_same_snapshot_result`

Integration:
- `daemon_cli_mcp::cli_and_mcp_roundtrip_against_local_daemon`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test daemon_cli_mcp -- --nocapture`
Expected: failing tests for this task only because CLI and MCP clients do not exist yet.

**Implementation Steps**

1. Add CLI argument parsing and daemon client wrappers.
2. Add MCP tool handlers that map directly to daemon requests.
3. Add end-to-end tests using a local daemon fixture.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test daemon_cli_mcp -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag/src/**`, `crates/rarag-mcp/src/**`
Re-run: `cargo test -p rarag-core --test daemon_cli_mcp -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 10: Add Review/Fix Loop Guardrails and End-to-End Verification

**Files:**

- Create: `crates/rarag-core/src/workflow.rs`
- Create: `crates/rarag-core/tests/workflow_review_loop.rs`
- Modify: `crates/rarag-core/src/lib.rs`
- Modify: `CHANGELOG.md`
- Test: `crates/rarag-core/tests/workflow_review_loop.rs`

**Preconditions**

- Query modes and client surfaces are complete enough to support review flows.

**Invariants**

- Review/fix loops stop after three iterations by default.
- Verification evidence is attached before a loop iteration is considered successful.
- Unresolved risks are returned explicitly when the loop stops early.

**Postconditions**

- Workflow helper types exist for spec/plan/tests/code/verify/review/fix phases.
- Review/fix iteration accounting is enforced consistently across CLI and MCP paths.

**Tests (must exist before implementation)**

Unit:
- `workflow_review_loop::stops_after_three_iterations`
- `workflow_review_loop::requires_verification_evidence_for_success`

Invariant:
- `workflow_review_loop::phase_ordering_rejects_review_before_verify`

Integration:
- `workflow_review_loop::bounded_refactor_review_cycle_returns_unresolved_risk_after_limit`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test workflow_review_loop -- --nocapture`
Expected: failing tests for this task only because workflow guardrails do not exist yet.

**Implementation Steps**

1. Add workflow phase and review-loop helper types.
2. Enforce iteration limits and verification evidence checks.
3. Add end-to-end workflow tests that exercise the bounded refactor path.
4. Update `CHANGELOG.md`.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test workflow_review_loop -- --nocapture`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/workflow.rs`, `crates/rarag-core/tests/workflow_review_loop.rs`
Re-run: `cargo test -p rarag-core --test workflow_review_loop -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
