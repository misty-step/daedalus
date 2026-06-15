# Request Bitterblossom ad-hoc dispatch agent pack

Priority: P0
Status: pending
Estimate: L

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
Daedalus to find bespoke agent and harness configs for these recurring
operator flows, while Bitterblossom stays the event/dispatch plane.

The desired split:

- Daedalus owns task specs, arenas, candidate compositions, cost/latency
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
- Treating Codex/Claude subscription lanes as Daedalus-certified before they
  have an arena and comparable trace.
- Replacing the current Daedalus `pi` V1 harness unless this ticket produces a
  separate approved harness-adapter child.

## Gut-Instinct Seed Configs

These are seeds for Daedalus search, not launch recommendations.

OpenRouter catalog facts were checked on 2026-06-15 from
`https://openrouter.ai/api/v1/models`. `z-ai/glm-5.2` was not listed at that
time; keep it as a requested/pending model until the catalog and a local Pi
smoke prove it. All model ids, including subscription-auth lane names, remain
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
| pending GLM | Pi | `z-ai/glm-5.2` | requested by operator; do not use until listed and smoke-tested |

## Requested Daedalus Work

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
   are clearly marked unmeasured unless Daedalus adds a comparable harness
   adapter and trace.

## Oracle

- [ ] A human-reviewable Daedalus G1 task-spec packet is produced for each
      selected dispatch family, with goal, inputs, outputs, risk class, budget
      posture, and negative examples. This does not imply self-approval of G1.
- [ ] At least three Pi/OpenRouter candidates are run for the first vertical
      slice, including `moonshotai/kimi-k2.7-code`,
      `deepseek/deepseek-v4-pro`, and `deepseek/deepseek-v4-flash`, with
      run records showing cost, latency, traces, and composition hashes.
- [ ] `z-ai/glm-5.2` is either verified live and smoke-tested or explicitly
      waived as unavailable; `z-ai/glm-5.1` may be used as the available GLM
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
- Current Daedalus primitive facts:
  `docs/primitives.md`
- Current Harness Kit model facts:
  `/Users/phaedrus/Development/harness-kit/skills/roster/references/model-provider-harness-index.md`
