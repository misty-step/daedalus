# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | 07d8650e238cd916 | 6 | 0 |
| g2b-seed2-kimi-k2-7-code-trace-callers | pi | moonshotai/kimi-k2.7-code | 1366fbcb37b7fb0b | 30 | 1 |
| seed2-kimi-k2-7-code-trace-callers | pi | moonshotai/kimi-k2.7-code | 1df8c73c5cfbb4db | 30 | 0 |
| seed6-glm-5-2-test-runner | pi | z-ai/glm-5.2 | 11eada1eb772ce33 | 25 | 0 |
| g1a-seed3-gpt-5-4-mini-test-runner | pi | openai/gpt-5.4-mini | 687d605505c0b9ac | 15 | 0 |
| seed5-deepseek-v4-pro-trace-callers | pi | deepseek/deepseek-v4-pro | 73e6809a98ab443d | 20 | 0 |
| g2a-g1a-seed3-gpt-5-4-mini-test-runner | pi | z-ai/glm-5.2 | f5f3e00386ea8638 | 20 | 1 |
| seed3-gpt-5-4-mini-test-runner | pi | openai/gpt-5.4-mini | 0557a75164a92fdd | 20 | 0 |
| probe-oneshot | oneshot | deepseek/deepseek-v4-pro | 8d5c0d7a1188c916 | 6 | 0 |
| g1b-seed1-glm-5-2-spec-first | pi | z-ai/glm-5.2 | 736b3998490093fb | 15 | 0 |
| seed1-glm-5-2-spec-first | pi | z-ai/glm-5.2 | ba1cd08718fb0e5f | 20 | 0 |
| seed4-deepseek-v4-flash-spec-first | pi | deepseek/deepseek-v4-flash | 2ba897345f64ac56 | 20 | 0 |
| null | null | — | 40f2f6e05112c409 | 6 | 0 |

## Mean reward per task (n trials in parentheses)

| candidate | js-cart-total | js-clean-rename | py-auth-sqli | py-file-cache | py-pagination | rs-retry-backoff | **overall** |
|---|---|---|---|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | 1.00 (1) | **1.0000** |
| g2b-seed2-kimi-k2-7-code-trace-callers | 0.80 (5) | 1.00 (5) | 0.70 (5) | 0.57 (5) | 1.00 (5) | 0.50 (5) | **0.7622** |
| seed2-kimi-k2-7-code-trace-callers | 0.80 (5) | 1.00 (5) | 0.52 (5) | 0.51 (5) | 1.00 (5) | 0.70 (5) | **0.7544** |
| seed6-glm-5-2-test-runner | 0.60 (5) | 1.00 (5) | 0.50 (5) | 0.43 (5) | 1.00 (5) | — | **0.7053** |
| g1a-seed3-gpt-5-4-mini-test-runner | 1.00 (3) | 1.00 (3) | 0.43 (3) | 0.00 (3) | 1.00 (3) | — | **0.6867** |
| seed5-deepseek-v4-pro-trace-callers | 0.33 (3) | 1.00 (3) | 0.50 (3) | 0.31 (3) | 1.00 (3) | 0.52 (5) | **0.6017** |
| g2a-g1a-seed3-gpt-5-4-mini-test-runner | 1.00 (3) | 1.00 (3) | 0.43 (3) | 0.22 (3) | 1.00 (3) | 0.20 (5) | **0.5983** |
| seed3-gpt-5-4-mini-test-runner | 1.00 (3) | 1.00 (3) | 0.43 (3) | 0.16 (3) | 1.00 (3) | 0.20 (5) | **0.5883** |
| probe-oneshot | 0.00 (1) | 1.00 (1) | 1.00 (1) | 0.13 (1) | 1.00 (1) | 0.00 (1) | **0.5222** |
| g1b-seed1-glm-5-2-spec-first | 0.00 (3) | 1.00 (3) | 0.43 (3) | 0.11 (3) | 1.00 (3) | — | **0.5089** |
| seed1-glm-5-2-spec-first | 0.00 (3) | 1.00 (3) | 0.50 (3) | 0.13 (3) | 1.00 (3) | 0.20 (5) | **0.4450** |
| seed4-deepseek-v4-flash-spec-first | 0.33 (3) | 0.33 (3) | 0.50 (3) | 0.27 (3) | 1.00 (3) | 0.30 (5) | **0.4400** |
| null | 0.00 (1) | 1.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | 0.00 (1) | **0.1667** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| oracle | $0.0000 | $0.0000 | 0.0s |
| g2b-seed2-kimi-k2-7-code-trace-callers | $0.0312 | $0.9345 | 141.7s |
| seed2-kimi-k2-7-code-trace-callers | $0.0184 | $0.5505 | 74.8s |
| seed6-glm-5-2-test-runner | $0.0072 | $0.1788 | 39.5s |
| g1a-seed3-gpt-5-4-mini-test-runner | $0.0067 | $0.0999 | 14.0s |
| seed5-deepseek-v4-pro-trace-callers | $0.0037 | $0.0747 | 46.4s |
| g2a-g1a-seed3-gpt-5-4-mini-test-runner | $0.0110 | $0.2209 | 85.0s |
| seed3-gpt-5-4-mini-test-runner | $0.0060 | $0.1196 | 11.9s |
| probe-oneshot | $0.0023 | $0.0136 | 6.3s |
| g1b-seed1-glm-5-2-spec-first | $0.0079 | $0.1180 | 44.6s |
| seed1-glm-5-2-spec-first | $0.0062 | $0.1235 | 31.1s |
| seed4-deepseek-v4-flash-spec-first | $0.0024 | $0.0485 | 51.9s |
| null | $0.0000 | $0.0000 | 0.0s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- g2b-seed2-kimi-k2-7-code-trace-callers
- seed2-kimi-k2-7-code-trace-callers
- seed6-glm-5-2-test-runner
- g1a-seed3-gpt-5-4-mini-test-runner
- seed5-deepseek-v4-pro-trace-callers
- seed3-gpt-5-4-mini-test-runner
- seed4-deepseek-v4-flash-spec-first

## Recommendation

**seed2-kimi-k2-7-code-trace-callers** — mean reward 0.7544 at $0.0184/trial (74.8s mean wall). Within-0.05 reward ties resolve to the cheapest candidate per trial.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation. Every recommendable candidate is an agent composition._

_Certified (≥5 trials per search task AND 95% CI lower bound > +0.0000 vs the null floor): g2b-seed2-kimi-k2-7-code-trace-callers, seed2-kimi-k2-7-code-trace-callers, seed6-glm-5-2-test-runner. Recommendation restricted to certified candidates._

## Reward delta vs baseline (95% CI)

Cluster-robust 95% CI on (candidate − `null`) mean reward, tasks clustered by source repo, using t_(G−1) critical values — honest with few clusters (a 2-repo arena gives df=1, t=12.7, so it certifies almost nothing). A CI that excludes 0 is an improvement over the floor at 95% confidence. `clstr→95%` is the power note (039 child-5): the number of independent clusters (tasks today, source repos once 040 lands labels) at which the *observed* effect's CI is expected to just reach 0 — compare it to n_clusters. Adding tasks within existing clusters does not shrink the SE; clusters do.

| candidate | Δ reward | 95% CI | n_tasks | n_clusters | clstr→95% | sig |
|---|---|---|---|---|---|---|
| g2b-seed2-kimi-k2-7-code-trace-callers | +0.5956 | [+0.2379, +0.9532] | 6 | 6 | 4 | ✓ |
| seed2-kimi-k2-7-code-trace-callers | +0.5878 | [+0.2290, +0.9466] | 6 | 6 | 4 | ✓ |
| seed6-glm-5-2-test-runner | +0.5053 | [+0.0598, +0.9509] | 5 | 5 | 5 | ✓ |

## Reliability (pass rate at reward ≥ 1.00)

Fraction of trials that reach the floor, and pass^5 — the chance all 5 independent trials reach it. Reliability, reported separately from mean reward: a high mean with low pass^5 is not deployable (τ-bench; arXiv 2605.10516). Lower `--consistency-floor` to discriminate mid-tier candidates.

| candidate | n | pass≥1.00 | pass^5 |
|---|---|---|---|
| g2b-seed2-kimi-k2-7-code-trace-callers | 30 | 0.6000 | 0.0601 |
| seed2-kimi-k2-7-code-trace-callers | 30 | 0.5667 | 0.0434 |
| seed6-glm-5-2-test-runner | 25 | 0.5600 | 0.0377 |

## Spend accounting

Known spend including optimizer calls, certification, and holdout: $2.5224.
