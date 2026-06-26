# Experiment lineage — 20260625T161856Z-search-cerberus-reviewer

## Rig

- oracle 1.0 · null 0.1667 · one-shot probe 0.7667 — arena discriminates

## Landscape scan (seed population)

rng_seed 1 · packet stances: spec-first, trace-callers, test-runner

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-glm-5-2-spec-first | z-ai/glm-5.2 | off | explore | 0.576 | 15 | $0.089 |
| seed2-kimi-k2-7-code-trace-callers | moonshotai/kimi-k2.7-code | medium | full | 0.758 | 30 | $0.483 |
| seed3-gpt-5-4-mini-test-runner | openai/gpt-5.4-mini | low | no-exec | 0.584 | 30 | $0.235 |
| seed4-deepseek-v4-flash-spec-first | deepseek/deepseek-v4-flash | high | explore | 0.397 | 20 | $0.045 |
| seed5-deepseek-v4-pro-trace-callers | deepseek/deepseek-v4-pro | off | full | 0.562 | 20 | $0.091 |
| seed6-glm-5-2-test-runner | z-ai/glm-5.2 | medium | no-exec | 0.488 | 20 | $0.181 |

## Generations (hypothesis → measurement → decision)

- g1.0 `g1a-seed1-glm-5-2-spec-first` ← `seed1-glm-5-2-spec-first` (slot `prompt_packet`) (transplant from `seed2-kimi-k2-7-code-trace-callers`)
  - hypothesis: The trace-callers strategy from seed2 (top reward 0.876) may correctly identify defects without the mutation false positive that plagued the spec-first approach on glm-5.2, potentially translating high reward to the cheaper model.
  - measured: reward 0.52 vs parent 0.5756 (paired Δ -0.0556) → no improvement — direction discarded
  - prediction partially confirmed: reward up: ✗ (Δ-0.056), cost hold: ✓ (×0.91)
- g1.1 `g1b-seed2-kimi-k2-7-code-trace-callers` ← `seed2-kimi-k2-7-code-trace-callers` (slot `model`)
  - hypothesis: The trace-callers prompt packet (seed2) already achieved 0.8756 reward on kimi-k2.7-code. Switching to gpt-5.4-mini should retain comparable reasoning quality for call-chain analysis while reducing cost, given gpt-5.4-mini's decent performance on other reviewer configurations (0.6644 on test-runner). This is the highest-information model swap because no trial has tested gpt-5.4-mini with this exact prompt.
  - measured: reward 0.7311 vs parent 0.8756 (paired Δ -0.1444) → no improvement — direction discarded
  - prediction refuted: reward hold: ✗ (Δ-0.144), cost down: ✗ (×1.21)
- g2.0 `g2a-seed2-kimi-k2-7-code-trace-callers` ← `seed2-kimi-k2-7-code-trace-callers` (slot `prompt_packet`)
  - hypothesis: The current packet produces high-quality findings but suffers from false positives (e.g., TOCTOU/error-handling findings on lines not in the diff, or where the caller trace is incomplete). Adding an explicit verification gate that forces the model to check each finding against the actual diff and caller evidence before writing should reduce fabricated or ungrounded defects, raising reward by cutting the false-positive penalty.
  - measured: reward 0.6485 vs parent 0.8756 (paired Δ -0.2444) → no improvement — direction discarded
  - prediction refuted: reward up: ✗ (Δ-0.244), cost hold: ✗ (×1.41)
- g2.1 `g2b-g1a-seed1-glm-5-2-spec-first` ← `g1a-seed1-glm-5-2-spec-first` (slot `prompt_packet`)
  - hypothesis: The worst-trial evidence shows GLM-5.2 reports valid defects (e.g., side-effect mutation) but gets zero reward because it fails to demonstrate a concrete caller in the repo that triggers the flaw, causing findings to be scored as false positives. Adding an explicit caller-citation requirement should suppress findings unsupported by repo evidence, raising reward.
  - measured: reward 0.4422 vs parent 0.52 (paired Δ -0.0778) → no improvement — direction discarded
  - prediction refuted: reward up: ✗ (Δ-0.078), cost hold: ✗ (×1.16)

## Outcome

- stop: plateau · generations 2 · known spend $2.0969
- certified: none
- seed2-kimi-k2-7-code-trace-callers (hash 8f66a041352d3385): reward 0.7578, $0.0161/trial
- g1b-seed2-kimi-k2-7-code-trace-callers (hash 9ec001c6793125fc): reward 0.7156, $0.0150/trial
- seed3-gpt-5-4-mini-test-runner (hash 41bcc61fe7f92478): reward 0.5844, $0.0078/trial
- seed1-glm-5-2-spec-first (hash 04e2a155e7f1dd2d): reward 0.5756, $0.0060/trial
- seed5-deepseek-v4-pro-trace-callers (hash ae486ad3b306c97d): reward 0.5617, $0.0045/trial
- g1a-seed1-glm-5-2-spec-first (hash 08a2da249906fa86): reward 0.52, $0.0054/trial
- seed4-deepseek-v4-flash-spec-first (hash cb2039b025d207ee): reward 0.3967, $0.0022/trial

## What this run taught us

- [partially confirmed: reward up: ✗ (Δ-0.056), cost hold: ✓ (×0.91)] The trace-callers strategy from seed2 (top reward 0.876) may correctly identify defects without the mutation false positive that plagued the spec-first approach on glm-5.2, potentially translating high reward to the cheaper model.
- [refuted: reward hold: ✗ (Δ-0.144), cost down: ✗ (×1.21)] The trace-callers prompt packet (seed2) already achieved 0.8756 reward on kimi-k2.7-code. Switching to gpt-5.4-mini should retain comparable reasoning quality for call-chain analysis while reducing cost, given gpt-5.4-mini's decent performance on other reviewer configurations (0.6644 on test-runner). This is the highest-information model swap because no trial has tested gpt-5.4-mini with this exact prompt.
- [refuted: reward up: ✗ (Δ-0.244), cost hold: ✗ (×1.41)] The current packet produces high-quality findings but suffers from false positives (e.g., TOCTOU/error-handling findings on lines not in the diff, or where the caller trace is incomplete). Adding an explicit verification gate that forces the model to check each finding against the actual diff and caller evidence before writing should reduce fabricated or ungrounded defects, raising reward by cutting the false-positive penalty.
- [refuted: reward up: ✗ (Δ-0.078), cost hold: ✗ (×1.16)] The worst-trial evidence shows GLM-5.2 reports valid defects (e.g., side-effect mutation) but gets zero reward because it fails to demonstrate a concrete caller in the repo that triggers the flaw, causing findings to be scored as false positives. Adding an explicit caller-citation requirement should suppress findings unsupported by repo evidence, raising reward.
