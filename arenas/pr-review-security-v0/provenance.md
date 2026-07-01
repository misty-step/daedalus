# pr-review-security-v0 provenance

Prepared for backlog 034 as the first explicit security-specialist arena for
the review-swarm suite.

v0.1.0 contains:

- `py-markup-escape`, adapted from `pr-review-v2` and remapped from the old
  `security` category to review-swarm `injection`;
- `py-save-token-leak`, a real-repo-scale Rich workspace copied from
  `py-padding-clean` with a synthetic PR diff that logs an environment token in
  `Console.save_text`;
- `py-padding-clean`, retained as a clean false-positive trap.

This arena is not score-comparable with `pr-review-v2`; it answers a
specialist-lens question. Any fixture, key, template, split, or scorer change
requires a version bump and fresh oracle/null/probe baselines.

## v0.1.0 freeze and bounded search

Freeze packet: `runs/20260613T151035Z-freeze-pr-review-security-v0`

- oracle mean: `1.0`
- null mean: `0.3333`
- one-shot probe mean: `0.0`
- freeze report: `runs/20260613T151035Z-freeze-pr-review-security-v0/freeze-report.md`

Diagnostic interrupted run:
`runs/20260613T151153Z-search-pr-review-security`

- status: interrupted, not certification evidence
- finding: optimizer-authored `seed-spec-first.md` was degenerate repeated
  punctuation and drove a timeout-heavy candidate measurement
- remediation: the Rust prompt-packet module now rejects visibly corrupted packet
  text for seed packets and prompt-packet mutations

Bounded seed-only search:
`runs/20260613T153751Z-search-pr-review-security`

- command: `cargo run --quiet --bin threshold -- run specs/pr-review-security/taskspec.toml --rng-seed 9 --budget-usd 0.40 --max-candidates 0 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2`
- recommended bounded baseline: `seed5-kimi-k2-6-checklist`
- model: `moonshotai/kimi-k2.6`
- composition hash: `d112f8dd00b0f84b`
- certified: yes, under this seed-only run shape
- mean reward: `0.8333`
- total known spend: `$0.3527`

Evaluation caveat: the recommended candidate was perfect on the authored
credential-token holdout but unstable on repeated `py-markup-escape`
injection trials. Treat this as a sandbox-inspection candidate at most, not a
primary security reviewer.
