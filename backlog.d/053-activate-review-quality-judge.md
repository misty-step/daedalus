# Activate the review-quality judge family (or delete it)

Priority: P0 · Status: pending · Estimate: M

> Child of [[054]] — hum bar gap #5. Elevated P1→P0 2026-06-24: review-quality
> corroboration beyond the deterministic recall/FP scorer is part of the
> deployability bar, not a nice-to-have.

## Goal
Either (a) ship one arena that uses the calibration-gated `judge.rs` family to
score review *quality* (clarity, actionability, false-positive severity) beyond
the deterministic recall/false-positive scorer, **or** (b) delete the judge
family as dead code. Resolve the unused-scoring-mechanism debt either way.

## Why
`judge.rs` exists — a calibration-gated LLM-judge scorer — but the 2026-06-23
eval mapping found **no arena ships a `scoring.toml` that invokes it**: every
arena uses the deterministic `reward = recall − 0.2·fp` scorer. That scorer is
the right spine (cheap, reproducible, ungameable, no judge variance) — but it
treats every true finding as equal and is blind to whether a real bug was
explained *usefully*. For a *reviewer* product, "found it but the comment is
useless" is a real quality gap the current rig can't see. Meanwhile a whole
calibration-gated subsystem sits unexercised — pure carrying cost. Layer 1
delete-before-adding says: prove it earns its keep or cut it.

## Oracle
- [ ] DECISION recorded (activate or delete) with rationale.
- [ ] If activate: one arena ships a `scoring.toml` that composes the
      deterministic scorer (gate) with a judge quality score (tie-breaker only,
      never able to reward an invented finding); its calibration set passes the
      judge gate; a run shows the quality score separating two compositions that
      tie on raw recall.
- [ ] If delete: `judge.rs` + its tests + wiring removed, gate green, and a note
      in the eval docs that quality is deterministic-only by design.
- [ ] `bin/gate` passes.

## Notes
The deterministic scorer must remain the gate — a judge can only *break ties*
among findings that are already deterministically true, never resurrect a
hard-zeroed invented finding. This preserves the "grader is gospel /
ungameable" invariant while letting quality matter. Surfaced by the eval mapping;
pairs with the scoring-toml plumbing in the arena format.
