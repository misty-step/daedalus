# Holdout exposure ledger — pr-review-v0

Every `--final` scoring of holdout tasks is recorded here (`threshold run`
appends automatically at stage 4). When a holdout task accumulates **5
exposure entries**, it is burned: rotate it into train/validation and author a
replacement (version bump).

| date | version | run | candidates exposed | tasks |
|---|---|---|---|---|
| 2026-06-23 | 20260623T183514Z-search-cerberus-reviewer | g2b-seed2-kimi-k2-7-code-trace-callers, seed2-kimi-k2-7-code-trace-callers, g2a-g1a-seed3-gpt-5-4-mini-test-runner, seed3-gpt-5-4-mini-test-runner, seed5-deepseek-v4-pro-trace-callers, seed1-glm-5-2-spec-first, seed4-deepseek-v4-flash-spec-first | rs-retry-backoff x35 |

| 2026-06-25 | 20260625T161856Z-search-cerberus-reviewer | seed2-kimi-k2-7-code-trace-callers, g1b-seed2-kimi-k2-7-code-trace-callers, seed3-gpt-5-4-mini-test-runner, seed5-deepseek-v4-pro-trace-callers, seed6-glm-5-2-test-runner, seed4-deepseek-v4-flash-spec-first | rs-retry-backoff x30 |

