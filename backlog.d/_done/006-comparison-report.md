# Emit a comparison report artifact per experiment

Priority: P1
Status: ready
Estimate: M

## Goal
Every experiment ends with a human-readable report the operator can act on: per-composition reward distributions, pairwise per-task table, Pareto set over quality/cost/latency, error/void counts, and a recommendation block.

## Non-Goals
- Dashboards or web UI; markdown + JSON only
- Statistical significance machinery beyond min/mean/max + trial counts (note n everywhere)

## Oracle
- [ ] `runner/report.py runs/<exp-id>/` writes `report.md` with: composition table (slots that differ highlighted), per-task × per-composition reward grid, cost/latency totals, Pareto set, voided-trial accounting
- [ ] Works on Phase 0 records (re-run over existing runs/ JSONL) and on 002's run directories
- [ ] The G2 reviewer can answer "which composition wins under the taskspec mode and why" from the report alone

## Notes
Codex lane: mean-only aggregates are the first orchestration break. Standalone
value before 005 lands; 005 then calls it.
