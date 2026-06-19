# Holdout exposure ledger — pr-review-v2

Every final scoring of holdout tasks is recorded here. `cargo run --quiet --bin
daedalus -- run ...` appends automatically at stage 4; manual holdout replays
through `cargo run --quiet --bin daedalus -- regression ...` must be logged by
hand if they expose holdout tasks outside that stage. When a holdout task
accumulates **5 exposure entries**, it is burned: rotate it into
train/validation and author a replacement (version bump).

| date | run | candidates exposed | tasks |
|---|---|---|---|
| 2026-06-10 | 20260610T160533Z-search-pr-review-v0 (arena v0.1.0) | g1b-seed1-glm-5-spec-first, seed4-gpt-5-mini-spec-first | py-live-lock |
| 2026-06-10 | delivered-agent certification repro (arena v0.1.0) | pr-review-glm5-specfirst-medium | py-live-lock ×2 runs |
| 20260611 | 20260611T173632Z-search-pr-review-v0 | g1a-seed1-gpt-5-mini-checklist, g3b-g2b-seed1-gpt-5-mini-checklist, seed3-glm-4-7-flash-test-runner | py-live-lock, py-export-clear, py-plugin-cache |
