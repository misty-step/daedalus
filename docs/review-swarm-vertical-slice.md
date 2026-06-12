# Review Swarm Vertical Slice

Prepared for backlog 034 on 2026-06-12.

This document records the first non-deploying review-swarm slice:
`general + correctness + security + master`. It is a Daedalus lab artifact,
not a production launch packet and not an enterprise-readiness claim.

## Member Specs

| member | task spec | current status | measured source |
|---|---|---|---|
| general | `specs/pr-review/taskspec.toml` | certified existing baseline | `deliveries/pr-review/DELIVERY.md` |
| correctness | `specs/pr-review-correctness/taskspec.toml` | suite spec ready; search blocked on G1 | adapted from correctness-owned `pr-review-v2` tasks |
| security | `specs/pr-review-security/taskspec.toml` | suite spec ready; needs another unambiguous fixture before strong G2 | adapted from `py-markup-escape` plus future security fixture |
| master | `specs/pr-review-master/taskspec.toml` | synthetic member-artifact arena authored; one-shot probe blocked on G1 | `arenas/pr-review-master-v0` |

The existing `pr-review-v2` arena remains the general-review baseline and the
real-repo-scale source for the first specialist adaptations. Its categories do
not exactly match the review-swarm taxonomy, so the specialist specs declare
owned categories and adapted tasks explicitly instead of pretending the whole
arena is a correctness or security benchmark.

## Arena State

`arenas/pr-review-master-v0` contains four synthetic master-synthesis tasks:

- `credential-duplicate`: duplicate credential exposure plus one unsupported
  correctness report; expected output keeps one blocking credential finding.
- `runtime-crash`: duplicate crash reports plus one speculative security
  report; expected output keeps one blocking runtime-crash finding.
- `clean-noise`: member noise on a sound change; expected output is empty.
- `missing-security-member`: security member timeout with a real correctness
  finding; expected output keeps the correctness finding and does not invent
  security coverage.

No candidate-visible fixture contains hidden `tests/` or `solution/` labels.
The master arena is a reducer benchmark only: it measures dedupe, severity,
false-positive suppression, and coverage disclosure over member artifacts. It
does not measure fresh source-code defect discovery.

## Reference Evidence

No-spend reference run:

```sh
python3 runner/run.py --candidate candidates/oracle.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T205852Z-freeze-pr-review-master-v0 --split all --trials 1 --final
python3 runner/run.py --candidate candidates/null.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T205852Z-freeze-pr-review-master-v0 --split all --trials 1 --final
bin/daedalus arena-validate arenas/pr-review-master-v0 --probe-run runs/20260612T205852Z-freeze-pr-review-master-v0 --report runs/20260612T205852Z-freeze-pr-review-master-v0/freeze-report.md
```

Observed:

- oracle mean: `1.0`
- null mean: `0.25`
- holdout exposures: `{"missing-security-member": 2}`
- freeze report status: `FAIL`, because one-shot probe records are absent
- blocker: `approvals/G1-pr-review-suite.md` is pending, so model-budget
  one-shot/search runs are not allowed yet

## Plane Incumbents

Current Bitter Blossom surfaces, read on 2026-06-12:

- `plane/tasks/correctness/card.md` and `plane/tasks/security/card.md` are
  read-only verdict commissions. They fetch the target rev, review one lens,
  emit one strict JSON verdict, and never post, push, merge, or edit code.
- `plane/tasks/review/card.md` is the older general reviewer path; it posts
  exactly one PR comment with `gh pr comment`, so it is not the preferred
  swarm-member boundary.
- `plane/tasks/arbiter/card.md` settles one disputed blocking finding and
  preserves fingerprints.

Current Olympus Charon surfaces, read on 2026-06-12:

- `orchestrator/agent-specs/charon.yaml` defines Charon v3 on `pi` with
  `moonshotai/kimi-k2.7-code`, high reasoning effort, a 20 minute timeout,
  and a $3 per-run budget.
- `orchestrator/prompts/charon-review.md` requires strict JSON output to
  `/home/sprite/review/charon-review.json`; the agent never posts.
- `orchestrator/src/charon-review-poster.ts` validates JSON shape and
  severity, caps findings, validates diff anchors, suppresses superseded-head
  reviews, suppresses duplicates with a hidden marker, and owns posting.

The swarm handoff must preserve the safer common boundary: members write
artifacts only; any posting remains control-plane owned and gated by G3/G4.

## Next Gate

Human G1 for `pr-review-suite` must approve the suite spec, master spec,
taxonomy, specialist member specs, and budget envelope before Daedalus spends
model budget on the one-shot probe or certified search. Passing G1 would
unlock:

1. one-shot probe for `pr-review-master-v0`;
2. first certified master search;
3. correctness/security specialist candidate runs;
4. real-member replay through the master benchmark;
5. suite export into `deliveries/pr-review-swarm/` only if replay and budget
   gates pass, otherwise a `member-only` inspection packet.
