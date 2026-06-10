# In-loop meta-eval monitor: the eval is a measured object during the run

Priority: P0
Status: ready
Estimate: M

## Goal
The run notices mid-flight when the benchmark — not the candidates — is the
bottleneck, and says so as a first-class output: saturation-at-the-top,
per-task variance flip-flops, and FP-traps that never fire become recorded
alarms that change behavior instead of footnotes a human must catch.

## Non-Goals
- Auto-editing the arena mid-run (frozen surfaces stay frozen)
- Replacing the boundary gates (probe abort, freeze gate, G2)

## Oracle
- [ ] `run_search` accepts an injected `monitor_fn(summary, generation)`;
      alarms accumulate in loop.json and render as a "Meta-eval alarms"
      section in report.md and lineage.md
- [ ] Saturation alarm: best candidate at reward ceiling → under max-quality
      the run stops with stop_reason `arena-saturated-at-top`; under
      cost-sensitive modes it records "reward gradient exhausted; searching
      cost only" (offline tests for both)
- [ ] Variance alarm: any candidate-task with both 0.0 and 1.0 trials →
      "certification required" note
- [ ] Clean-trap alarm at report time: a clean task every candidate passed →
      "FP trap never fired"
- [ ] An alarmed run emits a draft arena-iteration note (file in the exp dir)
      the operator can promote to a backlog ticket; `bin/gate` green

## Notes
Evidence: the capstone searched a saturated train+validation from generation
0 (seed1 at 1.000) and spent stage-3 budget mutating against a ceiling with
no reward gradient; only the operator noticed. The answer to "what if the
evals turn out inadequate mid-run" must be machinery, not vigilance.
