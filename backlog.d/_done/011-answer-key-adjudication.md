# Define the answer-key adjudication workflow

Priority: P2
Status: pending
Estimate: S

## Goal
A standing procedure for "the candidate found something the key doesn't list": adjudicate, then either extend the key (arena version bump, baselines re-run) or record it as out-of-scope — so keys improve instead of silently punishing better reviewers.

## Non-Goals
- Automating adjudication (human judgment, by design)

## Oracle
- [ ] `arenas/<id>/adjudications.md` format defined; the two open py-file-cache disputes (temp-file write race; os.rename vs os.replace) adjudicated through it as the worked example
- [ ] DESIGN.md documents the rule: key changes bump arena version and invalidate cross-version averaging

## Notes
Direct product of the first G2 review (approvals/G2-pr-review-v0.md
observation 1). Small, but it is the eval-improvement flywheel.
