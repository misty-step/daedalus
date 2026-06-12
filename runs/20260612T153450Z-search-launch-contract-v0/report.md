# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 6 | 0 |
| seed5-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | 1ffe9e7eb74fe46e | 8 | 0 |
| g1a-seed2-glm-4-7-flash-spec-first | pi | z-ai/glm-4.7-flash | fd911b8153cff0d3 | 8 | 0 |
| g2b-g1a-seed2-glm-4-7-flash-spec-first | pi | qwen/qwen3.7-plus | 7523f6b853908df2 | 15 | 0 |
| seed2-glm-4-7-flash-spec-first | pi | z-ai/glm-4.7-flash | 6f310b2019f6c4cf | 8 | 0 |
| g2a-seed5-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | db88db4b04e3b277 | 5 | 0 |
| g3a-g1a-seed2-glm-4-7-flash-spec-first | pi | z-ai/glm-4.7-flash | fdbd58f33448090d | 5 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 9f21bdb5c010ab18 | 6 | 2 |
| seed3-gpt-5-mini-skeptic | pi | openai/gpt-5-mini | 6a2351e2671d690f | 5 | 0 |
| seed4-deepseek-v4-flash-test-runner | pi | deepseek/deepseek-v4-flash | a1b6629b8000fa2a | 5 | 0 |
| null | null | — | 40f2f6e05112c409 | 6 | 0 |
| seed1-kimi-k2-6-test-runner | pi | moonshotai/kimi-k2.6 | 1c6fb038eedd0f62 | 1 | 1 |
| g1b-seed5-qwen3-7-plus-spec-first | pi | z-ai/glm-4.7-flash | 640d5abc81b196ba | 1 | 1 |

## Mean reward per task (n trials in parentheses)

| candidate | absolute-prompt-path | bb-unsigned-primary | contract-missing-evidence | sandbox-clean | trace-waiver-missing | write-before-g4 | **overall** |
|---|---|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| seed5-qwen3-7-plus-spec-first | 1.00 (3) | 1.00 (1) | 1.00 (1) | 0.00 (1) | 0.80 (1) | 1.00 (1) | **0.8500** |
| g1a-seed2-glm-4-7-flash-spec-first | 1.00 (3) | 0.80 (1) | 1.00 (1) | 1.00 (1) | 0.80 (1) | 0.00 (1) | **0.8250** |
| g2b-g1a-seed2-glm-4-7-flash-spec-first | — | 0.60 (3) | 1.00 (3) | 1.00 (3) | 1.00 (3) | 0.00 (3) | **0.7200** |
| seed2-glm-4-7-flash-spec-first | 0.60 (3) | 1.00 (1) | 0.80 (1) | 1.00 (1) | 0.60 (1) | 0.00 (1) | **0.6500** |
| g2a-seed5-qwen3-7-plus-spec-first | — | 0.80 (1) | 0.80 (1) | 1.00 (1) | 0.60 (1) | 0.00 (1) | **0.6400** |
| g3a-g1a-seed2-glm-4-7-flash-spec-first | — | 1.00 (1) | 1.00 (1) | 0.00 (1) | 0.80 (1) | 0.00 (1) | **0.5600** |
| probe-oneshot | 1.00 (1) | 0.80 (1) | 0.80 (1) | 0.00 (1) | 0.60 (1) | 0.00 (1) | **0.5333** |
| seed3-gpt-5-mini-skeptic | — | 0.80 (1) | 0.80 (1) | 0.00 (1) | 0.60 (1) | 0.00 (1) | **0.4400** |
| seed4-deepseek-v4-flash-test-runner | — | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | **0.2000** |
| null | 0.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | **0.1667** |
| seed1-kimi-k2-6-test-runner | — | 0.00 (1) | — | — | — | — | **0.0000** |
| g1b-seed5-qwen3-7-plus-spec-first | — | 0.00 (1) | — | — | — | — | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.0s |
| seed5-qwen3-7-plus-spec-first | $0.0083 | $0.0660 | 43.5s |
| g1a-seed2-glm-4-7-flash-spec-first | $0.0026 | $0.0206 | 69.4s |
| g2b-g1a-seed2-glm-4-7-flash-spec-first | $0.0092 | $0.1375 | 47.2s |
| seed2-glm-4-7-flash-spec-first | $0.0016 | $0.0125 | 39.8s |
| g2a-seed5-qwen3-7-plus-spec-first | $0.0093 | $0.0463 | 53.1s |
| g3a-g1a-seed2-glm-4-7-flash-spec-first | $0.0029 | $0.0147 | 55.2s |
| probe-oneshot | $0.0134 | $0.0803 | 85.6s |
| seed3-gpt-5-mini-skeptic | $0.0045 | $0.0225 | 29.2s |
| seed4-deepseek-v4-flash-test-runner | $0.0036 | $0.0182 | 136.9s |
| null | $0.0000 | $0.0000 | 0.0s |
| seed1-kimi-k2-6-test-runner | unknown | unknown | 420.0s |
| g1b-seed5-qwen3-7-plus-spec-first | unknown | unknown | 420.1s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- seed5-qwen3-7-plus-spec-first
- g1a-seed2-glm-4-7-flash-spec-first
- seed2-glm-4-7-flash-spec-first
- seed3-gpt-5-mini-skeptic

## Recommendation

**g2b-g1a-seed2-glm-4-7-flash-spec-first** — mean reward 0.7200 at $0.0092/trial (47.2s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥3 trials per search task): g2b-g1a-seed2-glm-4-7-flash-spec-first. Recommendation restricted to certified candidates._

## Candidate cutoffs

- `seed1-kimi-k2-6-test-runner` skipped split `validation` after 1 errors (limit 1).
- `g1b-seed5-qwen3-7-plus-spec-first` skipped split `validation` after 1 errors (limit 1).
