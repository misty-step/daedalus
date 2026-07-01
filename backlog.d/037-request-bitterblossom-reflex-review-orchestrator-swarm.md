# Request Bitterblossom reflex review orchestrator swarm

Priority: P2
Status: blocked - 048 delivered; gated behind 054; Sprites runner contract scoped
Estimate: XL

> **GATED 2026-06-24** behind [[054]] — Bitterblossom reflex-review fan-out waits
> until the Cerberus reviewer loop hums (VISION). 048 is delivered, so this no
> longer waits on the lab; it waits on the hum bar. Demoted P0→P2.

> **Wiring update 2026-07-01:** use
> `docs/crucible-eval-optimization-contract.md` as the Threshold-side contract.
> Reflex-review candidates run on Bitter Blossom/Sprites through
> `threshold.sprite_trial_request.v1` and return
> `threshold.sprite_trial_receipt.v1`; Threshold scores and optimizes the
> artifacts against the Crucible/Harbor target. Bitter Blossom owns triggers,
> idempotency, budgets, queueing, and receipts.

## Goal

Produce a Bitterblossom-specific reflex code-review launch packet from the
Cerberus lab recommendation: the exact Cerberus request/artifact path,
substrate, topology, payload, and event-plane mapping that should run on
PR-ready/update events and later on a less GitHub-dependent VCS event source.

This ticket should consume, not fork, `backlog.d/048-build-cerberus-substrate-rd-lab.md`.
`034` is now historical/reusable review-swarm evidence, not the parent product
path. The output is the Bitterblossom import and runtime contract: which
Cerberus config runs, what payload it receives, what artifact it emits, how any
selected multi-agent topology consolidates findings, and what trigger/debounce
rules keep the event plane sane.

## Why Now

The target product loop is an Amjad/Replit-style reflex plane:

- an orchestrator fans out focused agents,
- verifier/security/production agents return evidence,
- follow-up fix prompts are generated at lifecycle boundaries,
- the plane records the exact event, run, cost, artifact, and decision.

Bitterblossom already has review lanes, submissions, storms, gates, and manual
builder dispatch. It needs a measured, Threshold-backed reflex review packet
that can replace ad-hoc review prompts with a reproducible Cerberus-backed
review artifact path.

## Boundaries

- Pre-G3 output is sandbox-only and secondary. Do not make this the primary
  production reviewer without human approval.
- Member agents write artifacts. The plane owns comment formatting, posting,
  dedupe, budget checks, and ledger state.
- Do not add workload-specific Rust branches to Bitterblossom dispatch,
  ledger, recovery, substrate, or budget code.
- Do not assume parallel Pi execution inside Threshold until the documented Pi
  concurrency deadlock is retested. The target Bitterblossom plane may run
  members in parallel only after its substrate proves it safely.

## Proposed Runtime Shape

Initial trigger shape:

| event | filter | action |
|---|---|---|
| `pull_request.opened` | repo allowlist, non-draft, additions cap | enqueue review orchestrator |
| `pull_request.ready_for_review` | repo allowlist, non-draft | enqueue review orchestrator |
| `pull_request.synchronize` | non-draft, debounce by repo/pr/head_sha | refresh or open review submission |
| manual `bb run review-orchestrator` | operator payload | run same artifact path without webhook |

Candidate topology is no longer assumed to be a Pi specialist swarm. `048` must
first decide whether Cerberus should run a single autonomous reviewer, dynamic
lanes, or a measured multi-member topology. The legacy lane shape below is
only a comparison scaffold if a multi-member topology survives the lab.

| lane | responsibility | substrate/model source |
|---|---|---|
| `general` | broad review baseline and context sanity | `048` Cerberus recommendation |
| `correctness` | logic, invariants, data loss, crashes, lifecycle races | `048` Cerberus recommendation |
| `security` | auth, secrets, injection, unsafe input, supply chain | `048` Cerberus recommendation |
| `verification` | tests, gates, CI evidence, acceptance proof gaps | `048` Cerberus recommendation |
| `simplification` | gate weakening, needless surface, shallow abstraction | `048` Cerberus recommendation |
| `product` | ticket fit, UX/API intent, scope mismatch | `048` Cerberus recommendation |
| `master` | consolidate member artifacts, dedupe, severity calibration | `048` Cerberus recommendation, only if the lab recommends a multi-member topology |

`z-ai/glm-5.2` is requested by the operator and its OpenRouter model page was
visible on 2026-06-15 as released, with API access releasing 2026-06-16. It was
not listed in the OpenRouter API catalog on 2026-06-15. Include it only after a
live catalog check and Pi smoke receipt prove it dispatchable.

## Artifact Contract

Each member emits strict JSON, then the master emits one strict consolidated
artifact. Bitterblossom can later format/post that artifact.

Member minimum:

```json
{
  "member_id": "correctness",
  "status": "ok",
  "repo": "owner/repo",
  "base_rev": "base commit",
  "head_rev": "reviewed commit",
  "event_kind": "pull_request.synchronize",
  "coverage": "what was inspected",
  "findings": [
    {
      "fingerprint": "stable-local-id",
      "severity": "blocking|serious|minor",
      "category": "correctness",
      "path": "src/file.rs",
      "line": 42,
      "claim": "one sentence",
      "evidence": "quoted diff, command, or artifact reference",
      "confidence": "high|medium|low"
    }
  ],
  "residual_risk": "what this member did not inspect"
}
```

Master minimum:

```json
{
  "decision": "block|comment|pass",
  "repo": "owner/repo",
  "base_rev": "base commit",
  "head_rev": "reviewed commit",
  "event_kind": "pull_request.synchronize",
  "summary": "short operator-facing synthesis",
  "findings": [],
  "member_status": [],
  "cost_usd": 0.0,
  "wall_seconds": 0,
  "residual_risk": "coverage gaps and failed lanes"
}
```

Malformed or missing required member output is non-pass by default. A master
may recommend `pass` only if the launch packet defines an explicit waiver path
and the artifact records which member was waived and why.

## Requested Threshold Work

1. Align this ticket with `048` so Bitterblossom consumes the Cerberus request,
   artifact, topology, and substrate recommendation instead of forking a
   divergent review system.
2. Import the selected Crucible/Harbor eval as
   `threshold.optimization_target.v1`, preserving eval version, answer-key and
   scorer digests, split policy, incumbent baseline, and G2 state.
3. Produce a Bitterblossom launch packet that maps the selected Cerberus config
   into `plane/agents`, `plane/tasks`, cards, payload examples, budget defaults,
   idempotency keys, and status surfaces.
4. Import or run the Cerberus candidates recommended by `048` under the
   review cost and wall-time envelope, preserving artifacts as first-class
   evidence.
5. Make the master a consolidation task, not a fresh broad reviewer that
   launders weak member output.
6. Define failure behavior: timed-out member, malformed JSON, budget-blocked
   member, duplicate finding, stale head SHA, draft PR, closed PR, forked PR.
7. Produce a sandbox import path for Bitterblossom that can be dogfooded by
   `bb submit` / `bb gate` before any production webhook posting.
8. Prove the remote runner seam with one manual `bb run`/Sprites trial before
   webhook reflex work: the receipt must include run id, task id, candidate
   composition hash, status, cost/wall fields, artifact refs, and error state.

## Oracle

- [ ] The packet names the exact Bitterblossom task ids, agent ids, payload
      fields, idempotency keys, artifact paths, and run/gate read commands.
- [ ] The packet names the `threshold.optimization_target.v1` eval ref,
      answer-key and scorer digests, incumbent baseline, and holdout policy it
      optimizes against.
- [ ] A manual Bitter Blossom/Sprites trial returns
      `threshold.sprite_trial_receipt.v1`; Threshold scores the artifact and
      records the trial without exposing `tests/` or `solution/`.
- [ ] At least one measured Threshold/Cerberus vertical slice covers
      `ReviewRequest.v1`, `ReviewArtifact.v1`, and any selected topology
      rather than assuming a Pi specialist swarm.
- [ ] Total recommended sandbox envelope is <= $2.00/PR and <= 20 minutes
      wall time, or the packet carries an explicit waiver and a cheaper
      fallback.
- [ ] Member lanes report false-positive discipline, missed seeded defects,
      malformed output rate, cost, and latency separately from master quality.
- [ ] Malformed, missing, timed-out, and budget-blocked required members cannot
      silently collapse to `pass`; every waiver is explicit in the master
      artifact.
- [ ] The launch packet preserves G3/G4/G5 boundaries: sandbox import before
      approval, no production write authority before G4, no production trace
      re-ingestion before G5.
- [ ] The packet can run from PR events and from a manual `bb run` payload.
- [ ] `bin/gate` passes.

## Evidence

- Active Threshold parent: `backlog.d/048-build-cerberus-substrate-rd-lab.md`
- Historical review-swarm evidence: `backlog.d/034-build-threshold-review-swarm.md`
- Bitterblossom reflex ticket:
  `/Users/phaedrus/Development/bitterblossom/backlog.d/061-sdlc-lifecycle-reflex-pack.md`
- Current handoff precedent: `deliveries/pr-review/plane-handoff.md`
- Current loop doctrine: `docs/review-autoresearch-loop.md`
