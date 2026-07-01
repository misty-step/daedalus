# Vocabulary — the pieces of an agent

> **Status: proposed canon (2026-06-24).** Grounded in `mutate.rs`
> (`MUTABLE_SLOTS`) + a survey of the settled 2026 industry/research vocabulary.
> Two choices remain open for the operator: the whole-bundle noun (this doc
> recommends **composition**) and whether to promote this glossary into
> `VISION.md`. Anchors to [VISION.md](../VISION.md): this defines *what Threshold
> searches over*.

Threshold exists to certify the best **composition** for a plane. To do that
precisely, every distinct part of an agent needs one unambiguous name. The good
news: the kernel already encodes most of this, and already resolves the two
terms the field most often confuses.

## The pieces

| Piece | Canonical term | Threshold slot (`mutate.rs`) | Searched? |
|---|---|---|---|
| Base model (weights + provider) | **model** | `model` | ✅ mutable |
| Inference controls (reasoning effort) | **thinking** (industry: "reasoning effort") | `thinking` | ✅ mutable |
| Initializing instruction | **system prompt** | `prompt_packet` + `system_prompt_mode` | ✅ mutable |
| Callable actions | **tools** | `tools` | ✅ mutable |
| Packaged procedural knowledge | **skills** (Anthropic "Agent Skills") | `skills` | ✅ mutable |
| Loaded context / briefing | **context** (industry: "context engineering") | `agents_md` | ✅ mutable |
| External capability/context providers | **MCP servers** | _— none —_ | ❌ **gap** — pi has no `--mcp`; its seam is `--extension` ([[052]]) |
| Runtime loop (Pi / OpenCode / OMP) | **substrate** | `kind` | ❌ frozen by design |

> **Runner-verified 2026-06-24:** `run.rs` `compose_pi_argv` genuinely attaches
> `skills`/`tools`/`context` to the agent (`--skill`, `--tools`,
> `--no-context-files`) — the surface is wired, not hollow; it was just *unfed*
> for cerberus. But pi exposes **no MCP flag** (its capability seam is
> `--extension`, and the runner forces `--no-extensions`), so MCP is not a clean
> mutable slot yet — see [[052]].

## The three terms that earn their keep

- **Composition** — the whole configured bundle (model + thinking + system
  prompt + capability surface). Already in the kernel (`composition_hash`); the
  unit Threshold certifies and a plane deploys. (Externally the whole-bundle noun
  is essentially uncoined — "agent" is ambiguous — so "composition" is ours with
  near-zero collision.)
- **Capability surface** — the searchable subset that *equips* an agent: tools +
  skills + MCPs + context. Distinct from the *engine* (model + thinking). This is
  the axis where domain wins live: a reviewer with the right reviewing
  skills/MCPs, a red-teamer with real white-hat tooling. (Externally near-coined
  — only the security-flavored "MCP exposure surface" exists.)
- **Substrate** — the runtime loop (Pi / OpenCode / OMP / Claude Code / Codex).
  Frozen by design: pi exposes no flag for it, so varying it would change the
  composition hash without isolating a behavior change. Default: OpenCode.

## Retire the bare word "harness"

It is the most overloaded term in the field, and Threshold already escaped the
ambiguity — so don't re-import it:

- Eval orgs (SWE-bench) use **harness** = the *test runner / grader*. Threshold
  calls that the **arena**.
- Tooling practitioners (OpenAI Codex, Willison) use **harness** = the *runtime
  loop*. Threshold calls that the **substrate**.

Use **arena** and **substrate**; reserve "harness" for loose external reference
only.

## Why this matters to the search

The certified Cerberus run varied model/prompt/tools but ran every seed with
`skill_set_name: null` and no MCP slot — the capability surface was *wired but
unfed*. "How much can capability surface move quality?" is unmeasured not because
the search can't explore it, but because we haven't populated rich skill/MCP sets
to explore. Closing the MCP-slot gap + feeding capability sets is [[052]].
