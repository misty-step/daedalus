# Gate the recommendation on deployable reliability (pass^k)

Priority: P0 · Status: pending · Estimate: M

Child of [[054]]. Retro gap #3.

## Goal
Make pass^k a first-class objective that can veto a recommendation, so the foundry
never recommends a high-mean config that fails most of the time.

## Why
The 2026-06-23 recommended config (`seed2-kimi-k2.7-code-trace-callers`) had mean
reward 0.7544 but pass^5 ≈ 0.043 — a ~4% chance all five trials reach the reward
floor. The run's own report quotes τ-bench — "a high mean with low pass^5 is not
deployable" — and then recommended it anyway. pass^k is computed (`loop.json`
`consistency`) but does not gate the recommendation. A reviewer that posts on
every PR cannot be a coin flip.

## Oracle
- [ ] A declared reliability floor (pass^k ≥ threshold at a declared k) is a
      spec/CLI knob; a candidate below it cannot be the recommendation.
- [ ] The report shows the reliability-gate verdict per candidate; a high-mean,
      low-pass^k config is visibly demoted, not recommended.
- [ ] Re-running `cerberus-reviewer` either recommends a reliability-passing
      config or honestly reports "no candidate clears the reliability floor —
      search more / harden the arena."

## Verification System
- Claim: the recommended config is reliable enough to deploy, not just high-mean.
- Falsifier: the gate, applied to the 2026-06-23 candidate set, leaves no
  recommendable candidate → the prior recommendation was not deployable.
- Driver: re-score the existing run's `consistency` block through the new gate.
- Grader: pass^k vs the declared floor.
- Evidence packet: a report with the reliability column gating the recommendation.
- Cadence: every recommendation.

## Notes
Tune k and the floor to the task: a reflex reviewer posting on every PR needs a
higher pass^k than an advisory one. The floor is an operator knob, not a magic
constant — record the value and the reason. Interacts with [[057]]: a low pass^k
may mean "search harder," not "this config is bad."
