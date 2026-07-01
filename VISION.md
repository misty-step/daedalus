# Threshold Vision

Threshold exists to discover, optimize, and certify agent configurations.

Given a task specification and an eval surface, Threshold searches the large
configuration space that humans otherwise tune by feel: model, provider,
reasoning budget, prompt packet, system-prompt mode, skill set, tool policy,
context briefing, subagent topology, runtime substrate, turn limit, wall-clock
budget, and cost ceiling. The output is not "an agent ran." The output is an
evidence-backed composition and launch contract: what was tried, what won, what
it beat, what it costs, what remains unproven, and what authority it may have.

The heart of the project is a Carpathia-style auto-research loop under evidence.
A high-judgment master agent should be able to look at failures, propose focused
mutations, run candidate compositions under comparable constraints, preserve a
Pareto frontier, and stop when the evidence says the search is saturated,
unreliable, too costly, or not yet meaningful.

## The Split With Crucible

Threshold is no longer trying to become the general eval-design product.

Crucible is the natural home for defining, designing, running, reviewing, and
iterating evals: task sets, fixtures, deterministic graders, rubric/model judges,
human-judgment queues, reports, publications, and the delightful operator UI
for judging outputs. Project-specific evals may still live with the project that
cares about them, and Harness Kit may carry portable eval contracts for its
primitives.

Threshold consumes evals and uses them as the measurement surface for search. It
may run evals, validate eval metadata, and refuse bad or saturated evals, but it
should not grow into the whole eval workbench or benchmark-publishing product.
If an eval is the thing being designed, that is Crucible-shaped work. If an
agent configuration is being optimized against an eval, that is
Threshold-shaped work.

This split should make both projects sharper:

- Crucible asks, "Are we measuring the right thing, in the right way?"
- Threshold asks, "Given that measurement surface, which agent configuration
  actually performs best enough to trust?"

## What Threshold Searches

A Threshold candidate is a composition, not a vague agent persona. The useful
slots are explicit so search can be scientific instead of anecdotal:

- model and provider;
- reasoning effort or thinking budget;
- prompt packet and system-prompt mode;
- skills, tools, context, and MCP/capability surface;
- runtime substrate such as Pi, OpenCode, OMP, Codex, or another runner;
- lead-agent versus specialist-subagent structure;
- critic topology and synthesis rules;
- token, wall-clock, and dollar budgets;
- hard constraints such as "maximize score under $5 per run" or "hit the
  threshold with the cheapest reliable composition";
- output contract, escalation rule, and required evidence.

Threshold should allow creative candidate proposals, but comparisons must remain
interpretable. Single-slot mutations, baselines, incumbents, holdouts,
certification trials, and reliability gates matter because otherwise the project
learns only that "something changed."

## What Must Stay True

- **Agent-vs-agent, not agent-vs-story.** The unit under test is a composition
  that could actually run again, not a prose recommendation.
- **Eval quality gates search.** If the eval is saturated, leaky, underpowered,
  too easy, or misaligned with the task, Threshold stops and sends the problem
  back to eval design instead of optimizing noise.
- **Prove better, not merely different.** A higher mean inside the noise floor
  is not a result. Use baselines, incumbents, confidence intervals, reliability
  floors, and cost/latency envelopes before recommending a composition.
- **Cost and latency are quality.** A composition that is slightly better at
  ten times the price may be wrong. The default output is a Pareto frontier
  across quality, cost, latency, reliability, and authority. A single launch
  winner is chosen only when the task contract supplies a scalar objective,
  threshold, or deployment constraint.
- **Contracts over chat.** Task specs, candidate manifests, run records,
  summaries, reports, launch contracts, approvals, and traces are the durable
  interfaces. A future operator should be able to audit the decision cold.
- **Human gates are real.** Automation prepares evidence for G1-G5; it does not
  self-approve spend, eval trust, deployment, write authority, or production-data
  re-ingestion.
- **Planes own production.** Threshold mints measured compositions and launch
  contracts. Bitter Blossom, Olympus, Cerberus, or another plane owns triggers,
  runtime permissions, posting, rollback, production observability, and policy.

## Why This Exists

The motivating problem is that agent configuration is becoming too high
dimensional for vibes. Changing the model, the prompt, the skills, the tools,
the context, the reasoning budget, or the subagent structure can matter more
than the application code around it. A master orchestrator with the right search
discipline should be able to discover better configurations than a human
hand-tuning one prompt in one chat window.

Threshold is the optimizer for that world. It is how Misty Step can ask,
"What should this agent actually be?" and get back measured evidence rather
than a plausible answer.

The first concrete proving ground remains agentic code review and Cerberus-like
reviewer configurations, because review has real stakes, measurable artifacts,
clear downstream consumers, and enough failure modes to expose bad science.
That focus should not become a cage. Once the search loop is trustworthy,
Threshold should optimize other agent families against credible eval surfaces
designed in Crucible, owned by their projects, or carried by Harness Kit.

## Ideal Form

In the mature version, an operator can point Threshold at a task specification
and an eval package, set risk and budget boundaries, and let the system search
the configuration space with scientific discipline.

An ideal Threshold run produces:

- a frozen task/eval reference and the reason it is trusted enough to search;
- baseline and incumbent measurements;
- candidate compositions with mechanical hashes and provenance;
- comparable run records with cost, latency, artifacts, and failures;
- a report that shows the Pareto frontier, dominated candidates, recommended
  launch choice when one is justified, and why that choice is reliable;
- a launch contract that downstream planes can import without guessing;
- residual risks, unsigned gates, and follow-up eval needs.

The long-term ambition is not a marketplace of agents. It is a lab-grade
optimizer for building trustworthy, task-specific agent configurations.

## What This Repo Refuses

- It is not the general eval authoring product. That is Crucible-shaped.
- It is not a benchmark leaderboard or public score factory.
- It is not a production scheduler, event plane, or permission broker.
- It does not silently mutate global Harness Kit primitives, provider defaults,
  production triggers, or downstream repos.
- It does not promote an offline winner without a human-readable launch
  contract and unsigned-gate disclosure.
- It does not treat model-judge output as the only oracle or cost-free truth.
- It does not continue optimizing when the eval is the bottleneck.

## Where The Depth Lives

- `AGENTS.md` is the repo contract and gate map.
- `DESIGN.md` defines the existing file contracts, pipeline, run records,
  launch contracts, and human gates.
- `docs/operator-sop.md` is the cold-start path for specifying, freezing,
  searching, exporting, and closing a task family.
- `docs/vocabulary.md` names the searched pieces of an agent composition.
- `docs/philosophy.md` carries the older foundry principles that still matter:
  headroom before search, contracts over prose, and human gates.
- `ROADMAP.md` preserves historical phase evidence and should be revised when
  the Threshold/Crucible split changes active milestones.
- `backlog.d/` holds shaped work. The old "rename Threshold to Crucible" ticket
  is superseded by this split: Threshold stays the optimizer; Crucible becomes
  the eval workbench.
