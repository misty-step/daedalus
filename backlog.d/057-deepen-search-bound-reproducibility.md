# Deepen the search and bound its reproducibility

Priority: P1 · Status: pending · Estimate: M

Child of [[054]]. Retro gap — trust the search itself.

2026-07-01 grooming: also child 5 of [[061]]. The multi-seed and
optimizer-vs-seed-scan checks here are now acceptance criteria for the
Crucible-backed optimization loop, not a separate science project.

## Goal
Make the search actually search — beat its own seed scan and produce a winner
stable across seeds — so the recommended config is the best one found, not the
luckiest random seed.

## Why
In the 2026-06-23 run, all four optimizer mutations returned `improved: false`
(`loop.json` history); the recommended config is a raw landscape-scan seed
(`seed2`), and the whole run used a single `rng_seed: 1` (`seed.json`). So there
is no evidence the optimizer adds value over random sampling, and no estimate of
how much the "winner" depends on the seed. A 2-generation plateau on four moves is
barely a search — yet it stopped and shipped a recommendation.

## Oracle
- [ ] A run reports whether the optimizer's best beats the seed scan's best by a
      CI that excludes zero — or honestly flags "optimizer added nothing over
      random sampling."
- [ ] The `cerberus-reviewer` search is repeated across ≥2 rng seeds; the report
      states whether the recommendation is stable or seed-dependent.
- [ ] Plateau / stop criteria are tightened so a 2-generation no-improvement run
      is flagged as under-searched, not silently accepted as converged.

## Verification System
- Claim: the recommended config is the best found, and stable across seeds.
- Falsifier: a second seed picks a different winner, or the optimizer never beats
  its seed scan → the search is sampling, not optimizing.
- Driver: multi-seed re-runs of `cerberus-reviewer`.
- Grader: cross-seed recommendation agreement; optimizer-vs-seed-scan delta CI.
- Evidence packet: a comparison report across seeds.
- Cadence: once to characterize; spot-check on major arena/model changes.

## Notes
Cheap to test — the trials are cents each ($2.52 for the entire prior run); the
expensive input is operator attention to the conclusion. This may reveal the seed
scan is sufficient and the optimizer is the deletion candidate — a fine, clarifying
outcome.
