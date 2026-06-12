# Cross-plane handoff: seed4-qwen3-7-plus-checklist

Generated: `2026-06-12T00:18:48Z`

Lab evidence is not launch approval. This packet is import guidance for humans and control-plane dry runs; G3/G4/G5 approval still gates deployment, write authority, and production-data re-ingestion. Unsigned use is sandbox-only and must not operate as a primary reviewer.

Use `docs/operator-sop.md` for the maintained export, trace, launch-pack,
approval, regression, and closeout sequence. This handoff records the
historical import shape for this delivery.

## Certified composition identity

| field | value |
|---|---|
| agent | `seed4-qwen3-7-plus-checklist` |
| composition hash | `4a73f1fd213aa1a5` |
| taskspec | `pr-review-v0` |
| mode | `threshold-then-cheap` |
| harness | `pi` (`0.78.1`) |
| provider/model | `openrouter/qwen/qwen3.7-plus` |
| thinking | `low` |
| tools | `read, bash, edit, write` |
| prompt packet | `/Users/phaedrus/Development/daedalus/runs/20260611T173632Z-search-pr-review-v0/packets/seed-checklist.md` |
| output contract | `findings.json: {findings: [{file, line, category, description}]} with categories from the fixed taxonomy (see arena instruction.md)` |
| trigger intent | `GitHub PR webhook (Phase 3); manual runs until then` |
| budget | `$0.5` and `600s` per run |
| env | `OPENROUTER_API_KEY` |
| approval | `g3_signed = false` until a human signs the launch gate |

## Incumbent comparison

| plane | current incumbent | model | posting / output boundary | config surfaces | import delta |
|---|---|---|---|---|---|
| Bitter Blossom | review-coordinator v2 | `moonshotai/kimi-k2.6` | agent posts exactly one PR comment directly through gh | plane/agents/review-coordinator.toml, plane/tasks/review/task.toml, plane/tasks/review/card.md | Replace or overlay agent/persona fields from this packet; preserve task filters, dedupe, budgets, and HMAC ingress. |
| Olympus | charon v2 | `~moonshotai/kimi-latest` | agent writes strict JSON artifact; orchestrator validates anchors, caps output, suppresses duplicates, and posts | orchestrator/agent-specs/charon.yaml, orchestrator/prompts/charon-review.md, orchestrator/src/charon-review-dispatcher.ts, orchestrator/src/charon-review-poster.ts | Replace or overlay AgentSpec runtime/model/prompt fields from this packet; preserve activation gating, strict artifact validation, duplicate suppression, and orchestrator-side posting. |

## Bitter Blossom import shape

Map the measured composition into `plane/agents/` and keep the review task's existing trigger/filter/budget guardrails unless a G3 launch approval says otherwise.

```toml
# plane/agents/seed4-qwen3-7-plus-checklist.toml
id = "seed4-qwen3-7-plus-checklist"
version = 1
harness = "pi"
provider = "openrouter"
model = "qwen/qwen3.7-plus"
thinking = "low"
composition_hash = "4a73f1fd213aa1a5"
contract = "contract.toml"
persona = "persona.md"
tools = ["read", "bash", "edit", "write"]
secrets = ["OPENROUTER_API_KEY"]
```

- If Bitter Blossom keeps direct `gh pr comment` posting, the task card must retain the no-approve/no-merge/no-code-edit red lines and the measured prompt packet must remain byte-identical.
- Preferred safer import: keep the measured review persona, have the agent emit the structured findings contract, and let the plane own comment formatting/posting after G3.
- Before G3, any Bitter Blossom run must be sandboxed and secondary to the existing review path; it is evidence for Daedalus, not an enterprise-ready reviewer deployment.

## Olympus AgentSpec import shape

Map the same measured composition into Charon or a successor AgentSpec without weakening Olympus' control-plane-owned validation/posting boundary.

```yaml
id: charon
version: <human-bumped>
runtime: pi
model: qwen/qwen3.7-plus
provider: openrouter
thinking: low
prompt_ref: deliveries/pr-review/persona.md
composition_hash: 4a73f1fd213aa1a5
contract_ref: deliveries/pr-review/contract.toml
output_contract: strict findings artifact, then orchestrator review posting
budgets:
  max_cost_usd_per_run: 0.5
  max_wall_sec: 600
activation:
  g3_signed: false
```

- Preserve pinned-head checkout, untrusted PR metadata handling, output caps, diff-anchor validation, hidden marker duplicate suppression, and control-plane posting.
- Treat this packet as an AgentSpec overlay candidate, not an automatic replacement for the live Charon config.

## Residual risks and next gates

- Exact replay against incumbents may be impossible when their prompts, posting contract, runtime wrappers, or model aliases do not map to a Daedalus composition slot; record that in the run report instead of pretending parity.
- G3 decides whether either plane imports this packet.
- G4 is required before any production write authority expands beyond advisory review output.
- G5 is required before production traces or PR data flow back into arena fixtures.
- This handoff is not a public benchmark-quality claim; keep the G2 calibration caveats attached until a stronger arena version supersedes them.

### Bitter Blossom incumbent notes

- Current task is event-plane shaped with manual/webhook ingress, HMAC dedupe, repo filters, additions caps, and per-run budget controls.
- Current posting boundary lives inside the review card: the agent fetches the PR and posts through gh itself.
- Live evidence exists from real Bitter Blossom PR review runs, but this incumbent was not selected by Daedalus evals.

### Olympus incumbent notes

- Current Charon is AgentSpec-shaped and activation-gated, with the control plane owning GitHub posting.
- Current eval evidence is promptfoo fixture acceptance plus activation receipts, not a Daedalus arena search.
- The import delta should preserve Olympus' strict artifact and orchestrator-side posting boundary.
