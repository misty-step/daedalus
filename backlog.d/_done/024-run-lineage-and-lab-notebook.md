# Every run emits its lineage; the lab keeps a notebook

Priority: P0
Status: ready
Estimate: M

## Goal
Each run produces a human-readable `lineage.md` tracing how the final agent
contract was discovered — rig results, sampled space, every hypothesis with
its measured outcome and keep/discard decision, alarms, certification,
recommendation — and appends a summary entry to a committed `runs/NOTEBOOK.md`
so the lab accumulates "what works under what circumstances" across runs.

## Non-Goals
- Replacing raw run records (trials.jsonl stays the source of truth)
- External trace sinks (ticket 014 owns Langfuse)

## Oracle
- [ ] `runner/lineage.py` renders lineage.md purely from existing artifacts
      (rig.json, seed.json, loop.json, pareto.json, trials.jsonl) — works
      retroactively on the capstone run
- [ ] Per generation: parent, slot changed, hypothesis text, measured delta,
      improved verdict — hypotheses labeled confirmed / not confirmed
- [ ] `bin/daedalus` writes lineage.md at stage 5 and appends a NOTEBOOK.md
      entry (date, spec, arena+version, spend, stop reason, pick + hash,
      key findings, alarms); NOTEBOOK.md is committed
- [ ] Offline test renders a fixture experiment dir and asserts hypothesis
      and decision text appear
- [ ] `bin/gate` green

## Notes
Operator direction 2026-06-10: "we should be able to trace the work done to
get to the contract… better understanding of what works and what doesn't and
what experiments resulted in what results." Ticket 025's structured
predicted-effect field will sharpen confirmed/refuted labels; this ticket
ships the narrative from data that already exists.
