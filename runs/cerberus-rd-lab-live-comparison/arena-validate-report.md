# Arena freeze report: cerberus-fixture-v0 0.1.0

Status: **FAIL**

| check | value |
|---|---|
| oracle mean | `1.0` |
| null mean | `0.0` |
| one-shot probe mean | `None` |
| one-shot probe verdict | `None` |
| one-shot probe errors | `None` |
| one-shot probe trials | `None` |
| holdout exposures | `{}` |

## Findings

- one-shot probe not checked: pass --probe-run

## Advisories

- contamination-resistant: all sources are private/synthetic — suitable as a holdout (040 item 4)
