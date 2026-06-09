# Daedalus repo contracts

- **Gate:** `bin/gate` (offline pytest + compile check). Run it before
  claiming done. Never weaken a test to get green.
- **The grader is gospel.** Changes to `runner/score.py`, answer keys, or
  scorer constants require an arena version bump (`arenas/<id>/arena.toml`)
  and re-running oracle/null baselines. Never average rewards across arena
  versions.
- **Candidates never read `tests/` or `solution/`** in any arena task, and
  experiment code must never grant them that access.
- **Run records are evidence.** `runs/*.jsonl` is append-only history; never
  edit or delete committed records. Unknown cost is `null`, never an estimate.
- **Gates G1–G5** (spec, eval quality, launch, permissions, prod-data
  re-ingestion) are human approvals in `approvals/`. Do not self-approve.
- Architecture and schemas: `DESIGN.md`. Phases: `ROADMAP.md`. Work items:
  `backlog.d/` (Goal + Oracle required; close via `Closes-backlog: <id>`
  commit trailers).
