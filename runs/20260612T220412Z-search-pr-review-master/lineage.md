# Experiment lineage — 20260612T220412Z-search-pr-review-master

## Rig

- oracle 1.0 · null 0.1667 · one-shot probe 0.0 — arena discriminates

## Landscape scan (seed population)

rng_seed 3406 · packet stances: checklist, spec-first, test-runner

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-kimi-k2-6-checklist | moonshotai/kimi-k2.6 | low | read-artifacts | 1.000 | 5 | $0.291 |
| seed2-qwen3-7-plus-spec-first | qwen/qwen3.7-plus | off | read-artifacts | 1.000 | 12 | $0.216 |

## Generations (hypothesis → measurement → decision)

- (no search generations recorded)

## Meta-eval alarms

- **fp-trap-never-fired**: every agent passed clean task clean-noise; the trap may be too easy to discriminate FP discipline

## Outcome

- stop: max-candidates · generations 0 · known spend $0.529
- certified: seed2-qwen3-7-plus-spec-first
- seed2-qwen3-7-plus-spec-first (hash 491643a3b1de61e3): reward 1.0, $0.0180/trial ← **recommended**

## What this run taught us

- [arena] every agent passed clean task clean-noise; the trap may be too easy to discriminate FP discipline
