# Launch the PR-review agent only under signed gates

Priority: P1
Status: pending
Estimate: XL

## Goal
Make the measured PR-review delivery importable, observable, and impossible to
mistake for production-ready until G3/G4/G5 approval artifacts exist.

## Non-Goals
- Giving the agent write authority before G4
- Replacing the control planes' own import/runtime implementations

## Oracle
- [ ] `deliveries/pr-review/contract.toml` is regenerated from certified
      evidence with a non-unknown `harness_version`, evidence pointers, and a
      concrete trace destination or explicit JSONL-only waiver
- [ ] `approvals/G3-pr-review-*.md` records the launch decision and is required
      by any deploy/import command that can create runtime-facing artifacts
- [ ] A dry-run import packet for the target control plane preserves the
      measured packet byte-for-byte and refuses unsigned contracts by default
- [ ] Regression-eval cadence is executable: one command replays the certified
      arena holdout and emits a trace/export artifact
- [ ] G4/G5 templates exist for write authority and production-data
      re-ingestion, with redaction and approval boundaries explicit
- [ ] `bin/gate` green

## Children
1. Add an approval-aware export/import path that can emit review artifacts but
   refuses deployment intent while `g3_signed = false`.
2. Bind observability for the delivery: either a live trace sink or a documented
   derived-trace artifact generated from `runner/trace.py`.
3. Produce the first non-production control-plane import dry run.
4. Add G4/G5 templates for future write authority and trace-to-fixture
   ingestion.

## Notes
**Why:** harness-readiness lane. The current contract says `g3_signed = false`,
`harness_version = "unknown"`, and trace destination `TBD`; that is acceptable
as a lab artifact, not as a launch path.
