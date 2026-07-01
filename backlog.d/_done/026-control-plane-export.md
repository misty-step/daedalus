# Export deliveries as control-plane agents (Olympus, Bitter Blossom)

Priority: P1
Status: ready
Estimate: M

## Goal
`threshold export <delivery>` turns a delivered agent into artifacts a
control plane can import directly: a machine-readable launch contract
(trigger class, input packet construction, permissions, budgets, escalation,
trace destination, regression-eval schedule, pinned composition hash + pi
version) plus a persona file in the Bitter Blossom sprite shape (YAML
frontmatter: name/description/model/skills; body = the prompt packet).

## Non-Goals
- Deploying anything (G3 stays a human gate)
- Building the control planes' intake side (Olympus/BB own their import)

## Oracle
- [ ] `deliveries/<id>/contract.toml`: versioned schema covering trigger,
      inputs, permissions, budgets (cost/wall per run), escalation, trace
      destination, regression-eval cadence, composition hash, harness
      (pi) version pin, evidence pointers (run dir, lineage.md)
- [ ] `deliveries/<id>/persona.md`: BB-sprite-shaped (frontmatter + packet
      body) generated from agent.toml + packet.md; round-trips: hash of the
      embedded packet matches the composition hash basis
- [ ] Export is a pure function of the delivery dir (offline test); schema
      documented in DESIGN.md (launch contract section upgraded from sketch)
- [ ] One worked example committed for the pr-review delivery

## Notes
Operator direction 2026-06-10: Olympus (work) and Bitter Blossom (personal)
will host event/trigger-driven focused agents; Threshold generates the
bespoke harnesses. BB persona format verified locally
(~/Development/bitterblossom/sprites/*.md: frontmatter name/description/
model/memory/permissionMode/skills + system-prompt body). Olympus not local
yet — keep the contract schema control-plane-neutral and the persona
renderer pluggable. Fixes the pi-version pin gap from the first delivery.
