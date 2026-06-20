# Cerberus Lab Import Report

## Candidate

- Candidate: `omp-fixture-review`
- Substrate: `omp`
- Model: `null`

## Artifact

- Artifact: `artifact-fake-omp`
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

- request: `runs/cerberus-rd-lab-omp/request.json`
- artifact: `runs/cerberus-rd-lab-omp/artifact.json`
- findings: `runs/cerberus-rd-lab-omp/findings.json`
- score: `runs/cerberus-rd-lab-omp/score.json`
- summary: `runs/cerberus-rd-lab-omp/summary.json`
- report: `runs/cerberus-rd-lab-omp/report.md`
