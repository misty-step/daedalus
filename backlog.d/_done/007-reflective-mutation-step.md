# Build the reflective mutation step (the search brain)

Priority: P1
Status: ready
Estimate: M

## Goal
A bounded LLM step that reads the incumbent composition, its worst-trial transcripts, and the Pareto archive, then proposes exactly one single-slot mutation with a written hypothesis — the unit the loop driver (005) calls per iteration.

## Non-Goals
- Multi-slot or architectural mutations (explicitly out, v1)
- GEPA library adoption (only if this hand-rolled step plateaus)

## Oracle
- [ ] Input contract: composition + N worst trial transcripts + archive summary; output contract: child composition manifest + `hypothesis` string + `slot_changed`
- [ ] Validator rejects proposals that mutate ≠1 slot, touch frozen slots (harness in V1), or exceed budget fields
- [ ] On the pr-review arena, three consecutive mutation rounds produce ≥1 child that beats its parent's mean reward on the validation split
- [ ] Mutation prompt template is a versioned file (hash in run records)

## Notes
VeRO findings imported: optimizers bias to prompt-tweaks (fine — prompt packet
IS our v1 mutable surface) and constrained single-variable templates trade
peak gains for stability — we take stability for MVP. Decagon-style
test-driven GEPA noted as the upgrade path.
