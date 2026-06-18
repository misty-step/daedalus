# Daedalus repo contracts

- **Gate:** `bin/gate` (offline `cargo fmt --check` + `cargo test --workspace`
  + `cargo clippy -D warnings`). Run it before claiming done. Never weaken a
  test to get green.
- **The grader is gospel.** Changes to the scorer
  (`crates/daedalus-core/src/score.rs`), answer keys, or scorer constants
  require an arena version bump (`arenas/<id>/arena.toml`)
  and re-running oracle/null baselines. Never average rewards across arena
  versions.
- **Latest models only.** `docs/primitives.md` is the verified model pool —
  only each provider's *latest* model per tier; a superseded version is
  removed, not kept alongside (e.g. glm-5 → glm-5.2, gpt-5-mini → gpt-5.4-mini).
  Every `specs/*/taskspec.toml [search].models` entry must exist in the pool.
  `daedalus doctor` enforces both halves: `model-primitives` (pool re-verified
  within `--stale-days`) and `roster-in-pool` (no taskspec model outside the
  pool). Re-verify against OpenRouter `/api/v1/models` before adding a model.
  Optimizer default: `deepseek/deepseek-v4-pro` (escalate to `openai/gpt-5.5`
  or `anthropic/claude-opus-4.8` for a high-stakes final search).
- **Candidates never read `tests/` or `solution/`** in any arena task, and
  experiment code must never grant them that access.
- **Run records are evidence.** `runs/*.jsonl` is append-only history; never
  edit or delete committed records. Unknown cost is `null`, never an estimate.
- **Gates G1–G5** (spec, eval quality, launch, permissions, prod-data
  re-ingestion) are human approvals in `approvals/`. Do not self-approve.
- Architecture and schemas: `DESIGN.md`. Phases: `ROADMAP.md`. Work items:
  `backlog.d/` (Goal + Oracle required; close via `Closes-backlog: <id>`
  commit trailers).
- Philosophy, vision, and operating principles: `docs/philosophy.md`. Keep
  delivery choices consistent with that document, or update it deliberately
  when the philosophy changes.
