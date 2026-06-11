# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 10 | 0 |
| g2b-seed1-gpt-5-mini-checklist | pi | openai/gpt-5-mini | 73c5a2b2adde67a4 | 7 | 0 |
| g3b-g2b-seed1-gpt-5-mini-checklist | pi | openai/gpt-5-mini | 3c7c3cbe5648b374 | 22 | 0 |
| seed1-gpt-5-mini-checklist | pi | openai/gpt-5-mini | 8a97635f0309d4eb | 7 | 0 |
| seed4-qwen3-7-plus-checklist | pi | qwen/qwen3.7-plus | 4a73f1fd213aa1a5 | 35 | 0 |
| seed6-glm-5-test-runner | pi | z-ai/glm-5 | 3730cd9b3e7d6b4a | 7 | 0 |
| g3a-g1a-seed1-gpt-5-mini-checklist | pi | qwen/qwen3.7-plus | 93b0cf0b3490357f | 7 | 0 |
| g4a-g1a-seed1-gpt-5-mini-checklist | pi | qwen/qwen3.7-plus | c9016cc5fc0bca3b | 7 | 0 |
| seed3-glm-4-7-flash-test-runner | pi | z-ai/glm-4.7-flash | da3b9a61b46b102d | 8 | 1 |
| g1a-seed1-gpt-5-mini-checklist | pi | qwen/qwen3.7-plus | 8420212f9276b288 | 22 | 0 |
| seed5-deepseek-v4-flash-trace-callers | pi | deepseek/deepseek-v4-flash | a4a2fc6b5ee7c97a | 7 | 0 |
| g1b-seed4-qwen3-7-plus-checklist | pi | qwen/qwen3.7-plus | 5d9c9dfcb0aee310 | 7 | 0 |
| null | null | — | 40f2f6e05112c409 | 10 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 9f21bdb5c010ab18 | 10 | 10 |
| seed2-kimi-k2-6-trace-callers | pi | moonshotai/kimi-k2.6 | 855ca3ae1f880ef0 | 1 | 1 |

## Mean reward per task (n trials in parentheses)

| candidate | py-export-clear | py-formatter-clean | py-guess-swallow | py-live-lock | py-markup-escape | py-measure-normalize | py-padding-clean | py-plugin-cache | py-progress-speed | py-save-leak | **overall** |
|---|---|---|---|---|---|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| g2b-seed1-gpt-5-mini-checklist | — | 1.00 (1) | 1.00 (1) | — | 0.00 (1) | 1.00 (1) | 0.00 (1) | — | 1.00 (1) | 0.80 (1) | **0.6857** |
| g3b-g2b-seed1-gpt-5-mini-checklist | 0.56 (5) | 1.00 (1) | 0.00 (1) | 0.40 (5) | 0.00 (1) | 1.00 (1) | 1.00 (1) | 0.96 (5) | 1.00 (1) | 1.00 (1) | **0.6636** |
| seed1-gpt-5-mini-checklist | — | 1.00 (1) | 1.00 (1) | — | 0.00 (1) | 1.00 (1) | 0.00 (1) | — | 0.00 (1) | 1.00 (1) | **0.5714** |
| seed4-qwen3-7-plus-checklist | — | 1.00 (5) | 0.00 (5) | — | 0.00 (5) | 0.20 (5) | 1.00 (5) | — | 1.00 (5) | 0.80 (5) | **0.5714** |
| seed6-glm-5-test-runner | — | 1.00 (1) | 0.00 (1) | — | 0.00 (1) | 1.00 (1) | 1.00 (1) | — | 1.00 (1) | 0.00 (1) | **0.5714** |
| g3a-g1a-seed1-gpt-5-mini-checklist | — | 1.00 (1) | 0.00 (1) | — | 0.00 (1) | 1.00 (1) | 0.00 (1) | — | 1.00 (1) | 1.00 (1) | **0.5714** |
| g4a-g1a-seed1-gpt-5-mini-checklist | — | 1.00 (1) | 0.00 (1) | — | 0.00 (1) | 1.00 (1) | 1.00 (1) | — | 0.00 (1) | 1.00 (1) | **0.5714** |
| seed3-glm-4-7-flash-test-runner | 0.00 (1) | 1.00 (1) | 0.00 (1) | — | 0.00 (1) | 1.00 (1) | 1.00 (1) | — | 1.00 (1) | 0.00 (1) | **0.5000** |
| g1a-seed1-gpt-5-mini-checklist | 0.20 (5) | 1.00 (1) | 0.00 (1) | 0.60 (5) | 0.00 (1) | 1.00 (1) | 1.00 (1) | 0.20 (5) | 1.00 (1) | 1.00 (1) | **0.4545** |
| seed5-deepseek-v4-flash-trace-callers | — | 1.00 (1) | 0.00 (1) | — | 0.00 (1) | 0.00 (1) | 0.00 (1) | — | 1.00 (1) | 1.00 (1) | **0.4286** |
| g1b-seed4-qwen3-7-plus-checklist | — | 1.00 (1) | 0.00 (1) | — | 0.00 (1) | 1.00 (1) | 1.00 (1) | — | 0.00 (1) | 0.00 (1) | **0.4286** |
| null | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.2000** |
| probe-oneshot | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.0000** |
| seed2-kimi-k2-6-trace-callers | — | — | — | — | 0.00 (1) | — | — | — | — | — | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.1s |
| g2b-seed1-gpt-5-mini-checklist | $0.0267 | $0.1870 | 150.3s |
| g3b-g2b-seed1-gpt-5-mini-checklist | $0.0219 | $0.4828 | 113.0s |
| seed1-gpt-5-mini-checklist | $0.0177 | $0.1239 | 89.4s |
| seed4-qwen3-7-plus-checklist | $0.0170 | $0.5953 | 70.7s |
| seed6-glm-5-test-runner | $0.0383 | $0.2684 | 83.3s |
| g3a-g1a-seed1-gpt-5-mini-checklist | $0.0232 | $0.1622 | 88.5s |
| g4a-g1a-seed1-gpt-5-mini-checklist | $0.0189 | $0.1323 | 82.2s |
| seed3-glm-4-7-flash-test-runner | unknown | unknown | 107.7s |
| g1a-seed1-gpt-5-mini-checklist | $0.0178 | $0.3916 | 85.0s |
| seed5-deepseek-v4-flash-trace-callers | $0.0168 | $0.1175 | 165.0s |
| g1b-seed4-qwen3-7-plus-checklist | $0.0206 | $0.1444 | 86.1s |
| null | $0.0000 | $0.0000 | 0.0s |
| probe-oneshot | $0.0000 | $0.0000 | 2.5s |
| seed2-kimi-k2-6-trace-callers | unknown | unknown | 600.1s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- g2b-seed1-gpt-5-mini-checklist
- g3b-g2b-seed1-gpt-5-mini-checklist
- seed4-qwen3-7-plus-checklist
- seed5-deepseek-v4-flash-trace-callers

## Recommendation

**seed4-qwen3-7-plus-checklist** — mean reward 0.5714 at $0.0170/trial (70.7s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥5 trials per search task): seed4-qwen3-7-plus-checklist. Recommendation restricted to certified candidates._

## Meta-eval alarms

- **fp-trap-never-fired**: every agent passed clean task py-formatter-clean; the trap may be too easy to discriminate FP discipline

## Candidate cutoffs

- `seed2-kimi-k2-6-trace-callers` skipped split `validation` after 1 errors (limit 1).
