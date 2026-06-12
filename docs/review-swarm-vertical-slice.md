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
| verification | `specs/pr-review-verification/taskspec.toml` | optional non-runnable scaffold; blocked on headroom fixtures | no recommendation yet |
| simplification | `specs/pr-review-simplification/taskspec.toml` | optional non-runnable scaffold; blocked on deterministic taste-free fixtures | no recommendation yet |
| product | `specs/pr-review-product/taskspec.toml` | optional non-runnable scaffold; blocked on explicit ticket-context fixtures | no recommendation yet |
| master | `specs/pr-review-master/taskspec.toml` | v0.2 synthetic reducer arena frozen; certified Qwen baseline pending human G2 | `runs/20260612T220412Z-search-pr-review-master` |

The existing `pr-review-v2` arena remains the general-review baseline and the
real-repo-scale source for the first specialist adaptations. Its categories do
not exactly match the review-swarm taxonomy, so the specialist specs declare
owned categories and adapted tasks explicitly instead of pretending the whole
arena is a correctness or security benchmark.

The optional verification, simplification, and product specs are scaffold-only
records. They deliberately omit `[search]`, set `[scaffold].runnable = false`,
and use `scaffold-only:*` fixture markers so a future operator cannot mistake
them for calibrated search targets.

## Arena State

`arenas/pr-review-master-v0` contains six synthetic master-synthesis tasks in
version `0.2.0`:

- `credential-duplicate`: duplicate credential exposure plus one unsupported
  correctness report; expected output keeps one blocking credential finding.
- `runtime-crash`: duplicate crash reports plus one speculative security
  report; expected output keeps one blocking runtime-crash finding.
- `clean-noise`: member noise on a sound change; expected output is empty.
- `missing-security-member`: security member timeout with a real correctness
  finding; expected output keeps the correctness finding and does not invent
  security coverage.
- `dual-defect-conflict`: two distinct defects with cross-member ownership and
  severity conflicts; expected output keeps both.
- `gate-regression`: a false-green verification break plus a distinct runtime
  crash; expected output keeps both.

No candidate-visible fixture contains hidden `tests/` or `solution/` labels.
The master arena is a reducer benchmark only: it measures dedupe, severity,
false-positive suppression, and coverage disclosure over member artifacts. It
does not measure fresh source-code defect discovery.

v0.2.0 fixes the v0.1.0 one-shot saturation by expanding each
candidate-visible `member_artifacts.json` to roughly 2 MB of synthetic
prefilter/member noise. This creates context-overflow headroom for the
one-shot probe while preserving file-tool accessibility for agentic
candidates. It is still synthetic: member artifacts include triage metadata
that is too label-like for a public benchmark claim, so real-member replay is
required before full-swarm export.

## Reference Evidence

v0.1.0 reference/probe run:

```sh
python3 runner/run.py --candidate candidates/oracle.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T205852Z-freeze-pr-review-master-v0 --split all --trials 1 --final
python3 runner/run.py --candidate candidates/null.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T205852Z-freeze-pr-review-master-v0 --split all --trials 1 --final
python3 runner/run.py --candidate candidates/probe-oneshot.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T205852Z-freeze-pr-review-master-v0 --split all --trials 1 --final --max-errors 1
bin/daedalus arena-validate arenas/pr-review-master-v0 --probe-run runs/20260612T205852Z-freeze-pr-review-master-v0 --report runs/20260612T205852Z-freeze-pr-review-master-v0/freeze-report.md
```

Observed:

- oracle mean: `1.0`
- null mean: `0.25`
- one-shot probe mean: `1.0`
- freeze report status: `FAIL`, because v0.1.0 saturated

v0.2.0 freeze/search evidence:

```sh
python3 runner/run.py --candidate candidates/oracle.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T215810Z-freeze-pr-review-master-v020 --split all --trials 1 --final
python3 runner/run.py --candidate candidates/null.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T215810Z-freeze-pr-review-master-v020 --split all --trials 1 --final
python3 runner/run.py --candidate candidates/probe-oneshot.toml --arena arenas/pr-review-master-v0 --exp-dir runs/20260612T215810Z-freeze-pr-review-master-v020 --split all --trials 1 --final
bin/daedalus arena-validate arenas/pr-review-master-v0 --probe-run runs/20260612T220412Z-search-pr-review-master --report runs/20260612T220412Z-search-pr-review-master/freeze-report.md
bin/daedalus run specs/pr-review-master/taskspec.toml --rng-seed 3406 --budget-usd 0.55 --max-candidates 0 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2
bin/daedalus trace runs/20260612T220412Z-search-pr-review-master
```

Observed:

- freeze report status: `PASS`
- oracle mean: `1.0`
- null mean: `0.1667`
- one-shot probe mean: `0.0` with six HTTP 400 context-overflow errors and
  known `$0.0000` cost
- current holdout exposures: `{"missing-security-member": 2}`
- certified recommendation:
  `seed2-qwen3-7-plus-spec-first` / `qwen/qwen3.7-plus` /
  `491643a3b1de61e3`
- certified reward: `1.0` across train, validation, and holdout in this
  bounded run
- candidate cost/latency: `$0.2158` over 12 trials, `$0.0180` per trial,
  93.6s mean wall/task
- total known experiment spend including optimizer calls, certification, and
  holdout: `$0.5290`
- meta-eval alarm: every agent passed `clean-noise`, so the clean trap may be
  too easy to discriminate false-positive discipline

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

Human G1 for `pr-review-suite` is approved for low-risk offline synthetic
experiments. The next human gate is G2 review of
`approvals/G2-pr-review-master-v0.md`.

Remaining work before any suite export:

1. run correctness/security specialist candidate searches or explicitly defer
   them behind stronger specialist arenas;
2. replay the master benchmark with artifacts emitted by real member
   candidates, not generated synthetic member artifacts;
3. export `deliveries/pr-review-swarm/` only if replay and budget gates pass;
4. otherwise emit a `member-only` inspection packet with the failing master
   evidence attached.
