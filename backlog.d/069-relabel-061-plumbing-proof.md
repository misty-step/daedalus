# Relabel 061 as plumbing proof only

Priority: P0 · Status: ready · Estimate: S

## Goal

Make every current narrative surface describe the 061 optimizer loop as a
plumbing proof only, not as a measured Pareto frontier or candidate-quality
result.

## Oracle

- [ ] `runs/20260701T182031Z-optimizer-loop-pr-review-key-recall-v0/report.md`
      states before the tables that the run is plumbing-proof-only because its
      candidate-dependent score is remote self-verdict, not answer-key grading.
- [ ] `runs/NOTEBOOK.md` records the same caveat for the 061 run without
      editing any `runs/*.jsonl` file.
- [ ] `README.md` status/onboarding text says Threshold is parked behind
      Crucible and that the 061 run is not a certified measurement.
- [ ] `backlog.d/061-build-crucible-backed-optimization-loop.md` and dependent
      tickets point to [[066]] as the reentry gate.
- [ ] `bin/gate` passes.

## Verification System

- Claim: a cold reader cannot mistake the 061 run for a trusted optimizer
  result.
- Falsifier: README, run report, notebook, or backlog still presents the
  score/cost table as evidence that one candidate beat another.
- Driver: targeted `rg` over README, run report, notebook, and backlog.
- Grader: reviewed prose plus the exact 061 `pareto.json` formula
  `source_split_key_recall * remote_verdict_score`.
- Evidence packet: PR diff and `bin/gate` output.
- Cadence: once, immediately after the groom backlog lands.

## Notes

- This is the top executable item in the 2026-07-01 Threshold operator
  decisions.
- Do not edit committed run JSONL history.
