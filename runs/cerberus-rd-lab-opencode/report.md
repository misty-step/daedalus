# Cerberus Lab Import Report

## Candidate

- Candidate: `opencode-fixture-review`
- Substrate: `opencode`
- Model: `null`

## Artifact

- Artifact: `artifact-fake-opencode`
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

- request: `runs/cerberus-rd-lab-opencode/request.json`
- artifact: `runs/cerberus-rd-lab-opencode/artifact.json`
- findings: `runs/cerberus-rd-lab-opencode/findings.json`
- score: `runs/cerberus-rd-lab-opencode/score.json`
- summary: `runs/cerberus-rd-lab-opencode/summary.json`
- report: `runs/cerberus-rd-lab-opencode/report.md`
