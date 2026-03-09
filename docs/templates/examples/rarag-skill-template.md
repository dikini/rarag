# rarag Skill Template

Use this template for agent skills that consume `rarag` retrieval evidence.

## Skill Purpose

- Describe the repository task family this skill handles.
- State expected query mode(s) and when to include history.

## Evidence Contract

- Required evidence classes: `code`, `document`, `history` (if applicable)
- Normative precedence:
  1. `docs/specs/**`
  2. `docs/ops/**`, `docs/integrations/**`
  3. code/tests evidence
  4. `docs/plans/**`
  5. `CHANGELOG.md`

## Retrieval Contract

- Require bounded `limit`.
- Require `symbol_path` when available.
- Require explicit `include_history` + `history_max_nodes` for historical tasks.
- Attach `eval_task_id` during fixture replay.

## Output Contract

1. Findings and recommendations
2. Evidence citations (path + symbol/id where available)
3. Uncertainty and follow-up retrieval needs

## Verification Contract

- Define fixture tasks to replay.
- Record ideal/acceptable/distractor hits.
- Record evidence-class coverage.
- Reject templates that improve recall by increasing distractor rate or unbounded context.
