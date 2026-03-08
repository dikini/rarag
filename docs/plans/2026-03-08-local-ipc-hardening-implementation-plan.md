# Local IPC Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Eliminate the current socket permission hazard plus the daemon and MCP unbounded-read denial-of-service issues.
Architecture: Keep the existing local Unix-socket topology and APIs, but harden the runtime in three isolated steps: safe socket-parent permissions, bounded daemon transport reads, and bounded MCP transport reads. Each task should land as its own TDD cycle and its own commit.
Tech Stack: Rust 1.93+, edition 2024, `tokio`, `serde_json`, local Unix sockets, and existing `rarag-core`, `raragd`, `rarag`, and `rarag-mcp` transport helpers.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless the user explicitly requests plan-only output.

## Output Contract

- Keep task definitions concrete: exact files, commands, and expected outcomes.
- Do not add new user-facing configuration for socket timeouts or request limits in this pass.
- Land one task per commit exactly as requested.

## Task Update Contract

- Any daemon transport framing change must update every local daemon caller and the related tests in the same task.
- Any MCP request-read hardening change must preserve the current tool contract and JSON-RPC behavior.
- Do not combine the three hardening tasks into one commit.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are all satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints must be restated in plain language for model-robust adherence.

---

### Task 1: Preserve Existing Socket Parent Permissions

**Files:**

- Modify: `crates/rarag-core/src/unix_socket.rs`
- Modify: `crates/rarag-core/tests/config_binary_entrypoints.rs`
- Test: `crates/rarag-core/src/unix_socket.rs`

**Preconditions**

- `prepare_socket_path` currently creates missing parent directories and tightens parent permissions unconditionally.

**Invariants**

- Newly created private runtime directories still become owner-only.
- Existing socket files are still removed only when they are actual sockets.
- Existing parent directories keep their prior mode unchanged.

**Postconditions**

- Socket startup no longer chmods arbitrary pre-existing parent directories.
- The runtime still hardens directories it creates itself.

**Tests (must exist before implementation)**

Unit:
- `unix_socket::tests::preserves_existing_parent_directory_permissions`
- `unix_socket::tests::tightens_only_newly_created_runtime_directory`

Invariant:
- `unix_socket::tests::rejects_non_socket_files`
- `unix_socket::tests::remove_socket_if_present_keeps_non_socket_files`

Integration:
- `config_binary_entrypoints::daemon_and_mcp_default_to_private_home_runtime_root_without_xdg_runtime`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core unix_socket::tests::preserves_existing_parent_directory_permissions unix_socket::tests::tightens_only_newly_created_runtime_directory -- --nocapture`
Expected: new permission tests fail until socket preparation distinguishes created directories from existing ones.

**Implementation Steps**

1. Add failing unit tests for existing-directory preservation and newly created-directory hardening.
2. Track whether the socket parent directory was created during `prepare_socket_path`.
3. Apply owner-only permissions only to directories created in that call.
4. Re-run unit and integration tests.

**Green Phase (required)**

Command: `cargo test -p rarag-core --lib --test config_binary_entrypoints -- --nocapture`
Expected: Unix-socket safety tests and related binary-entrypoint tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-core/src/unix_socket.rs`, related tests only
Re-run: `cargo test -p rarag-core --lib --test config_binary_entrypoints -- --nocapture`

**Commit**

Command:

```bash
git add crates/rarag-core/src/unix_socket.rs crates/rarag-core/tests/config_binary_entrypoints.rs
git commit -m "fix: preserve existing socket parent permissions"
```

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- Commit created with the required message

### Task 2: Bound Daemon Unix-Socket Request Reads

**Files:**

- Modify: `crates/raragd/src/transport.rs`
- Modify: `crates/rarag/src/client.rs`
- Modify: `crates/rarag-mcp/src/server.rs`
- Modify: `crates/rarag-core/tests/daemon_transport.rs`
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`

**Preconditions**

- Daemon request reads currently rely on `read_to_end` and EOF.

**Invariants**

- The daemon request/response API stays unary and JSON-based.
- CLI and MCP daemon-client paths stay compatible with the daemon after framing changes.
- Oversized or stalled clients fail fast instead of blocking unrelated requests indefinitely.

**Postconditions**

- Daemon request reads have an explicit bounded request boundary.
- Daemon request reads enforce a maximum size and read deadline.
- CLI and MCP daemon callers use the same daemon framing rules.

**Tests (must exist before implementation)**

Unit:
- `daemon_transport::rejects_oversized_requests`
- `daemon_transport::times_out_incomplete_requests`

Invariant:
- `daemon_transport::serializes_unix_socket_requests`
- `daemon_transport::requests_require_snapshot_or_unambiguous_worktree`

Integration:
- `daemon_transport::daemon_roundtrip_serves_query_payload`
- `daemon_cli_mcp::cli_and_mcp_roundtrip_against_local_daemon`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test daemon_transport --test daemon_cli_mcp -- --nocapture`
Expected: new oversized-request and incomplete-request tests fail under the EOF-delimited transport.

**Implementation Steps**

1. Add failing daemon transport tests for oversized and incomplete requests.
2. Introduce a bounded daemon request frame reader/writer with a read deadline.
3. Update CLI and MCP daemon-client request/response helpers to use the shared framing rules.
4. Re-run daemon transport and CLI/MCP integration tests.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test daemon_transport --test daemon_cli_mcp -- --nocapture`
Expected: daemon transport tests and CLI/MCP daemon roundtrip tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/raragd/src/transport.rs`, `crates/rarag/src/client.rs`, `crates/rarag-mcp/src/server.rs`, related daemon transport tests
Re-run: `cargo test -p rarag-core --test daemon_transport --test daemon_cli_mcp -- --nocapture`

**Commit**

Command:

```bash
git add crates/raragd/src/transport.rs crates/rarag/src/client.rs crates/rarag-mcp/src/server.rs crates/rarag-core/tests/daemon_transport.rs crates/rarag-core/tests/daemon_cli_mcp.rs
git commit -m "fix: harden daemon unix socket transport"
```

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- Commit created with the required message

### Task 3: Bound MCP Unix-Socket Request Reads

**Files:**

- Modify: `crates/rarag-mcp/src/server.rs`
- Modify: `crates/rarag-core/tests/mcp_protocol.rs`
- Modify: `crates/rarag-core/tests/daemon_cli_mcp.rs`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Preconditions**

- MCP inbound request handling still relies on `read_to_end` and EOF.

**Invariants**

- MCP tool names and payload contracts stay unchanged.
- Local MCP JSON-RPC compatibility remains intact.
- One stalled or oversized MCP client cannot hold the server indefinitely.

**Postconditions**

- MCP inbound request handling uses bounded reads with a deadline.
- MCP regression coverage includes oversized and incomplete client behavior.
- Operator-facing docs mention the hardened local IPC behavior without adding new config.

**Tests (must exist before implementation)**

Unit:
- `mcp_protocol::rejects_oversized_socket_request`
- `mcp_protocol::times_out_incomplete_socket_request`

Invariant:
- `mcp_protocol::standard_client_can_initialize_and_call_rag_tools`

Integration:
- `daemon_cli_mcp::cli_and_mcp_support_reload_config`
- `daemon_cli_mcp::cli_and_mcp_observe_same_snapshot_result`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `cargo test -p rarag-core --test mcp_protocol --test daemon_cli_mcp -- --nocapture`
Expected: new MCP oversized/incomplete request tests fail while legacy EOF-delimited reads are still in place.

**Implementation Steps**

1. Add failing MCP protocol tests for oversized and incomplete socket requests.
2. Replace inbound `read_to_end` usage in the MCP server with bounded request reading and a deadline.
3. Update docs and changelog for the hardened MCP runtime behavior.
4. Re-run MCP protocol and CLI/MCP integration tests.

**Green Phase (required)**

Command: `cargo test -p rarag-core --test mcp_protocol --test daemon_cli_mcp -- --nocapture`
Expected: MCP protocol and related integration tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/rarag-mcp/src/server.rs`, MCP protocol tests, and the directly related docs
Re-run: `cargo test -p rarag-core --test mcp_protocol --test daemon_cli_mcp -- --nocapture`

**Commit**

Command:

```bash
git add crates/rarag-mcp/src/server.rs crates/rarag-core/tests/mcp_protocol.rs crates/rarag-core/tests/daemon_cli_mcp.rs README.md CHANGELOG.md
git commit -m "fix: bound mcp unix socket request reads"
```

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- Commit created with the required message
