# Harden stale Bitter Blossom arm handling

Priority: P1 · Status: blocked · Estimate: M

Child of [[061]]. Depends on Bitter Blossom [[083]] for robust stale-running
Sprite recovery/cancellation semantics.

2026-07-01 factory groom correction: still needed, but parked until Bitter
Blossom [[083]] supplies recovery/cancellation semantics. It remains a hard
precondition for [[064]].

## Goal
Make stale Bitter Blossom Sprite arms first-class optimizer outcomes with
bounded recovery, clear cancellation or quarantine, and no path to accidental
promotion or hidden spend.

## Oracle
- [ ] Threshold detects a Sprite arm that remains `running` without fresh phase,
      cost, token, or artifact updates past the configured stale threshold.
- [ ] The optimizer records the stale arm in `asha.json`, `guardrails.json`, and
      `certification.json` with unknown cost preserved as `null`.
- [ ] After Bitter Blossom [[083]] lands, Threshold can request recovery or
      cancellation and records the recovery receipt before deciding whether the
      arm is failed, retried, or quarantined.
- [ ] Stale/unknown-cost arms never advance to held-out certification or Pareto
      frontier unless a completed recovery receipt supplies a valid verdict and
      cost.
- [ ] `bin/gate` passes.

## Verification System
- Claim: stale Sprite arms cannot corrupt optimizer score, cost, or
  certification state.
- Falsifier: a stale arm appears as pass, zero-cost success, certified, or
  silently disappears from the ledger.
- Driver: a synthetic stale receipt fixture plus a live Bitter Blossom recovery
  run after [[083]].
- Grader: unit tests for stale receipts and one live run packet with recovery.
- Evidence packet: stale fixture, live receipt, `asha.json`, `guardrails.json`,
  and report.
- Cadence: once for the 083 integration, then regression-tested.

## Notes
The 2026-07-01 optimizer run exposed this directly: `correctness-kimi` stayed
in `phase=executing` with no cost/tokens. Threshold handled it honestly as
`not_certified`; this ticket makes the handling robust and recoverable.
