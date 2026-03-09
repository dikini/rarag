# Changelog

All notable changes to this project will be documented in this file.

The format is based on Common Changelog:
<https://common-changelog.org/>

## Unreleased

### Added

- Added a Stage 1 architecture/design package for semantic document retrieval, temporal/causal history retrieval, and task-based usefulness evaluation, plus deferred Stage 2 and Stage 3 plans for prompt/skill templates and offline optimization.
- Added Stage 1 retrieval-foundation implementation batch 1 in `rarag-core`: shared TOML document-source defaults with override support (including `docs/tasks/tasks.csv` CSV parsing), metadata storage for document/history/lineage records, and evaluation-ready observation fields (`eval_task_id`, evidence-class coverage).
- Added Stage 1 retrieval-foundation implementation batch 2 in `rarag-core`: history ingestion/lineage derivation persistence, bounded history retrieval selectors in CLI/daemon/MCP, eval fixture loading and replay tests, and query-time `doc_rank_weight` backfill from `document_sources.rules` so TOML rank overrides also apply to pre-marker snapshots.
- Added `rarag eval replay` in the CLI to run fixture-driven offline retrieval replay via daemon query requests, attach `eval_task_id`, and report ideal/acceptable/distractor hit metrics plus evidence-class coverage.
- Added Stage 2 prompt/skill template artifacts: `docs/templates/examples/rarag-evidence-contract.md`, `docs/templates/examples/rarag-skill-template.md`, and task-family prompt templates under `docs/templates/prompts/`.
- Added Stage 3 optimization-governance artifacts: strict-profile metrics/workflow plans (`docs/plans/optimization-metrics-rubric.md`, `docs/plans/offline-optimization-workflow.md`), proposal template (`docs/templates/examples/rarag-optimization-proposal.md`), and operator rollout/rollback guide (`docs/ops/optimization-rollout.md`).
- Added semantic document ingestion for Markdown heading blocks and CSV row blocks, plus indexing persistence of document blocks and new regression fixtures covering specs/plans/ops/templates/changelog/tasks.
- Added mixed code+document retrieval regressions and rerank priors that prefer normative spec/ops evidence over future-plan or changelog evidence for present-behavior queries while keeping retrieval bounded and snapshot-local.
- Added `docs/ops/quickstart.md` as a terse user/operator onboarding path with explicit install, run, validation, and Codex MCP handoff steps.
- Added onboarding guidance for single-project use, multi-project concurrency, and git worktree usage in `docs/ops/quickstart.md`.
- Added `rarag-mcp serve-stdio` with MCP `Content-Length` stdio framing so harnesses like Codex can run `rarag-mcp` as a stdio MCP server while still routing tool calls through the daemon socket.
- Added repository-level `nextest` configuration to cap concurrent test execution at 4 workers to reduce memory pressure during test runs.
- Added a `rarag service` porcelain for user-systemd operations with `install`, `start`, `stop`, `restart`, and daemon-HUP `reload`, including managed-unit safeguards and dry-run support.
- Added `INSTALL.md` as the canonical user install guide with Debian/Ubuntu-first setup, runtime dependency notes, and command discovery guidance.
- Added `docs/ops/systemd-user.md` with user-level unit examples, lifecycle commands, reload behavior, logs, and troubleshooting.
- Added `docs/integrations/` documentation with a tiered support matrix and per-client MCP setup pages for Codex, Claude, opencode, goose, and kimi.
- Added follow-up plan `docs/plans/2026-03-08-service-porcelain-followup-implementation-plan.md` for service-porcelain path-resolution fixes.
- Added configurable heuristic rerank and neighborhood weights in shared TOML config, plus opt-in retrieval observation persistence for offline eval generation.
- Added LanceDB ANN drift integration coverage that compares indexed (`nprobes`/`refine_factor`) versus flat-baseline score and top-k behavior for cosine, L2, and dot metrics.
- Added low-memory nextest tuning by explicitly setting single-test execution in `.config/nextest.toml` and reducing Cargo build parallelism in `scripts/check-tests.sh` (configurable via `RARAG_NEXTEST_BUILD_JOBS` and `RARAG_NEXTEST_PROFILE`).
- Added daemon config reload controls through `SIGHUP`, `rarag daemon reload`, and MCP tool `rag_reload_config`.
- Added a repository RAG architecture spec, design note, and phased implementation plan for a Rust-first, worktree-aware hybrid retrieval system using Turso, Tantivy, Qdrant, `ra_ap_syntax`, and `rust-analyzer`.
- Added the Phase 1 Rust workspace skeleton with `rarag-core`, `raragd`, `rarag`, and `rarag-mcp`, plus bootstrap tests and toolchain configuration.
- Added initial application config and snapshot identity types with validation and JSON roundtrip coverage for worktree-aware indexing.
- Added shared app-config defaults and optional binary-specific config sections for CLI, daemon, and MCP settings.
- Added shared TOML config loading with deterministic search order and merge-on-default behavior in `rarag-core`.
- Added TOML example config and minimal shared-config consumption in `rarag`, `raragd`, and `rarag-mcp`.
- Added the Turso-backed metadata schema and snapshot store with indexing-run and query-audit recording.
- Added the first `ra_ap_syntax` structural chunker with workspace fixture coverage for symbols, tests, and oversized body-region splits.
- Added Tantivy indexing, prepared Qdrant point ingestion, and an OpenAI-compatible embedding request builder tied together through snapshot reindexing.
- Added repository-assistance retrieval modes with bounded neighborhood assembly, ranking evidence, and snapshot-local hybrid lookup.
- Added a checked-in OpenAI-compatible example config that references environment variables instead of secrets.
- Added heuristic semantic enrichment, snapshot-scoped semantic edges, and worktree-diff reranking bias for bounded refactor and review workflows.
- Added a Unix-socket daemon API with snapshot-aware index, query, status, and shutdown requests backed by the shared repository retrieval pipeline.
- Added a shell-friendly CLI and a local MCP-style Unix-socket server that both map directly to the shared daemon contract.
- Added an opt-in `scripts/check-live-rag-stack.sh` pre-merge check for live OpenAI embeddings plus a real Qdrant endpoint.
- Added `docs/ops/qdrant-runtime.md` to document Qdrant runtime setup for operators and developers.
- Added a focused follow-up compatibility plan for bringing `main` into line with the canonical repository RAG spec around MCP transport, example/doctest indexing, and lexical schema coverage.

### Fixed

- Fixed dependency/security policy alignment by adding explicit GPL license metadata to workspace crates, allowing required transitive licenses in `deny.toml`, and pinning local path dependency versions to satisfy wildcard-ban checks.
- Fixed the `policy-checks` GitHub Actions workflow to invoke the official `rhysd/actionlint` installer script with the correct positional arguments (`latest <dir>`) in a writable runner-local bin directory exported to `PATH`.
- Fixed policy verification portability by installing `ripgrep` in the GitHub Actions runner and adding `grep` fallbacks in shell-quality and task-registry policy scripts when `rg` is unavailable.
- Fixed `rarag service install` unit generation to resolve `raragd` and `rarag-mcp` executable paths from the installed CLI location (with `$PATH` fallback) and to use the active resolved config path instead of hardcoded `~/.cargo/bin` and `~/.config/rarag/rarag.toml` assumptions.
- Fixed local Unix-socket hardening so daemon and MCP request reads are size-bounded and time-limited, and so socket startup no longer tightens permissions on pre-existing parent directories.
- Fixed daemon framed-response decoding so valid query payloads larger than the inbound request ceiling still round-trip through the CLI and MCP daemon clients.
- Fixed MCP inbound request handling so the read deadline applies to the full request-assembly window, preventing slow-drip local clients from monopolizing the endpoint.
- Added MCP protocol compatibility regressions, example/doctest chunking regressions, and rich Tantivy schema contract tests for the repository RAG compatibility work.

### Changed

- Changed onboarding docs to optimize readability by routing README and installation flows through a single quickstart-first user/ops path.
- Added a README dependency note explaining that `CDLA-Permissive-2.0` appears via `webpki-roots` pulled by `reqwest`/`rustls` TLS support.
- Changed `README.md` into a concise documentation hub that routes users to install, ops, and integration guides instead of duplicating detailed operator steps inline.
- Changed service-porcelain docs/spec to explicitly document current hardcoded unit path assumptions and the tracked follow-up contract for resolved binary/config unit generation.
- Initialized required project docs by resolving startup placeholders in `README.md` and `AGENTS.md`, and aligned security reporting guidance with repository issue-based intake.
- Replaced the leftover scaffold README with a project-specific overview based on the repo template, covering the actual `rarag` workspace, runtime model, configuration, and verification workflow.
- Made the OpenAI-compatible embedding client configurable for provider base URLs and endpoint paths, with the OpenAI default target aligned to `/v1/embeddings`.
- Removed runtime workflow enforcement from the `rarag` roadmap; workflow orchestration remains in scripts, docs, policy, and external tools rather than the daemon, CLI, or MCP runtime.
- Changed daemon and MCP defaults to use distinct Unix socket paths, while runtime socket overrides derive a companion MCP socket by default.
- Changed the CLI and MCP contract implementations to expose the spec-named command and tool surfaces while preserving compatibility aliases for existing callers.
- Clarified the architecture spec so MCP means actual MCP-compatible Unix-socket transport, and so chunking plus lexical storage requirements explicitly cover examples, doctests, docs text, signatures, and retrieval markers.
- Changed `rarag-mcp` to accept JSON-RPC/MCP-style initialize, tool discovery, and tool call messages over the Unix socket while retaining the legacy local protocol as a compatibility shim for existing tests.
- Changed structural chunking and metadata to carry docs text, signature text, parent relationships, retrieval markers, and repository-state hints across `src/`, `examples/`, integration tests, and extracted Rust doctests.
- Changed the runtime query contract to drop workflow-phase inputs from `rarag-core`, `rarag`, `raragd`, and `rarag-mcp`, and renamed lexical/storage hint fields from `workflow_hints` to `repository_state_hints`.
- Changed project policy to treat backward compatibility as out of scope until the first release unless a spec or plan explicitly requires it.
- Changed retrieval scoring to read config-backed rerank and neighborhood weights while preserving the previous defaults when no overrides are set.
- Changed the canonical repository RAG architecture and active implementation plan to migrate vector storage from endpoint-backed Qdrant to in-process local LanceDB while keeping snapshot-scoped retrieval and rerank evidence contracts.
- Changed runtime operations and live-stack verification to use local LanceDB persistence instead of external Qdrant endpoint configuration.

### Fixed

- Fixed retrieval observability so it records ranked candidate features for eval generation without changing the returned top-N results, and so structured query logs still emit even if observation persistence fails.
- Fixed observation persistence to store observation list fields losslessly and commit query plus candidate observation rows atomically, avoiding comma-corrupted eval data and per-candidate write amplification when observability is enabled.
- Fixed `scripts/check-live-rag-stack.sh` cleanup to use bounded waits with TERM/KILL fallback so daemon shutdown does not hang after successful live-check output.
- Fixed dependency license policy for the Lance/LanceDB stack by explicitly allowing additional transitive OSS license IDs (`BSD-2-Clause`, `0BSD`, `BSL-1.0`, `CC0-1.0`, `MPL-2.0`) required by current graph resolution.
- Fixed `policy-checks` CI reliability by installing `protobuf-compiler` (`protoc`) on the runner so LanceDB/Lance build scripts can compile during workspace test steps.
- Fixed `rust-hygiene` strict mode breakage by removing unsupported `--workspace` from `cargo msrv verify` invocation in `scripts/check-rust-hygiene.sh`.
- Fixed `rust-hygiene` strict mode for workspace manifests by passing explicit MSRV (`RUST_HYGIENE_MSRV`, default `1.93`) to `cargo msrv verify`.
- Changed `policy-checks` CI to use `sccache` plus Rust dependency/build caching and prebuilt `cargo-deny`/`cargo-audit` installs, reducing redundant compile time on repeated runs.
- Changed `rust-hygiene` CI to use `sccache`, Rust caches, prebuilt hygiene tool installs, and `protobuf-compiler` so workspace hygiene runs avoid repeated tool compilation and LanceDB build-script failures.
- Changed policy checks to validate external references by ensuring changed/ranged URLs are reachable and GitHub Action refs in workflows resolve before merge.

- Excluded local `target/` build artifacts from `scripts/init-from-backbone.sh` copies so starter repository initialization stays deterministic and does not pull developer build output into generated repos.
- Fixed worktree-root snapshot resolution to select the latest snapshot instead of failing after repeated indexing, switched the operational vector store to endpoint-backed Qdrant with an explicit test-only in-memory fallback, and hardened Unix-socket cleanup to refuse non-socket paths.
- Fixed retrieval to restore BM25/Tantivy candidate search alongside vector search, added automated CLI and MCP contract regressions, and moved non-`XDG_RUNTIME_DIR` socket defaults into private per-user runtime directories with `0700` permissions.
- Fixed CLI and MCP retrieval contracts to align on repository-assistance modes and home-state runtime socket fallback coverage.
- Fixed lexical retrieval so code-like queries fall back to normalized BM25 parsing instead of failing on Tantivy syntax errors, and expanded lexical indexing to cover symbol names, docs text, signatures, and retrieval markers.

### Removed

- Removed workflow-phase-aware retrieval from the repository RAG architecture, design, and implementation plan; repository state and snapshot-local signals are now the only runtime retrieval context beyond query mode.

### Deprecated

### Security
