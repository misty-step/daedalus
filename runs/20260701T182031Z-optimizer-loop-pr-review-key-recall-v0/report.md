# Optimizer Loop: pr-review-key-recall-v0

> Plumbing proof only. This run proves that a Crucible-shaped target can reach
> Bitter Blossom Sprites and return Threshold artifacts; it does not measure
> candidate quality. The recorded score formula is
> `source_split_key_recall * remote_verdict_score`: source recall came from the
> incumbent, and the candidate-dependent signal was the remote Sprite's own
> verdict string, not answer-key grading. Treat the tables below as wiring and
> execution-health evidence, not as a trusted Pareto frontier.

- Headroom verdict: `pass`
- Budget cap: `$5.00`
- Known validation spend: `$0.2111`
- Validation tasks: `js-cart-total, js-clean-rename, py-auth-sqli, py-file-cache`
- Heldout tasks: `py-pagination, rs-retry-backoff`
- Certification: `not_certified` because the Kimi validation Sprite run stayed stale in execution and the optimizer command was stopped.

## Validation Population

| candidate | bb task | score | source recall | remote gate | cost | run | status |
|---|---|---:|---:|---:|---:|---|---|
| gepa-false-positive-averse-correctness | correctness | 0.2500 | 0.5000 | 0.5000 | $0.0972 | eb29971e0725 | advisory |
| gepa-caller-context-correctness-glm | correctness-glm | 0.5000 | 0.5000 | 1.0000 | $0.1139 | eb33567afcce | pass |
| gepa-clean-fixture-sentinel-correctness-kimi | correctness-kimi | 0.0000 | 0.5000 | 0.0000 | unknown | eb6c403c5ae7 | stale_running |

## Provisional Frontier Table

| candidate | score | cost | run |
|---|---:|---:|---|
| gepa-caller-context-correctness-glm | 0.5000 | $0.1139 | eb33567afcce |
| gepa-false-positive-averse-correctness | 0.2500 | $0.0972 | eb29971e0725 |

## Heldout Certification

Heldout certification was not claimed. The optimizer implementation writes the heldout split and ASHA promotion packet, but this first live run stopped after a stale Kimi validation arm. No heldout score was fed back into GEPA.

## Guardrail Read

- The score/cost table is not candidate-quality evidence until [[066]] grades
  Sprite artifacts against answer keys and removes self-report from the
  objective.
- The first BB correctness result returned `advisory` and caught a compact-verdict scoring bug; the code now scores compact advisory verdicts as a 0.5 remote gate and has a regression test.
- Seed trust is not certified by this first run; run the multi-seed 057 check before any launch recommendation.
- Crucible grade parity remains a caveat until the Crucible scorer matches Threshold's Rust scorer.
