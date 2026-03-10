# Config-Only Optimization Experiments Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Run offline retrieval optimization experiments with prebuilt `rarag` binaries only, preserving every candidate config and replay report while using selective diagnostics to guide later experiment cycles.
Architecture: Keep the optimization loop outside the runtime retrieval code. Drive all experiments through generated TOML configs and pinned fixture replay, then analyze trends between cycles to propose the next candidate set. Preserve full per-run reports under `docs/ops/optimization-runs/` and treat raw diagnostics as secondary evidence collected only when a run is interesting.
Tech Stack: `target/release/rarag`, `target/release/raragd`, fixture replay, shell automation in `scripts/`, Bats shell tests, and existing optimization docs in `docs/ops/` and `docs/plans/`.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design document's experiment, artifact, and analysis contracts.
4. Explicit updates recorded in this document.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints.

## Output Contract

- Use only prebuilt binaries rooted at `target/release/`.
- Preserve exact configs and replay reports for every experiment.
- Keep extra diagnostics selective and hypothesis-driven rather than exhaustive.

## Task Update Contract

- New experiment cycles must preserve the same pinned fixture basis within that run.
- Candidate configs may change TOML parameters only; code and binaries remain fixed.
- If a candidate cannot be reproduced from preserved artifacts, the run is invalid.

## Completion Gate

- The design is complete only when experiment structure, tuning scope, artifact policy, and analysis rules are explicit.
- Completion also requires documentation verification evidence.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

## Requirements

- Use only prebuilt binaries from `target/release/`.
- Do not modify Rust code or rebuild binaries during experiments.
- Optimize for higher `acceptable_task_hit_rate` and lower `distractor_task_hit_rate`.
- Run 3 cycles of 10 candidate experiments each.
- Preserve every run's config and replay report, not just deltas.
- Preserve extra diagnostics only when they help explain metric movement or anomalies.

## Workflow

1. Generate a baseline config and 10 candidate configs for the cycle.
2. Run a baseline replay against the pinned fixture worktree/revision.
3. Run 10 candidate replays against the same pinned fixture worktree/revision.
4. Store, per run:
   - exact TOML config
   - replay JSON
   - run metadata manifest
5. Store selective diagnostics only when:
   - `acceptable_task_hit_rate` or `distractor_task_hit_rate` changes
   - warnings appear or disappear
   - evidence coverage changes
   - result counts shift unexpectedly
   - the run is otherwise an outlier
6. Write a cycle analysis note with trend summaries and next-cycle hypotheses.

## Tuning Surface

- `[retrieval.rerank]` weights
- `[retrieval.neighborhood]` weights, if warranted by earlier cycle results
- `[[document_sources.rules]]` weights

No experiment may modify code, schema, fixture content, or binary inputs beyond config and daemon runtime paths.

## Artifact Layout

- `docs/ops/optimization-runs/<run-id>/`
- `docs/ops/optimization-runs/<run-id>/cycle-<n>/baseline/`
- `docs/ops/optimization-runs/<run-id>/cycle-<n>/experiment-<nn>/`
- `docs/ops/optimization-runs/<run-id>/cycle-<n>/analysis.md`

Each run directory stores enough information to reproduce a candidate replay without relying on mutable working files.

## Interesting Diagnostics Policy

The primary record is the replay JSON plus exact config. Selective diagnostics are attached only when they materially affect interpretation. This keeps the artifact set useful for later hypothesis generation without storing high-noise logs for all 30 experiments.

---

### Task 1: Define The Config-Only Experiment Contract

**Files:**

- Create: `docs/plans/2026-03-10-config-only-optimization-experiments-design.md`
- Create: `docs/plans/2026-03-10-config-only-optimization-experiments.md`
- Test: `scripts/check-fast-feedback.sh`

**Preconditions**

- Existing offline optimization docs and fixture replay CLI are available.
- Prebuilt binaries exist or will be validated before experiment execution.

**Invariants**

- Experiments remain config-only.
- Each cycle uses the same pinned fixture basis for baseline and candidates.
- Per-run artifacts preserve exact configs and replay reports.

**Postconditions**

- The workflow, artifact layout, and interesting-diagnostics policy are explicit.
- The design gives enough structure to execute three cycles without code changes.

**Tests (must exist before implementation)**

Unit:
- `doc-lint config-only optimization design`

Invariant:
- `doc-lint strict profile check`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fail until the design and implementation docs satisfy strict-profile requirements.

**Implementation Steps**

1. Record the approved config-only experiment constraints.
2. Define artifact layout and selective diagnostics rules.
3. Link execution expectations to the implementation plan.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: pass after the strict-profile sections are complete.

**Refactor Phase (optional but controlled)**

Allowed scope: design and plan docs only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
