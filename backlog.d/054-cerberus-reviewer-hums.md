# Make the Cerberus reviewer loop hum

Priority: P0 · Status: in-progress · Estimate: XL (epic)

> **This epic is the Now from [VISION.md](../VISION.md) "Focus — one customer
> until it hums."** Threshold has exactly one customer until this epic closes: the
> Cerberus code reviewer. No second agent, domain, or plane is searched until the
> gaps below close. This epic is the gate; 035 / 036 / 037 / 038 sit behind it.

## Goal
Close the distance between the floor the first certified reviewer search cleared
and a reviewer config we would actually deploy — so the foundry emits a Cerberus
reviewer contract on evidence we would stake the product on.

## Why
The 2026-06-23 search (`runs/20260623T183514Z-search-cerberus-reviewer`) cleared
the *floor*: three configs beat an empty submission at 95% CI on a 6-task arena
for $2.52. Reading its own records — `seed.json`, `loop.json`, `rig.json`,
`lineage.md`, the holdout ledger — exposed exactly how far the floor sits from
humming. Each gap below is a child. Until they close, "certified" overclaims.

## Children (ordered — the hum bar)
1. **Search the capability surface** — [[052]]. Every seed ran
   `skill_set_name: null` and no MCP, so the central bet (outfitting a reviewer
   with real reviewing capability beats the raw model) is untested. Highest
   leverage; the reason Cerberus exists.
2. **Certify vs the incumbent, not `null`** — [[055]]. `loop.json`
   `reward_delta_baseline: "null"`: "certified" means "beats submitting nothing,"
   not "beats the config we ship."
3. **Gate on deployable reliability** — [[056]]. The recommended config had
   pass^5 ≈ 0.04. pass^k is computed but does not gate the recommendation.
4. **A durable, multi-cluster, unburned holdout** — [[051]]. The lone holdout
   (`rs-retry-backoff`) was exposed 35× (burn rule = 5) and is now burned; the
   6-cluster arena barely certifies.
5. **Corroborate review quality beyond recall** — [[053]]. The scorer is
   `max(0, recall − 0.2·FP)`; whether a human would act on a finding is unmeasured.
6. **Trust the search itself** — [[057]]. All four optimizer mutations returned
   `improved: false` (winner is a raw seed) from a single `rng_seed: 1`; the
   search's depth and reproducibility are unknown.

## Oracle (epic-level — the hum bar)
- [ ] A reviewer search runs with a non-null capability surface, and the
      skills/MCP axis is shown to move quality-per-dollar or is honestly reported
      as null-effect — [[052]].
- [ ] The recommended config's reward-delta 95% CI clears a registered incumbent
      baseline (not `null`) under cluster-robust stats — [[055]].
- [x] The recommendation is reliability-gated: pass^k above a declared floor, or
      it is not recommended — [[056]] **delivered (merge-ready)**. Demonstration
      on the real cerberus data at $0 awaits [[059]]; paid re-run awaits [[057]].
- [ ] The arena certifies on ≥8 independent clusters with an unburned holdout;
      `rs-retry-backoff` is rotated and replaced — [[051]].
- [ ] At least one certified reviewer carries a quality-judge corroboration
      alongside the deterministic score — [[053]].
- [ ] The winning config is stable across ≥2 rng seeds and the optimizer is shown
      to beat its seed scan, or the plateau is explained — [[057]].
- [ ] Only then does the gate on 035 / 036 / 037 / 038 lift.

## Verification System
- Claim: the foundry can mint a Cerberus reviewer contract we would deploy.
- Falsifier: with incumbent baseline + reliability gate + a clean multi-cluster
  holdout, no candidate certifies — the prior result was floor-clearing only.
- Driver: a re-run of the `cerberus-reviewer` search under the closed gaps.
- Grader: the six child oracles above, in aggregate.
- Evidence packet: a new run dir whose report names the incumbent, the
  reliability gate, the cluster count, the quality-judge corroboration, and the
  cross-seed stability.
- Cadence: the epic closes when one such run produces a deployable recommendation.

## Notes
- This epic does not deploy anything — the plane owns production trust (VISION).
  It makes the *evidence* deployable.
- Likely sequence: [[051]] (rebuild the burned holdout) → [[052]] (capability
  surface needs a clean holdout to search against) → ([[055]], [[056]] in
  parallel) → [[053]] → [[057]].
- The honest-outcome clause stands: if closing these gaps shows the certified
  configs do *not* beat the incumbent or are not reliable, that is the result —
  surface it, do not soften it.
