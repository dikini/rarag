# Optimization Metrics Rubric Plan

Goal: Establish fixed metrics and acceptance rules for offline retrieval/template optimization proposals.
Architecture: Compare baseline and candidate runs on identical pinned fixtures. Promotion remains review-gated and is blocked by safety or distractor regressions.
Tech Stack: `rarag eval replay`, template-eval rubric, optimization proposal template, and rollout ops docs.
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

- Metric vocabulary must be stable across proposals.
- Rubric must include absolute metrics and delta reporting requirements.
- Safety regressions must override usefulness gains.

## Task Update Contract

- New proposal types must map to existing metric groups.
- If a metric cannot be measured, the proposal must be blocked.
- Do not approve candidates with unresolved safety regressions.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Publish Optimization Metrics Rubric

**Files:**

- Modify: `docs/plans/optimization-metrics-rubric.md`
- Create: `docs/templates/examples/rarag-optimization-proposal.md`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- Template eval rubric exists.
- Stage 1/Stage 2 artifacts are available for baseline/candidate replay.

**Invariants**

- Candidate evaluation remains offline and review-gated.
- Safety/boundedness remains hard-gated.

**Postconditions**

- Metrics rubric defines usefulness, relevance, efficiency, and safety/boundedness with scoring guidance.

**Tests (must exist before implementation)**

Unit:
- `doc-lint optimization-metrics-rubric`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until strict sections are present.

**Implementation Steps**

1. Define metric groups and acceptance/rejection guidance.
2. Define before/after comparison contract.
3. Align proposal template fields with rubric outputs.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/plans/optimization-metrics-rubric.md`, `docs/templates/examples/rarag-optimization-proposal.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
