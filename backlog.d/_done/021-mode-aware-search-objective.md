# Make keep/plateau/parent selection mode-aware

Priority: P0
Status: ready
Estimate: S

## Goal
The search optimizes the same objective the report grades: under
cost-sensitive modes, a child that holds reward within the noise band while
cutting cost (or wall, for fast-enough) counts as an improvement, resets the
plateau, and cost-frontier candidates are eligible parents.

## Non-Goals
- Numeric quality thresholds in the taskspec (separate decision)
- Full scalarization framework

## Oracle
- [ ] `loop.improved_over(child, parent, mode)`: reward gain beyond noise
      improves in every mode; reward-held + cost/trial ≥10% lower improves
      under threshold-then-cheap/pareto; reward-held + wall ≥10% lower
      improves under fast-enough (offline tests for each)
- [ ] Regression test reproducing the capstone: a g1b-shaped child (reward
      equal, cost −42%) registers improved=True under threshold-then-cheap
      and does not advance the plateau counter
- [ ] `parent_pool` includes the cheapest candidate within 0.05 reward of
      the best (cost frontier stays breedable)
- [ ] `bin/threshold` passes the taskspec mode into the loop; `bin/gate` green

## Notes
Evidence: capstone loop.json — g1b (the eventual recommendation) recorded
improved=False and helped trigger plateau; the search stopped partly because
it succeeded the way the mode wanted. Search and selection must agree.
