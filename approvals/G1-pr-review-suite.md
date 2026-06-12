# G1 - Spec sign-off: pr-review-suite

- **Status:** approved by human G1 reviewer
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

Approved for low-risk offline synthetic experiment spend under the budget and
boundary below.

This packet prepares the first review-swarm task family and master-synthesis
spec. The human G1 approval below authorizes Daedalus to spend model budget on
the vertical-slice one-shot probes and candidate search. The optional scaffold
specs are included for boundary review only; they are marked non-runnable and
require a later fixture/headroom approval before any model budget is spent on
those members.

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

Approved by the operator on 2026-06-12:

> Ahh I see. Yes you have pre-approved spend to run experiments and tackle
> issues, within reason

Interpreted scope for this G1:

- Applies to `pr-review-suite`, `pr-review-master`, and the required
  vertical-slice member specs only.
- Allows offline synthetic one-shot probes, certified candidate searches,
  real-member replay, and sandbox-only export preparation.
- Budget envelope for this G1 run: max `$8.00` known spend total before a new
  human decision, and max per-trial ceilings from the task specs.
- Does not approve optional scaffold member search; those specs remain
  non-runnable until fixtures and headroom are approved.
- Does not approve public benchmark claims, G3 launch, G4 write authority,
  G5 production-data re-ingestion, PR comments, or primary-reviewer use.

This is the human G1 approval record; it is not an agent self-approval.

## No-Spend Evidence Prepared

- `bin/daedalus taxonomy-validate docs/review-swarm-taxonomy.md --suite specs/pr-review-suite/taskspec.toml`
  passes.
- `runs/20260612T205852Z-freeze-pr-review-master-v0` contains oracle/null
  reference records for `arenas/pr-review-master-v0`.
- `runs/20260612T205852Z-freeze-pr-review-master-v0/freeze-report.md` is
  intentionally failing until a one-shot probe is run after G1.
