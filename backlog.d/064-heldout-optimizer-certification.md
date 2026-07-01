# Run held-out optimizer certification

Priority: P1 · Status: pending · Estimate: M

Child of [[061]]. The first multi-candidate run wrote a held-out certification
packet, but honestly left it `not_certified` after a stale Sprite arm blocked
the validation rung.

## Goal
Complete the final held-out certification step for a promoted optimizer
candidate without feeding held-out failures back into GEPA or ASHA decisions.

## Oracle
- [ ] `threshold optimize-loop` promotes at least one validation-frontier
      candidate to a held-out rung and executes held-out Sprite trials.
- [ ] `certification.json` records held-out tasks, promoted candidate hash,
      score/cost, confidence interval or effect floor, and pass/fail verdict.
- [ ] Held-out evidence is append-only and blocked from mutation prompts,
      allocator decisions, and follow-up candidate generation in the same run.
- [ ] The report distinguishes validation Pareto rank from held-out
      certification.
- [ ] `bin/gate` passes.

## Verification System
- Claim: Threshold can certify an optimizer winner on unseen tasks after search.
- Falsifier: held-out failures influence later mutations; no held-out run record
  exists; certification is claimed with stale/unknown-cost arms; or validation
  rank is reported as final certification.
- Driver: a spend-capped `pr-review-key-recall-v0` optimizer run with
  `--certify-top 1`.
- Grader: `certification.json`, held-out Sprite receipts, and report text.
- Evidence packet: run dir containing validation and held-out rungs.
- Cadence: required before any launch recommendation.

## Notes
The first 2026-07-01 run proved the certification artifact shape but not the
certification claim.
