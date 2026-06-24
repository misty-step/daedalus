# Arena freeze report: pr-review-v0 0.3.0

Status: **PASS**

| check | value |
|---|---|
| oracle mean | `1.0` |
| null mean | `0.1667` |
| one-shot probe mean | `0.6` |
| one-shot probe verdict | `unsaturated` |
| one-shot probe errors | `0` |
| one-shot probe trials | `6` |
| holdout exposures | `{"rs-retry-backoff": 0}` |

## Advisories

- contamination-resistant: all sources are private/synthetic — suitable as a holdout (040 item 4)
