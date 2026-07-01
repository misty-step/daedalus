# Build a second task family to pressure-test Threshold

Priority: P1
Status: done
Estimate: XL

## Goal
Prove Threshold is not a PR-review-only optimizer by running the full
spec-to-delivery loop on a second domain with different artifacts, failure
modes, and scoring pressure.

## Non-Goals
- Copying the PR-review taxonomy into an unrelated domain
- Deploying the second agent before its own G3

## Oracle
- [x] A new `specs/<id>/taskspec.toml` has G1 approval, mode, output contract,
      negative examples, data boundaries, and human checkpoints
- [x] A new `arenas/<id>/` passes oracle/null/probe rig checks and demonstrates
      agent spread before search claims are trusted
- [x] The run emits the standard Threshold records, lineage, report, and Pareto
      archive without PR-review-specific special cases
- [x] Any schema or runner assumptions discovered during the second family are
      either generalized behind file contracts or deliberately rejected in
      `DESIGN.md`
- [x] The result updates the reopen trigger for a Rust validation kernel with
      evidence from two task families
- [x] `bin/gate` green

## Children
1. [x] Pick the domain by evidence, with backlog grooming, inbox triage, browser QA,
   or launch-contract review as candidate families.
2. [x] Author the new G1 task spec and arena freeze gate.
3. [x] Run the one-shot probe and prepare/sign G2 before trusting search
   scores.
4. [x] Run the first certified comparative search and delivery export.
5. [x] Audit hard-coded PR-review assumptions in docs, runner output, and export
   contracts.

## Current Evidence

- Chosen second domain: `launch-contract-v0`, reviewing control-plane launch
  contracts and import packets for approval, evidence, permission,
  observability, and portability defects.
- Spec: `specs/launch-contract/taskspec.toml`
- Arena: `arenas/launch-contract-v0`
- G1 packet: `approvals/G1-launch-contract.md` (approved by human G1 reviewer
  on 2026-06-12 for offline synthetic experimentation only)
- No-spend rig run: `runs/20260612T000000Z-freeze-launch-contract-v0`
  records oracle 1.0 and null 0.1667, matching one clean task out of six.
- Corrected certified run: `runs/20260612T153450Z-search-launch-contract-v0`
  records oracle 1.0, null 0.1667, one-shot probe 0.5333, known spend
  `$0.4947`, and certified recommendation
  `g2b-g1a-seed2-glm-4-7-flash-spec-first` at reward 0.72.
- Superseded run: `runs/20260612T024051Z-search-launch-contract-v0` revealed
  that mutation allowed `thinking = "high"` outside the G1-approved
  `off`/`low`/`medium` space. The corrected run followed a code fix and
  focused tests for `search.thinking_levels` enforcement.
- Delivery export: `deliveries/launch-contract/` with unsigned
  Bitter Blossom and Olympus dry-run import packets. G3 remains unsigned, so
  the packets are sandbox-only, non-deployable, and not primary-reviewer
  capable.
- G2 packet: `approvals/G2-launch-contract-v0.md` (accepted by the operator on
  2026-06-12 for internal Threshold second-family benchmarking,
  contract-discovery, and sandbox-only delivery export).
- Rust validation-kernel trigger: now met by two accepted task families,
  `pr-review-v2` and `launch-contract-v0`; `DESIGN.md` and `ROADMAP.md` record
  that the validation-kernel decision is reopened for future implementation.

## Notes
**Why:** premise-challenge lane. The README describes Threshold as a general
agent-building lab, but all measured evidence currently comes from PR review.
The next ambitious proof is domain transfer.
