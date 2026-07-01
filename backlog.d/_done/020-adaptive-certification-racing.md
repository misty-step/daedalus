# Adaptive certification racing: spend trials on survivors, not luck

Priority: P0
Status: ready
Estimate: M

## Goal
Kill the winner's curse: after the search loop, the top-k candidates under
the declared mode are topped up to n ≥ 5 trials per search task before any
recommendation, and the recommendation may only come from certified
candidates — so a lucky 8/8 can never ship again.

## Non-Goals
- Full bandit/ASHA machinery (simple screen→certify is enough at $0.014/trial)
- Changing the scorer

## Oracle
- [ ] `bin/threshold` stage 3.5 certifies top-k (default 3) by mode objective:
      each reaches `--certify-trials` (default 5) per train+validation task,
      budget-metered (shrinks k, never silently shrinks n)
- [ ] Holdout final runs at certification depth for the front
- [ ] `report.recommend` restricted to certified candidates when any exist;
      report.md and pareto.json show per-candidate trial counts + certified
      flag; grid shows per-task n and min/max range
- [ ] Offline test: a candidate with fewer trials and a lucky mean is not
      recommended over a certified rival
- [ ] `bin/gate` green

## Notes
Evidence: capstone winner g1b measured 8/8 in-search, ~0.69 on repro
(deliveries/pr-review/DELIVERY.md). Selection on n=2/task noise inflates the
winner's measured score by construction. Trials cost cents; buy them where
they matter. Probe trials with rejected requests should record cost 0, not
unknown (fold the known-spend understatement fix in here).
