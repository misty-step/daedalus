# Stop leaking the operator environment into candidate subprocesses

Priority: P0
Status: ready
Estimate: S

## Goal
Candidate subprocesses receive only an explicit env allowlist (default: `OPENROUTER_API_KEY` only), so a compromised or curious candidate cannot read GITHUB_TOKEN, other provider keys, or shell state.

## Non-Goals
- Full sandboxing (that is Harbor/Docker, ticket 004)

## Oracle
- [ ] `run_pi` passes `env=` built from `candidate.env_allowlist` (manifest field, defaulted), not inherited environ
- [ ] A probe candidate that runs `env` in bash sees only the allowlisted variables (test asserts GITHUB_TOKEN absent)
- [ ] pi-kimi rerun still records tokens/cost correctly

## Notes
Security lane: live today — pi inherits the full parent env including
unrelated production keys. Smallest-possible blast-radius fix before Docker.
