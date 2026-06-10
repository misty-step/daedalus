# Delivered agent: pr-review-glm5-specfirst-medium

The output of the first full Daedalus cycle: spec → search space → arena
with headroom → seeded landscape scan → hypothesis races → holdout final →
this package. Recommended under the taskspec's **threshold-then-cheap**
mode.

## Composition

| slot | value | how it was chosen |
|---|---|---|
| harness | pi (headless, OpenRouter) | V1 frozen slot |
| model | `z-ai/glm-5` | seeded landscape scan (beat 5 other models) |
| prompt packet | `packet.md` — spec-first review stance | optimizer-authored seed stance; survived two mutation challenges |
| thinking | `medium` | **search-discovered**: mutated from `high` (g1b), kept reward 1.0 at ~42% lower search-phase cost |
| tools | read, bash, edit, write (`full` policy) | seeded; restricted policies never beat it |

Composition hash at selection: `44a9aa47e96933ed`
(candidate `g1b-seed1-glm-5-spec-first`).

## Evidence

**Search run** (`runs/20260610T160533Z-search-pr-review-v0`): 8/8 trials
at reward 1.000 across train, validation, and the unseen holdout task, at
$0.0138/trial and 61.2s mean wall — best of 10 measured compositions
(report.md, pareto.json). Rig context: oracle 1.000, null 0.250, one-shot
saturation probe 0.000. Landscape: 6 seeds spanned 0.167–1.000 at 230×
cost spread (reproducible with `--rng-seed 1106`); both
accuracy-for-cost packet mutations regressed and were discarded on
evidence (loop.json).

**Certification repro** (2026-06-10, 8 further trials of this exact
package): the in-search 1.000 does not survive larger n. Observed per-task
success across all 16 trials of this composition:

| task | success | note |
|---|---|---|
| py-progress-speed | 3/3 | stable |
| py-padding-clean (FP trap) | 3/3 | stable — never invents findings |
| py-measure-normalize | 3/5 | finds the defect every time; twice cited a line outside the key span (arena calibration, backlog 019) |
| py-live-lock (holdout) | 2/5 | genuinely flaky on the subtle concurrency defect |

Honest point estimate: **~0.69 mean reward** with high per-task variance,
~$0.014–0.12 per trial depending on how long it deliberates. The
*ranking* among candidates stands (all were measured under the same
protocol), but certification at n ≥ 5 per task is now a delivery
requirement (backlog 019) before any reward number is contract-grade.

Reproduce:

```sh
runner/run.py --candidate deliveries/pr-review/agent.toml \
    --arena arenas/pr-review-v2 --final --trials 5
```

## Launch contract sketch (G3 — not signed; do not deploy)

- **Trigger intent:** GitHub PR webhook (Phase 3); manual runs until then.
- **Input packet:** post-change checkout + unified diff, arena template
  instruction (arenas/pr-review-v2/template.md shape).
- **Permissions:** read-only on the repo checkout; writes only
  `findings.json` in a throwaway workdir; env restricted to
  `OPENROUTER_API_KEY`.
- **Budgets:** ≤ $0.50 and ≤ 600s per review (taskspec); observed ~$0.014
  and ~61s.
- **Escalation:** malformed output or timeout → no findings posted, run
  flagged for human review. Never auto-merge/auto-block; findings are
  advisory comments until G4 grants more.
- **Regression eval:** re-run this arena's holdout monthly and on any
  packet/model/harness change; composition hash must match the contract.

## Residual risks

- **Within-composition variance** is the dominant risk: the same agent
  swings 0.0–1.0 on the two subtle tasks. The search's n=2 protocol
  overestimated the winner; do not deploy on the search numbers alone
  (certification gate, backlog 019).
- Whether g1b truly beats seed1/seed5 (the other reward-1.0 candidates)
  at production sample sizes is unresolved at this n.
- One holdout task only; scores come from seeded synthetic defects in one
  repo (rich); real-PR distribution shift is unmeasured until Phase 3
  traces exist.
- glm-5 pricing/routing can change upstream; the contract pins the model
  id, not the provider's serving stack (provider pinning available via
  manifest `provider` table if needed).
