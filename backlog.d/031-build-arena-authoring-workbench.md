# Build the arena authoring and calibration workbench

Priority: P1
Status: pending
Estimate: L

## Goal
Give arena authors a repeatable way to create, validate, calibrate, and rotate
tasks without relying on session memory or hand-written provenance.

## Non-Goals
- Auto-generating defects without human review
- Hiding scorer or answer-key changes behind convenience tooling

## Oracle
- [ ] A task scaffold command creates Harbor-format task directories with
      template, intent, environment, verifier, answer key, and oracle-solution
      placeholders
- [ ] A validation command checks fixture symlinks, answer-key shape, oracle
      1.0, null floor, one-shot probe behavior, split membership, and
      holdout-ledger exposure count
- [ ] Calibration support records disputed findings as ACCEPT or OUT-OF-SCOPE,
      forces version bumps on accepted key changes, and reruns baselines
- [ ] The workbench can report category/span disagreement without changing
      scorer constants
- [ ] Documentation names when auto-generated defects are worth revisiting
- [ ] `bin/gate` green

## Children
1. Wrap the existing authoring pipeline from
   `arenas/pr-review-v2/provenance.md` in commands or scripts.
2. Add category/span adjudication helpers for findings that are semantically
   right but key-misaligned.
3. Add holdout burn-threshold checks and rotation prompts.
4. Generate a freeze-gate report suitable for G2 review.

## Notes
**Why:** arena-quality lane. The scaled PR-review arena already produced
category strictness and too-hard-task questions; those should become
repeatable calibration mechanics instead of one-off notes.
