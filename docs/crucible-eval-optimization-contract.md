# Crucible Eval to Optimization Contract

Status: proposed Threshold-side contract, 2026-07-01.

Threshold does not own the eval workbench. A Crucible eval becomes useful to
Threshold only when it is pulled as a frozen optimization target: a versioned
eval bundle, a runner package, a trusted scorer, baselines, split discipline,
and gates that say whether search is allowed.

## Ownership

| Surface | Owner | Contract |
|---|---|---|
| Eval intent, fixtures, answer keys, adjudication, contamination, holdout policy | Crucible | `crucible.eval_spec.v1` plus a Harbor task-directory export |
| Optimization objective, candidate space, search strategy, reports, launch contract | Threshold | `threshold.optimization_target.v1`, taskspec, runs, reports, exports |
| Remote candidate execution, queueing, leases, budget, receipts | Bitter Blossom | task/agent files running on `substrate = "sprites"` and returning receipts |

Crucible can design or repair an eval. Threshold can reject, freeze, score, and
search against it. Bitter Blossom can run candidates on Sprites and record what
happened. None of the three should silently absorb another repo's job.

## Target Shape

`threshold.optimization_target.v1` is the small wrapper Threshold needs around a
Crucible eval bundle.

```toml
schema = "threshold.optimization_target.v1"
id = "bb-review-correctness"
source = "crucible"
mode = "threshold-then-cheap"
decision = "certify the cheapest reliable correctness reviewer config for Bitter Blossom"

[eval]
schema = "crucible.eval_spec.v1"
spec_ref = "crucible://evals/code-review-correctness/eval.json"
version = "0.1.0"
digest = "sha256:<eval bundle digest>"
harbor_package = "crucible://exports/code-review-correctness/harbor"
fixtures_digest = "sha256:<fixtures digest>"
answer_key_digest = "sha256:<tests/expected.json digest>"
oracle_solution_digest = "sha256:<solution/findings.json digest>"
scorer_binary_digest = "sha256:<threshold-score or Crucible scorer digest>"

[scorer]
kind = "threshold-score"
command = "threshold-score <findings.json> <expected.json>"
reward = "max(0, recall - 0.2 * false_positives); clean task with any finding scores 0"
authoritative_until = "crucible-backlog-008-grade-reward-parity"

[splits]
train = "declared by Crucible export"
validation = "declared by Crucible export"
holdout = "declared by Crucible export"
holdout_policy = "append exposure, burn after the Crucible-owned limit"

[baselines]
oracle = "Crucible/Threshold oracle fixture solution"
null = "empty findings"
oneshot = "saturation probe only; never a candidate"
incumbent = "Bitter Blossom storm-correctness current config"

[runner]
default = "harbor-local"
remote = "bitter-blossom-sprites"
request_schema = "threshold.sprite_trial_request.v1"
receipt_schema = "threshold.sprite_trial_receipt.v1"

[gates.g1]
state = "unsigned"
approval_ref = "approvals/G1-bb-review-correctness.md"
meaning = "operator approves the task/search spend"

[gates.g2]
state = "unsigned"
approval_ref = "approvals/G2-code-review-correctness.md"
required_for_search = true
meaning = "eval quality/headroom approved before scores are trusted"

[gates.g3]
state = "unsigned"
approval_ref = "approvals/G3-bb-review-correctness.md"
meaning = "launch contract approval before downstream deployment"

[gates.g5]
state = "not_applicable"
approval_ref = ""
meaning = "only for production trace re-ingestion, not for authored Crucible evals"
```

The first target is the code-review correctness family. Until Crucible owns and
exports the full bundle, Threshold may use the existing
`specs/pr-review-correctness/taskspec.toml` and `arenas/pr-review-correctness-v0`
as the transitional Harbor package. The optimization objective remains the Rust
`threshold-score` reward because Crucible backlog 008 says `crucible grade` is
not yet a faithful predictor of Threshold reward.

## Pull Flow

1. Crucible exports `crucible.eval_spec.v1`, a Harbor package, answer-key,
   oracle-solution, scorer-binary digests, contamination metadata, and holdout
   policy.
2. Threshold imports that bundle as `threshold.optimization_target.v1` and
   validates schema version, digests, gate state and approval refs, runner
   contract, scorer availability, and model-pool compatibility.
3. Threshold runs the no-search rig: oracle, null, one-shot saturation probe,
   key red-team checks, and at least two distinct reference compositions when
   budget allows.
4. A bounded headroom probe, capped at about $5 unless G1 says otherwise, runs
   the incumbent and a few diverse seed candidates on the validation split. If
   everything ties inside noise, the one-shot saturates, or the incumbent is at
   oracle, the target returns to Crucible instead of entering search.
5. Threshold searches only after the target clears the gates. Final claims use
   a held-out split, paired deltas, cost and latency, and launch-contract
   provenance.

## Bitter Blossom Sprites Runner

When local Harbor is not the right execution substrate, Threshold submits a
trial request to Bitter Blossom and scores the returned artifact itself.

`threshold.sprite_trial_request.v1` minimum fields:

```json
{
  "schema": "threshold.sprite_trial_request.v1",
  "experiment_id": "run id",
  "trial_id": "candidate/task/trial id",
  "task_id": "harbor task id",
  "candidate": {
    "composition_hash": "sha256:...",
    "model": "provider/model",
    "thinking": "medium",
    "prompt_packet_digest": "sha256:...",
    "tool_policy": "explore"
  },
  "workspace": {
    "harbor_package": "content-addressed eval task ref",
    "candidate_visible_paths": ["instruction.md", "environment/"]
  },
  "output_contract": "findings.json",
  "secret_names": ["OPENROUTER_API_KEY"],
  "env_allowlist": ["OPENROUTER_API_KEY"],
  "budget": {
    "max_cost_usd": 0.5,
    "timeout_seconds": 600
  }
}
```

`threshold.sprite_trial_receipt.v1` minimum fields:

```json
{
  "schema": "threshold.sprite_trial_receipt.v1",
  "trial_id": "candidate/task/trial id",
  "task_id": "harbor task id",
  "composition_hash": "sha256:...",
  "bitter_blossom_run_id": "bb ledger id",
  "substrate": "sprites",
  "status": "ok|failed|timed_out|budget_blocked",
  "artifact_refs": {
    "findings": "path or object ref for findings.json",
    "transcript": "path or object ref for transcript"
  },
  "model_served": "provider/model",
  "tokens_prompt": null,
  "tokens_completion": null,
  "cost_usd": null,
  "wall_ms": 0,
  "error": null,
  "receipt_digest": "sha256:..."
}
```

Bitter Blossom never receives hidden `tests/` or `solution/`, never persists
secret payloads, and never decides the optimization winner. It executes a
declared trial on Sprites, records budget/receipt truth, and returns the
candidate output for Threshold to score.

## Optimizer Guards

- Overfitting: GEPA and other mutators see only train/validation evidence;
  final holdout exposure is appended to the Crucible-owned ledger and is not fed
  back into the same search.
- Judge gaming: deterministic scorers remain primary where available; judge
  objectives require calibration and never stand alone.
- Non-stationarity: model ids, provider-served ids, prices, prompt packets,
  scorer digests, fixture digests, and eval version are recorded in every run.
  A model or eval change triggers a fresh headroom probe before comparison.
- Budget discipline: Hyperband/ASHA may promote candidates across larger
  budgets only when paired validation deltas clear observed noise and the
  known spend remains below the approved cap.
