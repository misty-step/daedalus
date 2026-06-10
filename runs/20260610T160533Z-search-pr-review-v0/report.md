# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | dac86ce481e480d9 | 4 | 0 |
| seed1-glm-5-spec-first | pi | z-ai/glm-5 | ffa1ebbd0a79ab89 | 6 | 0 |
| seed5-qwen3-7-plus-test-runner | pi | qwen/qwen3.7-plus | f645b7dfee3b5b15 | 6 | 0 |
| g1b-seed1-glm-5-spec-first | pi | z-ai/glm-5 | 44a9aa47e96933ed | 8 | 0 |
| seed2-glm-4-7-flash-test-runner | pi | z-ai/glm-4.7-flash | 04eba1523b345086 | 6 | 1 |
| g1a-seed1-glm-5-spec-first | pi | z-ai/glm-5 | 24136eec39b07c57 | 6 | 0 |
| seed6-kimi-k2-6-trace-callers | pi | moonshotai/kimi-k2.6 | ba31e6f78e27486b | 6 | 1 |
| g2a-g1a-seed1-glm-5-spec-first | pi | z-ai/glm-5 | 275e003efee5ea9d | 6 | 0 |
| g2b-g1b-seed1-glm-5-spec-first | pi | z-ai/glm-5 | 1268d01355e9cd85 | 6 | 0 |
| seed3-deepseek-v4-flash-trace-callers | pi | deepseek/deepseek-v4-flash | c8926d02ec0530fa | 6 | 0 |
| null | null | — | eaedabf2780259e2 | 4 | 0 |
| seed4-gpt-5-mini-spec-first | pi | openai/gpt-5-mini | 8b6e58f5ead72aea | 8 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 84e313ca2b655104 | 4 | 4 |

## Mean reward per task (n trials in parentheses)

| candidate | py-live-lock | py-measure-normalize | py-padding-clean | py-progress-speed | **overall** |
|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| seed1-glm-5-spec-first | — | 1.00 (2) | 1.00 (2) | 1.00 (2) | **1.0000** |
| seed5-qwen3-7-plus-test-runner | — | 1.00 (2) | 1.00 (2) | 1.00 (2) | **1.0000** |
| g1b-seed1-glm-5-spec-first | 1.00 (2) | 1.00 (2) | 1.00 (2) | 1.00 (2) | **1.0000** |
| seed2-glm-4-7-flash-test-runner | — | 0.50 (2) | 1.00 (2) | 1.00 (2) | **0.8333** |
| g1a-seed1-glm-5-spec-first | — | 0.50 (2) | 1.00 (2) | 1.00 (2) | **0.8333** |
| seed6-kimi-k2-6-trace-callers | — | 0.00 (2) | 1.00 (2) | 1.00 (2) | **0.6667** |
| g2a-g1a-seed1-glm-5-spec-first | — | 0.00 (2) | 1.00 (2) | 1.00 (2) | **0.6667** |
| g2b-g1b-seed1-glm-5-spec-first | — | 0.00 (2) | 1.00 (2) | 1.00 (2) | **0.6667** |
| seed3-deepseek-v4-flash-trace-callers | — | 0.00 (2) | 0.50 (2) | 1.00 (2) | **0.5000** |
| null | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | **0.2500** |
| seed4-gpt-5-mini-spec-first | 0.00 (2) | 0.00 (2) | 0.50 (2) | 0.00 (2) | **0.1250** |
| probe-oneshot | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.0s |
| seed1-glm-5-spec-first | $0.0171 | $0.1026 | 65.2s |
| seed5-qwen3-7-plus-test-runner | $0.0179 | $0.1074 | 68.7s |
| g1b-seed1-glm-5-spec-first | $0.0138 | $0.1105 | 61.2s |
| seed2-glm-4-7-flash-test-runner | unknown | unknown | 227.5s |
| g1a-seed1-glm-5-spec-first | $0.0143 | $0.0861 | 61.5s |
| seed6-kimi-k2-6-trace-callers | unknown | unknown | 357.2s |
| g2a-g1a-seed1-glm-5-spec-first | $0.0295 | $0.1772 | 103.2s |
| g2b-g1b-seed1-glm-5-spec-first | $0.0147 | $0.0883 | 60.1s |
| seed3-deepseek-v4-flash-trace-callers | $0.0141 | $0.0847 | 137.2s |
| null | $0.0000 | $0.0000 | 0.0s |
| seed4-gpt-5-mini-spec-first | $0.0033 | $0.0264 | 18.2s |
| probe-oneshot | unknown | unknown | 1.2s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- g1b-seed1-glm-5-spec-first
- g2b-g1b-seed1-glm-5-spec-first
- seed4-gpt-5-mini-spec-first

## Recommendation

**g1b-seed1-glm-5-spec-first** — mean reward 1.0000 at $0.0138/trial (61.2s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._
