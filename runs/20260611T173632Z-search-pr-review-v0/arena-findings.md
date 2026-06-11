# Draft arena-iteration note (promote to a backlog ticket)

Run: 20260611T173632Z-search-pr-review-v0  mode: threshold-then-cheap

- fp-trap-never-fired: every agent passed clean task py-formatter-clean; the trap may be too easy to discriminate FP discipline

## Certification findings to address before public benchmark claims

- `py-markup-escape` remains a calibration or task-design failure for the
  recommended certified candidate: `seed4-qwen3-7-plus-checklist` scored 0/5
  at certification depth. Earlier probes also showed agents finding the
  location but disagreeing with the strict `security` category. Do not make a
  cross-agent quality claim that depends on this task until the category
  strictness is adjudicated or waived.
- `py-guess-swallow` remains hard for the certified candidate: seed4 scored
  0/5. The run did show small-n passability (`seed1` and `g2b` each hit it
  once), so the fixture is not impossible, but it needs either stronger
  execution/retrieval affordance or an explicit waiver before publication.
- `py-measure-normalize` is high variance under certification: seed4 went from
  1/1 in the search phase to 1/5 after top-up. Treat the search-phase 1.0 as
  a lucky draw, not a stable capability.
- Uncertified candidates `g2b` and `g3b` posted higher apparent means than the
  certified recommendation, but they do not have n >= 5 on train+validation and
  cannot be used for a recommendation without another certification pass.
