# Superseded: Rebrand Daedalus to Crucible

Priority: P2 · Status: killed · Estimate: L

Closed without implementation on 2026-06-28. The operator decided not to rename
Daedalus. Instead, Daedalus keeps the agent-configuration search and
certification role, while a new `crucible` repo owns eval design, run
measurement, reporting, iteration, and human judgment.

Keep this file as the decision record for why the mechanical rename should not
be picked back up from old context.

## Superseding Decision

- Daedalus remains the optimizer: it searches agent configurations against
  trusted evals and emits evidence-backed composition/launch contracts.
- Crucible becomes the eval workbench: it defines, designs, runs, reviews, and
  iterates evals, including deterministic graders, model judges, human
  judgment, and report surfaces.
- The right move is two repos, not a rename.

> Orthogonal to epic [[054]] — a rebrand neither advances nor blocks the hum bar.
> Per the Cerberus-first mandate it is not "humming" work; do it opportunistically
> or after the reviewer hums, operator's timing call. Filed now so the name debt
> doesn't compound as surface accretes.

## Original Goal
Rename the project from **Daedalus** to **Crucible** across every owned surface —
crate names, the CLI binary, docs, VISION, AGENTS, and config — in one coherent
sweep, without breaking the build, the gate, or exported contracts.

## Original Why
Operator chose "Crucible" as the project name (2026-06-25). The metaphor fits the
product better than Daedalus: a crucible is where raw material is tested under
heat until only the proven survives — which is exactly the foundry's job
(arena → search → certify → contract). The longer the rename waits, the more
run records, deliveries, and cross-project references (Cerberus/Bitterblossom
imports) accrue the old name.

## Original Oracle
- [ ] The CLI binary is `crucible` (was `daedalus`); `crucible doctor` and the
      core subcommands run.
- [ ] Crates rename (`daedalus-core` → `crucible-core`, etc.); `cargo build
      --workspace` and `bin/gate` are green.
- [ ] `VISION.md`, `AGENTS.md`, `README*`, `DESIGN.md`, `ROADMAP.md`, and
      `docs/**` say Crucible; no stray "Daedalus" except in dated historical run
      records / changelog entries that are deliberately preserved.
- [ ] A decision is recorded for each boundary that *can't* be renamed unilaterally:
      the git repo / local checkout dir, exported packet schema names
      (`ReviewerConfigPacket.v1`), run-dir naming, and the names cross-project
      consumers (Cerberus, Bitterblossom) import by — rename, alias, or
      deliberately keep, each with a reason.
- [ ] `rg -i daedalus` returns only the deliberately-preserved set above.

## Original Verification System
- Claim: the project is consistently "Crucible" with no broken references.
- Falsifier: the build/gate breaks, a consumer's import path dangles, or
  `rg -i daedalus` surfaces an unaccounted reference.
- Driver: `cargo build --workspace`, `bin/gate`, `rg -i daedalus`, and a smoke of
  `crucible doctor` + one search subcommand.
- Grader: green gate + the residual-reference audit reduced to the preserved set.
- Evidence packet: the rename diff, the gate output, and the rg audit.
- Cadence: once; it's a mechanical sweep with a few judgment calls at the seams.

## Original Notes
- Mostly mechanical, but the **seams carry the risk**: anything another repo
  imports by name (packet schemas, the binary name in Cerberus/bb runbooks) is a
  cross-project contract — coordinate or alias rather than break.
- Decide early whether to rename the git repository and the local checkout
  directory (`~/Development/daedalus`), or keep the dir and rename only the
  product — the cheaper, lower-risk option.
- Keep `cerberus`, `bitterblossom`, `olympus` untouched — those are plane names,
  not this project.
