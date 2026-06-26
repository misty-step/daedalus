# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| probe-oneshot | oneshot | deepseek/deepseek-v4-pro | 8d5c0d7a1188c916 | 3 | 0 |
| seed2-kimi-k2-7-code-trace-callers | pi | moonshotai/kimi-k2.7-code | 8f66a041352d3385 | 30 | 2 |
| g1b-seed2-kimi-k2-7-code-trace-callers | pi | openai/gpt-5.4-mini | 9ec001c6793125fc | 30 | 0 |
| incumbent | incumbent | deepseek/deepseek-v4-pro | 5c8cde491fe1b6aa | 30 | 0 |
| g2a-seed2-kimi-k2-7-code-trace-callers | pi | moonshotai/kimi-k2.7-code | 1d2daccb35a6d10f | 11 | 1 |
| seed3-gpt-5-4-mini-test-runner | pi | openai/gpt-5.4-mini | 41bcc61fe7f92478 | 30 | 0 |
| seed1-glm-5-2-spec-first | pi | z-ai/glm-5.2 | 04e2a155e7f1dd2d | 15 | 0 |
| seed5-deepseek-v4-pro-trace-callers | pi | deepseek/deepseek-v4-pro | ae486ad3b306c97d | 20 | 0 |
| g1a-seed1-glm-5-2-spec-first | pi | z-ai/glm-5.2 | 08a2da249906fa86 | 15 | 0 |
| seed6-glm-5-2-test-runner | pi | z-ai/glm-5.2 | dd913f653e69393f | 20 | 0 |
| g2b-g1a-seed1-glm-5-2-spec-first | pi | z-ai/glm-5.2 | fe64d9e3a6d6a599 | 15 | 0 |
| seed4-deepseek-v4-flash-spec-first | pi | deepseek/deepseek-v4-flash | cb2039b025d207ee | 20 | 0 |

## Mean reward per task (n trials in parentheses)

| candidate | js-cart-total | js-clean-rename | py-auth-sqli | py-file-cache | py-pagination | rs-retry-backoff | **overall** |
|---|---|---|---|---|---|---|---|
| probe-oneshot | — | — | — | 0.80 (1) | 1.00 (1) | 0.50 (1) | **0.7667** |
| seed2-kimi-k2-7-code-trace-callers | 0.96 (5) | 1.00 (5) | 0.80 (5) | 0.43 (5) | 0.80 (5) | 0.56 (5) | **0.7578** |
| g1b-seed2-kimi-k2-7-code-trace-callers | 0.80 (5) | 1.00 (5) | 0.50 (5) | 0.49 (5) | 1.00 (5) | 0.50 (5) | **0.7156** |
| incumbent | 0.96 (5) | 1.00 (5) | 0.50 (5) | 0.16 (5) | 1.00 (5) | 0.50 (5) | **0.6867** |
| g2a-seed2-kimi-k2-7-code-trace-callers | 0.67 (3) | 1.00 (3) | 0.67 (3) | 0.07 (2) | — | — | **0.6485** |
| seed3-gpt-5-4-mini-test-runner | 1.00 (5) | 0.80 (5) | 0.46 (5) | 0.15 (5) | 1.00 (5) | 0.10 (5) | **0.5844** |
| seed1-glm-5-2-spec-first | 0.00 (3) | 1.00 (3) | 0.50 (3) | 0.38 (3) | 1.00 (3) | — | **0.5756** |
| seed5-deepseek-v4-pro-trace-callers | 0.67 (3) | 1.00 (3) | 0.43 (3) | 0.04 (3) | 1.00 (3) | 0.36 (5) | **0.5617** |
| g1a-seed1-glm-5-2-spec-first | 0.00 (3) | 1.00 (3) | 0.33 (3) | 0.27 (3) | 1.00 (3) | — | **0.5200** |
| seed6-glm-5-2-test-runner | 0.33 (3) | 1.00 (3) | 0.43 (3) | 0.16 (3) | 1.00 (3) | 0.20 (5) | **0.4883** |
| g2b-g1a-seed1-glm-5-2-spec-first | 0.00 (3) | 1.00 (3) | 0.50 (3) | 0.04 (3) | 0.67 (3) | — | **0.4422** |
| seed4-deepseek-v4-flash-spec-first | 1.00 (3) | 0.00 (3) | 0.50 (3) | 0.04 (3) | 0.33 (3) | 0.46 (5) | **0.3967** |

## Cost and latency

| candidate | cost/trial | total cost | mean wall/task |
|---|---|---|---|
| probe-oneshot | $0.0077 | $0.0231 | 32.4s |
| seed2-kimi-k2-7-code-trace-callers | $0.0161 | $0.4834 | 157.5s |
| g1b-seed2-kimi-k2-7-code-trace-callers | $0.0150 | $0.4499 | 34.9s |
| incumbent | $0.0038 | $0.1151 | 50.1s |
| g2a-seed2-kimi-k2-7-code-trace-callers | $0.0153 | $0.1682 | 157.8s |
| seed3-gpt-5-4-mini-test-runner | $0.0078 | $0.2346 | 17.1s |
| seed1-glm-5-2-spec-first | $0.0060 | $0.0894 | 47.9s |
| seed5-deepseek-v4-pro-trace-callers | $0.0045 | $0.0908 | 47.5s |
| g1a-seed1-glm-5-2-spec-first | $0.0054 | $0.0814 | 39.0s |
| seed6-glm-5-2-test-runner | $0.0090 | $0.1807 | 58.4s |
| g2b-g1a-seed1-glm-5-2-spec-first | $0.0063 | $0.0947 | 43.2s |
| seed4-deepseek-v4-flash-spec-first | $0.0022 | $0.0449 | 48.8s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- seed2-kimi-k2-7-code-trace-callers
- g1b-seed2-kimi-k2-7-code-trace-callers
- seed3-gpt-5-4-mini-test-runner
- seed1-glm-5-2-spec-first
- seed5-deepseek-v4-pro-trace-callers
- g1a-seed1-glm-5-2-spec-first
- seed4-deepseek-v4-flash-spec-first

## Recommendation

No certified recommendation.

_References are excluded from Pareto and recommendation: oracle/null bound the verifier; the one-shot probe only detects arena saturation; the incumbent is the baseline-to-beat. Every recommendable candidate is an agent composition._

## Verdict

The search stopped because **plateau** — consecutive non-improving generations exhausted the plateau limit.

- **Recommended:** none — no candidate is provably better than the incumbent.
- **Certified:** none
- **Known spend:** $2.0969

_Trial-complete but NOT certified (5 trials, but the reward-delta CI spans the +0.0000 minimum effect — no provable win over the incumbent): g1b-seed2-kimi-k2-7-code-trace-callers, seed2-kimi-k2-7-code-trace-callers, seed3-gpt-5-4-mini-test-runner. See the CI table; raise --certify-trials or task count, or widen the arena (040)._

> **No candidate is provably better than the incumbent.** Every trial-complete candidate's 95% reward-delta CI spans the minimum detectable effect — the tournament is underpowered, not necessarily the candidates. Add trials/tasks (see the power note) or accept a wider MDE before trusting a ranking.

## Reward delta vs baseline (95% CI)

Cluster-robust 95% CI on (candidate − `incumbent`) mean reward, tasks clustered by source repo, using t_(G−1) critical values — honest with few clusters (a 2-repo arena gives df=1, t=12.7, so it certifies almost nothing). A CI that excludes 0 is an improvement over the selected baseline at 95% confidence. `clstr→95%` is the power note (039 child-5): the number of independent clusters (tasks today, source repos once 040 lands labels) at which the *observed* effect's CI is expected to just reach 0 — compare it to n_clusters. Adding tasks within existing clusters does not shrink the SE; clusters do.

| candidate | Δ reward | 95% CI | n_tasks | n_clusters | clstr→95% | sig |
|---|---|---|---|---|---|---|
| g1b-seed2-kimi-k2-7-code-trace-callers | +0.0289 | [-0.1415, +0.1992] | 6 | 6 | 124 | — |
| seed2-kimi-k2-7-code-trace-callers | +0.0711 | [-0.1249, +0.2671] | 6 | 6 | 29 | — |
| seed3-gpt-5-4-mini-test-runner | -0.1022 | [-0.2783, +0.0738] | 6 | 6 | — | — |

## Reliability (pass rate at reward ≥ 1.00)

Fraction of trials that reach the floor, and pass^5 — the chance all 5 independent trials reach it. Reliability, reported separately from mean reward: a high mean with low pass^5 is not deployable (τ-bench; arXiv 2605.10516). Lower `--consistency-floor` to discriminate mid-tier candidates.

| candidate | n | pass≥1.00 | pass^5 |
|---|---|---|---|
| g1b-seed2-kimi-k2-7-code-trace-callers | 30 | 0.5000 | 0.0211 |
| seed2-kimi-k2-7-code-trace-callers | 30 | 0.6667 | 0.1088 |
| seed3-gpt-5-4-mini-test-runner | 30 | 0.4667 | 0.0140 |

## Spend accounting

Known spend including optimizer calls, certification, and holdout: $2.0969.
