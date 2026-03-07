# Repository RAG Architecture

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-07
Status: active
Owner: Codex
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This spec's task contracts and invariants.
4. In-task updates recorded explicitly in this document.

## Output Contract

- Preserve exact section headings in this template unless intentionally revised.
- Keep claims concrete and tied to observable evidence.
- Avoid introducing unstated requirements or hidden assumptions.

## Evidence / Verification Contract

- Every completion claim must cite verification commands/results in `## Verification`.
- Conflicting evidence must be called out explicitly before task closure.
- If verification cannot run, record why and the residual risk.

## Model Compatibility Notes

- XML-style delimiter blocks (e.g. `<context>`, `<constraints>`) are optional structure aids.
- Critical constraints must also be restated in plain language.
- This fallback is required for cross-model robustness (including GPT-5.3 behavior).

## Purpose

Task Registry ID: `2026-03-07-repository-rag-design`

Define the canonical architecture for a Rust-first repository assistance RAG system that supports agents working on this project through a strict development workflow: `spec -> plan -> tests -> code -> verify -> review -> fix`, with at most three review/fix iterations before escalation.

The system is not a generic question-answering assistant. It is a repository memory and retrieval system for understanding unfamiliar code, adding or modifying features, performing bounded refactors safely, and locating examples, invariants, and blast radius.

## Scope

### In Scope

- Rust implementation targeting toolchain `1.93+` and edition `2024`.
- Hybrid retrieval using `ra_ap_syntax` chunk spans, `rust-analyzer` semantic enrichment when available, BM25 via Tantivy, metadata in Turso, and vector storage in Qdrant.
- Snapshot-aware indexing keyed by repository, git worktree, commit SHA, target triple, feature set, and cfg profile.
- Local developer use through a CLI and a local MCP server over Unix sockets.
- Retrieval modes tuned for workflow phases and repository-assistance tasks.
- Test-first development, verification evidence, and review/fix loops.

### Out of Scope

- Generic chatbot or open-domain Q&A behavior.
- Remote multi-tenant hosting or cloud control planes.
- Automatic code writing without explicit client-side orchestration.
- Non-Rust indexing in the MVP.
- Full reliance on LSP as the primary index source.

## Core Terms

- `snapshot`: An immutable indexable build world identified by `(repo_root, worktree_root, git_sha, cargo_target, feature_set, cfg_profile)`.
- `chunk`: A retrievable unit of source-derived content with stable source span metadata.
- `symbol chunk`: A chunk representing a repository-level symbol such as a function, method, trait, impl block, struct, enum, constant, macro, or test.
- `body region`: A subordinate chunk created when a symbol body exceeds size limits; it must retain its owning symbol header and symbol id.
- `edge`: A typed relationship between chunks or symbols such as `contains`, `references`, `implements`, or `tested_by`.
- `neighborhood`: A bounded set of chunks assembled around one or more target symbols for a specific retrieval request.
- `workflow phase`: One of `spec`, `plan`, `tests`, `code`, `verify`, `review`, or `fix` used to bias retrieval and reranking.
- `query mode`: One of `understand-symbol`, `implement-adjacent`, `bounded-refactor`, `find-examples`, or `blast-radius`.
- `daemon`: The long-lived local process that owns open indexes, background indexing, and the Unix-socket service surface.
- `worktree lineage`: The set of snapshots derived from a single git worktree over time.

## Interfaces / Contracts

### Component Topology

The architecture consists of four Rust crates in one workspace:

- `rarag-core`: shared library for config, snapshot model, chunking, storage adapters, retrieval, reranking, and neighborhood assembly.
- `raragd`: local daemon process that owns Tantivy indexes, Turso connections, Qdrant clients, file watching, and the internal Unix-socket API.
- `rarag`: CLI wrapper around daemon requests, with stable shell and JSON output modes.
- `rarag-mcp`: local MCP server over Unix sockets that exposes repository-assistance tools backed by the daemon.

### Storage Contract

- Turso stores snapshot metadata, chunk metadata, graph edges, indexing runs, query audit rows, and provider configuration metadata.
- Tantivy stores lexical fields for chunk text, symbol path, names, docs, signatures, file path, test markers, and workflow hints.
- Qdrant stores chunk vectors and optional reranking helper payloads keyed by `chunk_id` and `snapshot_id`.
- All three stores must use the same stable `chunk_id` and `snapshot_id` values.

### Embedding Provider Contract

- The MVP must use a real embedding provider, not a fake or no-op implementation.
- The first concrete provider is `OpenAI-compatible HTTP embeddings` configured by environment variables.
- Provider configuration must include model name, vector dimension, endpoint base URL, and credential source.
- Query and chunk embeddings must come from the same configured model within a given snapshot lineage.

### Chunking Contract

- `ra_ap_syntax` is the required source-preserving chunk substrate.
- Chunk creation is top-down: crate -> module -> item -> body-region.
- Oversized bodies may split into nested regions, but each body-region must preserve the owning symbol header, canonical symbol path, and source span.
- Tests, doctests, and example code are first-class chunks.
- Structural chunking must succeed even when semantic enrichment is unavailable.

### Semantic Enrichment Contract

- `rust-analyzer` semantic APIs are optional for base indexing and required for enrichment paths when available.
- Enrichment attaches definitions, references, implementations, trait relationships, type hints, containment, re-exports, and test links.
- Failure to enrich must degrade gracefully to structural retrieval without corrupting existing snapshots.

### Retrieval Contract

Inputs:

- `snapshot selector`: exact snapshot id or a resolver input such as repo root plus worktree root.
- `workflow phase`
- `query mode`
- `query text`
- optional symbol hint, file hint, path filters, and diff scope

Outputs:

- ranked target symbols
- bounded neighborhood chunks
- evidence metadata describing why each chunk was selected
- unresolved gaps when semantic enrichment is missing or stale

Retrieval order:

1. resolve snapshot
2. run hybrid candidate search via Tantivy and Qdrant
3. perform bounded graph expansion
4. rerank with workflow phase and diff/worktree locality
5. assemble compact neighborhood

### CLI Contract

The CLI must remain shell-friendly and scriptable.

Required commands:

- `rarag index build`
- `rarag index status`
- `rarag query`
- `rarag symbol`
- `rarag examples`
- `rarag blast-radius`
- `rarag doctor`

Required flags:

- `--repo-root`
- `--worktree`
- `--phase`
- `--mode`
- `--json`
- `--snapshot`

### MCP Contract

The MCP server must listen on a Unix socket and expose tools that map directly to repository-assistance operations.

Required tools:

- `rag_query`
- `rag_symbol_context`
- `rag_examples`
- `rag_blast_radius`
- `rag_index_status`
- `rag_reindex`

Tool outputs must include enough source references and ranking evidence for agents to cite retrieved context in later workflow steps.

### Runtime Path Contract

Defaults:

- runtime socket: `$XDG_RUNTIME_DIR/rarag/raragd.sock`
- state root: `$XDG_STATE_HOME/rarag/`
- cache root: `$XDG_CACHE_HOME/rarag/`

All paths must be overridable by config or CLI flags.

## Invariants

- Snapshot identity is immutable after creation.
- Retrieval results never mix chunks from different snapshots.
- Every body-region retains its owning symbol header and symbol id.
- Worktree selection is explicit; implicit fallback must only occur when exactly one worktree snapshot is available.
- Structural indexing remains usable without semantic enrichment.
- Tantivy, Turso, and Qdrant records remain referentially consistent by `chunk_id` and `snapshot_id`.
- Tests and examples remain first-class retrieval candidates, not optional decorations.
- Query neighborhoods stay bounded and mode-specific; retrieval must not degenerate into whole-file dumping.
- The system surfaces evidence and uncertainty instead of hiding stale or missing semantic data.
- Development workflow support is phase-aware and stops after three review/fix loops unless the caller explicitly overrides policy.

## Task Contracts

### Task 1: Establish Rust Workspace and Runtime Configuration

**Preconditions**

- Rust `1.93+` toolchain is available or installable.
- The user can install required system packages when local package management requires `sudo`.

**Invariants**

- Workspace crates use edition `2024`.
- No task automation assumes the agent can run `sudo`.
- Build configuration remains compatible with CLI, daemon, and MCP binaries.

**Postconditions**

- A Rust workspace exists with crates for core, daemon, CLI, and MCP.
- Config loading supports repo root, worktree root, socket path, store endpoints, and embedding provider settings.

**Tests (must exist before implementation)**

Unit:
- `config::tests::parses_runtime_paths`
- `config::tests::rejects_missing_embedding_provider_fields`

Invariant:
- `workspace::tests::all_crates_use_edition_2024`

Integration:
- `tests/workspace_bootstrap.rs::workspace_builds_with_default_features`

Property-based (optional):
- none

### Task 2: Define Snapshot and Metadata Schema

**Preconditions**

- Workspace and config types exist.
- Turso connection lifecycle is defined.

**Invariants**

- Snapshot keys contain repo root, worktree root, git SHA, cargo target, feature set, and cfg profile.
- Snapshot rows are append-only.
- Metadata schema supports future enrichment without destructive migration assumptions.

**Postconditions**

- Turso schema exists for snapshots, chunks, edges, indexing runs, and query audits.
- Snapshot creation and lookup APIs are deterministic.

**Tests (must exist before implementation)**

Unit:
- `snapshot::tests::normalizes_feature_sets`
- `metadata::tests::serializes_snapshot_key_roundtrip`

Invariant:
- `tests/metadata_invariants.rs::snapshot_ids_are_unique_per_build_world`

Integration:
- `tests/turso_snapshot_store.rs::create_and_load_snapshot`

Property-based (optional):
- none

### Task 3: Implement Structural Chunking

**Preconditions**

- Snapshot model is stable enough to tag chunk output.
- Rust fixture repositories exist for parser tests.

**Invariants**

- Chunk spans round-trip to original source text.
- Oversized symbol bodies split only into body-regions that preserve parent symbol headers.
- Tests and doctests are indexed as first-class chunks.

**Postconditions**

- `ra_ap_syntax` chunking produces crate, module, symbol, body-region, and test/example chunks.
- Chunk metadata includes symbol path, kind, span, file path, docs, and parent relationships.

**Tests (must exist before implementation)**

Unit:
- `chunking::tests::extracts_symbol_chunks_from_fixture`
- `chunking::tests::preserves_symbol_header_on_body_split`

Invariant:
- `tests/chunker_invariants.rs::span_text_matches_source_slice`

Integration:
- `tests/fixture_indexing.rs::indexes_small_workspace_structurally`

Property-based (optional):
- none

### Task 4: Implement Lexical, Vector, and Metadata Storage

**Preconditions**

- Chunk metadata schema is defined.
- Embedding provider configuration is available.

**Invariants**

- Every stored chunk has exactly one metadata row, one lexical document, and one vector point per snapshot.
- Failed embedding batches never leave partial referential state marked complete.
- The system can rebuild a single snapshot without corrupting sibling worktree snapshots.

**Postconditions**

- Turso, Tantivy, and Qdrant adapters ingest and expose indexed chunks.
- A real embedding provider is used for chunk and query vectors.

**Tests (must exist before implementation)**

Unit:
- `storage::tests::maps_chunk_fields_to_tantivy_document`
- `embeddings::tests::builds_openai_compatible_request`

Invariant:
- `tests/index_consistency.rs::metadata_lexical_and_vector_counts_match`

Integration:
- `tests/index_pipeline.rs::reindexes_fixture_repository`

Property-based (optional):
- none

### Task 5: Implement Task-Aware Retrieval and Neighborhood Assembly

**Preconditions**

- Hybrid candidate stores are queryable.
- Chunk edges and workflow phase enums are defined.

**Invariants**

- Neighborhood expansion remains bounded.
- Workflow phase and query mode affect reranking and assembly.
- Exact symbol or path matches outrank vague semantic matches when both refer to the same snapshot.

**Postconditions**

- Retrieval supports `understand-symbol`, `implement-adjacent`, `bounded-refactor`, `find-examples`, and `blast-radius`.
- Responses include explanation metadata for ranking and any unresolved gaps.

**Tests (must exist before implementation)**

Unit:
- `retrieval::tests::prioritizes_exact_symbol_match`
- `retrieval::tests::caps_neighborhood_size_by_mode`

Invariant:
- `tests/retrieval_invariants.rs::results_never_cross_snapshot_boundary`

Integration:
- `tests/retrieval_modes.rs::bounded_refactor_returns_tests_and_references`

Property-based (optional):
- none

### Task 6: Add Semantic Enrichment from Rust Analyzer APIs

**Preconditions**

- Structural chunks already exist and are queryable.
- Rust-analyzer APIs are available in the configured environment.

**Invariants**

- Enrichment is additive and optional.
- Missing or failed semantic calls do not block structural retrieval.
- Edges remain snapshot-scoped.

**Postconditions**

- Definitions, references, implementations, type hints, re-exports, and test links are attached when available.
- Retrieval can expand neighborhoods using semantic edges.

**Tests (must exist before implementation)**

Unit:
- `semantic::tests::maps_reference_results_to_chunk_edges`
- `semantic::tests::falls_back_when_analysis_unavailable`

Invariant:
- `tests/semantic_invariants.rs::enrichment_never_rewrites_chunk_source_spans`

Integration:
- `tests/semantic_fixture.rs::enriches_impl_and_reference_edges`

Property-based (optional):
- none

### Task 7: Deliver Unix-Socket Clients and Workflow Support

**Preconditions**

- Query API and retrieval payloads are stable.
- Runtime path config is stable.

**Invariants**

- CLI and MCP clients return results derived from the same daemon request/response contract.
- Shell output remains stable with `--json`.
- Review/fix loop metadata counts iterations and stops at three by default.

**Postconditions**

- The daemon listens on a Unix socket.
- The CLI can build indexes, inspect status, query neighborhoods, and compute blast radius.
- The MCP server exposes equivalent repository-assistance tools over a Unix socket.

**Tests (must exist before implementation)**

Unit:
- `transport::tests::serializes_unix_socket_requests`
- `workflow::tests::stops_after_three_review_fix_iterations`

Invariant:
- `tests/client_invariants.rs::cli_and_mcp_observe_same_snapshot_result`

Integration:
- `tests/daemon_cli_mcp.rs::cli_and_mcp_roundtrip_against_local_daemon`

Property-based (optional):
- none

## Scenarios

1. `understand-symbol`
   - Input: a symbol path or a code snippet inside one worktree.
   - Expected retrieval: target symbol, parent module summary, direct references, and 1-3 relevant tests/examples.

2. `implement-adjacent`
   - Input: a feature goal and the module or symbol to extend.
   - Expected retrieval: sibling implementations, trait contracts, nearby tests, and current diff neighbors if present.

3. `bounded-refactor`
   - Input: rename or signature-change intent for one symbol.
   - Expected retrieval: definition site, references, related impls/traits, affected tests, and docs/examples.

4. `blast-radius`
   - Input: a target symbol and selected worktree.
   - Expected retrieval: impacted files, symbols, tests, and docs within the selected snapshot only.

5. `workflow-aware review`
   - Input: review phase plus changed file set.
   - Expected retrieval: invariants, tests, and high-risk references near the current diff before proposing fixes.

## Verification

- Required before implementation claims:
  - `scripts/doc-lint.sh --changed --strict-new`
  - `scripts/check-fast-feedback.sh`
- Required during implementation:
  - red/green test commands listed in the implementation plan for the active task
  - targeted fixture indexing and retrieval tests per task
- Required before merge or branch completion:
  - full fast-feedback on current tree state
  - explicit review evidence and unresolved-risk notes

## Risks and Failure Modes

- `rust-analyzer` API churn may increase maintenance cost for enrichment code.
- Embedding provider drift may change vector dimensions or ranking behavior across snapshots.
- Worktree-local diffs may bias retrieval too strongly if reranking weights are not bounded.
- Macro-heavy repositories may expose incomplete semantic edges in early versions.
- Qdrant or embedding-provider outages may reduce retrieval quality; the system must still return lexical plus structural results with warnings.
- Overly large neighborhoods can silently become prompt stuffing if size caps are not enforced and tested.

## Open Questions

- Whether to support additional embedding providers beyond the initial OpenAI-compatible implementation in the first production milestone.
- Whether to persist analyzer-derived call edges in the MVP or start with definitions, references, impls, and test links only.
- Whether reranking should remain heuristic in the MVP or add a learned reranker after retrieval behavior stabilizes.

## References

- Turso Rust crate docs: <https://docs.rs/turso/latest/turso/>
- Tantivy Rust crate docs: <https://docs.rs/tantivy/latest/tantivy/>
- Qdrant Rust client docs: <https://docs.rs/qdrant-client/latest/qdrant_client/>
- `ra_ap_syntax` docs: <https://docs.rs/ra_ap_syntax/latest/ra_ap_syntax/>
- `ra_ap_ide` docs: <https://docs.rs/ra_ap_ide/latest/ra_ap_ide/>
- Vault note: `sharo/research/rust-code-rag-semantic-chunking.md`
