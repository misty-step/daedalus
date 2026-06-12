# Low-risk offline synthetic spend policy

- **Status:** approved by operator
- **Approved:** 2026-06-12
- **Scope:** Daedalus experiments over low-risk offline synthetic fixtures

## Operator Approval

> Ahh I see. Yes you have pre-approved spend to run experiments and tackle
> issues, within reason

## Standing Envelope

Daedalus may proceed without another per-run human prompt when all conditions
are true:

- The task spec and arena declare `[risk].class = "low"`.
- Inputs are offline synthetic fixtures or open-source snapshots already
  committed to the repo.
- Candidates cannot read `tests/` or `solution/`.
- No production data, secrets, PR comments, write authority, or networked
  control-plane actions are involved.
- A concrete budget is passed to the run command and does not exceed `$8.00`
  known spend for a single backlog item without a new human decision.
- `bin/gate`, relevant validators, oracle/null baselines, and a fresh-context
  critic are used before merge or G2 trust claims.

## Exclusions

This policy does not approve:

- G3 launch.
- G4 production write authority.
- G5 production-data re-ingestion.
- Public benchmark-quality claims.
- Primary-reviewer deployment.
- Optional scaffold specs that are marked non-runnable.

When any exclusion applies, write or update the explicit gate file in
`approvals/` and wait for a human decision.
