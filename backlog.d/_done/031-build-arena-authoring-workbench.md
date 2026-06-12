# Build the arena authoring and calibration workbench

Priority: P1
Status: done
Estimate: L

## Goal
Give arena authors a repeatable way to create, validate, calibrate, and rotate
tasks without relying on session memory or hand-written provenance.

## Non-Goals
- Auto-generating defects without human review
- Hiding scorer or answer-key changes behind convenience tooling

## Oracle
- [x] A task scaffold command creates Harbor-format task directories with
      template, intent, environment, verifier, answer key, and oracle-solution
      placeholders
- [x] A validation command checks fixture symlinks, answer-key shape, oracle
      1.0, null floor, one-shot probe behavior, split membership, and
      holdout-ledger exposure count
- [x] Calibration support records disputed findings as ACCEPT or OUT-OF-SCOPE,
      forces version bumps on accepted key changes, and reruns baselines
- [x] The workbench can report category/span disagreement without changing
      scorer constants
- [x] Documentation names when auto-generated defects are worth revisiting
- [x] `bin/gate` green

## Children
1. [x] Wrap the existing authoring pipeline from
   `arenas/pr-review-v2/provenance.md` in commands or scripts.
2. [x] Add category/span adjudication helpers for findings that are semantically
   right but key-misaligned.
3. [x] Add holdout burn-threshold checks and rotation prompts.
4. [x] Generate a freeze-gate report suitable for G2 review.

## Evidence

- Added `runner/workbench.py` plus `bin/daedalus arena-scaffold`,
  `arena-validate`, `arena-adjudicate`, and `arena-disagreements`.
- `arena-scaffold` creates Harbor-format task placeholders, including
  `template.md`, `intent.md`, `environment/`, `tests/expected.json`,
  `tests/test.sh`, `solution/findings.json`, and `task.toml`.
- `arena-validate` checks fixture symlinks, answer-key shape, oracle 1.0,
  null floor, one-shot probe behavior from an existing run, split membership,
  and holdout exposure counts.
- `arena-adjudicate` records ACCEPT/OUT-OF-SCOPE, requires a version bump for
  ACCEPT, and reruns offline freeze validation before updating `arena.toml`.
- `arena-disagreements` reports category/span mismatches without changing
  scorer constants.
- Docs: `docs/arena-workbench.md`, linked from `README.md` and `DESIGN.md`,
  including the auto-generated-defect revisit triggers.
- Real validation smoke: `bin/daedalus arena-validate arenas/pr-review-v2
  --probe-run runs/20260611T173632Z-search-pr-review-v0 --report
  /tmp/daedalus-031-pr-review-v2-freeze2.md` -> PASS.
- Focused tests: `python3 -m pytest -q tests/test_workbench.py tests/test_run.py
  tests/test_launch.py tests/test_export.py` -> 47 passed.
- Full gate: `bin/gate` -> 129 passed.
- Fresh critic: opencode returned `NO BLOCKING FINDINGS`.

## Notes
**Why:** arena-quality lane. The scaled PR-review arena already produced
category strictness and too-hard-task questions; those should become
repeatable calibration mechanics instead of one-off notes.
