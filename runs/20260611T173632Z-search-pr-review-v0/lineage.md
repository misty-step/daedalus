# Experiment lineage — 20260611T173632Z-search-pr-review-v0

## Rig

- oracle 1.0 · null 0.2 · one-shot probe 0.0 — arena discriminates

## Landscape scan (seed population)

rng_seed 2806 · packet stances: checklist, trace-callers, test-runner

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-gpt-5-mini-checklist | openai/gpt-5-mini | high | full | 0.571 | 7 | $0.124 |
| seed2-kimi-k2-6-trace-callers | moonshotai/kimi-k2.6 | medium | explore | 0.000 | 1 | $0.000 |
| seed3-glm-4-7-flash-test-runner | z-ai/glm-4.7-flash | off | no-exec | 0.500 | 8 | $0.011 |
| seed4-qwen3-7-plus-checklist | qwen/qwen3.7-plus | low | full | 0.571 | 35 | $0.595 |
| seed5-deepseek-v4-flash-trace-callers | deepseek/deepseek-v4-flash | high | explore | 0.429 | 7 | $0.117 |
| seed6-glm-5-test-runner | z-ai/glm-5 | medium | no-exec | 0.571 | 7 | $0.268 |

## Generations (hypothesis → measurement → decision)

- g1.0 `g1a-seed1-gpt-5-mini-checklist` ← `seed1-gpt-5-mini-checklist` (slot `model`)
  - hypothesis: The current gpt-5-mini model with this checklist prompt hallucinates false positives and misses real defects (0.57 reward); archive candidate seed4 shows qwen3.7-plus with a checklist prompt achieves the highest reward in the archive (0.71), suggesting model capability is the primary bottleneck.
  - measured: reward 0.7143 vs parent 0.5714 (paired Δ 0.1429) → **improvement — kept as a direction**
  - prediction partially confirmed: reward up: ✓ (Δ+0.143), cost down: ✗ (×1.33)
- g1.1 `g1b-seed4-qwen3-7-plus-checklist` ← `seed4-qwen3-7-plus-checklist` (slot `prompt_packet`) (transplant from `g1a-seed1-gpt-5-mini-checklist`)
  - hypothesis: g1a-seed1 reaches the current best mean reward of 0.7143 on a weaker gpt-5-mini model, suggesting its prompt variant is more robust than the seed4 checklist; moving it onto the stronger qwen3.7-plus targets the false-positive and false-negative failures seen in py-markup-escape and py-guess-swallow to push mean reward above the 0.7143 ceiling.
  - measured: reward 0.4286 vs parent 0.7143 (paired Δ -0.2857) → no improvement — direction discarded
  - prediction refuted: reward up: ✗ (Δ-0.286), cost hold: ✗ (×1.30)
- g2.0 parent `seed4-qwen3-7-plus-checklist` — **proposal rejected**: slot 'None' is not mutable (allowed: ['agents_md', 'model', 'prompt_packet', 'skills', 'system_prompt_mode', 'thinking', 'tools'])
- g2.1 `g2b-seed1-gpt-5-mini-checklist` ← `seed1-gpt-5-mini-checklist` (slot `prompt_packet`) (transplant from `g1a-seed1-gpt-5-mini-checklist`)
  - hypothesis: The current checklist prompt on gpt-5-mini produced false positives and missed a real defect (seed1 reward 0.57). The g1a variant with the same model achieved 0.71, suggesting its prompt packet better constrains analysis. Transplanting it should raise reward by reducing hallucinated findings and catching the missed seeded defect.
  - measured: reward 0.6857 vs parent 0.5714 (paired Δ 0.1143) → **improvement — kept as a direction**
  - prediction partially confirmed: reward up: ✓ (Δ+0.114), cost hold: ✗ (×1.51)
- g3.0 `g3a-g1a-seed1-gpt-5-mini-checklist` ← `g1a-seed1-gpt-5-mini-checklist` (slot `thinking`)
  - hypothesis: The worst-trial transcripts show that 'high' thinking drives elaborate hallucination of edge-case defects (two false positives in py-markup-escape) and long reasoning chains that inflate token usage without improving detection. Lowering thinking to 'medium' should suppress this over-generation, reduce cost, and improve mean reward by eliminating spurious findings while still catching clear-cut logic bugs.
  - measured: reward 0.5714 vs parent 0.7143 (paired Δ -0.1429) → no improvement — direction discarded
  - prediction refuted: reward up: ✗ (Δ-0.143), cost down: ✗ (×0.99)
- g3.1 `g3b-g2b-seed1-gpt-5-mini-checklist` ← `g2b-seed1-gpt-5-mini-checklist` (slot `prompt_packet`)
  - hypothesis: The worst-trial evidence shows gpt-5-mini hallucinating speculative API-contract and correctness defects when forced through a strict checklist; adding an explicit prohibition against hypothetical issues will suppress these false positives while preserving the systematic coverage that catches real resource leaks.
  - measured: reward 0.7143 vs parent 0.6857 (paired Δ 0.0286) → **improvement — kept as a direction**
  - prediction confirmed: reward up: ✓ (Δ+0.029), cost down: ✓ (×0.66)
- g4.0 `g4a-g1a-seed1-gpt-5-mini-checklist` ← `g1a-seed1-gpt-5-mini-checklist` (slot `thinking`)
  - hypothesis: The checklist prompt already structures the review workflow, so reducing thinking from high to medium should cut reasoning-token costs without materially hurting defect detection, and may even reduce the false positives observed in py-markup-escape by curtailing over-analysis.
  - measured: reward 0.5714 vs parent 0.7143 (paired Δ -0.1429) → no improvement — direction discarded
  - prediction partially confirmed: reward hold: ✗ (Δ-0.143), cost down: ✓ (×0.81)

## Meta-eval alarms

- **fp-trap-never-fired**: every agent passed clean task py-formatter-clean; the trap may be too easy to discriminate FP discipline

## Outcome

- stop: max-candidates · generations 4 · known spend $1.7639
- certified: seed4-qwen3-7-plus-checklist
- g2b-seed1-gpt-5-mini-checklist (hash 73c5a2b2adde67a4): reward 0.6857, $0.0267/trial
- g3b-g2b-seed1-gpt-5-mini-checklist (hash 3c7c3cbe5648b374): reward 0.6636, $0.0219/trial
- seed4-qwen3-7-plus-checklist (hash 4a73f1fd213aa1a5): reward 0.5714, $0.0170/trial ← **recommended**
- seed5-deepseek-v4-flash-trace-callers (hash a4a2fc6b5ee7c97a): reward 0.4286, $0.0168/trial

## What this run taught us

- [partially confirmed: reward up: ✓ (Δ+0.143), cost down: ✗ (×1.33)] The current gpt-5-mini model with this checklist prompt hallucinates false positives and misses real defects (0.57 reward); archive candidate seed4 shows qwen3.7-plus with a checklist prompt achieves the highest reward in the archive (0.71), suggesting model capability is the primary bottleneck.
- [refuted: reward up: ✗ (Δ-0.286), cost hold: ✗ (×1.30)] g1a-seed1 reaches the current best mean reward of 0.7143 on a weaker gpt-5-mini model, suggesting its prompt variant is more robust than the seed4 checklist; moving it onto the stronger qwen3.7-plus targets the false-positive and false-negative failures seen in py-markup-escape and py-guess-swallow to push mean reward above the 0.7143 ceiling.
- [partially confirmed: reward up: ✓ (Δ+0.114), cost hold: ✗ (×1.51)] The current checklist prompt on gpt-5-mini produced false positives and missed a real defect (seed1 reward 0.57). The g1a variant with the same model achieved 0.71, suggesting its prompt packet better constrains analysis. Transplanting it should raise reward by reducing hallucinated findings and catching the missed seeded defect.
- [refuted: reward up: ✗ (Δ-0.143), cost down: ✗ (×0.99)] The worst-trial transcripts show that 'high' thinking drives elaborate hallucination of edge-case defects (two false positives in py-markup-escape) and long reasoning chains that inflate token usage without improving detection. Lowering thinking to 'medium' should suppress this over-generation, reduce cost, and improve mean reward by eliminating spurious findings while still catching clear-cut logic bugs.
- [confirmed: reward up: ✓ (Δ+0.029), cost down: ✓ (×0.66)] The worst-trial evidence shows gpt-5-mini hallucinating speculative API-contract and correctness defects when forced through a strict checklist; adding an explicit prohibition against hypothetical issues will suppress these false positives while preserving the systematic coverage that catches real resource leaks.
- [partially confirmed: reward hold: ✗ (Δ-0.143), cost down: ✓ (×0.81)] The checklist prompt already structures the review workflow, so reducing thinking from high to medium should cut reasoning-token costs without materially hurting defect detection, and may even reduce the false positives observed in py-markup-escape by curtailing over-analysis.
- [arena] every agent passed clean task py-formatter-clean; the trap may be too easy to discriminate FP discipline
