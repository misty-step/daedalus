# Build the Crucible-backed optimization loop

Priority: P1 · Status: ready · Estimate: XL

## Goal

Turn a Crucible-owned eval bundle into a Threshold optimization target and run
the search loop that can cheaply reject bad targets, allocate budget across
candidates, improve packets with GEPA-style evidence, and defend the final
recommendation against overfitting, judge gaming, and model/eval drift.

## Oracle

- [ ] A Crucible eval is imported as `threshold.optimization_target.v1`, with
      eval version, Harbor package, answer-key and scorer digests, split policy,
      incumbent baseline, and gate state recorded.
- [ ] A headroom probe capped at about $5 runs before search and aborts when the
      eval is saturated, cannot rank reference agents, lacks scorer parity, or
      ties everything inside observed noise.
- [ ] The optimizer supports a GEPA-style inner loop: use failure evidence to
      propose packet/tool/thinking mutations, run comparable trials, and keep
      only deltas that clear trial noise.
- [ ] Budget allocation supports Hyperband/ASHA-style promotion: many cheap
      configurations get small budgets, only promising arms receive larger
      validation/certification budgets, and every promotion records the
      comparison basis.
- [ ] Guardrails are enforced and reported: train/validation/final-holdout
      separation, holdout exposure ledger, deterministic scorer primacy or judge
      calibration, contamination notes, pinned model/provider ids, stale-model
      re-probe, and incumbent re-measurement.
- [ ] The first end-to-end slice uses the code-review correctness target and can
      run at least one candidate trial through Bitter Blossom/Sprites using the
      request/receipt contract in `docs/crucible-eval-optimization-contract.md`.
- [ ] `bin/gate` passes.

## Verification System

- **Claim:** Threshold can optimize against a Crucible-owned eval without
  owning eval design, and the result is more than lucky seed sampling.
- **Falsifier:** the headroom probe ties or saturates; GEPA never beats the seed
  scan; ASHA promotes candidates that fail paired validation; a judge-only
  objective decides the winner; holdout evidence leaks back into mutation; a
  model/eval version change is compared without re-probing.
- **Driver:** import a code-review correctness eval target, run headroom probe,
  run a bounded GEPA plus ASHA search, and execute one remote trial through
  Bitter Blossom/Sprites.
- **Grader:** target validation report, search report with seed-vs-optimizer
  delta, promotion ledger, guardrail checklist, Sprites receipt, and final
  paired holdout delta.
- **Evidence packet:** `threshold.optimization_target.v1`, `rig.json`,
  `headroom-probe.json`, `seed.json`, `loop.history.jsonl`, `asha.json`,
  `guardrails.json`, run records, report, and Sprites receipt.
- **Cadence:** required for the first Crucible-backed target, then rerun when
  eval version, scorer digest, model pool, or downstream incumbent changes.

## Children

1. **Target import contract.** Add `threshold.optimization_target.v1` parsing and
   validation for a Crucible `crucible.eval_spec.v1` plus Harbor export. Refuse
   unknown schema, missing digests, missing scorer, absent split policy, or
   untrusted G2 state.
2. **About $5 headroom probe gate.** Run oracle/null/oneshot plus incumbent and
   a few diverse seed candidates on validation, report agent spread, and abort
   search when the target cannot rank.
3. **GEPA inner loop.** Replace the current shallow reflective mutation plateau
   with an evidence-driven loop over prompt packets, tool policy, thinking
   level, and candidate stance. Keep the hand-rolled path only until the library
   version proves better in the same oracle.
4. **Hyperband/ASHA budget allocator.** Start many arms cheaply, promote by
   paired lower-confidence-bound improvement and cost/latency envelope, and
   record why each arm advanced or stopped.
5. **Search reproducibility and seed trust.** Fold [[057]] into this epic's
   acceptance: compare optimizer-best vs seed-best, rerun across at least two
   rng seeds, and report seed-dependent recommendations as non-certified.
6. **Guardrail packet.** Emit and validate overfitting, judge-gaming,
   contamination, holdout-burn, and non-stationarity checks before any launch
   recommendation.
7. **Bitter Blossom/Sprites trial runner.** Submit one remote candidate trial,
   consume `threshold.sprite_trial_receipt.v1`, score the returned artifact
   locally, and preserve the run id, task id, composition hash, artifact refs,
   cost/wall fields, and failure state in Threshold run records.
8. **First end-to-end slice.** Use the code-review correctness target to produce
   a report that either certifies a Bitter Blossom correctness-lane candidate or
   explains which guard blocked search.

## Completion Slice

2026-07-01 core factory slice: PR #26 proved the end-to-end path from a
Crucible eval target to GEPA-style candidates, Bitter Blossom Sprite dispatch,
Threshold-owned scoring, and a score/cost Pareto frontier. The run packet lives
at `runs/20260701T182031Z-optimizer-loop-pr-review-key-recall-v0/` and
intentionally reports held-out certification as `not_certified` because the
Kimi Sprite arm stayed stale in execution with unknown cost.

The remaining depth is now split out so this core slice can close without
pretending the full research loop is complete:

- [[062]] ASHA/Hyperband budget allocation across more candidates.
- [[063]] deeper GEPA reflective mutation.
- [[064]] held-out optimizer certification.
- [[065]] robust stale Bitter Blossom arm handling, dependent on Bitter Blossom
  [[083]].

## Notes

- This epic is the active home for the optimizer-loop work requested after the
  Threshold rename. It does not replace [[035]] production trace re-ingestion;
  035 starts after a deployed incumbent and G5-approved traces exist.
- [[036]] and [[037]] are downstream consumers of this loop. They should import
  candidates only after this epic proves the eval target and runner seam.
- Crucible backlog 008 remains a hard caveat: until grade parity is proven,
  Crucible may author/export the eval, but Threshold's Rust scorer remains the
  optimization objective.
- No child may weaken the scorer, loosen G2, hide unknown cost, or compare
  across eval/model versions without an explicit fresh probe.
