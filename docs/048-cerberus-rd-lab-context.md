# Context Packet: Cerberus R&D Lab Operating Model

## PRD Summary

- User: the Daedalus operator deciding what Cerberus should run next.
- Problem: Cerberus now has a real Rust review runner, OpenCode default,
  OMP fallback, fixture verification, and PR/git-range request builders, but
  Daedalus still mainly measures Pi compositions and cannot compare Cerberus
  review substrates as first-class experimental candidates.
- Goal: make Daedalus able to compare Cerberus autonomous review
  configurations by validated artifacts, cost, latency, context truth, and
  failure mode, then recommend a sandbox-only Cerberus configuration.
- Why now: Cerberus `master` now includes the PR request-builder surface
  (`99c4b21 Add Cerberus PR request builder (#463)`), so Daedalus can start
  from real `ReviewRequest.v1` and `ReviewArtifact.v1` boundaries instead of
  shaping around hypothetical acquisition.
- UX enabled: the operator can inspect a lab workbench, choose controlled
  substrate/model/context experiments, read comparable evidence, and decide
  whether a Cerberus config is ready for sandbox import or still blocked.
- Deliverable type: harness primitive plus research report, with a shaped
  operator workbench model and no production posting authority.
- Success signal: one Cerberus fixture artifact and one live OpenCode-vs-OMP
  comparison enter a Daedalus run/report shape without weakening freeze,
  scorer, gate, or approval contracts.

## Product Requirements

- P0: score or import a validated Cerberus `ReviewArtifact.v1` against a
  Daedalus PR-review arena without live model spend.
- P0: compare at least OpenCode and OMP Cerberus runs for the same request,
  preserving substrate, model, context tier, lifecycle state, latency, cost or
  `null`, transcript/receipt path, and artifact validity.
- P0: keep Daedalus as the lab and Cerberus as the review runner; Daedalus
  does not post PR comments, mutate Cerberus defaults, self-approve G1-G5, or
  own runtime eventing.
- P0: stop before any real-repo recommendation if freeze validation remains
  inconclusive because of the `047` saturation-probe blocker.
- P1: compare a current Pi-style reviewer baseline where comparable; otherwise
  mark it incomparable with a reason.
- P1: update stale Pi-first language in active review-lab tickets after the
  Cerberus-first lab vertical slice proves the boundary.
- Non-goals: no arbitrary lab framework, no dashboard product, no production
  Cerberus deployment, no static predefined reviewer roster, no Bitterblossom
  webhook/event loop, no model-pool changes from memory.

## Constraints

- The grader is gospel. Any scorer, answer-key, or scorer-constant change
  needs an arena version bump and fresh oracle/null/probe baselines.
- Candidates must never see `tests/` or `solution/` in any arena.
- `runs/*.jsonl` is append-only evidence; unknown cost is `null`.
- `docs/primitives.md` is the model-pool source of truth. Refresh live before
  authoring any new paid search space.
- Cerberus owns `ReviewRequest.v1 -> ReviewArtifact.v1`, artifact validation,
  context-capability truth, and harness receipts.
- Daedalus owns arenas, frozen evals, candidate/run records, analysis,
  Pareto/reporting, and sandbox-only promotion logic.
- Bitterblossom and Olympus remain control-plane consumers/comparators.

## Lead Repo Read

- `AGENTS.md`: gate, grader, model-pool, candidate-hidden-path, run-record,
  approval, and backlog contracts.
- `DESIGN.md`: Daedalus pipeline, candidate manifest, run records, launch
  contract, and Cerberus handoff boundary.
- `docs/premises/2026-06-19-coding-agent-substrates.md`: OpenCode-first
  substrate premise and Cerberus artifact evaluation axis.
- `docs/primitives.md`: current Pi-only V1 harness slot and verified model
  pool discipline.
- `docs/arena-workbench.md`: scaffold, freeze, validate, adjudicate, and
  disagreement commands.
- `docs/review-autoresearch-loop.md`: controlled hypothesis loop and plateau
  postmortem discipline.
- `docs/daedalus-ui-lab/round-2/console.html`: existing instrument-panel UI
  direction for candidate comparison, headroom, gates, cost, and task grids.
- `backlog.d/047-replace-real-repo-saturation-probe.md`: current freeze
  blocker for real-repo arenas.
- `/Users/phaedrus/Development/cerberus/spec.md`: Cerberus master reviewer,
  context tier, substrate, contract, harness, and verification rules.
- `/Users/phaedrus/Development/cerberus/docs/adr/0002-opencode-as-default-review-substrate.md`:
  OpenCode default and OMP fallback decision.
- `/Users/phaedrus/Development/cerberus/docs/plans/pr-review-tracer-bullet.md`:
  merged PR/git-range request-builder tracer bullet.

## Delete-First Check

- Requirement questioned: does Daedalus need a general lab product now, or only
  a Cerberus-first proof that the lab boundary works?
- Deleted or simplified: defer generic lab framework, hosted UI, posting,
  event loops, new model roster, and arbitrary adapters.
- Only then optimized/automated because: one Cerberus vertical slice is the
  smallest repeated, verified work that can falsify or justify generalizing.

## Alternatives

| Option | Why It Helps | Failure Mode | Verdict |
|---|---|---|---|
| Manual report over Cerberus artifacts | Fastest and code-light | Produces prose without Daedalus run records, no repeatable scorer path, no promotion-quality evidence | Reject as final shape; allow only as a scouting note |
| Continue Pi-only Daedalus review search | Uses current runner | Optimizes the old harness axis and does not answer whether Cerberus should run OpenCode or OMP | Reject as strategic default |
| Switch Cerberus to OpenCode globally | Matches the premise | Replaces one unmeasured default with another; no Daedalus comparison or rollback evidence | Reject |
| Generic external-agent lab framework | Long-term attractive | Broad abstraction before one consumer succeeds; likely shallow seams and unearned CLI surface | Defer |
| Cerberus-first lab adapter and workbench | Uses live Cerberus contracts, keeps Daedalus as foundry, produces comparable evidence | Requires careful artifact mapping and may expose arena-freeze blockers | Choose |

## Design

### Product Shape

The lab is an operator workbench, not a marketing dashboard and not a
production control plane. It should feel like the existing Daedalus UI lab's
instrument direction: dense, quiet, ruled, evidence-first, with one loud
decision surface at a time.

The first-viewport lab contract has five surfaces:

1. Task contract: arena, objective, context tier, split, gate state, and stop
   conditions.
2. Candidate rack: substrate, model, prompt packet, context capability, tool
   policy, budget, timeout, and run source.
3. Experiment console: controlled hypothesis, seeds/trials, baseline/probe
   state, and the exact commands that will run.
4. Evidence notebook: artifacts, transcripts, receipts, cost, latency,
   lifecycle, invalid-output reasons, and overclaimed-context flags.
5. Promotion gate: sandbox-only recommendation, G2/G3/G4/G5 state, rollback,
   and explicit residual risks.

### Levers and Dials

The exposed dials must map to a real Cerberus or Daedalus decision:

| Dial Group | Dials | Owner | Notes |
|---|---|---|---|
| Task | arena id/version, task family, split, clean-trap mix, context tier | Daedalus | Changing fixtures/keys/scorer constants bumps arena version |
| Candidate | substrate (`opencode`, `omp`, `pi`), model, prompt packet, system prompt stance, dynamic-lane permission | Daedalus measures, Cerberus runs | Model ids come only from refreshed primitives |
| Context | `diff_only`, `repo_head`, `repo_base_and_head`, local runtime, external research policy | Cerberus records, Daedalus scores | Overstated context is a scored failure |
| Safety | env allowlist, network/secrets policy, timeout, sandbox profile, degraded-run permission | Cerberus harness, Daedalus risk metadata | Sensitive or adversarial runs use Harbor/Docker or stop |
| Analysis | reward metric, artifact-validity gate, false-positive penalty, cost objective, latency objective, clustered CI, holdout burn | Daedalus | Do not average across arena versions |
| Promotion | sandbox-only export, threshold, waiver, rollback, G2-G5 gate state | Daedalus emits, human approves | No self-approval |

### Minimum Technical Surface

The first implementation should be a narrow Cerberus lab adapter, not a
general external-agent framework. It can be implemented as a new focused
Daedalus path that:

- reads a Cerberus `ReviewRequest.v1` and `ReviewArtifact.v1`;
- validates the artifact through Cerberus or a schema-compatible parser before
  scoring;
- maps Cerberus findings/comments into the existing Daedalus PR-review finding
  shape where possible;
- records substrate, model, context capabilities, lifecycle state, latency,
  cost, receipt path, and artifact validity in run evidence;
- emits a comparison report that can sit beside existing `summary.json`,
  `pareto.json`, and `report.md` style outputs;
- refuses to recommend if the arena freeze is inconclusive, artifact
  validation fails, or substrate failure is confused with model quality.

The adapter may later become a generic external-run importer only after the
Cerberus path proves useful.

## CLI Surface

- Command tree: `daedalus cerberus-lab import`
- Purpose: import and compare Cerberus review artifacts under a Daedalus arena.
- Primary user: human operator and scripts.
- Inputs: arena path, Cerberus request JSON, Cerberus artifact JSON,
  optional transcript/receipt path, candidate id, substrate/model metadata,
  output run dir.
- Outputs: Daedalus evidence directory with score/report JSON, Markdown
  comparison report, and artifact index; diagnostics to stderr.
- Interactivity: no prompts; all command paths must be scriptable.
- Safety: no production posting, no Cerberus default mutation, no model spend
  in fixture import mode, explicit flags for live Cerberus runs.
- Config precedence: flags > taskspec/search config > repo defaults.
- Platform/runtime: local macOS/Linux CLI; sensitive runs route to
  Harbor/Docker before live model spend.

Happy-path sketch:

```sh
cargo run --quiet --bin daedalus -- doctor
cargo run --quiet --bin daedalus -- arena-validate arenas/pr-review-correctness-v0 \
  --probe-run runs/<freeze-dir> --report runs/<freeze-dir>/freeze-report.md
cargo run --quiet --bin daedalus -- cerberus-lab import \
  --arena arenas/pr-review-correctness-v0 \
  --request /Users/phaedrus/Development/cerberus/target/cerberus/request.json \
  --artifact /Users/phaedrus/Development/cerberus/target/cerberus/artifact.json \
  --candidate-id opencode-self-review \
  --substrate opencode \
  --out-dir runs/cerberus-rd-lab-<stamp>
```

Failure examples:

- invalid Cerberus artifact exits nonzero and writes no recommendation;
- unknown model id exits nonzero until `docs/primitives.md` is refreshed;
- inconclusive freeze exits nonzero for recommendation mode and suggests
  limiting the run to fixture/adapter proof.

## Oracle

Commands that must exit 0 after implementation:

```sh
# Daedalus structural gate.
bin/gate

# Daedalus readiness stays current and model-roster checks do not regress.
cargo run --quiet --bin daedalus -- doctor

# Freeze/validate the target arena before paid search or recommendation.
cargo run --quiet --bin daedalus -- arena-freeze arenas/pr-review-correctness-v0 \
  --out-dir runs/cerberus-lab-freeze-smoke \
  --report runs/cerberus-lab-freeze-smoke/freeze-report.md
cargo run --quiet --bin daedalus -- arena-validate arenas/pr-review-correctness-v0 \
  --probe-run runs/cerberus-lab-freeze-smoke \
  --report runs/cerberus-lab-freeze-smoke/validate-report.md

# Cerberus still proves its request/artifact path.
cd /Users/phaedrus/Development/cerberus && ./scripts/verify.sh

# The new Daedalus Cerberus lab path imports at least one fixture artifact and
# writes an inspectable report without live model spend.
cargo run --quiet --bin daedalus -- cerberus-lab import \
  --arena arenas/pr-review-correctness-v0 \
  --request /Users/phaedrus/Development/cerberus/target/cerberus/git-range-request.json \
  --artifact /Users/phaedrus/Development/cerberus/target/cerberus/git-range-artifact.json \
  --candidate-id fixture-self-review \
  --substrate fixture \
  --out-dir runs/cerberus-rd-lab-fixture
test -s runs/cerberus-rd-lab-fixture/report.md
```

Observable outcomes:

- The report answers which substrate produced the best valid review artifact,
  what it cost, what context it actually used, and what remains unproven.
- Invalid or overclaimed Cerberus artifacts are rejected or scored as failures
  rather than silently normalized.
- A fresh critic can review the diff and packet without chat context.

## Verification System

- Claim: Daedalus can compare Cerberus autonomous review substrates credibly
  enough to recommend what Cerberus should run next in sandbox mode.
- Falsifier: Daedalus accepts invalid Cerberus artifacts, confuses substrate
  execution failure with model quality, hides unknown cost, recommends from an
  inconclusive/saturated arena, or emits a production-looking handoff before
  human gates.
- Driver: Cerberus fixture run, Cerberus OpenCode/OMP runs or imported
  receipts, Daedalus import/scoring/report generation, `arena-freeze`,
  `arena-validate`, and `bin/gate`.
- Grader: `ReviewArtifact.v1` validation, hidden-key score where mappable,
  artifact-validity status, context-overclaim checks, cost/latency/lifecycle
  fields, clustered CI or explicit low-sample waiver, and G2/G3/G4/G5 state.
- Evidence packet: Cerberus request/artifact/transcript, Daedalus run
  directory, `report.md`, substrate comparison table, freeze/validate reports,
  and critic receipt.
- Cadence: fixture import before live spend; freeze before recommendation;
  fresh critic before any G2/G3-facing handoff; postmortem after two
  non-improving iterations.
- Gaps / waiver: no production posting, no generic external-run framework, and
  no real-repo recommendation while `047` remains unresolved.

## Premise Source

- `sha256:2c10aea3a38c845bfe492fa42aede414a049ec9b78c5007c5c66a5a1db6fbc05 docs/premises/2026-06-19-coding-agent-substrates.md`
- `sha256:dd04900704a3bdf1d0d6d50d333b2131e2a56ae4b62ebe7cc373f6b921a01d75 /Users/phaedrus/Development/cerberus/spec.md`
- `sha256:a037ad81a79f6d9040c9cbdf4459c452d2f4b70334aa3933914c58d228a7f285 /Users/phaedrus/Development/cerberus/docs/adr/0002-opencode-as-default-review-substrate.md`
- `sha256:4e3ba2a25ff3963dd4f3e325e64d1dd7414706b92024ddcb0c6fd9befa01b2cc /Users/phaedrus/Development/cerberus/docs/plans/pr-review-tracer-bullet.md`

## HTML Plan

`docs/048-cerberus-rd-lab-shape.html`

## Risks + Rollout

- Risk: the adapter becomes a generic lab framework. Mitigation: Cerberus-only
  first, with a later extraction criterion based on a second consumer.
- Risk: report UI implies confidence the arena cannot support. Mitigation:
  surface freeze status, probe verdict, sample size, CI, and gaps in the first
  viewport.
- Risk: OpenCode wins by substrate affordance rather than reviewer quality.
  Mitigation: record lifecycle, context, latency, tool/session receipts, and
  failure mode separately from reward.
- Risk: Pi baseline is incomparable. Mitigation: mark incomparable explicitly
  rather than coercing it into the same score table.
- Rollout: land the context packet, implement fixture importer, add live
  Cerberus OpenCode/OMP receipts, then consider general lab extraction only
  after the Cerberus recommendation report is useful.

## Critic Prompt

Read only this packet, the linked HTML plan, and the implementation diff.
Return `BLOCKING: yes` or `BLOCKING: no`. Focus on failures that would make a
Daedalus recommendation embarrassing in production: invalid artifact acceptance,
wrong ownership boundary, overbroad lab framework, inconclusive arena treated as
quality evidence, hidden cost/latency/failure states, or unclear operator
workflow. Ignore naming and style nits.
