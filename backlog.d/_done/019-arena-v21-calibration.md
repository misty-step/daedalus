# Calibrate pr-review-v2.1: wider key spans, deeper holdout, certification trials

Priority: P1
Status: ready
Estimate: M

## Goal
Fix the calibration gaps the delivered-agent repro exposed: a finding that
identifies the right defect must not score 0 for citing a line a few rows
outside a tight key span, and certification of a winner needs more than n=2
trials per task.

## Non-Goals
- Loosening the scorer itself (grader is gospel; spans are arena data)
- Judge-based partial credit (ticket 010 owns that)

## Oracle
- [ ] v2.1 key spans cover the enclosing function/method of each seeded
      defect; oracle still 1.0, null still 0.25, probe still 0.0
- [ ] ≥ 2 additional holdout tasks so the final evaluation is not one
      concurrency task (freeze gate re-run and recorded in provenance.md)
- [ ] `bin/threshold` (or delivery procedure) certifies the recommended
      candidate with n ≥ 5 trials/task before the delivery doc may claim a
      reward number; the claim states mean ± observed range
- [ ] Arena version bumped to 2.1; prior v2.0 records not mixed into new
      comparisons

## Notes
Evidence (2026-06-10): winning composition g1b scored 8/8 in-search but a
fresh repro scored 0 on py-measure-normalize despite finding the defect
(line 117 cited vs key span [108, 111]) and missed py-live-lock once —
within-composition variance the n=2 protocol underestimates. Recorded in
arenas/pr-review-v2/provenance.md ("Known calibration finding").
