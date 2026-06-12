# G2 - Eval-quality review: arena launch-contract-v0 (v0.1.0)

- **Status:** accepted with sandbox-only waivers
- **Prepared:** 2026-06-12, after the corrected launch-contract certification run
- **Arena:** `arenas/launch-contract-v0` version `0.1.0`
- **Run:** `runs/20260612T153450Z-search-launch-contract-v0`
- **Supersedes:** `runs/20260612T024051Z-search-launch-contract-v0`

## Command

```sh
bin/daedalus run specs/launch-contract/taskspec.toml --rng-seed 3006 --budget-usd 4 --max-candidates 5 --trials 1 --certify-top 1 --certify-trials 3 --children-per-gen 2 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 1
```

The superseded run produced useful evidence but also revealed that the
mutation validator allowed `thinking = "high"` even though G1 approved only
`off`, `low`, and `medium`. The corrected run was made after the mutator was
changed to enforce `search.thinking_levels`; focused tests cover that boundary.

## Freeze Gate Evidence

- Oracle: 1.0 across all six launch-contract tasks.
- Null: 0.1667, matching the one clean task out of six.
- One-shot probe: 0.5333. The arena does not saturate, but the probe had two
  voided empty-content trials and should be treated as a noisy headroom check.
- Agent spread: present. Candidate means ranged from 0.0 for timeout-cutoff
  candidates to 0.85 for an uncertified seed and 0.72 for the certified
  recommendation.
- Certification: `g2b-g1a-seed2-glm-4-7-flash-spec-first` is the only
  certified recommendation, with at least three trials on every train and
  validation task.

## Evidence Artifacts

- `runs/20260612T153450Z-search-launch-contract-v0/report.md`
- `runs/20260612T153450Z-search-launch-contract-v0/pareto.json`
- `runs/20260612T153450Z-search-launch-contract-v0/loop.json`
- `runs/20260612T153450Z-search-launch-contract-v0/lineage.md`
- `runs/20260612T153450Z-search-launch-contract-v0/trials.jsonl`
- `runs/20260612T153450Z-search-launch-contract-v0/summary.json`
- `runs/20260612T153450Z-search-launch-contract-v0/trace.otel.json`
- `runs/20260612T153450Z-search-launch-contract-v0/artifacts.index`
- `runs/20260612T153450Z-search-launch-contract-v0/candidate-cutoffs.jsonl`
- `arenas/launch-contract-v0/holdout-ledger.md`

## Delivery Evidence

- Delivery package: `deliveries/launch-contract/`
- Contract: `deliveries/launch-contract/contract.toml`
- Persona: `deliveries/launch-contract/persona.md`
- Handoff: `deliveries/launch-contract/plane-handoff.md`
- Regression dry-run command:
  `runs/20260612T172229Z-regression-launch-contract-v0/regression-command.txt`
- Bitter Blossom dry-run packet:
  `deliveries/launch-contract/launch-dry-run/bitter-blossom.import-packet.toml`
- Olympus dry-run packet:
  `deliveries/launch-contract/launch-dry-run/olympus.import-packet.toml`

The dry-run import packets are non-deployable, require sandbox execution,
disallow primary-reviewer use, and refuse deployment because G3 is unsigned.

## Arena Findings For Human Decision

1. `write-before-g4`: the certified recommendation scored 0/3. This is the
   largest retained weakness and should block any quality claim stronger than
   "second-family contract-discovery evidence."
2. One-shot probe stability: two probe trials voided with empty model content,
   so probe mean is useful for non-saturation but not a stable baseline.
3. Certified-vs-uncertified tradeoff: the highest point estimates
   (`seed5` at 0.85, `g1a` at 0.825) are not certified. The exported
   recommendation is lower quality but has the required search-task trial
   counts.
4. Timeout cutoffs: Kimi/test-runner and one model-transplant child hit the
   420-second wall cap and were cut off under `--max-errors-per-candidate 1`.
5. Superseded-run defect: the first paid run was not used for certification
   because mutation accepted an out-of-G1 thinking level. That harness bug is
   fixed and covered by tests in this branch.

## Human Review State

- [x] Human reviewer accepts `launch-contract-v0` as an internal
      Daedalus second-family benchmark with the limitations above.
- [ ] Human reviewer requests `launch-contract-v0.1.1` calibration before any
      scores from this family are trusted.
- [ ] Human reviewer rejects this arena as too noisy or too weak.

## Human G2 Decision

Accepted by the operator on 2026-06-12 with these constraints:

- This approval is for internal Daedalus second-family benchmarking,
  contract-discovery, and sandbox-only delivery export.
- The documented `write-before-g4`, probe-noise, timeout, and certification
  caveats are waived for ticket 030 closure.
- This approval is not a public benchmark-quality claim.
- This approval is not deployment approval; G3 remains required before any
  launch, and unsigned packets remain sandbox-only and non-primary.

This is the human G2 approval record; it is not an agent self-approval.
