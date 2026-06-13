#!/usr/bin/env python3
"""The reflective mutation step: propose exactly one single-slot change to a
composition, grounded in its worst trials. The LLM proposes; the validator
disposes. Frozen slots (harness kind, tools, env) cannot be mutated in V1.
"""

import json
import os
import time
import urllib.request
from pathlib import Path

from prompt_packet import is_sane_prompt_packet

REPO = Path(__file__).resolve().parent.parent
OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"

# temperature/max_tokens are NOT mutable: pi exposes no flag for either
# (docs/primitives.md), so mutating them would change the composition hash
# without changing behavior — false attribution by construction.
MUTABLE_SLOTS = {
    "prompt_packet", "model", "thinking", "tools",
    "system_prompt_mode", "agents_md", "skills",
}
THINKING_LEVELS = {"off", "minimal", "low", "medium", "high", "xhigh"}
SYSTEM_PROMPT_MODES = {"append", "replace"}
PREDICTED_REWARD = {"up", "hold"}
PREDICTED_COST = {"down", "hold", "up"}


def normalize_predicted_effect(proposal):
    """The proposer's testable prediction, scored later against measurement.
    A missing prediction defaults to the classic implicit claim (reward up,
    cost held) and is flagged as defaulted rather than failing the proposal."""
    pe = proposal.get("predicted_effect")
    if pe is None:
        return {"reward": "up", "cost": "hold"}, True
    if (not isinstance(pe, dict)
            or pe.get("reward") not in PREDICTED_REWARD
            or pe.get("cost") not in PREDICTED_COST):
        raise ValueError(
            'predicted_effect must be {"reward": "up|hold", '
            '"cost": "down|hold|up"}'
        )
    return {"reward": pe["reward"], "cost": pe["cost"]}, False


def resolve_donor(proposal, archive_manifests, parent_manifest):
    """Transplant operator: take the proposed slot's value from a named
    archive candidate. Still a one-slot delta on the parent — it just lets
    the search recombine discovered wins (cheap model under winning packet)
    instead of only perturbing."""
    donor = proposal.get("donor")
    slot = proposal.get("slot")
    if not archive_manifests or donor not in archive_manifests:
        raise ValueError(f"unknown transplant donor '{donor}'")
    snap = archive_manifests[donor]
    if slot == "prompt_packet":
        value = snap.get("prompt_packet_text")
    elif slot == "agents_md":
        value = snap.get("agents_md_text")
    else:
        value = snap.get(slot)
    if value is None:
        raise ValueError(f"donor '{donor}' has no value for slot '{slot}'")
    out = dict(proposal)
    out["value"] = value
    return out


def proposal_instructions(tool_policies=None, allowed_models=None,
                          allowed_thinking=None, avoid_slots=(),
                          skill_sets=None, mode=None, donors=None):
    """Compose the slot menu from the declared search space, so the
    optimizer can only propose values the space contains. The brief asks
    for the highest-information experiment under the declared mode — no
    slot is privileged."""
    lines = [
        "You are the search step of an agent-optimization loop. Your job: "
        "propose",
        "EXACTLY ONE change to ONE slot of the candidate composition below — "
        "the",
        "highest-information single-variable experiment available given the "
        "evidence",
        f"and the objective mode ({mode or 'max-quality'}). Never change two "
        "things.",
        "",
        "Mutable slots and value rules:",
        '- "prompt_packet": value is the FULL replacement packet text '
        "(system-prompt",
        "  guidance for the review agent).",
    ]
    if allowed_models:
        lines.append(f'- "model": one of {json.dumps(sorted(allowed_models))}.')
    else:
        lines.append('- "model": an OpenRouter model id string.')
    if allowed_thinking:
        lines.append(
            f'- "thinking": one of {json.dumps(sorted(allowed_thinking))}.'
        )
    else:
        lines.append('- "thinking": one of off|minimal|low|medium|high|xhigh.')
    lines.append(
        '- "system_prompt_mode": "append" (packet added to pi\'s default '
        'coding prompt) or "replace" (packet IS the whole system prompt).'
    )
    lines.append(
        '- "agents_md": the FULL text of an AGENTS.md placed in the agent\'s '
        "workspace root (repo-context briefing it reads on startup)."
    )
    if tool_policies:
        lines.append(
            f'- "tools": one of the named tool policies '
            f"{json.dumps(sorted(tool_policies))} (value is the policy name)."
        )
    if skill_sets:
        lines.append(
            f'- "skills": one of the named skill sets '
            f"{json.dumps(sorted(skill_sets))} (value is the set name)."
        )
    if donors:
        lines += [
            "",
            "You may instead TRANSPLANT one slot's value from another "
            "archive candidate",
            f"(donors: {json.dumps(sorted(donors))}) by adding "
            '"donor": "<candidate_id>"',
            'and omitting "value" — e.g. move a strong packet onto a cheaper '
            "model, or a",
            "cheap model under a winning packet. Still exactly one slot.",
        ]
    if avoid_slots:
        lines += [
            "",
            "Competing hypotheses this generation already target: "
            f"{sorted(set(avoid_slots))}. Propose a DIFFERENT slot.",
        ]
    lines += [
        "",
        "Respond with ONLY a JSON object:",
        '{"slot": "<slot>", "value": <value> (or "donor": "<candidate_id>"),',
        ' "hypothesis": "<one or two sentences: what evidence this addresses '
        'and why>",',
        ' "predicted_effect": {"reward": "up|hold", "cost": "down|hold|up"}}',
        "",
        "predicted_effect is your testable prediction; it will be scored "
        "against the",
        "measured outcome and recorded in the lab notebook.",
    ]
    return "\n".join(lines)


def worst_trials(records, candidate_id, n=3):
    """The candidate's lowest-reward trials, worst first."""
    own = [r for r in records if r["candidate_id"] == candidate_id]
    return sorted(own, key=lambda r: (r["reward"], -r["wall_ms"]))[:n]


def evidence_block(trials, exp_dir, transcript_chars=3000):
    parts = []
    for r in trials:
        matched = len(r.get("matched") or [])
        expected = r.get("expected_defects")
        # The scorer's verdict structure (never the key itself): how many
        # seeded defects were matched vs missed, and how many findings were
        # penalized as false positives — so the proposer reasons from real
        # failure shape instead of guessing it from transcripts.
        parts.append(
            f"### Trial {r['run_id']}\n"
            f"task: {r['task_id']}  reward: {r['reward']}  "
            f"verdict: matched {matched} of {expected} seeded defects, "
            f"{r.get('false_positives')} finding(s) penalized as false "
            f"positives  error: {r.get('error')}\n"
            f"findings: {json.dumps(r.get('findings'))[:800]}\n"
        )
        art = r.get("artifacts")
        if art and exp_dir:
            transcript = exp_dir / art / "transcript.txt"
            if transcript.exists():
                tail = transcript.read_text()[-transcript_chars:]
                parts.append(f"transcript tail:\n```\n{tail}\n```\n")
    return "\n".join(parts)


def build_prompt(taskspec, parent_snapshot, trials_evidence, archive_summary,
                 instructions=None):
    slots = {
        k: parent_snapshot.get(k)
        for k in ("model", "thinking", "tools", "kind")
    }
    return (
        (instructions or proposal_instructions())
        + "\n## Task\n"
        + f"goal: {taskspec.get('goal')}\nmode: {taskspec.get('mode')}\n"
        + "\n## Candidate composition (parent)\n"
        + json.dumps(slots, indent=2)
        + "\n\ncurrent prompt_packet text:\n---\n"
        + (parent_snapshot.get("prompt_packet_text") or "(none)")
        + "\n---\n"
        + "\n## Archive (what has been tried)\n"
        + json.dumps(archive_summary, indent=2)[:2000]
        + "\n\n## Worst-trial evidence\n"
        + trials_evidence
    )


def call_optimizer(prompt, model, timeout=180, retries=3):
    """Call the optimizer model, retrying transient errors with backoff so a
    flaky network does not consume the loop's proposal-failure budget."""
    key = os.environ.get("OPENROUTER_API_KEY")
    if not key:
        raise RuntimeError("OPENROUTER_API_KEY is not set")
    # Reasoning models spend tokens before emitting content; give generous
    # headroom so the JSON proposal is not truncated to empty.
    body = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.7,
        "max_tokens": 16384,
        "usage": {"include": True},
    }
    last_exc = None
    for attempt in range(retries):
        req = urllib.request.Request(
            OPENROUTER_URL,
            data=json.dumps(body).encode(),
            headers={
                "Authorization": f"Bearer {key}",
                "Content-Type": "application/json",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                payload = json.loads(resp.read())
            choice = payload["choices"][0]
            msg = choice["message"]
            # Some reasoning models put the answer in reasoning when content is
            # empty; fall back to it so a parseable proposal is not lost.
            content = msg.get("content") or msg.get("reasoning") or ""
            if not content.strip():
                raise RuntimeError(
                    f"optimizer returned empty content "
                    f"(finish_reason={choice.get('finish_reason')})"
                )
            return content, (payload.get("usage") or {}).get("cost")
        except Exception as exc:  # noqa: BLE001 - retry with backoff
            last_exc = exc
            if attempt < retries - 1:
                time.sleep(2 ** attempt)
    raise RuntimeError(f"optimizer call failed after {retries} attempts: {last_exc}")


def parse_proposal(text):
    start = text.find("{")
    while start != -1:
        depth = 0
        in_str = False
        escape = False
        for i in range(start, len(text)):
            ch = text[i]
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_str = not in_str
            elif not in_str:
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        try:
                            return json.loads(text[start : i + 1])
                        except json.JSONDecodeError:
                            break
        start = text.find("{", start + 1)
    raise ValueError("optimizer returned no parseable proposal")


def validate_proposal(proposal, parent_manifest, tool_policies=None,
                      allowed_models=None, allowed_thinking=None,
                      avoid_slots=(), skill_sets=None, donor=None):
    """Reject anything that is not a well-formed single-slot mutation drawn
    from the declared search space."""
    slot = proposal.get("slot")
    value = proposal.get("value")
    hypothesis = proposal.get("hypothesis")
    if slot not in MUTABLE_SLOTS:
        raise ValueError(f"slot '{slot}' is not mutable (allowed: {sorted(MUTABLE_SLOTS)})")
    if slot in avoid_slots:
        raise ValueError(
            f"slot '{slot}' already targeted by a competing hypothesis "
            "this generation"
        )
    if not hypothesis or not str(hypothesis).strip():
        raise ValueError("proposal missing hypothesis")
    if slot == "prompt_packet":
        if not is_sane_prompt_packet(value):
            raise ValueError("prompt_packet value failed sanity check")
    elif slot == "model":
        if not isinstance(value, str) or "/" not in value:
            raise ValueError("model value must be an OpenRouter model id")
        if allowed_models and value not in allowed_models:
            raise ValueError(
                f"model '{value}' is outside the declared search space"
            )
        if value == parent_manifest.get("model"):
            raise ValueError("model mutation must differ from parent")
    elif slot == "thinking":
        if value not in THINKING_LEVELS:
            raise ValueError(f"thinking must be one of {sorted(THINKING_LEVELS)}")
        if allowed_thinking and value not in allowed_thinking:
            raise ValueError(
                f"thinking '{value}' is outside the declared search space"
            )
        if value == parent_manifest.get("thinking"):
            raise ValueError("thinking mutation must differ from parent")
    elif slot == "tools":
        if donor:
            if list(value) == list(parent_manifest.get("tools") or []):
                raise ValueError("tools transplant must differ from parent")
        else:
            if not tool_policies:
                raise ValueError("tools mutation requires declared tool_policies")
            if value not in tool_policies:
                raise ValueError(
                    f"tools value must be a policy name from {sorted(tool_policies)}"
                )
            if list(tool_policies[value]) == list(parent_manifest.get("tools") or []):
                raise ValueError("tools mutation must differ from parent")
    elif slot == "system_prompt_mode":
        if value not in SYSTEM_PROMPT_MODES:
            raise ValueError(
                f"system_prompt_mode must be one of {sorted(SYSTEM_PROMPT_MODES)}"
            )
        if value == parent_manifest.get("system_prompt_mode", "append"):
            raise ValueError("system_prompt_mode mutation must differ from parent")
    elif slot == "agents_md":
        if not isinstance(value, str) or len(value.strip()) < 20:
            raise ValueError("agents_md value must be substantial briefing text")
    elif slot == "skills":
        if donor:
            if list(value) == list(parent_manifest.get("skills") or []):
                raise ValueError("skills transplant must differ from parent")
        else:
            if not skill_sets:
                raise ValueError("skills mutation requires declared skill_sets")
            if value not in skill_sets:
                raise ValueError(
                    f"skills value must be a set name from {sorted(skill_sets)}"
                )
            if list(skill_sets[value]) == list(parent_manifest.get("skills") or []):
                raise ValueError("skills mutation must differ from parent")
    return slot, value, str(hypothesis).strip()


def build_child(parent_manifest, slot, value, child_id, packets_dir,
                tool_policies=None, skill_sets=None):
    """Materialize the child manifest (and any mutated text file). tools and
    skills mutations carry the *name* of a declared set; the manifest gets
    the resolved list. prompt_packet and agents_md carry full text, written
    to versioned files."""
    child = {
        k: v
        for k, v in parent_manifest.items()
        if not k.startswith("_") and k not in ("id",)
    }
    child["id"] = child_id

    def write_text_slot(suffix, text):
        packets_dir.mkdir(parents=True, exist_ok=True)
        path = packets_dir / f"{child_id}{suffix}"
        path.write_text(text if text.endswith("\n") else text + "\n")
        return str(path)

    if slot == "prompt_packet":
        child["prompt_packet"] = write_text_slot(".md", value)
    elif slot == "agents_md":
        child["agents_md"] = write_text_slot("-agents.md", value)
    elif slot == "tools":
        child["tools"] = (
            list(value) if isinstance(value, list) else list(tool_policies[value])
        )
    elif slot == "skills":
        child["skills"] = (
            list(value) if isinstance(value, list) else list(skill_sets[value])
        )
    else:
        child[slot] = value
    return child


def write_manifest(child, path):
    lines = []
    for key, val in child.items():
        if isinstance(val, bool):
            rendered = "true" if val else "false"
        elif isinstance(val, (int, float)):
            rendered = json.dumps(val)
        elif isinstance(val, list):
            rendered = json.dumps(val)
        else:
            rendered = json.dumps(str(val))
        lines.append(f"{key} = {rendered}")
    path.write_text("\n".join(lines) + "\n")
    return path


def propose(taskspec, parent_snapshot, parent_manifest, records, exp_dir,
            child_id, optimizer_model, packets_dir, manifests_dir,
            archive_summary=None, tool_policies=None, allowed_models=None,
            allowed_thinking=None, avoid_slots=(), skill_sets=None,
            archive_manifests=None, mode=None):
    """Full step: evidence → LLM proposal → validation → child on disk.
    Returns (manifest_path, metadata)."""
    trials = worst_trials(records, parent_snapshot["id"])
    donors = sorted(
        cid for cid in (archive_manifests or {})
        if cid != parent_snapshot["id"]
    )
    prompt = build_prompt(
        taskspec,
        parent_snapshot,
        evidence_block(trials, exp_dir),
        archive_summary=archive_summary or {},
        instructions=proposal_instructions(
            tool_policies=tool_policies,
            allowed_models=allowed_models,
            allowed_thinking=allowed_thinking,
            avoid_slots=avoid_slots,
            skill_sets=skill_sets,
            mode=mode,
            donors=donors,
        ),
    )
    content, cost = call_optimizer(prompt, optimizer_model)
    proposal = parse_proposal(content)
    predicted_effect, pe_defaulted = normalize_predicted_effect(proposal)
    donor = proposal.get("donor")
    if donor is not None:
        proposal = resolve_donor(proposal, archive_manifests or {},
                                 parent_manifest)
    slot, value, hypothesis = validate_proposal(
        proposal, parent_manifest,
        tool_policies=tool_policies,
        allowed_models=allowed_models,
        allowed_thinking=allowed_thinking,
        avoid_slots=avoid_slots,
        skill_sets=skill_sets,
        donor=donor,
    )
    child = build_child(parent_manifest, slot, value, child_id, packets_dir,
                        tool_policies=tool_policies, skill_sets=skill_sets)
    manifests_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = write_manifest(child, manifests_dir / f"{child_id}.toml")
    meta = {
        "child_id": child_id,
        "parent_id": parent_snapshot["id"],
        "slot_changed": slot,
        "value_summary": (
            "(new text)" if slot in ("prompt_packet", "agents_md") else value
        ),
        "hypothesis": hypothesis,
        "predicted_effect": predicted_effect,
        "optimizer_cost_usd": cost,
    }
    if pe_defaulted:
        meta["predicted_effect_defaulted"] = True
    if donor:
        meta["donor"] = donor
    return manifest_path, meta
