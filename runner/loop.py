#!/usr/bin/env python3
"""Search-loop core v2: seed archive → race competing hypotheses → keep what
clears the noise → stop.

Pure orchestration with injected callables so the policy is testable offline.
Each generation draws parents from the archive pool (best-on-mean plus every
per-task winner, GEPA-style), proposes children_per_generation competing
single-slot hypotheses (distinct slots when they share a parent), runs them
under equal budgets, and credits improvement only when the paired per-task
delta clears the observed trial noise. The CLI (bin/daedalus) wires in the
real runner subprocess and the LLM mutation step.
"""

import random

REFERENCE_IDS = {"null", "oracle"}
REFERENCE_KINDS = {"null", "oracle", "oneshot"}

# Modes where holding reward while cutting cost (or wall) is progress.
COST_SENSITIVE_MODES = {"threshold-then-cheap", "pareto"}
LATENCY_SENSITIVE_MODES = {"fast-enough", "pareto"}


def cost_per_trial(stats):
    cost = stats.get("cost_usd_total")
    trials = stats.get("trials") or 0
    if cost is None or not trials:
        return None
    return cost / trials


def wall_mean_ms(stats):
    walls = [
        w for t in stats.get("tasks", {}).values() for w in t.get("wall_ms", [])
    ]
    return (sum(walls) / len(walls)) if walls else None


def is_reference(cid, stats):
    """References (floor, ceiling, saturation probe) never compete: not as
    incumbents, parents, or winners. Comparisons are agent vs agent."""
    return cid in REFERENCE_IDS or stats.get("kind") in REFERENCE_KINDS


def best_candidate(summary):
    """Highest mean reward among non-reference candidates; ties go to the
    cheapest (unknown cost ranks worst)."""
    real = {cid: v for cid, v in summary.items() if not is_reference(cid, v)}
    if not real:
        raise ValueError("no non-reference candidates in summary")

    def key(item):
        cid, stats = item
        cost = stats.get("cost_usd_total")
        return (stats["reward_mean"], -(cost if cost is not None else float("inf")))

    return max(real.items(), key=key)[0]


def parent_pool(summary):
    """Archive-eligible parents: the best-on-mean candidate plus every
    per-task winner. A specialist that wins one task while losing the mean
    stays selectable — its trick may generalize once mutated."""
    real = {cid: v for cid, v in summary.items() if not is_reference(cid, v)}
    if not real:
        raise ValueError("no non-reference candidates in summary")
    pool = {best_candidate(summary)}
    tasks = sorted({t for v in real.values() for t in v.get("tasks", {})})
    for task in tasks:
        contenders = sorted(c for c in real if task in real[c].get("tasks", {}))
        if contenders:
            pool.add(max(contenders, key=lambda c: real[c]["tasks"][task]["mean"]))
    # Cost frontier: the cheapest candidate within near-tie of the best
    # reward stays breedable — cheap-and-almost-as-good is where the
    # cost-sensitive modes want to search.
    best_r = real[best_candidate(summary)]["reward_mean"]
    near = [
        c for c in sorted(real)
        if real[c]["reward_mean"] >= best_r - 0.05
        and cost_per_trial(real[c]) is not None
    ]
    if near:
        pool.add(min(near, key=lambda c: cost_per_trial(real[c])))
    return sorted(pool)


def trial_noise(stats):
    """Half the mean within-task reward range: the observable radius of
    trial-to-trial noise for this candidate. 0.0 with single trials."""
    spreads = [
        t["max"] - t["min"]
        for t in stats.get("tasks", {}).values()
        if len(t.get("rewards", [])) >= 2
    ]
    return (sum(spreads) / len(spreads) / 2) if spreads else 0.0


def improved_over(child, parent, mode="max-quality", epsilon=0.01):
    """Variance- and mode-aware keep rule. A reward gain must clear both
    candidates' observed trial noise (means drifting inside the band are
    dice, not progress). Under cost-sensitive modes, holding reward within
    the band while cutting cost per trial ≥10% is also progress — the
    search must chase what the report grades; same for wall under
    latency-sensitive modes."""
    common = sorted(set(child.get("tasks", {})) & set(parent.get("tasks", {})))
    if not common:
        return False, 0.0
    deltas = [
        child["tasks"][t]["mean"] - parent["tasks"][t]["mean"] for t in common
    ]
    mean_delta = sum(deltas) / len(deltas)
    band = max(trial_noise(child), trial_noise(parent), epsilon)
    if mean_delta > band:
        return True, round(mean_delta, 4)
    if mean_delta >= -band:  # reward held within noise
        cc, pc = cost_per_trial(child), cost_per_trial(parent)
        if (mode in COST_SENSITIVE_MODES
                and cc is not None and pc is not None and cc <= pc * 0.9):
            return True, round(mean_delta, 4)
        cw, pw = wall_mean_ms(child), wall_mean_ms(parent)
        if (mode in LATENCY_SENSITIVE_MODES
                and cw is not None and pw is not None and cw <= pw * 0.9):
            return True, round(mean_delta, 4)
    return False, round(mean_delta, 4)


def known_spend(summary, optimizer_costs):
    spent = sum(v.get("cost_usd_total") or 0 for v in summary.values())
    return spent + sum(c for c in optimizer_costs if c is not None)


def run_search(
    *,
    summary_fn,
    propose_fn,
    run_child_fn,
    max_children,
    budget_usd,
    optimizer_costs,
    plateau_limit=2,
    max_proposal_failures=2,
    children_per_generation=2,
    mode="max-quality",
    monitor_fn=None,
    rng=None,
):
    """Drive the search. Returns {stop_reason, history, generations, best_id,
    alarms, spend_known_usd}.

    summary_fn() -> {candidate_id: {"reward_mean", "cost_usd_total", "kind",
                                    "tasks": {task: {"rewards","mean",...}}}}
    propose_fn(parent_id, generation, attempt, avoid_slots)
        -> (child_id, meta with "slot_changed") or raises
    run_child_fn(child_id) -> None (its trials land in the summary)
    monitor_fn(summary, generation) -> [alarm dicts]; an alarm carrying a
        "stop" key halts the search with that stop reason (the meta-eval
        monitor's verdict that the arena, not the candidates, is the
        bottleneck).
    """
    rng = rng or random.Random(0)
    history = []
    alarms = []
    seen_alarms = set()
    non_improving_gens = 0
    proposal_failures = 0
    children_built = 0
    generation = 0
    stop_reason = "max-candidates"
    stopped = False

    while children_built < max_children and not stopped:
        generation += 1
        summary = summary_fn()
        spent = known_spend(summary, optimizer_costs)
        if budget_usd is not None and spent >= budget_usd:
            stop_reason = "budget"
            break

        if monitor_fn is not None:
            for alarm in monitor_fn(summary, generation) or []:
                key = (alarm.get("kind"), alarm.get("detail"))
                if key not in seen_alarms:
                    seen_alarms.add(key)
                    alarms.append(alarm)
                if alarm.get("stop"):
                    stop_reason = alarm["stop"]
                    stopped = True
            if stopped:
                break

        parents = parent_pool(summary)
        rng.shuffle(parents)
        k = min(children_per_generation, max_children - children_built)
        proposed_slots = {}
        ran_any = improved_any = False

        for attempt in range(k):
            parent = parents[attempt % len(parents)]
            avoid = tuple(s for s in proposed_slots.get(parent, ()) if s)
            try:
                child_id, meta = propose_fn(parent, generation, attempt, avoid)
            except Exception as exc:  # noqa: BLE001 - a bad proposal is data
                proposal_failures += 1
                history.append(
                    {"generation": generation, "attempt": attempt,
                     "parent_id": parent, "proposal_error": str(exc)}
                )
                if proposal_failures >= max_proposal_failures:
                    stop_reason = "proposal-failures"
                    stopped = True
                    break
                continue
            proposed_slots.setdefault(parent, []).append(meta.get("slot_changed"))
            run_child_fn(child_id)
            children_built += 1
            after = summary_fn()
            improved, delta = improved_over(after[child_id], after[parent], mode)
            history.append(
                dict(
                    meta,
                    generation=generation,
                    attempt=attempt,
                    parent_id=parent,
                    parent_reward_mean=after[parent]["reward_mean"],
                    reward_mean=after[child_id]["reward_mean"],
                    mean_task_delta=delta,
                    improved=improved,
                )
            )
            ran_any = True
            improved_any = improved_any or improved

        if stopped:
            break
        if ran_any:
            non_improving_gens = 0 if improved_any else non_improving_gens + 1
            if non_improving_gens >= plateau_limit:
                stop_reason = "plateau"
                break

    final_summary = summary_fn()
    return {
        "stop_reason": stop_reason,
        "mode": mode,
        "history": history,
        "alarms": alarms,
        "generations": generation,
        "best_id": best_candidate(final_summary),
        "spend_known_usd": round(known_spend(final_summary, optimizer_costs), 4),
    }
