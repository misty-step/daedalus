# Arena schema v0.2: instruction template + fixture splits

Priority: P1
Status: ready
Estimate: S

## Goal
Arenas get one shared instruction template (per-task intent only varies) and a declared train/validation/holdout split, so instruction versioning stops being six copy-pastes and the search loop cannot overfit the benchmark.

## Non-Goals
- New fixtures (009 owns those)

## Oracle
- [ ] `arena.toml` declares `[template]` (file ref) and `[split]` lists; per-task `instruction.md` is generated or composed from template + intent at run time
- [ ] Changing the template once changes all tasks (verified by hash)
- [ ] Runner refuses to let the search loop score holdout tasks before final evaluation (enforced, tested in `bin/gate`)
- [ ] pr-review-v0 migrated; oracle/null revalidate

## Notes
Codex simplify (six duplicated instructions are versioning debt) + VeRO's
validation/test split as the anti-reward-hacking guard. Small, blocks 005.
