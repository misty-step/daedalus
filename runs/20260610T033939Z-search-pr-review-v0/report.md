# Experiment comparison

## Compositions

| candidate | kind | model | hash | trials | voided |
|---|---|---|---|---|---|
| oracle | oracle | — | dac86ce481e480d9 | 2 | 0 |
| baseline-oneshot | oneshot | moonshotai/kimi-k2.6 | 546acf97c8be1b1a | 4 | 0 |
| pi-kimi | pi | moonshotai/kimi-k2.6 | 37e97969ef9a457e | 4 | 0 |
| gen1-baseline-oneshot | oneshot | moonshotai/kimi-k2.6 | 97c0efdf5e7b9942 | 4 | 0 |
| gen2-baseline-oneshot | oneshot | moonshotai/kimi-k2.6 | 5c9f64a452c694f4 | 4 | 1 |
| null | null | — | eaedabf2780259e2 | 2 | 0 |

## Mean reward per task (n trials in parentheses)

| candidate | discount-after-tax | extract-subtotal | **overall** |
|---|---|---|---|
| oracle | 1.00 (1) | 1.00 (1) | **1.0000** |
| baseline-oneshot | 1.00 (2) | 1.00 (2) | **1.0000** |
| pi-kimi | 1.00 (2) | 1.00 (2) | **1.0000** |
| gen1-baseline-oneshot | 1.00 (2) | 1.00 (2) | **1.0000** |
| gen2-baseline-oneshot | 1.00 (2) | 0.50 (2) | **0.7500** |
| null | 0.00 (1) | 1.00 (1) | **0.5000** |

## Cost and latency

| candidate | total cost | mean wall/task |
|---|---|---|
| oracle | $0.0000 | 0.0s |
| baseline-oneshot | $0.0345 | 57.0s |
| pi-kimi | $0.0605 | 157.8s |
| gen1-baseline-oneshot | $0.0525 | 123.9s |
| gen2-baseline-oneshot | $0.0752 | 100.8s |
| null | $0.0000 | 0.0s |

## Pareto set (reward ↑, cost ↓, latency ↓)

- baseline-oneshot

## Recommendation

**baseline-oneshot** — mean reward 1.0000 at $0.0345 (57.0s mean wall). Within-0.05 reward ties resolve to the cheapest candidate.

_Reference candidates (oracle/null) bound the verifier; they are excluded from Pareto and recommendation._
