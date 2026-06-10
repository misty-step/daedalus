# Loop v2: Pareto-archive parent selection, competing hypotheses per round

Priority: P0
Status: ready
Estimate: M

## Goal
Replace single-incumbent hill-climbing with archive-based search: each
generation samples parents from the Pareto archive (per-task winners eligible,
GEPA-style), proposes ≥2 competing single-slot hypotheses, runs them against
each other, and updates the archive — so the loop races agent configurations
instead of nudging one champion.

## Non-Goals
- Elo/pairwise ranking (irrelevant under deterministic scalar rewards; becomes
  interesting with judge scorers, ticket 010)
- Multi-slot mutations (single-variable discipline stays)

## Oracle
- [ ] Parent selection draws from the Pareto front / per-task winners, not
      only the global best mean (offline test: a candidate that wins one task
      but loses on mean is selectable as a parent)
- [ ] Each generation produces k ≥ 2 children with distinct hypotheses
      (different slots or different parents); both run under equal budgets;
      loop.json records the race outcome per generation
- [ ] `tools` joins MUTABLE_SLOTS, validated against the taskspec's declared
      `tool_policies`
- [ ] Keep/discard is variance-aware: a child only counts as improved when its
      paired per-task comparison clears the observed trial noise, not when the
      mean moves by epsilon (offline test with synthetic noisy rewards)
- [ ] Plateau = no archive improvement for N generations; budget and
      max-candidates stops unchanged; `bin/gate` green

## Notes
Depends on 017 (seeds form the initial archive). Operator direction
2026-06-10: "come up with two hypotheses about two different configurations
and see what their scores are … running them against each other." With
trials=3 on small arenas, mean-reward deltas between *agents* are noise-sized
— the variance-aware keep rule is what makes the race honest. Mutation
machinery (runner/mutate.py validator, child materialization) carries over;
only proposer fan-out and parent policy change.
