# Repository RAG Architecture

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-08
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

Define the canonical architecture for a Rust-first repository assistance RAG system that supports agents working on this project through a strict development workflow: `spec -> plan -> tests -> code -> verify -> review -> fix`.

The system is not a generic question-answering assistant. It is a repository memory and retrieval system for understanding unfamiliar code, adding or modifying features, performing bounded refactors safely, and locating examples, invariants, and blast radius.

Related Task Registry ID: `2026-03-07-shared-config`
Related Task Registry ID: `2026-03-08-rerank-observability`
Related Task Registry ID: `2026-03-08-local-ipc-hardening`

## Scope

### In Scope

- Rust implementation targeting toolchain `1.93+` and edition `2024`.
- Hybrid retrieval using `ra_ap_syntax` chunk spans, `rust-analyzer` semantic enrichment when available, BM25 via Tantivy, metadata in Turso, and vector storage in Qdrant.
- Snapshot-aware indexing keyed by repository, git worktree, commit SHA, target triple, feature set, and cfg profile.
- Local developer use through a CLI and a local MCP server over Unix sockets.
- Shared TOML configuration for `rarag`, `raragd`, and `rarag-mcp`, with code defaults that remain overridable.
- Retrieval modes tuned for repository-assistance tasks.
- Configurable heuristic reranking and opt-in retrieval observability for evaluation and tuning.
- Test-first development and verification-aware repository assistance.

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

### Configuration Contract

- A single optional TOML file, `rarag.toml`, is the canonical user-facing configuration surface for `rarag`, `raragd`, and `rarag-mcp`.
- Shared sections cover runtime paths, storage endpoints, embedding provider settings, indexing behavior, and retrieval behavior.
- Shared config must also cover retrieval rerank weights and observability controls.
- Binary-specific sections cover CLI output defaults, daemon socket/service settings, and MCP socket/tool exposure settings.
- Code defaults remain the first layer. Config values override code defaults. Explicit environment and CLI overrides may apply on top where supported by a binary.
- Supported config resolution order is:
  1. compiled defaults
  2. explicit `--config <path>`
  3. `$RARAG_CONFIG`
  4. `$XDG_CONFIG_HOME/rarag/rarag.toml`
  5. `~/.config/rarag/rarag.toml`
  6. per-field env overrides where explicitly documented
  7. CLI flags
- Config loading must be shared in `rarag-core`; binaries may layer only binary-local overrides and validation on top.
- Missing config files must not be fatal when defaults are sufficient.
- Secrets must never be stored in repo examples or required in config files; config must reference env var names for credentials.

### Storage Contract

- Turso stores snapshot metadata, chunk metadata, graph edges, indexing runs, query audit rows, and provider configuration metadata.
- Turso may also store retrieval observation rows and candidate-feature rows when observability is enabled.
- Tantivy stores lexical fields for chunk text, symbol path, symbol name, docs text, extracted signature text, file path, chunk kind, test/example markers, and repository-state hints.
- Qdrant stores chunk vectors and optional reranking helper payloads keyed by `chunk_id` and `snapshot_id`.
- All three stores must use the same stable `chunk_id` and `snapshot_id` values.

### Embedding Provider Contract

- The MVP must use a real embedding provider, not a fake or no-op implementation.
- The first concrete provider is `OpenAI-compatible HTTP embeddings` configured by environment variables.
- Provider configuration must include model name, vector dimension, endpoint base URL, and credential source.
- Provider configuration must also include an overridable endpoint path so OpenAI-compatible proxies and gateways can be targeted without code changes.
- Query and chunk embeddings must come from the same configured model within a given snapshot lineage.

### Chunking Contract

- `ra_ap_syntax` is the required source-preserving chunk substrate.
- Chunk creation is top-down: crate -> module -> item -> body-region.
- Oversized bodies may split into nested regions, but each body-region must preserve the owning symbol header, canonical symbol path, and source span.
- Structural indexing must traverse repository Rust sources needed for repository assistance, including `src/`, Rust integration tests, and `examples/` when present.
- Tests, doctests, and example code are first-class chunks.
- Rust doc comments with runnable fenced code blocks must be extracted into retrievable doctest/example chunks with source backreferences to the owning item.
- Structural chunking must succeed even when semantic enrichment is unavailable.

### Semantic Enrichment Contract

- `rust-analyzer` semantic APIs are optional for base indexing and required for enrichment paths when available.
- Enrichment attaches definitions, references, implementations, trait relationships, type hints, containment, re-exports, and test links.
- Failure to enrich must degrade gracefully to structural retrieval without corrupting existing snapshots.

### Retrieval Contract

Inputs:

- `snapshot selector`: exact snapshot id or a resolver input such as repo root plus worktree root.
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
4. rerank with diff/worktree locality plus query-mode-specific signals
5. assemble compact neighborhood

Rerank configuration:

- The heuristic reranker must remain deterministic by default.
- Rerank and neighborhood weights must be configurable through shared TOML without changing the daemon request surface.
- Code defaults must preserve the current ranking behavior when no overrides are present.

Observability:

- Retrieval observation is opt-in and disabled by default.
- Observability levels must support at least `off`, `summary`, and `detailed`.
- When enabled, the system must emit structured query observation logs suitable for correlation with agent logs.
- When enabled, the system must also persist enough candidate-feature history to generate offline evaluation sets and compare rerank tuning changes later.
- Observation capture must not change retrieval outputs or ranking decisions.

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
- `rarag daemon reload`
- `rarag service install`
- `rarag service start`
- `rarag service stop`
- `rarag service restart`
- `rarag service reload`

Required flags:

- `--repo-root`
- `--worktree`
- `--mode`
- `--json`
- `--snapshot`

### MCP Contract

The MCP server must listen on a Unix socket and expose tools that map directly to repository-assistance operations.

- The server must speak actual MCP request/response semantics over the Unix-socket transport rather than a project-local tagged JSON protocol.
- `initialize`, tool discovery, and tool invocation must remain compatible with local MCP clients that expect standard MCP tool metadata and call shapes.

Required tools:

- `rag_query`
- `rag_symbol_context`
- `rag_examples`
- `rag_blast_radius`
- `rag_index_status`
- `rag_reindex`
- `rag_reload_config`

Tool outputs must include enough source references and ranking evidence for agents to cite retrieved context in later workflow steps.

### Runtime Path Contract

Defaults:

- runtime socket: `$XDG_RUNTIME_DIR/rarag/raragd.sock`
- state root: `$XDG_STATE_HOME/rarag/`
- cache root: `$XDG_CACHE_HOME/rarag/`

All paths must be overridable by config or CLI flags.

- `rarag` may create missing private runtime directories for its own sockets and state roots.
- If `rarag` creates a runtime directory, it must tighten that directory to owner-only access.
- `rarag` must never implicitly chmod an already existing socket parent directory that it did not create.
- Existing shared directories such as `/tmp`, checked-in project directories, or operator-provided runtime roots must preserve their prior mode and ownership.

### Local IPC Contract

- The daemon and MCP servers are local IPC endpoints over Unix sockets and must remain unary request/response services.
- Inbound request handling must use an explicit bounded request boundary rather than waiting for peer EOF as the sole delimiter.
- Inbound request handling must enforce both:
  - a maximum request size
  - a whole-request read deadline
- These limits may be implementation constants in the MVP; they do not require a new user-facing config surface.
- A single stalled, slow, or oversized local client must not be able to block unrelated daemon or MCP requests indefinitely or drive unbounded memory growth.
- The daemon-side framing/serialization rules must be shared by the daemon server, CLI client, MCP-to-daemon client path, and transport tests so request handling does not drift across binaries.
- Daemon framing must distinguish bounded request sizes from daemon response handling:
  - inbound daemon requests must enforce the configured request ceiling
  - valid daemon responses must not be rejected merely because they exceed the inbound request ceiling
- MCP request handling must stay compatible with the existing local MCP contract while still enforcing bounded reads and timeouts.

### Admin Reload Contract

- `raragd` must support configuration reload via `SIGHUP`.
- The daemon must also expose an explicit admin reload request so CLI and MCP callers can trigger the same behavior safely.
- `rarag service reload` must target daemon `SIGHUP` behavior; MCP service reload is intentionally unsupported.
- Reload must be validate-then-swap:
  - parse candidate config
  - validate it
  - initialize any newly required sinks or dependencies
  - atomically replace active runtime config only after validation succeeds
- Failed reloads must preserve the last known-good active configuration.
- In-flight requests must complete using the configuration snapshot they started with; later requests may observe the new configuration.

### Security Contract

- Example config files must contain no tokens, passwords, or inline secrets.
- Config parsing and error reporting must avoid echoing secret values from environment variables.
- Binaries must tolerate missing secret env vars until the dependent operation is actually invoked, unless the binary is explicitly validating readiness.
- Config examples may reference credential env var names only.
- Existing operator-managed directories must not have permissions tightened as a side effect of socket startup.

## Invariants

- Snapshot identity is immutable after creation.
- Retrieval results never mix chunks from different snapshots.
- Every body-region retains its owning symbol header and symbol id.
- Existing socket parent directory permissions are never implicitly tightened.
- Local IPC request reads are bounded in bytes and time.
- Local IPC deadlines apply to the full request assembly window, not just individual socket read calls.
- Valid daemon responses are not truncated or rejected by the daemon request-size ceiling.
- Worktree selection is explicit; implicit fallback must only occur when exactly one worktree snapshot is available.
- Structural indexing remains usable without semantic enrichment.
- Tantivy, Turso, and Qdrant records remain referentially consistent by `chunk_id` and `snapshot_id`.
- Tests and examples remain first-class retrieval candidates, not optional decorations.
- Query neighborhoods stay bounded and mode-specific; retrieval must not degenerate into whole-file dumping.
- The system surfaces evidence and uncertainty instead of hiding stale or missing semantic data.
- Development workflow enforcement remains in scripts, docs, and external orchestration rather than retrieval contracts or runtime flags.
- Config defaults remain available even when no config file exists.
- Shared config semantics remain consistent across CLI, daemon, and MCP binaries.
- The MCP transport remains interoperable with standard local MCP clients over Unix sockets.
- Lexical storage remains rich enough to satisfy symbol, docs/example, and bounded-refactor retrieval use cases without depending entirely on embeddings.
- Rerank defaults preserve baseline behavior unless explicitly overridden in config.
- Observability remains off unless explicitly enabled in config.
- Observation capture remains side-effect free with respect to ranking and retrieval outputs.
- Config reload never leaves the daemon in a partially applied configuration state.

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
- Chunk edges and query mode enums are defined.

**Invariants**

- Neighborhood expansion remains bounded.
- Query mode and repository-state signals affect reranking and assembly.
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

### Task 7: Deliver Unix-Socket Clients

**Preconditions**

- Query API and retrieval payloads are stable.
- Runtime path config is stable.

**Invariants**

- CLI and MCP clients return results derived from the same daemon request/response contract.
- Shell output remains stable with `--json`.
- Workflow enforcement remains outside the runtime and is handled by scripts, docs, and external orchestration.

**Postconditions**

- The daemon listens on a Unix socket.
- The CLI can build indexes, inspect status, query neighborhoods, and compute blast radius.
- The MCP server exposes equivalent repository-assistance tools over a Unix socket.

**Tests (must exist before implementation)**

Unit:
- `transport::tests::serializes_unix_socket_requests`

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

5. `diff-local review`
   - Input: changed file set plus an optional symbol or path hint.
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
