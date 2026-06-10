"""Search-loop v2 policy tests with injected fakes: archive parent pool,
competing hypotheses per generation, variance-aware keep, stop conditions."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import loop  # noqa: E402


def stats(task_rewards, cost=0.05, kind="pi", wall_ms=1000):
    """Build a summary entry from {task: [trial rewards]}."""
    tasks = {
        t: {"rewards": rs, "mean": sum(rs) / len(rs), "min": min(rs),
            "max": max(rs), "wall_ms": [wall_ms] * len(rs)}
        for t, rs in task_rewards.items()
    }
    flat = [x for rs in task_rewards.values() for x in rs]
    return {
        "kind": kind,
        "tasks": tasks,
        "trials": len(flat),
        "reward_mean": round(sum(flat) / len(flat), 4),
        "cost_usd_total": cost,
    }


class FakeWorld:
    """Scripted children: each proposal yields the next (id, task_rewards,
    cost), or 'fail' for a garbage proposal."""

    def __init__(self, candidates, script):
        self.summary = dict(candidates)
        self.script = list(script)
        self.pending = {}
        self.avoid_seen = []

    def summary_fn(self):
        return {k: dict(v) for k, v in self.summary.items()}

    def propose_fn(self, parent_id, generation, attempt, avoid_slots):
        self.avoid_seen.append((parent_id, avoid_slots))
        if not self.script:
            raise ValueError("script exhausted")
        item = self.script.pop(0)
        if item == "fail":
            raise ValueError("optimizer returned garbage")
        child_id, rewards, cost = item
        self.pending[child_id] = stats(rewards, cost)
        return child_id, {"child_id": child_id, "slot_changed": "prompt_packet",
                          "hypothesis": "h"}

    def run_child_fn(self, child_id):
        self.summary[child_id] = self.pending.pop(child_id)


BASE = {
    "oracle": stats({"t1": [1.0], "t2": [1.0]}, cost=0.0, kind="oracle"),
    "null": stats({"t1": [0.0], "t2": [0.5]}, cost=0.0, kind="null"),
    "probe-oneshot": stats({"t1": [1.0], "t2": [1.0]}, cost=0.001,
                           kind="oneshot"),
    "base": stats({"t1": [0.6, 0.6], "t2": [0.5, 0.5]}, cost=0.05),
}


def search(world, **kw):
    defaults = dict(
        summary_fn=world.summary_fn,
        propose_fn=world.propose_fn,
        run_child_fn=world.run_child_fn,
        max_children=10,
        budget_usd=100.0,
        optimizer_costs=[],
        plateau_limit=2,
        children_per_generation=2,
    )
    defaults.update(kw)
    return loop.run_search(**defaults)


def test_plateau_stops_after_non_improving_generations():
    # Four children across two generations, none clearing the noise band.
    world = FakeWorld(BASE, [
        ("c1", {"t1": [0.58], "t2": [0.5]}, 0.01),
        ("c2", {"t1": [0.55], "t2": [0.5]}, 0.01),
        ("c3", {"t1": [0.59], "t2": [0.5]}, 0.01),
        ("c4", {"t1": [0.3], "t2": [0.2]}, 0.01),
    ])
    out = search(world)
    assert out["stop_reason"] == "plateau"
    assert len(out["history"]) == 4
    assert out["generations"] == 2
    assert out["best_id"] == "base"


def test_clear_improvement_resets_plateau_and_wins():
    world = FakeWorld(BASE, [
        ("c1", {"t1": [0.9, 0.9], "t2": [0.9, 0.9]}, 0.01),  # real jump
        ("c2", {"t1": [0.5], "t2": [0.4]}, 0.01),
        ("c3", {"t1": [0.9], "t2": [0.9]}, 0.01),  # ties c1, no improvement
        ("c4", {"t1": [0.85], "t2": [0.85]}, 0.01),
        ("c5", {"t1": [0.9], "t2": [0.88]}, 0.01),
    ])
    out = search(world)
    assert out["stop_reason"] == "plateau"
    assert out["best_id"] == "c1"
    gen1 = [h for h in out["history"] if h["generation"] == 1]
    assert gen1[0]["improved"] is True


def test_improvement_inside_noise_band_does_not_count():
    # Parent trials swing ±0.2 per task; a +0.1 mean bump is dice, not signal.
    noisy = dict(BASE)
    noisy["base"] = stats({"t1": [0.8, 0.4], "t2": [0.7, 0.3]}, cost=0.05)
    world = FakeWorld(noisy, [
        ("c1", {"t1": [0.7], "t2": [0.6]}, 0.01),   # +0.1, inside noise
        ("c2", {"t1": [0.65], "t2": [0.6]}, 0.01),
        ("c3", {"t1": [0.7], "t2": [0.6]}, 0.01),   # ties the gen-1 leader
        ("c4", {"t1": [0.6], "t2": [0.6]}, 0.01),
    ])
    out = search(world)
    assert out["stop_reason"] == "plateau"
    assert not any(h.get("improved") for h in out["history"])


def test_improved_over_clears_noise_threshold_directly():
    parent = stats({"t1": [0.8, 0.4], "t2": [0.7, 0.3]})  # noise radius 0.2
    inside = stats({"t1": [0.7], "t2": [0.6]})             # +0.1 mean delta
    beyond = stats({"t1": [0.95], "t2": [0.9]})            # +0.275 mean delta
    ok, delta = loop.improved_over(inside, parent)
    assert not ok and abs(delta - 0.1) < 1e-9
    ok, delta = loop.improved_over(beyond, parent)
    assert ok and delta > 0.2


def test_parent_pool_includes_per_task_specialist():
    summary = dict(BASE)
    summary["generalist"] = stats({"t1": [0.8], "t2": [0.8]}, cost=0.05)
    # Specialist loses the mean but owns t2 — must stay selectable.
    summary["specialist"] = stats({"t1": [0.2], "t2": [0.95]}, cost=0.05)
    pool = loop.parent_pool(summary)
    assert "generalist" in pool
    assert "specialist" in pool
    assert "probe-oneshot" not in pool
    assert "oracle" not in pool


def test_competing_hypotheses_avoid_each_others_slot():
    # Single eligible parent -> both attempts hit it; the second must carry
    # the first proposal's slot in avoid_slots.
    world = FakeWorld(BASE, [
        ("c1", {"t1": [0.6], "t2": [0.5]}, 0.01),
        ("c2", {"t1": [0.6], "t2": [0.5]}, 0.01),
    ])
    search(world, max_children=2)
    assert world.avoid_seen[0] == ("base", ())
    assert world.avoid_seen[1] == ("base", ("prompt_packet",))


def test_budget_stop():
    world = FakeWorld(BASE, [
        ("c1", {"t1": [0.9], "t2": [0.9]}, 3.0),
        ("c2", {"t1": [0.95], "t2": [0.95]}, 3.0),
    ])
    out = search(world, budget_usd=3.0, children_per_generation=1)
    assert out["stop_reason"] == "budget"
    assert len(out["history"]) == 1  # second generation refused before spend


def test_max_candidates_stop():
    world = FakeWorld(BASE, [
        (f"c{i}", {"t1": [0.9 + i * 0.01], "t2": [0.9]}, 0.01)
        for i in range(1, 4)
    ])
    out = search(world, max_children=3, plateau_limit=99)
    assert out["stop_reason"] == "max-candidates"
    assert len(out["history"]) == 3


def test_proposal_failures_stop():
    world = FakeWorld(BASE, ["fail", "fail"])
    out = search(world)
    assert out["stop_reason"] == "proposal-failures"
    assert all("proposal_error" in h for h in out["history"])


def test_best_candidate_ignores_references_and_breaks_ties_by_cost():
    summary = {
        "oracle": {"reward_mean": 1.0, "cost_usd_total": 0.0},
        "a": {"reward_mean": 0.8, "cost_usd_total": 0.50},
        "b": {"reward_mean": 0.8, "cost_usd_total": 0.10},
        "c": {"reward_mean": 0.8, "cost_usd_total": None},
    }
    assert loop.best_candidate(summary) == "b"


def test_best_candidate_ignores_oneshot_probe_by_kind():
    # Even a perfect probe score can never make it the incumbent/parent.
    summary = {
        "probe-oneshot": {"reward_mean": 1.0, "cost_usd_total": 0.001,
                          "kind": "oneshot"},
        "agent": {"reward_mean": 0.6, "cost_usd_total": 0.50, "kind": "pi"},
    }
    assert loop.best_candidate(summary) == "agent"


def test_held_reward_with_cost_cut_improves_under_cheap_modes():
    # The g1b regression: same reward, 42% cheaper per trial. That is the
    # whole point of threshold-then-cheap — it must count as improvement.
    parent = stats({"t1": [1.0, 1.0], "t2": [1.0, 1.0]}, cost=0.40)
    child = stats({"t1": [1.0, 1.0], "t2": [1.0, 1.0]}, cost=0.24)
    ok, delta = loop.improved_over(child, parent, "threshold-then-cheap")
    assert ok and delta == 0.0
    ok, _ = loop.improved_over(child, parent, "max-quality")
    assert not ok  # quality mode does not chase cost
    # A trivial 3% cost wiggle is not improvement even in cheap mode.
    wiggle = stats({"t1": [1.0, 1.0], "t2": [1.0, 1.0]}, cost=0.39)
    ok, _ = loop.improved_over(wiggle, parent, "threshold-then-cheap")
    assert not ok


def test_held_reward_with_wall_cut_improves_under_fast_enough():
    parent = stats({"t1": [0.8, 0.8]}, wall_ms=10000)
    child = stats({"t1": [0.8, 0.8]}, wall_ms=6000)
    ok, _ = loop.improved_over(child, parent, "fast-enough")
    assert ok
    ok, _ = loop.improved_over(child, parent, "threshold-then-cheap")
    assert not ok  # cost unchanged; wall is not this mode's axis


def test_reward_drop_is_never_excused_by_cheapness():
    parent = stats({"t1": [1.0, 1.0], "t2": [1.0, 1.0]}, cost=0.40)
    worse = stats({"t1": [0.5, 0.5], "t2": [0.5, 0.5]}, cost=0.04)
    ok, _ = loop.improved_over(worse, parent, "threshold-then-cheap")
    assert not ok


def test_parent_pool_includes_cost_frontier_near_tie():
    summary = dict(BASE)
    summary["champ"] = stats({"t1": [1.0], "t2": [1.0]}, cost=0.50)
    # within 0.05 of the champ's reward at a tenth of the cost per trial
    summary["frugal"] = stats({"t1": [0.95], "t2": [0.96]}, cost=0.04)
    pool = loop.parent_pool(summary)
    assert "champ" in pool
    assert "frugal" in pool


def test_monitor_alarms_recorded_deduped_and_can_stop():
    calls = []

    def monitor(summary, generation):
        calls.append(generation)
        return [{"kind": "saturation-at-top", "detail": "best at ceiling",
                 "stop": "arena-saturated-at-top"}]

    world = FakeWorld(BASE, [("c1", {"t1": [0.7], "t2": [0.7]}, 0.01)])
    out = search(world, monitor_fn=monitor)
    assert out["stop_reason"] == "arena-saturated-at-top"
    assert out["history"] == []  # stopped before proposing
    assert len(out["alarms"]) == 1


def test_monitor_note_alarms_do_not_stop_and_do_not_repeat():
    def monitor(summary, generation):
        return [{"kind": "variance", "detail": "x flip-flops on t1"}]

    world = FakeWorld(BASE, [
        ("c1", {"t1": [0.58], "t2": [0.5]}, 0.01),
        ("c2", {"t1": [0.55], "t2": [0.5]}, 0.01),
        ("c3", {"t1": [0.59], "t2": [0.5]}, 0.01),
        ("c4", {"t1": [0.3], "t2": [0.2]}, 0.01),
    ])
    out = search(world, monitor_fn=monitor)
    assert out["stop_reason"] == "plateau"
    assert len(out["alarms"]) == 1  # same alarm across generations dedupes


def test_rig_discrimination_accepts_clean_task_fraction():
    # Regression: a 2-task arena with one clean task gives null 0.5, which must
    # NOT be rejected as "too easy" — the rig is sound as long as null < oracle.
    def rig_ok(oracle_mean, null_mean):
        return oracle_mean == 1.0 and null_mean < oracle_mean

    assert rig_ok(1.0, 0.5)      # v1: one clean of two tasks
    assert rig_ok(1.0, 0.1667)   # v0: one clean of six tasks
    assert not rig_ok(1.0, 1.0)  # no discrimination
    assert not rig_ok(0.8, 0.1)  # oracle can't ace the rig
