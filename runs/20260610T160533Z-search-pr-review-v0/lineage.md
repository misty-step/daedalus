# Experiment lineage — 20260610T160533Z-search-pr-review-v0

## Rig

- oracle 1.0 · null 0.25 · one-shot probe 0.0 — arena discriminates

## Landscape scan (seed population)

rng_seed 1106 · packet stances: spec-first, test-runner, trace-callers

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-glm-5-spec-first | z-ai/glm-5 | high | full | 1.000 | 6 | $0.103 |
| seed2-glm-4-7-flash-test-runner | z-ai/glm-4.7-flash | off | no-exec | 0.833 | 6 | $0.009 |
| seed3-deepseek-v4-flash-trace-callers | deepseek/deepseek-v4-flash | medium | explore | 0.500 | 6 | $0.085 |
| seed4-gpt-5-mini-spec-first | openai/gpt-5-mini | low | full | 0.125 | 8 | $0.026 |
| seed5-qwen3-7-plus-test-runner | qwen/qwen3.7-plus | high | no-exec | 1.000 | 6 | $0.107 |
| seed6-kimi-k2-6-trace-callers | moonshotai/kimi-k2.6 | off | explore | 0.667 | 6 | $2.134 |

## Generations (hypothesis → measurement → decision)

- g1.0 `g1a-seed1-glm-5-spec-first` ← `seed1-glm-5-spec-first` (slot `prompt_packet`)
  - hypothesis: The py-padding-clean transcript shows the model generating 794+ tokens of thinking to manually verify every valid tuple-length case, which inflates cost without improving reward; adding an explicit conciseness instruction should preserve the 1.0 defect-detection rate while significantly reducing token usage on clean diffs.
  - measured: reward 0.8333 vs parent 1.0 (paired Δ -0.1667) → no improvement — direction discarded
- g1.1 `g1b-seed1-glm-5-spec-first` ← `seed1-glm-5-spec-first` (slot `thinking`)
  - hypothesis: The worst-trial transcript reveals the agent performing verbose self-check reasoning under high thinking (e.g., re-reading findings.json after writing it), which inflates token usage and cost without improving defect-detection accuracy; lowering thinking to medium should suppress such superfluous verification loops while retaining enough analytical depth to spot contract violations.
  - measured: reward 1.0 vs parent 1.0 (paired Δ 0.0) → no improvement — direction discarded
- g2.0 `g2a-g1a-seed1-glm-5-spec-first` ← `g1a-seed1-glm-5-spec-first` (slot `prompt_packet`)
  - hypothesis: The worst trial produced a false positive by treating the removal of an internal .normalize() defensive call as a contract violation. Adding an explicit instruction not to report the removal of defensive checks or normalization as defects unless they are part of a documented public interface will prevent this over-reporting.
  - measured: reward 0.6667 vs parent 0.8333 (paired Δ -0.1667) → no improvement — direction discarded
- g2.1 `g2b-g1b-seed1-glm-5-spec-first` ← `g1b-seed1-glm-5-spec-first` (slot `prompt_packet`)
  - hypothesis: The transcript evidence shows the agent repeatedly performs redundant read-back tool calls and verbose closing summaries after writing findings.json, which inflates cost without improving accuracy. Adding an explicit stop instruction should eliminate these unnecessary turns and reduce spend while preserving the already-demonstrated detection accuracy.
  - measured: reward 0.6667 vs parent 1.0 (paired Δ -0.3333) → no improvement — direction discarded

## Outcome

- stop: plateau · generations 2 · known spend $3.027
- g1b-seed1-glm-5-spec-first (hash 44a9aa47e96933ed): reward 1.0, $0.0138/trial ← **recommended**
- g2b-g1b-seed1-glm-5-spec-first (hash 1268d01355e9cd85): reward 0.6667, $0.0147/trial
- seed4-gpt-5-mini-spec-first (hash 8b6e58f5ead72aea): reward 0.125, $0.0033/trial

## What this run taught us

- [not confirmed (Δ -0.1667)] The py-padding-clean transcript shows the model generating 794+ tokens of thinking to manually verify every valid tuple-length case, which inflates cost without improving reward; adding an explicit conciseness instruction should preserve the 1.0 defect-detection rate while significantly reducing token usage on clean diffs.
- [not confirmed (Δ 0.0)] The worst-trial transcript reveals the agent performing verbose self-check reasoning under high thinking (e.g., re-reading findings.json after writing it), which inflates token usage and cost without improving defect-detection accuracy; lowering thinking to medium should suppress such superfluous verification loops while retaining enough analytical depth to spot contract violations.
- [not confirmed (Δ -0.1667)] The worst trial produced a false positive by treating the removal of an internal .normalize() defensive call as a contract violation. Adding an explicit instruction not to report the removal of defensive checks or normalization as defects unless they are part of a documented public interface will prevent this over-reporting.
- [not confirmed (Δ -0.3333)] The transcript evidence shows the agent repeatedly performs redundant read-back tool calls and verbose closing summaries after writing findings.json, which inflates cost without improving accuracy. Adding an explicit stop instruction should eliminate these unnecessary turns and reduce spend while preserving the already-demonstrated detection accuracy.
