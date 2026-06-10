"""Mutation-step tests: proposal parsing, single-slot validation, child
materialization. No network — the LLM call itself is not under test."""

import sys
import tomllib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import mutate  # noqa: E402

PARENT = {
    "composition": 1,
    "id": "pi-kimi",
    "kind": "pi",
    "model": "moonshotai/kimi-k2.6",
    "prompt_packet": "packets/reviewer-v1.md",
    "thinking": "medium",
    "tools": ["read", "bash", "edit", "write"],
    "timeout_sec": 600,
}

POLICIES = {
    "full": ["read", "bash", "edit", "write"],
    "explore": ["read", "bash"],
}


def test_parse_proposal_with_fences_and_braces_in_strings():
    text = (
        'Reasoning...\n```json\n{"slot": "prompt_packet", '
        '"value": "Use {curly} braces and review carefully always.", '
        '"hypothesis": "more specific"}\n```'
    )
    p = mutate.parse_proposal(text)
    assert p["slot"] == "prompt_packet"
    assert "{curly}" in p["value"]


def test_validate_rejects_unknown_and_frozen_slots():
    # temperature/max_tokens are frozen: pi has no flag for them, so a
    # "mutation" there changes the hash without changing behavior.
    for slot in ("kind", "env_allowlist", "temperature", "max_tokens",
                 "nonsense"):
        with pytest.raises(ValueError, match="not mutable"):
            mutate.validate_proposal(
                {"slot": slot, "value": "x", "hypothesis": "h"}, PARENT
            )


def test_validate_rejects_no_op_mutations():
    with pytest.raises(ValueError, match="differ from parent"):
        mutate.validate_proposal(
            {"slot": "thinking", "value": "medium", "hypothesis": "h"}, PARENT
        )
    with pytest.raises(ValueError, match="differ from parent"):
        mutate.validate_proposal(
            {"slot": "model", "value": "moonshotai/kimi-k2.6", "hypothesis": "h"},
            PARENT,
        )


def test_validate_rejects_thin_packet_and_missing_hypothesis():
    with pytest.raises(ValueError, match="substantial"):
        mutate.validate_proposal(
            {"slot": "prompt_packet", "value": "be good", "hypothesis": "h"}, PARENT
        )
    with pytest.raises(ValueError, match="hypothesis"):
        mutate.validate_proposal(
            {"slot": "thinking", "value": "high", "hypothesis": " "}, PARENT
        )


def test_validate_bounds():
    with pytest.raises(ValueError):
        mutate.validate_proposal(
            {"slot": "thinking", "value": "ultra", "hypothesis": "h"}, PARENT
        )


def test_validate_model_must_be_in_search_space():
    with pytest.raises(ValueError, match="search space"):
        mutate.validate_proposal(
            {"slot": "model", "value": "made-up/model", "hypothesis": "h"},
            PARENT,
            allowed_models=["z-ai/glm-5", "moonshotai/kimi-k2.6"],
        )
    slot, value, _ = mutate.validate_proposal(
        {"slot": "model", "value": "z-ai/glm-5", "hypothesis": "h"},
        PARENT,
        allowed_models=["z-ai/glm-5", "moonshotai/kimi-k2.6"],
    )
    assert (slot, value) == ("model", "z-ai/glm-5")


def test_validate_tools_policy_mutation():
    # Unknown policy name rejected; same-as-parent rejected; valid accepted.
    with pytest.raises(ValueError, match="policy name"):
        mutate.validate_proposal(
            {"slot": "tools", "value": "yolo", "hypothesis": "h"},
            PARENT, tool_policies=POLICIES,
        )
    with pytest.raises(ValueError, match="differ from parent"):
        mutate.validate_proposal(
            {"slot": "tools", "value": "full", "hypothesis": "h"},
            PARENT, tool_policies=POLICIES,
        )
    with pytest.raises(ValueError, match="tool_policies"):
        mutate.validate_proposal(
            {"slot": "tools", "value": "explore", "hypothesis": "h"}, PARENT
        )
    slot, value, _ = mutate.validate_proposal(
        {"slot": "tools", "value": "explore", "hypothesis": "h"},
        PARENT, tool_policies=POLICIES,
    )
    assert (slot, value) == ("tools", "explore")


def test_validate_system_prompt_mode_mutation():
    with pytest.raises(ValueError, match="append"):
        mutate.validate_proposal(
            {"slot": "system_prompt_mode", "value": "yolo", "hypothesis": "h"},
            PARENT,
        )
    with pytest.raises(ValueError, match="differ from parent"):
        # parent has no explicit mode -> defaults to append
        mutate.validate_proposal(
            {"slot": "system_prompt_mode", "value": "append", "hypothesis": "h"},
            PARENT,
        )
    slot, value, _ = mutate.validate_proposal(
        {"slot": "system_prompt_mode", "value": "replace", "hypothesis": "h"},
        PARENT,
    )
    assert (slot, value) == ("system_prompt_mode", "replace")


def test_validate_agents_md_and_build_child_writes_file(tmp_path):
    with pytest.raises(ValueError, match="substantial"):
        mutate.validate_proposal(
            {"slot": "agents_md", "value": "hi", "hypothesis": "h"}, PARENT
        )
    text = "Workspace briefing: trace callers across modules before judging."
    slot, value, _ = mutate.validate_proposal(
        {"slot": "agents_md", "value": text, "hypothesis": "h"}, PARENT
    )
    child = mutate.build_child(PARENT, slot, value, "gen9", tmp_path)
    assert "trace callers" in Path(child["agents_md"]).read_text()
    assert child["prompt_packet"] == PARENT["prompt_packet"]


def test_validate_skills_mutation_against_declared_sets(tmp_path):
    sets = {"review-pack": ["packets/skill-a.md"], "bare": []}
    with pytest.raises(ValueError, match="skill_sets"):
        mutate.validate_proposal(
            {"slot": "skills", "value": "review-pack", "hypothesis": "h"},
            PARENT,
        )
    with pytest.raises(ValueError, match="set name"):
        mutate.validate_proposal(
            {"slot": "skills", "value": "nonsense", "hypothesis": "h"},
            PARENT, skill_sets=sets,
        )
    slot, value, _ = mutate.validate_proposal(
        {"slot": "skills", "value": "review-pack", "hypothesis": "h"},
        PARENT, skill_sets=sets,
    )
    child = mutate.build_child(PARENT, slot, value, "gen10", tmp_path,
                               skill_sets=sets)
    assert child["skills"] == ["packets/skill-a.md"]


def test_validate_rejects_slot_taken_by_competing_hypothesis():
    with pytest.raises(ValueError, match="competing"):
        mutate.validate_proposal(
            {"slot": "thinking", "value": "high", "hypothesis": "h"},
            PARENT, avoid_slots=("thinking",),
        )


def test_validate_accepts_good_packet_mutation():
    slot, value, hyp = mutate.validate_proposal(
        {
            "slot": "prompt_packet",
            "value": "You are a meticulous reviewer. Always cross-check callers.",
            "hypothesis": "missed cross-file defects",
        },
        PARENT,
    )
    assert slot == "prompt_packet"
    assert hyp == "missed cross-file defects"


def test_build_child_packet_mutation_roundtrips(tmp_path):
    child = mutate.build_child(
        PARENT,
        "prompt_packet",
        "New packet text that is long enough to be substantial.",
        "gen1-pi-kimi",
        packets_dir=tmp_path / "packets",
    )
    manifest_path = mutate.write_manifest(child, tmp_path / "gen1.toml")
    loaded = tomllib.loads(manifest_path.read_text())
    assert loaded["id"] == "gen1-pi-kimi"
    assert loaded["kind"] == "pi"
    assert loaded["timeout_sec"] == 600
    packet = Path(loaded["prompt_packet"])
    assert packet.exists()
    assert "substantial" in packet.read_text()


def test_build_child_scalar_mutation(tmp_path):
    child = mutate.build_child(PARENT, "thinking", "high", "gen2", tmp_path)
    assert child["thinking"] == "high"
    assert child["prompt_packet"] == PARENT["prompt_packet"]


def test_build_child_tools_mutation_resolves_policy_name(tmp_path):
    child = mutate.build_child(PARENT, "tools", "explore", "gen3", tmp_path,
                               tool_policies=POLICIES)
    assert child["tools"] == ["read", "bash"]
    assert child["model"] == PARENT["model"]


def test_parse_proposal_recovers_from_reasoning_style_text():
    # The optimizer sometimes wraps JSON in prose/reasoning; the parser must
    # still recover the proposal object.
    text = (
        "Let me think. The agent missed cross-file context, so:\n\n"
        '{"slot": "model", "value": "anthropic/claude-x", '
        '"hypothesis": "stronger model for cross-file reasoning"}\n\n'
        "That should help."
    )
    p = mutate.parse_proposal(text)
    assert p["slot"] == "model"
    assert p["value"] == "anthropic/claude-x"


def test_worst_trials_orders_by_reward():
    records = [
        {"candidate_id": "x", "reward": 1.0, "wall_ms": 1, "run_id": "a"},
        {"candidate_id": "x", "reward": 0.0, "wall_ms": 1, "run_id": "b"},
        {"candidate_id": "y", "reward": 0.0, "wall_ms": 1, "run_id": "c"},
        {"candidate_id": "x", "reward": 0.5, "wall_ms": 1, "run_id": "d"},
    ]
    worst = mutate.worst_trials(records, "x", n=2)
    assert [w["run_id"] for w in worst] == ["b", "d"]
