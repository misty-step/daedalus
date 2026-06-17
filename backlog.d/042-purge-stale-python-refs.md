# Purge stale post-migration Python references from docs

Priority: P2 · Status: ready · Estimate: S

## Goal
No authoritative doc tells a cold agent or operator to run deleted Python; every reference points at the Rust reality.

## Oracle
- [ ] `grep -rnE 'runner/|python3|pytest|\.py\b' DESIGN.md ROADMAP.md docs/ backlog.d/034-*.md` returns only intentional historical-evidence lines, each clearly annotated "pre-migration".
- [ ] DESIGN.md anchors corrected: `runner/score.py|judge.py|trace.py|launch.py` → `crates/daedalus-core/src/{score,judge,trace,launch}.rs`.
- [ ] `docs/adr-004-review-swarm-contract.md:58` `python3 -m pytest …` → `bin/gate` (the canonical gate).
- [ ] 034's Repo Anchors point at Rust CLI equivalents (`daedalus export-suite` / `taxonomy-validate` / the swarm path), not `runner/*.py`.

## Notes
Same staleness class the /ci pass already fixed in AGENTS.md. Found refs (investigation lane): DESIGN.md:34,107,113,211,256 · adr-004:58 · ROADMAP.md:108,158-159 (historical phase notes — annotate) · docs/review-swarm-vertical-slice.md:61,142-160 (historical evidence — annotate, don't delete) · backlog.d/034:110,423,425. Cheap; do it before the docs mislead the next cold agent. DESIGN.md and adr-004 are the load-bearing fixes.
