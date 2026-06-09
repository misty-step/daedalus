# Build `daedalus run`: the autonomous search loop

Priority: P0
Status: ready
Estimate: L

## Goal
One command takes a taskspec and runs the whole loop — validate rig → baselines → propose composition → run trials → score → reflect → next candidate — stopping at budget, plateau, turn cap, or threshold, and emitting a Pareto archive plus comparison report; the operator only answers interview questions and reads reports.

## Non-Goals
- Arena *generation* from scratch (loop assumes an arena exists)
- Multi-objective optimizers (NSGA-II etc.) — Pareto archive + single-slot reflective mutation only
- Deploy/runtime anything

## Oracle
- [ ] `daedalus run specs/pr-review/taskspec.toml --budget-usd 2 --max-candidates 6` completes unattended: validates oracle/null, runs both baselines, then ≥3 generated compositions, each n≥3 trials
- [ ] Candidate proposals come from the reflection step (007) reading worst-trial transcripts; every mutation differs from its parent in exactly one slot
- [ ] Search scores against the train/validation split only (008); holdout is scored once, at the end, for the final report
- [ ] Emits `runs/<exp-id>/report.md` (006) + `pareto.json` (quality/cost/latency archive) + stop reason
- [ ] Total spend recorded and ≤ budget; loop halts on plateau (2 consecutive non-improving candidates)

## Notes
pi lane premise reframe: the manual loop is the binding constraint on
"usable" — the MVP session breaks at "operator must hand-drive the next
candidate". VeRO guards imported: validation/holdout split vs reward hacking;
best-commit-found-early ⇒ plateau stop. Master-agent LLM calls go through
OpenRouter (or `claude -p`) — pick during implementation spike, record an ADR
in the ticket on completion.
