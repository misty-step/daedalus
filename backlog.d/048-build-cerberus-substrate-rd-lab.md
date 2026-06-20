# Build the Cerberus substrate R&D lab

Priority: P0 · Status: in progress - fixture import/comparison slice verified; live model-backed comparison remains · Estimate: L

## Shaped Context

- Context packet: `docs/048-cerberus-rd-lab-context.md`
- HTML plan: `docs/048-cerberus-rd-lab-shape.html`
- Strategic direction: Cerberus now supersedes the Daedalus review-swarm
  product lane for autonomous/reflex code review. The old 034 swarm work is
  reusable evaluation evidence, not the parent path.
- Deliverable type: harness primitive plus research report. The first
  implementation proves a Cerberus-first lab vertical slice before any generic
  external-agent lab framework.

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
artifact validation. Cerberus `master` now also includes PR/git-range request
builders (`99c4b21 Add Cerberus PR request builder (#463)`), so Daedalus can
start from real `ReviewRequest.v1` and `ReviewArtifact.v1` fixtures. Daedalus,
however, still treats `pi` as the only V1 candidate harness and cannot yet
compare Cerberus substrate runs.

This is the narrowest useful bridge between Daedalus and real applications:
prove one upstream R&D lab for Cerberus, then extract the lab template for
Allie-style evidence systems and other task domains.

## Scope

- Add or shape a Daedalus candidate/import path for Cerberus
  `ReviewArtifact.v1` outputs.
- Shape the operator-facing lab surface: task contract, candidate rack,
  experiment console, evidence notebook, and promotion gate.
- Compare at least one OpenCode-backed Cerberus run, one OMP-backed Cerberus
  run, and the current Pi-style review baseline where comparable.
- Record substrate, model, context capability, artifact validity, lifecycle
  state, cost, latency, and failure mode in evidence Daedalus can rank.
- Keep Bitterblossom and Olympus as control-plane consumers/comparators; do not
  move production posting, trigger, budget, or approval authority into
  Daedalus, and do not resurrect 034's Pi-first specialist swarm as the parent
  product path.
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

- [x] The context packet and HTML plan name the lab surfaces, dials, ownership
      boundaries, executable oracle, stop conditions, and premise sources
      without relying on chat context.
- [x] A Cerberus fixture `ReviewArtifact.v1` can be validated and converted or
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
      or that `034` is the parent product path for unsupervised/reflex review
      labs.
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
   and `034`-parent wording in `036` and `037` so they depend on this lab
   instead of forking it.
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

- Context packet: `docs/048-cerberus-rd-lab-context.md`
- HTML plan: `docs/048-cerberus-rd-lab-shape.html`
- Fixture import command:
  `daedalus cerberus-lab import --arena arenas/cerberus-fixture-v0 --request /Users/phaedrus/Development/cerberus/fixtures/requests/diff-only.json --artifact /Users/phaedrus/Development/cerberus/target/cerberus/artifact.json --candidate-id fixture-self-review --substrate fixture --task-id ratio-zero --out-dir runs/cerberus-rd-lab-fixture`
- Fixture comparison command:
  `daedalus cerberus-lab compare --run-dir runs/cerberus-rd-lab-fixture --run-dir runs/cerberus-rd-lab-opencode --run-dir runs/cerberus-rd-lab-omp --out-dir runs/cerberus-rd-lab-comparison`
- Fixture evidence:
  `runs/cerberus-rd-lab-fixture/`, `runs/cerberus-rd-lab-opencode/`,
  `runs/cerberus-rd-lab-omp/`, and
  `runs/cerberus-rd-lab-comparison/report.md`
- Premise source:
  `sha256:2c10aea3a38c845bfe492fa42aede414a049ec9b78c5007c5c66a5a1db6fbc05`
  `docs/premises/2026-06-19-coding-agent-substrates.md`
- Cerberus runner source:
  `sha256:05d51736e468c41bee0ebe6d6cabccca72e7f7cc52348e07e5202d12cb219449`
  `/Users/phaedrus/Development/cerberus/src/harness.rs`
- Cerberus request-builder source:
  `sha256:d2c75d882aab7917112c8d2431656308275eef8674303c87059e96151abf8648`
  `/Users/phaedrus/Development/cerberus/src/request.rs`
- Allie generalization anchor:
  `sha256:447d2b863ca2fca4b7bc13c1fa67bc4c22fbe29559f5e36d1d8992bf80796cb8`
  `/Users/phaedrus/Development/allie/README.md`

## Notes

2026-06-20 delivery slice: Daedalus now has `cerberus-lab import` and
`cerberus-lab compare`, plus `arenas/cerberus-fixture-v0`, to validate
Cerberus `ReviewArtifact.v1`, map findings into Daedalus scoring, preserve
receipt/transcript provenance, and compare fixture-backed substrate artifacts
without live spend. The checked evidence is adapter proof only: the OpenCode
and OMP artifacts imported here come from Cerberus' fake-harness verification
receipts, not live model-backed autonomous runs. Child 3, Pi comparability, and
047-gated real-repo freeze/probe evidence remain open before any substrate
recommendation.

The strategic split is supervised versus unsupervised:

- supervised dispatch agents can remain terminal-first and operator-steered;
- unsupervised/reflex agents need strict artifacts, lifecycle states,
  env/secret control, budget visibility, and replayable receipts.

OpenCode is the first hypothesis for the unsupervised side, not an assumed
winner. The lab exists to prove or falsify it.
