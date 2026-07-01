# Launch the PR-review agent only under signed gates

Priority: P1
Status: done
Estimate: XL

## Goal
Make the measured PR-review delivery importable, observable, and impossible to
mistake for production-ready until G3/G4/G5 approval artifacts exist.

## Non-Goals
- Giving the agent write authority before G4
- Replacing the control planes' own import/runtime implementations

## Oracle
- [x] `deliveries/pr-review/contract.toml` is regenerated from certified
      evidence with a non-unknown `harness_version`, evidence pointers, and a
      concrete trace destination or explicit JSONL-only waiver
- [x] `approvals/G3-pr-review-*.md` records the launch decision and is required
      by any deploy/import command that can create runtime-facing artifacts
- [x] A dry-run import packet for the target control plane preserves the
      measured packet byte-for-byte and refuses unsigned contracts by default
- [x] Regression-eval cadence is executable: one command replays the certified
      arena holdout and emits a trace/export artifact
- [x] G4/G5 templates exist for write authority and production-data
      re-ingestion, with redaction and approval boundaries explicit
- [x] `bin/gate` green

## Children
1. [x] Add an approval-aware export/import path that can emit review artifacts but
   refuses deployment intent while `g3_signed = false`.
2. [x] Bind observability for the delivery: either a live trace sink or a documented
   derived-trace artifact generated from `runner/trace.py`.
3. [x] Produce the first non-production control-plane import dry run.
4. [x] Add G4/G5 templates for future write authority and trace-to-fixture
   ingestion.

## Evidence

- Approval-aware import command:
  `bin/threshold launch-pack deliveries/pr-review --plane bitter-blossom`
  refuses while G3 is unsigned.
- Sandbox packet:
  `deliveries/pr-review/launch-dry-run/bitter-blossom.import-packet.toml`
- G3/G4/G5 gates:
  `approvals/G3-pr-review-seed4-qwen3-7-plus-checklist.md`,
  `approvals/G4-pr-review-write-authority.md`,
  `approvals/G5-pr-review-production-reingestion.md`
- Trace artifact:
  `runs/20260611T173632Z-search-pr-review-v0/trace.otel.json`
- Regression replay command:
  `runs/pr-review-regression-dry-run/regression-command.txt`

## Notes
**Why:** harness-readiness lane. The current contract says `g3_signed = false`,
`harness_version = "unknown"`, and trace destination `TBD`; that is acceptable
as a lab artifact, not as a launch path.

G2 for ticket 028 accepted the v0.2.0 contract only for internal Threshold
learning and sandboxed plane experiments. Ticket 029 must preserve that
boundary: Bitter Blossom may not run the packet as a primary reviewer before
G3, and any import/dry run should be secondary to the existing review path.
