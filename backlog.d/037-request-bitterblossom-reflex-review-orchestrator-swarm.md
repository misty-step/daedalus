# Request Bitterblossom reflex review orchestrator swarm

Priority: P0
Status: pending
Estimate: XL

## Goal

Produce a Bitterblossom-specific reflex code-review launch packet: an
orchestrator plus focused reviewer subagents that can run on PR-ready/update
events now and later on a less GitHub-dependent VCS event source.

This ticket should consume, not fork, the broader Daedalus review-swarm program
in `backlog.d/034-build-daedalus-review-swarm.md`. The output is the
Bitterblossom import and runtime contract: which agents run, what payload they
receive, what artifacts they emit, how the master consolidates findings, and
what trigger/debounce rules keep the event plane sane.

## Why Now

The target product loop is an Amjad/Replit-style reflex plane:

- an orchestrator fans out focused agents,
- verifier/security/production agents return evidence,
- follow-up fix prompts are generated at lifecycle boundaries,
- the plane records the exact event, run, cost, artifact, and decision.

Bitterblossom already has review lanes, submissions, storms, gates, and manual
builder dispatch. It needs a measured, Daedalus-backed reflex review packet
that can replace ad-hoc review prompts with a reproducible swarm.

## Boundaries

- Pre-G3 output is sandbox-only and secondary. Do not make this the primary
  production reviewer without human approval.
- Member agents write artifacts. The plane owns comment formatting, posting,
  dedupe, budget checks, and ledger state.
- Do not add workload-specific Rust branches to Bitterblossom dispatch,
  ledger, recovery, substrate, or budget code.
- Do not assume parallel Pi execution inside Daedalus until the documented Pi
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

Member lanes:

| lane | responsibility | first model seeds |
|---|---|---|
| `general` | broad review baseline and context sanity | Pi `moonshotai/kimi-k2.7-code`; Pi `qwen/qwen3-coder-next` |
| `correctness` | logic, invariants, data loss, crashes, lifecycle races | Pi `deepseek/deepseek-v4-pro`; Pi `moonshotai/kimi-k2.7-code` |
| `security` | auth, secrets, injection, unsafe input, supply chain | Pi `deepseek/deepseek-v4-pro`; Pi `minimax/minimax-m3` |
| `verification` | tests, gates, CI evidence, acceptance proof gaps | Pi `deepseek/deepseek-v4-flash`; Pi `qwen/qwen3-coder-next` |
| `simplification` | gate weakening, needless surface, shallow abstraction | Pi `deepseek/deepseek-v4-flash`; Pi `z-ai/glm-5.1` |
| `product` | ticket fit, UX/API intent, scope mismatch | Pi `moonshotai/kimi-k2.7-code`; Pi `z-ai/glm-5.1` |
| `master` | consolidate member artifacts, dedupe, severity calibration | Pi `deepseek/deepseek-v4-pro`; Pi `moonshotai/kimi-k2.7-code`; Fusion only for council experiments |

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

## Requested Daedalus Work

1. Align this ticket with `034` so Bitterblossom does not get a divergent
   review taxonomy or output schema.
2. Produce a Bitterblossom launch packet that maps Daedalus member/master
   contracts into `plane/agents`, `plane/tasks`, cards, payload examples,
   budget defaults, idempotency keys, and status surfaces.
3. Search the Pi/OpenRouter candidates above under the review-swarm cost and
   wall-time envelope, preserving member artifacts as first-class evidence.
4. Make the master a consolidation task, not a fresh broad reviewer that
   launders weak member output.
5. Define failure behavior: timed-out member, malformed JSON, budget-blocked
   member, duplicate finding, stale head SHA, draft PR, closed PR, forked PR.
6. Produce a sandbox import path for Bitterblossom that can be dogfooded by
   `bb submit` / `bb gate` before any production webhook posting.

## Oracle

- [ ] The packet names the exact Bitterblossom task ids, agent ids, payload
      fields, idempotency keys, artifact paths, and run/gate read commands.
- [ ] At least one measured Daedalus vertical slice covers member artifacts
      plus master consolidation, not just a single reviewer prompt.
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

- Parent Daedalus work: `backlog.d/034-build-daedalus-review-swarm.md`
- Bitterblossom reflex ticket:
  `/Users/phaedrus/Development/bitterblossom/backlog.d/061-sdlc-lifecycle-reflex-pack.md`
- Current handoff precedent: `deliveries/pr-review/plane-handoff.md`
- Current loop doctrine: `docs/review-autoresearch-loop.md`
