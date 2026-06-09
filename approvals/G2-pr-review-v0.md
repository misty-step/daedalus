# G2 — Eval-quality review: arena pr-review-v0 (v0.1.1)

- **Status:** pending human review
- **Prepared:** 2026-06-09, after the first full baseline comparison

## Rig validation (automated, passing)

- Oracle scores 1.0 on all 6 tasks; null scores exactly the clean-task
  fraction (0.1667). Records: `runs/20260609T223034Z-oracle.jsonl`,
  `runs/20260609T223035Z-null.jsonl`.
- Scorer unit checks: clean-task invented finding → 0.0; one-of-two defects
  plus one FP → 0.3.
- Fresh-context critic (codex, read-only) reviewed the runner/scorer/arena;
  its two blocking findings (grader tamper window, nonzero-exit scoring) and
  clean-task FP weakness are fixed in runner v0.1.0 + arena v0.1.1.

## Observations for the human reviewer

1. **Answer-key completeness (py-file-cache).** pi-kimi reported two findings
   scored as false positives that are arguably real defects: (a) concurrent
   `set()` writers for the same key race on the same deterministic temp file;
   (b) `os.rename` raises on Windows when the destination exists
   (`os.replace` is the portable atomic move). Decide: extend the answer key
   (arena version bump) or document them as out-of-scope (minor/platform).
   This is the known eval failure mode where the key punishes a better
   reviewer than its author.
2. **Output-contract compliance is part of the task (py-pagination).**
   pi-kimi spent 61s, wrote no findings.json, scored 0. Confirm we are happy
   that contract-following failures and detection failures are deliberately
   conflated in one reward (current position: yes — a review you can't parse
   is a review you didn't get).
3. **Difficulty spread looks sane:** every candidate ordering is
   interpretable (oracle 1.0 > baseline 0.667 ≈ pi 0.60 > null 0.167), no
   task is universally saturated, py-file-cache is the discriminating hard
   task, and the clean-PR trap was passed by both real candidates.
4. **Single-trial caveat:** all comparisons are n=1 per task. No keep/discard
   decision should be made from this data; Phase 1 multi-trial distributions
   exist precisely for that.
