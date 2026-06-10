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
    "timeout_sec": 600,
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
    for slot in ("kind", "tools", "env_allowlist", "nonsense"):
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
            {"slot": "temperature", "value": 0.5, "hypothesis": " "}, PARENT
        )


def test_validate_bounds():
    with pytest.raises(ValueError):
        mutate.validate_proposal(
            {"slot": "temperature", "value": 3.0, "hypothesis": "h"}, PARENT
        )
    with pytest.raises(ValueError):
        mutate.validate_proposal(
            {"slot": "max_tokens", "value": 64, "hypothesis": "h"}, PARENT
        )
    with pytest.raises(ValueError):
        mutate.validate_proposal(
            {"slot": "thinking", "value": "ultra", "hypothesis": "h"}, PARENT
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
