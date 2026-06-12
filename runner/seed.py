#!/usr/bin/env python3
"""Seed a diverse agent population from the taskspec's declared search space.

The broad landscape scan that precedes iterative search: sample N pi
compositions spanning models × thinking × tool policies, paired with
distinct prompt-packet stances authored by the optimizer model. Scalar
sampling is deterministic given the recorded RNG seed; packets are
versioned files hashed into each composition. Slot values must come from
the taskspec [search] tables, which in turn draw on docs/primitives.md.
"""

import random
from pathlib import Path

import mutate

REPO = Path(__file__).resolve().parent.parent

DEFAULT_POLICIES = {"full": ["read", "bash", "edit", "write"]}

# Stance library for packet diversity: each seed packet embodies a distinct
# review strategy. The optimizer writes the packet text to one of these
# briefs, so seeds differ in strategy, not just wording.
STANCES = [
    ("checklist", "Systematic checklist review: enumerate the defect "
     "taxonomy and check the change against every category in order."),
    ("skeptic", "Minimal-false-positive review: report only findings you "
     "can prove from the code in front of you; when unsure, stay silent."),
    ("spec-first", "Specification-first review: read SPEC/docs/invariants "
     "before the diff; flag violations of documented contracts."),
    ("trace-callers", "Cross-file dataflow review: for every changed "
     "function, trace its callers and callees before judging the change."),
    ("test-runner", "Evidence-by-execution review: when tests or a runnable "
     "entrypoint exist, run them and ground findings in observed behavior."),
]

PACKET_BRIEF = """\
Write a system prompt (a "prompt packet") for a focused review agent.

Task goal: {goal}
Review stance the packet must embody: {stance}

Requirements: under 250 words, imperative voice, no preamble, no markdown
headers. The packet must instruct the agent to ground every finding in
file/line evidence and to report nothing on a clean change.
Respond with ONLY the packet text."""


def sample_compositions(search, n, rng):
    """n slot combos cycling each shuffled axis independently, so a small
    population still spans models, thinking levels, tool policies, and any
    optional axes the search space declares (system prompt mode, skill
    sets, workspace AGENTS.md options)."""
    models = list(search["models"])
    rng.shuffle(models)
    levels = list(search.get("thinking_levels") or ["medium"])
    rng.shuffle(levels)
    policies = sorted((search.get("tool_policies") or DEFAULT_POLICIES).items())
    rng.shuffle(policies)
    sp_modes = list(search.get("system_prompt_modes") or ["append"])
    rng.shuffle(sp_modes)
    skill_sets = sorted((search.get("skill_sets") or {}).items()) or [(None, None)]
    rng.shuffle(skill_sets)
    agents_opts = list(search.get("agents_md_options") or [None])
    rng.shuffle(agents_opts)
    combos = []
    for i in range(n):
        set_name, set_files = skill_sets[i % len(skill_sets)]
        combos.append({
            "model": models[i % len(models)],
            "thinking": levels[i % len(levels)],
            "policy_name": policies[i % len(policies)][0],
            "tools": list(policies[i % len(policies)][1]),
            "system_prompt_mode": sp_modes[i % len(sp_modes)],
            "skill_set_name": set_name,
            "skills": list(set_files) if set_files else None,
            "agents_md": agents_opts[i % len(agents_opts)],
        })
    return combos


def author_packets(taskspec, k, optimizer_model, rng, packets_dir,
                   call=None, fallback_text=None):
    """k stance packets written by the optimizer. A failed call falls back to
    the base packet (diversity lost, validity kept) when one is declared."""
    call = call or mutate.call_optimizer
    stances = list(STANCES)
    rng.shuffle(stances)
    packets_dir.mkdir(parents=True, exist_ok=True)
    packets, costs = [], []
    for name, brief in stances[:k]:
        prompt = PACKET_BRIEF.format(goal=taskspec.get("goal"), stance=brief)
        try:
            text, cost = call(prompt, optimizer_model)
            costs.append(cost)
            text = text.strip() + "\n"
        except Exception:  # noqa: BLE001 - degrade to base packet
            if fallback_text is None:
                raise
            text = fallback_text
        path = packets_dir / f"seed-{name}.md"
        path.write_text(text)
        packets.append((name, path))
    return packets, costs


def build_seeds(combos, packets, manifests_dir, timeout_sec):
    """Materialize one hashed pi manifest per combo, packets round-robin.
    temperature/max_tokens are deliberately absent: pi has no flag for them
    (docs/primitives.md), so they would change the hash, not the behavior."""
    manifests_dir.mkdir(parents=True, exist_ok=True)
    out = []
    for i, combo in enumerate(combos):
        pname, ppath = packets[i % len(packets)]
        model_slug = combo["model"].split("/")[-1].replace(".", "-")
        seed_id = f"seed{i + 1}-{model_slug}-{pname}"[:48]
        manifest = {
            "composition": 1,
            "id": seed_id,
            "kind": "pi",
            "provider_name": "openrouter",
            "model": combo["model"],
            "prompt_packet": str(ppath),
            "thinking": combo["thinking"],
            "tools": combo["tools"],
            "timeout_sec": timeout_sec,
        }
        if combo.get("system_prompt_mode", "append") != "append":
            manifest["system_prompt_mode"] = combo["system_prompt_mode"]
        if combo.get("skills"):
            manifest["skills"] = combo["skills"]
        if combo.get("agents_md"):
            manifest["agents_md"] = combo["agents_md"]
        path = mutate.write_manifest(manifest, manifests_dir / f"{seed_id}.toml")
        out.append((seed_id, path))
    return out


def seed_population(spec, optimizer_model, packets_dir, manifests_dir,
                    rng_seed=None, call=None):
    """Full step: search space → combos → packets → manifests on disk.
    Returns ([(seed_id, manifest_path)], meta). meta records the RNG seed so
    a landscape scan is reproducible."""
    search = spec.get("search") or {}
    if not search.get("models"):
        raise ValueError("taskspec [search] must declare a models list")
    if rng_seed is None:
        rng_seed = random.randrange(2**32)
    rng = random.Random(rng_seed)
    n = int(search.get("seed_count", 6))
    combos = sample_compositions(search, n, rng)
    base_ref = search.get("base_packet")
    fallback = (REPO / base_ref).read_text() if base_ref else None
    k = int(search.get("packet_stances") or min(3, n))
    packets, costs = author_packets(
        spec, k, optimizer_model, rng, packets_dir,
        call=call, fallback_text=fallback,
    )
    timeout = int((spec.get("budget") or {}).get("max_wall_per_trial_sec", 600))
    seeds = build_seeds(combos, packets, manifests_dir, timeout)
    meta = {
        "rng_seed": rng_seed,
        "seed_count": n,
        "packet_stances": [name for name, _ in packets],
        "optimizer_costs": costs,
        "combos": [
            {k_: v for k_, v in c.items() if k_ not in ("tools", "skills")}
            for c in combos
        ],
    }
    return seeds, meta
