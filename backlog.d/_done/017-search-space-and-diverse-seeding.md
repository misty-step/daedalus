# Declare the search space in the taskspec; seed a diverse agent population

Priority: P0
Status: ready
Estimate: M

## Goal
Replace the hardcoded two-manifest baseline pair with a `[search]` section in
the taskspec (allowed models, packet seeds, thinking levels, temperature
range, tool policies, seed count) and a seeder that samples N diverse *agent*
configurations from that space — the broad landscape scan that the iterative
search then exploits.

## Non-Goals
- Unfreezing the harness slot (pi stays the only executor in V1)
- Archive-based search itself (ticket 018 consumes the seeds)

## Oracle
- [ ] taskspec schema gains `[search]`: `models` (OpenRouter ids), `packet_seeds`
      (count or dir), `thinking_levels`, `temperature_range`, `tool_policies`
      (named pi tool subsets), `seed_count`; G1-validated before spend
- [ ] Seeder emits `seed_count` hashed composition manifests, all `kind = "pi"`,
      spanning the declared space (scalar slots sampled deterministically with
      a recorded seed; packet variants generated as k distinct review stances
      by the optimizer model and saved as versioned packet files)
- [ ] `bin/threshold` stage 2 runs all seeds on train and the report shows the
      landscape: per-seed reward/cost/latency spread, not a two-row table
- [ ] Budget math is explicit: seeding cost (seeds × tasks × trials) is
      estimated up front against `--budget-usd` and the seeder shrinks
      `seed_count` rather than blowing the budget
- [ ] No hardcoded `BASELINES` list remains in bin/threshold; tests cover the
      seeder offline (sampling + manifest materialization, no network)

## Notes
Operator direction 2026-06-10: "roll the dice pretty randomly in terms of the
system prompt and the model used … get a broad sense of the landscape, then
start iterating and tuning." Seeds are the population for 018's archive.
Diversity beats optimality here — distinct models × distinct packet stances ×
distinct thinking budgets, not five near-clones. Keep the sampler boring and
recorded (seeded RNG in the run record) so a landscape scan is reproducible.
