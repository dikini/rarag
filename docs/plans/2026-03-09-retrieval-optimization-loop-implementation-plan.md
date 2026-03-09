# Retrieval Optimization Loop Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Build a long-term offline optimization loop for `rarag` retrieval parameters, prompts, skills, and templates using Stage 1 evaluation traces and Stage 2 template artifacts.
Architecture: Keep optimization offline, reproducible, and review-gated. Use persisted retrieval traces, curated task fixtures, and template-eval results to generate candidate config or template changes, then require explicit human approval before rollout.
Tech Stack: Existing shared TOML config, Stage 1 observation store, Stage 1 eval fixtures, Stage 2 prompt/skill templates, documentation/report generation, and existing daemon config reload semantics for approved changes.
Template-Profile: tdd-strict-v1

Task Registry ID: `2026-03-09-retrieval-optimization-loop`

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: plan-only
- Default: plan-only because optimization depends on future implemented stages.

## Output Contract

- Keep optimization offline and review-gated by default.
- Optimize for repository usefulness metrics, not only raw recall or runtime speed.
- Never grant the system silent authority to rewrite prompts or weights in production.

## Task Update Contract

- Stage 3 remains blocked until Stage 1 evaluation traces and Stage 2 template contracts exist.
- Any proposed optimization must cite the metric deltas and eval tasks it intends to improve.
- If automated proposals cannot explain trade-offs in boundedness or distractor risk, they must not be promoted.

## Completion Gate

- A task is complete only when preconditions, invariants, postconditions, and listed tests are satisfied.
- Plan completion requires explicit governance, rollback, and verification contracts.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Define the Experiment and Proposal Schema

**Files:**

- Create: `docs/templates/examples/rarag-optimization-proposal.md`
- Create: `docs/plans/optimization-metrics-rubric.md`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- Stage 1 metrics include usefulness and boundedness signals.
- Stage 2 templates are represented as explicit artifacts, not hidden behavior.

**Invariants**

- Every optimization candidate is traceable to eval evidence.
- Proposal docs distinguish retrieval-weight changes from prompt/template changes.
- Approval remains explicit and manual.

**Postconditions**

- A standard optimization proposal schema exists for retrieval and template candidates.
- Metric reporting vocabulary is fixed before any automation is attempted.

**Tests (must exist before implementation)**

Unit:
- `doc-lint optimization proposal template`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the optimization proposal and metric rubric docs exist.

**Implementation Steps**

1. Define the proposal schema for retrieval and prompt/template changes.
2. Define the metrics rubric that candidates must report against.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: optimization proposal docs pass lint.

**Refactor Phase (optional but controlled)**

Allowed scope: optimization proposal docs
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Build an Offline Candidate Evaluation Workflow

**Files:**

- Create: `docs/plans/offline-optimization-workflow.md`
- Modify: `README.md`
- Test: `scripts/check-fast-feedback.sh`

**Preconditions**

- Experiment schema exists.
- Stage 1 and Stage 2 artifacts are available to compare candidate variants.

**Invariants**

- Offline evaluation uses pinned revisions and explicit task fixtures.
- Candidate promotion requires before/after metric comparison.
- Boundedness regressions block otherwise attractive recall gains.

**Postconditions**

- A documented offline workflow exists for generating and reviewing retrieval/template candidates.
- Contributors know where approved changes feed back into config or template artifacts.

**Tests (must exist before implementation)**

Unit:
- `doc-lint offline optimization workflow`

Invariant:
- `fast-feedback documentation policy check`

Integration:
- `check-fast-feedback current-tree marker`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fail or warn until the new workflow docs are present.

**Implementation Steps**

1. Document the offline optimization workflow from trace capture to proposal review.
2. Update top-level docs to point to the review-gated optimization process.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: fast feedback passes on the updated docs tree.

**Refactor Phase (optional but controlled)**

Allowed scope: workflow docs and README references
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Define Rollout, Approval, and Rollback Controls

**Files:**

- Create: `docs/ops/optimization-rollout.md`
- Modify: `docs/ops/quickstart.md`
- Test: `scripts/check-fast-feedback.sh`

**Preconditions**

- Offline workflow exists.
- The project has a clear distinction between approved config/template artifacts and candidate experiments.

**Invariants**

- No automatic rollout of generated candidates occurs without human approval.
- Rollback remains deterministic for both config changes and template changes.
- Approved retrieval config changes continue to use existing daemon reload semantics.

**Postconditions**

- Operations docs explain how approved optimization changes are applied and rolled back safely.
- Long-term optimization remains an operations-governed process, not hidden automation.

**Tests (must exist before implementation)**

Unit:
- `doc-lint optimization rollout docs`

Invariant:
- `fast-feedback documentation policy check`

Integration:
- `check-fast-feedback current-tree marker`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fail or warn until rollout/rollback docs are present.

**Implementation Steps**

1. Document approval gates, rollout steps, and rollback steps for approved optimization candidates.
2. Link the process from operator-facing docs where config reload and retrieval tuning already exist.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: fast feedback passes on the updated docs tree.

**Refactor Phase (optional but controlled)**

Allowed scope: ops docs and quickstart references
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
