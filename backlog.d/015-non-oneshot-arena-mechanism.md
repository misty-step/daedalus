# Make an arena that genuinely defeats the one-shot baseline

Priority: P1
Status: ready
Estimate: L

## Goal
Produce at least one arena where the cheap one-shot baseline scores < 0.5 while
an agentic composition scores well — the missing proof that harness/tools
matter, which pr-review-v1 failed to provide.

## Non-Goals
- Visual/execution QA (ticket 012 owns that path)

## Oracle
- [ ] An arena task family where one-shot (full-workspace-inline) scores < 0.5
      mean and pi (or another agentic composition) scores ≥ 0.7, with run
      records as evidence
- [ ] The mechanism is documented: large-repo context overflow, hidden/
      retrieval-gated files, or a defect requiring a tool the one-shot path
      lacks

## Notes
Evidence from pr-review-v1 (arenas/pr-review-v1/provenance.md): synthetic
cross-file defects do NOT defeat one-shot because the runner inlines every
workspace file. Candidate mechanisms: (1) snapshot a real repo large enough
that inlining is impractical and give the agent grep/read tools the one-shot
path lacks; (2) gate context behind retrieval; (3) require execution. This is
the binding constraint on proving agent-vs-agent differences for the PR-review
domain. Discovered while building 009.
