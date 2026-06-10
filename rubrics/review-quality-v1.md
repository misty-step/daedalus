# Review-quality rubric v1 (judge family)

A versioned, hashed rubric for qualities the seeded-defect key cannot
capture. The judge scores an agent's findings on each criterion 0–5; the
mean normalizes to [0,1]. This is a SECONDARY signal: it counts toward
keep/discard only after the calibration gate passes (two independent judges
agree, and where a key exists the judge ranking agrees with it). It never
replaces the deterministic reward.

Score each criterion 0–5 (0 = absent/wrong, 3 = adequate, 5 = excellent):

- **evidence**: every finding cites a specific file and line and explains
  *why* the code is defective, not merely what it does. Vague or
  unlocatable findings score low.
- **actionability**: a developer could act on each finding without further
  investigation — the fix or its direction is clear from the finding.
- **severity_calibration**: stated/implied severity matches real impact; no
  crying wolf on cosmetic issues, no burying a data-loss bug among nits.
- **precision**: findings are real defects introduced by the change, free of
  style nitpicks, speculation, and reports on untouched code.

Do not reward length or confidence. A short, correct, well-evidenced review
beats a long uncertain one.

**Scope: judge quality GIVEN correctness; do not judge correctness.** Whether
the change was actually defective is the deterministic scorer's job — you
cannot tell a correct silence from a missed defect by looking at findings
alone. Score only the craft of what was (or wasn't) reported. Empty findings
are scored by the calibration harness against the known clean/defective
status, not by you.

## Calibration record

- **2026-06-10, v1, FAILED (and that is the finding).** Two judges
  (claude-sonnet-4.6, gpt-5-mini) over three graded samples disagreed
  (inter-judge Spearman 0.0): on empty findings for a *defective* change
  sonnet scored 1.0 ("clean, correctly silent"), gpt-5-mini scored 0.0
  ("missed it"). Root cause: the original rubric asked the judge to reward
  correct silence, but the judge cannot distinguish clean from missed from
  findings alone. The calibration gate did its job — it refused to let an
  uncalibrated judge touch keep/discard. The scope fix above is v1's
  response; re-run calibration before this rubric counts.
