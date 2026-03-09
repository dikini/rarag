# LanceDB Vector Store Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Qdrant with an in-process local LanceDB vector store while preserving retrieval/search/reranking behavior and removing out-of-process runtime dependencies.

**Architecture:** Introduce a LanceDB-backed vector store in `rarag-core` and replace Qdrant-specific configuration, runtime wiring, and docs. Keep retrieval logic and reranking semantics intact by preserving the same vector hit contract (`snapshot_id`, `chunk_id`, `symbol_path`, `score`) and snapshot-scoped vector search behavior. Backward compatibility is intentionally out of scope; all Qdrant-specific code paths and docs are removed.

**Tech Stack:** Rust 1.93+ (edition 2024), `lancedb`, `arrow-array`, `arrow-schema`, `tokio`, `tantivy`, `turso`, existing retrieval/rerank pipeline.

Template-Profile: tdd-strict-v1
Task Registry ID: `2026-03-09-lancedb-vector-store-migration`

---

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. `docs/specs/repository-rag-architecture.md` and this plan.
4. Task-level updates recorded while executing.

## Execution Mode

- Mode: execute-with-checkpoints
- Compatibility policy: pre-release breaking changes allowed; no compatibility shims.

## Output Contract

- Keep each execution step concrete: exact file paths, commands, and expected outcomes.
- Preserve TDD sequencing: Red before implementation, Green before task completion.
- Record verification evidence for every completion claim.

## Task Update Contract

- Map new requirements to specific tasks before implementation continues.
- If priorities conflict, resolve using Instruction Priority and record the decision in this plan.
- Do not silently drop previously accepted requirements.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and required tests are satisfied.
- Plan completion requires updated `CHANGELOG.md`, updated `docs/tasks/tasks.csv`, and verification evidence.

## Model Compatibility Notes

- XML-style blocks are optional structure aids; enforce constraints in plain language as written.
- Execution agents must treat this plan as canonical unless superseded by higher-priority instructions.

## Scope Guardrails

- Preserve behavior for:
  - indexing vectors per snapshot
  - semantic retrieval candidate generation
  - reranking inputs and evidence emission
- Explicitly out of scope:
  - config migration from `[qdrant]` to new section
  - dual-backend support
  - remote vector DB operation

---

### Task 1: Update Canonical Spec and Task Registry for LanceDB Runtime

**Files:**
- Modify: `docs/specs/repository-rag-architecture.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`

**Preconditions**

- Existing spec names Qdrant as vector store.
- Task registry contains the active repository RAG task lineage.

**Invariants**

- Spec remains explicit about retrieval order and evidence contracts.
- Pre-release non-compatibility policy is respected.

**Postconditions**

- Spec and task registry reflect LanceDB as canonical vector store.
- Changelog includes the migration work item in Common Changelog style.

**Tests (must exist before implementation)**

Unit:
- N/A (doc-only task)

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`

Integration:
- `scripts/check-fast-feedback.sh`

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fails or reports stale references to Qdrant before edits.

**Implementation Steps**

1. Replace Qdrant references in spec sections covering topology, storage contract, retrieval flow, and risk model.
2. Add/update task registry row for LanceDB migration with plan reference.
3. Add changelog entry scoped to vector backend replacement and operational simplification.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: docs checks pass for updated spec/plan/task registry/changelog.

**Completion Evidence**

- Spec updated
- Task registry updated
- Changelog updated
- Doc checks passing

---

### Task 2: Replace Qdrant Configuration with LanceDB Configuration

**Files:**
- Modify: `crates/rarag-core/src/config.rs`
- Modify: `crates/rarag-core/src/config_loader.rs`
- Modify: `crates/rarag-core/tests/config_snapshot.rs`
- Modify: `crates/rarag-core/tests/config_binary_entrypoints.rs`
- Modify: `examples/rarag.example.toml`

**Preconditions**

- Config currently exposes `[qdrant]` endpoint/collection settings.

**Invariants**

- Shared config loading order and override precedence remain unchanged.
- Embedding dimension contract remains mandatory for vector operations.

**Postconditions**

- New config section (for example `[lancedb]`) defines local DB root and vector table name.
- All tests/assertions use LanceDB keys instead of Qdrant keys.

**Tests (must exist before implementation)**

Unit:
- `crates/rarag-core/tests/config_snapshot.rs`

Invariant:
- `crates/rarag-core/tests/config_binary_entrypoints.rs`

Integration:
- `cargo test -p rarag-core config_snapshot -- --nocapture`
- `cargo test -p rarag-core config_binary_entrypoints -- --nocapture`

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core config_snapshot -- --nocapture`
Expected: FAIL after adding test expectations for `[lancedb]` before config implementation changes.

**Implementation Steps**

1. Replace `QdrantConfig` with `LanceDbConfig` and defaults suitable for local file-backed runtime.
2. Update loader override structs/env mappings from `qdrant` to `lancedb`.
3. Update example TOML and config tests to new section/fields.

**Green Phase (required)**

Command: `cargo test -p rarag-core config_snapshot config_binary_entrypoints -- --nocapture`
Expected: PASS with updated snapshots/assertions.

**Completion Evidence**

- Config model compiled and tested
- Example config aligned
- No remaining config tests reference Qdrant keys

---

### Task 3: Introduce LanceDB Vector Store Adapter in `rarag-core`

**Files:**
- Create: `crates/rarag-core/src/indexing/lancedb_store.rs`
- Modify: `crates/rarag-core/src/indexing/mod.rs`
- Modify: `crates/rarag-core/Cargo.toml`
- Modify: `Cargo.lock`
- Delete: `crates/rarag-core/src/indexing/qdrant_store.rs`

**Preconditions**

- Existing `QdrantPointStore` API drives indexing and retrieval.

**Invariants**

- Public hit contract remains functionally equivalent (`snapshot_id`, `chunk_id`, `symbol_path`, score order descending).
- `replace_snapshot` remains idempotent for same snapshot input.

**Postconditions**

- `LanceDbPointStore` (or renamed `VectorPointStore`) provides:
  - `new(...)`
  - `replace_snapshot(...)`
  - `search_snapshot(...)`
  - `point_count(...)`
- Adapter is fully in-process and local-path based.

**Tests (must exist before implementation)**

Unit:
- New unit tests in `crates/rarag-core/src/indexing/lancedb_store.rs` for:
  - dimension mismatch errors
  - snapshot replacement semantics
  - score sorting and limit behavior

Invariant:
- `crates/rarag-core/tests/index_pipeline.rs::metadata_lexical_and_vector_counts_match`

Integration:
- `cargo test -p rarag-core index_pipeline -- --nocapture`

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core index_pipeline -- --nocapture`
Expected: FAIL after changing test imports/constructors to LanceDB store name before implementation exists.

**Implementation Steps**

1. Add `lancedb` and required Arrow deps to `rarag-core`.
2. Implement table schema with vector + payload fields (`chunk_id`, `snapshot_id`, `symbol_path`, vector).
3. Implement snapshot replacement by deleting snapshot rows then bulk-inserting current chunk vectors.
4. Implement snapshot-filtered nearest-neighbor search and map results into existing hit struct.
5. Export new store from `indexing/mod.rs` and remove Qdrant module.

**Green Phase (required)**

Command: `cargo test -p rarag-core index_pipeline retrieval_modes semantic_fixture -- --nocapture`
Expected: PASS with identical retrieval behavior assertions.

**Completion Evidence**

- Vector adapter replaced in core
- No `qdrant-client` dependency in `rarag-core`
- Indexing/retrieval test suites pass

---

### Task 4: Rewire Daemon and Retrieval Integration to LanceDB Store

**Files:**
- Modify: `crates/raragd/src/server.rs`
- Modify: `crates/rarag-core/src/retrieval/mod.rs`
- Modify: `crates/rarag-core/tests/retrieval_modes.rs`
- Modify: `crates/rarag-core/tests/semantic_fixture.rs`
- Modify: `crates/rarag-core/tests/daemon_transport.rs`

**Preconditions**

- Daemon currently constructs `QdrantPointStore` using endpoint/collection config.

**Invariants**

- Request/response contracts for daemon and MCP remain unchanged.
- Reranking controls and observation behavior remain unchanged.

**Postconditions**

- Daemon constructs LanceDB store with local path/table settings.
- Retrieval still emits semantic candidate evidence and warnings semantics as before.

**Tests (must exist before implementation)**

Unit:
- `crates/rarag-core/tests/retrieval_modes.rs`

Invariant:
- `crates/rarag-core/tests/semantic_fixture.rs`

Integration:
- `crates/rarag-core/tests/daemon_transport.rs`
- `crates/rarag-core/tests/mcp_protocol.rs`

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core retrieval_modes semantic_fixture daemon_transport -- --nocapture`
Expected: FAIL after renaming store/config references in tests before server/retrieval updates.

**Implementation Steps**

1. Replace daemon store field/type usage from Qdrant to LanceDB store.
2. Update retrieval wiring and error text to vector-store-generic wording where needed.
3. Update daemon transport tests for new config keys and expected startup/runtime behavior.

**Green Phase (required)**

Command: `cargo test -p rarag-core retrieval_modes semantic_fixture daemon_transport mcp_protocol -- --nocapture`
Expected: PASS with equivalent behavior coverage.

**Completion Evidence**

- Daemon and retrieval use LanceDB store
- Contract tests pass
- No Qdrant-specific runtime assumptions remain

---

### Task 5: Remove Qdrant Operational Surface and Update Runtime Tooling

**Files:**
- Delete: `docs/ops/qdrant-runtime.md`
- Create: `docs/ops/lancedb-runtime.md`
- Modify: `README.md`
- Modify: `INSTALL.md`
- Modify: `docs/ops/systemd-user.md`
- Modify: `scripts/check-live-rag-stack.sh`
- Modify: `crates/rarag-core/Cargo.toml` (remove `qdrant-client`)
- Modify: `deny.toml`
- Modify: `audit.toml`

**Preconditions**

- Runtime docs/scripts assume external Qdrant endpoint.
- Security ignore comments include Qdrant transitive advisories.

**Invariants**

- Live check still validates real embeddings path and end-to-end retrieval.
- Security policy files stay internally consistent with active dependency tree.

**Postconditions**

- Runtime guidance describes local LanceDB persistence and troubleshooting.
- Live check no longer depends on externally running Qdrant.
- Dependency/security config no longer references removed Qdrant stack.

**Tests (must exist before implementation)**

Unit:
- N/A (docs/scripts/deps)

Invariant:
- `scripts/check-fast-feedback.sh`

Integration:
- `scripts/check-live-rag-stack.sh` (in a configured local environment)

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fails after docs/script expectation updates until code/deps are aligned.

**Implementation Steps**

1. Remove Qdrant install/runtime instructions and add LanceDB local runtime docs.
2. Update README/INSTALL/systemd guidance to remove external vector DB dependency.
3. Update live stack script to assert LanceDB-backed runtime path conditions.
4. Remove `qdrant-client` and clean advisory-ignore comments tied to that stack.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: PASS for docs/policy/script checks.

**Completion Evidence**

- Operational docs are LanceDB-first
- Scripted checks no longer require Qdrant
- Dependency audit policy references are current

---

### Task 6: Full Verification and Branch Closure Readiness

**Files:**
- Modify: `CHANGELOG.md` (finalized entry if needed)
- Modify: `docs/tasks/tasks.csv` (status/evidence updates)

**Preconditions**

- All migration tasks implemented.

**Invariants**

- Verification evidence is command-backed.
- No success claims without passing outputs.

**Postconditions**

- End-to-end checks pass and task records are complete.

**Tests (must exist before implementation)**

Unit:
- `cargo test -p rarag-core --lib --tests`

Invariant:
- `scripts/check-fast-feedback.sh`

Integration:
- `cargo test --workspace`
- `scripts/check-live-rag-stack.sh` (when live env vars are configured)

**Red Phase (required before code changes)**

Command: `cargo test --workspace`
Expected: baseline failures acceptable before completing all migration tasks.

**Implementation Steps**

1. Run full workspace test suite and capture outputs.
2. Run fast-feedback and doc lint checks on final tree.
3. Update changelog/task registry completion evidence with exact commands used.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh && cargo test --workspace`
Expected: PASS; if live checks cannot run, document why and residual risk explicitly.

**Completion Evidence**

- Verification commands and outcomes recorded
- Changelog/task registry finalized
- Residual risks documented (if any)

---

## Risks and Mitigations

- Risk: LanceDB filtering/query semantics differ from Qdrant snapshot filtering.
  - Mitigation: dedicated red/green tests for snapshot isolation and hit mapping.
- Risk: ranking drift due to distance score interpretation differences.
  - Mitigation: preserve ranking assertions in retrieval mode tests and add explicit score-order checks.
- Risk: dependency/build complexity increases with Arrow/Lance stack.
  - Mitigation: keep features minimal and verify CI build/test wall time impact early.

## Verification Checklist (Execution-Time)

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-fast-feedback.sh`
- `cargo test -p rarag-core config_snapshot config_binary_entrypoints -- --nocapture`
- `cargo test -p rarag-core index_pipeline retrieval_modes semantic_fixture daemon_transport mcp_protocol -- --nocapture`
- `cargo test --workspace`
- `scripts/check-live-rag-stack.sh` (optional when live env configured)
