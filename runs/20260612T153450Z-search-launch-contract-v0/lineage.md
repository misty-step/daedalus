# Experiment lineage — 20260612T153450Z-search-launch-contract-v0

## Rig

- oracle 1.0 · null 0.1667 · one-shot probe 0.5333 — arena discriminates

## Landscape scan (seed population)

rng_seed 3006 · packet stances: test-runner, spec-first, skeptic

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-kimi-k2-6-test-runner | moonshotai/kimi-k2.6 | off | no-exec | 0.000 | 1 | $0.000 |
| seed2-glm-4-7-flash-spec-first | z-ai/glm-4.7-flash | low | inspect | 0.650 | 8 | $0.012 |
| seed3-gpt-5-mini-skeptic | openai/gpt-5-mini | medium | full | 0.440 | 5 | $0.023 |
| seed4-deepseek-v4-flash-test-runner | deepseek/deepseek-v4-flash | off | no-exec | 0.200 | 5 | $0.018 |
| seed5-qwen3-7-plus-spec-first | qwen/qwen3.7-plus | low | inspect | 0.850 | 8 | $0.066 |

## Generations (hypothesis → measurement → decision)

- g1.0 `g1a-seed2-glm-4-7-flash-spec-first` ← `seed2-glm-4-7-flash-spec-first` (slot `prompt_packet`)
  - hypothesis: Worst-trial evidence shows the cheap model systematically hallucinates approval-gate defects by treating unsigned g3/g4 fields as violations regardless of spec context; adding explicit conditional language to the prompt should suppress these false positives and raise reward while keeping cost minimal.
  - measured: reward 0.72 vs parent 0.68 (paired Δ 0.04) → **improvement — kept as a direction**
  - prediction partially confirmed: reward up: ✓ (Δ+0.040), cost hold: ✗ (×2.78)
- g1.1 `g1b-seed5-qwen3-7-plus-spec-first` ← `seed5-qwen3-7-plus-spec-first` (slot `model`) (transplant from `g1a-seed2-glm-4-7-flash-spec-first`)
  - hypothesis: The spec-first prompt is the main driver of the strong 0.76 reward; transplanting the proven cheaper glm-4.7-flash model tests whether threshold-level accuracy can be retained at roughly one-quarter the cost of qwen3.7-plus.
  - measured: reward 0.0 vs parent 0.76 (paired Δ -1.0) → no improvement — direction discarded
  - prediction partially confirmed: reward hold: ✗ (Δ-1.000), cost down: ✓ (×0.00)
- g2.0 `g2a-seed5-qwen3-7-plus-spec-first` ← `seed5-qwen3-7-plus-spec-first` (slot `thinking`)
  - hypothesis: The worst-trial evidence shows a false positive on trace-waiver-missing and a zero-reward sandbox-clean trial likely caused by insufficient reasoning depth; increasing thinking to medium should improve precision and recall.
  - measured: reward 0.64 vs parent 0.76 (paired Δ -0.12) → no improvement — direction discarded
  - prediction partially confirmed: reward up: ✗ (Δ-0.120), cost up: ✓ (×1.13)
- g2.1 `g2b-g1a-seed2-glm-4-7-flash-spec-first` ← `g1a-seed2-glm-4-7-flash-spec-first` (slot `model`)
  - hypothesis: The current spec-first prompt is well-structured, but glm-4.7-flash lacks the reasoning fidelity to avoid the false positives seen in the worst-trial evidence (e.g., flagging missing local evidence files and TBD observability fields as defects); upgrading to the archive's highest-performing model should raise reward toward the threshold while accepting a higher cost.
  - measured: reward 0.8 vs parent 0.72 (paired Δ 0.08) → **improvement — kept as a direction**
  - prediction confirmed: reward up: ✓ (Δ+0.080), cost up: ✓ (×4.45)
- g3.0 `g3a-g1a-seed2-glm-4-7-flash-spec-first` ← `g1a-seed2-glm-4-7-flash-spec-first` (slot `prompt_packet`) (transplant from `g2b-g1a-seed2-glm-4-7-flash-spec-first`)
  - hypothesis: g2b is the only archive candidate to reach the 0.8 threshold and it uses the same cheap glm-4.7-flash model, so its prompt_packet likely encodes instructions that suppress false positives while preserving true defects; transplanting it should replicate that reward level on this parent.
  - measured: reward 0.56 vs parent 0.72 (paired Δ -0.16) → no improvement — direction discarded
  - prediction partially confirmed: reward up: ✗ (Δ-0.160), cost up: ✓ (×1.56)

## Outcome

- stop: max-candidates · generations 3 · known spend $0.4947
- certified: g2b-g1a-seed2-glm-4-7-flash-spec-first
- seed5-qwen3-7-plus-spec-first (hash 1ffe9e7eb74fe46e): reward 0.85, $0.0083/trial
- g1a-seed2-glm-4-7-flash-spec-first (hash fd911b8153cff0d3): reward 0.825, $0.0026/trial
- seed2-glm-4-7-flash-spec-first (hash 6f310b2019f6c4cf): reward 0.65, $0.0016/trial
- seed3-gpt-5-mini-skeptic (hash 6a2351e2671d690f): reward 0.44, $0.0045/trial

## What this run taught us

- [partially confirmed: reward up: ✓ (Δ+0.040), cost hold: ✗ (×2.78)] Worst-trial evidence shows the cheap model systematically hallucinates approval-gate defects by treating unsigned g3/g4 fields as violations regardless of spec context; adding explicit conditional language to the prompt should suppress these false positives and raise reward while keeping cost minimal.
- [partially confirmed: reward hold: ✗ (Δ-1.000), cost down: ✓ (×0.00)] The spec-first prompt is the main driver of the strong 0.76 reward; transplanting the proven cheaper glm-4.7-flash model tests whether threshold-level accuracy can be retained at roughly one-quarter the cost of qwen3.7-plus.
- [partially confirmed: reward up: ✗ (Δ-0.120), cost up: ✓ (×1.13)] The worst-trial evidence shows a false positive on trace-waiver-missing and a zero-reward sandbox-clean trial likely caused by insufficient reasoning depth; increasing thinking to medium should improve precision and recall.
- [confirmed: reward up: ✓ (Δ+0.080), cost up: ✓ (×4.45)] The current spec-first prompt is well-structured, but glm-4.7-flash lacks the reasoning fidelity to avoid the false positives seen in the worst-trial evidence (e.g., flagging missing local evidence files and TBD observability fields as defects); upgrading to the archive's highest-performing model should raise reward toward the threshold while accepting a higher cost.
- [partially confirmed: reward up: ✗ (Δ-0.160), cost up: ✓ (×1.56)] g2b is the only archive candidate to reach the 0.8 threshold and it uses the same cheap glm-4.7-flash model, so its prompt_packet likely encodes instructions that suppress false positives while preserving true defects; transplanting it should replicate that reward level on this parent.
