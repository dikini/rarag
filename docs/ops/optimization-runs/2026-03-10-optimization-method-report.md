# 2026-03-10 Optimization Method Report

## Scope

- Constraint: config-only optimization (no code edits/rebuild between experiments).
- Binary set: `target/release/rarag`, `target/release/raragd`.
- Indexed source set: staged workspace at `/tmp/rarag-opt-workspace` (symlinked repo docs/examples/tests plus local root crate shim).
- Objective: maximize `acceptable_task_hit_rate`, minimize `distractor_task_hit_rate`.
- Each full run: `3 cycles x 10 experiments` plus baseline per cycle.

## Candidate Selection Strategy

Two methods were used:

1. `heuristic`
- Cycle 1: hand-crafted directional hypotheses (document priors, rerank boosts, neighborhood boosts).
- Cycle 2/3: exploit prior best using top-3 trend deltas plus targeted distractor/coverage probes.

2. `random-jitter`
- Cycle 1: seeded random perturbations around baseline.
- Cycle 2/3: seeded random perturbations around prior cycle best.

Selection rule per cycle:

- objective = `acceptable - distractor + 0.1*ideal - 0.01*avg_result_count - 0.02*warning_task_rate`
- tie-break: higher acceptable, lower distractor, lower avg_result_count, lower warning_task_rate, then experiment id.

## Full Runs Executed

### Split Fixtures, No History

- `split-br-heur-r1`, `split-br-rand-r1`
- `split-ia-heur-r1`, `split-ia-rand-r1`
- `split-bref-heur-r1`, `split-bref-rand-r1`
- `split-fe-heur-r1`, `split-fe-rand-r1`
- `split-us-heur-r1`, `split-us-rand-r1`

Observed best metrics (stable across all cycles and both methods):

- `blast-radius`: acceptable `0.5`, distractor `0.0`, ideal `0.0`, avg results `3.0`, warning rate `0.0`
- `implement-adjacent`: acceptable `1.0`, distractor `0.0`, ideal `1.0`, avg results `6.0`, warning rate `0.0`
- `bounded-refactor`: acceptable `1.0`, distractor `0.0`, ideal `1.0`, avg results `8.0`, warning rate `0.0`
- `find-examples`: acceptable `1.0`, distractor `0.0`, ideal `1.0`, avg results `6.0`, warning rate `0.0`
- `understand-symbol`: acceptable `1.0`, distractor `0.0`, ideal `1.0`, avg results `4.0`, warning rate `0.0`

### Hard Fixture, With/Without History

- no history: `rarag-hard3-r1`
- with history: `hard3-hist-heur-r1`, `hard3-hist-rand-r1`

Observed best metrics (stable across all cycles and both methods):

- no history (`rarag-hard3-r1`): acceptable `0.90909094`, distractor `0.0`, ideal `0.8181818`, avg results `3.0`, warning rate `0.0`
- with history (`hard3-hist-*`): acceptable `1.0`, distractor `0.0`, ideal `1.0`, avg results `6.9091`, warning rate `1.0`

History-enabled warning class:

- `history selector requested but no history candidates were found`

Additional blast-radius history check:

- `split-br-hist-heur-r1`, `split-br-hist-rand-r1` both reached acceptable `1.0`, distractor `0.0`, ideal `1.0`, with avg results `10.0`, warning rate `1.0`.

## Interpretation

- For these fixtures, parameter sweeps are flat inside each run: candidate metrics are invariant (`experiment-01` always selected by tie-break).
- Optimization method choice (`heuristic` vs `random-jitter`) had no measurable effect under this search space and fixture set.
- The only large behavioral shift came from enabling `--include-history`, which improved hit rates but introduced warnings on all tasks and increased result volume.

## Next Hypotheses

1. Keep two operating profiles:
- `strict-no-warning`: `--include-history` off (lower recall on blast/hard3, cleaner warnings and tighter result set).
- `max-recall`: `--include-history` on (higher recall/ideal, accepts warning + result-count inflation).

2. If warning-free high recall is required without code changes:
- adjust eval policy to treat this specific history warning separately, or
- run dual reporting (`with` and `without` history) and gate rollout on both.

3. For additional optimization signal, increase fixture difficulty/coverage:
- add more blast-radius tasks and harder distractor-heavy tasks; current split fixtures are largely saturated at baseline.
