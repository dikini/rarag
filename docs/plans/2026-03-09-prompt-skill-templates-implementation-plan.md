# Prompt and Skill Templates Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Create prompt and skill templates that consume Stage 1 document/history/evaluation outputs so agents can use `rarag` evidence more consistently for development, debugging, and maintenance tasks.
Architecture: Keep prompts and skills outside the runtime retrieval contract. Build template artifacts around Stage 1 evidence classes, history selectors, and evaluation tasks so template quality is measurable rather than anecdotal.
Tech Stack: Repository docs/templates workflow, existing prompt template examples, agent-facing documentation, Stage 1 eval fixtures and retrieval traces, and MCP/CLI integration docs.
Template-Profile: tdd-strict-v1

Task Registry ID: `2026-03-09-prompt-skill-templates`

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: plan-only
- Default: plan-only because Stage 1 is not implemented yet.

## Output Contract

- Derive templates from observed evidence contracts and eval tasks, not from abstract “good prompts”.
- Keep templates reproducible and testable.
- Do not introduce runtime self-prompting or hidden orchestration logic.

## Task Update Contract

- Stage 2 must remain blocked until Stage 1 produces document/history evidence and evaluation fixtures.
- Template changes must cite the Stage 1 task types and evidence classes they rely on.
- If a proposed skill/template cannot be evaluated, document the gap instead of hand-waving quality.

## Completion Gate

- A task is complete only when preconditions, invariants, postconditions, and listed tests are satisfied.
- Plan completion requires explicit dependency sequencing and verification evidence.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Define the Template Contract

**Files:**

- Modify: `docs/templates/README.md`
- Modify: `docs/templates/examples/prompt-contract-minimal.md`
- Create: `docs/templates/examples/rarag-evidence-contract.md`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- Stage 1 retrieval outputs include explicit evidence classes and history selectors.
- Eval fixtures define development, debugging, and maintenance task families.

**Invariants**

- Template contracts stay external to runtime retrieval.
- Templates remain explicit about required evidence and allowed uncertainty.
- Current strict doc template profile remains intact.

**Postconditions**

- A shared template contract exists for `rarag`-backed prompts and skills.
- Templates clearly specify required evidence classes, preferred ordering, and citation expectations.

**Tests (must exist before implementation)**

Unit:
- `doc-lint template contract header check`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the new template contract documents exist and lint cleanly.

**Implementation Steps**

1. Write a reusable evidence contract template based on Stage 1 retrieval outputs.
2. Align existing prompt examples with explicit evidence and uncertainty requirements.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: template docs pass strict lint.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/templates/**`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Add Task-Specific Prompt and Skill Template Bundles

**Files:**

- Create: `docs/templates/prompts/rarag-understand-symbol.prompt.txt`
- Create: `docs/templates/prompts/rarag-doc-constrained-change.prompt.txt`
- Create: `docs/templates/prompts/rarag-regression-archaeology.prompt.txt`
- Create: `docs/templates/prompts/rarag-maintenance-safety.prompt.txt`
- Create: `docs/templates/examples/rarag-skill-template.md`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- Shared template contract exists.
- Stage 1 task families and evidence classes are stable enough to target.

**Invariants**

- Templates ask for bounded evidence, not open-ended dump-everything retrieval.
- Historical reasoning templates require explicit history selectors or uncertainty language.
- Prompt and skill templates remain aligned with the same evidence contract.

**Postconditions**

- `rarag` has first-class prompt/skill templates for core repository task types.
- Templates encode evidence ordering and failure-handling expectations consistently.

**Tests (must exist before implementation)**

Unit:
- `doc-lint prompt template files`

Invariant:
- `doc-lint template examples preserve evidence contract sections`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the new prompt/skill template bundle exists and lints.

**Implementation Steps**

1. Write one template per high-value task family.
2. Add one skill-template example that mirrors the prompt evidence contract.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: template bundle passes lint.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/templates/**`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Evaluate Template Variants Against Stage 1 Fixtures

**Files:**

- Create: `docs/plans/template-eval-rubric.md`
- Modify: `README.md`
- Test: `scripts/check-fast-feedback.sh`

**Preconditions**

- Stage 1 eval fixtures and traces exist.
- Task-specific templates exist.

**Invariants**

- Template evaluation remains tied to repository tasks, not generic stylistic preference.
- Results remain reproducible at pinned revisions.
- No template is promoted without explicit evaluation notes.

**Postconditions**

- A documented rubric exists for comparing prompt/skill template variants against Stage 1 task fixtures.
- Template work is positioned as an evaluated consumer of retrieval behavior.

**Tests (must exist before implementation)**

Unit:
- `doc-lint template eval rubric`

Invariant:
- `fast-feedback documentation policy check`

Integration:
- `check-fast-feedback current-tree marker`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fail or warn until the new rubric/docs are added to the current tree.

**Implementation Steps**

1. Document the template-eval rubric and required task families.
2. Update top-level docs to point future contributors to the evaluated template workflow.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: fast feedback passes on the updated documentation tree.

**Refactor Phase (optional but controlled)**

Allowed scope: template docs and README-level references
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
