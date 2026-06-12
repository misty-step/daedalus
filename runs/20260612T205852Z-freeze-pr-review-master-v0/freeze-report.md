# Arena freeze report: pr-review-master-v0 0.1.0

Status: **FAIL**

| check | value |
|---|---|
| oracle mean | `1.0` |
| null mean | `0.25` |
| one-shot probe mean | `1.0` |
| holdout exposures | `{"missing-security-member": 2}` |

## Findings

- one-shot probe saturates the arena: 1.0000 >= oracle 1.0000 - 0.1
