# Code Review Autoresearch Loop

Daedalus should improve code reviewers through patient, recorded research
loops, not by treating one good prompt as the product. The product is still
the evidence-backed agent contract: frozen arena, composition hash, run
records, export boundary, and residual risk.

This document is the operating primitive for the review-swarm research program.
It complements `docs/review-swarm-taxonomy.md`, `docs/primitives.md`, and
`docs/operator-sop.md`.

Direction update, 2026-06-20: Cerberus has usurped the active product path for
autonomous/reflex code review. Treat this loop as preserved research evidence
for lane scoring, artifact contracts, and plateau discipline. New review
product work flows through `backlog.d/048-build-cerberus-substrate-rd-lab.md`
unless that lab explicitly revives a swarm topology. Pi remains the native
Daedalus V1 search harness for this archived research loop; it is not the
default substrate for Cerberus-backed reflex review labs.

The machine-readable contract is intentionally small. Tests parse this block;
the prose below explains judgment around it.

```toml
schema = "review-autoresearch-loop.v1"
program = "pr-review-swarm"
first_lane = "correctness"
next_arena_iteration = "pr-review-correctness-v0.3"
sandbox_boundary = "member-artifacts-only-before-g3"
do_not_average_across_arena_versions = true
full_swarm_blocked_until = ["correctness-member-quality", "real-member-replay"]
required_loop_evidence = [
  "primitive-refresh",
  "arena-freeze",
  "controlled-hypothesis",
  "certified-search-or-postmortem",
]
```

## Objective

Run one reviewer lens at a time until either:

- a measured composition clears the lens threshold with acceptable cost,
  latency, artifact validity, and false-positive discipline; or
- the loop produces a postmortem explaining why the current arena/search
  strategy cannot produce a trustworthy reviewer.

Do not promote a full swarm from weak member agents. Member artifacts may be
exported for sandbox inspection only while G2/G3 remain unsigned or while real
member replay has not passed.

## Reviewer Lanes

The first measured lanes remain:

| lane | status | current next question |
|---|---|---|
| general | certified baseline | Does it still help master synthesis after specialist members improve? |
| correctness | weak v0.2 baseline; v0.3 holdout rotation prepared | Can the next run certify a Qwen/GPT correctness member on the rotated hard holdout without clean-trap regressions? |
| security | promising v0.1 baseline | Can injection instability be reduced without losing credential-exposure recall? |
| verification | scaffold-only | Which deterministic fixtures prove execution-aware review instead of test-prose guessing? |
| simplification | scaffold-only | Which fixtures distinguish real gate/surface risk from subjective taste? |
| product | scaffold-only | Which fixtures encode ticket intent without turning the oracle into preference scoring? |
| master | synthetic reducer baseline | Does performance transfer to artifacts emitted by real member candidates? |

Candidate future lanes must earn their own arena and scorer story before they
become part of the suite:

- API contract and backwards compatibility.
- Data migration, persistence, and schema safety.
- Performance and resource regression.
- Concurrency and lifecycle races.
- Frontend behavior, accessibility, and visual regression.
- Dependency, supply-chain, and generated-code risk.
- Test quality and verification honesty.
- Release, rollback, and operational risk.

## Primitive Inventory

Refresh `docs/primitives.md` before authoring or materially changing a search
space. Model prices, context lengths, tool support, and supported parameters
are live facts. A stale primitive list creates false attribution.

The current mutable surfaces are:

- `model` from the verified OpenRouter pool.
- `thinking` level.
- `tool_policies`: `full`, `explore`, and `no-exec` where declared.
- `prompt_packet`.
- `system_prompt_mode`: `append` or `replace`.
- optional `skills` and `agents_md` only when the taskspec declares them.
- `timeout_sec` and cost envelope.
- arena construction: fixture mix, split, answer key, clean traps, and hidden
  holdouts.

Not every surface should move in every loop. Prefer one controlled hypothesis
per iteration so a result can be attributed.

## Loop

1. Pick one lane and write the hypothesis in that arena's provenance.
2. Check whether the arena can answer the question. If not, bump the arena
   version before spending search budget.
3. Freeze the arena: oracle 1.0, null floor equal to clean fraction, one-shot
   below saturation, and visible agent spread.
4. Run a bounded search with seeded diversity and at least one reflective
   generation when budget allows.
5. Read the task grid, not just the mean. Classify each miss and false
   positive by task, category, model, prompt stance, tool policy, and cost.
6. Change one thing: arena, search space, prompt stance, model pool, tool
   policy, timeout, or output contract.
7. Repeat until the lane clears its threshold or reaches a plateau.

Every iteration writes durable evidence under the arena provenance and run
directory. Never average rewards across arena versions.

## Plateau Postmortem

Write a postmortem instead of spending more when two consecutive iterations
fail to improve beyond observed trial noise or when failures cluster around an
unmodeled requirement. The postmortem names the suspected bottleneck: arena,
answer key, prompt, model pool, tool policy, timeout, schema, lens breadth, or
true budget-constrained plateau.

If the answer is "bad eval", fix the arena before another search. If the
answer is "bad loop strategy", change the search plan before another spend. If
the answer is "intractable under current constraints", preserve the evidence
and keep the member out of the swarm recommendation.

## Correctness v0.3 Plan

Correctness is the first focused loop because it is required for the vertical
slice and the existing evidence is not sandbox-ready. The v0.1 evidence showed
repeated misses on seeded defects, repeated false positives on
`py-padding-clean`, and no seeded `runtime-crash` task despite
`runtime-crash` being an owned category.

The v0.2 loop added `py-formatter-missing-crash` for `runtime-crash`, but the
certified child was still weak and the v0.2 holdout became burned:
`py-plugin-cache` and `py-export-clear` each have eight exposures, above the
default threshold of five.

The v0.3 loop should:

- keep the v0.2 fixtures, answer keys, scorer constants, and taxonomy;
- rotate the burned holdout to `py-live-lock` and
  `py-formatter-missing-crash`;
- re-run the freeze gate with non-inconclusive one-shot probe evidence before
  any paid search; `backlog.d/047-replace-real-repo-saturation-probe.md`
  tracks the real-repo-scale probe blocker exposed by v0.3 diagnostics;
- keep both clean traps, unless an adjudication explains a replacement;
- preserve real-repo-scale context and candidate isolation;
- run a certified search with enough candidate diversity to compare at least
  model, prompt stance, and one execution-aware tool policy;
- report per-task deltas against earlier versions as narrative only, not
  averaged scores;
- end in either a sandbox-candidate recommendation or a plateau postmortem.

The next suite-level work remains blocked until correctness v0.3 and
real-member replay give the master reviewer real member artifacts to reduce.
