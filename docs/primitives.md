# Agent primitives: what a composition can be made of

The verified menu of models, providers, tools, and harness knobs available to
candidate compositions. The taskspec's `[search]` section and the master
agent's seed/mutation proposals must draw from here — never from memory.

Verified live against the OpenRouter `/api/v1/models` endpoint on
**2026-06-18** (model pages + `/api/v1/models`; superseded entries swapped to
their current successors — see git log). Policy: this pool lists only each
provider's **latest** model per tier; a model superseded by a newer version of
the same family must be removed, not kept alongside. Refresh with:

```sh
curl -s https://openrouter.ai/api/v1/models | jq . | less
```

Prices move and models are delisted; re-verify before authoring a new
search space.

## Harness (the executor)

| kind | what it is | search status |
|---|---|---|
| `pi` | pi coding agent (headless `-p --mode json --no-session`), default tools read/bash/edit/write, OpenRouter provider | **the V1 agent harness** — every candidate is a pi composition |
| `oneshot` | single chat completion, workspace inlined | reference probe only (saturation detection); never a candidate |
| `null` / `oracle` | floor / ceiling | references only |
| claude-code, codex, … | other harnesses via Harbor adapters | frozen out of V1; reopen at Phase 2 prompt-plateau (DESIGN.md) |

pi version is captured per run record (`harness_version`); pin-worthy
behavior changes ride on that field.

**Concurrency constraint (live finding, 2026-06-10, pi 0.78.1):** two
concurrent `pi -p` agent processes deadlock at startup (zero stdout until
timeout) — isolated HOMEs and `--offline` do not help; `pi --version` and
plain network calls are fine concurrently. Run pi trials **sequentially**
per machine. The Rust `daedalus run` path is naturally sequential; do not
parallelize runner invocations until pi fixes this. Re-test on pi upgrades.

## Slots a pi composition actually has

| slot | values | notes |
|---|---|---|
| `model` | OpenRouter model id (see pool below) | `--model openrouter/<id>` |
| `prompt_packet` | file under `packets/` | the primary mutable surface |
| `system_prompt_mode` | `append` (default) \| `replace` | append adds the packet to pi's default coding prompt (`--append-system-prompt`); replace makes the packet the entire system prompt (`--system-prompt`). Both verified working: a replace-mode glm-4.7-flash scored 1.0 at $0.003 on a real task (2026-06-10, sequential). An earlier "replace wanders to timeout" note was retracted — that hang was the concurrency deadlock below, not the slot. |
| `thinking` | off \| minimal \| low \| medium \| high \| xhigh | `--thinking`; reasoning budget knob |
| `tools` | subset of `read, bash, edit, write` | `--tools` allowlist; see policies below |
| `skills` | list of pi skill files | repeated `--skill`; declaring any drops `--no-skills`; contents hashed into the composition |
| `agents_md` | file ref | written to the workspace root as `AGENTS.md` and pi's context-file discovery is enabled (drops `--no-context-files`); contents hashed |
| `timeout_sec` | int | wall-clock kill |
| `env_allowlist` | env vars passed through | default `["OPENROUTER_API_KEY"]` |

Search-space declarations for the optional axes: `system_prompt_modes`
(list), `[search.skill_sets]` (named lists of skill files, mutable by set
name like tool policies), `agents_md_options` (list of file refs).

**Not real slots for pi:** `temperature` and `max_tokens` — pi exposes no
CLI flag for either, so the runner ignores them for `kind = "pi"`. A
mutation there would change the composition hash without changing behavior
(false attribution). They are oneshot-adapter knobs only; the mutation
validator must reject them for pi parents.

## Tool policies (named subsets for the search space)

Every policy must preserve a path to write `findings.json` (the output
contract), so bare `read` is excluded.

| policy | tools | the question it answers |
|---|---|---|
| `full` | read, bash, edit, write | default; everything pi ships |
| `explore` | read, bash | navigation + grep + execution, no editor — does the editor matter? |
| `no-exec` | read, edit, write | no shell — does execution matter? cheaper + safer when it doesn't |

## Model pool (tool-capable, agentic-fit, priced per 1M tokens)

A curated, verified subset; all entries support tool calls on OpenRouter as
of 2026-06-10. Tiers are about search strategy: seed broadly across tiers,
then let cost/quality mode decide who survives.

### Cheap tier (≲ $0.30 in / ≲ $2 out)

| model | in / out | ctx | notes |
|---|---|---|---|
| `deepseek/deepseek-v4-flash` | $0.098 / $0.196 | 1M | strongest cheap all-rounder |
| `z-ai/glm-4.7-flash` | $0.06 / $0.40 | 200K | |
| `openai/gpt-oss-120b` | $0.039 / $0.18 | 131K | cheapest plausible agent |
| `qwen/qwen3.6-flash` | $0.19 / $1.13 | 1M | supersedes qwen3.5-flash |
| `google/gemini-3-flash-preview` | $0.50 / $3.00 | 1M | supersedes gemini-2.5-flash-lite; cheapest current Gemini on OR |

### Mid tier (workhorses)

| model | in / out | ctx | notes |
|---|---|---|---|
| `moonshotai/kimi-k2.6` | $0.68 / $3.41 | 262K | current Kimi general flagship; reasoning eats token budget — give headroom |
| `z-ai/glm-5.2` | $1.20 / $4.20 | 1M | supersedes glm-5/glm-5.1 (Jun 2026); 1M ctx, MIT weights |
| `openai/gpt-5.4-mini` | $0.75 / $4.50 | 400K | supersedes gpt-5-mini; current OpenAI mini on OR (gpt-5.5-mini is API-only, not on OR) |
| `deepseek/deepseek-v4-pro` | $0.435 / $0.87 | 1M | SOTA reasoning + structured output at ~1/10 frontier price; default optimizer + a strong candidate |
| `minimax/minimax-m3` | $0.30 / $1.20 | 1M | |
| `qwen/qwen3.7-plus` | $0.32 / $1.28 | 1M | 2026-06-13 refresh found price drift down from $0.40 / $1.60 |
| `mistralai/mistral-large-2512` | $0.50 / $1.50 | 262K | |

### Frontier tier (quality ceiling probes)

| model | in / out | ctx | notes |
|---|---|---|---|
| `anthropic/claude-sonnet-4.6` | $3.00 / $15.00 | 1M | |
| `anthropic/claude-opus-4.8` | $5.00 / $25.00 | 1M | |
| `openai/gpt-5.5` | $5.00 / $30.00 | 1M | supersedes gpt-5.2/5.4; strongest strict structured-output discipline (alt optimizer) |
| `google/gemini-3.1-pro-preview` | $2.00 / $12.00 | 1M | |
| `x-ai/grok-4.3` | $1.25 / $2.50 | 1M | cheap output for a frontier model |
| `qwen/qwen3.7-max` | $1.25 / $3.75 | 1M | |

## Provider

**OpenRouter** is the single API: one key, per-generation `usage.cost`,
provider routing pinning via the manifest `provider` table. Known quirks
learned live (gate-protected):

- Reasoning models may spend the whole `max_tokens` budget thinking and
  return empty `content` — give 16K+ headroom and fall back to the
  `reasoning` field (optimizer calls do both).
- Cost can be missing on some routes; record `null`, never estimate.

## Optimizer / master-agent seats

- **Master agent:** Claude, interactive (this seat). High-judgment, low
  volume.
- **Optimizer model** (seed packets + mutation proposals): any OpenRouter id,
  default `moonshotai/kimi-k2.6`; `--optimizer-model` overrides. Its spend is
  metered into the experiment budget.
