# Daedalus Roadmap

Phases are gated by evidence, not dates. Each phase has an exit criterion a
human can check. Work items live in `backlog.d/`.

## MVP milestone (current target)

MVP = an operator can actually use Daedalus, not just demo it:
`daedalus run <taskspec>` autonomously searches compositions (typed, hashed,
single-slot mutations) against an isolated arena whose tasks cannot be
one-shotted, under fixture splits that prevent benchmark overfitting, and
emits a comparison report + Pareto archive the operator can act on.

MVP = backlog 001 002 003 005 006 007 008 (core) + 004 009 (isolation +
non-one-shot arena) with 010 (judge family) wanted but droppable. 011–015 are
post-MVP unless evidence promotes them.

Status (2026-06-09): **MVP reached.** Core machinery 001–009 built, gate-green
(60 tests), and live-validated end-to-end:

- `bin/daedalus run` completes the full pipeline unattended (rig → baselines →
  reflective single-slot search → holdout/skip → report.md + pareto.json +
  loop.json), demonstrated proposing two prompt-packet mutations with
  hypotheses, running them as hashed child candidates, and plateau-stopping.
- Harbor Docker isolation proven: oracle 1.000, pi 0.300 single-task, and a
  full 6-task × 5-attempt pi run at mean 0.663.
- First lab finding: on the (saturated) pr-review-v1 arena the cheap one-shot
  ties the agentic harness at 1.7× lower cost, so threshold-then-cheap mode
  correctly recommends the baseline.

Live runs surfaced and fixed four real bugs (rig discrimination threshold,
empty-holdout crash, optimizer transient-retry, reasoning-model token
headroom) — each now gate-protected.

009 finding retained: synthetic cross-file defects do NOT defeat the one-shot
baseline (it inlines every file), so a genuinely non-one-shot arena needs a
large repo or execution (backlog 015). The freeze gate correctly kept
pr-review-v1 unfrozen.

Remaining backlog (post-MVP): 010 judge family, 011 adjudication, 012 visual-QA
spike, 013 runs retention, 014 Langfuse, 015 non-one-shot arena mechanism.

## Phase 0 — Prose-first pilot (current)

The loop run by hand, every interface a file, zero framework dependencies.

- [x] Task spec schema + first spec (`specs/pr-review/taskspec.toml`, gate G1)
- [x] Arena `pr-review-v0`: 6 PR fixtures in Harbor task format (8 seeded
      defects + 1 clean PR), hidden answer keys, oracle solutions
- [x] Deterministic scorer (`runner/score.py`) and thin runner
      (`runner/run.py`): null/oracle/openrouter/pi candidate kinds, JSONL run
      records with tokens/cost/latency
- [x] Reference candidates validate the rig: oracle = 1.0, null = clean-task
      fraction
- [x] First real comparison recorded: baseline-oneshot vs pi-kimi (same
      model), run records committed (`runs/20260609T*.jsonl`)
- [ ] G2 meta-eval review of arena quality — report drafted at
      `approvals/G2-pr-review-v0.md`, awaiting human sign-off

**Exit (met):** the eval discriminates (oracle > agentic > one-shot > null
ordering interpretable), run records capture cost/tokens/latency, and the
autonomous loop produces a Pareto archive + comparison report unattended.

## Phase 1 — Harbor adoption

- `PiAgent` adapter (`BaseInstalledAgent`): install pi in task containers, run
  headless, parse usage into `AgentContext`
- Port arena as-is (already Harbor format); verifier becomes self-contained
- Multi-trial reward distributions (n ≥ 5); local Docker first, Daytona/Modal
  when parallelism pays
- Langfuse (self-hosted) as trace sink for lab runs
- One claude-code adapter run as a harness-comparison teaser

**Exit:** Phase 0 comparison reproduced under Harbor with distributions, on
identical fixtures.

## Phase 2 — Search + meta-eval

- Reflective loop: master agent reads worst-trial transcripts, proposes
  single-slot mutations (prompt packet first), keeps a Pareto archive over
  (quality, cost, latency) per the taskspec mode
- Meta-eval checks automated (cheap-baseline saturation, known-bad set,
  fixture-leak); GEPA library only if hand-rolled reflection plateaus
- Consider unfreezing the harness slot (pi vs claude-code vs codex)

**Exit:** a candidate beats both baselines on the declared objective;
meta-eval green; G2 signed.

## Phase 3 — Contract, deploy, observe

- Launch contract schema + agent package format (pinned pi version, config,
  prompts, tool policy, model, budgets)
- Reference deploy: one trigger class (GitHub PR webhook for pr-review);
  G3 approval required
- Production observation: Langfuse traces, budget alarms, weekly regression
  eval of the frozen arena against the live agent

**Exit:** the generated agent reviews real PRs on one repo under a signed
contract, traces visible.

## Phase 4 — The flywheel

- Production trace harvesting → redaction → new fixtures (gate G5)
- "Re-optimize for cost/quality/latency" re-enters Phase 2 with the live agent
  as incumbent baseline
- Rust kernel for stable schemas, receipt validation, contract tooling
- Second task family to pressure-test schema generality

**Exit:** one full cycle: deployed agent → production evidence → re-optimized
candidate → contract revision.
