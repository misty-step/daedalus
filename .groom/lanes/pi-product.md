## Premise verdicts

1. **Reframe.** More-realistic arenas and a sweep engine are not the shortest path. The binding constraint is that the master-agent "loop" is a manual protocol (`SKILL.md`) executed by a human prompting Claude interactively. Arena authoring cost is real but secondary: you cannot amortize arena investment until a single command can autonomously iterate clarify → validate → baseline → generate → run → score → reflect.
2. **Hold.** Trust in scores (G2 unsigned) is a symptom, not the root cause. An autonomous loop would surface meta-eval gaps as routine output; today they are blocked on the loop not existing.

## MVP user session

Operator runs `daedalus run specs/foo.toml`. T+5 min: G1 clarifying interview done. T+15 min: arena and scorer drafted. T+25 min: oracle/null validate the rig. T+35 min: baseline and first candidate run complete. T+45 min: **session breaks**—the operator must manually read transcripts and prompt the master agent for the next candidate. There is no headless loop, no Pareto archive automation, no stop-condition logic. The missing piece is the iterated search runner, not bigger fixtures.

## Ideal form

A "compiler for agents": paste a task description, answer 3 clarifying questions, and receive a tested agent package with cost/latency estimates, a signed launch contract, and a regression suite in under 30 minutes. Daedalus owns the lab; control planes own runtime.

## Cut list

- Visual QA as the next domain (too subjective; second family should have deterministic oracles)
- Full agent-vs-agent composition sweep engine (start with single-slot mutation only)
- Production scheduler / reference deploy (emit a contract; let Olympus/Bitter Blossom handle triggers)
- GEPA library, Rust kernel, and multi-objective optimizer (defer until the hand-rolled loop plateaus)

## Single recommendation

**Goal:** Automate the master-agent loop end-to-end. **Oracle:** One CLI command, `daedalus run specs/pr-review/taskspec.toml`, executes clarify → arena → validate → baseline → generate → run → score → reflect → iterate without human prompting, stops at budget/plateau, and emits a Pareto archive of ≥3 candidates with JSONL run records where the best candidate beats the one-shot baseline on the taskspec's declared objective.
