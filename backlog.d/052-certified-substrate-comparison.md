# Certified substrate comparison — Pi vs OpenCode vs OMP in one table

Priority: P0 · Status: pending · Estimate: M

## Goal
A single certified search that races the **same** certified composition
(kimi-k2.7-code / trace-callers) across the candidate Cerberus *substrates* (Pi,
OpenCode, OMP harnesses) on the same arena, so the harness choice for Cerberus is
made on a reward-delta CI, not a hunch — directly answering the mission's
"provably most cost-effective high-quality **agent and harness** config."

## Why
The 2026-06-23 certified run proved the **agent** (model + prompt + tools): it
varied model/prompt/tools but every candidate ran on the **Pi** runner. The
mission names *harness* as a first-class axis, and 048 stood up a substrate lab
(Pi vs OpenCode vs OMP) — but that lab is a fixture/smoke comparison, not a
certified table. The harness is a real reward+cost lever (tool-call efficiency,
context handling, retry behavior all differ), and it's currently unmeasured. The
`kind` field on `agent.toml` (`kind = "pi"`) is exactly the slot to mutate.

## Oracle
- [ ] One run varies the substrate (`kind`: pi | opencode | omp) holding the
      certified composition fixed, on `arenas/pr-review-v0` (or 051's arena).
- [ ] The report ranks substrates by reward with cluster-robust 95% CIs and a
      cost-per-trial column; if one substrate is certified better (CI excludes 0
      vs the others) it's named; if indistinguishable, that's stated explicitly
      (a tie is a valid, honest result).
- [ ] `deliveries/cerberus-reviewer/agent.toml` `kind` reflects the measured
      choice (or a recorded "Pi retained — substrates tie within CI" note).
- [ ] `bin/gate` passes.

## Notes
Builds on [[048]] (the substrate lab fixtures + runners). The cheapest run that
closes the biggest remaining gap between the current single-substrate result and
the mission's literal "agent **and** harness" wording. Spend-gated.
