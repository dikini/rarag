# Document, History, and Evaluation Foundations Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Extend `rarag` with first-class document semantics, bounded temporal/causal repository history retrieval, and task-based usefulness evaluation as the foundation for later prompt/template and optimization work.
Architecture: Keep the existing snapshot-scoped repository retrieval core, then add two new retrievable graph families alongside the Rust code graph: a document graph for repository knowledge sources and a history graph for bounded change/lineage evidence. Reuse the existing observability substrate to evaluate mixed evidence retrieval before adding derived prompt or optimization layers.
Tech Stack: Rust 1.93+, edition 2024, existing `rarag-core` chunking/retrieval stack, Turso metadata store, Tantivy lexical index, LanceDB vector store, git CLI/lib integration, and shared TOML config plus daemon reload semantics.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design document's graph, retrieval, and evaluation contracts.
4. Explicit updates recorded in this document.

## Execution Mode

- Mode: plan-only
- Default: plan-only for this design document.

## Output Contract

- Keep document, history, and evaluation work integrated into one retrieval architecture rather than three disconnected subsystems.
- Preserve bounded repository assistance; avoid turning `rarag` into an unbounded knowledge dump or generic git archaeology assistant.
- Treat prompt/skill templates and adaptive optimization as downstream consumers of these foundations, not peer architecture layers.

## Task Update Contract

- New requirements must first map to one of the three foundation layers: document graph, history graph, or evaluation substrate.
- If a requirement primarily affects prompting or optimization, defer it unless the necessary foundation contract is missing here.
- Any new retrieval surface must justify why it cannot be expressed through the current query-mode and evidence model.

## Completion Gate

- The design is complete only when document semantics, history/causal evidence, retrieval fusion, evaluation tasks, and staged follow-on work are explicit.
- Completion also requires doc verification evidence and task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

## Design Summary

### Recommended Option

Use a three-foundation architecture with two deferred consumers.

- Foundation 1: document graph
- Foundation 2: temporal/causal history graph
- Foundation 3: task-based retrieval evaluation
- Deferred consumer A: prompt and skill templates built from Stage 1 evidence contracts and eval fixtures
- Deferred consumer B: offline, review-gated optimization of retrieval weights, prompts, skills, and templates using persisted eval traces

This option matches the repo's current shape. `rarag` already has:

- snapshot-scoped code retrieval
- query-mode-aware reranking
- opt-in structured retrieval observations

The missing capability is not raw vector quality. The missing capability is richer evidence selection across code, docs, and history, plus a disciplined way to measure whether the system actually helps real repository tasks.

### Foundation 1: Document Graph

Treat repository knowledge files as structured evidence, not generic text blobs.

Required object families:

- `Document`
  - path, kind, title, status, last-modified commit, checksum
- `DocumentBlock`
  - heading path, block type, normalized text, source span
- `DocumentSemantic`
  - intent labels, normativity, lifecycle, confidence
- `DocumentRef`
  - typed references to symbols, files, config keys, commands, tests, and history objects when resolvable

Design rules:

- Chunk Markdown by heading and block boundaries, not fixed token windows.
- Chunk supported CSV knowledge sources by row boundaries, not file-wide blobs.
- Current repo layout drives the default classes and order:
  - `docs/specs/**` receives strong normative prior
  - `docs/plans/**` is future-facing and must not outrank current specs when the question is about present behavior
  - `docs/ops/**` and `docs/integrations/**` rank strongly for commands, configuration, service behavior, and troubleshooting
  - `CHANGELOG.md` is historical summary evidence, not a substitute for current normative behavior
  - `docs/tasks/tasks.csv` is a structured repository-governance source and should be retrievable as row-scoped documentation evidence
- Those classes, parsers, and weights must be defaults, not hardwired assumptions.
- Shared TOML must allow overrides for:
  - path classification
  - parser selection such as `markdown` or `csv`
  - document kind
  - semantic priors
  - retrieval weighting within bounded ranges

Integration point:

- Document blocks join the same candidate merge and rerank pipeline as code chunks.
- They are not a separate retrieval UI unless later evidence shows that mixed retrieval is insufficient.

### Foundation 2: Temporal and Causal History Graph

Treat snapshot identity as the temporal baseline, then add bounded history objects on top.

Required object families:

- `HistoryNode`
  - commit, change summary, diff summary, symbol-history summary
- `LineageEdge`
  - rename, move, split, merge, follow-up, revert, fix, dependency, invariant-introduction
- `HistoryRef`
  - explicit selector by commit, tag, time window, or bounded comparison range

Design rules:

- Start from explicit git evidence and typed derivation rules.
- Keep inferred causal edges evidence-backed and confidence-scored.
- Prefer symbol/path lineage over full free-form commit search when possible.
- Keep history retrieval bounded to the caller's explicit target plus a constrained neighborhood of related changes.

Integration point:

- Present-state retrieval remains snapshot-scoped by default.
- Historical cross-snapshot retrieval happens only when the caller explicitly requests it.
- History nodes participate in candidate merge and rerank alongside code and docs, with history-locality and query-mode-specific priors.

### Foundation 3: Task-Based Evaluation

Evaluate repository usefulness, not only generic search relevance.

Required task fixture shape:

- task prompt
- repository revision or snapshot selector
- ideal evidence set
- acceptable evidence set
- distractor set
- expected evidence-class coverage

Required metrics:

- `hit@1`, `hit@3`, `hit@5`
- full evidence-class coverage
- mean rank of first decisive item
- noise ratio
- context efficiency
- answerability
- sufficiency
- actionability

Required failure taxonomy:

- candidate-generation failure
- ranking failure
- granularity failure
- fusion failure
- boundedness failure

Integration point:

- Reuse the existing opt-in observation pipeline and persisted candidate features.
- Extend it so traces are useful for offline evaluation reports and later prompt/template optimization.
- Keep evaluation separate from live retrieval semantics; traces must not alter ranking outputs.

### Retrieval Fusion Model

Use one merged candidate pipeline, not three retrieval silos.

Candidate families:

- code chunks
- document blocks
- history nodes

Shared rerank signals:

- exact symbol/path match
- query-mode prior
- diff/worktree locality
- document-kind prior
- normativity prior
- history-locality prior
- evidence-class diversity
- budget-aware decisiveness

Assembly rules:

- Prefer decisive mixed evidence over large homogeneous neighborhoods.
- For present-state behavioral questions, current spec/runbook evidence outranks plans and changelog summaries.
- For regression archaeology, recent change evidence and follow-up chains outrank static background docs unless those docs define the broken invariant.

### CLI and MCP Surface Direction

Keep the surface compact in Stage 1.

Preferred path:

- extend existing `query` and MCP retrieval flows with optional evidence/history selectors where possible
- add dedicated read-only history/document operations only when mixed retrieval proves insufficient for explainability or tooling ergonomics

This avoids prematurely committing to a large MCP surface before evaluation shows which operations are actually necessary.

### Staging

Stage 1:

- document graph
- history graph
- evaluation substrate
- document defaults shipped for the current repo structure plus explicit TOML override support

Stage 2:

- prompt and skill templates derived from Stage 1 evidence contracts and task fixtures

Stage 3:

- offline, review-gated optimization of retrieval weights, prompts, skills, and templates

The ordering is intentional:

- no template work before evidence classes and task fixtures exist
- no optimization loop before retrieval traces and evaluation baselines exist

### Review Focus

Architecture review:

- one retrieval architecture, not parallel subsystems
- bounded evidence remains central
- present-state and historical retrieval stay explicitly distinguishable

Robustness review:

- inferred causal edges expose uncertainty
- plans do not outrank specs for current behavior
- evaluation traces remain side-effect free

Program review:

- Stage 2 and Stage 3 are explicitly blocked on Stage 1 outputs
- optimization remains offline and review-gated by default

### Task 1: Ratify the Foundation Program

**Files:**

- Modify: `docs/specs/repository-rag-architecture.md`
- Create: `docs/plans/2026-03-09-doc-history-eval-foundations-design.md`
- Create: `docs/plans/2026-03-09-doc-history-eval-foundations-implementation-plan.md`
- Create: `docs/plans/2026-03-09-prompt-skill-templates-implementation-plan.md`
- Create: `docs/plans/2026-03-09-retrieval-optimization-loop-implementation-plan.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`
- Test: `scripts/doc-lint.sh --changed --strict-new`

**Preconditions**

- The canonical repository architecture spec exists.
- The shared conversation and current repo docs have been reviewed.
- The user approved the staged dependency order: foundations first, then templates, then optimization.

**Invariants**

- Foundations remain independent of later prompt/optimization policy.
- Stage 2 depends on Stage 1 outputs.
- Stage 3 depends on Stage 1 evaluation traces and Stage 2 template contracts.

**Postconditions**

- The canonical architecture spec reflects document and history graph extensions plus task-based evaluation.
- The canonical architecture spec also reflects config-overridable document classes and structured-file support.
- A dedicated Stage 1 implementation plan exists.
- Deferred Stage 2 and Stage 3 plans exist with explicit dependency sequencing.

**Tests (must exist before implementation)**

Unit:
- `doc-lint foundations design header check`

Invariant:
- `doc-lint strict profile check`

Integration:
- `fast-feedback documentation policy check`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fail until the new design and plan documents satisfy the strict profile and registry requirements.

**Implementation Steps**

1. Extend the canonical architecture spec with document graph, history graph, and evaluation contracts.
2. Capture the integrated design and staged downstream consumers in this design doc.
3. Write one Stage 1 implementation plan and two explicitly blocked downstream plans.
4. Update the task registry and changelog.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: all new and modified docs pass strict policy checks.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/repository-rag-architecture.md`, `docs/plans/2026-03-09-*.md`, `docs/tasks/tasks.csv`, `CHANGELOG.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
