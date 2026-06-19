# ADR-004: Review-swarm contract v1 and suite export

Status: accepted (2026-06-12) - backlog 034

## Context

The single-agent `contract.v1` works for one measured composition. The review
swarm needs a higher-level artifact: multiple member agents, a master
synthesis stage, suite-level thresholds, total cost/wall accounting, and a
handoff mode that can distinguish a full swarm from member-only inspection.

The target planes also differ. Bitter Blossom is event-plane shaped and has
historically allowed direct posting from a review card; Olympus Charon writes a
strict artifact and the orchestrator validates/post posts. The swarm contract
must preserve the safer common boundary: member agents write artifacts only,
and a control plane owns any eventual posting after G3.

## Decision

Introduce `swarm-contract.v1` and `cargo run --quiet --bin daedalus -- export-suite`.

- `cargo run --quiet --bin daedalus -- export-suite <delivery> --suite <taskspec>` reads
  `<delivery>/summary.json`, validates the summary against the suite taskspec,
  and writes `<delivery>/swarm-contract.toml` plus a human-readable
  `plane-handoff.md`.
- `summary.json` is the evidence boundary. Export refuses to fabricate suite
  cost, wall time, handoff mode, or master real-member replay status.
- `cargo run --quiet --bin daedalus -- launch-pack <delivery> --plane ... --dry-run` recognizes
  `swarm-contract.toml` and emits sandbox-only import packets while G3 is
  unsigned.
- Non-dry-run suite packets require G3 just like single-agent launch
  contracts.

## Invariants

- Member agents do not post comments.
- Full-swarm handoff requires master replay against real member artifacts.
  Otherwise export may only produce `member-only` inspection handoff.
- Suite candidates over the cost or wall-time envelope need explicit waivers
  before sandbox recommendation.
- G4 remains required before any production write authority.
- G5 remains required before production traces or PR data become arena
  fixtures.

## Rejected Alternatives

- **Fold members into `contract.v1`.** Rejected because single-agent fields
  such as `agent`, `composition_hash`, and `prompt_packet` become ambiguous.
- **Let planes assemble the suite themselves.** Rejected because Daedalus would
  lose the measured composition and threshold accounting at the exact import
  boundary.
- **Export a deployable swarm while G3 is unsigned.** Rejected by the existing
  launch-gate doctrine.

## Verification

```sh
bin/gate
```
