# Capability-surface search — add an MCP slot, feed skill/MCP sets

Priority: P0 · Status: pending · Estimate: L

> **Reframed 2026-06-24** (was: "certified substrate comparison — Pi vs OpenCode
> vs OMP"). Operator clarified that "agent **and** harness" never meant the
> substrate (Pi/OC/OMP) — that's settled, default OpenCode, and the kernel
> already freezes `kind` by design. It meant the **capability surface**: the
> skills / tools / MCPs / context an agent is outfitted with. See
> [docs/vocabulary.md](../docs/vocabulary.md).

## Goal
Make the capability surface a *fully searchable* axis, so Daedalus can prove how
much outfitting a domain agent with the right capabilities moves
quality-per-dollar — the operator's core thesis (a reviewer with real reviewing
skills/MCPs, a red-teamer with real white-hat tooling, beats the raw model).

## Why
`mutate.rs` `MUTABLE_SLOTS` already varies `tools`, `skills`, `system_prompt_mode`,
and `agents_md` — but two things block the thesis from being measured:
1. **No `mcp` slot exists.** MCP servers — the capability lever the operator most
   believes in — cannot be varied by the search at all. It's the one piece of an
   agent with no slot ([docs/vocabulary.md](../docs/vocabulary.md) table).
2. **The skills axis is wired but unfed.** The certified Cerberus run ran every
   seed with `skill_set_name: null`, so the search had nothing to explore on the
   skills axis. The lever exists; we never loaded the cartridge.

So "how much can capability surface help?" is currently unmeasurable — not for
lack of search machinery, but for lack of declared capability sets to search over.

## Non-Goals
- Racing substrates (Pi/OC/OMP) — settled; `kind` stays frozen.
- Building domain skill/MCP libraries for every plane — this ticket proves the
  *mechanism* on one domain (code review); domain libraries are follow-ups.

## Design (sketch — shape before building)
1. **Add `mcp` to the capability surface.** A new mutable slot mirroring how
   `skills` works: declare named MCP sets per arena/taskspec (like `skill_sets`),
   let the optimizer mutate which set a composition gets, thread it through the
   manifest + export so a certified composition records its MCP loadout. Keep the
   ungameable-grader invariant: candidates still never read `tests/`/`solution/`.
2. **Feed the surface.** Author a real reviewer **skill set** + **MCP set** for
   the pr-review arena (e.g. a code-review skill, a static-analysis/LSP MCP) so a
   search has a non-null capability surface to explore.
3. **Measure.** Run a certified search that varies the capability surface
   (skills/MCPs/tools) on a fixed model+substrate, and report the reward-delta CI
   of "equipped" vs "raw" — the first quantified answer to the thesis.

## Oracle
- [ ] `mcp` is a searchable composition slot: declarable per arena, mutable by the
      optimizer, recorded in the manifest + certified export (parallel to `skills`).
- [ ] The pr-review arena ships a non-empty reviewer skill set + MCP set.
- [ ] A certified search varies the capability surface and reports a reward-delta
      95% CI for equipped-vs-raw on a fixed model/substrate (spend-gated).
- [ ] `daedalus export-cerberus` carries the capability loadout into the
      ReviewerConfigPacket.
- [ ] `bin/gate` passes.

## Verification System
- Claim: outfitting the agent with domain capabilities measurably improves
  quality-per-dollar (and Daedalus can certify by how much).
- Falsifier: an equipped composition scores no better than raw within CI (a real,
  publishable negative result — the thesis would be wrong for this domain).
- Driver: a spend-gated certified search varying skills/MCPs/tools; `arena-validate`
  on the new capability sets.
- Evidence: the run report's CI + the certified packet's recorded loadout.

## Notes
This is the literal completion of the goal's "provably most cost-effective high
quality agent **and** capability config for Cerberus" — the agent (model/prompt)
is proven; this proves the equip. Pairs with [[048]] (substrate lab, now
understood as substrate-not-capability) and the certified run in
`runs/20260623T183514Z-search-cerberus-reviewer`. Operator deferred active work
("thinking about it, not building now") — this captures it as the next major
research investment when greenlit.
