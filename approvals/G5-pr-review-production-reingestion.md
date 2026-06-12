# G5 - Production-data re-ingestion review: pr-review

- **Status:** pending
- **Scope:** production PR data or traces flowing back into Daedalus arenas

## Current Decision

No production data re-ingestion is approved.

Before production PRs, review outputs, traces, comments, repository metadata,
or user data can become arena fixtures, a human reviewer must approve
redaction, consent/ownership boundaries, retention, fixture provenance, and
holdout exposure policy.

## Required Before Approval

- [ ] Data sources are named and scoped.
- [ ] Secrets, private code, user identifiers, and customer data are redacted
      or explicitly excluded.
- [ ] Fixture provenance records the source, transformation, and reviewer.
- [ ] Holdout exposure ledger rules are updated for production-derived tasks.
- [ ] Deletion/removal path is documented.

This file is a template for future production-data re-ingestion; it approves
none.
