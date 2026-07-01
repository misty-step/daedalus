# Request Bitterblossom ad-hoc dispatch agent pack

Priority: P2
Status: blocked - gated behind 054 (Cerberus-first mandate); first slice stays shaped
Estimate: L

> **GATED 2026-06-24** behind [[054]] — per [VISION.md](../VISION.md) "one
> customer until it hums," no second plane (Bitterblossom) is searched until the
> Cerberus reviewer loop hums. The First Slice below is still shaped and
> deliver-ready; it resumes the moment 054 closes. Demoted P0→P2.

> **Shaped 2026-06-22 — the immediate `/deliver` is the First Slice below.**
> The 5-family content beneath it is the broader roadmap. The first slice was
> reframed during shaping (operator steer: "why not code review / Cerberus?") —
> see the alternatives table.

---

## First Slice — measure one bb review lane on the existing arena ($0 dry-run)

**Reframe.** Don't build a new dispatch-agent arena+scorer. Code review already
has the whole measurement loop: validated arenas
(`arenas/pr-review-correctness-v0`), the deterministic findings scorer
(`crates/threshold-core/src/score.rs`), and the Cerberus substrate lab (048) —
and Bitterblossom already ships the review consumers
(`plane/agents/storm-correctness.toml`, `review-coordinator.toml`, the storm
lanes), today wired to **hand-picked** models (`storm-correctness` →
`deepseek/deepseek-v4-pro`). So 036's value is to **certify** those model/config
choices with cost/latency/quality evidence, reusing the loop that exists.

### Goal
Stand up and rig-validate (at ~$0) a measured search for **one** Bitterblossom
code-review lane (`storm-correctness`) over bb's candidate models on the existing
`pr-review-correctness-v0` arena — so the next, paid, G1 search can certify the
model/config bb imports instead of the current gut-picked `deepseek-v4-pro`.

### Non-Goals (this slice)
- The paid candidate search itself — a separate, informed G1 once the rig holds.
- Building any new arena or scorer — the pr-review-correctness arena + findings
  scorer exist and are validated.
- builder / fixer / a diff-producing harness adapter / execution-graded arenas
  (036 non-goal); other lenses (security/simplification/product); the
  review-coordinator; the diagnoser/verifier/release-support families — replicate
  after the first lane proves the loop.
- Editing bb plane files directly — emit a *proposed* overlay shape; bb imports.

### Constraints / Invariants
- Candidates never read `tests/` or `solution/` (arena boundary holds).
- Pi runs sequentially per machine (concurrency-deadlock constraint).
- The grader is gospel — no scorer/answer-key change without an arena version bump.
- Latest-models-only — every `[search].models` entry must be in the refreshed pool.
- bb plane boundary — the overlay carries `model`/config only; bb owns dispatch,
  ledger, budget, leases, receipts.

### Repo Anchors
- `specs/pr-review-security/taskspec.toml` — the `[search]` block shape to copy.
- `arenas/pr-review-correctness-v0/` — the validated arena (rig: oracle 1.0 / null
  floor / one-shot probe).
- `crates/threshold-core/src/score.rs` — the deterministic findings scorer (unchanged).
- `docs/primitives.md` — the model pool to refresh + the Pi concurrency constraint.
- `~/Development/bitterblossom/plane/agents/storm-correctness.toml` — import target
  (`harness="pi"`, `model=…`).
- `threshold doctor` · `arena-freeze` · `arena-validate` — the reused rig harness.

### Alternatives
| option | how it fails | verdict |
|---|---|---|
| **Reuse code-review arena + Cerberus (chosen)** | report-quality nuance not graded (deterministic only) — acceptable, judge later | **chosen** — zero new machinery, serves bb's shipped review storm |
| Build a new `diagnoser` arena+scorer first | a whole measurement loop (arena, root-cause keys, scorer) before any bb value; weeks of work the code-review loop already provides | rejected (operator steer) — revisit for the diagnoser family later |
| Go wide (all 5 families' specs+arenas) | spreads spend and scorer-design risk across 5 unbuilt loops before proving one | rejected — verification-system-first on one lane |
| Paid search immediately | spends before confirming the arena ranks these cheap reflex models (saturation risk) | rejected — `$0` dry-run gates the spend |

### Design (reuse, don't rebuild)
1. **Refresh `docs/primitives.md`** — verify bb's candidates
   (`moonshotai/kimi-k2.7-code`, `deepseek/deepseek-v4-pro`,
   `deepseek/deepseek-v4-flash`, `z-ai/glm-5.2`) against OpenRouter
   `/api/v1/models`; confirm `glm-5.2` now catalog-listed (bb confirmed 2026-06-16). $0.
2. **Author `specs/bb-review-correctness/taskspec.toml`** — copy the
   pr-review-correctness shape; `[search].models` = bb's candidate set;
   `fixtures = arenas/pr-review-correctness-v0`; lens = correctness;
   `mode = "threshold-then-cheap"` (cheap reflex lane).
3. **Validate the rig at ~$0** — `threshold doctor` (model-primitives +
   roster-in-pool); `arena-freeze` + `arena-validate` confirm non-saturable for
   this model set (oracle 1.0 / null floor / one-shot probe). Oracle/null are
   costless; the probe is one cheap call. **No paid candidate search.**
4. **Emit the dry-run packet** — the validated taskspec + the `arena-validate`
   report (rig holds) + a **paid-search plan** (candidate count, sequential
   budget, expected cluster-robust CIs given the arena's cluster count) + the
   **proposed `storm-correctness.toml` overlay shape** (the `model`/config the
   certified search will fill + the evidence fields it carries). Paid search is a
   separate `/deliver` under an explicit G1 budget.

### Oracle (executable — this $0 slice)
- [ ] `cargo run -q --bin threshold -- doctor` passes `model-primitives` and
      `roster-in-pool` (every `specs/bb-review-correctness` model is in the pool).
- [ ] `docs/primitives.md` lists all four bb candidates with current
      price/context/tools; `z-ai/glm-5.2` is catalog-listed or explicitly waived
      with a live-checked reason.
- [ ] `cargo run -q --bin threshold -- arena-freeze arenas/pr-review-correctness-v0
      --out-dir <tmp>` then `arena-validate … --probe-run <tmp>` exits 0
      (non-saturable) or records a true probe verdict for the bb model set — with
      **no paid candidate search**.
- [ ] A committed dry-run report names the rig (oracle/null/probe), the
      paid-search plan, and the proposed `storm-correctness.toml` `model`/config
      shape + evidence fields. No bb plane file is edited.
- [ ] `bin/gate` passes.

### Verification System
- **Claim:** the existing pr-review-correctness arena ranks bb's review-lane
  candidate models and is non-saturable for them, so a paid search would certify.
- **Falsifier:** a one-shot ties the oracle (saturated) → no paid search until the
  arena is hardened; or a bb candidate is absent from the verified pool.
- **Driver:** `threshold doctor` + `arena-freeze`/`arena-validate` over the new
  taskspec ($0).
- **Grader:** `arena-validate` exit 0 / probe verdict; `doctor` green; the report.
- **Evidence:** committed taskspec + arena-validate report + paid-search plan.
- **Cadence:** once, as the $0 gate before any paid bb-review search.
- **Gaps/waiver:** report-quality nuance (judge-rubric) deferred; other
  lenses/lanes/families deferred; bb-side import is a bb step. HTML plan waived —
  the contestable framing is resolved here; the slice is a fenced $0 rig
  validation reusing existing machinery.

### Risks + Rollout
- Refresh finds a bb candidate retired/renamed → swap to the latest per-tier
  (`doctor` enforces).
- The arena IS saturated for the cheap reflex models → the falsifier fires; harden
  the arena (040) before paying. The $0 slice exists to catch this *before* spend.
- Rollout: purely additive (a new taskspec + a report); no code/scorer change;
  trivially revertible.
- Follow-up (separate G1 `/deliver`): paid search → certified config → proposed bb
  overlay → (bb-side) import; then replicate to the next lens/lane.

### Premise Source
`sha256:3753299836dca286a27a53ab7afb0928093c047e03f32619db426b6bbfec9540`
`~/Development/bitterblossom/backlog.d/_done/061-sdlc-lifecycle-reflex-pack.md`
(the shipped consumer) + this ticket (036) + the 2026-06-22 shaping interrogation
(operator: reuse code-review/Cerberus; deterministic findings scorer; $0 dry-run
first).

---

## Goal

Design, evaluate, and export a focused agent pack for Bitterblossom ad-hoc
dispatch work: operator-started builder, fixer, diagnoser, verifier, and
release-support agents that can run through subscription-auth Codex/Claude
Code or through Pi on OpenRouter.

The immediate consumer is Bitterblossom's manual builder-dispatch path and
`backlog.d/061-sdlc-lifecycle-reflex-pack.md`. The output should let
Bitterblossom import narrow agent/task/card configs instead of asking a general
agent to improvise every dispatch lane.

## Why Now

Bitterblossom has a merged manual builder-dispatch role, but it is still a
single useful lane rather than a measured dispatch portfolio. The user wants
Threshold to find bespoke agent and harness configs for these recurring
operator flows, while Bitterblossom stays the event/dispatch plane.

The desired split:

- Threshold owns task specs, arenas, candidate compositions, cost/latency
  measurement, and launch-contract recommendations.
- Bitterblossom owns durable task/agent/card config, queueing, leases, budgets,
  run ledger, and operator-visible receipts.
- Codex/Claude subscription auth remains manual dispatch only unless a later
  approval creates a safe production auth boundary.
- Pi/OpenRouter configs are the first measured API-backed candidates.

## Scope

Design at least these agent families:

| family | purpose | first output contract |
|---|---|---|
| `builder` | implement a bounded ticket or patch from a shaped packet | branch/diff plus verification report |
| `fixer` | turn gate/review failures into a narrow patch attempt | fix plan, patch, and rerun evidence |
| `diagnoser` | investigate a fuzzy failure without committing speculative fixes | root-cause report and next-run packet |
| `verifier` | run the live acceptance loop and report what actually passed/failed | evidence packet with commands/artifacts |
| `release-support` | prepare closeout, demo, changelog, or rollback notes | operator-ready release packet |

Non-goals:

- A semantic workflow engine inside Bitterblossom dispatch.
- Production write authority for reflex agents.
- Treating Codex/Claude subscription lanes as Threshold-certified before they
  have an arena and comparable trace.
- Replacing the current Threshold `pi` V1 harness unless this ticket produces a
  separate approved harness-adapter child.

## Gut-Instinct Seed Configs

These are seeds for Threshold search, not launch recommendations.

OpenRouter facts were checked on 2026-06-15 from both the model page and
`https://openrouter.ai/api/v1/models`. `z-ai/glm-5.2` is page-visible at
`https://openrouter.ai/z-ai/glm-5.2` as released on 2026-06-15 with API access
releasing 2026-06-16, but it was not listed in the API catalog on 2026-06-15.
Keep it as page-visible/API-pending until the catalog and a local Pi smoke prove
it dispatchable. All model ids, including subscription-auth lane names, remain
subject to the requested `docs/primitives.md` refresh before any import
recommendation.

| role | harness | model/config | why seed it |
|---|---|---|---|
| builder baseline | Codex CLI | `gpt-5.5`, subscription auth, high/xhigh reasoning | current strongest local coding seat for manual dispatch; measure as incumbent only when a comparable adapter exists |
| builder open model | Pi | `moonshotai/kimi-k2.7-code`, `--thinking xhigh`, `full` tools | current Harness Kit open-model default and coding-focused OpenRouter model |
| builder comparator | Pi | `qwen/qwen3-coder-next`, medium/high thinking, `full` tools | cheap coding-family comparator |
| fixer long context | Pi | `deepseek/deepseek-v4-pro`, high thinking, `full` or `no-exec` tools | 1M context, cheap output, good fit for large failure packets |
| fixer triage | Pi | `deepseek/deepseek-v4-flash`, medium thinking, `explore` tools | very cheap 1M-context triage before spending builder budget |
| diagnoser | Pi | `deepseek/deepseek-v4-pro` or `minimax/minimax-m3`, high thinking, `read,bash` | broad context plus execution-aware investigation |
| verifier | Pi | `deepseek/deepseek-v4-flash`, low/medium thinking, `read,bash` | cheap evidence gathering and gate interpretation |
| release-support | Pi | `moonshotai/kimi-k2.7-code` or `z-ai/glm-5.1`, medium thinking | structured prose and repo-aware summary candidate |
| architecture council | OpenRouter | `openrouter/fusion` | use only for research/council questions worth multi-model routing; not a routine coding lane |
| pending GLM | Pi | `z-ai/glm-5.2` | requested by operator; page-visible/API-pending; do not use until catalog-listed and smoke-tested |

## Requested Threshold Work

1. Write task specs and arenas for the five families above, starting with a
   vertical slice of `builder`, `fixer`, and `diagnoser`.
2. Refresh `docs/primitives.md` before finalizing the search space; model ids,
   prices, context, tools, and Pi behavior are live facts.
3. Run Pi candidates sequentially per machine until the documented Pi
   concurrency deadlock is retested and cleared.
4. Measure quality, cost, latency, output-schema validity, and artifact
   usefulness against deterministic fixtures before recommending any import.
5. Emit a Bitterblossom import packet with:
   - `plane/agents/*.toml` overlays,
   - `plane/tasks/*/task.toml` and `card.md` suggestions,
   - payload examples for `bb run`,
   - expected ledger/artifact fields,
   - cost and wall-clock budget defaults,
   - residual risks and approval gates.
6. Emit separate "manual subscription" overlays for Codex and Claude Code that
   are clearly marked unmeasured unless Threshold adds a comparable harness
   adapter and trace.

## Oracle

- [ ] A human-reviewable Threshold G1 task-spec packet is produced for each
      selected dispatch family, with goal, inputs, outputs, risk class, budget
      posture, and negative examples. This does not imply self-approval of G1.
- [ ] At least three Pi/OpenRouter candidates are run for the first vertical
      slice, including `moonshotai/kimi-k2.7-code`,
      `deepseek/deepseek-v4-pro`, and `deepseek/deepseek-v4-flash`, with
      run records showing cost, latency, traces, and composition hashes.
- [ ] `z-ai/glm-5.2` is either catalog-listed and smoke-tested or explicitly
      waived as API-pending; `z-ai/glm-5.1` may be used as the available GLM
      comparator.
- [ ] The launch recommendation names a quality/cost/latency mode for each
      family rather than using one universal "best model" claim.
- [ ] Bitterblossom import artifacts preserve the plane boundary: task cards
      own workload judgment; the Rust spine only routes, budgets, leases,
      records, and reports.
- [ ] Codex/Claude subscription-auth paths are marked manual dispatch only
      and are not represented as API reflex lanes.
- [ ] `bin/gate` passes.

## Evidence

- Bitterblossom anchor:
  `/Users/phaedrus/Development/bitterblossom/backlog.d/061-sdlc-lifecycle-reflex-pack.md`
- Current Threshold primitive facts:
  `docs/primitives.md`
- Current Harness Kit model facts:
  `/Users/phaedrus/Development/harness-kit/skills/roster/references/model-provider-harness-index.md`
