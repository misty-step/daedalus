# Coding Agent Substrate Premise

Date: 2026-06-19

Source: local pasted report, "Modern coding-agent systems as execution
substrates", current through 2026-06-19.

## Finding

For a code-review platform that should be owned and differentiated, the report
ranks OpenCode ahead of OMP as the execution substrate. The core reason is not
reviewer quality in isolation; it is architecture. OpenCode is server/session
first, which makes concurrent reviewer sessions, structured event collection,
SDK/server integration, retries, model routing, and custom tool policy cleaner
than wrapping a terminal-first local agent.

OMP remains valuable as a local/power-user coding environment and agent-loop
laboratory. It should not be treated as the default durable organization-wide
control plane without further evidence.

## Implication for Threshold

Threshold should evaluate Cerberus reviewer configurations as artifacts from a
substrate-neutral runner contract, not as a frozen OMP topology. The important
candidate axis is now:

```text
master substrate: opencode | omp | codex | claude
composition rule: one master, no predefined reviewers, dynamic lanes allowed
artifact contract: ReviewArtifact.v1
context tier: diff_only | repo_head | repo_base_and_head | local_runtime | remote_runtime
```

Do not reward hardcoded specialist rosters by default. Reward the reviewed
artifact: evidence grounding, false-positive restraint, context truthfulness,
line anchoring, finding usefulness, degraded-state honesty, and receipt quality.

## Arena Guidance

When Threshold builds or refreshes PR-review arenas:

- include OpenCode and OMP as substrate candidates when current tooling is
  available;
- keep model/provider facts in the verified model pool before use;
- treat static reviewer-count strategies as experimental variables, not product
  assumptions;
- score whether the final artifact overstates unavailable context;
- keep "external research" as an explicit capability with citation checks;
- preserve enough command/session trace to let Cerberus runs be replayed.

## Non-Goals

- Do not move Cerberus's runtime decisions into Threshold.
- Do not make Threshold responsible for publishing PR comments.
- Do not assume Cloudflare's seven-reviewer shape generalizes; evaluate whether
  dynamic master-selected lanes outperform simpler runs.
