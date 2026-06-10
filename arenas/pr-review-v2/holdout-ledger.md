# Holdout exposure ledger — pr-review-v2

Every `--final` scoring of holdout tasks is recorded here (bin/daedalus
appends automatically at stage 4; manual `runner/run.py --final` holdout
runs must be logged by hand). When a holdout task accumulates **5 exposure
entries**, it is burned: rotate it into train/validation and author a
replacement (version bump).

| date | run | candidates exposed | tasks |
|---|---|---|---|
| 2026-06-10 | 20260610T160533Z-search-pr-review-v0 (arena v0.1.0) | g1b-seed1-glm-5-spec-first, seed4-gpt-5-mini-spec-first | py-live-lock |
| 2026-06-10 | delivered-agent certification repro (arena v0.1.0) | pr-review-glm5-specfirst-medium | py-live-lock ×2 runs |
