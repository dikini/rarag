# rarag Evidence Contract Template

Use this contract for prompts and skills that consume `rarag` retrieval results.

## Task Metadata

- `task_family`: `understand-symbol | doc-constrained-change | regression-archaeology | maintenance-safety`
- `query_mode`: `understand-symbol | implement-adjacent | bounded-refactor | find-examples | blast-radius`
- `scope`: snapshot id or worktree root used for retrieval

## Required Evidence Classes

- `code`: symbol/body/test/doctest chunks tied to implementation behavior
- `document`: spec/ops/integration/template/tasks/changelog chunks with source-kind markers
- `history` (optional unless archaeology task): history nodes and lineage edges selected with explicit history controls

## Retrieval Controls

- Always set a bounded `limit`.
- Use `symbol_path` when available.
- Use `include_history=true` only when historical reasoning is required.
- Set `history_max_nodes` to a bounded value when history is included.

## Ordering and Trust Policy

- For present behavior: prefer `spec` and `ops` evidence over `plan` and `changelog`.
- Treat `plan` evidence as forward-looking intent, not current truth.
- Treat `changelog` as historical summary, not normative contract.
- If evidence conflicts, state the conflict and identify which source class is normative.

## Citation Contract

- Every assertion must cite at least one retrieval item by file path and symbol/path id when present.
- Distinguish direct evidence from inference.
- If no sufficient evidence exists, return a bounded uncertainty statement and request targeted follow-up retrieval.

## Failure Handling

- If only distractor evidence is present, explicitly say retrieval quality is insufficient.
- Do not fabricate history causality; use confidence language for heuristics.
- Do not widen retrieval scope without stating why the current evidence is inadequate.

## Evaluation Hooks

- Include `eval_task_id` when replaying fixture tasks.
- Record evidence-class coverage (`code`, `document`, `history`) for each run.
- Report ideal/acceptable/distractor hit counts for template comparisons.
