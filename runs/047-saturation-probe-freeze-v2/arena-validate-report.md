# Arena freeze report: pr-review-correctness-v0 0.3.0

Status: **PASS**

| check | value |
|---|---|
| oracle mean | `1.0` |
| null mean | `0.25` |
| one-shot probe mean | `0.625` |
| one-shot probe verdict | `unsaturated` |
| one-shot probe errors | `0` |
| one-shot probe trials | `8` |
| holdout exposures | `{"py-formatter-missing-crash": 0, "py-live-lock": 0}` |

## Advisories

- contamination: source Textualize/rich is public — plausibly in model training data; pair with a contamination-resistant holdout before trusting rankings
- contamination: source pygments/pygments is public — plausibly in model training data; pair with a contamination-resistant holdout before trusting rankings
