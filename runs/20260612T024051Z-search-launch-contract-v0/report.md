# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 6 | 0 |
| seed5-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | e93314677f18b138 | 8 | 0 |
| g2a-seed5-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | 38ff179d3de4da5b | 8 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 9f21bdb5c010ab18 | 6 | 0 |
| seed3-gpt-5-mini-skeptic | pi | openai/gpt-5-mini | e8492da875ed8b6e | 8 | 0 |
| seed2-glm-4-7-flash-spec-first | pi | z-ai/glm-4.7-flash | 2c79fecaa4fdbbca | 8 | 0 |
| g3a-seed2-glm-4-7-flash-spec-first | pi | z-ai/glm-4.7-flash | 11ef90f2168dca9a | 18 | 0 |
| seed4-deepseek-v4-flash-test-runner | pi | deepseek/deepseek-v4-flash | ef8c4a938e22000f | 5 | 0 |
| g1a-seed2-glm-4-7-flash-spec-first | pi | z-ai/glm-4.7-flash | ffb49589fe1f9ef9 | 8 | 0 |
| g2b-g1b-seed5-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | abb91b6ebde16e9f | 5 | 0 |
| g1b-seed5-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | 844ba7a09eb7a44a | 5 | 0 |
| null | null | — | 40f2f6e05112c409 | 6 | 0 |
| seed1-kimi-k2-6-test-runner | pi | moonshotai/kimi-k2.6 | 4840fbc7cae86313 | 1 | 1 |

## Mean reward per task (n trials in parentheses)

| candidate | absolute-prompt-path | bb-unsigned-primary | contract-missing-evidence | sandbox-clean | trace-waiver-missing | write-before-g4 | **overall** |
|---|---|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| seed5-qwen3-7-plus-spec-first | 1.00 (3) | 1.00 (1) | 0.80 (1) | 1.00 (1) | 0.60 (1) | 0.00 (1) | **0.8000** |
| g2a-seed5-qwen3-7-plus-spec-first | 1.00 (3) | 0.80 (1) | 0.80 (1) | 1.00 (1) | 0.60 (1) | 0.00 (1) | **0.7750** |
| probe-oneshot | 1.00 (1) | 1.00 (1) | 0.60 (1) | 1.00 (1) | 0.60 (1) | 0.00 (1) | **0.7000** |
| seed3-gpt-5-mini-skeptic | 0.93 (3) | 0.80 (1) | 0.60 (1) | 0.00 (1) | 0.60 (1) | 0.00 (1) | **0.6000** |
| seed2-glm-4-7-flash-spec-first | 0.53 (3) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | **0.5750** |
| g3a-seed2-glm-4-7-flash-spec-first | 0.33 (3) | 0.53 (3) | 0.87 (3) | 0.67 (3) | 0.60 (3) | 0.00 (3) | **0.5000** |
| seed4-deepseek-v4-flash-test-runner | — | 0.60 (1) | 0.20 (1) | 1.00 (1) | 0.60 (1) | 0.00 (1) | **0.4800** |
| g1a-seed2-glm-4-7-flash-spec-first | 0.60 (3) | 0.80 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | **0.4500** |
| g2b-g1b-seed5-qwen3-7-plus-spec-first | — | 0.80 (1) | 0.60 (1) | 0.00 (1) | 0.80 (1) | 0.00 (1) | **0.4400** |
| g1b-seed5-qwen3-7-plus-spec-first | — | 0.80 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.3600** |
| null | 0.00 (1) | 0.00 (1) | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | **0.1667** |
| seed1-kimi-k2-6-test-runner | — | 0.00 (1) | — | — | — | — | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.0s |
| seed5-qwen3-7-plus-spec-first | $0.0085 | $0.0681 | 40.2s |
| g2a-seed5-qwen3-7-plus-spec-first | $0.0083 | $0.0660 | 43.2s |
| probe-oneshot | $0.0182 | $0.1094 | 70.9s |
| seed3-gpt-5-mini-skeptic | $0.0040 | $0.0320 | 27.1s |
| seed2-glm-4-7-flash-spec-first | $0.0009 | $0.0073 | 74.2s |
| g3a-seed2-glm-4-7-flash-spec-first | $0.0012 | $0.0222 | 50.4s |
| seed4-deepseek-v4-flash-test-runner | $0.0043 | $0.0213 | 63.9s |
| g1a-seed2-glm-4-7-flash-spec-first | $0.0046 | $0.0369 | 76.4s |
| g2b-g1b-seed5-qwen3-7-plus-spec-first | $0.0089 | $0.0447 | 50.8s |
| g1b-seed5-qwen3-7-plus-spec-first | $0.0068 | $0.0342 | 35.9s |
| null | $0.0000 | $0.0000 | 0.0s |
| seed1-kimi-k2-6-test-runner | unknown | unknown | 420.0s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- seed5-qwen3-7-plus-spec-first
- g2a-seed5-qwen3-7-plus-spec-first
- seed3-gpt-5-mini-skeptic
- seed2-glm-4-7-flash-spec-first
- g3a-seed2-glm-4-7-flash-spec-first

## Recommendation

**g3a-seed2-glm-4-7-flash-spec-first** — mean reward 0.5000 at $0.0012/trial (50.4s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥3 trials per search task): g3a-seed2-glm-4-7-flash-spec-first. Recommendation restricted to certified candidates._

## Candidate cutoffs

- `seed1-kimi-k2-6-test-runner` skipped split `validation` after 1 errors (limit 1).
