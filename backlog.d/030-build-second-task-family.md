# Build a second task family to pressure-test Daedalus

Priority: P1
Status: pending G1 human review
Estimate: XL

## Goal
Prove Daedalus is not a PR-review-only optimizer by running the full
spec-to-delivery loop on a second domain with different artifacts, failure
modes, and scoring pressure.

## Non-Goals
- Copying the PR-review taxonomy into an unrelated domain
- Deploying the second agent before its own G3

## Oracle
- [ ] A new `specs/<id>/taskspec.toml` has G1 approval, mode, output contract,
      negative examples, data boundaries, and human checkpoints
- [ ] A new `arenas/<id>/` passes oracle/null/probe rig checks and demonstrates
      agent spread before search claims are trusted
- [ ] The run emits the standard Daedalus records, lineage, report, and Pareto
      archive without PR-review-specific special cases
- [ ] Any schema or runner assumptions discovered during the second family are
      either generalized behind file contracts or deliberately rejected in
      `DESIGN.md`
- [ ] The result updates the reopen trigger for a Rust validation kernel with
      evidence from two task families
- [ ] `bin/gate` green

## Children
1. [x] Pick the domain by evidence, with backlog grooming, inbox triage, browser QA,
   or launch-contract review as candidate families.
2. [ ] Author the new G1 task spec and arena freeze gate.
3. [ ] Run the one-shot probe and prepare/sign G2 before trusting search
   scores.
4. [ ] Run the first certified comparative search and delivery export.
5. [ ] Audit hard-coded PR-review assumptions in docs, runner output, and export
   contracts.

## Current Evidence

- Chosen second domain: `launch-contract-v0`, reviewing control-plane launch
  contracts and import packets for approval, evidence, permission,
  observability, and portability defects.
- Spec: `specs/launch-contract/taskspec.toml`
- Arena: `arenas/launch-contract-v0`
- G1 packet: `approvals/G1-launch-contract.md` (pending human review)
- No-spend rig run: `runs/20260612T000000Z-freeze-launch-contract-v0`
  records oracle 1.0 and null 0.1667, matching one clean task out of six.
- One-shot probe and comparative search are intentionally deferred until G1
  approval because they spend model budget.
- G2 remains required after the one-shot probe and before trusting any
  comparative search result.

## Notes
**Why:** premise-challenge lane. The README describes Daedalus as a general
agent-building lab, but all measured evidence currently comes from PR review.
The next ambitious proof is domain transfer.
