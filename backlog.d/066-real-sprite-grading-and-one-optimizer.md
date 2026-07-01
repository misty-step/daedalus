# Real sprite grading and one optimizer loop

Priority: P0 · Status: parked-behind-crucible · Estimate: XL

Epic. Created by the 2026-07-01 factory groom as Threshold's reentry gate.

## Goal

Resume Threshold optimization only after remote Sprite candidate artifacts are
graded against the eval's answer keys and the July Sprite path is collapsed into
the older typed optimizer machinery. The optimizer may use remote verdicts as
execution-health signals, but not as the score.

## Oracle

- [ ] Sprite trial artifacts are graded against arena answer keys through the
      shared Crucible/Threshold scorer path; `remote_verdict_score` is removed
      from the objective and kept only as a health/checkpoint field.
- [ ] A missing, failed, stale, or `not_dispatched` Sprite receipt cannot score
      as success and cannot enter a Pareto frontier.
- [ ] `threshold optimize-loop` uses the typed search/statistics machinery
      instead of maintaining a separate stringly-typed frontier, scorer, and
      certification path in `optimization_target.rs`.
- [ ] Delta CIs, min-effect floors, pass^k reliability, seed trust, and coded
      certification predicates are wired into the Sprite-backed path before any
      launch recommendation.
- [ ] [[062]], [[063]], [[064]], and [[057]] remain blocked until this ticket's
      grading and one-optimizer criteria pass.
- [ ] `bin/gate` passes.

## Verification System

- Claim: Threshold can compare Sprite-run candidates using the eval's own
  answer-key scorer, not self-reported verdict strings.
- Falsifier: a candidate that always prints `"verdict": "pass"` can dominate
  the frontier without matching answer-key findings; a `not_dispatched` trial
  scores above failure; or the Sprite path bypasses `stats.rs`.
- Driver: rerun the 061 target with at least one known positive and one clean
  fixture, then compare the returned artifacts against the answer key.
- Grader: scorer output, `pareto.json`, `guardrails.json`, and certification
  predicate show answer-key-derived rewards and no self-report objective.
- Evidence packet: updated run dir with scorer traces, Sprite receipts,
  frontier, guardrails, and a report that labels any remaining caveat.
- Cadence: required once before optimization resumes; rerun when scorer schema,
  Crucible export, Sprite receipt schema, or optimizer stats policy changes.

## Children

1. **Answer-key grade Sprite artifacts.** Import the returned candidate findings
   into the same scorer family used for local arena trials, or link the shared
   `crucible-core` scorer when that seam is available.
2. **Retire self-report scoring.** Remove `remote_verdict_score` from the
   quality objective, delete the `not_dispatched -> 1.0` path, and retain remote
   verdict only as a health/triage signal.
3. **Collapse the optimizer fork.** Make Sprite dispatch a runner substrate
   under the typed search loop; replace raw `serde_json::Value` plumbing where
   deterministic code branches on schema.
4. **Wire statistics and certification.** Reuse the existing confidence,
   reliability, seed-trust, and certification predicates instead of a parallel
   prose label.
5. **Gate the unpark.** Add a doctor or regression check that prevents
   `optimize-loop --dispatch-bitterblossom` from claiming a frontier when this
   scorer path is unavailable.

## Notes

- This is the operator's binding reentry criterion for Threshold after the
  2026-07-01 groom. It absorbs the groom report's Epic A and Epic B.
- Do not advance ASHA, GEPA, or held-out certification depth until this is true.
