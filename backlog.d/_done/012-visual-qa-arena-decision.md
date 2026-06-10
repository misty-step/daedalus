# Decide: run-the-app / visual QA arena (spike, then go/no-go)

Priority: P2
Status: pending
Estimate: S

## Goal
A one-day spike that determines whether a run-the-app arena (agent launches an app in the task container, exercises it, reports defects) is buildable with deterministic-enough oracles — producing a go/no-go ADR, not an arena.

## Non-Goals
- Building the arena (that is its own L+ ticket if "go")
- Screenshot-judge subjectivity as the primary oracle

## Oracle
- [ ] Spike report answers: can app-state assertions (HTTP probes, DB state, DOM checks via Playwright in-container) give Harbor-style 0/1 rewards? What does one task cost to author? What does Harbor's container model add/block?
- [ ] ADR recorded: go (with scope) or no-go (with revisit trigger)

## Notes
Operator wants this direction; pi lane's cut list argues the second family
should keep deterministic oracles. The spike resolves the disagreement with
evidence instead of debate. Depends: 004.
