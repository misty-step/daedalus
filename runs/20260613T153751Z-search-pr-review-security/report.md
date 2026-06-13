# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 3 | 0 |
| seed5-kimi-k2-6-checklist | pi | moonshotai/kimi-k2.6 | d112f8dd00b0f84b | 6 | 0 |
| seed1-qwen3-7-plus-skeptic | pi | qwen/qwen3.7-plus | 7fbfbfbdb1909b1c | 2 | 0 |
| seed3-glm-4-7-flash-spec-first | pi | z-ai/glm-4.7-flash | 67eaf3b04bf30b16 | 2 | 1 |
| seed4-gpt-5-mini-skeptic | pi | openai/gpt-5-mini | 724026968317addc | 4 | 0 |
| seed2-deepseek-v4-flash-checklist | pi | deepseek/deepseek-v4-flash | 743a42bac423828d | 2 | 0 |
| null | null | — | 40f2f6e05112c409 | 3 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 9f21bdb5c010ab18 | 3 | 3 |

## Mean reward per task (n trials in parentheses)

| candidate | py-markup-escape | py-padding-clean | py-save-token-leak | **overall** |
|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| seed5-kimi-k2-6-checklist | 0.50 (2) | 1.00 (2) | 1.00 (2) | **0.8333** |
| seed1-qwen3-7-plus-skeptic | 0.00 (1) | 1.00 (1) | — | **0.5000** |
| seed3-glm-4-7-flash-spec-first | 0.00 (1) | 1.00 (1) | — | **0.5000** |
| seed4-gpt-5-mini-skeptic | 0.00 (1) | 1.00 (1) | 0.50 (2) | **0.5000** |
| seed2-deepseek-v4-flash-checklist | 0.80 (1) | 0.00 (1) | — | **0.4000** |
| null | 0.00 (1) | 1.00 (1) | 0.00 (1) | **0.3333** |
| probe-oneshot | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.1s |
| seed5-kimi-k2-6-checklist | $0.0417 | $0.2502 | 122.3s |
| seed1-qwen3-7-plus-skeptic | $0.0217 | $0.0434 | 96.3s |
| seed3-glm-4-7-flash-spec-first | unknown | unknown | 305.6s |
| seed4-gpt-5-mini-skeptic | $0.0040 | $0.0160 | 15.2s |
| seed2-deepseek-v4-flash-checklist | $0.0112 | $0.0225 | 185.5s |
| null | $0.0000 | $0.0000 | 0.0s |
| probe-oneshot | $0.0000 | $0.0000 | 1.6s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- seed5-kimi-k2-6-checklist
- seed4-gpt-5-mini-skeptic

## Recommendation

**seed5-kimi-k2-6-checklist** — mean reward 0.8333 at $0.0417/trial (122.3s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥2 trials per search task): seed5-kimi-k2-6-checklist. Recommendation restricted to certified candidates._

## Spend accounting

Known spend including optimizer calls, certification, and holdout: $0.3527.
