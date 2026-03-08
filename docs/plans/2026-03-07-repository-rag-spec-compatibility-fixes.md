# Repository RAG Spec Compatibility Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Bring the shipped `rarag` tree back into compatibility with the canonical repository RAG spec by fixing MCP transport compliance, first-class example/doctest indexing, and lexical schema coverage.

**Architecture:** Keep the existing four-crate topology, but add a real MCP transport layer in `rarag-mcp`, expand `rarag-core` chunk metadata and chunking roots for examples/doctests, and enrich the Tantivy schema so hybrid retrieval can honor the spec’s symbol/docs/example retrieval contract.

**Tech Stack:** Rust 1.93+, edition 2024, `tokio`, Unix domain sockets, MCP-compatible JSON-RPC transport, `ra_ap_syntax`, Tantivy, Turso, Qdrant.

Task Registry ID: `2026-03-07-repository-rag-spec-compatibility`

Template-Profile: tdd-strict-v1

---

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: plan-only
- Default: execute-with-checkpoints unless the user explicitly requests plan-only output.

## Output Contract

- Keep the fix scope constrained to concrete spec/implementation mismatches.
- Define every task with exact files, Red/Green verification commands, and completion evidence.
- Preserve the existing four-crate architecture unless a task explicitly documents an approved boundary change.

## Task Update Contract

- New review findings must be mapped to an existing phase/task or added as a new task before implementation proceeds.
- If implementation evidence contradicts the spec, correct the implementation unless the user explicitly approves a spec change.
- Do not silently normalize custom protocols or partial indexing behavior as “compatible.”

## Completion Gate

- Plan completion requires the spec, design note, and this implementation plan to agree on MCP transport, chunking coverage, and lexical schema expectations.
- Execution against this plan is complete only when all Red/Green gates and repo verification commands pass with changelog/task-registry updates.

## Model Compatibility Notes

- XML-style delimiters are optional and treated as plain text.
- Critical transport and chunking constraints are restated in plain language for cross-model robustness.

## Scope

This plan covers only the compatibility gaps found during post-merge review on `main`:

1. `rarag-mcp` is still a custom Unix-socket protocol rather than a real MCP server.
2. Structural chunking ignores `examples/`, integration-test sources, and Rust doctests.
3. Chunk metadata and Tantivy lexical fields are too narrow for the storage and retrieval contracts in the spec.

This plan does not redesign retrieval modes, snapshot identity, or the daemon request model.

## Phase 1: Acceptance Tests and Transport Boundary

### Task 1: Add Spec-Compatibility Regression Tests

**Files:**
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`
- Create: `crates/rarag-core/tests/mcp_protocol.rs`
- Create: `crates/rarag-core/tests/chunker_examples_doctests.rs`
- Create: `crates/rarag-core/tests/tantivy_schema_contract.rs`
- Modify: `CHANGELOG.md`

**Preconditions**
- Current `main` builds and existing integration tests pass.
- The spec remains the canonical behavior source.

**Invariants**
- New tests assert only documented behavior, not incidental implementation details.
- Existing CLI/daemon compatibility coverage remains intact.

**Postconditions**
- There are explicit failing tests for:
  - standard MCP initialization, tool listing, and tool call flow over the Unix socket
  - indexing of `examples/` Rust files
  - indexing of runnable Rust doctest code blocks
  - Tantivy document coverage for symbol name, docs text, signature text, example/test markers, and repository-state hints

**Tests (must exist before implementation)**

Unit:
- `tantivy_schema_contract::maps_chunk_fields_to_rich_lexical_document`

Invariant:
- `chunker_examples_doctests::indexes_examples_and_doctests_as_first_class_chunks`

Integration:
- `mcp_protocol::standard_client_can_initialize_and_call_rag_tools`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command:
```bash
cargo test -p rarag-core --test mcp_protocol --test chunker_examples_doctests --test tantivy_schema_contract -- --nocapture
```

Expected:
- failures showing missing MCP protocol support
- failures showing missing example/doctest chunks
- failures showing missing lexical fields

**Implementation Steps**

1. Add a minimal MCP protocol harness that exercises initialization, tool listing, and tool invocation against `rarag-mcp`.
2. Add a fixture workspace containing:
   - `src/` item docs with runnable fenced Rust examples
   - `examples/` Rust files
   - integration-test Rust files
3. Add lexical-schema contract assertions against Tantivy indexed documents.
4. Update `CHANGELOG.md` for the compatibility-fix track.

**Green Phase (required)**

Command:
```bash
cargo test -p rarag-core --test mcp_protocol --test chunker_examples_doctests --test tantivy_schema_contract -- --nocapture
```

Expected: all new regression tests pass.

**Completion Evidence**
- Preconditions satisfied
- Invariants preserved
- Postconditions met
- New failing tests existed before implementation

## Phase 2: Rich Chunk Metadata and Structural Coverage

### Task 2: Expand Chunk Metadata for Docs, Signatures, and Relationships

**Files:**
- Modify: `crates/rarag-core/src/chunking/types.rs`
- Modify: `crates/rarag-core/src/chunking/mod.rs`
- Modify: `crates/rarag-core/src/chunking/rust.rs`
- Modify: `crates/rarag-core/src/metadata/schema.sql`
- Modify: `crates/rarag-core/src/metadata/store.rs`
- Modify: `crates/rarag-core/tests/chunker_fixture.rs`
- Modify: `crates/rarag-core/tests/chunker_examples_doctests.rs`

**Preconditions**
- Regression tests from Task 1 exist.

**Invariants**
- Existing chunk ids and source spans remain snapshot-local and deterministic for unchanged content.
- Structural indexing remains usable without semantic enrichment.

**Postconditions**
- `Chunk` metadata includes at least:
  - symbol path
  - symbol name
  - docs text
  - extracted signature text
  - parent symbol/module relationship
  - example/test/doctest markers
- Metadata persistence round-trips those fields.

**Tests (must exist before implementation)**

Unit:
- `chunker_fixture::captures_symbol_docs_and_signature_text`

Invariant:
- `chunker_fixture::body_region_preserves_parent_relationships`

Integration:
- `chunker_examples_doctests::metadata_roundtrips_example_and_doctest_chunks`

**Red Phase (required before code changes)**

Command:
```bash
cargo test -p rarag-core --test chunker_fixture --test chunker_examples_doctests -- --nocapture
```

Expected: failures for missing docs/signatures/parent metadata and missing example/doctest chunk persistence.

**Implementation Steps**

1. Extend `ChunkKind` as needed for example/doctest representation without overcomplicating the enum.
2. Extend `Chunk` to store docs text, signature text, parent identifiers, and retrieval markers.
3. Update the metadata schema and store layer to persist/load the expanded fields.
4. Keep source-preserving chunk spans as the primary structural truth.

**Green Phase (required)**

Command:
```bash
cargo test -p rarag-core --test chunker_fixture --test chunker_examples_doctests -- --nocapture
```

Expected: all chunk metadata and persistence checks pass.

**Completion Evidence**
- Preconditions satisfied
- Invariants preserved
- Postconditions met

### Task 3: Index `examples/`, Integration Tests, and Doctests

**Files:**
- Modify: `crates/rarag-core/src/chunking/rust.rs`
- Modify: `crates/rarag-core/tests/chunker_examples_doctests.rs`
- Modify: `tests/fixtures/mini_repo/`

**Preconditions**
- Expanded chunk metadata exists.

**Invariants**
- `src/` indexing behavior remains unchanged for existing symbol/test chunks.
- Example/doctest extraction must not duplicate unrelated source chunks.

**Postconditions**
- Structural chunking traverses:
  - `src/`
  - `examples/`
  - Rust integration-test files
- Runnable Rust doctests are emitted as first-class retrievable chunks linked back to the owning item.

**Tests (must exist before implementation)**

Unit:
- `chunker_examples_doctests::indexes_examples_directory_files`
- `chunker_examples_doctests::extracts_runnable_rust_doctests`

Invariant:
- `chunker_examples_doctests::example_and_doctest_chunks_have_source_backreferences`

Integration:
- `chunker_examples_doctests::fixture_workspace_emits_test_example_and_doctest_chunks`

**Red Phase (required before code changes)**

Command:
```bash
cargo test -p rarag-core --test chunker_examples_doctests -- --nocapture
```

Expected: failures showing missing example/doctest chunks.

**Implementation Steps**

1. Generalize workspace traversal beyond `root/src`.
2. Parse Rust doc comments and extract runnable fenced Rust blocks conservatively.
3. Emit stable chunk identities and parent links for extracted example/doctest chunks.
4. Avoid parsing non-Rust fixture content unnecessarily.

**Green Phase (required)**

Command:
```bash
cargo test -p rarag-core --test chunker_examples_doctests -- --nocapture
```

Expected: all example/doctest chunk tests pass.

**Completion Evidence**
- Preconditions satisfied
- Invariants preserved
- Postconditions met

## Phase 3: Rich Lexical Schema and Retrieval Use

### Task 4: Expand Tantivy Schema and Indexing Inputs

**Files:**
- Modify: `crates/rarag-core/src/indexing/tantivy_store.rs`
- Modify: `crates/rarag-core/src/indexing/mod.rs`
- Modify: `crates/rarag-core/tests/tantivy_schema_contract.rs`
- Modify: `crates/rarag-core/tests/index_pipeline.rs`

**Preconditions**
- Rich chunk metadata exists.

**Invariants**
- Every stored chunk still maps to exactly one lexical document per snapshot.
- Exact-symbol retrieval remains available and fast.

**Postconditions**
- Tantivy schema stores and indexes:
  - symbol path
  - symbol name
  - docs text
  - signature text
  - file path
  - chunk kind
  - test/example/doctest markers
  - repository-state hints where available
- Query parsing uses the richer lexical fields for hybrid candidate recall.

**Tests (must exist before implementation)**

Unit:
- `tantivy_schema_contract::maps_chunk_fields_to_rich_lexical_document`

Invariant:
- `index_pipeline::metadata_lexical_and_vector_counts_match`

Integration:
- `retrieval_modes::lexical_query_can_hit_docs_and_example_text`

**Red Phase (required before code changes)**

Command:
```bash
cargo test -p rarag-core --test tantivy_schema_contract --test index_pipeline --test retrieval_modes -- --nocapture
```

Expected: failures for missing fields and lexical retrieval over docs/example text.

**Implementation Steps**

1. Extend the Tantivy schema with the required lexical fields.
2. Map expanded chunk metadata into lexical documents.
3. Extend lexical query parsing to include the richer fields.
4. Keep snapshot filtering and exact-symbol lookup behavior intact.

**Green Phase (required)**

Command:
```bash
cargo test -p rarag-core --test tantivy_schema_contract --test index_pipeline --test retrieval_modes -- --nocapture
```

Expected: all lexical-schema and retrieval checks pass.

**Completion Evidence**
- Preconditions satisfied
- Invariants preserved
- Postconditions met

## Phase 4: Real MCP Transport Compatibility

### Task 5: Replace the Custom Socket Protocol with MCP-Compatible Transport

**Files:**
- Modify: `crates/rarag-mcp/src/main.rs`
- Modify: `crates/rarag-mcp/src/server.rs`
- Modify: `crates/rarag-mcp/src/tools.rs`
- Modify: `crates/rarag-core/tests/mcp_protocol.rs`
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`

**Preconditions**
- Tool surface is already named per spec.
- MCP protocol regression tests exist.

**Invariants**
- Tool names remain:
  - `rag_query`
  - `rag_symbol_context`
  - `rag_examples`
  - `rag_blast_radius`
  - `rag_index_status`
  - `rag_reindex`
- Unix-socket deployment remains local-first and scriptable.

**Postconditions**
- `rarag-mcp` accepts MCP-compatible initialization and tool-call flows from standard local MCP clients.
- Tool metadata and call shapes remain thin adapters over daemon requests.

**Tests (must exist before implementation)**

Unit:
- `mcp_protocol::tool_metadata_matches_spec_names`

Invariant:
- `mcp_protocol::initialize_then_list_tools_roundtrip`

Integration:
- `mcp_protocol::standard_client_can_initialize_and_call_rag_tools`

**Red Phase (required before code changes)**

Command:
```bash
cargo test -p rarag-core --test mcp_protocol -- --nocapture
```

Expected: failures showing the custom protocol is not MCP-compatible.

**Implementation Steps**

1. Introduce an MCP-compatible message model and request dispatch layer.
2. Map MCP tool calls to existing daemon requests without duplicating retrieval logic.
3. Preserve local Unix-socket startup, config loading, and socket hardening.
4. Keep any temporary compatibility shims explicitly documented and removable.

**Green Phase (required)**

Command:
```bash
cargo test -p rarag-core --test mcp_protocol -- --nocapture
```

Expected: MCP protocol tests pass against the local Unix-socket server.

**Completion Evidence**
- Preconditions satisfied
- Invariants preserved
- Postconditions met

## Phase 5: End-to-End Verification and Documentation Alignment

### Task 6: Verify the Full Compatibility Surface

**Files:**
- Modify: `docs/specs/repository-rag-architecture.md` (only if implementation changed accepted behavior)
- Modify: `README.md` (only if user-facing invocation examples change)
- Modify: `CHANGELOG.md`

**Preconditions**
- Tasks 1-5 are complete.

**Invariants**
- The spec remains canonical; implementation drift must be corrected, not normalized silently.
- Documentation examples must match the shipped interface exactly.

**Postconditions**
- `main` is verified against the spec for:
  - CLI surface
  - MCP surface
  - chunk/example/doctest indexing
  - lexical schema
  - socket/runtime safety defaults

**Tests (must exist before implementation)**

Unit:
- none

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`

Integration:
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `scripts/check-fast-feedback.sh`

**Green Phase (required)**

Command:
```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/check-fast-feedback.sh
```

Expected: clean pass on the full tree.

**Completion Evidence**
- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Full verification recorded
