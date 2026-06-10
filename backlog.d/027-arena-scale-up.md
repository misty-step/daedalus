# Scale the arena: task volume, repo diversity, rotation, holdout ledger

Priority: P1
Status: ready
Estimate: L

## Goal
Enough independent tasks that the statistics can work and packet evolution
cannot memorize the key: 10–20 tasks across ≥2 repos covering the defect
taxonomy, split sizes that make train/validation/holdout meaningful, fixture
rotation between arena versions, and a ledger that tracks holdout exposure.

## Non-Goals
- Visual/execution QA arenas (ticket 012)
- Judge scoring (ticket 010)

## Oracle
- [ ] ≥10 tasks across ≥2 distinct repos; every category in the taxonomy
      seeded at least once; ≥2 clean FP-traps; holdout ≥3 tasks
- [ ] Freeze gate (incl. agent-spread and probe checks) re-run and recorded
- [ ] Holdout-exposure ledger: each `--final` scoring appends (date, run,
      candidates) to the arena's provenance; the freeze gate names a burn
      threshold after which holdout rotates
- [ ] A defect-authoring pipeline note: how tasks were generated/verified
      (oracle+key authored together; defects provably absent from upstream)
- [ ] Explore auto-generated defects (mutation-testing style: seeded changes
      that demonstrably flip an existing test) — adopt or reject with a
      recorded reason

## Notes
Single-author, single-repo, 3-defect arenas measure one skill and leak
style: the capstone's spec-first winner partly reflects spec-shaped defect
authoring. Volume + diversity is the real Goodhart defense; everything else
is noise management. Supersedes nothing — 019 (v2.1 calibration) lands
first and this builds on it.
