# G4 - Production write-authority review: pr-review

- **Status:** pending
- **Scope:** PR-review agent write authority

## Current Decision

No production write authority is approved.

Before any PR-review agent can post comments, approve reviews, merge code,
open commits, edit branches, or mutate repository state outside an isolated
artifact workspace, a human reviewer must approve the exact action class,
target repositories, rate limits, rollback path, and audit log destination.

## Required Before Approval

- [ ] Exact write actions are enumerated.
- [ ] Target repositories and branch protections are named.
- [ ] Output validation and duplicate suppression are enforced outside the
      agent.
- [ ] Audit logs include agent id, composition hash, input ref, output ref,
      posting actor, and control-plane decision.
- [ ] Emergency disable path is documented and tested.

This file is a template for future write authority; it grants none.
