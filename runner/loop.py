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


def improved_over(child, parent, epsilon=0.01):
    """Variance-aware keep rule: the mean paired per-task delta must clear
    both candidates' observed trial noise (and a minimal epsilon). A mean
    drifting inside the noise band is not progress, it is dice."""
    common = sorted(set(child.get("tasks", {})) & set(parent.get("tasks", {})))
    if not common:
        return False, 0.0
    deltas = [
        child["tasks"][t]["mean"] - parent["tasks"][t]["mean"] for t in common
    ]
    mean_delta = sum(deltas) / len(deltas)
    threshold = max(trial_noise(child), trial_noise(parent), epsilon)
    return mean_delta > threshold, round(mean_delta, 4)


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
    rng=None,
):
    """Drive the search. Returns {stop_reason, history, generations, best_id,
    spend_known_usd}.

    summary_fn() -> {candidate_id: {"reward_mean", "cost_usd_total", "kind",
                                    "tasks": {task: {"rewards","mean",...}}}}
    propose_fn(parent_id, generation, attempt, avoid_slots)
        -> (child_id, meta with "slot_changed") or raises
    run_child_fn(child_id) -> None (its trials land in the summary)
    """
    rng = rng or random.Random(0)
    history = []
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
            improved, delta = improved_over(after[child_id], after[parent])
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
        "history": history,
        "generations": generation,
        "best_id": best_candidate(final_summary),
        "spend_known_usd": round(known_spend(final_summary, optimizer_costs), 4),
    }
