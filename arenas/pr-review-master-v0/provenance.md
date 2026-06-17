# pr-review-master-v0 provenance

Prepared 2026-06-12 for backlog 034 as the first review-swarm master
synthesis arena.

This arena is a bootstrapped consolidation benchmark. Candidate-visible inputs
are synthetic specialist member artifacts, not real member outputs from a
certified swarm. G2 must later distinguish synthetic-member performance from
real-member replay performance before any full-swarm export is trusted.

Frozen surfaces for v0.1.0: task fixtures, answer keys, template, split
membership, taxonomy references, and scorer constants. Changing any answer key,
fixture, or template requires an arena version bump and rerunning oracle/null
baselines before comparing scores.

## v0.1.0 saturation

Run `runs/20260612T205852Z-freeze-pr-review-master-v0` recorded the G1-paid
one-shot probe after the initial no-spend oracle/null references. The probe
scored 1.000 against oracle 1.000, so v0.1.0 is saturated and must not be used
for comparative search.

The v0.1.0 run directory was append-extended after G1: the original
oracle/null JSONL records remain unchanged, and the probe records were appended
with regenerated summary/freeze-report artifacts.

This is a fixture failure, not an agent success: each task exposed a small,
direct member-artifact JSON file whose true defect could be inlined and
restated by a single completion.

## v0.2.0 headroom change

v0.2.0 keeps the same reducer contract and scorer constants, but changes the
frozen fixture surface:

- expands each candidate-visible `member_artifacts.json` to roughly 2 MB of
  varied synthetic member/prefilter noise, pushing the one-shot probe beyond
  the practical context window while preserving file-tool accessibility for
  agentic candidates;
- adds `dual-defect-conflict` and `gate-regression` to check multi-defect
  preservation, taxonomy ownership, and false-green gate synthesis;
- retains `missing-security-member` as holdout, with version-scoped exposure
  accounting in `holdout-ledger.md`;
- preserves the answer-key/scorer rule that weaker severity, wrong category,
  duplicate emission, or unsupported member-copying reduces reward.

Scores from v0.1.0 and v0.2.0 are not comparable and must not be averaged.

## v0.2.0 evidence

- Freeze packet: `runs/20260612T215810Z-freeze-pr-review-master-v020`
- Search packet: `runs/20260612T220412Z-search-pr-review-master`
- Current freeze report: `runs/20260612T220412Z-search-pr-review-master/freeze-report.md`
- Oracle mean: 1.0
- Null mean: 0.1667
- One-shot probe mean: 0.0; all six probe attempts failed with HTTP 400
  context-overflow errors and known `$0.0000` cost.
- Certified recommendation: `seed2-qwen3-7-plus-spec-first`
  (`qwen/qwen3.7-plus`, composition hash `491643a3b1de61e3`).
- Total known experiment spend including optimizer calls, certification, and
  holdout: `$0.5290`.
- Search caveat: the run used `--max-candidates 0`, so it certified a
  bounded seed comparison rather than a reflective child search.
- Arena caveat: generated member artifacts include candidate-visible synthetic
  triage metadata; real-member replay is required before full-swarm export.
- Meta-eval alarm: every agent passed the `clean-noise` false-positive trap,
  so future versions should make clean/noisy cases more discriminating.
