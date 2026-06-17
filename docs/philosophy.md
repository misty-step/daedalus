# Daedalus Philosophy

Daedalus is a lab for measured agent craft. It exists to turn a task into a
focused agent with evidence, not to generate prompts that feel convincing.

## Vision

The highest-quality version of Daedalus is a disciplined agent foundry:

- an operator states a task and its risk boundary;
- Daedalus turns that task into a frozen arena, candidate search space, and
  launch contract;
- the system compares agent compositions by records, not vibes;
- a human reviews eval quality before trusting scores;
- downstream planes such as Olympus and Bitter Blossom import only signed,
  sandbox-aware contracts and keep live permissions under their own control;
- production evidence can later improve the lab only through explicit G5
  re-ingestion.

The product is not the model call. The product is the evidence-backed contract
that says why this agent should exist, what it may do, and what remains
unproven.

## Principles

- **Agent-vs-agent, never agent-vs-story.** References such as oracle, null,
  and one-shot probes calibrate the rig; they are not deliverable agents.
- **Headroom before search.** An arena that a one-shot can saturate cannot rank
  agents. Fix the arena before spending search budget.
- **Contracts over prose.** Specs, arenas, manifests, run records, launch
  contracts, and approval files are the interfaces. Narrative explains them; it
  does not replace them.
- **Human gates are real.** G1 approves spend, G2 trusts the eval, G3 launches,
  G4 grants write authority, and G5 lets production data back into fixtures.
  Automation can prepare a gate; it cannot self-approve one.
- **Cost and latency are part of quality.** A powerful agent that is too slow,
  too expensive, or too opaque is not a good deployment candidate under a
  threshold-then-cheap task.
- **The plane owns production trust.** Daedalus recommends measured contracts.
  Olympus, Bitter Blossom, or another control plane owns triggers, posting,
  permissions, dedupe, rollback, and operator-visible state.
- **No hidden grader leakage.** Candidates never read `tests/` or `solution/`,
  and experiments fail closed when fixture or path boundaries are suspect.
- **Evidence survives the session.** Run records, reports, traces, approvals,
  and handoffs must be durable enough for a future operator to audit without
  reconstructing the chat.
- **Strong claims require adversaries.** Important packets need fresh-context
  critique and explicit residual risk. The author's confidence is not a gate.

## Review-Swarm Implication

For PR review, the best system is unlikely to be one heroic reviewer. It is a
measured review organization: specialists create coverage, a master reviewer
creates restraint and synthesis, and the control plane posts one validated
review. Daedalus should optimize that organization only where the suite
contract, taxonomy, cost envelope, latency envelope, and human gates are
explicit.
