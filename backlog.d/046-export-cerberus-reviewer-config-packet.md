# Export Cerberus reviewer config packets

Priority: P1
Status: pending
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

- [ ] `daedalus` can emit a `ReviewerConfigPacket.v1` JSON artifact from a
      measured review delivery.
- [ ] The exporter preserves delivery id, composition hash, prompt hash,
      run id, arena version, score distribution, measured cost/wall envelope,
      G2/G3/G4/G5 evidence, and sandbox-only state.
- [ ] The embedded `ReviewConfig.v1` has a deterministic config hash that
      matches Cerberus' serialized JSON digest.
- [ ] The checked fixture validates with
      `cerberus-cli validate-reviewer-config`.
- [ ] The checked fixture dry-run imports with
      `cerberus-cli import-reviewer-config --dry-run` and records rejection
      reasons while G3/G4/G5 remain unsigned.
- [ ] `bin/gate` green.

## Notes

The first target should be the existing `deliveries/pr-review` evidence packet.
If Daedalus exports a multi-agent review swarm, emit either one Cerberus packet
per reviewer role or an explicitly modeled suite packet only after Cerberus has
a matching schema. Do not squeeze suite semantics into a single unscored
reviewer config.
