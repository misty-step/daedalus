# Runs retention policy + runner output hygiene

Priority: P2
Status: pending
Estimate: S

## Goal
Keep run *records* (JSONL/summaries) committed and heavy *artifacts* (transcripts, workspaces) out of git with an indexed local store, before Phase 1 multi-trial volume makes runs/ a bloat sink; fix runner stdout buffering so background runs stream progress.

## Non-Goals
- External trace sinks (014 owns Langfuse)

## Oracle
- [ ] DESIGN.md states the retention rule: records committed, artifacts under `runs/<exp-id>/artifacts/` gitignored with an index file naming what existed
- [ ] `print(..., flush=True)` (or unbuffered mode) in runner; backgrounded runs show per-trial lines live
- [ ] A 4-composition × 5-trial experiment adds <100KB to git

## Notes
Agent-readiness lane finding #2; buffering observed during the first live runs
(stdout silent until exit).
