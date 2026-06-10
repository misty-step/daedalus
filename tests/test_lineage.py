"""Lineage tests: the run's story renders from its artifacts alone."""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import lineage  # noqa: E402


def build_run(tmp_path):
    exp = tmp_path / "20260610T000000Z-search-demo"
    exp.mkdir()
    (exp / "rig.json").write_text(json.dumps({
        "oracle_mean": 1.0, "null_mean": 0.25, "probe_mean": 0.0,
        "saturated": False,
    }))
    (exp / "seed.json").write_text(json.dumps({
        "rng_seed": 7, "seed_count": 2,
        "packet_stances": ["spec-first", "skeptic"],
        "optimizer_costs": [0.01],
        "combos": [
            {"model": "z-ai/glm-5", "thinking": "high", "policy_name": "full"},
            {"model": "openai/gpt-5-mini", "thinking": "low",
             "policy_name": "explore"},
        ],
    }))
    (exp / "loop.json").write_text(json.dumps({
        "stop_reason": "plateau", "mode": "threshold-then-cheap",
        "generations": 2, "spend_known_usd": 1.23,
        "certified": ["seed1-glm-5-spec-first"],
        "alarms": [{"kind": "saturation-at-top",
                    "detail": "seed1 at ceiling; cost search only"}],
        "history": [
            {"generation": 1, "attempt": 0, "child_id": "g1a-seed1",
             "parent_id": "seed1-glm-5-spec-first",
             "slot_changed": "thinking",
             "hypothesis": "medium thinking keeps reward and cuts cost",
             "parent_reward_mean": 1.0, "reward_mean": 1.0,
             "mean_task_delta": 0.0, "improved": True},
            {"generation": 2, "attempt": 0, "child_id": "g2a-g1a",
             "parent_id": "g1a-seed1", "slot_changed": "prompt_packet",
             "hypothesis": "stop instruction reduces spend",
             "parent_reward_mean": 1.0, "reward_mean": 0.66,
             "mean_task_delta": -0.33, "improved": False},
            {"generation": 2, "attempt": 1,
             "parent_id": "g1a-seed1", "proposal_error": "slot not mutable"},
        ],
    }))
    (exp / "pareto.json").write_text(json.dumps([
        {"candidate_id": "g1a-seed1", "composition_hash": "abc123",
         "reward_mean": 1.0, "cost_usd_per_trial": 0.0138,
         "certified": True, "recommended": True},
    ]))
    trials = [
        {"candidate_id": "seed1-glm-5-spec-first", "candidate_kind": "pi",
         "task_id": "t1", "reward": 1.0, "cost_usd": 0.02},
        {"candidate_id": "seed2-gpt-5-mini-skeptic", "candidate_kind": "pi",
         "task_id": "t1", "reward": 0.2, "cost_usd": 0.01},
        {"candidate_id": "oracle", "candidate_kind": "oracle",
         "task_id": "t1", "reward": 1.0, "cost_usd": None},
    ]
    (exp / "trials.jsonl").write_text(
        "\n".join(json.dumps(t) for t in trials) + "\n"
    )
    return exp


def test_render_tells_the_whole_story(tmp_path):
    text = lineage.render(build_run(tmp_path))
    # rig
    assert "arena discriminates" in text
    # landscape with combos matched to seeds by index
    assert "z-ai/glm-5" in text and "spec-first" in text
    # hypotheses with verdicts
    assert "medium thinking keeps reward and cuts cost" in text
    assert "[confirmed]" in text
    assert "[not confirmed (Δ -0.33)]" in text
    assert "proposal rejected" in text
    # alarms, certification, recommendation
    assert "saturation-at-top" in text
    assert "certified: seed1-glm-5-spec-first" in text
    assert "← **recommended**" in text


def test_render_survives_missing_artifacts(tmp_path):
    exp = tmp_path / "bare"
    exp.mkdir()
    text = lineage.render(exp)
    assert "no rig.json recorded" in text
    assert "no search generations recorded" in text


def test_notebook_entry_summarizes(tmp_path):
    exp = build_run(tmp_path)
    entry = lineage.notebook_entry(
        exp, {"id": "pr-review", "mode": "threshold-then-cheap"},
        {"id": "pr-review-v2", "version": "0.1.0"},
    )
    assert "pr-review-v2" in entry
    assert "g1a-seed1" in entry
    assert "certified=True" in entry
    assert "lineage.md" in entry
