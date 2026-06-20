# Cerberus Lab Import Report

## Candidate

- Candidate: `fixture-self-review`
- Substrate: `fixture`
- Model: `null`

## Artifact

- Artifact: `artifact-d4736e4b-fa5b-4fac-8344-231a36c948db`
- Lifecycle: `completed`
- Verdict: `WARN`
- Validation: passed

## Score

- Task: `ratio-zero`
- Reward: `1.0`
- Recall: `1.0`
- False positives: `0`
- Matched: `["ratio-zero"]`

## Summary

Diff-only review found one behavioral concern

The guard avoids division by zero, but returning 0.0 silently changes the mathematical meaning and may hide caller bugs.

## Residual Risk

- No surrounding call sites were available in diff-only mode.

## Evidence

- request: `runs/cerberus-rd-lab-fixture/request.json`
- artifact: `runs/cerberus-rd-lab-fixture/artifact.json`
- findings: `runs/cerberus-rd-lab-fixture/findings.json`
- score: `runs/cerberus-rd-lab-fixture/score.json`
- summary: `runs/cerberus-rd-lab-fixture/summary.json`
- report: `runs/cerberus-rd-lab-fixture/report.md`
