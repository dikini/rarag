# Repository RAG Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Capture the approved architecture baseline for a Rust-first repository assistance RAG before implementation work begins.
Architecture: Use a local daemon plus thin CLI and MCP clients. Keep indexing and retrieval logic in `rarag-core`, use `ra_ap_syntax` for structural chunking, and enrich with `rust-analyzer` semantics when available.
Tech Stack: Rust 1.93+, edition 2024, Turso, Tantivy, Qdrant, `ra_ap_syntax`, `ra_ap_ide`, Unix domain sockets.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design document's architecture and verification contracts.
4. Explicit updates recorded in this file.

## Execution Mode

- Mode: plan-only
- Default: plan-only for this design document.

## Output Contract

- Keep the architecture constrained to repository assistance, not generic QA.
- Keep component boundaries simple enough to support phased implementation.
- Record worktree, workflow, and retrieval assumptions explicitly.

## Task Update Contract

- New architecture constraints must be added to the relevant design sections before implementation.
- Any implementation plan derived from this document must remain consistent with the approved component boundaries.
- Changes to storage or transport choices must preserve CLI and MCP compatibility over Unix sockets.

## Completion Gate

- The design is complete only when the architecture, storage model, retrieval model, worktree model, and runtime boundary are all explicit.
- Completion also requires doc verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

## Design Summary

### Recommended Option

Use a `daemon + thin clients` architecture.

- `rarag-core` owns chunking, indexing, retrieval, and neighborhood assembly.
- `raragd` owns warm indexes, Turso/Tantivy/Qdrant connections, and a Unix-socket API.
- `rarag` is the shell-facing CLI.
- `rarag-mcp` is the local MCP adapter over a Unix socket and must remain compatible with standard local MCP clients rather than a project-local protocol.

This keeps the runtime small, supports shell scripts cleanly, and avoids duplicating indexing logic across CLI and MCP clients.

### Chunking

Use `ra_ap_syntax` to create source-preserving chunk spans.

Primary chunk types:

- crate summaries
- module summaries
- symbol chunks
- body regions
- test/example chunks

Repository-assistance chunking must include `src/`, Rust integration-test files, `examples/`, and runnable Rust doctests extracted from item docs.

Oversized bodies split recursively, but every body region keeps the owning symbol header and canonical symbol path.

### Semantic Enrichment

Use `rust-analyzer` semantic APIs when available for:

- definitions
- references
- impl relationships
- type hints
- re-exports
- test links

Structural indexing must still work without enrichment.

### Retrieval

Use hybrid retrieval:

- lexical recall with Tantivy BM25
- semantic recall with Qdrant vectors
- bounded graph expansion using metadata edges from Turso

The lexical side must include symbol names, docs text, signatures, and example/test markers so repository assistance does not collapse into vector-only retrieval.

Retrieval is always snapshot-scoped and tuned to repository-assistance task modes.

### Snapshot Model

A snapshot is keyed by:

- repo root
- worktree root
- git SHA
- cargo target
- feature set
- cfg profile

This prevents incompatible semantic worlds from mixing, especially across worktrees and feature sets.

### Embeddings

The MVP uses a real `OpenAI-compatible HTTP embeddings` provider configured by environment variables. Chunk and query embeddings use the same model per snapshot lineage.

### Retrieval Alignment

The system is designed to support repository-assistance tasks directly through `query mode` and repository-state signals.

Retrieval requests carry a `query mode` plus snapshot-local hints such as symbol, path, and diff scope. Reranking can bias toward invariants, nearby examples, tests, or refactor blast radius based on the requested repository-assistance task and the selected snapshot, without carrying an explicit workflow-phase parameter through the runtime.

Workflow enforcement remains outside the runtime and is handled by scripts, docs, and external orchestration rather than `rarag` binaries.

### Worktree Model

Worktrees are first-class.

- snapshot identity includes worktree root
- current diff locality can bias reranking
- blast-radius queries never cross snapshot boundaries
- shared stores are allowed, but all retrieval is filtered by snapshot id

### Operational Defaults

- socket: `$XDG_RUNTIME_DIR/rarag/raragd.sock`
- state: `$XDG_STATE_HOME/rarag/`
- cache: `$XDG_CACHE_HOME/rarag/`

## Phases

1. Foundation: workspace, config, snapshot model, Turso schema
2. Indexing: `ra_ap_syntax` chunker, Tantivy, Qdrant, real embeddings
3. Retrieval: query modes, bounded neighborhoods, reranking
4. Enrichment: `rust-analyzer` semantic edges
5. Clients: daemon, CLI, MCP over Unix sockets
6. Verification: fixture repos and worktree scenarios

### Task 1: Ratify Architecture Baseline

**Files:**

- Modify: `docs/specs/repository-rag-architecture.md`
- Modify: `docs/plans/2026-03-07-repository-rag-implementation-plan.md`
- Modify: `CHANGELOG.md`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- User requirements and the vault research note are available.
- Storage and client choices are constrained to Turso, Tantivy, Qdrant, CLI, and MCP over Unix sockets.

**Invariants**

- The design remains centered on symbols and change neighborhoods.
- The architecture stays simple enough for phased implementation.
- Worktree-aware snapshots remain mandatory.

**Postconditions**

- Architecture choices are explicit and aligned with the implementation plan.
- Review/fix loop constraints and worktree behavior are documented.

**Tests (must exist before implementation)**

Unit:
- `doc-lint design header check`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --path docs/plans/2026-03-07-repository-rag-design.md --strict-new`
Expected: fail until the design document contains the required strict-profile sections.

**Implementation Steps**

1. Record the selected architecture, storage, and transport choices.
2. Record runtime, worktree, and retrieval boundaries.
3. Align the design summary with the spec and implementation plan.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: all design and planning documents pass lint.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/repository-rag-architecture.md`, `docs/plans/2026-03-07-repository-rag-design.md`, `docs/plans/2026-03-07-repository-rag-implementation-plan.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
