# Build pr-review-v1: codebase-context arena from real repo snapshots

Priority: P1
Status: ready
Estimate: L

## Goal
An arena whose tasks cannot be one-shotted from the diff alone: real repo snapshots (own repos first) where seeded or historical defects require reading cross-file context, project conventions, or spec documents to find — the arena that makes composition differences (tools, context policy) actually matter.

## Non-Goals
- Visual QA / run-the-app tasks (012 decides that separately)
- Third-party or proprietary-customer repos

## Oracle
- [ ] ≥8 tasks from ≥2 real repo snapshots; each task ships evidence that the cheap one-shot baseline scores <0.5 on it while the oracle scores 1.0 (that gap is the "requires context" proof, checked per task before freeze)
- [ ] Defect mix: ≥3 cross-file (caller/callee or config/code mismatch), ≥1 spec-contradiction (workspace contains a spec/README the diff violates), ≥1 clean PR
- [ ] Data boundary check recorded per snapshot before freeze: no secrets in files or git history (gitleaks or equivalent), G1 boundaries respected — written as `arenas/<id>/provenance.md`
- [ ] Train/validation/holdout split declared (008); runs under Harbor isolation (004)

## Notes
Operator steer (one-shot-able arenas betray simple evals) + codex lane #2 +
SWE Atlas precedent (codebase-QA task family). Security lane G0.5: redaction
gate before any real-repo fixture lands. Depends: 004, 008.
