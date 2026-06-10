# Add the calibrated-judge scorer family (dual rubric)

Priority: P1
Status: ready
Estimate: M

## Goal
A second scorer family for qualities answer keys can't capture (finding quality, severity calibration, actionability): deterministic absolute checks stay primary, a weighted 0–5 rubric judge adds graded quality — calibrated before it counts.

## Non-Goals
- Judge-only scoring (red line: never the only oracle)
- Online/production judging

## Oracle
- [ ] Scorer interface supports families: `deterministic` (existing) and `judge` (rubric file + judge model + scale); arena declares which apply per task
- [ ] Judge uses a 0–5 scale with per-criterion rubric (research: 0–5 maximizes human-LLM agreement)
- [ ] Calibration gate before a judge score affects keep/discard: two independent judge models agree (Spearman ≥0.8 on a 20-output calibration set) AND judge ranking agrees with answer-key ranking on tasks that have keys
- [ ] Judge prompts/rubrics are versioned files, hashed into run records; judge cost is metered into the experiment budget

## Notes
Needed before 009's graded qualities matter; dual-rubric pattern validated by
the real-world agent benchmark (absolute functional + weighted relative).
Meta-eval checklist in DESIGN.md already anticipates the agreement checks.
