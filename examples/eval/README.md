# Eval Fixtures

This folder documents a minimal offline retrieval-eval loop for repository tasks.

## Fixture Shape

Task fixtures are JSON arrays of records with:

- `task_id`
- `revision` (git sha or pinned revision label)
- `query_mode`
- `query_text`
- `symbol_path` (optional)
- `ideal` (must-hit evidence ids)
- `acceptable` (good-but-not-required evidence ids)
- `distractors` (known irrelevant ids)

See: `tests/fixtures/eval/tasks.json`.

## Running Fixture Replay

Use the existing test harness:

```bash
cargo test -p rarag-core --test eval_fixtures -- --nocapture
```

The replay test indexes a pinned fixture repo revision, runs retrieval with observability enabled,
and verifies persisted observation rows include:

- `eval_task_id`
- evidence-class coverage (for example `code`, `document`, `history`)

## Notes

- Evaluation capture is observational only and must not alter ranking output.
- Fixture revisions should stay pinned for reproducibility.
