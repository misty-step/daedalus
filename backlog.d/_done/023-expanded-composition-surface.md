# Expand the composition surface: system prompt mode, skills, AGENTS.md

Priority: P1
Status: ready
Estimate: M

## Goal
The master agent can manipulate everything that shapes a pi agent's
behavior: full system-prompt replacement vs append, pi skills, a workspace
AGENTS.md, tools, model, and reasoning budget — all as typed, hashed,
mutable slots drawn from the taskspec search space.

## Non-Goals
- Unfreezing the harness kind (pi stays V1)
- Slots pi cannot express (temperature/max_tokens stay frozen out)

## Oracle
- [ ] New slots in composition.v1, hashed and validated: `system_prompt_mode`
      (replace → `--system-prompt`, append → `--append-system-prompt`),
      `skills` (list of packet-like skill files → `--skill`, with
      `--no-skills` dropped when present), `agents_md` (file written into
      the workspace root; runner drops `--no-context-files` when set)
- [ ] Seeder can sample the new axes when the taskspec declares them;
      mutation validator accepts them as single-slot changes
- [ ] docs/primitives.md documents the new slots and their pi flags
- [ ] Offline tests: manifest roundtrip per slot; runner composes the right
      pi argv (no network); hash changes when any new slot's content changes
- [ ] `bin/gate` green

## Notes
Operator direction 2026-06-10: even with pi frozen, the system prompt, the
tools and skills available, what AGENTS.md articulates, the model and its
reasoning budget must all be searchable. pi 0.78.1 flags verified: 
--system-prompt, --append-system-prompt, --skill, --no-skills,
--no-context-files, --tools, --thinking.
