# Build the Cerberus substrate R&D lab

Priority: P0 · Status: ready · Estimate: L

## Goal

Make Daedalus useful for Cerberus by measuring autonomous code-review
substrates as first-class candidate configurations, starting with OpenCode and
OMP through Cerberus' `ReviewArtifact.v1` contract, before generalizing the lab
pattern to arbitrary tasks, projects, and contexts.

## Why Now

The substrate premise is now explicit:
`docs/premises/2026-06-19-coding-agent-substrates.md` argues that OpenCode is a
better default substrate than OMP for unsupervised review services because it is
server/session-first. Live Cerberus code has moved from architecture memory into
an actual Rust runner with `opencode`, `omp`, and fixture harnesses plus strict
artifact validation. Daedalus, however, still treats `pi` as the only V1
candidate harness and cannot yet compare Cerberus substrate runs.

This is the narrowest useful bridge between Daedalus and real applications:
prove one upstream R&D lab for Cerberus, then extract the lab template for
Allie-style evidence systems and other task domains.

## Scope

- Add or shape a Daedalus candidate/import path for Cerberus
  `ReviewArtifact.v1` outputs.
- Compare at least one OpenCode-backed Cerberus run, one OMP-backed Cerberus
  run, and the current Pi-style review baseline where comparable.
- Record substrate, model, context capability, artifact validity, lifecycle
  state, cost, latency, and failure mode in evidence Daedalus can rank.
- Keep Bitterblossom and Olympus as control-plane consumers/comparators; do not
  move production posting, trigger, budget, or approval authority into
  Daedalus.
- Resolve or explicitly sequence with `047` so real-repo arena validation is
  not based on an inconclusive one-shot saturation probe.

## Non-Goals

- No production PR posting from Daedalus or Cerberus in this ticket.
- No arbitrary lab framework before the Cerberus lab works.
- No hardcoded specialist roster as the product claim; reviewer topology is a
  measured variable or substrate-internal behavior.
- No self-approval of G1-G5.
- No weakening of freeze validation, holdout exposure accounting, or artifact
  validation to get a result.

## Oracle

- [ ] A Cerberus fixture `ReviewArtifact.v1` can be validated and converted or
      scored through a Daedalus run/eval path without live model spend.
- [ ] A live OpenCode-backed Cerberus review and an OMP-backed Cerberus review
      for the same request leave comparable Daedalus evidence: artifact,
      transcript/receipt, substrate, model, lifecycle state, cost or `null`,
      latency, and scoring output.
- [ ] The current Pi review baseline remains measurable or is explicitly
      marked incomparable with a documented reason.
- [ ] The comparison report answers: which substrate produced the best valid
      autonomous review artifact under the chosen objective, what it cost, what
      context it actually used, and what remains unproven.
- [ ] `docs/primitives.md`, `docs/review-autoresearch-loop.md`, and active
      Bitterblossom-facing tickets no longer imply Pi is the default substrate
      for unsupervised/reflex review labs.
- [ ] `arena-freeze` / `arena-validate` either produce non-inconclusive probe
      evidence for the chosen real-repo arena or the run is explicitly limited
      to fixture/adapter proof before paid search.
- [ ] `bin/gate` passes.

## Verification System

- Claim: Daedalus can compare Cerberus autonomous review substrates credibly
  enough to recommend what Cerberus should run next.
- Falsifier: Daedalus cannot score a valid Cerberus artifact, substrate errors
  are confused with model quality, OpenCode/OMP runs do not leave comparable
  evidence, or a saturated/inconclusive arena is used for a recommendation.
- Driver: Cerberus fixture run, live Cerberus OpenCode/OMP runs, Daedalus
  scoring/report generation, and the repo `bin/gate`.
- Grader: `ReviewArtifact.v1` validation, Daedalus score against hidden answer
  key or fixture oracle, report fields present, lifecycle/failure states
  preserved, and non-inconclusive freeze evidence before any real-repo
  recommendation.
- Evidence packet: Cerberus request/artifact/transcript, Daedalus run directory,
  summary/report, substrate comparison table, and critic receipt.
- Cadence: fixture before live spend; live run before recommendation; fresh
  critic before any G2/G3-facing handoff.

## Children

1. Land and reconcile the substrate premise on `master`; update stale Pi-first
   wording in `036` and `037` so they depend on this lab instead of forking it.
2. Add a fixture-only Cerberus artifact ingestion/scoring path in Daedalus.
3. Add live Cerberus OpenCode and OMP candidate execution or import receipts
   with honest cost/latency/failure fields.
4. Connect the comparison to the review arena without weakening `047`; if the
   real-repo freeze remains blocked, stop at adapter proof.
5. Run a first OpenCode-vs-OMP-vs-Pi review comparison and write a recommendation
   that stays sandbox-only until human gates.
6. Extract the general lab template only after Cerberus succeeds: task contract,
   candidate adapter, artifact schema, scorer, freeze, report, and handoff.

## Evidence

- Context packet: `docs/048-cerberus-rd-lab-shape.html`
- Premise source:
  `sha256:2c10aea3a38c845bfe492fa42aede414a049ec9b78c5007c5c66a5a1db6fbc05`
  `docs/premises/2026-06-19-coding-agent-substrates.md`
- Cerberus runner source:
  `sha256:40183960599f9b076c4fd453609a76b7b2ed917f156303f93e51497bfabd3555`
  `/Users/phaedrus/Development/cerberus/src/harness.rs`
- Allie generalization anchor:
  `sha256:447d2b863ca2fca4b7bc13c1fa67bc4c22fbe29559f5e36d1d8992bf80796cb8`
  `/Users/phaedrus/Development/allie/README.md`

## Notes

The strategic split is supervised versus unsupervised:

- supervised dispatch agents can remain terminal-first and operator-steered;
- unsupervised/reflex agents need strict artifacts, lifecycle states,
  env/secret control, budget visibility, and replayable receipts.

OpenCode is the first hypothesis for the unsupervised side, not an assumed
winner. The lab exists to prove or falsify it.
