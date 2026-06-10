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

REPO = Path(__file__).resolve().parent.parent
OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"

# temperature/max_tokens are NOT mutable: pi exposes no flag for either
# (docs/primitives.md), so mutating them would change the composition hash
# without changing behavior — false attribution by construction.
MUTABLE_SLOTS = {"prompt_packet", "model", "thinking", "tools"}
THINKING_LEVELS = {"off", "minimal", "low", "medium", "high", "xhigh"}


def proposal_instructions(tool_policies=None, allowed_models=None,
                          avoid_slots=()):
    """Compose the slot menu from the taskspec's declared search space, so
    the optimizer can only propose values the space contains."""
    lines = [
        "You are the search step of an agent-optimization loop. Your job: "
        "propose",
        "EXACTLY ONE change to ONE slot of the candidate composition below, "
        "grounded",
        "in the failure evidence. Single-variable experiments only — never "
        "change two",
        "things.",
        "",
        "Mutable slots and value rules:",
        '- "prompt_packet": value is the FULL replacement packet text '
        "(system-prompt",
        "  guidance for the review agent). Most mutations should target this "
        "slot.",
    ]
    if allowed_models:
        lines.append(f'- "model": one of {json.dumps(sorted(allowed_models))}.')
    else:
        lines.append('- "model": an OpenRouter model id string.')
    lines.append('- "thinking": one of off|minimal|low|medium|high|xhigh.')
    if tool_policies:
        lines.append(
            f'- "tools": one of the named tool policies '
            f"{json.dumps(sorted(tool_policies))} (value is the policy name)."
        )
    if avoid_slots:
        lines += [
            "",
            "Competing hypotheses this generation already target: "
            f"{sorted(set(avoid_slots))}. Propose a DIFFERENT slot.",
        ]
    lines += [
        "",
        "Respond with ONLY a JSON object:",
        '{"slot": "<slot>", "value": <value>, "hypothesis": "<one or two '
        "sentences:",
        'what failure this addresses and why this change should fix it>"}',
    ]
    return "\n".join(lines)


def worst_trials(records, candidate_id, n=3):
    """The candidate's lowest-reward trials, worst first."""
    own = [r for r in records if r["candidate_id"] == candidate_id]
    return sorted(own, key=lambda r: (r["reward"], -r["wall_ms"]))[:n]


def evidence_block(trials, exp_dir, transcript_chars=3000):
    parts = []
    for r in trials:
        parts.append(
            f"### Trial {r['run_id']}\n"
            f"task: {r['task_id']}  reward: {r['reward']}  "
            f"expected defects: {r.get('expected_defects')}  "
            f"false positives: {r.get('false_positives')}  "
            f"error: {r.get('error')}\n"
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
                      allowed_models=None, avoid_slots=()):
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
        if not isinstance(value, str) or len(value.strip()) < 20:
            raise ValueError("prompt_packet value must be substantial packet text")
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
        if value == parent_manifest.get("thinking"):
            raise ValueError("thinking mutation must differ from parent")
    elif slot == "tools":
        if not tool_policies:
            raise ValueError("tools mutation requires declared tool_policies")
        if value not in tool_policies:
            raise ValueError(
                f"tools value must be a policy name from {sorted(tool_policies)}"
            )
        if list(tool_policies[value]) == list(parent_manifest.get("tools") or []):
            raise ValueError("tools mutation must differ from parent")
    return slot, value, str(hypothesis).strip()


def build_child(parent_manifest, slot, value, child_id, packets_dir,
                tool_policies=None):
    """Materialize the child manifest (and packet file when mutated). A
    tools mutation carries the policy *name* as value; the manifest gets the
    resolved tool list."""
    child = {
        k: v
        for k, v in parent_manifest.items()
        if not k.startswith("_") and k not in ("id",)
    }
    child["id"] = child_id
    if slot == "prompt_packet":
        packets_dir.mkdir(parents=True, exist_ok=True)
        packet_path = packets_dir / f"{child_id}.md"
        packet_path.write_text(value if value.endswith("\n") else value + "\n")
        child["prompt_packet"] = str(packet_path)
    elif slot == "tools":
        child["tools"] = list(tool_policies[value])
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
            avoid_slots=()):
    """Full step: evidence → LLM proposal → validation → child on disk.
    Returns (manifest_path, metadata)."""
    trials = worst_trials(records, parent_snapshot["id"])
    prompt = build_prompt(
        taskspec,
        parent_snapshot,
        evidence_block(trials, exp_dir),
        archive_summary=archive_summary or {},
        instructions=proposal_instructions(
            tool_policies=tool_policies,
            allowed_models=allowed_models,
            avoid_slots=avoid_slots,
        ),
    )
    content, cost = call_optimizer(prompt, optimizer_model)
    proposal = parse_proposal(content)
    slot, value, hypothesis = validate_proposal(
        proposal, parent_manifest,
        tool_policies=tool_policies,
        allowed_models=allowed_models,
        avoid_slots=avoid_slots,
    )
    child = build_child(parent_manifest, slot, value, child_id, packets_dir,
                        tool_policies=tool_policies)
    manifests_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = write_manifest(child, manifests_dir / f"{child_id}.toml")
    meta = {
        "child_id": child_id,
        "parent_id": parent_snapshot["id"],
        "slot_changed": slot,
        "value_summary": value if slot != "prompt_packet" else "(new packet)",
        "hypothesis": hypothesis,
        "optimizer_cost_usd": cost,
    }
    return manifest_path, meta
