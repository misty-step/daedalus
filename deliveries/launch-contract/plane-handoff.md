# Cross-plane handoff: g2b-g1a-seed2-glm-4-7-flash-spec-first

Generated: `2026-06-12T17:22:32Z`

Lab evidence is not launch approval. This packet is import guidance for humans and control-plane dry runs; G3/G4/G5 approval still gates deployment, write authority, and production-data re-ingestion. Unsigned use is sandbox-only and must not operate as a primary reviewer.

## Certified composition identity

| field | value |
|---|---|
| agent | `g2b-g1a-seed2-glm-4-7-flash-spec-first` |
| composition hash | `7523f6b853908df2` |
| taskspec | `launch-contract-v0` |
| mode | `threshold-then-cheap` |
| harness | `pi` (`0.78.1`) |
| provider/model | `openrouter/qwen/qwen3.7-plus` |
| thinking | `low` |
| tools | `read, bash` |
| prompt packet | `/Users/phaedrus/Development/daedalus/runs/20260612T153450Z-search-launch-contract-v0/packets/g1a-seed2-glm-4-7-flash-spec-first.md` |
| output contract | `findings.json: {findings: [{file, line, category, description}]} with categories: approval-gate, evidence, permissions, observability, portability` |
| trigger intent | `Manual launch-contract review before G3/G4/G5 approval` |
| budget | `$0.35` and `420s` per run |
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
# plane/agents/g2b-g1a-seed2-glm-4-7-flash-spec-first.toml
id = "g2b-g1a-seed2-glm-4-7-flash-spec-first"
version = 1
harness = "pi"
provider = "openrouter"
model = "qwen/qwen3.7-plus"
thinking = "low"
composition_hash = "7523f6b853908df2"
contract = "contract.toml"
persona = "persona.md"
tools = ["read", "bash"]
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
model: qwen/qwen3.7-plus
provider: openrouter
thinking: low
prompt_ref: deliveries/launch-contract/persona.md
composition_hash: 7523f6b853908df2
contract_ref: deliveries/launch-contract/contract.toml
output_contract: strict findings artifact, then orchestrator review posting
budgets:
  max_cost_usd_per_run: 0.35
  max_wall_sec: 420
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
