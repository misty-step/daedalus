# Introduce composition.v1 and the experiment-run contract

Priority: P0
Status: ready
Estimate: M

## Goal
Candidates become typed, hashed compositions (harness, model, prompt packet, tool policy, thinking, budgets) and every run leaves a self-contained run directory with retained evidence, so agent-vs-agent comparisons can attribute wins to specific slots.

## Non-Goals
- Planner/executor/critic graph topologies (slot for later, not built now)
- New harness adapters beyond pi + one-shot baseline

## Oracle
- [ ] Candidate manifests carry a `composition` version + computed manifest hash recorded in every run record; pi/harness version captured automatically (`pi --version`)
- [ ] Prompt packet is a referenced file (hashable), not an inline string
- [ ] `runner/run.py --trials 3` writes `runs/<exp-id>/` containing: immutable composition snapshots, per-trial JSONL, retained transcripts (pi stdout) and findings per trial, and a summary.json with per-task reward distributions (not just mean)
- [ ] One-shot becomes a `baseline` adapter inside the composition executor; the ad-hoc `KINDS` dict is gone
- [ ] `bin/gate` still passes; oracle/null revalidate under the new layout

## Notes
Codex lane finding #1 (attribution breaks first) + #3 (evidence retention).
Operator steer: comparisons are almost always composition-vs-composition.
Includes ratified deletion: `openrouter` demoted from peer kind to baseline
adapter.
