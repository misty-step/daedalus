# Make Threshold cold-start operable for a new agent

Priority: P2
Status: done
Estimate: M

## Goal
Let a cold agent or operator discover the current command sequence, approval
state, run constraints, and residual risks from one maintained surface.

## Non-Goals
- Replacing `DESIGN.md` or `ROADMAP.md`
- Creating a marketing quickstart that omits gates or risk boundaries

## Oracle
- [x] A single operator SOP names the exact sequence for spec, rig validation,
      certified run, export, approvals, trace export, and closeout
- [x] The SOP links every gate artifact and distinguishes lab evidence from
      launch approval
- [x] A lightweight doctor/check command or documented checklist catches stale
      model-primitives dates, missing approvals, unknown harness versions,
      unsupported parallel pi runs, and dirty run artifacts
- [x] `README.md`, `ROADMAP.md`, `.agents/skills/threshold/SKILL.md`, and
      delivery docs link to the SOP instead of duplicating command sequences
- [x] `bin/gate` green

## Children
1. [x] Write the cold-start SOP under `docs/`.
2. [x] Add a machine-checkable readiness summary where cheap to do so.
3. [x] Remove or link through duplicated operator guidance in existing docs.
4. [x] Verify a cold read by following the SOP on a no-spend dry path.

## Evidence

- Added `docs/operator-sop.md` with the maintained sequence for start-clean,
  spec/G1, arena validation, certified run, export/trace, launch gates, and
  closeout.
- Added `runner/doctor.py` and `bin/threshold doctor`; it checks model
  primitive freshness, missing/unsigned approvals, unknown harness versions,
  pi sequential-run constraints, and dirty run artifacts.
- `README.md`, `ROADMAP.md`, `.agents/skills/threshold/SKILL.md`,
  `deliveries/pr-review/DELIVERY.md`, and
  `deliveries/pr-review/plane-handoff.md` link to the SOP.
- No-spend SOP smoke: `bin/threshold doctor --today 2026-06-12`;
  `bin/threshold arena-validate arenas/pr-review-v2 --probe-run
  runs/20260611T173632Z-search-pr-review-v0 --report
  /tmp/threshold-032-sop-freeze.md`; `bin/threshold launch-pack
  deliveries/pr-review --plane bitter-blossom --dry-run --out-dir
  /tmp/threshold-032-sop-launch`.
- Focused tests: `python3 -m pytest -q tests/test_doctor.py
  tests/test_workbench.py tests/test_launch.py tests/test_export.py` -> 23
  passed.
- Full gate: `bin/gate` -> 134 passed.
- Fresh critic: opencode returned `NO BLOCKING FINDINGS`.

## Notes
**Why:** harness-readiness lane. The facts exist, but a new operator currently
has to stitch them from README, DESIGN, ROADMAP, the Threshold skill, approvals,
deliveries, and primitives.
