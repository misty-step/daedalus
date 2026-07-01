# Build the production-trace flywheel under G5

Priority: P2
Status: pending
Estimate: XL

> **Gated 2026-06-24** behind [[054]] and a real Cerberus G3 deploy. The
> incumbent-baseline mechanism this ticket needs is being built synthetically
> first in [[055]] (no live deploy required); 035 upgrades it to the *live*
> incumbent once Cerberus ships and emits production traces.

## Goal

Build the Phase 4 production-trace flywheel workstream, with a sandbox
rehearsal first and a real G5-gated cycle as the closing condition: a deployed
agent's real run traces are harvested, redacted, and turned into new arena
fixtures, then Phase 2 search re-runs with the *live agent as the incumbent
baseline*, producing a measured re-optimized candidate and a contract revision.
The exit is one complete live cycle — deployed agent → production evidence →
re-optimized candidate → revised contract — with the before/after delta visible
in run evidence.

## Why Now

Phases 0–2 and the Phase 3 launch-contract machinery are far enough along to
build the lab side now: the lab is scientifically rigorous, `threshold export`
emits pinned launch contracts (026), and `launch-pack` emits approval-gated
import packets for Olympus / Bitter Blossom (029). Phase 3 itself is not done:
G3 deployment remains unsigned, so no live deployed incumbent exists yet. The
G5 *template* exists (`approvals/G5-pr-review-production-reingestion.md`) but
the G5 *machinery* does not — production traces cannot yet re-enter the lab as
fixtures, so nothing compounds once a deployment does land.

This ticket owns the production-trace flywheel only. Phase 4's reopened Rust
validation-kernel work is tracked separately in
`backlog.d/045-rust-validation-kernel.md`.

This is the single piece that turns a one-shot delivery into a hill-climbing
machine: every deployed run becomes training signal for the next, cheaper,
better candidate. It is the literal mechanism behind the "private RL
environments grow stronger on real internal traces" thesis (Nadella, "A
frontier without an ecosystem is not stable", 2026-06; synthesis note
`inbox/2026-06-15-nadella-learning-loop-vs-portfolio.md` in the daybook
vault).

The harvesting + fixture-synthesis machinery can be built and certified against
existing sandbox run traces now. That rehearsal is useful evidence, but it does
not close this ticket. The final "live incumbent re-optimization" exit remains
blocked until a real G3 deploy produces production traces and G5 explicitly
approves re-ingestion.

## Non-Goals

- Auto-promoting a re-optimized candidate to production without a fresh G2/G3.
- Storing raw production transcripts, prompts, tool output, secrets, or
  customer data as lab state (redaction-first, fail-closed, per the G5 template).
- Auto-editing global harness skills/doctrine from harvested traces (the G5
  re-ingestion boundary forbids silent global mutation).
- Replacing Bitter Blossom's `bb runs export` (056) — consume it, don't rebuild
  it.

## Oracle

- [ ] A redaction-first harvester ingests run traces from a control-plane
      export (Bitter Blossom `bb runs export` per its ticket 056, or an
      Olympus equivalent) and refuses to persist secret-like or
      non-low-risk content (fail-closed, reusing `trace_record` refusal).
- [ ] G5 approval is explicit before any production-derived fixture is
      committed: named/scoped data sources, redaction or exclusion of secrets
      and private/user/customer data, consent/ownership boundaries, retention,
      fixture provenance, holdout exposure policy, and deletion/removal path.
- [ ] Harvested traces are converted into candidate arena fixtures with hidden
      answer keys, and the freeze gate (oracle ceiling / null floor /
      saturation probe) passes on the synthesized arena before it can rank.
- [ ] A search re-run accepts the currently-deployed agent as a named
      incumbent baseline, and the comparison report shows the re-optimized
      candidate's quality/cost/latency delta *against that incumbent*, not
      against a null/oracle reference.
- [ ] Certification (n ≥ 5/task, 020) gates any "re-optimized winner" claim;
      an uncertified re-optimization aborts rather than ships.
- [ ] A contract revision is emitted (`threshold export`) carrying provenance:
      which production traces seeded the fixtures, which incumbent it beat, and
      the measured delta.
- [ ] One full cycle is recorded end-to-end in `runs/` with lineage
      (24) and an entry in `runs/NOTEBOOK.md` against production-derived traces.
      A sandbox-trace rehearsal may be recorded earlier, but it is not closure.
- [ ] `bin/gate` green.

## Children

1. Trace harvester + redaction adapter that reads a control-plane export
   contract (align with Bitter Blossom 056's export schema; document the seam).
2. Trace → fixture synthesizer: derive seeded-defect-style fixtures (or
   real-repo arena entries, cf. pr-review-v2) from redacted traces; run the
   freeze gate.
3. Incumbent-baseline mode for the search loop: register the deployed agent's
   pinned contract as the baseline-to-beat; report deltas against it.
4. G5 re-ingestion command wiring the above under the existing approval gate,
   with provenance recorded on the revised contract.
5. End-to-end rehearsal on sandbox traces + NOTEBOOK entry; keep the ticket
   open until the production-derived G5 cycle lands.
6. End-to-end production-derived cycle after G3 deploy + G5 approval.

## Evidence

- (to be filled on delivery)

## Notes

**Why P2 / pending:** the machinery is unblocked and certifiable on sandbox
traces, but the *defining* exit — re-optimizing against a live deployed
incumbent — depends on a real G3 deployment of the pr-review (or another)
agent to a control plane, which G2 has not yet authorized beyond sandbox use.
Build the loop now; do not close the ticket until the live cycle lands.

**Seam discipline:** this ticket owns the threshold side (ingest → fixture →
re-optimize → revise). The control-plane side (emit run telemetry in a
threshold-consumable shape) is Bitter Blossom ticket 056. Keep the export
contract versioned and shared; do not fork it.

**Roadmap anchor:** ROADMAP.md "Phase 4 — The flywheel", specifically the
production trace harvesting → redaction → new fixtures and incumbent-baseline
re-optimization workstreams. It does not own the Rust kernel workstream.
