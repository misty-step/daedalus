# Make Daedalus cold-start operable for a new agent

Priority: P2
Status: pending
Estimate: M

## Goal
Let a cold agent or operator discover the current command sequence, approval
state, run constraints, and residual risks from one maintained surface.

## Non-Goals
- Replacing `DESIGN.md` or `ROADMAP.md`
- Creating a marketing quickstart that omits gates or risk boundaries

## Oracle
- [ ] A single operator SOP names the exact sequence for spec, rig validation,
      certified run, export, approvals, trace export, and closeout
- [ ] The SOP links every gate artifact and distinguishes lab evidence from
      launch approval
- [ ] A lightweight doctor/check command or documented checklist catches stale
      model-primitives dates, missing approvals, unknown harness versions,
      unsupported parallel pi runs, and dirty run artifacts
- [ ] `README.md`, `ROADMAP.md`, `.agents/skills/daedalus/SKILL.md`, and
      delivery docs link to the SOP instead of duplicating command sequences
- [ ] `bin/gate` green

## Children
1. Write the cold-start SOP under `docs/`.
2. Add a machine-checkable readiness summary where cheap to do so.
3. Remove or link through duplicated operator guidance in existing docs.
4. Verify a cold read by following the SOP on a no-spend dry path.

## Notes
**Why:** harness-readiness lane. The facts exist, but a new operator currently
has to stitch them from README, DESIGN, ROADMAP, the Daedalus skill, approvals,
deliveries, and primitives.
