# G3 - Launch review: g2b-g1a-seed2-glm-4-7-flash-spec-first

- **Status:** pending
- **Agent:** `g2b-g1a-seed2-glm-4-7-flash-spec-first`
- **Composition hash:** `7523f6b853908df2`
- **Contract:** `deliveries/launch-contract/contract.toml`
- **G2:** `approvals/G2-launch-contract-v0.md`

## Current Decision

Not approved for deployment.

The G2 packet is still pending human review. Before this file can be marked
approved, a human reviewer must choose the target plane, runtime boundary,
posting or artifact boundary, observability target, rollback behavior, and
regression cadence.

## Required Before Approval

- [ ] Human G2 accepts `launch-contract-v0` or records an explicit waiver.
- [ ] Import dry run preserves the measured prompt packet byte-for-byte.
- [ ] `deliveries/launch-contract/contract.toml` has a non-unknown harness
      version, evidence pointers, and a trace destination or explicit waiver.
- [ ] Target plane treats the run as secondary/sandboxed unless a later human
      approval explicitly promotes it.
- [ ] No direct approve/merge/code-edit or production write authority is
      granted.
- [ ] Regression replay command and trace artifact are reviewed.
- [ ] G4 and G5 boundaries are acknowledged.

This file is a pending launch gate, not an agent self-approval.
