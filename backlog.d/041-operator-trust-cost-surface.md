# Operator trust & cost surface — forecast, compare, read trust at a glance

Priority: P1 · Status: delivered · Estimate: M

## Goal
An operator can forecast a search's cost before committing, compare two runs mechanically, and read a result's trustworthiness (CI, consistency, early-stop reason) without grepping prose.

## Oracle
- [x] `daedalus run --estimate` prints projected scale/cost (compositions × tasks × trials × per-trial ceiling) and exits without spending. Offline forecast in `crates/daedalus-core/src/forecast.rs`; `--estimate` early-returns before `runs/` is created (zero trials). Cost is the worst-case ceiling and prints only when the taskspec declares `[budget].max_cost_per_trial_usd`, else "unknown" (AGENTS: unknown cost is null). Latency forecasting deferred — see deferral note below.
- [x] `daedalus compare <runA> <runB>` emits a delta table — per-candidate reward, rank change, cost — so cross-run comparison is not manual report-diffing. Pure delta in `crates/daedalus-core/src/compare.rs`; reads each run's pareto.json + loop.json. Adds run-level spend + stop-reason deltas. Unknown cost prints "—"/"unknown", never 0.
- [x] report.md states WHY the search stopped (budget / plateau / max-candidates) via `report::verdict_markdown`, alongside the recommended/certified summary. The CI + consistency signals from [[039]] (`stats::delta_ci_markdown` / `consistency_markdown`) were already surfaced in report.md; this ticket adds the missing stop-reason/verdict section.

## Deferral
- **Latency forecast:** `--estimate` projects trial count and worst-case cost, not wall-clock. Per-trial latency is not bounded by the taskspec the way cost is (`[budget].max_wall_per_trial_sec` is an abort ceiling, not a predictor), and projecting wall time honestly needs a historical model per (model × thinking level) that does not yet exist. Forecasting trial count + cost ceiling is the high-leverage, honest half; wall-clock projection is a follow-up once run history supports it.

## Notes
From the operator-experience lane: spend is known only post-hoc (`main.rs:148`, `search_loop.rs:297`); cross-run comparison is manual against `holdout-ledger.md`; trust is an unsurfaced trial count. Cheap, offline, high-leverage for "really experimenting" at volume. Surfacing depends on [[039]] delivering the trust metrics. A `--plateau` early-stop is currently invisible to the operator and can fire after as few as ~6 trials at default settings — make the stop reason explicit.
