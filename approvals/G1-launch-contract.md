# G1 - Spec sign-off: launch-contract-v0

- **Status:** pending human review
- **Spec:** `specs/launch-contract/taskspec.toml`
- **Arena:** `arenas/launch-contract-v0`
- **Prepared:** 2026-06-12

## Scope

This second Daedalus task family reviews control-plane launch contracts and
import packets before deployment gates consume them. It is deliberately not a
PR-review arena: workspaces contain TOML/Markdown policy artifacts, and the
defect taxonomy is about approval gates, evidence traceability, permissions,
observability, and portability.

Approved execution posture, if G1 is signed:

- Mode: `threshold-then-cheap`.
- Budget: max `$0.35` per trial and `420s` wall time per trial.
- Search space: five OpenRouter model slots, three packet stances, and
  `off`/`low`/`medium` thinking only. `high` is intentionally excluded for
  the first run because these fixtures are compact contract artifacts and the
  ticket is testing domain transfer, not maximum reasoning spend.
- Required next gate before trusting scores: G2 eval-quality review after
  oracle/null/probe validation.

## Human Decision Needed

Approve or reject spending model budget on `launch-contract-v0` as the second
task family for ticket 030. Approval covers offline synthetic fixtures only;
runtime deployment remains gated by each delivery's G3/G4/G5 artifacts.

## No-Spend Rig Evidence

- Oracle/null run directory:
  `runs/20260612T000000Z-freeze-launch-contract-v0`
- Oracle mean: 1.0 across all six launch-contract tasks.
- Null mean: 0.1667, matching the one clean sandbox task out of six.
- One-shot probe and comparative search are intentionally not run until this
  G1 packet is approved.

This file is not an agent self-approval.
