# Operator trust & cost surface — forecast, compare, read trust at a glance

Priority: P1 · Status: pending · Estimate: M

## Goal
An operator can forecast a search's cost before committing, compare two runs mechanically, and read a result's trustworthiness (CI, consistency, early-stop reason) without grepping prose.

## Oracle
- [ ] `daedalus run --estimate` (or `--dry-run`) prints projected cost/latency (compositions × tasks × trials × per-trial ceiling) and exits without spending.
- [ ] `daedalus compare <runA> <runB>` emits a delta table — per-candidate reward, rank change, cost — so cross-run comparison is not manual report-diffing.
- [ ] report.md states WHY the search stopped (budget / plateau / max-candidates) and surfaces the CI + consistency signals from [[039]].

## Notes
From the operator-experience lane: spend is known only post-hoc (`main.rs:148`, `search_loop.rs:297`); cross-run comparison is manual against `holdout-ledger.md`; trust is an unsurfaced trial count. Cheap, offline, high-leverage for "really experimenting" at volume. Surfacing depends on [[039]] delivering the trust metrics. A `--plateau` early-stop is currently invisible to the operator and can fire after as few as ~6 trials at default settings — make the stop reason explicit.
