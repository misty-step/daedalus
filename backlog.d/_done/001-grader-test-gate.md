# Add the repo gate: tests for the grader and runner

Priority: P0
Status: ready
Estimate: S

## Goal
One command (`bin/gate`) that blocks silent grader/runner regressions, so every future experiment stands on tested measurement.

## Non-Goals
- CI service wiring (local gate first)
- Coverage targets or lint perfection

## Oracle
- [ ] `bin/gate` runs pytest over new `tests/`: score.py edge cases (zero recall, clean-task FP hard-zero, one-of-two + FP = 0.3, malformed findings, unknown category), run.py helpers (extract_json_object, extract_pi_usage on a captured pi transcript fixture, tree_digest tamper detection), plus an offline integration test (oracle candidate = 1.0, null = clean fraction)
- [ ] `bin/gate` exits nonzero on any seeded scorer mutation (flip FP_PENALTY sign manually to verify, then revert)
- [ ] Repo `AGENTS.md` names `bin/gate` as the gate agents must run

## Notes
Agent-readiness lane: a one-line bug in score.py poisons all future evidence —
highest-impact failure mode in a measurement system. Gate must run offline
(no network, no API keys).
