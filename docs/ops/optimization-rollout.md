# Optimization Rollout and Rollback

## Scope

Apply only approved offline optimization proposals.

## Approval Preconditions

- Proposal exists and is approved by a human reviewer.
- Baseline and candidate reports are attached.
- Rollback steps are explicit and tested.

## Rollout Steps

1. Apply approved config/template files only.
2. Record commit hash and proposal id.
3. If retrieval config changed, reload daemon config:
   - `rarag daemon reload --json`
4. Run fast verification:
   - `scripts/check-fast-feedback.sh`
5. Run focused replay on critical tasks.

## Rollback Steps

1. Revert to baseline commit or apply documented rollback patch.
2. Reload daemon config if retrieval settings changed.
3. Re-run fast verification and focused replay.
4. Mark proposal outcome as rolled back with cause.

## Operational Guardrails

- No auto-merge and no auto-reload from generated candidates.
- Keep rollout batches small and traceable.
- Prefer deterministic file changes over runtime mutation.
