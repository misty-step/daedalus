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

- command: `cargo run --quiet --bin threshold -- run specs/pr-review-correctness/taskspec.toml --rng-seed 11 --budget-usd 0.75 --max-candidates 0 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2`
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

## v0.2.0 correctness autoresearch loop

Hypothesis: v0.1 under-specifies the correctness lens because it declares
`runtime-crash` ownership but contains no seeded runtime-crash fixture. v0.2.0
adds `py-formatter-missing-crash`, adapted from the existing Pygments
formatter workspace, to test normal-input crash detection without changing the
clean traps.

The new task changes the unknown-formatter error path in
`pygments/formatters/__init__.py`: the refactor keeps the assignment
expression but interpolates undefined `alias` instead of `_alias`, so an
unknown formatter alias raises `NameError` before the documented
`ClassNotFound` can be raised.

Freeze packet: `runs/20260613T213700Z-freeze-pr-review-correctness-v020`

- oracle mean: `1.0`
- null mean: `0.25`
- one-shot probe mean: `0.0`
- freeze report:
  `runs/20260613T213700Z-freeze-pr-review-correctness-v020/freeze-report.md`

Bounded search with one reflective child:
`runs/20260613T214006Z-search-pr-review-correctness`

- command: `cargo run --quiet --bin threshold -- run specs/pr-review-correctness/taskspec.toml --rng-seed 12 --budget-usd 1.25 --max-candidates 1 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2`
- runner-recommended certified baseline:
  `g1a-seed3-qwen3-7-plus-skeptic`
- model: `z-ai/glm-4.7-flash`
- composition hash: `196352774b5cab55`
- certified: yes, under this run shape
- mean reward: `0.5625`
- total known spend: `$1.3002`

Postmortem: v0.2.0 is a better arena, not a sandbox-ready member. The new
runtime-crash fixture discriminates agents: `seed3-qwen3-7-plus-skeptic`
found it once and the certified child found it once during certification, but
most candidates missed it. The runner recommendation is not quality-credible
for sandbox import: it is lower-reward than the non-certified Qwen seed, missed
`py-live-lock` in both measured trials, was unstable on `py-padding-clean`,
`py-progress-speed`, `py-plugin-cache`, and the new runtime-crash task, and
carried long-tail latency. The useful result is the diagnosis:

- `py-live-lock` remains the hardest train defect for most configurations;
- `py-measure-normalize` is expensive and inconsistent across stronger models;
- Qwen found the new runtime-crash task and swept holdout, but was not
  certified across all search tasks;
- the cost-saving GLM child did not preserve Qwen's quality profile.

Next loop: keep v0.2.0 frozen for comparability, then run targeted
certification or a v0.2b search that certifies the high-signal Qwen/GPT
candidates across all train and validation tasks before any suite replay.
If clean-trap instability persists, write a plateau postmortem instead of
exporting a correctness member.

Post-search holdout state: `py-export-clear` and `py-plugin-cache` each have
eight v0.2.0 exposures, above the default burn threshold of five. Do not run
another certified holdout search against v0.2.0 without rotating holdouts and
bumping the arena version, or explicitly raising the burn threshold in a
documented diagnostic-only command.

## v0.3.0 holdout rotation

Backlog 034's next loop needs a valid holdout before any paid certification
attempt. Current `arena-validate` correctly fails v0.2.0 because
`py-export-clear` and `py-plugin-cache` each have eight holdout exposures,
above the default burn threshold of five.

v0.3.0 keeps the same fixtures, answer keys, scorer constants, and taxonomy.
Only the arena version and split move:

- train: `py-progress-speed`, `py-padding-clean`, `py-plugin-cache`
- validation: `py-measure-normalize`, `py-formatter-clean`, `py-export-clear`
- holdout: `py-live-lock`, `py-formatter-missing-crash`

Rationale: the new holdout focuses the next certification attempt on the two
defect shapes that blocked the prior correctness member: concurrent live
update atomicity and reachable formatter-crash behavior. The former v0.2
holdout tasks remain in train/validation so the next run still measures data
loss/cache-invariant behavior without treating already-burned tasks as hidden
final evidence.

This is not a new benchmark-quality claim. The source repositories are still
public, and v0.3.0 still requires a fresh freeze report with non-inconclusive
one-shot probe evidence before a certified search can be trusted.

Diagnostic freeze attempts on 2026-06-19:

- `cargo run --quiet --bin threshold -- arena-validate
  arenas/pr-review-correctness-v0 --probe-run
  runs/20260613T213700Z-freeze-pr-review-correctness-v020 --report
  /tmp/threshold-034-v03-validate-old-probe.md`:
  oracle `1.0`, null `0.25`, holdout exposures
  `{"py-formatter-missing-crash": 0, "py-live-lock": 0}`, but the old
  one-shot probe is inconclusive (`1/1` probe trial errored).
- `cargo run --quiet --bin threshold -- arena-freeze
  arenas/pr-review-correctness-v0 --out-dir
  /tmp/threshold-034-freeze-v03 --probe-model
  deepseek/deepseek-v4-pro --probe-context-window 1000000`: oracle `1.0`,
  null `0.25`, holdout exposures
  `{"py-formatter-missing-crash": 0, "py-live-lock": 0}`, probe mean
  `0.375`, but still inconclusive because `3/8` one-shot trials exceeded
  the one-million-token context preflight. Known probe spend: `$2.207417`.
- `cargo run --quiet --bin threshold -- arena-freeze
  arenas/pr-review-correctness-v0 --out-dir
  /tmp/threshold-034-freeze-v03-scout --probe-model
  meta-llama/llama-4-scout --probe-context-window 10000000`: oracle `1.0`,
  null `0.25`, holdout exposures
  `{"py-formatter-missing-crash": 0, "py-live-lock": 0}`, but the probe is
  inconclusive because `8/8` trials returned empty content.

Conclusion: v0.3.0 repairs the burned-holdout state, but the current one-shot
saturation probe is not yet a reliable freeze oracle for this real-repo-scale
arena. Do not run paid certification until the saturation-probe design is
updated or a fresh non-inconclusive freeze report exists.
