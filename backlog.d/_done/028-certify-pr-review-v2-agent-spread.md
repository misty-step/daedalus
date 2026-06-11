# Certify pr-review-v2 and produce cross-plane handoff packets

Priority: P0
Status: done
Estimate: L

## Goal
Turn `arenas/pr-review-v2` v0.2.0 from a promising scaled arena into a
publishable benchmark with demonstrated new-task agent spread, calibrated
keys, signed G2 evidence, a delivery-quality comparative run, and
plane-handoff artifacts for the current Olympus Charon and Bitter Blossom
review-coordinator surfaces.

## Non-Goals
- Deploying the PR-review agent (that is G3+ launch work)
- Changing scorer constants to make provisional tasks pass

## Oracle
- [x] A fresh `bin/daedalus run specs/pr-review/taskspec.toml` against
      `arenas/pr-review-v2` v0.2.0 commits `report.md`, `pareto.json`,
      `loop.json`, `lineage.md`, `trials.jsonl`, `summary.json`, and
      `artifacts.index`
- [x] Every train+validation task has n >= 5 trials for every recommended
      candidate; holdout candidates run at certification depth and are recorded
      in `holdout-ledger.md`
- [x] The six new v0.2.0 tasks show measurable agent spread, or failures are
      promoted into an arena-iteration note before any cross-agent claim
- [x] Category/span calibration findings such as `py-markup-escape` are
      adjudicated without weakening the grader; any key change bumps the arena
      version and reruns oracle/null/probe baselines
- [x] The certified run compares the recommended candidate against the current
      Bitter Blossom `review-coordinator` and Olympus `charon` incumbents, or
      explicitly records why exact replay is impossible
- [x] Delivery includes a `plane-handoff.md` packet or `plane-handoff/`
      directory mapping composition hash, prompt packet, model, tools, budgets,
      output contract, observability, approval state, and residual risks to both
      Bitter Blossom review task/agent config and Olympus AgentSpec/activation
      surfaces
- [x] The handoff distinguishes lab evidence from launch approval: 028 may
      recommend import shapes, but G3/G4/G5 approval and control-plane
      deployment remain in 029
- [x] `approvals/G2-pr-review-v2.md` exists with the freeze gate, run-record
      paths, residual risks, and human review state
- [x] `bin/gate` green

## Children
1. [x] Run the full v0.2.0 search sequentially with a recorded RNG seed and
   certification depth.
2. [x] Convert any `arena-findings.md` alarms into either a v0.2.1 calibration
   patch or a written waiver.
3. [x] Add the v2 G2 approval artifact and link it from `ROADMAP.md`.
4. [x] Regenerate the PR-review delivery only from certified evidence.
5. [x] Write the cross-plane handoff packet by comparing the current Bitter Blossom
   and Olympus review agents, then mapping the certified winner into their
   import shapes without mutating either control-plane repo.

## Evidence

- Certification run: `runs/20260611T173632Z-search-pr-review-v0`
- Recommended certified candidate:
  `seed4-qwen3-7-plus-checklist` (`4a73f1fd213aa1a5`)
- Delivery packet: `deliveries/pr-review/`
- Cross-plane handoff: `deliveries/pr-review/plane-handoff.md`
- Incumbent comparison source: `deliveries/pr-review/plane-incumbents.toml`
- G2 packet accepted with sandbox-only waivers:
  `approvals/G2-pr-review-v2.md`
- Gate: `bin/gate` passed on 2026-06-11 after packet generation

## Closure Decision

Human G2 accepted v0.2.0 on 2026-06-11 for internal Daedalus contract
discovery and plane-handoff work only. The waiver explicitly does not make
public benchmark-quality claims and does not authorize Bitter Blossom to run
this packet as a primary reviewer; any pre-G3 use must be sandboxed,
experimental, and secondary.

## Notes
**Why:** product/eval lane. `ROADMAP.md` names the next full run, but
`arenas/pr-review-v2/provenance.md` still marks six new tasks provisional
until spread is established. This is the immediate long pole.

**Current plane read (2026-06-11):** Bitter Blossom runs
`review-coordinator` v2 as a pi/OpenRouter Kimi K2.6 review task and the agent
posts directly through `gh`; Olympus runs `charon` v2 as a pi/OpenRouter Kimi
AgentSpec with activation gating, strict JSON output, and orchestrator-side
posting. Neither current review agent was configured from Daedalus evals.
