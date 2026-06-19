# Export Cerberus reviewer config packets

Priority: P1
Status: done
Estimate: M

## Goal

Let a measured Daedalus review delivery emit the Cerberus handoff artifact that
Cerberus actually imports: `ReviewerConfigPacket.v1` JSON with an embedded
`ReviewConfig.v1`, benchmark evidence, model/harness metadata, prompt hashes,
cost envelope, rollback metadata, and G2/G3/G4/G5 gate state.

Cerberus is not a generic control-plane `launch-pack` target. A TOML import
packet that only names the delivery and prompt hash is useful orientation, but
it is not enough for Cerberus reviewer-config promotion.

## Why Now

Cerberus now has the import boundary:

- `cerberus-cli validate-reviewer-config <packet>`
- `cerberus-cli import-reviewer-config <packet> --dry-run`
- sandbox review execution through `--config-packet`

Daedalus has measured PR-review delivery evidence and human G2/G3 gate records.
The missing seam is an exporter that maps those facts into the Cerberus packet
without inventing approval, changing Cerberus defaults, or granting production
posting authority.

## Non-Goals

- Deploying the reviewer in Cerberus.
- Marking G3, G4, or G5 approved.
- Mutating Cerberus defaults or caller policy.
- Treating a raw Daedalus benchmark win as sufficient promotion evidence.
- Replacing Bitter Blossom or Olympus launch-pack exports.

## Oracle

- [x] `daedalus` can emit a `ReviewerConfigPacket.v1` JSON artifact from a
      measured review delivery.
- [x] The exporter preserves delivery id, composition hash, prompt hash,
      run id, arena version, score distribution, measured cost/wall envelope,
      G2/G3/G4/G5 evidence, and sandbox-only state.
- [x] The embedded `ReviewConfig.v1` has a deterministic config hash that
      matches Cerberus' serialized JSON digest.
- [x] The checked fixture validates with
      `cerberus-cli validate-reviewer-config`.
- [x] The checked fixture dry-run imports with
      `cerberus-cli import-reviewer-config --dry-run` and records rejection
      reasons while G3/G4/G5 remain unsigned.
- [x] `bin/gate` green.

## Implementation Receipt

Delivered 2026-06-19:

- Added rendered plan:
  `docs/046-cerberus-reviewer-config-export-plan.html`.
- Added `daedalus_core::cerberus`, a narrow downstream export view over an
  existing measured delivery. The module reads `agent.toml`, `contract.toml`,
  committed run summaries, the arena version, and approval files, then emits
  `ReviewerConfigPacket.v1` JSON without approving deployment or changing
  Cerberus defaults.
- Added CLI:
  `cargo run --quiet --bin daedalus -- export-cerberus deliveries/pr-review --spec specs/pr-review/taskspec.toml --out deliveries/pr-review/cerberus-reviewer-config.json`.
- Added checked packet:
  `deliveries/pr-review/cerberus-reviewer-config.json`.
- Packet facts:
  - producer: `daedalus`, `sandbox_only=true`
  - delivery: `deliveries/pr-review`
  - composition hash: `4a73f1fd213aa1a5`, embedded in the packet id and
    promotion rationale
  - benchmark: `pr-review-v2`, arena version `0.2.0`, run
    `runs/20260611T173632Z-search-pr-review-v0`, 10 arena tasks
  - score summary: min `0.0`, mean `0.5714`, median `1.0`, max `1.0`,
    certified trials `5`
  - model/harness: `pi` `0.78.1`, OpenRouter `qwen/qwen3.7-plus`
  - prompt hash:
    `sha256:4ce0f7d61af3b5b3ac6f58db7dae9e1e9278a61d169249ce8e932e3711eb9198`
  - promotion gates: G2 `waived`; G3, G4, and G5 `pending`
- Cerberus validation proof:
  `cargo run --locked -q -p cerberus-cli -- validate-reviewer-config /Users/phaedrus/Development/daedalus/deliveries/pr-review/cerberus-reviewer-config.json`
  returned `ok`.
- Cerberus import proof:
  `cargo run --locked -q -p cerberus-cli -- import-reviewer-config /Users/phaedrus/Development/daedalus/deliveries/pr-review/cerberus-reviewer-config.json --dry-run --out tmp/daedalus-cerberus-export-2026-06-19/import-report.json`
  accepted dry-run comparison, refused production import, and recorded:
  `packet is sandbox-only; dry-run comparison only` plus
  `promotion gate status is sandbox_only, not approved`.
- Focused Daedalus proof:
  `cargo test -p daedalus-core cerberus -- --nocapture`.
- Repo gate proof: `bin/gate`.
- Closeout: moved to `_done/` in the commit that closes it.

## Notes

The first target should be the existing `deliveries/pr-review` evidence packet.
If Daedalus exports a multi-agent review swarm, emit either one Cerberus packet
per reviewer role or an explicitly modeled suite packet only after Cerberus has
a matching schema. Do not squeeze suite semantics into a single unscored
reviewer config.
