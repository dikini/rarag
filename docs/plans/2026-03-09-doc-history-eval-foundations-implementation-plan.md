# Document, History, and Evaluation Foundations Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Task Registry ID: `2026-03-09-doc-history-eval-foundations-impl`

Goal: Implement the Stage 1 retrieval foundations for semantic documents, temporal/causal history, and task-based usefulness evaluation in `rarag`.
Architecture: Extend the existing snapshot-scoped retrieval pipeline in `rarag-core` with document and history object families while keeping one merged candidate/rerank/assembly flow. Reuse Turso, Tantivy, LanceDB, and existing observability plumbing so evaluation measures the same evidence model later used by prompt/template and optimization work, and treat repo-specific document classes as defaults that can be overridden through shared TOML.
Tech Stack: Rust 1.93+, edition 2024, `pulldown-cmark` or equivalent Markdown parser, a straightforward CSV parser, current git metadata extraction approach, Turso, Tantivy, LanceDB, shared TOML config, CLI/daemon/MCP crates, and existing retrieval observation pipeline.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: plan-only
- Default: plan-only because this turn produces planning artifacts, not implementation.

## Output Contract

- Keep one retrieval architecture across code, docs, and history.
- Deliver evaluation as measurement and fixture infrastructure, not a side-channel approximation of retrieval behavior.
- Prefer bounded heuristics and typed evidence over speculative causal inference.

## Task Update Contract

- Any requested Stage 2 or Stage 3 work must first map to a missing Stage 1 output before altering this plan.
- If a task expands surface area, document why the existing `query`/MCP retrieval surface is insufficient.
- If causal inference quality is weak, keep the representation explicit and confidence-scored rather than hiding uncertainty.

## Completion Gate

- A task is complete only when preconditions, invariants, postconditions, and listed tests are satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Extend the Core Data Model and Config for Documents and History

**Files:**

- Modify: `docs/specs/repository-rag-architecture.md`
- Modify: `crates/rarag-core/src/chunking/types.rs`
- Modify: `crates/rarag-core/src/metadata/schema.sql`
- Modify: `crates/rarag-core/src/metadata/store.rs`
- Modify: `crates/rarag-core/src/config.rs`
- Modify: `crates/rarag-core/src/config_loader.rs`
- Modify: `crates/rarag-core/tests/turso_snapshot_store.rs`
- Test: `crates/rarag-core/tests/config_snapshot.rs`
- Test: `crates/rarag-core/tests/turso_snapshot_store.rs`

**Preconditions**

- Current chunk, metadata, and observability schemas exist.
- The canonical spec has been updated with document/history contracts.

**Invariants**

- Stable ids remain consistent across metadata, lexical, and vector stores.
- Snapshot identity remains the base unit for present-state retrieval.
- Observation capture remains side-effect free.

**Postconditions**

- Core metadata types can represent document blocks, document semantics, document refs, history nodes, and lineage edges.
- Shared config can express default document-source classes and repository-specific overrides.
- Storage schema supports those families without breaking existing retrieval objects.

**Tests (must exist before implementation)**

Unit:
- `config_snapshot::parses_document_history_and_doc_source_sections`

Invariant:
- `turso_snapshot_store::document_and_history_rows_roundtrip_without_cross_snapshot_leakage`

Integration:
- `turso_snapshot_store::stores_document_blocks_history_nodes_and_observations`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test config_snapshot --test turso_snapshot_store -- --nocapture`
Expected: fail on new document/history schema and config assertions.

**Implementation Steps**

1. Add typed config and metadata model structures for document/history families.
2. Extend SQL schema and store APIs with document and lineage tables.
3. Add document-source classification config for default path rules plus TOML overrides.
4. Add observation fields needed for task-based evaluation labels and evidence-class coverage.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test config_snapshot --test turso_snapshot_store -- --nocapture`
Expected: new schema/config/storage tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/chunking/types.rs`, metadata/config modules and tests
Re-run: `cargo test -p rarag-core --test config_snapshot --test turso_snapshot_store -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Implement Semantic Markdown Document Ingestion

**Files:**

- Create: `crates/rarag-core/src/chunking/markdown.rs`
- Create: `crates/rarag-core/src/chunking/csv.rs`
- Modify: `crates/rarag-core/src/chunking/mod.rs`
- Modify: `crates/rarag-core/src/indexing/mod.rs`
- Modify: `crates/rarag-core/src/metadata/store.rs`
- Create: `crates/rarag-core/tests/document_chunking.rs`
- Modify: `crates/rarag-core/tests/index_pipeline.rs`
- Test: `crates/rarag-core/tests/document_chunking.rs`
- Test: `crates/rarag-core/tests/index_pipeline.rs`

**Preconditions**

- Core data model supports document objects.
- Repository document scope and path priors are defined.
- Shared config can resolve parser and document kind by path with built-in defaults.

**Invariants**

- Heading-scoped document chunks preserve heading path and line spans.
- Plans and changelog remain distinguishable from current normative specs.
- Unresolved references remain explicit rather than silently dropped.

**Postconditions**

- Markdown and supported CSV knowledge files are indexed as typed document blocks with semantic annotations and references.
- Document retrieval candidates are available to lexical/vector retrieval and metadata-backed expansion.

**Tests (must exist before implementation)**

Unit:
- `document_chunking::classifies_spec_plan_ops_changelog_and_tasks_registry_sources`
- `document_chunking::extracts_heading_path_and_line_spans`
- `document_chunking::extracts_tasks_csv_rows_as_structured_blocks`

Invariant:
- `document_chunking::sibling_sections_never_merge_into_one_chunk`

Integration:
- `index_pipeline::indexes_docs_templates_and_changelog_as_document_blocks`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test document_chunking --test index_pipeline -- --nocapture`
Expected: fail because markdown ingestion and document assertions do not exist yet.

**Implementation Steps**

1. Add Markdown parsing and heading/block chunking with path-aware document kind classification.
2. Extract semantic labels and typed references with bounded heuristic priors.
3. Resolve parser and document kind through built-in defaults plus TOML overrides.
4. Thread document objects through indexing persistence and lexical/vector ingestion.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test document_chunking --test index_pipeline -- --nocapture`
Expected: document ingestion and indexing tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: markdown chunking/indexing modules and related tests
Re-run: `cargo test -p rarag-core --test document_chunking --test index_pipeline -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Fuse Document Evidence into Retrieval and Reranking

**Files:**

- Modify: `crates/rarag-core/src/retrieval/mod.rs`
- Modify: `crates/rarag-core/src/retrieval/rerank.rs`
- Modify: `crates/rarag-core/src/retrieval/neighborhood.rs`
- Modify: `crates/rarag-core/tests/retrieval_modes.rs`
- Modify: `crates/rarag-core/tests/semantic_fixture.rs`
- Test: `crates/rarag-core/tests/retrieval_modes.rs`

**Preconditions**

- Document blocks are queryable through storage backends.
- Existing code retrieval and rerank baselines are covered by tests.

**Invariants**

- Current normative behavior questions prefer spec/runbook evidence over future plans or historical summaries.
- Retrieval remains bounded and evidence-class aware.
- Existing code-only retrieval regressions remain covered.

**Postconditions**

- Retrieval can return mixed code/document neighborhoods with explicit evidence-class coverage.
- Rerank logic understands document kind, normativity, and lifecycle priors.

**Tests (must exist before implementation)**

Unit:
- `retrieval_modes::prefers_normative_spec_over_plan_for_current_behavior`
- `retrieval_modes::returns_docs_code_and_tests_for_doc_constrained_change`

Invariant:
- `retrieval_modes::document_evidence_never_expands_to_unbounded_sibling_sections`

Integration:
- `semantic_fixture::mixed_code_and_doc_evidence_preserves_snapshot_boundary`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test retrieval_modes --test semantic_fixture -- --nocapture`
Expected: fail on new mixed-evidence and document-prior assertions.

**Implementation Steps**

1. Add document candidates to candidate merge and evidence metadata.
2. Extend rerank and neighborhood assembly with document priors and evidence-class diversity signals.
3. Add mixed-evidence regression coverage for current-behavior, implementation, and bounded-review tasks.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test retrieval_modes --test semantic_fixture -- --nocapture`
Expected: mixed retrieval tests pass without breaking existing code retrieval behavior.

**Refactor Phase (optional but controlled)**

Allowed scope: retrieval modules and related tests
Re-run: `cargo test -p rarag-core --test retrieval_modes --test semantic_fixture -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 4: Implement History Ingestion and Lineage Derivation

**Files:**

- Create: `crates/rarag-core/src/history/mod.rs`
- Create: `crates/rarag-core/src/history/git.rs`
- Create: `crates/rarag-core/src/history/lineage.rs`
- Modify: `crates/rarag-core/src/indexing/mod.rs`
- Modify: `crates/rarag-core/src/metadata/store.rs`
- Create: `crates/rarag-core/tests/history_lineage.rs`
- Test: `crates/rarag-core/tests/history_lineage.rs`
- Test: `crates/rarag-core/tests/turso_snapshot_store.rs`

**Preconditions**

- Document and code objects already have stable storage identity.
- Git repository metadata is accessible during indexing.

**Invariants**

- Historical state remains explicit and separate from present-state snapshot retrieval unless requested.
- Derived lineage edges carry evidence and confidence.
- Weak history inference does not masquerade as certain causality.

**Postconditions**

- Commit, change, and lineage metadata can be ingested and queried.
- File/path history and basic symbol-history summaries are available for later retrieval.

**Tests (must exist before implementation)**

Unit:
- `history_lineage::resolves_path_rename_chain`
- `history_lineage::marks_heuristic_causal_edges_with_confidence`

Invariant:
- `history_lineage::historical_objects_never_appear_in_present_state_results_without_selector`

Integration:
- `turso_snapshot_store::stores_history_nodes_and_lineage_edges`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test history_lineage --test turso_snapshot_store -- --nocapture`
Expected: fail because history ingestion and lineage derivation modules do not exist yet.

**Implementation Steps**

1. Add git-backed history extraction and typed change summaries.
2. Persist history nodes and lineage edges with bounded evidence payloads.
3. Implement initial path and symbol lineage derivation rules with explicit uncertainty metadata.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test history_lineage --test turso_snapshot_store -- --nocapture`
Expected: history ingestion and lineage tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: history ingestion modules, metadata store, related tests
Re-run: `cargo test -p rarag-core --test history_lineage --test turso_snapshot_store -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 5: Add Temporal and Causal Retrieval Primitives

**Files:**

- Modify: `crates/rarag-core/src/daemon.rs`
- Modify: `crates/rarag-core/src/retrieval/mod.rs`
- Modify: `crates/rarag-core/src/retrieval/rerank.rs`
- Modify: `crates/rarag/src/cli.rs`
- Modify: `crates/raragd/src/server.rs`
- Modify: `crates/rarag-mcp/src/tools.rs`
- Modify: `crates/rarag-core/tests/daemon_transport.rs`
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`
- Test: `crates/rarag-core/tests/retrieval_modes.rs`
- Test: `crates/rarag-core/tests/daemon_cli_mcp.rs`

**Preconditions**

- History nodes and lineage edges are stored and queryable.
- Retrieval already supports mixed code/document evidence.

**Invariants**

- Present-state query behavior stays stable unless explicit history selectors or history-focused modes are used.
- Historical queries remain bounded and evidence-backed.
- CLI, daemon, and MCP remain aligned on the same request contract.

**Postconditions**

- Callers can request bounded historical state/change context and receive mixed code/doc/history evidence.
- MCP and CLI expose the minimum viable temporal/causal retrieval controls needed for repository assistance.

**Tests (must exist before implementation)**

Unit:
- `retrieval_modes::history_selector_limits_results_to_requested_window`
- `retrieval_modes::regression_archaeology_returns_changes_docs_and_tests`

Invariant:
- `daemon_cli_mcp::history_queries_match_across_cli_and_mcp`

Integration:
- `daemon_cli_mcp::mixed_history_and_present_state_query_roundtrip`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test retrieval_modes --test daemon_cli_mcp --test daemon_transport -- --nocapture`
Expected: fail on new history-aware request and retrieval assertions.

**Implementation Steps**

1. Extend the shared request model with optional history/evidence selectors.
2. Add bounded history candidate generation and rerank integration.
3. Expose the new read-only controls through CLI and MCP without forking daemon semantics.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test retrieval_modes --test daemon_cli_mcp --test daemon_transport -- --nocapture`
Expected: history-aware retrieval and transport tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: shared request models, retrieval modules, CLI/daemon/MCP adapters, related tests
Re-run: `cargo test -p rarag-core --test retrieval_modes --test daemon_cli_mcp --test daemon_transport -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 6: Build Task-Based Evaluation Fixtures and Reporting

**Files:**

- Modify: `crates/rarag-core/src/retrieval/mod.rs`
- Modify: `crates/rarag-core/src/metadata/store.rs`
- Create: `crates/rarag-core/tests/eval_fixtures.rs`
- Create: `examples/eval/README.md`
- Modify: `README.md`
- Modify: `examples/rarag.example.toml`
- Test: `crates/rarag-core/tests/eval_fixtures.rs`
- Test: `crates/rarag-core/tests/retrieval_modes.rs`

**Preconditions**

- Observation capture already records candidate features.
- Mixed evidence retrieval exists for code, docs, and history.

**Invariants**

- Evaluation traces remain observational only.
- Task fixtures remain revision-pinned and reproducible.
- Metrics measure repository usefulness rather than generic free-text similarity only.

**Postconditions**

- `rarag` has a small curated eval fixture model for repository tasks.
- Observation persistence and reports can explain candidate, ranking, granularity, fusion, and boundedness failures.

**Tests (must exist before implementation)**

Unit:
- `eval_fixtures::loads_task_with_ideal_acceptable_and_distractor_sets`
- `retrieval_modes::observation_trace_records_evidence_class_coverage`

Invariant:
- `retrieval_modes::evaluation_capture_does_not_change_ranked_results`

Integration:
- `eval_fixtures::revision_pinned_eval_task_replays_against_observation_store`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test eval_fixtures --test retrieval_modes -- --nocapture`
Expected: fail on new eval-fixture and trace-shape assertions.

**Implementation Steps**

1. Add task-fixture loading and persistence/report helpers.
2. Extend observation records with evaluation-friendly evidence-class and failure-taxonomy fields.
3. Document how to run small repository usefulness evals against pinned revisions.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test eval_fixtures --test retrieval_modes -- --nocapture`
Expected: evaluation fixtures and trace reporting tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: evaluation fixtures, observation plumbing, docs/examples, related tests
Re-run: `cargo test -p rarag-core --test eval_fixtures --test retrieval_modes -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
