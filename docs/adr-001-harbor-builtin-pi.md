# ADR-001: Use Harbor's built-in pi agent, not a custom adapter

Status: accepted (2026-06-09)

## Context

Backlog 004 planned a custom `PiAgent(BaseInstalledAgent)` adapter so Daedalus
could run pi candidates under Harbor's Docker isolation. On inspecting the
installed Harbor 0.13.1, the framework already ships a first-class `pi` agent
(`harbor.agents.installed.pi.Pi`) that:

- installs pinned pi via nvm/npm in the task container;
- runs `pi --print --mode json --no-session --provider <p> --model <m>`
  with a `--thinking` flag;
- forwards `OPENROUTER_API_KEY` (and other provider keys) when the model id is
  `openrouter/...`;
- parses pi's `message_end` usage events into `AgentContext` cost/tokens —
  the same extraction Daedalus' own runner does.

It also ships `oracle` (replays `solution/solve.sh`) and `nop` reference
agents, matching our null/oracle reference candidates.

## Decision

Do not build a custom adapter. Run pi candidates with Harbor's built-in agent:

```
harbor run -p harbor-build/<arena>/<task> \
  --agent pi -m openrouter/<model> --ak thinking=<level> \
  --ae OPENROUTER_API_KEY=$OPENROUTER_API_KEY -y
```

Daedalus owns the *arena→Harbor port* (`cargo run --quiet --bin daedalus --
port-harbor ...`) and the task format; Harbor owns container lifecycle, the
agent, and reward plumbing. This is the deepest-module / smallest-surface
option: the only Daedalus code is the deterministic port, and the verifier is
the `daedalus-score` binary copied into `tests/`.

## Consequences

- The `prompt_packet` slot maps to Harbor's `--extra-instruction-path` (append
  to instruction) or a skills dir, not `--append-system-prompt`. The
  composition-vs-harness comparison across the local runner and Harbor is
  therefore approximate on the packet axis; record which executor produced a
  result (already in run records via `candidate_kind`).
- Cost attribution under Harbor depends on pi reporting usage for the served
  OpenRouter provider; same caveat as the local runner.
- If we later need a slot Harbor's pi agent doesn't expose (e.g. tool policy
  pinning, env allowlist inside the container), revisit with a thin subclass —
  but only then. Backlog 004's adapter is closed as "not needed".
- Phase 0's local runner stays the fast inner loop (no Docker build per
  trial); Harbor is the isolated, parallel, real-arena execution path.
