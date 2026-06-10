"""Search-loop policy tests with injected fakes: stop conditions, incumbent
selection, budget accounting."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import loop  # noqa: E402


class FakeWorld:
    """Scripted children: each proposal yields the next (id, reward, cost)."""

    def __init__(self, baseline_reward=0.6, script=()):
        self.summary = {
            "oracle": {"reward_mean": 1.0, "cost_usd_total": 0.0},
            "null": {"reward_mean": 0.15, "cost_usd_total": 0.0},
            "base": {"reward_mean": baseline_reward, "cost_usd_total": 0.05},
        }
        self.script = list(script)
        self.pending = {}

    def summary_fn(self):
        return {k: dict(v) for k, v in self.summary.items()}

    def propose_fn(self, parent_id, generation):
        if not self.script:
            raise ValueError("script exhausted")
        item = self.script.pop(0)
        if item == "fail":
            raise ValueError("optimizer returned garbage")
        child_id, reward, cost = item
        self.pending[child_id] = {"reward_mean": reward, "cost_usd_total": cost}
        return child_id, {"child_id": child_id, "slot_changed": "prompt_packet",
                          "hypothesis": "h"}

    def run_child_fn(self, child_id):
        self.summary[child_id] = self.pending.pop(child_id)


def search(world, **kw):
    defaults = dict(
        summary_fn=world.summary_fn,
        propose_fn=world.propose_fn,
        run_child_fn=world.run_child_fn,
        max_children=10,
        budget_usd=100.0,
        optimizer_costs=[],
        plateau_limit=2,
    )
    defaults.update(kw)
    return loop.run_search(**defaults)


def test_plateau_stops_after_two_non_improving():
    world = FakeWorld(script=[("c1", 0.5, 0.01), ("c2", 0.55, 0.01)])
    out = search(world)
    assert out["stop_reason"] == "plateau"
    assert len(out["history"]) == 2
    assert out["best_id"] == "base"


def test_improvement_resets_plateau_and_becomes_incumbent():
    world = FakeWorld(
        script=[("c1", 0.7, 0.01), ("c2", 0.5, 0.01), ("c3", 0.8, 0.01),
                ("c4", 0.1, 0.01), ("c5", 0.2, 0.01)]
    )
    out = search(world)
    assert out["stop_reason"] == "plateau"
    assert out["best_id"] == "c3"
    gen3 = out["history"][2]
    assert gen3["parent_id"] == "c1"  # c1 was incumbent when c3 proposed
    assert gen3["improved"]


def test_budget_stop():
    world = FakeWorld(script=[("c1", 0.9, 3.0), ("c2", 0.95, 3.0)])
    out = search(world, budget_usd=3.0)
    assert out["stop_reason"] == "budget"
    assert len(out["history"]) == 1  # second iteration refused before spend


def test_max_candidates_stop():
    world = FakeWorld(
        script=[(f"c{i}", 0.6 + i * 0.01, 0.01) for i in range(1, 4)]
    )
    out = search(world, max_children=3, plateau_limit=99)
    assert out["stop_reason"] == "max-candidates"
    assert len(out["history"]) == 3


def test_proposal_failures_stop():
    world = FakeWorld(script=["fail", "fail"])
    out = search(world)
    assert out["stop_reason"] == "proposal-failures"
    assert all("proposal_error" in h for h in out["history"])


def test_rig_discrimination_accepts_clean_task_fraction():
    # Regression: a 2-task arena with one clean task gives null 0.5, which must
    # NOT be rejected as "too easy" — the rig is sound as long as null < oracle.
    def rig_ok(oracle_mean, null_mean):
        return oracle_mean == 1.0 and null_mean < oracle_mean

    assert rig_ok(1.0, 0.5)      # v1: one clean of two tasks
    assert rig_ok(1.0, 0.1667)   # v0: one clean of six tasks
    assert not rig_ok(1.0, 1.0)  # no discrimination
    assert not rig_ok(0.8, 0.1)  # oracle can't ace the rig


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
