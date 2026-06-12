"""Comparison-report tests: aggregation, Pareto dominance, recommendation."""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import report  # noqa: E402


def record(cand, task, reward, cost, wall_ms, error=None, kind="pi"):
    return {
        "candidate_id": cand,
        "candidate_kind": kind,
        "composition_hash": f"hash-{cand}",
        "model": "m",
        "task_id": task,
        "reward": reward,
        "cost_usd": cost,
        "wall_ms": wall_ms,
        "error": error,
    }


def write_trials(tmp_path, records):
    exp = tmp_path / "exp"
    exp.mkdir()
    (exp / "trials.jsonl").write_text(
        "\n".join(json.dumps(r) for r in records) + "\n"
    )
    return exp


def test_pareto_excludes_dominated_candidate(tmp_path):
    records = [
        # good: high reward, cheap
        record("good", "t1", 1.0, 0.01, 1000),
        record("good", "t2", 0.8, 0.01, 1000),
        # dominated: same tasks, worse reward, pricier, slower
        record("worse", "t1", 0.5, 0.05, 5000),
        record("worse", "t2", 0.4, 0.05, 5000),
        # tradeoff: lower reward but cheaper -> stays on the front
        record("cheap", "t1", 0.6, 0.001, 500),
        record("cheap", "t2", 0.6, 0.001, 500),
    ]
    cands = report.aggregate(records)
    front = report.pareto_front(cands)
    assert "good" in front
    assert "cheap" in front
    assert "worse" not in front


def test_reference_candidates_never_in_front_or_pick(tmp_path):
    records = [
        record("oracle", "t1", 1.0, None, 1, kind="oracle"),
        record("null", "t1", 0.0, None, 1, kind="null"),
        record("real", "t1", 0.7, 0.02, 2000),
    ]
    cands = report.aggregate(records)
    front = report.pareto_front(cands)
    assert front == ["real"]
    assert report.recommend(cands, front) == "real"


def test_oneshot_probe_excluded_even_when_it_wins():
    # The saturation probe outscoring every agent means the arena is broken,
    # not that a one-shot should ship: it must never reach front or pick.
    records = [
        record("probe-oneshot", "t1", 1.0, 0.001, 500, kind="oneshot"),
        record("agent", "t1", 0.7, 0.02, 2000),
    ]
    cands = report.aggregate(records)
    front = report.pareto_front(cands)
    assert front == ["agent"]
    assert report.recommend(cands, front) == "agent"


def test_recommendation_breaks_near_ties_by_cost():
    records = [
        record("pricey", "t1", 0.90, 0.50, 1000),
        record("frugal", "t1", 0.88, 0.05, 1200),
    ]
    cands = report.aggregate(records)
    pick = report.recommend(cands, report.pareto_front(cands))
    assert pick == "frugal"


def test_recommendation_prefers_clear_winner_despite_cost():
    records = [
        record("strong", "t1", 0.95, 0.50, 1000),
        record("weak", "t1", 0.60, 0.01, 500),
    ]
    cands = report.aggregate(records)
    pick = report.recommend(cands, report.pareto_front(cands))
    assert pick == "strong"


def test_render_and_files(tmp_path):
    exp = write_trials(
        tmp_path,
        [
            record("a", "t1", 1.0, 0.01, 1000),
            record("a", "t1", 0.5, 0.01, 1100),
            record("b", "t1", 0.2, 0.30, 9000, error="boom"),
        ],
    )
    cands = report.aggregate(report.load_records([exp]))
    assert cands["a"]["reward_mean"] == 0.75
    assert cands["a"]["tasks"]["t1"] == [1.0, 0.5]
    assert cands["b"]["voided"] == 1
    text = report.render(cands, report.pareto_front(cands), "a")
    assert "0.75 (2)" in text
    assert "**a**" in text
    assert "hash-a" in text


def test_extra_holdout_trials_do_not_penalize_recommendation():
    # Regression (live, capstone 20260610T160533Z): the holdout-proven winner
    # ran more trials, so its *total* cost exceeded a dominated rival that
    # never faced holdout — recommendation must compare cost per trial.
    records = [
        record("untested", "t1", 1.0, 0.0171, 65000),
        record("untested", "t2", 1.0, 0.0171, 65000),
        record("proven", "t1", 1.0, 0.0138, 61000),
        record("proven", "t2", 1.0, 0.0138, 61000),
        record("proven", "holdout", 1.0, 0.0138, 61000),
        record("proven", "holdout", 1.0, 0.0138, 61000),
    ]
    cands = report.aggregate(records)
    assert cands["proven"]["cost"] > cands["untested"]["cost"]  # totals mislead
    pick = report.recommend(cands, report.pareto_front(cands))
    assert pick == "proven"


def test_recommend_restricted_to_certified_candidates():
    # A lucky single-trial 1.0 may rank, but only certified candidates ship.
    records = (
        [record("lucky", "t1", 1.0, 0.001, 500)]
        + [record("steady", "t1", 0.9, 0.0005, 800) for _ in range(5)]
    )
    cands = report.aggregate(records)
    front = report.pareto_front(cands)
    assert "lucky" in front
    assert report.recommend(cands, front) == "lucky"
    assert report.recommend(cands, front, eligible={"steady"}) == "steady"
    # Nothing eligible -> no recommendation; never fall back to uncertified.
    assert report.recommend(cands, front, eligible={"ghost"}) is None


def test_unknown_cost_treated_as_worst_in_dominance():
    records = [
        record("known", "t1", 0.8, 0.01, 1000),
        record("mystery", "t1", 0.8, None, 1000),
    ]
    cands = report.aggregate(records)
    front = report.pareto_front(cands)
    assert "known" in front
    assert "mystery" not in front
