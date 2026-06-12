# G2 - Eval-quality review: arena pr-review-master-v0 (v0.2.0)

- **Status:** pending human G2 review
- **Prepared:** 2026-06-12, after the v0.2.0 headroom fix and bounded search
- **Arena:** `arenas/pr-review-master-v0` version `0.2.0`
- **Freeze run:** `runs/20260612T215810Z-freeze-pr-review-master-v020`
- **Search run:** `runs/20260612T220412Z-search-pr-review-master`

## Command

```sh
bin/daedalus run specs/pr-review-master/taskspec.toml --rng-seed 3406 --budget-usd 0.55 --max-candidates 0 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2
```

The run intentionally used `--max-candidates 0`: this was a bounded
landscape/certification pass after the v0.1 arena saturated, not a broad
reflective search. The goal was to prove the corrected arena can support
agent-vs-agent comparison and produce one certified master-synthesis baseline
inside the G1 low-risk synthetic spend boundary.

## Freeze Gate Evidence

- v0.1.0 saturation: `runs/20260612T205852Z-freeze-pr-review-master-v0`
  shows the one-shot probe at 1.000 against oracle 1.000. v0.1.0 is invalid
  for comparative search.
- v0.2.0 oracle: 1.0 across all six tasks.
- v0.2.0 null: 0.1667, matching one clean task out of six.
- v0.2.0 one-shot probe: 0.0. All six probe requests failed with HTTP 400
  from oversized inlined context and recorded known `$0.0000` cost.
- Current v0.2.0 holdout exposure: `missing-security-member x2`, under the
  burn threshold of 5.

## Search Evidence

- Recommended composition:
  `seed2-qwen3-7-plus-spec-first`
- Model: `qwen/qwen3.7-plus`
- Composition hash: `491643a3b1de61e3`
- Certified: yes, with at least two trials on every train and validation task
  plus two holdout trials.
- Mean reward: 1.0000 on every measured task.
- Cost: `$0.2158` across 12 candidate trials, `$0.0180` per trial.
- Total known experiment spend including optimizer calls, certification, and
  holdout: `$0.5290`.
- Latency: 93.6 seconds mean wall time per task.
- Comparison baseline: `seed1-kimi-k2-6-checklist` also scored 1.0000 on
  train+validation point estimate, but was slower and more expensive
  (`$0.0582` per trial, 367.2 seconds mean wall time).

## Evidence Artifacts

- `runs/20260612T220412Z-search-pr-review-master/report.md`
- `runs/20260612T220412Z-search-pr-review-master/pareto.json`
- `runs/20260612T220412Z-search-pr-review-master/loop.json`
- `runs/20260612T220412Z-search-pr-review-master/lineage.md`
- `runs/20260612T220412Z-search-pr-review-master/trials.jsonl`
- `runs/20260612T220412Z-search-pr-review-master/summary.json`
- `runs/20260612T220412Z-search-pr-review-master/trace.otel.json`
- `runs/20260612T220412Z-search-pr-review-master/artifacts.index`
- `runs/20260612T220412Z-search-pr-review-master/freeze-report.md`

## Arena Findings For Human Decision

1. Synthetic transfer caveat: this arena measures master synthesis over
   generated member artifacts. It does not prove transfer to artifacts emitted
   by certified real member agents.
2. Triage-metadata caveat: v0.2.0 member artifacts include candidate-visible
   synthetic triage metadata such as suppression and duplicate hints. This is
   useful for a reducer smoke benchmark but too label-like for a strong
   benchmark claim.
3. Search breadth caveat: this was a two-seed, no-child search. It certifies
   one cheap baseline for v0.2.0; it does not claim global optimality.
4. Clean-trap caveat: every agent passed `clean-noise`; the false-positive
   trap may be too easy to discriminate restraint.
5. Latency caveat: even the recommended Qwen composition averages 93.6
   seconds per task, and one validation certification task took 487 seconds.
   Suite-level wall time must account for member execution plus master
   synthesis before any sandbox import.
6. Context-overflow probe caveat: the one-shot probe is defeated
   mechanically by large artifacts. This proves non-saturation for the
   current harness, but the arena still needs real-member replay before a
   full swarm handoff is trusted.

## Human Review State

- [ ] Human reviewer accepts v0.2.0 as an internal synthetic master-synthesis
      benchmark for sandbox-only Daedalus review-swarm exploration.
- [ ] Human reviewer requests v0.2.1 calibration before trusting this family.
- [ ] Human reviewer rejects this arena as too synthetic or too latency-heavy.

No approval is granted by this packet. G3, G4, and G5 remain unsigned.
