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
- Meta-eval finding (reframed 2026-06-10, ticket 016): the one-shot probe
  matches the oracle on pr-review-v1 — the arena is **saturated** and cannot
  rank agent configurations. The MVP's original framing ("one-shot ties
  agentic at 1.7× lower cost → recommend the baseline") was a category error:
  comparisons in this lab are agent vs agent; the one-shot is a rig probe,
  never a candidate, and a saturated arena now aborts the search.

Live runs surfaced and fixed four real bugs (rig discrimination threshold,
empty-holdout crash, optimizer transient-retry, reasoning-model token
headroom) — each now gate-protected.

009 finding retained: synthetic cross-file defects do NOT defeat the one-shot
baseline (it inlines every file), so a genuinely non-one-shot arena needs a
large repo or execution (backlog 015). The freeze gate correctly kept
pr-review-v1 unfrozen.

Remaining backlog (post-MVP): all cleared 2026-06-10 (see "Backlog cleared"
below). 015 became pr-review-v2 (real-repo arena).

## Agent-vs-agent redesign + first full cycle (2026-06-10)

Operator review found the MVP's baseline framing wrong (one-shot is never a
candidate); tickets 016–018 + re-scoped 015 rebuilt the search and the first
complete spec→delivery cycle ran end-to-end
(`runs/20260610T160533Z-search-pr-review-v0`, $3.03):

- **016** one-shot demoted to a saturation probe (reference kind); saturated
  arenas abort the search.
- **017** taskspec `[search]` space + seeder: 6 diverse pi compositions
  (model × thinking × tool policy × optimizer-authored packet stances),
  reproducible RNG, budget-metered.
- **018** loop v2: archive parent pool (per-task winners selectable), ≥2
  competing single-slot hypotheses raced per generation, variance-aware
  keep, `tools` mutable; temperature/max_tokens frozen out (pi cannot
  express them).
- **015** pr-review-v2: real-repo arena (rich v14, ~350K tokens/workspace).
  Freeze gate passed: oracle 1.000 / null 0.250 / probe **0.000** (context
  overflow) / seeds spread 0.167–1.000 at 230× cost spread.
- Capstone: landscape scan found glm-5+spec-first at 1.000; the loop's
  thinking=high→medium mutation held 1.000 at 42% lower cost and aced the
  unseen holdout; two accuracy-for-cost packet mutations regressed and were
  discarded on evidence. Delivered package: `deliveries/pr-review/`
  (hash 44a9aa47e96933ed).
- Live bug found by the run and fixed gate-protected: Pareto/recommendation
  compared total cost across unequal trial counts; now cost-per-trial.
- Honest certification finding: repro at larger n shows within-composition
  variance (per-task 3/5 and 2/5 on the subtle tasks; ~0.69 point estimate
  vs the in-search 1.000). Candidate *ranking* stands; contract-grade
  claims need n ≥ 5 certification → backlog 019 (arena v2.1 calibration).

## Backlog cleared — rigor + breadth program (2026-06-10)

Operator adversarial review → full backlog burn-down. All open tickets shipped
(108 gate tests, clean tree). The lab is now scientifically rigorous end to end:

- **020 certification racing** — top-k topped to n≥5/task before any
  recommendation; only certified candidates ship (kills the winner's curse).
- **021 mode-aware search** — keep/plateau/parents optimize the declared mode
  (held reward + lower cost = improvement under threshold-then-cheap).
- **022 in-loop meta-eval monitor** — saturation/variance/FP-trap alarms;
  "the arena is the bottleneck" is now a first-class run outcome with a draft
  arena-findings note.
- **023 expanded composition surface** — system_prompt_mode, skills, agents_md
  joined the hashed mutable slots (live: agents_md briefing took a cheap model
  to 1.0 at $0.002/trial).
- **024 lineage + lab notebook** — every run renders its full discovery story
  from artifacts; runs/NOTEBOOK.md accumulates cross-run lessons.
- **025 proposer evidence + ledger + transplant** — scorer-verdict evidence,
  predicted_effect scored against measurement, donor-slot recombination.
- **026 control-plane export** — `daedalus export` emits a pinned launch
  contract + Bitter Blossom sprite persona (byte-identical measured packet).
- **013 retention** — records committed, artifacts indexed + gitignored;
  recovered orphaned MVP run records.
- **011 adjudication** — `adjudications.md` workflow; both py-file-cache
  disputes worked live (one ACCEPT → arena 0.3.0, one OUT-OF-SCOPE).
- **019 + 027 arena scale-up** — pr-review-v2 → 10 tasks across rich +
  pygments, full taxonomy, 3-task holdout, exposure ledger, function-wide key
  spans. Rig passes; new-task agent-spread pending the next full search run.
- **010 judge family** — calibrated 0–5 rubric judge; the calibration gate
  *failed live* (two judges Spearman 0.0 on empty-findings) and correctly
  refused to certify — the gate works.
- **014 Langfuse → ADR-002** — trace as an export-time view (runner/trace.py,
  validated on the capstone); live stand-up deferred by design.
- **012 visual-QA → ADR-003** — GO with deterministic-oracle scope, proven by
  a real Playwright DOM-assertion probe (defective app 0, fixed app 1).

Two live harness findings recorded in primitives.md: concurrent `pi -p`
processes deadlock (run trials sequentially); replace-mode system prompts work
(an earlier timeout was that deadlock, not the slot).

Update (2026-06-11): the fresh v0.2.0 certification run is recorded at
`runs/20260611T173632Z-search-pr-review-v0` with lineage, Pareto archive,
artifacts index, holdout ledger entry, and a regenerated
`deliveries/pr-review/` package plus cross-plane handoff. The result is
useful but not triumphant: the only certified recommendation is
`seed4-qwen3-7-plus-checklist` at 0.5714, while `py-markup-escape`,
`py-guess-swallow`, and `py-measure-normalize` are promoted in
`arena-findings.md` for G2 human decision. Next: human G2 review in
`approvals/G2-pr-review-v2.md`; then Phase 3 launch/observe work remains
ticket 029 against Olympus / Bitter Blossom via the export contract.

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

**Exit (met):** the rig discriminates (oracle ceiling, null floor, saturation
probe interpretable), run records capture cost/tokens/latency, and the
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
