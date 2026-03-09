# Repository RAG Architecture

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-09
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
Related Task Registry ID: `2026-03-09-doc-history-eval-foundations`

## Scope

### In Scope

- Rust implementation targeting toolchain `1.93+` and edition `2024`.
- Hybrid retrieval using `ra_ap_syntax` chunk spans, `rust-analyzer` semantic enrichment when available, BM25 via Tantivy, metadata in Turso, and vector storage in LanceDB.
- Semantic document and structured-knowledge indexing for repository knowledge sources such as specs, plans, changelog, ops docs, install docs, integration docs, templates, and small task-registry tables.
- Snapshot-aware indexing keyed by repository, git worktree, commit SHA, target triple, feature set, and cfg profile.
- Temporal and causal retrieval over git-backed repository history, including symbol/file lineage, change neighborhoods, and bounded rationale evidence.
- Local developer use through a CLI and a local MCP server over Unix sockets.
- Shared TOML configuration for `rarag`, `raragd`, and `rarag-mcp`, with code defaults that remain overridable.
- Retrieval modes tuned for repository-assistance tasks.
- Configurable heuristic reranking and opt-in retrieval observability for evaluation and tuning.
- Task-based retrieval evaluation using persisted observation traces, explicit evidence classes, and small curated repository task fixtures.
- Test-first development and verification-aware repository assistance.

### Out of Scope

- Generic chatbot or open-domain Q&A behavior.
- Remote multi-tenant hosting or cloud control planes.
- Automatic code writing without explicit client-side orchestration.
- Non-Rust indexing in the MVP.
- Full reliance on LSP as the primary index source.
- Live self-modifying prompts, automatic online weight tuning, or autonomous template rollout without offline evaluation and explicit human approval.

## Core Terms

- `snapshot`: An immutable indexable build world identified by `(repo_root, worktree_root, git_sha, cargo_target, feature_set, cfg_profile)`.
- `chunk`: A retrievable unit of source-derived content with stable source span metadata.
- `symbol chunk`: A chunk representing a repository-level symbol such as a function, method, trait, impl block, struct, enum, constant, macro, or test.
- `body region`: A subordinate chunk created when a symbol body exceeds size limits; it must retain its owning symbol header and symbol id.
- `document`: A repository knowledge source such as a Markdown spec, plan, runbook, changelog, integration guide, template, README-class file, or a supported structured text file such as a task-registry CSV.
- `document block`: A heading-scoped, block-scoped, or row-scoped retrievable unit derived from a document with preserved structural path and source span.
- `document semantics`: Typed annotations attached to a document block, including intent, normativity, lifecycle, and resolved references.
- `edge`: A typed relationship between chunks or symbols such as `contains`, `references`, `implements`, or `tested_by`.
- `history node`: A commit-, diff-, or lineage-derived retrieval object representing repository evolution at a bounded point or range in time.
- `lineage edge`: A typed temporal or causal relationship such as `renamed_to`, `split_into`, `followed_by`, `reverted_by`, `fixes`, or `introduced_invariant`.
- `neighborhood`: A bounded set of chunks assembled around one or more target symbols for a specific retrieval request.
- `query mode`: One of `understand-symbol`, `implement-adjacent`, `bounded-refactor`, `find-examples`, or `blast-radius`.
- `evidence class`: A retrieval result family such as `code`, `docs`, `tests`, `config`, or `history` used both for ranking and evaluation.
- `evaluation task`: A repository-task fixture with a prompt, selected revision, expected evidence sets, and distractor expectations used to assess retrieval quality.
- `daemon`: The long-lived local process that owns open indexes, background indexing, and the Unix-socket service surface.
- `worktree lineage`: The set of snapshots derived from a single git worktree over time.

## Interfaces / Contracts

### Component Topology

The architecture consists of four Rust crates in one workspace:

- `rarag-core`: shared library for config, snapshot model, chunking, storage adapters, retrieval, reranking, and neighborhood assembly.
- `raragd`: local daemon process that owns Tantivy indexes, Turso connections, LanceDB handles, file watching, and the internal Unix-socket API.
- `rarag`: CLI wrapper around daemon requests, with stable shell and JSON output modes.
- `rarag-mcp`: local MCP server over Unix sockets that exposes repository-assistance tools backed by the daemon.

### Configuration Contract

- A single optional TOML file, `rarag.toml`, is the canonical user-facing configuration surface for `rarag`, `raragd`, and `rarag-mcp`.
- Shared sections cover runtime paths, storage endpoints, embedding provider settings, indexing behavior, and retrieval behavior.
- Shared sections must also cover repository knowledge-source classification defaults and overrides for document kinds, parser types, path globs, and retrieval priors.
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
- Turso also stores document metadata, document block metadata, document semantic annotations, document references, history/change metadata, lineage edges, and evaluation task metadata when those features are enabled by the active build.
- Turso may also store retrieval observation rows and candidate-feature rows when observability is enabled.
- Tantivy stores lexical fields for chunk text, symbol path, symbol name, docs text, extracted signature text, file path, chunk kind, test/example markers, and repository-state hints.
- Tantivy must also index document-block text, heading path, document kind, intent labels, commands/config references, and history/change summaries when those retrieval families are present.
- LanceDB stores chunk, document-block, and history-node vectors plus optional reranking helper payloads keyed by stable object id and `snapshot_id`.
- All three stores must use the same stable object ids and `snapshot_id` values for the same retrievable object family.

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

### Document Graph Contract

- Repository knowledge files are first-class retrieval sources, not auxiliary prose.
- Document indexing must cover at least these default classes:
  - `README.md`
  - `INSTALL.md`
  - `CHANGELOG.md`
  - `docs/specs/**`
  - `docs/plans/**`
  - `docs/ops/**`
  - `docs/integrations/**`
  - `docs/templates/**`
- Structured knowledge indexing must also support straightforward non-Markdown repository artifacts when explicitly classified, starting with:
  - `docs/tasks/tasks.csv`
- Document ingestion must preserve:
  - file path
  - structural path such as heading path or row identity
  - source line span
  - block type or row type
  - document kind
- Default document classes and retrieval priors may be derived from built-in path conventions, but they must be overridable through shared TOML configuration.
- Shared TOML overrides must support, at minimum:
  - path glob or exact path matching
  - parser type such as `markdown` or `csv`
  - document kind override
  - default intent/normativity priors
  - ranking-priority overrides within bounded allowed ranges
- Heading-scoped chunking is required for Markdown sources; sibling sections must not be merged into one retrievable unit merely to satisfy token budgets.
- Row-scoped chunking is required for supported CSV knowledge sources; unrelated rows must not be merged into one retrievable unit merely to satisfy token budgets.
- Document semantics must support at least:
  - intent labels such as `Requirement`, `Constraint`, `Invariant`, `DesignRationale`, `ImplementationStep`, `OperationalProcedure`, `MigrationNote`, `Troubleshooting`, `FutureWork`, and `NonGoal`
  - normativity labels `normative`, `advisory`, and `informative`
  - lifecycle labels sufficient to distinguish current behavior from future plans and historical notes
- Document references must resolve and store typed links where possible, including:
  - symbols
  - file paths
  - crate/module paths
  - config keys
  - commands and subcommands
  - environment variables
  - tests
  - commits, issues, and PR references when present
- Document indexing must degrade gracefully:
  - unresolved references remain explicit
  - failed semantic annotation must not suppress lexical document retrieval
  - document retrieval must still work without history enrichment
- Built-in path conventions for current repo workflow are defaults, not hidden hardcoded policy; other repositories may override them through config without patching `rarag`.

### Semantic Enrichment Contract

- `rust-analyzer` semantic APIs are optional for base indexing and required for enrichment paths when available.
- Enrichment attaches definitions, references, implementations, trait relationships, type hints, containment, re-exports, and test links.
- Failure to enrich must degrade gracefully to structural retrieval without corrupting existing snapshots.

### History and Causality Contract

- Snapshot identity remains the baseline temporal model for repository state at one build world.
- History retrieval must add bounded repository-evolution objects without relaxing snapshot isolation for present-state retrieval.
- History ingestion must support at least:
  - commit metadata
  - file change summaries
  - symbol change summaries when derivable
  - path rename/move lineage
  - worktree-local change windows
- The system must support explicit history selectors such as:
  - commit SHA
  - timestamp or bounded time window
  - release tag when resolvable
- Temporal retrieval must support at least:
  - `get state at ref`
  - `show changes between refs`
  - `resolve file history`
  - `resolve symbol history`
- Causal retrieval must remain evidence-backed and bounded.
- Causal/lineage edges may include:
  - `renamed_to`
  - `moved_to`
  - `split_into`
  - `merged_from`
  - `followed_by`
  - `reverted_by`
  - `fixes`
  - `depends_on`
  - `introduced_invariant`
  - `tested_by_change`
- The system must not present inferred causal relationships as certain facts when only weak heuristics exist.
- History retrieval must expose uncertainty and the evidence used to derive lineage or causal edges.

### Retrieval Contract

Inputs:

- `snapshot selector`: exact snapshot id or a resolver input such as repo root plus worktree root.
- optional `history selector`: commit SHA, time window, or tag when the query explicitly targets historical state or change analysis.
- `query mode`
- `query text`
- optional symbol hint, file hint, path filters, diff scope, evidence-class hints, and task-type hint

Outputs:

- ranked target symbols
- ranked document blocks and history nodes when relevant to the query
- bounded neighborhood chunks
- evidence-class coverage for the final result set
- evidence metadata describing why each chunk was selected
- unresolved gaps when semantic enrichment is missing or stale
- unresolved gaps when document reference resolution or history derivation is missing, stale, or low-confidence

Retrieval order:

1. resolve snapshot
2. resolve explicit historical scope when requested
3. run hybrid candidate search across code, documents, and history families via Tantivy and LanceDB
4. perform bounded graph expansion across code, document, and lineage edges
5. rerank with diff/worktree locality, document-kind priors, history locality, and query-mode-specific signals
6. assemble compact neighborhood

Rerank configuration:

- The heuristic reranker must remain deterministic by default.
- Rerank and neighborhood weights must be configurable through shared TOML without changing the daemon request surface.
- Code defaults must preserve the current ranking behavior when no overrides are present.
- Normative spec/runbook evidence must outrank advisory or historical-plan evidence when the query asks what behavior is required now, unless the caller explicitly requests planned or historical state.
- Built-in document-class ordering and weighting are defaults and must remain configurable through shared TOML overrides.
- Retrieval must prefer smaller decisive evidence sets over larger loosely related neighborhoods when both satisfy the query.

Observability:

- Retrieval observation is opt-in and disabled by default.
- Observability levels must support at least `off`, `summary`, and `detailed`.
- When enabled, the system must emit structured query observation logs suitable for correlation with agent logs.
- When enabled, the system must also persist enough candidate-feature history to generate offline evaluation sets and compare rerank tuning changes later.
- Observation capture must not change retrieval outputs or ranking decisions.
- Observation capture must preserve enough detail to explain:
  - candidate-generation failure
  - ranking failure
  - granularity failure
  - fusion failure
  - boundedness failure
- Evaluation must be task-based, not query-string-only.
- Evaluation tasks must support:
  - prompt text
  - snapshot or history reference
  - ideal evidence set
  - acceptable evidence set
  - distractor set
  - expected evidence-class coverage
- Supported retrieval-quality metrics must include at least:
  - `hit@k`
  - mean rank of first decisive item
  - evidence-class coverage
  - noise ratio
  - context efficiency
  - answerability
  - sufficiency
  - actionability

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

Service install contract:

- `rarag service install` must generate user systemd units that reference resolved executable paths for `raragd` and `rarag-mcp` instead of assuming a fixed install root such as `%h/.cargo/bin`.
- `rarag service install` must generate unit `--config` arguments using the active resolved config path when available (explicit `--config` first, then resolved config search order), not a hardcoded default path.
- Generated service units must remain deterministic and idempotent for identical resolved inputs.

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
- Heading-scoped document chunks preserve heading path and line span.
- Row-scoped structured-document chunks preserve row identity and source span.
- Document retrieval never silently upgrades advisory or historical-plan text into current normative behavior.
- Existing socket parent directory permissions are never implicitly tightened.
- Local IPC request reads are bounded in bytes and time.
- Local IPC deadlines apply to the full request assembly window, not just individual socket read calls.
- Valid daemon responses are not truncated or rejected by the daemon request-size ceiling.
- Worktree selection is explicit; implicit fallback must only occur when exactly one worktree snapshot is available.
- Structural indexing remains usable without semantic enrichment.
- Tantivy, Turso, and LanceDB records remain referentially consistent by `chunk_id` and `snapshot_id`.
- Tests and examples remain first-class retrieval candidates, not optional decorations.
- Query neighborhoods stay bounded and mode-specific; retrieval must not degenerate into whole-file dumping.
- The system surfaces evidence and uncertainty instead of hiding stale or missing semantic data.
- Development workflow enforcement remains in scripts, docs, and external orchestration rather than retrieval contracts or runtime flags.
- Config defaults remain available even when no config file exists.
- Shared config semantics remain consistent across CLI, daemon, and MCP binaries.
- The MCP transport remains interoperable with standard local MCP clients over Unix sockets.
- Lexical storage remains rich enough to satisfy symbol, docs/example, and bounded-refactor retrieval use cases without depending entirely on embeddings.
- Document references, history nodes, and lineage edges remain explicitly typed; unresolved edges remain visible as unresolved rather than being dropped silently.
- Historical queries only cross snapshot boundaries when the caller explicitly asks for temporal comparison or lineage.
- Rerank defaults preserve baseline behavior unless explicitly overridden in config.
- Observability remains off unless explicitly enabled in config.
- Observation capture remains side-effect free with respect to ranking and retrieval outputs.
- Evaluation data is reproducible against an explicit repository revision or snapshot selector.
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

- Turso, Tantivy, and LanceDB adapters ingest and expose indexed chunks.
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

6. `doc-constrained change`
   - Input: a feature or refactor intent with a subsystem hint.
   - Expected retrieval: current spec blocks, relevant plans, operational constraints, code symbols, config keys, and tests that jointly bound the allowed change.

7. `regression archaeology`
   - Input: a failing test, stack frame, log message, or symptom plus a time window.
   - Expected retrieval: recent related changes, semantic diff summaries, follow-up or revert chains, historical tests, and document rationale when available.

8. `repository usefulness eval`
   - Input: a curated evaluation task at a fixed revision.
   - Expected retrieval: decisive evidence classes appear early, distractors stay bounded, and persisted traces explain candidate loss or ranking noise.

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
- LanceDB or embedding-provider outages may reduce retrieval quality; the system must still return lexical plus structural results with warnings.
- Overly large neighborhoods can silently become prompt stuffing if size caps are not enforced and tested.
- Markdown semantic labeling may overfit repository-specific writing style if priors are not bounded and tested against neutral fixtures.
- Naive history ingestion can become noisy or expensive if commit-level indexing is allowed to outrun symbol/file lineage quality.
- Causal inference can create false confidence unless every derived edge carries source evidence and confidence metadata.
- Evaluation traces can create privacy or storage pressure if raw prompts and retrieved payloads are persisted without retention policy or sampling boundaries.

## Open Questions

- Whether initial history ingestion should be commit-window-first or symbol-lineage-first for the first causal retrieval milestone.
- Whether document semantic annotation should begin with heuristic/path priors only or include an explicit lightweight classifier before multi-repo rollout.
- Whether history and document retrieval should remain primarily behind `rag_query` plus optional hints or gain dedicated read-only CLI/MCP surfaces in the first implementation wave.
- When enough eval evidence exists to justify proposing offline learned rerank or template optimization candidates.

## References

- Turso Rust crate docs: <https://docs.rs/turso/latest/turso/>
- Tantivy Rust crate docs: <https://docs.rs/tantivy/latest/tantivy/>
- LanceDB Rust docs: <https://docs.rs/lancedb/latest/lancedb/>
- `ra_ap_syntax` docs: <https://docs.rs/ra_ap_syntax/latest/ra_ap_syntax/>
- `ra_ap_ide` docs: <https://docs.rs/ra_ap_ide/latest/ra_ap_ide/>
- Vault note: `sharo/research/rust-code-rag-semantic-chunking.md`
- Shared conversation: `https://chatgpt.com/share/69aef8cc-ba2c-8012-abb5-3cf5e5f5dc78`
