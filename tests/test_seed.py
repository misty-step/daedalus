"""Seeder tests: deterministic diverse sampling, packet authoring with
fallback, manifest materialization. No network — the optimizer call is
injected."""

import random
import sys
import tomllib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import seed  # noqa: E402

SEARCH = {
    "models": [
        "deepseek/deepseek-v4-flash",
        "z-ai/glm-4.7-flash",
        "openai/gpt-5-mini",
        "moonshotai/kimi-k2.6",
        "z-ai/glm-5",
        "qwen/qwen3.7-plus",
    ],
    "thinking_levels": ["off", "low", "medium", "high"],
    "tool_policies": {
        "full": ["read", "bash", "edit", "write"],
        "explore": ["read", "bash"],
        "no-exec": ["read", "edit", "write"],
    },
    "packet_stances": 3,
    "seed_count": 6,
}

SPEC = {
    "goal": "find the real defects",
    "search": SEARCH,
    "budget": {"max_wall_per_trial_sec": 300},
}


def fake_call(prompt, model):
    stance = "generic"
    for name, brief in seed.STANCES:
        if brief in prompt:
            stance = name
    return f"You are a {stance} reviewer. Ground every finding in evidence.", 0.001


def test_sampling_spans_the_axes_and_is_deterministic():
    a = seed.sample_compositions(SEARCH, 6, random.Random(7))
    b = seed.sample_compositions(SEARCH, 6, random.Random(7))
    assert a == b  # reproducible given the recorded rng seed
    assert len({c["model"] for c in a}) == 6  # all six models distinct
    assert len({c["thinking"] for c in a}) >= 3
    assert len({c["policy_name"] for c in a}) == 3
    different = seed.sample_compositions(SEARCH, 6, random.Random(8))
    assert different != a  # a new seed rolls new dice


def test_seed_population_materializes_hashed_pi_manifests(tmp_path):
    seeds, meta = seed.seed_population(
        SPEC, "opt-model", tmp_path / "packets", tmp_path / "manifests",
        rng_seed=42, call=fake_call,
    )
    assert len(seeds) == 6
    assert meta["rng_seed"] == 42
    assert len(meta["packet_stances"]) == 3
    assert meta["optimizer_costs"] == [0.001] * 3
    ids = [sid for sid, _ in seeds]
    assert len(set(ids)) == 6
    for sid, path in seeds:
        m = tomllib.loads(Path(path).read_text())
        assert m["kind"] == "pi"
        assert m["model"] in SEARCH["models"]
        assert m["thinking"] in SEARCH["thinking_levels"]
        assert m["tools"] in list(SEARCH["tool_policies"].values())
        assert m["timeout_sec"] == 300
        # pi has no temperature/max_tokens flag; a seed must not carry them
        assert "temperature" not in m and "max_tokens" not in m
        assert Path(m["prompt_packet"]).read_text().strip()


def test_packet_author_failure_falls_back_to_base(tmp_path):
    base = tmp_path / "base.md"
    base.write_text("Base reviewer packet.\n")
    spec = dict(SPEC, search=dict(SEARCH, base_packet=None))

    def broken_call(prompt, model):
        raise RuntimeError("optimizer down")

    with pytest.raises(RuntimeError):
        seed.author_packets(spec, 2, "m", random.Random(1),
                            tmp_path / "p1", call=broken_call)
    packets, costs = seed.author_packets(
        spec, 2, "m", random.Random(1), tmp_path / "p2",
        call=broken_call, fallback_text=base.read_text(),
    )
    assert len(packets) == 2
    assert costs == []  # no successful optimizer spend
    for _, path in packets:
        assert path.read_text() == "Base reviewer packet.\n"


def test_missing_models_raises(tmp_path):
    with pytest.raises(ValueError, match="models"):
        seed.seed_population({"search": {}}, "m", tmp_path, tmp_path,
                             rng_seed=1, call=fake_call)
