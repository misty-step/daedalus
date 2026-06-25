# Air-gapped private holdout with more independent source repos

Priority: P0 · Status: pending · Estimate: L

> Child of [[054]] — hum bar gap #4. **Urgency:** the lone holdout
> `rs-retry-backoff` was exposed 35× in the 2026-06-23 search (burn rule = 5) and
> is now burned; rebuilding the holdout likely gates [[052]] — the capability
> search needs a clean holdout to certify against.

## Goal
A pr-review holdout arena whose code was **never committed to any indexable
repo** (truly air-gapped, not just in-repo synthetic) with **≥8–10 independent
source repos**, so a certified win reflects review skill on unseen code *and* the
cluster count makes the proof statistically achievable.

## Why (the binding constraint on the whole mission)
Two coupled validity gaps the 2026-06-23 eval mapping surfaced:
- **Transfer:** the public arenas (rich/pygments) measure recall of *memorized*
  upstream bugs; the contamination-resistant `pr-review-v0` is synthetic but
  **lives in this repo**, so it's resistant *relative to* the public arenas, not
  air-gapped (043's own caveat).
- **Certifiability:** cluster-robust certification uses Student-t at `df = G−1`
  over `source_repo` clusters. A 2-repo arena → df=1 → t=12.7 → certifies almost
  nothing; the SE shrinks as `1/√G` over *clusters*, not tasks. `pr-review-v0`
  reaches df≥4 (6 repos); the live correctness run's seed wins all had CIs
  spanning 0 (040/039). More independent repos is the only lever that makes
  "provably better" reachable.

This is why the certified Cerberus run (`pr-review-v0`, df≥4) *could* certify
3 candidates — and why pushing clusters higher tightens every future CI.

## Oracle
- [ ] An air-gapped pr-review arena exists with ≥8 independent `source_repo`s of
      novel code not present in any public/indexable repository (generated-per-run
      defects, or gated/encrypted fixtures), `contamination.toml` `public=false`.
- [ ] `arena-validate` blesses it (oracle 1.0, null floor, holdout ledger);
      `arena-redteam` shows 0 wide spans.
- [ ] A certified search on it produces a candidate whose reward-delta 95% CI
      excludes 0 with df≥7 (t≈2.36 or tighter) — a materially stronger proof than
      `pr-review-v0`'s df≥4.
- [ ] `bin/gate` passes.

## Notes
Pairs with [[043]] (which designated the in-repo synthetic holdout as the
interim) and [[040]] (contamination + cluster theory). The single biggest threat
to transfer validity + the binding constraint on certifiability.
