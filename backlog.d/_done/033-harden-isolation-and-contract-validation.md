# Harden isolation and contract validation for sensitive arenas

Priority: P1
Status: done
Estimate: XL

## Goal
Move Daedalus from trusted synthetic fixtures toward sensitive or adversarial
arenas by making isolation and contract validation executable instead of
doctrine-only.

## Non-Goals
- Granting production credentials to candidate runs
- Rewriting the runner in Rust before schemas have survived two task families

## Oracle
- [x] Candidate runs that need network, secrets, adversarial fixtures, or user
      data default to Harbor/Docker isolation rather than local temp dirs
- [x] The runner has an executable guard against candidates reading grader
      `tests/` or `solution/` via absolute paths, or else refuses those arena
      risk classes with a clear error
- [x] Launch contracts are schema-validated and approval-validated before any
      runtime import path consumes them
- [x] A Rust contract/schema validator is either implemented after two task
      families or explicitly deferred with the current Python boundary named
- [x] Security posture docs are updated with exact commands and residual risks
- [x] `bin/gate` green

## Children
1. [x] Make Harbor/Docker the default for non-low-risk arenas.
2. [x] Add preflight risk classification that refuses unsafe local execution.
3. [x] Validate launch contracts independently of the Python renderer.
4. [x] Revisit the Rust-kernel trigger once the second task family lands.

## Evidence

- `runner/run.py` now refuses local execution when `[risk]` is missing, when
  `class != "low"`, or when sensitive flags are present, before creating run
  records.
- `runner/run.py` scans candidate-visible instructions, packets, skills,
  `AGENTS.md` overlays, and `environment/` files for absolute paths into
  hidden `tests/` or `solution/` directories before creating run records.
- `runner/launch.py` validates contract.v1 schema, prompt packet, evidence
  paths, G3 state, and exact no-write authority before rendering import
  packets. `runner/export.py` and `deliveries/pr-review/contract.toml` now use
  machine-checkable `write_actions = "none"`.
- `docs/security-posture.md`, `DESIGN.md`, `ROADMAP.md`, `README.md`, and the
  Daedalus skill document the local/Harbor boundary, Python validator, Rust
  deferral trigger, exact commands, and residual risks.
- Focused tests: `python3 -m pytest -q tests/test_run.py tests/test_launch.py
  tests/test_export.py` -> 40 passed.
- Full gate: `bin/gate` -> 122 passed.

## Notes
**Why:** security/privacy lane. `DESIGN.md` explicitly says local Phase 0 runs
cannot prevent absolute-path reads of answer keys; that is fine for synthetic
fixtures, but it is the blocker for broader arenas.
