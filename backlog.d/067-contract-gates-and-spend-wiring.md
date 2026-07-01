# Contract gates and spend wiring

Priority: P1 · Status: parked-behind-066 · Estimate: L

Epic. Created by the 2026-07-01 factory groom.

## Goal

Make the Crucible -> Threshold -> Bitter Blossom contract fail loudly on schema
drift and enforce spend/approval gates in code before paid remote dispatch.

## Oracle

- [ ] `threshold.optimization_target.v1` parsing rejects unknown schema versions,
      missing required fields, missing scorer digests, absent split policy, and
      untrusted eval gate state.
- [ ] Crucible exports the CI and holdout-policy fields Threshold consumes, or
      Threshold refuses the target with an actionable error.
- [ ] Bitter Blossom Sprite receipts include a version/contract handshake before
      Threshold trusts cost, status, artifact refs, or verdict health fields.
- [ ] `optimize-loop --dispatch-bitterblossom` checks the required approval
      artifacts before any paid search dispatch; no G1/G2/G3 gate is
      self-approved.
- [ ] The headroom probe budget check is part of the pass/fail verdict, not only
      an informational row.
- [ ] Per-arm and total optimizer caps come from the task spec or invocation and
      feed a cross-run spend ledger; unknown cost remains `null`.
- [ ] `bin/gate` passes.

## Verification System

- Claim: a malformed target, stale contract, missing approval, or over-budget
  headroom probe fails before spend or certification.
- Falsifier: paid Sprite dispatch occurs with no approval artifact; a renamed
  Crucible field silently defaults; a headroom probe exceeds its budget and
  still passes; or unknown cost is estimated.
- Driver: contract fixture tests plus one dry-run optimize-loop command with
  approvals absent and one with approvals present.
- Grader: CLI errors, fixture snapshots, spend ledger, and guardrail report.
- Evidence packet: failing and passing command transcripts, fixture target,
  receipt fixtures, and updated docs/operator-sop.md.
- Cadence: whenever target schema, Sprite receipt schema, or approval policy
  changes.

## Children

1. **Strict target parser.** Replace silent `.get()` defaults with required-field
   parsing for deterministic branches.
2. **Crucible CI/holdout seam.** Consume exported confidence intervals and
   holdout policy instead of recomputing or duplicating silently.
3. **Bitter Blossom receipt handshake.** Record and validate receipt schema and
   runner version before trusting remote status.
4. **Approval preflight.** Enforce G1/G2 before paid search and keep G3 for
   launch recommendations.
5. **Budget enforcement.** Make headroom under-budget part of the verdict and
   read per-arm caps from the task budget.
6. **Spend ledger.** Add a cross-run accounting surface for known spend and
   unknown-cost blockers.

## Notes

- This absorbs the groom report's Contract v1 and Spend & gate wiring epics.
- It is parked behind [[066]] because strict contracts are only useful once the
  scorer boundary is real.
