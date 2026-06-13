# pr-review-correctness-v0 provenance

Prepared for backlog 034 as the first explicit correctness-specialist arena
for the review-swarm suite.

v0.1.0 copies candidate-visible environments from `arenas/pr-review-v2`
without changing the parent workspaces, then remaps hidden answer keys and
oracle solutions to the review-swarm taxonomy:

- parent `correctness`, `concurrency`, and `data-loss` defects become
  `logic-invariant`;
- the arena has no seeded `runtime-crash` task yet;
- `py-padding-clean` and `py-formatter-clean` remain clean false-positive
  traps.

This arena is not score-comparable with `pr-review-v2`; it answers a
different specialist-lens question. Any fixture, key, template, split, or
scorer change requires a version bump and fresh oracle/null/probe baselines.

## v0.1.0 freeze and bounded search

Freeze packet: `runs/20260613T151035Z-freeze-pr-review-correctness-v0`

- oracle mean: `1.0`
- null mean: `0.2857`
- one-shot probe mean: `0.0`
- freeze report: `runs/20260613T151035Z-freeze-pr-review-correctness-v0/freeze-report.md`

Bounded seed-only search:
`runs/20260613T161359Z-search-pr-review-correctness`

- command: `bin/daedalus run specs/pr-review-correctness/taskspec.toml --rng-seed 11 --budget-usd 0.75 --max-candidates 0 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2`
- recommended bounded baseline: `seed1-gpt-5-mini-spec-first`
- model: `openai/gpt-5-mini`
- composition hash: `f090f8060cf36637`
- certified: yes, under this seed-only run shape
- mean reward: `0.5714`
- total known spend: `$0.6253`

Evaluation caveat: the certified baseline is not strong enough for sandbox
member import. It repeatedly missed defects and repeatedly failed the
`py-padding-clean` clean trap. Treat the run as spread/headroom evidence and
as a model-pruning input for the next specialist iteration.
