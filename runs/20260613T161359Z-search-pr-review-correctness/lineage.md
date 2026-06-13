# Experiment lineage — 20260613T161359Z-search-pr-review-correctness

## Rig

- oracle 1.0 · null 0.2857 · one-shot probe 0.0 — arena discriminates

## Landscape scan (seed population)

rng_seed 11 · packet stances: spec-first, test-runner, skeptic

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-gpt-5-mini-spec-first | openai/gpt-5-mini | medium | full | 0.571 | 14 | $0.145 |
| seed2-deepseek-v4-flash-test-runner | deepseek/deepseek-v4-flash | low | explore | 0.400 | 5 | $0.054 |
| seed3-glm-4-7-flash-skeptic | z-ai/glm-4.7-flash | off | full | 0.333 | 9 | $0.023 |
| seed4-qwen3-7-plus-spec-first | qwen/qwen3.7-plus | medium | explore | 0.600 | 5 | $0.090 |
| seed5-kimi-k2-6-test-runner | moonshotai/kimi-k2.6 | low | full | 0.600 | 5 | $0.297 |

## Generations (hypothesis → measurement → decision)

- (no search generations recorded)

## Outcome

- stop: max-candidates · generations 0 · known spend $0.6253
- certified: seed1-gpt-5-mini-spec-first
- seed4-qwen3-7-plus-spec-first (hash 9720f530f1a28cf0): reward 0.6, $0.0180/trial
- seed1-gpt-5-mini-spec-first (hash f090f8060cf36637): reward 0.5714, $0.0103/trial ← **recommended**
- seed3-glm-4-7-flash-skeptic (hash cb4ee6db50b7bbd3): reward 0.3333, $0.0026/trial

## What this run taught us

- (none recorded)
