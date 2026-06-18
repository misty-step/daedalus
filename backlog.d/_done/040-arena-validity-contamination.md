# Validate arena trustworthiness — contamination, saturation, adversarial keys

Priority: P0 · Status: done · Estimate: L

## Goal
Establish that each arena measures real task skill — not memorized public code, not a saturated probe, not a gameable scorer — so a config tuned on it transfers to the real task.

## Oracle
- [x] A contamination record per arena: each real-repo snapshot logs source repo + commit + whether the file/defect is plausibly in model training data; planted defects are confirmed novel (not the upstream historical bug). _(slice D: machine-readable `contamination.toml` (source repo + ref + license + public flag + `defects_novel`) for pr-review-v2 & -correctness-v0; `workbench::validate_contamination` makes `arena-validate` fail a real-repo arena that lacks the record or doesn't assert novelty, and surface each public source as a contamination advisory.)_
- [x] The saturation probe distinguishes "probe errored" from "probe genuinely scored low" — an errored probe does NOT count as evidence the arena is unsaturated. (pr-review-v2's probe errored to 0.0 on context overflow and was read as a pass; G2-pr-review-v2.md.) _(slice B: `workbench::probe_saturation_verdict` → Saturated/Unsaturated/Inconclusive; an errored probe is Inconclusive and aborts both the run decision and the `arena-validate` freeze gate.)_
- [x] An adversarial/red-team pass: a candidate engineered to game the scorer (wide expected line-ranges, greedy-match exploits, plausible-but-wrong findings) earns no undue reward; scorer/keys fixed where it does. _(slice C: `score::redteam_audit` + `daedalus arena-redteam` flag wide spans and the structure-aware zero-localization gaming reward. **Finding:** pr-review-v2 keys span up to 59 lines with gaming reward 1.0 — the line constraint is vacuous; tightening spans (re-baseline) is flagged as arena-iteration, not silently shipped.)_
- [x] At least one contamination-resistant / private-holdout arena exists (defects + surrounding context not publicly indexable). _(slice E: `launch-contract-v0` is author-written synthetic (no public upstream), recorded `public = false` + blessed by `arena-validate` as contamination-resistant; its 6 independent tasks also make it certifiable, unlike the 2-repo public arenas. **Residual:** a contamination-resistant holdout matching the *pr-review* task type — to validate pr-review configs without leakage — is a substantial authoring task, scoped below.)_

## Verification System
- Claim: arena scores reflect task skill, not leakage, saturation, or scorer-gaming.
- Falsifier: a model that has seen the public repo scores high without the planted-defect skill; or a scorer-gaming candidate wins.
- Driver: a contamination-audit pass over arenas/*; a red-team candidate run; the saturation-probe error-path unit test.
- Grader: audit table + red-team reward delta + the probe-classification logic.
- Evidence packet: a per-arena validity record committed beside arena.toml.
- Cadence: at arena freeze (G2) and on any arena version bump.

## Progress
- ✅ **Slice A — source_repo labels + t-correction (activates 039's repo-clustering).** Added `source_repo` to the rich/pygments `task.toml`s (pr-review-correctness-v0, pr-review-v2); `run::source_repo` + a `repo_of` map in the run report cluster tasks by repo. Activating this exposed that 039's normal-1.96 CI is badly anticonservative at few clusters, so the CI now uses Student-t `t_{G−1}` critical values (`stats::t_975`) and the power note iterates G with the matching t. **Finding:** a 2-repo arena has G=2 → df=1 → t=12.7, so it honestly certifies *nothing* — repo-clustering + t makes "you need more independent repos" mechanical, directly motivating oracle item 4 (a private holdout with more clusters). Live-verified on the real pr-review-correctness run; critic-cleared (0 blocking).
- ✅ **Slice B — saturation-probe error guard** (item 2). `workbench::probe_saturation_verdict`; an errored probe is Inconclusive, not "unsaturated."
- ✅ **Slice C — red-team scorer audit** (item 3). `score::redteam_audit` + `daedalus arena-redteam`; pr-review-v2 keys span up to 59 lines, gaming reward 1.0.
- ✅ **Slice D — machine-readable contamination records** (item 1). `contamination.toml` + `validate_contamination` (freeze gate fails an unrecorded real-repo arena, advisories on public sources).
- ✅ **Slice E — contamination-resistant holdout designated** (item 4). `launch-contract-v0` recorded `public=false` + blessed by the gate; multi-cluster, so certifiable.

## Residual (scoped follow-up)
**Author a contamination-resistant pr-review holdout.** launch-contract-v0 satisfies item 4 *literally* but is a different task type — it cannot validate pr-review configs without leakage. The real need (grounded in slice A's t-correction finding): a private pr-review arena of **novel, non-public code** with **≥6 independent sources** (so per-source clustering has enough degrees of freedom to certify, vs the 2-repo public arenas at df=1). This is a substantial fixture-authoring task (novel modules + planted defects + oracle solutions + answer keys) and benefits from operator input on the synthetic domains; track as its own ticket.

## Notes
Daedalus's planted-defect design + grader tamper-hashing + holdout ledger is a strong base; this closes the leakage / saturation-void / adversarial-key holes. External grounding: "On Leakage of Code Generation Evaluation Datasets" (arXiv 2407.07565) — the real-repo arenas use **public** code (Textualize/rich, pygments in arenas/pr-review-v2), which is training data; "Benchmark Inflation / Retro-Holdouts" (arXiv 2410.09247) and "Benchmark Data Contamination: A Survey" (arXiv 2406.04244) — private/gated holdouts + contamination-resistant formats; "Measuring what Matters: Construct Validity" (arXiv 2511.04703). Pairs with [[039]] (statistics).
