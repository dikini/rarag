# Config-Only Optimization Experiments Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a config-only experiment harness and run three offline optimization cycles with prebuilt binaries, preserving full per-run replay artifacts and cycle analysis.

**Architecture:** The harness will generate isolated TOML configs under `docs/ops/optimization-runs/`, run baseline and candidate eval replay through `target/release/rarag` and `target/release/raragd`, and persist structured artifacts for later review. The implementation stays outside Rust runtime code and uses shell automation plus Bats tests to keep the workflow reproducible.

**Tech Stack:** Bash, Bats, `target/release/rarag`, `target/release/raragd`, TOML config files, JSON replay reports, and existing repo docs/check scripts.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints.

## Output Contract

- Use prebuilt binaries only; do not rebuild during experiment execution.
- Preserve baseline and candidate reports for every experiment.
- Capture selective diagnostics only when they materially affect analysis.

## Task Update Contract

- Candidate generation must remain config-only.
- Experiment outputs must remain grouped by run id, cycle, and experiment id.
- If an experiment fails, preserve the failure evidence and continue unless baseline validity is compromised.

## Completion Gate

- The task is complete only when docs, harness, verification, and all three experiment cycles are recorded.
- Completion requires fresh verification evidence and artifact completeness checks.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Document The Approved Workflow

**Files:**
- Create: `docs/plans/2026-03-10-config-only-optimization-experiments-design.md`
- Create: `docs/plans/2026-03-10-config-only-optimization-experiments.md`
- Modify: `CHANGELOG.md`

**Preconditions**

- The experiment design is approved.
- Existing optimization docs and replay tooling are present.

**Invariants**

- Documentation must reflect config-only execution with prebuilt binaries.
- Task-completion workflow must remain traceable in repo docs and changelog.

**Postconditions**

- The design and implementation plan exist in strict-profile format.
- The changelog records the new experiment harness workflow support.

**Tests (must exist before implementation)**

Unit:
- `doc-lint config-only experiment docs`

Invariant:
- `doc-lint strict profile check`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fail until the new docs satisfy the strict-profile format.

**Step 1: Record the approved design and implementation plan**

Write the design and plan documents with the config-only, prebuilt-binary constraints and the 3x10 cycle structure.

**Step 2: Update the changelog entry**

Add a Common Changelog entry for the experiment harness and offline optimization workflow execution support.

**Step 3: Run the fast documentation check**

Run: `scripts/check-fast-feedback.sh`
Expected: pass with the new docs in place.

**Implementation Steps**

1. Record the approved design and plan in strict-profile format.
2. Update `CHANGELOG.md`.
3. Run fast feedback and keep the docs green before adding the harness.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: pass.

**Refactor Phase (optional but controlled)**

Allowed scope: docs and changelog only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Add Harness Tests First

**Files:**
- Create: `scripts/tests/test-run-optimization-experiments.bats`
- Modify: `scripts/run-shell-tests.sh`

**Preconditions**

- The implementation plan exists.
- Shell test infrastructure already runs through Bats.

**Invariants**

- Tests must describe config-only harness behavior without depending on a rebuild.
- Red phase must fail for the right reason: missing harness behavior.

**Postconditions**

- Shell tests cover artifact creation, command invocation, interesting-data capture, and summaries.

**Tests (must exist before implementation)**

Unit:
- `scripts/tests/test-run-optimization-experiments.bats`

Invariant:
- `scripts/run-shell-tests.sh --changed`

Integration:
- `scripts/run-shell-tests.sh --changed`

Property-based (optional):
- none

**Step 1: Write failing shell tests**

Add Bats coverage for:
- generated artifact layout
- baseline plus candidate command invocation
- selective interesting-data capture
- cycle summary generation

**Step 2: Run the test file and verify it fails**

Run: `scripts/run-shell-tests.sh --changed`
Expected: fail because the harness script does not exist yet.

**Red Phase (required before code changes)**

Command: `scripts/run-shell-tests.sh --changed`
Expected: fail because the new harness script does not exist yet.

**Implementation Steps**

1. Add a new Bats file that stubs the prebuilt binaries.
2. Assert baseline and candidate artifact creation.
3. Assert selective interesting-data capture and summary output.

**Green Phase (required)**

Command: `scripts/run-shell-tests.sh --changed`
Expected: pass after the harness exists.

**Refactor Phase (optional but controlled)**

Allowed scope: shell tests only
Re-run: `scripts/run-shell-tests.sh --changed`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Implement The Harness And Ignore Rules

**Files:**
- Modify: `.gitignore`
- Create: `scripts/run-optimization-experiments.sh`

**Preconditions**

- Harness tests exist and fail for missing behavior.
- The artifact target directory is `docs/ops/optimization-runs/`.

**Invariants**

- The harness must use `target/release/` binaries only.
- The harness must not rebuild or edit Rust source files.
- Every experiment must preserve config, replay report, and manifest.

**Postconditions**

- The harness can run 3 cycles of 10 experiments with preserved artifacts.
- Generated artifacts under `docs/ops/optimization-runs/` are gitignored.

**Tests (must exist before implementation)**

Unit:
- `scripts/tests/test-run-optimization-experiments.bats`

Invariant:
- `scripts/run-shell-tests.sh --changed`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Step 1: Write the minimal harness**

Implement a shell script that:
- accepts prebuilt binary paths rooted at `target/release`
- generates configs for baseline and candidates
- runs baseline plus 10 candidate experiments per cycle
- stores config, replay, manifest, and selective diagnostics under `docs/ops/optimization-runs/`
- writes per-cycle analysis inputs and summaries

**Step 2: Re-run shell tests and verify green**

Run: `scripts/run-shell-tests.sh --changed`
Expected: pass with the new harness in place.

**Red Phase (required before code changes)**

Command: `scripts/run-shell-tests.sh --changed`
Expected: fail until the harness satisfies the new tests.

**Implementation Steps**

1. Add gitignore rules for generated optimization artifacts.
2. Implement baseline and candidate execution, artifact preservation, and summary generation.
3. Implement selective diagnostics capture for interesting runs only.

**Green Phase (required)**

Command: `scripts/run-shell-tests.sh --changed`
Expected: pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `.gitignore`, harness script
Re-run: `scripts/run-shell-tests.sh --changed`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 4: Verify Policy Checks

**Files:**
- No code changes expected

**Preconditions**

- Docs, tests, and harness are in place.

**Invariants**

- Verification must run on the current tree state.

**Postconditions**

- Fast feedback passes before experiment execution.

**Tests (must exist before implementation)**

Unit:
- none

Invariant:
- `scripts/check-fast-feedback.sh`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Step 1: Run fast feedback**

Run: `scripts/check-fast-feedback.sh`
Expected: pass.

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fail until the changed tree satisfies policy checks.

**Implementation Steps**

1. Run fast feedback on the current tree.
2. Fix any policy or documentation issues before experiments.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: pass.

**Refactor Phase (optional but controlled)**

Allowed scope: any changed file required to satisfy checks
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Invariant and integration checks passing
- CHANGELOG.md updated

### Task 5: Execute The Optimization Cycles

**Files:**
- Create: `docs/ops/optimization-runs/<run-id>/**`

**Preconditions**

- Prebuilt binaries exist in `target/release/`.
- Fast feedback has passed on the harness changes.

**Invariants**

- Each cycle must preserve one baseline and ten candidate runs.
- Analysis for each later cycle must cite the prior cycle's evidence.
- Preserve failure evidence instead of silently overwriting or skipping it.

**Postconditions**

- Three complete cycles of preserved experiment artifacts exist.
- Cycle analyses explain trends and next-cycle hypotheses.

**Tests (must exist before implementation)**

Unit:
- `artifact completeness sanity check`

Invariant:
- `target/release/rarag eval replay` outputs preserved JSON for each run

Integration:
- harness execution across 3 cycles x 10 experiments

Property-based (optional):
- none

**Step 1: Run cycle 1 baseline and 10 candidate experiments**

Run the harness with prebuilt binaries and save every config and replay report.

**Step 2: Analyze cycle 1 and derive cycle 2**

Summarize metric trends and hypothesis-driven config shifts.

**Step 3: Repeat for cycles 2 and 3**

Preserve full per-run outputs and selective diagnostics for interesting runs.

**Step 4: Verify artifact completeness**

Run a focused artifact sanity check over the generated run directory.
Expected: each cycle contains one baseline, ten experiments, replay reports, configs, and analysis.

**Red Phase (required before code changes)**

Command: `test -x target/release/rarag -a -x target/release/raragd`
Expected: pass before execution; otherwise the run is blocked.

**Implementation Steps**

1. Run cycle 1 and preserve artifacts.
2. Analyze cycle 1 and derive cycle 2 candidates.
3. Repeat for cycles 2 and 3.
4. Validate artifact completeness at the end.

**Green Phase (required)**

Command: `find docs/ops/optimization-runs/<run-id> -maxdepth 3 -type f | sort`
Expected: baseline, 30 candidate experiment artifacts, and three analysis notes are present.

**Refactor Phase (optional but controlled)**

Allowed scope: generated optimization artifacts and analysis notes only
Re-run: artifact completeness sanity check

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
