# Scale ASHA/Hyperband budget allocation across more candidates

Priority: P1 · Status: blocked · Estimate: L

Child of [[061]]. Captured after the first 2026-07-01 optimizer plumbing proof
connected a Crucible-shaped eval target to GEPA-style candidates and Bitter
Blossom Sprite receipts.

2026-07-01 factory groom correction: the first loop proved plumbing, not Pareto
scoring. This ticket is blocked by [[066]]; broad budget allocation must not
resume until Sprite artifacts are scored against answer keys and the duplicate
optimizer path is collapsed.

## Goal
Turn the current small-rung ASHA packet into a real budget allocator that can
race a broad candidate population under a fixed dollar cap, cheaply stop weak
arms, and promote only candidates with enough paired evidence to justify more
spend.

## Oracle
- [ ] `threshold optimize-loop` can launch at least 8 candidate arms across at
      least 2 ASHA rungs while preserving the configured `--budget-usd` cap.
- [ ] Every stopped or promoted arm records rung, budget unit, score, cost,
      wall time, promotion comparator, and stop reason in `asha.json`.
- [ ] Promotion uses paired validation evidence and cost/latency envelope, not
      raw unpaired score alone.
- [ ] Unknown-cost, stale, or failed Sprite arms cannot consume promotion slots
      unless an explicit recovery receipt resolves them first.
- [ ] `bin/gate` passes.

## Verification System
- Claim: Threshold allocates optimizer budget across many candidates instead of
  running a fixed one-shot comparison.
- Falsifier: an arm advances without a recorded comparator; total known spend
  exceeds cap; a stale/unknown-cost arm is promoted; or replaying the same
  fixture changes promotions.
- Driver: a bounded run of `pr-review-key-recall-v0` with at least 8 arms and
  a small dollar cap.
- Grader: `asha.json`, `pareto.json`, and report promotion ledger.
- Evidence packet: run dir with request/receipt/result records for every arm.
- Cadence: first scaled optimizer run, then spot-check when allocator policy
  changes.

## Notes
This is the depth beyond the first proof. The 2026-07-01 slice proved the
interface path, not the scoring path; it does not yet prove allocator quality at
breadth.
