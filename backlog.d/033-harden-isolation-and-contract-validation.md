# Harden isolation and contract validation for sensitive arenas

Priority: P1
Status: pending
Estimate: XL

## Goal
Move Daedalus from trusted synthetic fixtures toward sensitive or adversarial
arenas by making isolation and contract validation executable instead of
doctrine-only.

## Non-Goals
- Granting production credentials to candidate runs
- Rewriting the runner in Rust before schemas have survived two task families

## Oracle
- [ ] Candidate runs that need network, secrets, adversarial fixtures, or user
      data default to Harbor/Docker isolation rather than local temp dirs
- [ ] The runner has an executable guard against candidates reading grader
      `tests/` or `solution/` via absolute paths, or else refuses those arena
      risk classes with a clear error
- [ ] Launch contracts are schema-validated and approval-validated before any
      runtime import path consumes them
- [ ] A Rust contract/schema validator is either implemented after two task
      families or explicitly deferred with the current Python boundary named
- [ ] Security posture docs are updated with exact commands and residual risks
- [ ] `bin/gate` green

## Children
1. Make Harbor/Docker the default for non-low-risk arenas.
2. Add preflight risk classification that refuses unsafe local execution.
3. Validate launch contracts independently of the Python renderer.
4. Revisit the Rust-kernel trigger once the second task family lands.

## Notes
**Why:** security/privacy lane. `DESIGN.md` explicitly says local Phase 0 runs
cannot prevent absolute-path reads of answer keys; that is fine for synthetic
fixtures, but it is the blocker for broader arenas.
