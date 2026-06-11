# Certify pr-review-v2 as a contract-grade benchmark

Priority: P0
Status: ready
Estimate: L

## Goal
Turn `arenas/pr-review-v2` v0.2.0 from a promising scaled arena into a
publishable benchmark with demonstrated new-task agent spread, calibrated
keys, signed G2 evidence, and a delivery-quality comparative run.

## Non-Goals
- Deploying the PR-review agent (that is G3+ launch work)
- Changing scorer constants to make provisional tasks pass

## Oracle
- [ ] A fresh `bin/daedalus run specs/pr-review/taskspec.toml` against
      `arenas/pr-review-v2` v0.2.0 commits `report.md`, `pareto.json`,
      `loop.json`, `lineage.md`, `trials.jsonl`, `summary.json`, and
      `artifacts.index`
- [ ] Every train+validation task has n >= 5 trials for every recommended
      candidate; holdout candidates run at certification depth and are recorded
      in `holdout-ledger.md`
- [ ] The six new v0.2.0 tasks show measurable agent spread, or failures are
      promoted into an arena-iteration note before any cross-agent claim
- [ ] Category/span calibration findings such as `py-markup-escape` are
      adjudicated without weakening the grader; any key change bumps the arena
      version and reruns oracle/null/probe baselines
- [ ] `approvals/G2-pr-review-v2.md` exists with the freeze gate, run-record
      paths, residual risks, and human review state
- [ ] `bin/gate` green

## Children
1. Run the full v0.2.0 search sequentially with a recorded RNG seed and
   certification depth.
2. Convert any `arena-findings.md` alarms into either a v0.2.1 calibration
   patch or a written waiver.
3. Add the v2 G2 approval artifact and link it from `ROADMAP.md`.
4. Regenerate the PR-review delivery only from certified evidence.

## Notes
**Why:** product/eval lane. `ROADMAP.md` names the next full run, but
`arenas/pr-review-v2/provenance.md` still marks six new tasks provisional
until spread is established. This is the immediate long pole.
