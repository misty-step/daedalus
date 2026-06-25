# Request Bitterblossom VCS-native trigger experiment

Priority: P3
Status: blocked - gated behind 054 (Cerberus-first mandate)
Estimate: M

> **GATED 2026-06-24** behind [[054]] — VCS-native trigger research is downstream
> of a humming Cerberus reviewer (VISION). Demoted P1→P3.

## Goal

Research and prototype a less GitHub-dependent trigger path for
Bitterblossom ad-hoc dispatch and reflex review agents.

The near-term product can run on GitHub PR open/update/ready events with
debounce and non-draft filters. The better long-term shape should also work
from local git state, push hooks, notes, refs, trailers, or explicit `bb`
submission records so Bitterblossom is an event plane for repos, not a GitHub
bot with extra steps.

## Why Now

The reflex review swarm needs reliable event semantics:

- exactly what revision was reviewed,
- which event opened or refreshed the run,
- how debounce/dedupe works,
- how review results attach to a revision without requiring GitHub comments,
- how a local/operator workflow can trigger the same agents as a hosted PR.

If this is left implicit, review agents become GitHub-shaped and later VCS
abstraction gets expensive.

## Candidate Trigger Surfaces

Compare at least these options:

| surface | sketch | likely strength | likely risk |
|---|---|---|---|
| GitHub webhook | PR open/update/ready events hit `bb serve` | easiest production slice | GitHub-specific semantics and auth |
| manual `bb submit` | operator opens a submission with repo, base, head, task | already close to current plane shape | manual unless paired with hooks |
| git notes | attach review/run metadata under `refs/notes/bb/*` | VCS-native and revision-attached | sync, conflict, and UX complexity |
| git refs | write `refs/bb/reviews/<sha>` or queue refs | cheap, scriptable, push-hook friendly | ref cleanup and remote policy issues |
| commit trailers | requests such as `BB-Review: correctness` | durable in commits | noisy history, hard to debounce |
| local queue files | `.bb/events/*.json` consumed by `bb serve` or `bb run` | simple dev-plane prototype | repo dirtiness and locking |
| post-receive/pre-push hook | synthesize events from git transport | Git-server friendly | install/distribution complexity |
| CI artifact/status | failed check writes event packet for `bb` | works beyond GitHub if CI is generic | latency and auth fragmentation |

## Gut-Instinct Seed Configs

This ticket is primarily design/prototype work. Use Pi/OpenRouter lanes for
research and implementation probes when Daedalus turns it into an arena.

| role | harness | model/config | why seed it |
|---|---|---|---|
| architecture researcher | Pi | `deepseek/deepseek-v4-pro`, high thinking, read/bash | large context for comparing repo, git, and plane contracts |
| prototype implementer | Pi | `moonshotai/kimi-k2.7-code`, xhigh thinking, full tools | coding-focused candidate for scripts/dev-plane fixtures |
| cheap adversarial critic | Pi | `deepseek/deepseek-v4-flash`, medium thinking, read/bash | inexpensive failure-mode scan |
| alternate critic | Pi | `qwen/qwen3-coder-next` or `z-ai/glm-5.1` | different model family for trigger tradeoff critique |
| council fallback | OpenRouter | `openrouter/fusion` | use only for high-leverage architecture comparison |

`z-ai/glm-5.2` is page-visible on OpenRouter but remains API-pending until the
OpenRouter catalog lists it and Pi can smoke it.

## Requested Daedalus Work

1. Produce a decision matrix for the trigger surfaces above, covering dedupe,
   debounce, revision identity, auth, offline operation, forked repos, branch
   deletes, force pushes, rebases, local-only work, hosted CI, and auditability.
2. Define a VCS-neutral event packet shape that Bitterblossom can route without
   knowing GitHub PR semantics:
   `repo`, `base_rev`, `head_rev`, `change_ref`, `event_kind`,
   `idempotency_key`, `requested_tasks`, `actor`, `source`, and `evidence`.
3. Prototype one dev-plane path that does not require a GitHub webhook. Good
   first candidates: `bb submit` from a local git repo plus git notes, or a
   local queue-file event consumed by `bb serve`.
4. Show how the reflex review swarm and ad-hoc dispatch agents consume the
   same packet.
5. Emit a Bitterblossom import/design packet: task/card changes, trigger
   config shape, idempotency strategy, migration plan from GitHub-first to
   VCS-native, and residual risks.

## Oracle

- [ ] A design packet ranks at least five trigger surfaces and chooses one
      first experiment plus one likely long-term substrate.
- [ ] The chosen event packet can represent GitHub PR updates, manual `bb`
      submissions, and local git revision requests without losing revision
      identity.
- [ ] A dev-plane prototype or executable recipe demonstrates one non-GitHub
      trigger creating a Bitterblossom-visible run/submission.
- [ ] The prototype remains sandbox/pre-G3 evidence only and does not imply
      production trigger approval, production write authority, or webhook
      replacement.
- [ ] The design explains how results attach back to git notes/refs or a
      ledger-only artifact without requiring GitHub comments.
- [ ] Failure modes are explicit: duplicate events, stale head, rewritten
      history, missing remote, deleted branch, forked repo, auth failure,
      poisoned local queue, and race between push and review.
- [ ] No workload-specific Rust branch is required for the prototype unless a
      separate ticket justifies it.
- [ ] `bin/gate` passes.

## Evidence

- Bitterblossom operator contract:
  `/Users/phaedrus/Development/bitterblossom/docs/spine.md`
- Bitterblossom reflex design:
  `/Users/phaedrus/Development/bitterblossom/docs/plans/2026-06-15-sdlc-reflex-agent-plane.md`
- Daedalus review-swarm parent: `backlog.d/034-build-daedalus-review-swarm.md`
