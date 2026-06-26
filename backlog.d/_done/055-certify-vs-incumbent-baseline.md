# Certify against a registered incumbent baseline, not the null floor

Priority: P0 · Status: done · Estimate: M

Child of [[054]]. Retro gap #2.

## Implementation Receipt (2026-06-26)
- Added `[incumbent]` support to `specs/cerberus-reviewer/taskspec.toml` and
  `daedalus run`: the incumbent materializes as `kind = "incumbent"`, runs
  `--certify-trials` deep on every task, and is excluded from Pareto,
  recommendation, mutation, and false-positive-trap alarms.
- Certification now selects the incumbent as the reward-delta baseline when one
  is present; otherwise it falls back to the null floor. The CI table, report
  verdict, and `loop.json.reward_delta_baseline` name the selected baseline.
- Run A evidence:
  `runs/20260625T161856Z-search-cerberus-reviewer/`. Result: the best challenger
  point estimate is `seed2-kimi-k2-7-code-trace-callers` at Δ reward `+0.0711`
  vs incumbent, but 95% CI `[-0.1249, +0.2671]`, so no candidate is certified.
  This closes the mechanism and falsifies any floor-clearing overclaim; it does
  not ship a Cerberus replacement.
- Report QA fixed the derived HTML/markdown surfaces so explicit
  `recommended: null` remains "no certified pick" instead of recomputing a Pareto
  recommendation.

## Goal
Make certification mean "beats the config we would otherwise ship" by registering
an incumbent baseline that the reward-delta CI is computed against — instead of
`null` (the empty submission).

## Why
The 2026-06-23 search certified three configs as "95% CI lower bound > null floor"
(`loop.json` `reward_delta_baseline: "null"`). That proves only that they beat
submitting nothing. It does not show they beat the hand-picked `deepseek-v4-pro`
that bb/Cerberus actually ship (`storm-correctness.toml`), nor a strong
off-the-shelf reviewer. VISION: "prove better, don't just rank" — against the
incumbent, not the floor. [[035]] covers the *live* incumbent (needs a real G3
deploy + production traces, neither of which exists yet); this ticket is the
synthetic precursor that needs neither and unblocks the hum bar now.

## Oracle
- [x] The search loop accepts a registered incumbent composition as the
      reward-delta baseline (spec/CLI field), distinct from oracle / null / probe.
- [x] A re-run of the `cerberus-reviewer` search reports each candidate's Δ and
      95% CI against the incumbent; "certified" is redefined as the CI clearing
      the incumbent, not `null`.
- [x] The report names the baseline used, and a candidate that beats `null` but
      not the incumbent is shown as not-certified.

## Verification System
- Claim: a candidate that "certifies" actually beats what we would otherwise deploy.
- Falsifier: with the incumbent registered, the 2026-06-23 winner fails to clear
  it → the prior certification was floor-clearing only.
- Driver: re-run `cerberus-reviewer` with `--baseline incumbent:<hash>`.
- Grader: cluster-robust reward-delta CI vs the incumbent.
- Evidence packet: a run dir with the incumbent-baselined report + CIs.
- Cadence: once to build; every certified search thereafter.

## Notes
Choose the incumbent honestly — the currently-shipped review model/config
(`deepseek-v4-pro` for bb `storm-correctness`) is the natural pick. Keep
oracle / null / probe as rig calibrators; this adds a fourth reference role
(incumbent) that is the *certification* baseline, not a rig check. Coordinate the
field name with [[035]] so the live-incumbent mode is a drop-in upgrade, not a
second mechanism.
