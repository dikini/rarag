# Template Evaluation Rubric Plan

Goal: Define a reproducible rubric for comparing Stage 2 prompt/skill template variants using Stage 1 fixtures and observations.
Architecture: Keep template evaluation offline and fixture-driven. Reuse retrieval observation fields (`eval_task_id`, evidence-class coverage) so comparisons are tied to repository-task usefulness.
Tech Stack: `rarag eval replay`, `tests/fixtures/eval/tasks.json`, template artifacts in `docs/templates/`, and retrieval observations.
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

- Rubric must define usefulness, relevance, and efficiency metrics.
- Rubric must include explicit pass/fail gates for distractor and boundedness regressions.
- Rubric must define reproducibility requirements.

## Task Update Contract

- Any new metric must be mapped to one of usefulness, relevance, efficiency, or safety/boundedness.
- If a metric cannot be measured with current replay outputs, document the gap.
- Do not relax distractor/boundedness gates without explicit approval.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Publish Template Evaluation Rubric

**Files:**

- Modify: `docs/plans/template-eval-rubric.md`
- Modify: `README.md`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- Stage 1 fixture replay exists.
- Stage 2 template artifacts are present.

**Invariants**

- Evaluation remains offline and reproducible.
- Distractor and boundedness regressions remain blocking failures.

**Postconditions**

- Rubric defines metric groups, pass/fail gates, reporting shape, and reproducibility contract.

**Tests (must exist before implementation)**

Unit:
- `doc-lint template-eval-rubric`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until strict sections and rubric content are present.

**Implementation Steps**

1. Add rubric metric definitions for usefulness, relevance, efficiency, and safety/boundedness.
2. Add pass/fail gate language for distractor and boundedness regressions.
3. Add reproducibility and reporting requirements.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/plans/template-eval-rubric.md`, `README.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
