# G1 - Spec sign-off: pr-review-suite

- **Status:** pending human approval
- **Spec:** `specs/pr-review-suite/taskspec.toml`
- **Master spec:** `specs/pr-review-master/taskspec.toml`
- **Required member specs:** `specs/pr-review/taskspec.toml`,
  `specs/pr-review-correctness/taskspec.toml`,
  `specs/pr-review-security/taskspec.toml`
- **Optional scaffold specs:** `specs/pr-review-verification/taskspec.toml`,
  `specs/pr-review-simplification/taskspec.toml`,
  `specs/pr-review-product/taskspec.toml`
- **Taxonomy:** `docs/review-swarm-taxonomy.md`
- **Scope:** offline synthetic PR-review swarm experimentation only

## Decision

Not approved for paid search yet.

This packet prepares the first review-swarm task family and master-synthesis
spec. A human G1 reviewer must approve the suite spec, master spec, taxonomy,
required member specs, budget posture, and search space before Daedalus
spends model budget on the vertical-slice one-shot probes or candidate
search. The optional scaffold specs are included for boundary review only;
they are marked non-runnable and require a later fixture/headroom approval
before any model budget is spent on those members.

## Approval Boundary

Approval, when granted, covers only:

- offline synthetic fixtures;
- member artifact and master synthesis evaluation;
- sandbox-only delivery export preparation.

Approval does not authorize:

- public benchmark-quality claims;
- primary reviewer deployment;
- PR comments or production write authority;
- production trace or PR data re-ingestion.

## Human Decision

Pending.

## No-Spend Evidence Prepared

- `bin/daedalus taxonomy-validate docs/review-swarm-taxonomy.md --suite specs/pr-review-suite/taskspec.toml`
  passes.
- `runs/20260612T205852Z-freeze-pr-review-master-v0` contains oracle/null
  reference records for `arenas/pr-review-master-v0`.
- `runs/20260612T205852Z-freeze-pr-review-master-v0/freeze-report.md` is
  intentionally failing until a one-shot probe is run after G1.
