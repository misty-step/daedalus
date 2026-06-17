# Author a contamination-resistant pr-review holdout arena

Priority: P1 · Status: ready · Estimate: L

## Goal
A private pr-review arena of novel, non-public code with enough independent sources to certify a config without leakage — so pr-review discoveries can be validated against fixtures no model has seen.

## Oracle
- [ ] A new arena (e.g. `arenas/pr-review-private-v0`) whose `environment/` code is author-written, not derived from any public repo (no GitHub-indexable upstream); `contamination.toml` records `public = false` and `arena-validate` blesses it contamination-resistant.
- [ ] **≥6 independent sources** (distinct synthetic modules/projects), each a `source_repo` cluster, so the cluster-robust CI has ≥5 df (t≈2.57, not the degenerate df=1 of the 2-repo public arenas — see [[040]] slice A / the `t_{G−1}` finding).
- [ ] Each defective task has a planted defect + answer key + oracle solution authored together before any candidate runs; `arena-validate` passes (oracle 1.0, null floor, probe not saturated/inconclusive, `arena-redteam` spans tightened so gaming reward is not trivially 1.0).
- [ ] A run on it certifies *something* at the observed pr-review effect sizes (the power note `clstr→95%` is achievable), demonstrating the holdout is both contamination-resistant and adequately powered.

## Verification System
- Claim: a config certified on this arena is genuinely good, not leakage-inflated.
- Falsifier: a model known to have seen public rich/pygments scores no higher here than a model that hasn't (no leakage signal); a gaming candidate (`arena-redteam`) earns no undue reward.
- Driver: `daedalus arena-validate` + `daedalus arena-redteam` on the new arena; a live search (spend-gated) to confirm certifiability.
- Grader: the freeze-gate report + redteam audit + a certified-candidate CI that excludes 0.
- Evidence packet: the arena dir + its validity records + a run report.
- Cadence: at arena freeze (G2).

## Notes
Split out of [[040]] (item 4): launch-contract-v0 satisfies "a contamination-resistant arena exists" literally, but it validates launch-contract, not pr-review. Substantial fixture-authoring; benefits from operator input on the synthetic domains (what kinds of bugs, what code style). The t-correction finding makes the cluster-count requirement concrete: 2 sources is unusable, ~6+ is the floor for certifiability.
