# Offline Optimization Workflow Plan

Goal: Define a read-only, review-gated process that turns fixture replay traces into explicit optimization proposals.
Architecture: Keep optimization outside runtime retrieval. Evaluate one candidate at a time against baseline on pinned tasks; rollout is manual and rollback-ready.
Tech Stack: `rarag eval replay`, fixture JSON, metrics rubric, proposal template, daemon reload semantics, and ops rollout docs.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints.

## Output Contract

- Workflow must remain offline and review-gated.
- Workflow must require pinned revisions and baseline/candidate diffs.
- Workflow must link to rollout and rollback operations.

## Task Update Contract

- New candidate classes must define where changes live and how rollback works.
- Proposals without baseline/candidate comparison are invalid.
- Auto-apply behavior is out of scope.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Publish Offline Optimization Workflow

**Files:**

- Modify: `docs/plans/offline-optimization-workflow.md`
- Modify: `docs/ops/optimization-rollout.md`
- Modify: `README.md`
- Modify: `docs/ops/quickstart.md`
- Test: `scripts/check-fast-feedback.sh`

**Preconditions**

- Metrics rubric exists.
- Proposal template exists.

**Invariants**

- No automatic rollout of generated candidates.
- Rollback remains deterministic.

**Postconditions**

- Offline workflow steps, governance rules, and output artifacts are documented and linked from operator docs.

**Tests (must exist before implementation)**

Unit:
- `doc-lint offline-optimization-workflow`

Invariant:
- `fast-feedback documentation policy check`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fail or warn until workflow and rollout docs are added and linked.

**Implementation Steps**

1. Document offline workflow from baseline replay to proposal review.
2. Add rollout/rollback operator guidance.
3. Link workflow from README and quickstart.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: pass.

**Refactor Phase (optional but controlled)**

Allowed scope: workflow and ops docs
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
