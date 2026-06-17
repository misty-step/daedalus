# Make the foundry prove "better," not just rank it

Priority: P0 · Status: ready · Estimate: XL

## Goal
A Daedalus "win" is a statistically defensible claim that a candidate beats the baseline (and its rivals) on the task — confidence-bounded, consistency-aware, and robust to the search trajectory — not the top point-estimate of a small, noisy tournament.

## Oracle
- [ ] Every certified candidate's report shows a 95% CI on (candidate − baseline) reward, computed with **clustered** standard errors over tasks (tasks from the same source repo form one cluster).
- [ ] `improved_over` / certification accept a candidate only when the CI excludes 0 (or a configured minimum detectable effect), not merely when n ≥ certify_trials.
- [ ] The report surfaces a per-candidate **consistency** metric (pass^k-style: fraction of trials at/above a reward floor), reported separately from mean reward.
- [ ] A robustness check runs ≥2 independent seed trajectories and flags when their certified tops diverge by more than the pooled noise (basin-trap detector).
- [ ] A power note: given measured per-task variance, the report states the minimum n to detect the target reward delta at 95% confidence.

## Verification System
- Claim: certified candidates are genuinely better — not noise, not a basin artifact.
- Falsifier: re-running a "certified win" at larger n, or from a second seed, overturns the ranking.
- Driver: `daedalus run` on pr-review-v2 with raised `--certify-trials` + a second `--rng-seed`; a Rust stats unit-battery over known synthetic reward distributions.
- Grader: CI/significance computed in Rust (bootstrap or paired t / Wilcoxon + clustered SE), asserted against known-distribution fixtures in `bin/gate`.
- Evidence packet: report.md CI table + consistency column + divergence flag + holdout-ledger row.
- Cadence: every `run`; the stats battery runs in `bin/gate`.

## Children
1. Clustered-SE 95% CIs on reward deltas in report.md + loop.json. **Why:** "Adding Error Bars to Evals" (Miller, arXiv 2411.00640) — evals are experiments; report ±1.96·SE and use clustered SE when items are correlated. pr-review-v2's 10 tasks come from 2 repos (rich, pygments) → naive SE understates variance.
2. CI-gated accept rule: `improved_over` + certification require the CI to exclude 0 (configurable MDE). **Why:** today acceptance compares point-estimate means against a within-run noise band (`search_loop.rs::trial_noise`); the foundry's own repro showed 0.69 vs 1.000 in-search variance (ROADMAP.md).
3. Per-candidate consistency / pass^k metric. **Why:** τ-bench (Sierra) + "Consistency as a Testable Property" (arXiv 2605.10516) — mean hides reliability; pass^k (succeed across ALL k) is the deployability axis. A reviewer right 60% of the time is not shippable at any mean.
4. Trajectory-divergence / basin-trap detector (≥2 seeds; compare certified tops at holdout). **Why:** the search is single-population reflective hill-climbing (`search_loop.rs`) with no basin escape; ~2× seed budget (~$0.60) buys a robustness signal (operator-lane finding).
5. Power/sample-size note from measured variance. **Why:** ROADMAP already concedes "contract-grade claims need n≥5" — make n a computed function of variance + target MDE, not a guess.

## Notes
This is THE gap between "ranks candidates" (today) and "discovers provably high-quality configs" (the stated mission). It builds ON the existing rigor (oracle/null/probe gates, holdout ledger, seeded RNG) by adding the inferential layer. Cost: the statistics are offline/free; spend only rises from larger n and the second seed trajectory. Pairs with [[040]] — valid statistics over a valid arena is what makes discovery trustworthy. External grounding: arXiv 2411.00640, τ-bench/Sierra, arXiv 2605.10516.
