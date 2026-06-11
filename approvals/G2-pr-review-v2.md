# G2 - Eval-quality review: arena pr-review-v2 (v0.2.0)

- **Status:** accepted with sandbox-only waivers
- **Prepared:** 2026-06-11, after the v0.2.0 certification run
- **Arena:** `arenas/pr-review-v2` version `0.2.0`
- **Run:** `runs/20260611T173632Z-search-pr-review-v0`

## Command

```sh
bin/daedalus run specs/pr-review/taskspec.toml --rng-seed 2806 --budget-usd 8 --max-candidates 6 --trials 1 --certify-top 1 --certify-trials 5 --children-per-gen 2 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 1
```

`--max-errors-per-candidate 1` was added after an aborted diagnostic run
(`runs/20260611T161435Z-search-pr-review-v0`) showed repeated 600-second
timeouts for noncompetitive candidates. Cutoffs are recorded in
`candidate-cutoffs.jsonl` and surfaced in `report.md`.

## Freeze Gate Evidence

- Oracle: 1.0 across all 10 tasks.
- Null: 0.20, matching the two clean tasks out of ten.
- One-shot probe: 0.0; all probe attempts failed on context overflow rather
  than saturating the benchmark.
- New-task spread: present, but not clean enough for a public quality claim.
  Candidate means ranged from 0.0 (`seed2-kimi-k2-6-trace-callers`, cutoff
  after timeout) to 0.6857 (`g2b`, uncertified). The only certified
  recommendation is `seed4-qwen3-7-plus-checklist` at 0.5714.
- Certification: `seed4-qwen3-7-plus-checklist` has n >= 5 on every
  train+validation task and is the only recommendable candidate under the
  current runner.

## Evidence Artifacts

- `report.md`, `pareto.json`, `loop.json`, `lineage.md`, `trials.jsonl`,
  `summary.json`, `artifacts.index`
- `arena-findings.md` records certification failures and the clean-trap alarm.
- `candidate-cutoffs.jsonl` records skipped candidate splits after timeout.
- `arenas/pr-review-v2/holdout-ledger.md` records the 20260611 holdout
  exposure for `g1a`, `g3b`, and `seed3`.

## Fresh-Context Critic Receipt

Read-only peer review was attempted with two model-family lanes:

- `pi` / OpenRouter `moonshotai/kimi-k2.6` timed out after 420 seconds with no
  output.
- `claude --print --model sonnet` completed a read-only review of this packet,
  the delivery handoff, and the run report. It found no blocking non-human gap
  before human G2. Non-blocking notes were limited to the intentionally
  absolute prompt-packet path used for composition hash preservation and the
  disclosed fact that the one-shot probe voided on context overflow rather than
  producing a usable saturation signal.

Critic verdict: the certification packet is ready for human G2 review; the
remaining open items are human decisions, not concealed branch work.

## Arena Findings For Human Decision

1. `py-markup-escape`: certified seed4 scored 0/5. Prior probes found the
   right location but disagreed with the strict `security` category. Decide
   whether this is a category-set calibration issue, a key-category change
   requiring v0.2.1, or a waiver.
2. `py-guess-swallow`: certified seed4 scored 0/5. Some uncertified
   candidates can hit it at small n, so it is passable but may need stronger
   execution/retrieval affordance.
3. `py-measure-normalize`: certified seed4 scored 1/5 after a search-phase
   hit. Treat the original hit as variance, not stable capability.
4. `py-formatter-clean`: every agent passed this clean validation task; it may
   be too easy to discriminate false-positive discipline.
5. Uncertified candidates `g2b` and `g3b` look better than the certified
   recommendation on point estimate, but they cannot be recommended without
   n >= 5 on train+validation.

## Human Review State

- [x] Human reviewer accepts v0.2.0 as a contract-grade benchmark for internal
      Daedalus-to-plane handoff despite the residual findings.
- [ ] Human reviewer requests v0.2.1 calibration before publication.
- [x] Human reviewer signs a written waiver for any task retained despite the
      failures above.

## Human G2 Decision

Accepted by the operator on 2026-06-11 with these constraints:

- This approval is for honing Daedalus contract discovery and handoff loops,
  not for claiming an enterprise-ready code-review agent.
- Bitter Blossom may only receive this packet experimentally in a sandboxed
  mode; it must not operate as the primary reviewer.
- No public cross-agent benchmark-quality claim may depend on this run.
- The documented `py-markup-escape`, `py-guess-swallow`,
  `py-measure-normalize`, `py-formatter-clean`, and one-shot probe limitations
  are waived for 028 closure only and should remain calibration follow-up.

This is the human G2 approval record; it is not an agent self-approval.
