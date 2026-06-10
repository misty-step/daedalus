# ADR-003: Run-the-app / visual-QA arena — GO, with deterministic-oracle scope

Status: accepted (2026-06-10) — spike for backlog 012

## Context

Operator wants a second arena family where the agent launches an application,
exercises it, and reports defects — defects that a diff-only reviewer cannot
see because they only manifest at runtime (broken interactions, wrong state
transitions, rendering/async bugs). The open question is whether such tasks
can be graded with **deterministic-enough oracles** (Harbor-style 0/1) rather
than subjective screenshot judging, which the project's red lines distrust as
a primary oracle.

## Spike: the feasibility question, answered with a probe

Built a minimal run-the-app task: a tiny HTTP app with one runtime defect —
a counter button that increments by **2** instead of **1**. The defect is
only observable by loading the page and clicking; the app renders no error.
Oracle: a Playwright script loads the page, clicks the button, reads the DOM,
and emits `{reward: 1}` iff the counter reads "1".

Probe results (host-side Playwright 1.55 against the running app, 2026-06-10):

```
defective app (increments by 2)  →  {"observed":"2","reward":0}
fixed app      (increments by 1)  →  {"observed":"1","reward":1}
```

Deterministic 0/1 in both directions, no human in the loop. The
DOM/state-assertion-as-oracle mechanism works. (The faithful in-container
variant — a `mcr.microsoft.com/playwright` image running the same probe — was
also wired up; its build is just a slow ~730MB image pull, and Harbor's
container model is already proven in ADR-001, so the host-side run is the
load-bearing evidence.)

## Decision: GO, scoped to deterministic assertions

Build a run-the-app arena family, with these scope rules:

- **Oracles are deterministic assertions only:** in-container HTTP probes, DB
  state checks, and Playwright DOM/state assertions that yield 0/1. Screenshot
  or LLM-judge subjectivity is NOT a primary oracle here (it may later be a
  *secondary* signal under the calibrated judge family, ticket 010, with its
  calibration gate — never alone).
- **Defects must be runtime-only:** the value of this family is defects a
  diff reviewer cannot catch statically. A task whose defect is visible in the
  diff belongs in the pr-review arena, not here.
- **Container model:** Harbor (ADR-001) owns isolation; the task image extends
  a Playwright-capable base. This is the one real cost — a ~730MB browser
  image vs the lightweight code-review containers — so trials are heavier and
  slower; budget accordingly.

## Cost to author one task

App fixture + one runtime defect + one Playwright/HTTP assertion script +
oracle solution (the fix) — comparable effort to a pr-review task, plus a
one-time per-arena Playwright base image. The assertion script *is* the
answer key; author it with the defect, as with every arena.

## Consequences

- This is a new L+ build ticket (the arena itself), unblocked by this GO:
  fixture apps, the Playwright-base task image, a handful of runtime defects
  across the interaction/state/async taxonomy, plus the standard rig
  (oracle 1.0, null floor, and a "clean app" FP trap where the app is
  correct and the agent must report nothing).
- Reuses everything already built: composition.v1 candidates, the
  deterministic scorer shape (0/1 reward), splits, certification racing,
  lineage. Only the arena and its in-container verifier are new.
- Revisit-as-no-go trigger: if authoring runtime defects that are *both*
  deterministic to assert *and* genuinely invisible to a strong static
  reviewer proves impractical at volume, fall back to keeping pr-review as
  the sole family and revisit when execution tooling improves.
