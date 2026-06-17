# Experiment lineage — 20260613T214006Z-search-pr-review-correctness

## Rig

- oracle 1.0 · null 0.25 · one-shot probe 0.0 — arena discriminates

## Landscape scan (seed population)

rng_seed 12 · packet stances: checklist, test-runner, skeptic

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-deepseek-v4-flash-checklist | deepseek/deepseek-v4-flash | medium | full | 0.600 | 10 | $0.052 |
| seed2-glm-4-7-flash-test-runner | z-ai/glm-4.7-flash | low | explore | 0.667 | 6 | $0.080 |
| seed3-qwen3-7-plus-skeptic | qwen/qwen3.7-plus | off | full | 0.800 | 10 | $0.270 |
| seed4-gpt-5-mini-checklist | openai/gpt-5-mini | medium | explore | 0.680 | 10 | $0.111 |
| seed5-kimi-k2-6-test-runner | moonshotai/kimi-k2.6 | low | full | 0.633 | 6 | $0.452 |

## Generations (hypothesis → measurement → decision)

- g1.0 `g1a-seed3-qwen3-7-plus-skeptic` ← `seed3-qwen3-7-plus-skeptic` (slot `model`) (transplant from `seed2-glm-4-7-flash-test-runner`)
  - hypothesis: The current prompt packet yields 0.667 reward on qwen3.7-plus but costs $0.240; seed2 achieves the same reward on glm-4.7-flash for $0.080. Transplanting the cheap model while keeping the current prompt tests whether the strong prompt can run on a cheaper backbone, directly addressing the cost side of the threshold-then-cheap objective.
  - measured: reward 0.6667 vs parent 0.6667 (paired Δ 0.0) → **improvement — kept as a direction**
  - prediction confirmed: reward hold: ✓ (Δ+0.000), cost down: ✓ (×0.16)

## Outcome

- stop: max-candidates · generations 1 · known spend $1.3002
- certified: g1a-seed3-qwen3-7-plus-skeptic
- seed3-qwen3-7-plus-skeptic (hash 0364f14e074e130d): reward 0.8, $0.0270/trial
- seed4-gpt-5-mini-checklist (hash 4671c3e06a946bd8): reward 0.68, $0.0111/trial
- seed1-deepseek-v4-flash-checklist (hash 05e59e2e8e9ddaaa): reward 0.6, $0.0052/trial

## What this run taught us

- [confirmed: reward hold: ✓ (Δ+0.000), cost down: ✓ (×0.16)] The current prompt packet yields 0.667 reward on qwen3.7-plus but costs $0.240; seed2 achieves the same reward on glm-4.7-flash for $0.080. Transplanting the cheap model while keeping the current prompt tests whether the strong prompt can run on a cheaper backbone, directly addressing the cost side of the threshold-then-cheap objective.
