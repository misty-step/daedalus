# pr-review-v2 provenance and freeze gate

## Why this is the starter task (decision record, 2026-06-10)

The proving-ground arena had to (a) defeat the one-shot saturation probe,
(b) spread agent configurations, and (c) keep the deterministic seeded-defect
scorer. Options weighed:

1. **PR review over a real large repo (chosen).** Context overflow defeats
   the probe *mechanically* (the workspace cannot be inlined), cross-file
   defectiveness rewards navigation strategy differently per composition,
   and all existing scorer/arena machinery carries over.
2. Execution-gated defects only — rejected as the primary mechanism: strong
   models often spot "runtime-only" defects statically, so probe < 0.5 is
   not guaranteed. Retained as flavor (the workspace ships rich's own test
   suite for execution-oriented agents).
3. A new task family (log triage, spec compliance) — rejected: discards the
   grader-is-gospel machinery for no extra discriminating power.

## Source

Textualize/rich v14.0.0 (MIT license), cloned from GitHub at the release
tag. Workspace per task: `rich/` (78 modules), `tests/`, `README.md`,
`pyproject.toml`, `LICENSE`, plus `PR.diff`. ~1.4MB ≈ 350–400K tokens —
beyond the one-shot probe's 262K context window.

Defects are authored for this arena (not real rich bugs); answer keys and
oracle solutions were written together with the edits, before any candidate
ran.

## Tasks

| task | split | category | defectiveness lives in |
|---|---|---|---|
| py-progress-speed | train | correctness | sample-append consumer ~12 lines below the hunk; `completed=` callers elsewhere in module |
| py-padding-clean | train | (clean FP trap) | nowhere — equivalent refactor with tempting "optimization" smell |
| py-measure-normalize | validation | correctness | `normalize()/with_maximum()` contract + layout consumers in other modules |
| py-live-lock | holdout | concurrency | `_RefreshThread.run` takes the same lock earlier in the file |

## Data boundary (G1 / security lane)

- Public MIT-licensed code; no credentials or user data. gitleaks result
  recorded below before freeze.
- Candidates never read `tests/` (the task's verifier dir) or `solution/`
  (enforced by the runner). The *workspace* `tests/` directory is rich's own
  test suite — fixture content, fair game.

## Freeze gate (ticket 015 oracle) — PASSED 2026-06-10, v0.1.0 frozen

Evidence run: `runs/20260610T160533Z-search-pr-review-v0` (rig.json,
trials.jsonl, seed.json, report.md).

- [x] oracle scores 1.0 on every task (4/4)
- [x] null scores exactly the clean-task fraction (0.25)
- [x] one-shot probe mean < 0.5 — scored **0.000** on all four tasks: the
      ~350K-token workspace exceeds its 262K context and the API rejects
      the request (HTTP 400)
- [x] agent compositions land measurably apart: six seeds spanned
      **0.167–1.000** mean reward (glm-5/spec-first 1.000 vs
      gpt-5-mini/spec-first 0.167; same packet, different model) at 230×
      cost spread — far beyond observed trial noise

Reference ordering: probe 0.000 < null 0.250 < weakest agent 0.167…
strongest agents 1.000 = oracle.

### Known calibration finding (post-freeze, for v2.1)

A fresh repro of the winning composition (2026-06-10, delivered-agent
verification) found the right defect on `py-measure-normalize` — correct
file, category, and description — but cited a line just outside the key
span [108, 111] and scored 0. Key spans should widen to the enclosing
method in v2.1 (version bump; see backlog 019), alongside more holdout
tasks. v0.1.0 stays frozen: comparisons against it remain valid.

## v0.2.0 (tickets 019 + 027, 2026-06-10)

Scale-up + calibration: ten tasks across two real repos, full taxonomy
coverage, three-task holdout, two clean traps, exposure ledger
(holdout-ledger.md, burn threshold 5 exposures → rotate + version bump).

### Freeze gate (v0.2.0): rig PASSED, new-task spread PENDING

- [x] oracle 1.0 on all 10 tasks; null 0.20 (= 2 clean / 10); one-shot
      probe 0.0 (HTTP 400, context overflow) on sampled rich + pygments
      tasks. gitleaks clean (23.2MB scanned).
- [x] agent spread proven on the v0.1.0 task subset (carried forward):
      0.167–1.000 across seeds (run 20260610T160533Z).
- [ ] **agent spread on the six NEW tasks: not yet established.** A
      sequential two-agent probe (glm-5/spec-first and glm-4.7-flash/lean)
      both scored 0 on py-markup-escape and py-guess-swallow:
      - markup-escape: agents found the right *location* (markup.py 61–62,
        inside the key span) but tagged it `correctness`/`data-loss`; the
        key requires `security`. This is category-match strictness on a
        legitimately cross-category defect — a calibration candidate
        (allow a category set, or pick the category the defect most
        load-bearingly is) for v0.2.1.
      - guess-swallow: both found nothing — genuinely hard, or the
        out-of-diff cue (blanket-except semantics) is too subtle to spot
        without running the analysers.
      A task no reasonable agent passes discriminates as poorly as one
      everyone passes. The next full `bin/daedalus run` (diverse seed
      population, ≥5-trial certification) is the real spread oracle; until
      it reports, treat the six new tasks as **provisional** and do not
      publish cross-agent claims that depend on them. The version bump,
      wider spans, second clean trap, ledger, and pygments repo all stand.

| task | split | repo | category |
|---|---|---|---|
| py-progress-speed | train | rich | correctness |
| py-padding-clean | train | rich | (clean) |
| py-markup-escape | train | rich | security |
| py-save-leak | train | rich | resource-leak |
| py-measure-normalize | validation | rich | correctness |
| py-guess-swallow | validation | pygments | error-handling |
| py-formatter-clean | validation | pygments | (clean) |
| py-live-lock | holdout | rich | concurrency |
| py-export-clear | holdout | rich | data-loss |
| py-plugin-cache | holdout | pygments | correctness |

Key spans now cover the **enclosing function** of each seeded defect
(calibration finding from the delivered-agent repro: a correct defect
identification scored 0 for citing a line a few rows outside a tight span).

### Authoring pipeline

Each task: read the real upstream code → design a single-PR edit whose
wrongness lives outside the diff hunk (callers, documented contracts,
sibling threads, stdlib semantics) → apply the edit and key + oracle
together (the build script asserts edit uniqueness and computes the
enclosing-function span mechanically) → validate oracle 1.0 / null floor
offline before any model run. Disputed findings flow through
adjudications.md (DESIGN.md, Adjudication).

### Auto-generated defects: rejected for v0.2.0, with a revisit trigger

Considered seeding defects mutation-testing style (changes that provably
flip an existing upstream test). Rejected for now: hand-authored cross-file
defects with the adjudication flywheel currently beat generated mutants on
the property that matters (out-of-diff defectiveness — mutants tend to be
locally visible), and quality control would still be manual. Revisit when
task demand exceeds authoring capacity or when execution arenas (ticket
012) make test-flipping defects first-class.

### Delivery claims (019)

A delivered agent's reward claims must come from ≥5 trials per task
(certification racing enforces this in-loop; DELIVERY.md states mean and
observed range, never a small-n point estimate).

## gitleaks

```
gitleaks detect --source arenas/pr-review-v2 --no-git
→ scanned ~5.60 MB in 568ms, no leaks found (2026-06-10, v0.1.0)
→ re-run at v0.2.0 freeze: see below
```
