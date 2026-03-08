# Rerank Observability Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Add configurable heuristic reranking plus opt-in retrieval observability so `rarag` can be tuned and evaluated without widening the repository-assistance query surface.
Architecture: Keep the existing retrieval pipeline and daemon request model mostly intact. Introduce shared TOML configuration for rerank weights and observability, add a safe daemon config reload path, and record retrieval observations through structured logs plus a persisted eval store that is disabled by default.
Tech Stack: Rust 1.93+, edition 2024, existing `serde`/`toml` config stack, Turso metadata store, `tokio`, Unix sockets, and the current retrieval/rerank pipeline in `rarag-core`.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design document's observability and reload contracts.
4. Explicit updates recorded in this document.

## Execution Mode

- Mode: plan-only
- Default: plan-only for this design document.

## Output Contract

- Keep retrieval and query contracts stable unless an admin surface is strictly necessary.
- Prefer internal configurability and observability over new user-facing retrieval features.
- Keep observability off by default and make it configurable only through shared TOML plus safe daemon reload.

## Task Update Contract

- New tuning or observability requirements must first update the shared config and storage contracts.
- Any new runtime surface must justify why signal-based reload alone is insufficient.
- Observation data shape must remain useful for offline eval generation and correlation with external agent logs.

## Completion Gate

- The design is complete only when rerank configurability, observation capture, and safe reload behavior are explicit.
- Completion also requires doc verification evidence and task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

## Design Summary

### Recommended Option

Use one shared opt-in observability and rerank-tuning layer.

- Add shared TOML config for:
  - heuristic rerank weights
  - neighborhood assembly weights
  - observability enablement and verbosity
- Keep retrieval requests and results unchanged.
- Add one daemon admin reload request and expose it through:
  - `SIGHUP`
  - `rarag daemon reload`
  - `rag_reload_config`
- When observability is enabled:
  - emit structured daemon logs for correlation with agent logs
  - persist query and candidate observation rows for offline evaluation

This keeps surface expansion minimal while enabling the tuning and measurement loop the project currently lacks.

### Why This Option

Logs alone are flexible but weak for later ranking experiments. A persisted store alone is useful for offline analysis but harder to correlate with agent behavior. The combined approach solves both:

- structured logs answer what happened during the live request
- persisted observations answer how the retrieval system behaved historically and whether rerank changes improved results

The combined approach is still small because it adds only:

- shared config
- one admin reload path
- internal observation capture

### Configuration Shape

Shared config additions:

- `[retrieval.rerank]`
- `[retrieval.neighborhood]`
- `[observability]`

Required observability fields:

- `enabled = false | true`
- `verbosity = "off" | "summary" | "detailed"`

Required reload semantics:

- config stays off by default
- reload reads the canonical config path already associated with the daemon process
- reload is validate-then-swap, never partially applied

### Observation Model

Structured daemon logs:

- one summary event per retrieval request when `summary` or `detailed`
- optional candidate-level events only when `detailed`
- stable correlation fields:
  - request id
  - timestamp
  - snapshot id
  - worktree root if known
  - query mode
  - query text
  - symbol hint
  - changed paths
  - warnings
  - returned chunk ids and ranks

Persisted eval store:

- keep existing `query_audits` for lightweight audit history
- add dedicated retrieval observation tables for eval-oriented data
- store enough information to reconstruct:
  - query inputs
  - merged candidate set
  - candidate feature values
  - final rank and score breakdown
  - returned result set

Candidate features should include only data already used by retrieval or reranking, for example:

- candidate source/evidence tags
- merged base score
- query-mode bias
- worktree-diff match and bias
- chunk kind
- retrieval markers
- file path
- symbol path
- semantic edge or neighborhood source tags

### Safe Reload Behavior

Reload must not degrade service.

- Requests already in progress complete under the config snapshot they started with.
- New requests use the new config only after validation succeeds.
- Failed reload leaves the old config active and returns a clear error.
- Reload may update:
  - observability enablement
  - observability verbosity
  - rerank weights
  - neighborhood weights
- Reload must not require the daemon to rebuild indexes or restart the process.

### Review Focus

Spec review:

- shared config owns tuning and observability controls
- reload is an admin control, not a retrieval feature
- observation capture is side-effect free

Operations review:

- observability is off by default
- reload is safe under concurrent requests
- failed reloads preserve service continuity

Evaluation review:

- observation records can generate eval sets later
- logged fields are enough to join with agent logs
- no extra LLM or learned-reranker work is required

### Task 1: Ratify Configurable Rerank and Observability Baseline

**Files:**

- Modify: `docs/specs/repository-rag-architecture.md`
- Create: `docs/plans/2026-03-08-rerank-observability-design.md`
- Create: `docs/plans/2026-03-08-rerank-observability-implementation-plan.md`
- Modify: `docs/tasks/tasks.csv`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- The repository RAG architecture spec already exists.
- The user approved opt-in observability, TOML configuration, and safe config reload.

**Invariants**

- Retrieval task/query surface stays unchanged apart from the admin reload path.
- Observability stays off by default.
- Reload remains validate-then-swap.

**Postconditions**

- The canonical spec records configurable reranking, observation capture, and safe reload behavior.
- A dedicated implementation plan exists for execution.
- The task registry records the new design/plan effort.

**Tests (must exist before implementation)**

Unit:
- `doc-lint rerank observability design header check`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the new design and plan documents satisfy the strict profile.

**Implementation Steps**

1. Extend the architecture spec with rerank tuning, observability, and reload contracts.
2. Record the approved design and review constraints.
3. Write the execution plan and register the work in `docs/tasks/tasks.csv`.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: all updated design and planning documents pass lint.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/repository-rag-architecture.md`, `docs/plans/2026-03-08-rerank-observability-design.md`, `docs/plans/2026-03-08-rerank-observability-implementation-plan.md`, `docs/tasks/tasks.csv`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- Task registry updated
