# Cross-plane handoff: seed1-gpt-5-mini-spec-first

Generated: `2026-06-13T17:36:01Z`

Lab evidence is not launch approval. This packet is import guidance for humans and control-plane dry runs; G3/G4/G5 approval still gates deployment, write authority, and production-data re-ingestion. Unsigned use is sandbox-only and must not operate as a primary reviewer.

## Certified composition identity

| field | value |
|---|---|
| agent | `seed1-gpt-5-mini-spec-first` |
| composition hash | `f090f8060cf36637` |
| taskspec | `pr-review-correctness` |
| mode | `threshold-then-cheap` |
| harness | `pi` (`0.78.1`) |
| provider/model | `openrouter/openai/gpt-5-mini` |
| thinking | `medium` |
| tools | `read, bash, edit, write` |
| prompt packet | `/Users/phaedrus/Development/daedalus/runs/20260613T161359Z-search-pr-review-correctness/packets/seed-spec-first.md` |
| output contract | `review-swarm member artifact with lens = correctness and findings in the shared member schema` |
| trigger intent | `Required member in the review-swarm suite after G3; manual sandbox runs until then` |
| budget | `$0.5` and `600s` per run |
| env | `OPENROUTER_API_KEY` |
| approval | `g3_signed = false` until a human signs the launch gate |

## Incumbent comparison

| plane | current incumbent | model | posting / output boundary | config surfaces | import delta |
|---|---|---|---|---|---|
| Bitter Blossom | not recorded | `-` | - | - | Replace or overlay agent/persona fields from this packet; preserve task filters, dedupe, budgets, and HMAC ingress. |
| Olympus | not recorded | `-` | - | - | Replace or overlay AgentSpec runtime/model/prompt fields from this packet; preserve activation gating, strict artifact validation, duplicate suppression, and orchestrator-side posting. |

## Bitter Blossom import shape

Map the measured composition into `plane/agents/` and keep the review task's existing trigger/filter/budget guardrails unless a G3 launch approval says otherwise.

```toml
# plane/agents/seed1-gpt-5-mini-spec-first.toml
id = "seed1-gpt-5-mini-spec-first"
version = 1
harness = "pi"
provider = "openrouter"
model = "openai/gpt-5-mini"
thinking = "medium"
composition_hash = "f090f8060cf36637"
contract = "contract.toml"
persona = "persona.md"
tools = ["read", "bash", "edit", "write"]
secrets = ["OPENROUTER_API_KEY"]
```

- If Bitter Blossom keeps direct posting or workflow side effects, the task card must retain the no-approval/no-write red lines and the measured prompt packet must remain byte-identical.
- Preferred safer import: keep the measured review persona, have the agent emit the structured findings contract, and let the plane own formatting/posting after G3.
- Before G3, any Bitter Blossom run must be sandboxed and secondary to the existing review path; it is evidence for Daedalus, not an enterprise-ready reviewer deployment.

## Olympus AgentSpec import shape

Map the same measured composition into an AgentSpec without weakening Olympus' control-plane-owned validation/posting boundary.

```yaml
id: <target-agent-id>
version: <human-bumped>
runtime: pi
model: openai/gpt-5-mini
provider: openrouter
thinking: medium
prompt_ref: deliveries/correctness/persona.md
composition_hash: f090f8060cf36637
contract_ref: deliveries/correctness/contract.toml
output_contract: strict findings artifact, then orchestrator review posting
budgets:
  max_cost_usd_per_run: 0.5
  max_wall_sec: 600
activation:
  g3_signed: false
```

- Preserve pinned input checkout, untrusted event metadata handling, output caps, artifact validation, duplicate suppression, and control-plane posting.
- Treat this packet as an AgentSpec overlay candidate, not an automatic replacement for the live Charon config.

## Residual risks and next gates

- Exact replay against incumbents may be impossible when their prompts, posting contract, runtime wrappers, or model aliases do not map to a Daedalus composition slot; record that in the run report instead of pretending parity.
- G3 decides whether either plane imports this packet.
- G4 is required before any production write authority expands beyond advisory review output.
- G5 is required before production traces or PR data flow back into arena fixtures.
- This handoff is not a public benchmark-quality claim; keep the G2 calibration caveats attached until a stronger arena version supersedes them.
