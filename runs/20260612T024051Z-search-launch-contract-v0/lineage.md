# Experiment lineage — 20260612T024051Z-search-launch-contract-v0

## Rig

- oracle 1.0 · null 0.1667 · one-shot probe 0.7 — arena discriminates

## Landscape scan (seed population)

rng_seed 3006 · packet stances: test-runner, spec-first, skeptic

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-kimi-k2-6-test-runner | moonshotai/kimi-k2.6 | off | no-exec | 0.000 | 1 | $0.000 |
| seed2-glm-4-7-flash-spec-first | z-ai/glm-4.7-flash | low | inspect | 0.575 | 8 | $0.007 |
| seed3-gpt-5-mini-skeptic | openai/gpt-5-mini | medium | full | 0.600 | 8 | $0.032 |
| seed4-deepseek-v4-flash-test-runner | deepseek/deepseek-v4-flash | off | no-exec | 0.480 | 5 | $0.021 |
| seed5-qwen3-7-plus-spec-first | qwen/qwen3.7-plus | low | inspect | 0.800 | 8 | $0.068 |

## Generations (hypothesis → measurement → decision)

- g1.0 `g1a-seed2-glm-4-7-flash-spec-first` ← `seed2-glm-4-7-flash-spec-first` (slot `system_prompt_mode`)
  - hypothesis: The appended default coding-assistant persona is causing GLM-4.7-flash to override the packet's strict output rules and invent requirements (evidenced by false-positive summaries and hallucinated approval-gate defects in the worst-trial transcripts); replacing the system prompt with the packet should enforce structured-only output and reduce false positives without increasing cost.
  - measured: reward 0.36 vs parent 0.6 (paired Δ -0.24) → no improvement — direction discarded
  - prediction refuted: reward up: ✗ (Δ-0.240), cost hold: ✗ (×1.34)
- g1.1 `g1b-seed5-qwen3-7-plus-spec-first` ← `seed5-qwen3-7-plus-spec-first` (slot `thinking`)
  - hypothesis: The low-thinking setting produces false positives (e.g., g3_signed flagged incorrectly) and misses seeded defects; increasing reasoning depth should improve precision and recall before we pivot to cost reduction.
  - measured: reward 0.36 vs parent 0.68 (paired Δ -0.32) → no improvement — direction discarded
  - prediction partially confirmed: reward up: ✗ (Δ-0.320), cost up: ✓ (×0.76)
- g2.0 `g2a-seed5-qwen3-7-plus-spec-first` ← `seed5-qwen3-7-plus-spec-first` (slot `prompt_packet`)
  - hypothesis: The worst-trial false positives show the model invents requirements despite the generic prohibition. Adding a mandatory citation constraint—requiring the model to quote the exact specification rule or invariant violated before reporting—will force grounding and eliminate hallucinated findings.
  - measured: reward 0.64 vs parent 0.68 (paired Δ -0.04) → no improvement — direction discarded
  - prediction partially confirmed: reward up: ✗ (Δ-0.040), cost hold: ✓ (×0.95)
- g2.1 `g2b-g1b-seed5-qwen3-7-plus-spec-first` ← `g1b-seed5-qwen3-7-plus-spec-first` (slot `thinking`)
  - hypothesis: The worst-trial transcripts show the agent missing seeded defects and emitting false positives on nuanced approval-gate and evidence clauses; increasing thinking from medium to high should deepen invariant verification and reduce both omission and hallucination errors.
  - measured: reward 0.44 vs parent 0.36 (paired Δ 0.08) → **improvement — kept as a direction**
  - prediction confirmed: reward up: ✓ (Δ+0.080), cost up: ✓ (×1.31)
- g3.0 `g3a-seed2-glm-4-7-flash-spec-first` ← `seed2-glm-4-7-flash-spec-first` (slot `system_prompt_mode`)
  - hypothesis: The default appended coding assistant prompt promises write/edit tools that are not actually provided, confusing the model and contributing to hallucinated findings and workflow errors; replacing the system prompt with only the review packet removes this contradiction.
  - measured: reward 0.68 vs parent 0.6 (paired Δ 0.08) → **improvement — kept as a direction**
  - prediction partially confirmed: reward up: ✓ (Δ+0.080), cost hold: ✗ (×1.22)

## Outcome

- stop: max-candidates · generations 3 · known spend $0.5066
- certified: g3a-seed2-glm-4-7-flash-spec-first
- seed5-qwen3-7-plus-spec-first (hash e93314677f18b138): reward 0.8, $0.0085/trial
- g2a-seed5-qwen3-7-plus-spec-first (hash 38ff179d3de4da5b): reward 0.775, $0.0083/trial
- seed3-gpt-5-mini-skeptic (hash e8492da875ed8b6e): reward 0.6, $0.0040/trial
- seed2-glm-4-7-flash-spec-first (hash 2c79fecaa4fdbbca): reward 0.575, $0.0009/trial
- g3a-seed2-glm-4-7-flash-spec-first (hash 11ef90f2168dca9a): reward 0.5, $0.0012/trial ← **recommended**

## What this run taught us

- [refuted: reward up: ✗ (Δ-0.240), cost hold: ✗ (×1.34)] The appended default coding-assistant persona is causing GLM-4.7-flash to override the packet's strict output rules and invent requirements (evidenced by false-positive summaries and hallucinated approval-gate defects in the worst-trial transcripts); replacing the system prompt with the packet should enforce structured-only output and reduce false positives without increasing cost.
- [partially confirmed: reward up: ✗ (Δ-0.320), cost up: ✓ (×0.76)] The low-thinking setting produces false positives (e.g., g3_signed flagged incorrectly) and misses seeded defects; increasing reasoning depth should improve precision and recall before we pivot to cost reduction.
- [partially confirmed: reward up: ✗ (Δ-0.040), cost hold: ✓ (×0.95)] The worst-trial false positives show the model invents requirements despite the generic prohibition. Adding a mandatory citation constraint—requiring the model to quote the exact specification rule or invariant violated before reporting—will force grounding and eliminate hallucinated findings.
- [confirmed: reward up: ✓ (Δ+0.080), cost up: ✓ (×1.31)] The worst-trial transcripts show the agent missing seeded defects and emitting false positives on nuanced approval-gate and evidence clauses; increasing thinking from medium to high should deepen invariant verification and reduce both omission and hallucination errors.
- [partially confirmed: reward up: ✓ (Δ+0.080), cost hold: ✗ (×1.22)] The default appended coding assistant prompt promises write/edit tools that are not actually provided, confusing the model and contributing to hallucinated findings and workflow errors; replacing the system prompt with only the review packet removes this contradiction.
