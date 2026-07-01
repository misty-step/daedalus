# Deepen GEPA reflective mutation

Priority: P1 · Status: blocked · Estimate: L

Child of [[061]]. Captured after the first optimizer loop used shallow
GEPA-style mutation descriptors to generate a plumbing artifact and provisional
frontier table.

2026-07-01 factory groom correction: the 061 run produced a plumbing artifact,
not a trusted scored frontier. This ticket is blocked by [[066]]; mutation depth
is meaningless until the score is answer-key grounded and there is one optimizer
loop.

## Goal
Replace the first slice's fixed mutation descriptors with an evidence-driven
GEPA inner loop that reads failure evidence, proposes prompt/tool/thinking
mutations, tests comparable children, and keeps only changes that beat the
parent outside observed noise.

## Oracle
- [ ] Mutation proposals cite concrete failure evidence from prior trials and
      name the behavioral hypothesis being tested.
- [ ] The loop can run at least 2 generations where children are compared
      against their parent on the same validation split and budget envelope.
- [ ] A child is marked improved only when paired score delta clears the
      configured noise/effect floor; otherwise it is retained only as evidence,
      not as a promoted winner.
- [ ] Prompt packet, tool policy, thinking level, and stance mutations are
      recorded with composition hashes and lineage in `loop.history.jsonl`.
- [ ] `bin/gate` passes.

## Verification System
- Claim: GEPA improves candidates using reflective evidence instead of merely
  sampling named prompt variants.
- Falsifier: mutations do not cite failures; lineage is missing; children run on
  incomparable budgets; or the best reported candidate is just the seed scan.
- Driver: `threshold optimize-loop` on `pr-review-key-recall-v0` with a
  multi-generation cap.
- Grader: parent/child delta report, lineage records, and seed-vs-optimizer
  comparison.
- Evidence packet: `loop.history.jsonl`, `seed.json`, `pareto.json`, and report.
- Cadence: first multi-generation optimizer run and whenever mutation policy
  changes.

## Notes
Pairs with [[057]] for seed trust: a reflective loop is not credible unless it
beats or honestly loses to the seed scan.
