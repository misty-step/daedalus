# G3 - Launch review: seed4-qwen3-7-plus-checklist

- **Status:** pending
- **Agent:** `seed4-qwen3-7-plus-checklist`
- **Composition hash:** `4a73f1fd213aa1a5`
- **Contract:** `deliveries/pr-review/contract.toml`
- **G2:** `approvals/G2-pr-review-v2.md`

## Current Decision

Not approved for deployment.

The G2 waiver accepts this packet only for Daedalus contract-discovery work
and sandboxed plane experiments. Before this file can be marked approved, a
human reviewer must choose the target plane, runtime boundary, posting
boundary, observability target, rollback behavior, and regression cadence.

## Required Before Approval

- [ ] Import dry run preserves the measured prompt packet byte-for-byte.
- [ ] `deliveries/pr-review/contract.toml` has a non-unknown harness version,
      evidence pointers, and a trace destination or explicit waiver.
- [ ] Target plane treats the run as secondary/sandboxed unless a later human
      approval explicitly promotes it.
- [ ] No direct approve/merge/code-edit authority is granted.
- [ ] Regression replay command and trace artifact are reviewed.
- [ ] G4 and G5 boundaries are acknowledged.

This file is a pending launch gate, not an agent self-approval.
