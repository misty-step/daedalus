# Make an arena with headroom: defeats the one-shot probe, spreads the agents

Priority: P0
Status: ready
Estimate: L

## Goal
Produce at least one arena that can actually *rank agent configurations*: the
one-shot saturation probe scores < 0.5, and diverse agent seeds show real
reward spread — the missing substrate without which agent-vs-agent search
(017/018) has zero gradient to climb.

## Non-Goals
- Visual/execution QA (ticket 012 owns that path)

## Oracle
- [ ] An arena task family where one-shot (full-workspace-inline) scores < 0.5
      mean and pi (or another agentic composition) scores ≥ 0.7, with run
      records as evidence
- [ ] The mechanism is documented: large-repo context overflow, hidden/
      retrieval-gated files, or a defect requiring a tool the one-shot path
      lacks
- [ ] Discrimination among agents, not just vs the probe: ≥2 distinct agent
      compositions (different model or packet stance) land measurably apart
      (mean reward gap > trial noise) — recorded as part of the freeze gate

## Notes
Evidence from pr-review-v1 (arenas/pr-review-v1/provenance.md): synthetic
cross-file defects do NOT defeat one-shot because the runner inlines every
workspace file. Candidate mechanisms: (1) snapshot a real repo large enough
that inlining is impractical and give the agent grep/read tools the one-shot
path lacks; (2) gate context behind retrieval; (3) require execution. This is
the binding constraint on the entire search redesign (016–018): on a saturated
arena every reasonable agent scores ~1.0 and the loop optimizes noise. Bumped
to P0 on 2026-06-10 as the long pole of the agent-vs-agent plan; sequence it
in parallel with 017/018. Discovered while building 009.
