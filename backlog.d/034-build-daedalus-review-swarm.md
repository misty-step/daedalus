# Build a Daedalus-optimized PR review swarm

Priority: P0
Status: ready
Estimate: XL

## PRD Summary

- User: the operator responsible for Olympus and Bitter Blossom review-agent
  quality.
- Problem: the current review agents are useful but plane-specific, manually
  shaped, and not selected by Daedalus against a benchmark that matches the
  desired swarm behavior.
- Goal: produce a Daedalus-backed review-swarm program that can discover,
  compare, and export specialist PR-review agents plus a master synthesis
  reviewer for sandbox-only Olympus and Bitter Blossom trials.
- Why now: Daedalus has two accepted task families, a control-plane export
  path, and live incumbent review-agent surfaces in both target planes.
- UX enabled: the operator can read one evidence packet and decide which
  specialist roles, model/harness configs, and synthesis contract deserve
  sandbox import before any primary-reviewer deployment.
- Deliverable type: harness primitive plus task specifications, arenas,
  certified run evidence, and sandbox-only control-plane handoff artifacts.
- Success signal: a G2 packet shows a certified vertical slice of the
  review-swarm suite with measured specialist spread inside the implemented
  lenses, master-reviewer consolidation quality, explicit total cost/latency
  ceilings, and dry-run import packets for both planes.

## Goal

Turn code review from one optimized reviewer into a measured portfolio of
specialist reviewers plus a master synthesis reviewer, with Daedalus evidence
strong enough for sandbox-only Olympus and Bitter Blossom trials.

## Product Requirements

- P0: Define a review-swarm suite as explicit Daedalus task specifications,
  not as control-plane prose or hidden prompt convention.
- P0: Include a general reviewer and specialist lenses for correctness,
  security, verification/CI, simplification/maintainability, and product
  intent.
- P0: Define the master reviewer as its own task spec: it consumes specialist
  artifacts and emits one consolidated review artifact with deduplication,
  severity calibration, false-positive suppression, and residual-coverage
  disclosure.
- P0: Define the master reviewer as a consolidation task, not a fresh defect
  discovery task. Novel defect discovery remains the member agents' job.
- P0: Keep all pre-G3 plane output sandbox-only, secondary, and non-primary;
  member agents write artifacts, while the target plane owns any posting.
- P0: Set an explicit suite cost envelope before search. A candidate that
  clears quality but exceeds the target-plane per-PR budget is not launchable.
- P0: Preserve Daedalus red lines: candidates never see `tests/` or
  `solution/`, saturated arenas abort, run records are evidence, and
  deployment remains G3/G4/G5 gated.
- P1: Search over model/config/tool policies immediately, and add harness as
  a measured slot only where Daedalus can record comparable run evidence.
- P1: Reuse Bitter Blossom's existing verdict-storm role boundaries where
  they survive arena design, rather than inventing new role names.
- P1: Preserve Olympus' stricter artifact-validation and orchestrator-posting
  boundary as the preferred production shape.
- Non-goals: public benchmark-quality claims, primary reviewer deployment,
  production write authority, production-data re-ingestion, or replacing
  Olympus/Bitter Blossom control-plane mechanics in this slice.

## Constraints

- "Optimal" means suite-conditional Pareto optimal under a named task family,
  objective mode, budget, and plane boundary. There is no universal optimal
  code-review agent config.
- The first implementation must make the suite oracle executable before
  spending search budget. If the master-reviewer oracle cannot be scored, the
  work stops at an arena/design packet.
- The first review-swarm G2 packet must not imply enterprise readiness if the
  total suite budget exceeds the intended plane ceiling. Target the first
  sandbox vertical slice at <= $2.00/PR and require a hard waiver before
  accepting any candidate above $3.00/PR. This is a launchability envelope,
  not a research stop: more expensive candidates may be measured and reported,
  but they are not sandbox-import recommendations without the waiver.
- The first review-swarm G2 packet must also report total wall time. Target
  the first sandbox vertical slice at <= 20 minutes end-to-end and require a
  hard waiver above 30 minutes. Do not assume parallel member execution until a
  plane proves it without the known `pi` concurrency deadlock.
- Subjective review quality cannot be trusted to an uncalibrated judge. Use
  deterministic planted-defect or curated-dispute keys first; judge scoring is
  diagnostic until the existing judge calibration gate passes.
- Sensitive, networked, secret-bearing, production-data, or adversarial
  arenas must run through Harbor/Docker or an equivalent isolated runner, not
  Daedalus' local low-risk runner.
- Olympus and Bitter Blossom imports remain dry-run/sandbox artifacts until
  human G3 approval. G4 is required before any production write authority.
  G5 is required before production traces feed back into fixtures.

## Repo Anchors

- `DESIGN.md` - file contracts, G1-G5 gates, launch-contract shape, and the
  reopened Rust validation-kernel trigger.
- `ROADMAP.md` - current evidence from `pr-review-v2`, `launch-contract-v0`,
  and the phase-3/phase-4 deploy-observe-reiterate direction.
- `.agents/skills/daedalus/SKILL.md` - Daedalus protocol: agent-vs-agent,
  search-space declaration, headroom, G2 before score trust, recommend never
  deploy.
- `specs/pr-review/taskspec.toml` - current general PR-review task spec and
  supported search-space shape.
- `deliveries/pr-review/plane-handoff.md` - current single-reviewer handoff,
  incumbent comparison, and pre-G3 sandbox boundary.
- `docs/operator-sop.md` - maintained G1/G2/search/export/launch-pack
  command sequence.
- `runner/export.py` - current single-agent contract/persona/handoff export
  boundary that a suite export must extend rather than bypass.
- `docs/premises/review-swarm-2026-06-12.md` - durable operator-premise
  artifact for this shape.
- `/Users/phaedrus/Development/bitterblossom/plane/tasks/*/card.md` - existing
  verdict-storm role cards for correctness, security, simplification,
  product, verifier, and arbiter.
- `/Users/phaedrus/Development/adminifi/olympus/orchestrator/agent-specs/charon.yaml`
  and `/Users/phaedrus/Development/adminifi/olympus/orchestrator/prompts/charon-review.md`
  - current Olympus Charon AgentSpec and strict JSON artifact contract.
- `/Users/phaedrus/Development/adminifi/olympus/orchestrator/src/charon-review-poster.ts`
  - control-plane-owned schema validation, diff-anchor checks, duplicate
  suppression, and posting.

## Alternatives

| Alternative | How it works | Failure mode | Verdict |
|---|---|---|---|
| Single general reviewer only | Keep optimizing `pr-review-v2` and export one Charon/review-coordinator replacement | A single aggregate score hides which failure class improved or regressed; expensive models may look best by averaging over role confusion | Reject as the end state; retain as the general-specialist baseline |
| Build the swarm directly in Olympus/Bitter Blossom | Add multiple plane agents and tune from live PR comments | Produces uncontrolled configs with no Daedalus composition hashes, no holdout discipline, and high operator trust risk | Reject for this phase |
| Daedalus specialist suite plus master synthesis | Treat each lens and the master reviewer as explicit task specs/arenas, search candidates, then export a suite contract | More upfront eval design, but the evidence maps cleanly into sandbox imports | Choose |
| Judge review quality directly | Let an LLM judge score holistic review usefulness | Existing judge-family work showed calibration can fail; uncalibrated judgment would launder taste as evidence | Reject as primary oracle; allow after calibration as secondary diagnostic |
| Use production traces immediately | Mine real Olympus/BB PR reviews into fixtures | Violates the current no-G5 boundary and risks benchmark leakage/privacy issues | Reject until G5 |

## Technical Design

### Chosen Architecture

Add a Daedalus "review swarm" program as a suite of related task specs and
arenas, then extend export to carry a multi-agent delivery:

1. `specs/pr-review-general/taskspec.toml` or an updated suite wrapper around
   the existing `specs/pr-review/taskspec.toml` keeps the general reviewer as
   the baseline lens.
2. `specs/pr-review-correctness/taskspec.toml` measures logic, invariants,
   error paths, data loss, and concurrency defects.
3. `specs/pr-review-security/taskspec.toml` measures secrets, authz,
   injection, unsafe input handling, dependency/supply-chain, and credential
   handling defects.
4. `specs/pr-review-verification/taskspec.toml` measures execution-aware
   review: whether an agent identifies the relevant gate/test/CI evidence and
   distinguishes failing verification from speculative review prose.
5. `specs/pr-review-simplification/taskspec.toml` measures gate weakening,
   needless abstraction, duplicate logic, dead code, and maintainability
   defects that can be deterministically keyed.
6. `specs/pr-review-product/taskspec.toml` measures ticket/intent mismatch,
   missing acceptance behavior, and unexpected scope creep using fixtures with
   explicit local backlog or issue context.
7. `specs/pr-review-master/taskspec.toml` measures synthesis: input is the PR
   context plus specialist JSON artifacts, including noisy false positives and
   duplicate findings; output is one consolidated strict review artifact.

The first deliverable is not all seven fully certified arenas. The first
deliverable is the suite harness and the first viable vertical slice:
general + correctness + security + master synthesis, with the remaining
lenses scaffolded and explicitly blocked on arena headroom evidence if needed.
That vertical slice proves the suite mechanics and the first two specialist
lenses; it does not claim that the full seven-role swarm is optimal.

Before any specialist arena fixture is authored, write
`docs/review-swarm-taxonomy.md` and a validator command. The charter assigns
canonical defect families to lenses, names allowed overlaps, defines severity
translation rules, and declares how ambiguous fixtures are adjudicated. Do not
reuse a `pr-review-v2` fixture for a specialist lens unless the defect can be
labeled unambiguously or the overlap is intentionally encoded in the expected
result.

### Data And Control Flow

1. The suite wrapper names member specs, member output contracts, allowed
   search slots, and the master synthesis input contract.
2. Each member agent runs independently against the same PR fixture split and
   writes a strict artifact. Member artifacts are never posted.
3. The master-reviewer task consumes member artifacts and produces a strict
   consolidated artifact.
4. The suite scorer evaluates both member quality and final review quality:
   true-defect recall, false-positive suppression, duplicate collapse,
   severity calibration, evidence quality, coverage disclosure, cost, and
   wall time. Defects carry stable hidden IDs and allowed lens ownership, so
   the report can show a per-lens confusion matrix instead of silently blaming
   the wrong specialist for an ambiguous finding.
5. Export emits `deliveries/pr-review-swarm/` with member contracts,
   master contract, suite manifest, composition hashes, trace pointers,
   plane-handoff notes, and dry-run import packets for Bitter Blossom and
   Olympus.

### Member Artifact Contract

Every member writes one strict JSON artifact before the master runs:

```json
{
  "member_id": "correctness",
  "lens": "correctness|security|verification|simplification|product|general",
  "status": "ok|error|timeout|truncated",
  "summary": "short coverage summary",
  "findings": [
    {
      "local_id": "member-local stable id",
      "path": "src/file.ext",
      "line": 42,
      "severity": "blocking|serious|minor",
      "category": "taxonomy category",
      "claim": "one sentence",
      "evidence": "quoted or command-backed evidence",
      "confidence": "high|medium|low"
    }
  ],
  "error": "message when status is not ok"
}
```

The first vertical slice marks `general`, `correctness`, and `security` as
required members. If a required member errors, times out, or emits malformed
JSON, the suite candidate is not recommendable. The master may still run for a
diagnostic artifact, but the G2 report must mark coverage incomplete. Optional
members may be skipped only when the suite taskspec marks them optional and the
master discloses the missing lens.

The general reviewer has two roles: single-agent baseline and broad recall
member. In member mode it is a normal input to the master, not a synthesis
authority. Specialist findings take precedence for their owned taxonomy
categories; general findings are deduped against specialist findings for the
same hidden defect ID and cannot downgrade a specialist's severity without
stronger evidence accepted by the master scorer.

### Master-Reviewer Oracle

The master reviewer is a reducer over member artifacts. Its first benchmark
does not measure whether the master can discover new defects from source code;
it measures whether the master can turn conflicting specialist outputs into
one safe review.

Master fixtures must therefore provide only realistic specialist artifacts:
file/line, severity, claim, evidence, member identity, and optional uncertainty
flags. They must not include labels like `true_positive`, `duplicate_of`, or
`correct_severity`. Those labels live only in hidden answer keys. The scorer
checks whether the master:

- keeps every hidden true finding at the correct or stricter acceptable
  severity;
- collapses duplicate reports for the same hidden defect ID;
- suppresses hidden false positives and known out-of-scope findings;
- preserves materially different defects even when they touch the same file;
- discloses missing member coverage or truncated exploration;
- emits one strict artifact within the configured finding and token budget.

If a later task wants the master to independently discover defects, that is a
different task family with source-code inputs and its own headroom proof.

Bootstrap path:

1. `pr-review-master-v0` starts with hand-authored member artifacts derived
   from hidden answer keys plus plausible false positives. These artifacts are
   not claimed to represent production specialist behavior; they prove the
   scorer and consolidation contract.
2. After the first correctness/security/general candidates are certified,
   replay the master benchmark with artifacts produced by those real member
   candidates on the same split.
3. G2 acceptance for the swarm must name both results. If the master only
   works on synthetic member artifacts and fails on real member outputs, the
   suite is not exportable; the report becomes arena-improvement evidence.

Fallback: if real-member replay fails, export may still emit a
`member-only` sandbox packet containing non-posting specialist artifacts and
the failing master evidence. That packet is not a review-swarm recommendation
and cannot post or synthesize a PR review; it exists only to let Olympus and
Bitter Blossom inspect member outputs while Daedalus iterates the master
arena. The next iteration should first normalize member artifacts to the
canonical schema, then rerun the master replay before any full-swarm handoff.

### Suite Objective

The first suite mode is `threshold-then-cheap`:

1. A suite candidate is recommendable only if the master keeps every hidden
   blocking/critical true finding, master true-finding recall is at or above
   the threshold declared in `specs/pr-review-suite/taskspec.toml`, hidden
   false-positive carry-through stays at or below the declared threshold,
   duplicate collapse meets the declared threshold, all artifacts are valid,
   and no required member exceeds its timeout/error cutoff.
2. Among recommendable suites, lower total measured cost wins. Wall time is a
   secondary tie-breaker unless the taskspec declares a stricter latency mode.
3. Candidates below quality threshold are reported on the Pareto frontier but
   are not recommendations, even if cheaper.
4. Candidates above the suite cost envelope require an explicit waiver before
   they can be exported as sandbox recommendations.
5. Candidates above the suite wall-time envelope require an explicit waiver
   before sandbox import, even when quality and cost clear threshold.

The thresholds live in the suite taskspec, not prose memory. The G2 packet must
print them next to the measured scores.

### Harness Slot

Do not declare "harness" searchable until Daedalus can measure it honestly.
Start with the supported `pi` runner and OpenRouter model/config slots. Add a
harness slot only when candidate manifests and run records can distinguish
runtime behavior, harness version, cost, timeout semantics, tool policy, and
artifact collection for at least two real harnesses.

The likely sequence:

1. `pi` over OpenRouter remains the first measured production-shaped harness
   because both Daedalus and Bitter Blossom already use it.
2. Olympus' Charon AgentSpec shape becomes an export target, not a Daedalus
   runner, until comparable run records exist.
3. Codex/Claude subscription-backed harnesses may be offline comparison arms
   only if their cost/auth/tool semantics are recorded honestly and the plane
   deployment boundary says whether they are admissible for recurring events.

### Plane Mapping

- Bitter Blossom: map members to event-plane tasks or a task group, preserve
  HMAC ingress, dedupe, repo/additions filters, per-run budgets, and no direct
  posting from member agents. The master/control plane owns the one review
  comment if G3 later allows posting.
- Olympus: map members and master to AgentSpecs or a composite Charon successor
  while preserving activation gates, pinned-head checkout, strict artifact
  validation, diff-anchor validation, duplicate suppression, superseded-head
  suppression, and orchestrator-side posting.

### ADR Decision

ADR required: introduce `swarm-contract.v1` and `bin/daedalus export-suite` as
explicit Daedalus primitives before export implementation. No ADR is required
for adding task specs or arenas that follow existing file contracts.

Escalation trigger: if the suite exporter needs plane-specific runtime logic
inside Daedalus instead of declarative import packets, stop and write an ADR
before implementation continues.

## Oracle

Commands that must all exit 0 for the shaped epic to close:

- `bin/gate` - existing Daedalus tests and compile checks stay green.
- `bin/daedalus doctor` - cold-start readiness passes; unsigned G3 warnings are
  acceptable but must be named.
- `bin/daedalus arena-validate arenas/pr-review-master-v0 --probe-run runs/<master-rig-run> --report runs/<master-rig-run>/freeze-report.md`
  - master synthesis has oracle/null/probe evidence and does not saturate.
- `bin/daedalus taxonomy-validate docs/review-swarm-taxonomy.md --suite specs/pr-review-suite/taskspec.toml`
  - the taxonomy charter covers every lens, category, allowed overlap,
  severity translation, and ambiguity rule referenced by the suite specs.
- `bin/daedalus run specs/pr-review-master/taskspec.toml --rng-seed <seed> --budget-usd <budget> --max-candidates <n> --trials 1 --certify-top 1 --certify-trials 5 --children-per-gen 2 --optimizer-model <model> --max-errors-per-candidate 1`
  - master-reviewer search emits committed run records, report, Pareto archive,
  lineage, trace, and certified recommendation.
- `test -f docs/adr-004-review-swarm-contract.md`
  - the suite contract/export boundary is decided before implementation lands.
- `bin/daedalus export-suite deliveries/pr-review-swarm --suite specs/pr-review-suite/taskspec.toml`
  - emits a suite manifest plus member/master contracts pinned to run
  evidence.
- `bin/daedalus launch-pack deliveries/pr-review-swarm --plane bitter-blossom --dry-run`
  and `bin/daedalus launch-pack deliveries/pr-review-swarm --plane olympus --dry-run`
  - dry-run packets are sandbox-only, non-primary, and blocked on G3.
- `jq -e '.suite.total_cost_usd <= 2.0 or .waivers.cost_ceiling == true' deliveries/pr-review-swarm/summary.json`
  - the sandbox candidate has an explicit total-cost story, with waiver if it
  exceeds the target envelope.
- `jq -e '.suite.total_wall_sec <= 1200 or .waivers.wall_time == true' deliveries/pr-review-swarm/summary.json`
  - the sandbox candidate has an explicit latency story, with waiver if it
  exceeds the target envelope.
- `jq -e '.master.real_member_replay.passed == true or .handoff.mode == "member-only"' deliveries/pr-review-swarm/summary.json`
  - full-swarm export requires master transfer to real member outputs; otherwise
  only member-only inspection output may be emitted.

Observable acceptance:

- A human G2 reviewer can tell which specialist lenses are measured, which are
  scaffolded only, why the master-reviewer recommendation won, and why the
  packet is still not deployment approval.
- The plane handoff packet names the current Bitter Blossom and Olympus
  incumbent boundaries and describes the import delta without weakening them.

## Children

1. [ ] Write `docs/review-swarm-taxonomy.md` and
   `bin/daedalus taxonomy-validate`: defect IDs, allowed lens ownership,
   overlap rules, severity rules, and ambiguity/adjudication workflow.
2. [ ] Write `docs/adr-004-review-swarm-contract.md` selecting
   `swarm-contract.v1` and `bin/daedalus export-suite`.
3. [ ] Define the member artifact JSON schema, required/optional member
   failure policy, and general-reviewer precedence rules.
4. [ ] Define `pr-review-suite` and `pr-review-master` task specs, including
   strict member-artifact and master-reviewer output contracts and the
   `threshold-then-cheap` suite objective.
5. [ ] Build the first master-synthesis arena from bootstrapped specialist artifact
   fixtures: true findings, duplicate findings, noisy false positives,
   conflicting severities, and clean/no-finding cases, with labels hidden from
   candidate inputs.
6. [ ] Build or adapt specialist arenas for correctness and security using
   real-repo-scale fixtures with one-shot headroom; keep general review as the
   baseline from `pr-review-v2`.
7. [ ] Add the suite run/export verification harness: member contracts,
   master contract, composition hashes, evidence pointers, and dry-run
   import packets.
8. [ ] Run the first certified suite search on the vertical slice
   general + correctness + security + master synthesis.
9. [ ] Replay the master-synthesis benchmark with artifacts emitted by the
   certified member candidates; block export if synthetic-artifact performance
   does not transfer, except for explicitly member-only inspection handoff.
10. [ ] Refresh Olympus and Bitter Blossom incumbent reads from live files and
   generate sandbox-only handoffs that preserve each plane's posting and
   validation boundary.
11. [ ] Decide whether harness becomes a Daedalus search slot now or remains a
   follow-up until comparable non-`pi` runner evidence exists.

## Lead Repo Read

- `git status --short --branch --untracked-files=all` in Daedalus showed a
  clean `master...origin/master` before shaping.
- `specs/pr-review/taskspec.toml` confirms the current review search space is
  model/thinking/tool-policy/packet focused under `pi`.
- `deliveries/pr-review/plane-handoff.md` records Charon and Bitter Blossom
  incumbent boundaries and the pre-G3 sandbox rule.
- Bitter Blossom has existing specialist verdict cards for correctness,
  security, simplification, product, verifier, and arbiter.
- Olympus Charon already uses a strict JSON artifact and control-plane-owned
  posting, with tests for invalid artifacts, caps, duplicate suppression, and
  superseded-head suppression.

## Alignment Questions

None; assumptions accepted for the first slice.

Locked decisions:

- "Optimal" means suite-conditional Pareto by role and by master synthesis
  quality, not one universal reviewer.
- The first vertical slice is general + correctness + security + master
  synthesis. It proves the suite mechanics and first specialist spread, not the
  full seven-role swarm.
- Harness search is deferred until runner evidence can compare harnesses
  honestly.
- Members write artifacts only. The master/control plane owns posting after
  validation.

## Premise Source

Premise Source: sha256:3bb9c4c6dd660c194edff3e8ca1cce709d897412abafc368bf9bdcbf19187c63 docs/premises/review-swarm-2026-06-12.md

## Risks + Rollout

- Risk: specialist arenas saturate or fail to show agent spread. Rollback:
  stop at G2, keep the arena as a design artifact, and do not run search.
- Risk: master synthesis rewards verbosity or consensus instead of true
  review quality. Rollback: strengthen false-positive and duplicate fixtures;
  keep judge scoring diagnostic only.
- Risk: suite export becomes a plane-specific semantic engine. Rollback:
  revert to declarative dry-run packets and write an ADR before adding runtime
  logic.
- Risk: non-`pi` harness comparisons are not apples-to-apples. Rollback:
  freeze harness to `pi` and leave harness search as a follow-up ticket.
- Risk: plane sandbox import is mistaken for deployment approval. Rollback:
  keep `g3_signed = false`, emit non-primary dry-run packets only, and preserve
  G4/G5 blocks in every handoff.

## Notes

**Why:** product + eval-design lens. The interesting product is not "a better
review prompt"; it is a measured review organization where specialists create
coverage and the master reviewer creates trust, restraint, and one coherent
operator-facing review.
