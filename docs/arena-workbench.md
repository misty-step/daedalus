# Arena Authoring Workbench

The workbench turns arena maintenance from session memory into repeatable file
commands. It does not generate defects, edit scorer constants, or make human
adjudication decisions.

## Scaffold A Task

```sh
cargo run --quiet --bin threshold -- arena-scaffold arenas/<arena-id> <task-id> \
  --taskspec specs/<spec-id>/taskspec.toml
```

The scaffold creates the Harbor-format task shape:

- `intent.md`
- `environment/README.md`
- `tests/expected.json`
- `tests/test.sh`
- `solution/findings.json`
- `task.toml`

For a new arena, it also creates `arena.toml` with `[template]`, `[risk]`, and
empty split lists, plus `template.md`. Authors still have to replace the
placeholder fixture files, write the key and oracle solution together, assign
the task to exactly one split, and review risk metadata before any candidate
run.

## Validate A Freeze Gate

```sh
cargo run --quiet --bin threshold -- arena-freeze arenas/pr-review-v2 \
  --out-dir runs/<freeze-run>

cargo run --quiet --bin threshold -- arena-validate arenas/pr-review-v2 \
  --probe-run runs/<freeze-run> \
  --report runs/<freeze-run>/freeze-report.md
```

`arena-freeze` runs the reference ceiling/floor and one-shot probe without
falling through into candidate seeding or search. The default one-shot probe is
bounded for real-repo arenas: it sends task intent, `PR.diff`, changed files,
and small project anchors rather than inlining the full copied repository. If a
fixture has no `PR.diff`, the probe falls back to the full workspace so small
non-PR arenas keep their old saturation signal.
`arena-validate` is offline. It checks:

- fixture symlinks;
- answer-key shape;
- oracle solution scores 1.0;
- null scores exactly the clean-task floor;
- every task is in exactly one split;
- one-shot probe behavior from an existing run directory, including
  inconclusive error counts and saturated/unsaturated semantics;
- holdout exposure counts against the burn threshold.

If a holdout ledger has an `arena version` semver column, validation counts
only rows for the current `arena.toml` version. Legacy ledgers without that
column are still counted by task name for backward compatibility.

The validation command does not spend model budget. `arena-freeze` may spend
model budget for the one-shot probe; run it before any certified search so an
inconclusive or saturated arena stops early. A failed provider call or context
overflow is not evidence that an arena is unsaturated.

## Adjudicate Disputed Findings

```sh
cargo run --quiet --bin threshold -- arena-adjudicate arenas/<arena-id> \
  --task <task-id> \
  --finding "candidate finding summary" \
  --ruling ACCEPT \
  --rationale "why the key missed it" \
  --new-version 0.2.1 \
  --baseline-run runs/<post-change-baseline>
```

`OUT-OF-SCOPE` records the human decision without changing the arena version.
`ACCEPT` enforces discipline: it requires a higher arena version and a baseline
run containing oracle, null, and one-shot evidence. The helper reruns the
offline arena validator against the current files before it updates the
`version` line in `arena.toml` and appends `adjudications.md`; it does not edit
answer keys for you.

## Report Category Or Span Disagreement

```sh
cargo run --quiet --bin threshold -- arena-disagreements \
  --findings path/to/findings.json \
  --expected arenas/<arena>/tasks/<task>/tests/expected.json
```

The report identifies findings that are in the keyed span with the wrong
category, or in the keyed file/category just outside the span. This is a
calibration aid only. If the human ruling is `ACCEPT`, update the key in a new
arena version and rerun baselines; never loosen scorer constants to make a
candidate look better.

## Auto-Generated Defects

Revisit auto-generated or mutation-test-derived defects when one of these is
true:

- hand-authored task demand exceeds authoring capacity;
- execution-gated arenas make test-flipping mutants first-class evidence;
- generated candidates can preserve out-of-diff defectiveness instead of
  creating locally obvious changes;
- a human review loop exists to reject trivial, equivalent, or ambiguous
  mutants before they enter a frozen arena.

Until then, use generated defects only as prompts for human-authored tasks, not
as automatic answer keys.
