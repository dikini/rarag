# Local IPC Hardening Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Fix the newly identified Unix-socket safety and local IPC denial-of-service issues without expanding the user-facing runtime surface.
Architecture: Keep the existing local Unix-socket topology, but harden it in three places: preserve permissions on existing socket parent directories, bound daemon request reads with framing and deadlines, and apply the same bounded-read discipline to the MCP server. Reuse shared helpers where possible so daemon, CLI, MCP, and tests cannot drift.
Tech Stack: Rust 1.93+, edition 2024, existing `tokio` Unix sockets, `serde_json`, `std::os::unix::fs::PermissionsExt`, and the current `rarag-core` socket helpers.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design document's transport-hardening contracts.
4. Explicit updates recorded in this document.

## Execution Mode

- Mode: plan-only
- Default: plan-only for this design document.

## Output Contract

- Keep the daemon, CLI, and MCP feature surface unchanged.
- Prefer internal transport hardening and shared helpers over new configuration.
- Keep the fixes narrowly scoped to the review findings.

## Task Update Contract

- Any transport change must update both server and client-side code paths together.
- Any socket permission change must distinguish newly created directories from pre-existing directories.
- New limits or deadlines must remain internal constants unless a later spec explicitly expands configuration.

## Completion Gate

- The design is complete only when all three review findings are reflected in the canonical spec and in an executable implementation plan.
- Completion also requires doc verification evidence and task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

## Design Summary

### Recommended Option

Use one shared local IPC hardening pass with three focused implementation tasks.

- Preserve permissions only on directories `rarag` creates itself.
- Replace EOF-delimited daemon request reads with explicit bounded request framing plus read deadlines.
- Apply bounded request reads and deadlines to the MCP server without widening the MCP tool surface.

This is the smallest change that closes the review findings without introducing new config or protocol features that the project does not need yet.

### Why This Option

The review findings are operational and security-sensitive, not product-surface gaps.

- The socket permission bug is caused by startup behavior, not by missing user configuration.
- The daemon and MCP denial-of-service issues are caused by unbounded inbound reads and serial request handling.
- Adding more knobs would complicate the runtime before first release for little gain.

The right fix is therefore:

- narrow
- deterministic
- internal
- shared across binaries where drift would be risky

### Permission Hardening

`prepare_socket_path` should only apply owner-only permissions when it creates a missing runtime directory itself.

- If the parent directory already exists, leave its mode unchanged.
- If the parent directory is newly created during socket preparation, lock it down immediately.
- Continue refusing to remove non-socket paths at the socket path itself.

This prevents `rarag` from unexpectedly tightening `/tmp`, checked-in directories, or any other operator-managed socket parent.

### Daemon Transport Hardening

The daemon transport should stop reading requests with `read_to_end`.

Recommended contract:

- explicit request boundary
- bounded request size
- per-connection read deadline

The daemon side, CLI client, MCP-to-daemon client path, and transport tests should all use the same helper or framing contract. That keeps the internal daemon protocol coherent and prevents one caller from silently using weaker semantics than another.

### MCP Transport Hardening

The MCP server has the same unbounded inbound-read problem and should get the same class of fix:

- bounded inbound request size
- read deadline
- no reliance on peer EOF alone

This can be implemented without adding new MCP tools, flags, or config sections. The MCP request/response payloads stay the same; only the local socket read discipline changes.

### Commit Strategy

Implementation should land as three commits, one per task:

1. socket parent permission fix
2. daemon transport hardening
3. MCP transport hardening

That keeps the risk reviewable and makes regressions easier to bisect.

### Review Focus

Spec review:

- existing directories are never implicitly chmod'ed
- local IPC reads are bounded and time-limited
- no new user-facing config is added

Runtime review:

- a stalled or oversized local client cannot hold the daemon or MCP server indefinitely
- daemon/CLI/MCP transport code stays aligned

Testing review:

- new tests prove existing directory modes stay unchanged
- daemon transport tests cover framing, oversize rejection, and timeout behavior
- MCP tests cover oversize/stalled client handling without changing tool semantics

### Task 1: Ratify Socket and Local IPC Hardening Scope

**Files:**

- Modify: `docs/specs/repository-rag-architecture.md`
- Create: `docs/plans/2026-03-08-local-ipc-hardening-design.md`
- Create: `docs/plans/2026-03-08-local-ipc-hardening-implementation-plan.md`
- Modify: `docs/tasks/tasks.csv`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- The review findings are accepted as real defects.
- The user wants fix planning before code changes.

**Invariants**

- Canonical architecture remains the source of truth.
- The plan must keep future code changes split one task per commit.

**Postconditions**

- The architecture spec records the permission and bounded-read requirements.
- A dedicated implementation plan exists for the three hardening tasks.
- The task registry records this follow-up effort.

**Tests (must exist before implementation)**

Unit:
- `doc-lint local-ipc-hardening header check`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the new design and plan documents satisfy the strict profile.

**Implementation Steps**

1. Extend the architecture spec with runtime-directory and bounded-read invariants.
2. Record the recommended hardening approach in this design note.
3. Write the task-by-task implementation plan and register the work.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`
Expected: updated docs and registry checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/repository-rag-architecture.md`, `docs/plans/2026-03-08-local-ipc-hardening-*.md`, `docs/tasks/tasks.csv`
Re-run: `scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- Task registry updated
