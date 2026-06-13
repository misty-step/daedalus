# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 7 | 0 |
| seed4-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | 9720f530f1a28cf0 | 5 | 0 |
| seed5-kimi-k2-6-test-runner | pi | moonshotai/kimi-k2.6 | a907bf70ed95f25e | 5 | 1 |
| seed1-gpt-5-mini-spec-first | pi | openai/gpt-5-mini | f090f8060cf36637 | 14 | 0 |
| seed2-deepseek-v4-flash-test-runner | pi | deepseek/deepseek-v4-flash | 8c4ee4de5cbd36ad | 5 | 0 |
| seed3-glm-4-7-flash-skeptic | pi | z-ai/glm-4.7-flash | cb4ee6db50b7bbd3 | 9 | 0 |
| null | null | — | 40f2f6e05112c409 | 7 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 9f21bdb5c010ab18 | 7 | 7 |

## Mean reward per task (n trials in parentheses)

| candidate | py-export-clear | py-formatter-clean | py-live-lock | py-measure-normalize | py-padding-clean | py-plugin-cache | py-progress-speed | **overall** |
|---|---|---|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| seed4-qwen3-7-plus-spec-first | — | 1.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | — | 1.00 (1) | **0.6000** |
| seed5-kimi-k2-6-test-runner | — | 1.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | — | 1.00 (1) | **0.6000** |
| seed1-gpt-5-mini-spec-first | 0.00 (2) | 1.00 (2) | 0.50 (2) | 0.50 (2) | 0.00 (2) | 1.00 (2) | 1.00 (2) | **0.5714** |
| seed2-deepseek-v4-flash-test-runner | — | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | — | 1.00 (1) | **0.4000** |
| seed3-glm-4-7-flash-skeptic | 0.00 (2) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.50 (2) | 0.00 (1) | **0.3333** |
| null | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | **0.2857** |
| probe-oneshot | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.1s |
| seed4-qwen3-7-plus-spec-first | $0.0180 | $0.0902 | 76.1s |
| seed5-kimi-k2-6-test-runner | unknown | unknown | 278.7s |
| seed1-gpt-5-mini-spec-first | $0.0103 | $0.1447 | 49.6s |
| seed2-deepseek-v4-flash-test-runner | $0.0108 | $0.0542 | 161.7s |
| seed3-glm-4-7-flash-skeptic | $0.0026 | $0.0233 | 68.8s |
| null | $0.0000 | $0.0000 | 0.1s |
| probe-oneshot | $0.0000 | $0.0000 | 3.0s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- seed4-qwen3-7-plus-spec-first
- seed1-gpt-5-mini-spec-first
- seed3-glm-4-7-flash-skeptic

## Recommendation

**seed1-gpt-5-mini-spec-first** — mean reward 0.5714 at $0.0103/trial (49.6s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥2 trials per search task): seed1-gpt-5-mini-spec-first. Recommendation restricted to certified candidates._

## Spend accounting

Known spend including optimizer calls, certification, and holdout: $0.6253.
