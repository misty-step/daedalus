# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 8 | 0 |
| seed3-qwen3-7-plus-skeptic | pi | qwen/qwen3.7-plus | 0364f14e074e130d | 10 | 0 |
| seed4-gpt-5-mini-checklist | pi | openai/gpt-5-mini | 4671c3e06a946bd8 | 10 | 0 |
| seed2-glm-4-7-flash-test-runner | pi | z-ai/glm-4.7-flash | 38e2624d7dff2829 | 6 | 1 |
| seed5-kimi-k2-6-test-runner | pi | moonshotai/kimi-k2.6 | 24a7bde906c45a9a | 6 | 0 |
| seed1-deepseek-v4-flash-checklist | pi | deepseek/deepseek-v4-flash | 05e59e2e8e9ddaaa | 10 | 0 |
| g1a-seed3-qwen3-7-plus-skeptic | pi | z-ai/glm-4.7-flash | 196352774b5cab55 | 16 | 0 |
| null | null | — | 40f2f6e05112c409 | 8 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 9f21bdb5c010ab18 | 8 | 8 |

## Mean reward per task (n trials in parentheses)

| candidate | py-export-clear | py-formatter-clean | py-formatter-missing-crash | py-live-lock | py-measure-normalize | py-padding-clean | py-plugin-cache | py-progress-speed | **overall** |
|---|---|---|---|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| seed3-qwen3-7-plus-skeptic | 1.00 (2) | 1.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 1.00 (2) | 1.00 (1) | **0.8000** |
| seed4-gpt-5-mini-checklist | 0.50 (2) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.80 (1) | 1.00 (1) | 1.00 (2) | 1.00 (1) | **0.6800** |
| seed2-glm-4-7-flash-test-runner | — | 1.00 (1) | 0.00 (1) | 1.00 (1) | 1.00 (1) | 0.00 (1) | — | 1.00 (1) | **0.6667** |
| seed5-kimi-k2-6-test-runner | — | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.80 (1) | 1.00 (1) | — | 1.00 (1) | **0.6333** |
| seed1-deepseek-v4-flash-checklist | 0.50 (2) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (2) | 1.00 (1) | **0.6000** |
| g1a-seed3-qwen3-7-plus-skeptic | 0.50 (2) | 1.00 (2) | 0.50 (2) | 0.00 (2) | 1.00 (2) | 0.50 (2) | 0.50 (2) | 0.50 (2) | **0.5625** |
| null | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | **0.2500** |
| probe-oneshot | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.1s |
| seed3-qwen3-7-plus-skeptic | $0.0270 | $0.2695 | 71.0s |
| seed4-gpt-5-mini-checklist | $0.0111 | $0.1113 | 48.8s |
| seed2-glm-4-7-flash-test-runner | unknown | unknown | 339.5s |
| seed5-kimi-k2-6-test-runner | $0.0754 | $0.4521 | 155.6s |
| seed1-deepseek-v4-flash-checklist | $0.0052 | $0.0517 | 60.7s |
| g1a-seed3-qwen3-7-plus-skeptic | $0.0173 | $0.2765 | 96.9s |
| null | $0.0000 | $0.0000 | 0.1s |
| probe-oneshot | $0.0000 | $0.0000 | 3.0s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- seed3-qwen3-7-plus-skeptic
- seed4-gpt-5-mini-checklist
- seed1-deepseek-v4-flash-checklist

## Recommendation

**g1a-seed3-qwen3-7-plus-skeptic** — mean reward 0.5625 at $0.0173/trial (96.9s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥2 trials per search task): g1a-seed3-qwen3-7-plus-skeptic. Recommendation restricted to certified candidates._

## Spend accounting

Known spend including optimizer calls, certification, and holdout: $1.3002.
