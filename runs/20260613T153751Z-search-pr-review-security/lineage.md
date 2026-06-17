# Experiment lineage — 20260613T153751Z-search-pr-review-security

## Rig

- oracle 1.0 · null 0.3333 · one-shot probe 0.0 — arena discriminates

## Landscape scan (seed population)

rng_seed 9 · packet stances: skeptic, checklist, spec-first

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-qwen3-7-plus-skeptic | qwen/qwen3.7-plus | low | explore | 0.500 | 2 | $0.043 |
| seed2-deepseek-v4-flash-checklist | deepseek/deepseek-v4-flash | medium | full | 0.400 | 2 | $0.022 |
| seed3-glm-4-7-flash-spec-first | z-ai/glm-4.7-flash | off | explore | 0.500 | 2 | $0.001 |
| seed4-gpt-5-mini-skeptic | openai/gpt-5-mini | low | full | 0.500 | 4 | $0.016 |
| seed5-kimi-k2-6-checklist | moonshotai/kimi-k2.6 | medium | explore | 0.833 | 6 | $0.250 |

## Generations (hypothesis → measurement → decision)

- (no search generations recorded)

## Outcome

- stop: max-candidates · generations 0 · known spend $0.3527
- certified: seed5-kimi-k2-6-checklist
- seed5-kimi-k2-6-checklist (hash d112f8dd00b0f84b): reward 0.8333, $0.0417/trial ← **recommended**
- seed4-gpt-5-mini-skeptic (hash 724026968317addc): reward 0.5, $0.0040/trial

## What this run taught us

- (none recorded)
