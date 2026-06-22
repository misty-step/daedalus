# Daedalus — Vision

Daedalus is the foundry where a task becomes a *certified* agent. An operator
states a task and its risk boundary; Daedalus freezes an arena, searches focused
agent compositions against it, **proves** which one is better (not merely ranks
them), and emits a signed, sandbox-aware **launch contract**. The product is
never the model call — it is the evidence-backed contract that says why this
agent should exist, what it may do, and what remains unproven.

It is the long-lived certification engine for the misty-step constellation.
Cerberus, Olympus, and Bitter Blossom import Daedalus contracts and own
production trust — triggers, permissions, posting, rollback. **Daedalus mints;
the planes deploy.** Without it, those planes ship agents chosen by vibes:
uncertified, with no headroom check, no cost-bounded Pareto, no audit trail.
Daedalus is the gate between "an agent ran" and "an agent earned the right to
run in production."

## What must stay true

The load-bearing commitments. `docs/philosophy.md` carries the full principle set.

- **Agent-vs-agent, never agent-vs-story.** Oracle, null, and one-shot probes
  calibrate the rig; they are never deliverable candidates. A one-shot that ties
  the oracle means the *arena* is saturated, not that the baseline won.
- **Prove better, don't just rank.** A win is a candidate whose reward-delta 95%
  CI clears the floor under cluster-robust statistics — the certification layer
  (CIs, pass^k, power) the commercial eval tier omits and Daedalus renders. A
  higher mean inside the noise is not a win.
- **Headroom before search.** An arena a one-shot can saturate cannot rank
  agents. Fix the arena before spending search budget.
- **Contracts over prose; evidence survives the session.** Specs, arenas, run
  records, launch contracts, and approvals are the interfaces — durable enough
  for a future operator or a cold agent to audit without reconstructing the chat.
- **Human gates are real (G1–G5).** Automation prepares a gate; it never
  self-approves one.
- **Cost and latency are quality.** A candidate that scores 3% higher at 10× the
  cost is wrong for a threshold-then-cheap task.
- **The plane owns production trust.** Daedalus recommends measured contracts; it
  does not deploy, schedule, post, or hold live permissions.

## The bet

That a high-judgment master agent can convert a task specification into a
cheaper, faster, narrower, better-tested focused agent — and that *statistical
certification plus signed contracts* are what make agent deployment trustworthy
at constellation scale. The durable edge is the rigor layer: a foundry that can
prove a win, draw its confidence interval, and refuse to ship one it cannot
bound. The clever part is the master agent; the harness stays boring.

## What excellent looks like

- **Now:** the review-swarm family (PR review) is genuinely non-saturable, and a
  certified reviewer composition ships to Cerberus as a signed contract — with
  its CI, cost envelope, and residual risk *drawn*, not asserted.
- **Next:** more than one task family (review, backlog grooming, ops), and the
  open substrate questions — which harness for supervised vs unsupervised/reflex
  agents — answered by evidence in the lab, not by argument.
- **Long horizon:** any plane can import a Daedalus contract and trust it cold;
  production traces flow back through G5 to sharpen the arenas; the foundry — not
  the operator's memory — is where agent quality compounds.

## What this repo refuses

- Not a universal "make me an agent" button, an agent marketplace, or a
  production scheduler. The planes own deploy, triggers, permissions, posting,
  and rollback.
- No offline winner promotes itself to production; no agent gets runtime
  authority without a human-readable launch contract.
- Candidates never read `tests/`, `solution/`, or answer keys; experiments fail
  closed when a boundary is suspect. A benchmark you can game is a benchmark you
  will game.
- LLM-as-judge is never the only oracle; no model default from vibes; no cost or
  latency claim without recorded usage or an honest "unknown."
- The experiment loop never silently mutates global skills, shared provider
  state, or production triggers.

## Where the depth lives

- `docs/philosophy.md` — operating principles and the review-swarm shape.
- `DESIGN.md` — the six-stage pipeline and the file contracts that are the real
  interfaces (Daedalus owns Specify→Lab→Contract; planes own Deploy→Observe).
- `ROADMAP.md` — phases, gated by evidence, not dates.
- `AGENTS.md` — the standing repo contracts and gates.
- `README.md` — the original research framing and external inspirations
  (Karpathy autoresearch, Inspect AI, GEPA, SWE-bench).
