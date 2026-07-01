# Premise: Threshold PR Review Swarm

Captured: 2026-06-12

Superseded: 2026-06-20 by the Cerberus-first substrate lab direction in
`backlog.d/048-build-cerberus-substrate-rd-lab.md`. Preserve this premise as
history and reusable evaluation context, not as the active product parent.

The operator wants Threshold to shape a program for finding an optimal code
review agent configuration and harness, with "optimal" understood as a
measured result rather than a prompt preference. The desired end state is not
one monolithic reviewer: Olympus and Bitter Blossom should eventually respond
to PR-open events by running a swarm of specialized code-review agents, each
covering a particular review angle, then synthesize their outputs into a
single Master Reviewer agent's review.

Initial role set requested by the operator:

- a general-purpose code reviewer;
- specialized reviewers for particular code-review angles;
- a Master Reviewer, probably itself modeled as another Threshold task spec,
  that consolidates specialist reviews into one coherent review.

Deployment expectation:

- Threshold should define and explore the task specifications and measured
  contracts first;
- Olympus and Bitter Blossom integration should remain sandboxed until human
  launch gates approve a stronger runtime posture.
