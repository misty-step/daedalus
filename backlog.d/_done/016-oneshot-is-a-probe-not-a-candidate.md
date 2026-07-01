# Demote one-shot to a rig probe; retract the agent-vs-oneshot framing

Priority: P0
Status: ready
Estimate: S

## Goal
One-shot can never be a candidate, win a Pareto slot, or be recommended: it
becomes a reference probe (like null/oracle) whose only job is detecting arena
saturation; the "first lab finding" is reframed in the docs as the meta-eval
result it actually was.

## Non-Goals
- Deleting the oneshot executor kind (it stays, as the saturation probe)
- Redesigning the search loop (ticket 018)

## Oracle
- [ ] `REFERENCE` in runner/report.py and runner/loop.py includes the one-shot
      probe; it appears in the report grid but is mechanically excluded from
      Pareto, recommendation, and parent selection (test proves it)
- [ ] `bin/threshold` stage 1 runs the probe and flags the arena **saturated**
      when probe mean ≥ oracle − 0.1; a saturated arena aborts the search by
      default (override flag exists, prints a loud warning into report.md)
- [ ] ROADMAP.md, DESIGN.md, and arenas/pr-review-v1/provenance.md no longer
      present "one-shot ties agentic at lower cost → recommend baseline" as a
      candidate finding; it is restated as "pr-review-v0/v1 are saturated and
      cannot rank agent configurations"
- [ ] `bin/gate` green

## Notes
Operator correction 2026-06-10: this domain is always agentic; there is no
one-shot deployment target, so recommending `baseline-oneshot` was a category
error. The probe still earns its keep via the DESIGN.md meta-eval item "does
a cheap baseline expose that the benchmark is too easy?" — that is its entire
role. Rename `candidates/baseline-oneshot.toml` → `candidates/probe-oneshot.toml`
to make the role unmistakable.
