# Gate the recommendation on deployable reliability (pass^k)

Priority: P0 · Status: done (merged to master 2026-06-25, commit 5b5b2ca) · Estimate: M

Child of [[054]]. Retro gap #3.

## Implementation Receipt (2026-06-25)
- `stats::partition_reliable` (new, `crates/daedalus-core/src/stats.rs`): pure
  gate splitting certified candidates into (reliable, demoted) by
  `pass^k ≥ reliability_floor`; `floor ≤ 0.0` is inert (pre-056 behaviour);
  undefined pass^k (k > n_trials) fails a positive floor.
- `--reliability-floor <p>` CLI flag (`daedalus run`), validated to `[0,1]`
  before any spend; restricts the recommendation (`pick`) to certified ∩
  reliable. `certified` keeps its pre-056 meaning (significance only).
- `report.md`: demoted candidates get a "Demoted by the reliability gate" block;
  a "No deployable candidate" note fires when certified is non-empty but nothing
  clears the floor. `loop.json` carries `reliability_floor`, `recommendable`,
  `reliability_demoted`. Documented in `docs/operator-sop.md` §3.
- Evidence: `bin/gate` green (fmt + 323 tests + clippy `-D warnings`); unit test
  `partition_reliable_demotes_the_real_cerberus_recommendation` encodes the
  decisive finding (seed2-kimi 17/30 → pass^5 ≈ 0.0434, demoted at a 0.10 floor);
  fresh-context adversarial review returned no blocking findings; live CLI:
  invalid floor rejected, valid floor + `--estimate` forecasts with no spend.
- **Deferred:** a paid end-to-end re-run of `cerberus-reviewer` under a positive
  floor (real-data demonstration) — needs budget sign-off and belongs with
  [[057]]'s multi-seed re-runs. The $0-on-real-data path is filed as [[059]].

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
