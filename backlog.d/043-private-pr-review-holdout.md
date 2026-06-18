# Author a contamination-resistant pr-review holdout arena

Priority: P1 · Status: in-progress · Estimate: L

## Goal
A contamination-resistant pr-review arena of synthetic, non-public-library code with enough independent sources to certify a config without leakage — so pr-review discoveries can be validated against fixtures whose code models have not heavily trained on.

## Resolution — designate the existing synthetic `pr-review-v0`
`pr-review-v0` already satisfies the intent: its tasks (auth, cart, file-cache, pagination, retry, rename across py/js/rs) are **author-written synthetic scenarios**, not derived from rich/pygments or any popular public library. Designating it is the right delivery — don't author what exists.

- [x] Contamination-resistant pr-review arena exists: `pr-review-v0` `contamination.toml` records `public = false`; `arena-validate` blesses it "contamination-resistant: all sources private/synthetic."
- [x] **≥6 independent sources**: 6 tasks, each a distinct `source_repo` (auth-service, cart, file-cache, pagination, retry, rename) → 5 search clusters (df≥4, t≈2.78), unlike the 2-repo public arenas at df=1.
- [x] `arena-validate` passes oracle (1.0), null floor (0.1667), holdout ledger; `arena-redteam` shows **0 wide spans** (max 6 lines) — the line constraint demands real localization, not trivially gameable.
- [~] A run certifies *something* live at pr-review effect sizes — exercised by the two-seed run (also produces the `--probe-run` data to close the last `arena-validate` check).

**Caveat / further hardening:** pr-review-v0's synthetic code lives in this repo, so it is contamination-resistant *relative to the heavily-trained public-lib arenas* (rich/pygments), not air-gapped. A truly private holdout (code never committed to any indexable repo) remains the gold standard — track separately if leakage from this repo becomes a measured concern.

## Verification System
- Claim: a config certified on this arena is genuinely good, not leakage-inflated.
- Falsifier: a model known to have seen public rich/pygments scores no higher here than a model that hasn't (no leakage signal); a gaming candidate (`arena-redteam`) earns no undue reward.
- Driver: `daedalus arena-validate` + `daedalus arena-redteam` on the new arena; a live search (spend-gated) to confirm certifiability.
- Grader: the freeze-gate report + redteam audit + a certified-candidate CI that excludes 0.
- Evidence packet: the arena dir + its validity records + a run report.
- Cadence: at arena freeze (G2).

## Notes
Split out of [[040]] (item 4): launch-contract-v0 satisfies "a contamination-resistant arena exists" literally, but it validates launch-contract, not pr-review. Substantial fixture-authoring; benefits from operator input on the synthetic domains (what kinds of bugs, what code style). The t-correction finding makes the cluster-count requirement concrete: 2 sources is unusable, ~6+ is the floor for certifiability.
