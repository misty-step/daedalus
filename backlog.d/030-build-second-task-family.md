# Build a second task family to pressure-test Daedalus

Priority: P1
Status: pending
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
1. Pick the domain by evidence, with backlog grooming, inbox triage, browser QA,
   or launch-contract review as candidate families.
2. Author the new G1 task spec and arena freeze gate.
3. Run the first certified comparative search and delivery export.
4. Audit hard-coded PR-review assumptions in docs, runner output, and export
   contracts.

## Notes
**Why:** premise-challenge lane. The README describes Daedalus as a general
agent-building lab, but all measured evidence currently comes from PR review.
The next ambitious proof is domain transfer.
