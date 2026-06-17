# Validate arena trustworthiness — contamination, saturation, adversarial keys

Priority: P0 · Status: ready · Estimate: L

## Goal
Establish that each arena measures real task skill — not memorized public code, not a saturated probe, not a gameable scorer — so a config tuned on it transfers to the real task.

## Oracle
- [ ] A contamination record per arena: each real-repo snapshot logs source repo + commit + whether the file/defect is plausibly in model training data; planted defects are confirmed novel (not the upstream historical bug).
- [ ] The saturation probe distinguishes "probe errored" from "probe genuinely scored low" — an errored probe does NOT count as evidence the arena is unsaturated. (pr-review-v2's probe errored to 0.0 on context overflow and was read as a pass; G2-pr-review-v2.md.)
- [ ] An adversarial/red-team pass: a candidate engineered to game the scorer (wide expected line-ranges, greedy-match exploits, plausible-but-wrong findings) earns no undue reward; scorer/keys fixed where it does.
- [ ] At least one contamination-resistant / private-holdout arena exists (defects + surrounding context not publicly indexable).

## Verification System
- Claim: arena scores reflect task skill, not leakage, saturation, or scorer-gaming.
- Falsifier: a model that has seen the public repo scores high without the planted-defect skill; or a scorer-gaming candidate wins.
- Driver: a contamination-audit pass over arenas/*; a red-team candidate run; the saturation-probe error-path unit test.
- Grader: audit table + red-team reward delta + the probe-classification logic.
- Evidence packet: a per-arena validity record committed beside arena.toml.
- Cadence: at arena freeze (G2) and on any arena version bump.

## Notes
Daedalus's planted-defect design + grader tamper-hashing + holdout ledger is a strong base; this closes the leakage / saturation-void / adversarial-key holes. External grounding: "On Leakage of Code Generation Evaluation Datasets" (arXiv 2407.07565) — the real-repo arenas use **public** code (Textualize/rich, pygments in arenas/pr-review-v2), which is training data; "Benchmark Inflation / Retro-Holdouts" (arXiv 2410.09247) and "Benchmark Data Contamination: A Survey" (arXiv 2406.04244) — private/gated holdouts + contamination-resistant formats; "Measuring what Matters: Construct Validity" (arXiv 2511.04703). Pairs with [[039]] (statistics).
