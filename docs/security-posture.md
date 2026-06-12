# Security Posture

Daedalus has two execution boundaries:

- The fast local runner, `runner/run.py`, is for low-risk offline synthetic
  arenas only.
- Harbor/Docker, via `bin/harbor-run`, is the isolation boundary for arenas
  that need network access, production or user data, secret-bearing workflows,
  adversarial fixtures, or untrusted candidate compositions.

## Local Runner Preconditions

Every local run loads `arenas/<id>/arena.toml` before creating run records. It
fails closed when `[risk]` is missing, and refuses any arena whose `[risk]`
table declares one of these fields:

```toml
[risk]
class = "sensitive"        # anything except "low" is refused locally
needs_network = true
needs_secrets = true
adversarial_fixtures = true
user_data = true
```

The refusal is intentional. Do not lower risk metadata to get a run through
the local path; port the arena and use Harbor:

```sh
python3 runner/port_harbor.py arenas/<arena-id>
bin/harbor-run arenas/<arena-id> all --agent pi -m openrouter/<model>
```

The local runner also rejects fixture symlinks and candidate-visible absolute
paths into a task's hidden `tests/` or `solution/` directories before it creates
`runs/<exp-id>/`. It scans the rendered instruction, prompt packet, skills,
`AGENTS.md` overlay, and `environment/` files. This prevents accidental grader
path leaks in low-risk arenas.

## Launch Contract Validation

`bin/daedalus launch-pack <delivery> --plane <plane>` validates
`contract.toml` before rendering any import packet. The validator checks:

- contract version and required identity fields;
- composition fields and prompt-packet existence;
- permissions, budget, observability, evidence, and approval tables;
- G3 approval state for deployable packets;
- signed G4 approval file before any contract grants production write
  authority.

Unsigned contracts may still render sandbox-only packets:

```sh
bin/daedalus launch-pack deliveries/pr-review \
  --plane bitter-blossom --dry-run
```

Deployable import packets require a human-signed G3 approval file. G4 remains
required before production write authority, and G5 remains required before
production data flows back into arena fixtures.

## Residual Risks

The local runner still executes candidates with the user's account-level file
permissions. It now rejects visible grader-path leaks, but it cannot prevent a
malicious tool-using candidate from guessing host absolute paths. That is why
any sensitive, adversarial, user-data, secret-bearing, or network-dependent
arena must use Harbor/Docker isolation.

The launch validator is currently Python (`runner/launch.py`). A Rust
contract/schema validator is deferred until schemas survive two accepted task
families or an external control plane starts consuming contracts as a hard
runtime dependency, whichever comes first.

## Verification Commands

```sh
python3 -m pytest -q tests/test_run.py tests/test_launch.py
bin/gate
```
