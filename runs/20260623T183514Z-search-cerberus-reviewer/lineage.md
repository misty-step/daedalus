# Experiment lineage — 20260623T183514Z-search-cerberus-reviewer

## Rig

- oracle 1.0 · null 0.1667 · one-shot probe 0.5222 — arena discriminates

## Landscape scan (seed population)

rng_seed 1 · packet stances: spec-first, trace-callers, test-runner

| seed | model | thinking | tools | mean | n | cost |
|---|---|---|---|---|---|---|
| seed1-glm-5-2-spec-first | z-ai/glm-5.2 | off | explore | 0.445 | 20 | $0.124 |
| seed2-kimi-k2-7-code-trace-callers | moonshotai/kimi-k2.7-code | medium | full | 0.754 | 30 | $0.551 |
| seed3-gpt-5-4-mini-test-runner | openai/gpt-5.4-mini | low | no-exec | 0.588 | 20 | $0.120 |
| seed4-deepseek-v4-flash-spec-first | deepseek/deepseek-v4-flash | high | explore | 0.440 | 20 | $0.049 |
| seed5-deepseek-v4-pro-trace-callers | deepseek/deepseek-v4-pro | off | full | 0.602 | 20 | $0.075 |
| seed6-glm-5-2-test-runner | z-ai/glm-5.2 | medium | no-exec | 0.705 | 25 | $0.179 |

## Generations (hypothesis → measurement → decision)

- g1.0 `g1a-seed3-gpt-5-4-mini-test-runner` ← `seed3-gpt-5-4-mini-test-runner` (slot `tools`)
  - hypothesis: The current tools list (read, edit, write) likely lacks execution capability, preventing the agent from running any test suites as instructed. The worst-trial failures show the agent resorted to static analysis and produced false positives. Granting the 'full' tool policy (which includes execute) should enable test execution, improving defect detection and reward.
  - measured: reward 0.6867 vs parent 0.7178 (paired Δ -0.0311) → no improvement — direction discarded
  - prediction refuted: reward up: ✗ (Δ-0.031), cost hold: ✗ (×1.14)
- g1.1 `g1b-seed1-glm-5-2-spec-first` ← `seed1-glm-5-2-spec-first` (slot `prompt_packet`) (transplant from `seed6-glm-5-2-test-runner`)
  - hypothesis: The spec-first prompt produces false positives by flagging mutations without a spec contract; the test-runner prompt from seed6-glm-5-2-test-runner (which achieved 0.7533 reward) focuses on running tests to identify actual defects, likely improving precision and recall.
  - measured: reward 0.5089 vs parent 0.5267 (paired Δ -0.0178) → no improvement — direction discarded
  - prediction partially confirmed: reward up: ✗ (Δ-0.018), cost up: ✓ (×1.36)
- g2.0 `g2a-g1a-seed3-gpt-5-4-mini-test-runner` ← `g1a-seed3-gpt-5-4-mini-test-runner` (slot `model`)
  - hypothesis: The gpt-5.4-mini with low thinking is too weak to distinguish real defects from false positives on py-file-cache; seed6-glm-5-2-test-runner achieved 0.7533 reward on similar tasks, so upgrading the model should improve defect detection precision and recall.
  - measured: reward 0.7311 vs parent 0.6867 (paired Δ 0.0444) → no improvement — direction discarded
  - prediction confirmed: reward up: ✓ (Δ+0.044), cost up: ✓ (×1.64)
- g2.1 `g2b-seed2-kimi-k2-7-code-trace-callers` ← `seed2-kimi-k2-7-code-trace-callers` (slot `prompt_packet`) (transplant from `seed6-glm-5-2-test-runner`)
  - hypothesis: The seed6 test-runner prompt instructs the agent to execute tests and use results to validate defects, which should reduce false positives compared to the current trace-callers-only prompt. By transplanting it onto the same kimi model, we isolate whether the prompt alone improves reward.
  - measured: reward 0.7822 vs parent 0.7511 (paired Δ 0.0311) → no improvement — direction discarded
  - prediction confirmed: reward up: ✓ (Δ+0.031), cost up: ✓ (×1.59)

## Outcome

- stop: plateau · generations 2 · known spend $2.5224
- certified: g2b-seed2-kimi-k2-7-code-trace-callers, seed2-kimi-k2-7-code-trace-callers, seed6-glm-5-2-test-runner
- g2b-seed2-kimi-k2-7-code-trace-callers (hash 1366fbcb37b7fb0b): reward 0.7622, $0.0312/trial
- seed2-kimi-k2-7-code-trace-callers (hash 1df8c73c5cfbb4db): reward 0.7544, $0.0184/trial ← **recommended**
- seed6-glm-5-2-test-runner (hash 11eada1eb772ce33): reward 0.7053, $0.0072/trial
- g1a-seed3-gpt-5-4-mini-test-runner (hash 687d605505c0b9ac): reward 0.6867, $0.0067/trial
- seed5-deepseek-v4-pro-trace-callers (hash 73e6809a98ab443d): reward 0.6017, $0.0037/trial
- seed3-gpt-5-4-mini-test-runner (hash 0557a75164a92fdd): reward 0.5883, $0.0060/trial
- seed4-deepseek-v4-flash-spec-first (hash 2ba897345f64ac56): reward 0.44, $0.0024/trial

## What this run taught us

- [refuted: reward up: ✗ (Δ-0.031), cost hold: ✗ (×1.14)] The current tools list (read, edit, write) likely lacks execution capability, preventing the agent from running any test suites as instructed. The worst-trial failures show the agent resorted to static analysis and produced false positives. Granting the 'full' tool policy (which includes execute) should enable test execution, improving defect detection and reward.
- [partially confirmed: reward up: ✗ (Δ-0.018), cost up: ✓ (×1.36)] The spec-first prompt produces false positives by flagging mutations without a spec contract; the test-runner prompt from seed6-glm-5-2-test-runner (which achieved 0.7533 reward) focuses on running tests to identify actual defects, likely improving precision and recall.
- [confirmed: reward up: ✓ (Δ+0.044), cost up: ✓ (×1.64)] The gpt-5.4-mini with low thinking is too weak to distinguish real defects from false positives on py-file-cache; seed6-glm-5-2-test-runner achieved 0.7533 reward on similar tasks, so upgrading the model should improve defect detection precision and recall.
- [confirmed: reward up: ✓ (Δ+0.031), cost up: ✓ (×1.59)] The seed6 test-runner prompt instructs the agent to execute tests and use results to validate defects, which should reduce false positives compared to the current trace-callers-only prompt. By transplanting it onto the same kimi model, we isolate whether the prompt alone improves reward.
