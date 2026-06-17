# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 6 | 0 |
| seed1-kimi-k2-6-checklist | pi | moonshotai/kimi-k2.6 | 2bfc1ec9870046c9 | 5 | 0 |
| seed2-qwen3-7-plus-spec-first | pi | qwen/qwen3.7-plus | 491643a3b1de61e3 | 12 | 0 |
| null | null | — | 40f2f6e05112c409 | 6 | 0 |
| probe-oneshot | oneshot | moonshotai/kimi-k2.6 | 9f21bdb5c010ab18 | 6 | 6 |

## Mean reward per task (n trials in parentheses)

| candidate | clean-noise | credential-duplicate | dual-defect-conflict | gate-regression | missing-security-member | runtime-crash | **overall** |
|---|---|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| seed1-kimi-k2-6-checklist | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | — | 1.00 (1) | **1.0000** |
| seed2-qwen3-7-plus-spec-first | 1.00 (2) | 1.00 (2) | 1.00 (2) | 1.00 (2) | 1.00 (2) | 1.00 (2) | **1.0000** |
| null | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.1667** |
| probe-oneshot | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.0000** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.0s |
| seed1-kimi-k2-6-checklist | $0.0582 | $0.2912 | 367.2s |
| seed2-qwen3-7-plus-spec-first | $0.0180 | $0.2158 | 93.6s |
| null | $0.0000 | $0.0000 | 0.0s |
| probe-oneshot | $0.0000 | $0.0000 | 1.8s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- seed2-qwen3-7-plus-spec-first

## Recommendation

**seed2-qwen3-7-plus-spec-first** — mean reward 1.0000 at $0.0180/trial (93.6s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥2 trials per search task): seed2-qwen3-7-plus-spec-first. Recommendation restricted to certified candidates._

## Meta-eval alarms

- **fp-trap-never-fired**: every agent passed clean task clean-noise; the trap may be too easy to discriminate FP discipline

## Spend accounting

Known spend including optimizer calls, certification, and holdout: $0.5290.
