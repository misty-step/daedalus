#!/usr/bin/env python3
"""Search-loop core: generate → run → score → keep/discard → stop.

Pure orchestration with injected callables so the policy (budget, plateau,
incumbent selection) is testable offline. The CLI (bin/daedalus) wires in the
real runner subprocess and the LLM mutation step.
"""

REFERENCE = {"null", "oracle"}


def best_candidate(summary):
    """Highest mean reward among non-reference candidates; ties go to the
    cheapest (unknown cost ranks worst)."""
    real = {cid: v for cid, v in summary.items() if cid not in REFERENCE}
    if not real:
        raise ValueError("no non-reference candidates in summary")

    def key(item):
        cid, stats = item
        cost = stats.get("cost_usd_total")
        return (stats["reward_mean"], -(cost if cost is not None else float("inf")))

    return max(real.items(), key=key)[0]


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
):
    """Drive the search. Returns {stop_reason, history, best_id}.

    summary_fn() -> {candidate_id: {"reward_mean", "cost_usd_total", ...}}
    propose_fn(parent_id, generation) -> (child_id, meta dict) or raises
    run_child_fn(child_id) -> None (its trials land in the summary)
    """
    history = []
    non_improving = 0
    proposal_failures = 0
    stop_reason = "max-candidates"

    for generation in range(1, max_children + 1):
        summary = summary_fn()
        spent = known_spend(summary, optimizer_costs)
        if budget_usd is not None and spent >= budget_usd:
            stop_reason = "budget"
            break

        incumbent = best_candidate(summary)
        incumbent_reward = summary[incumbent]["reward_mean"]

        try:
            child_id, meta = propose_fn(incumbent, generation)
        except Exception as exc:  # noqa: BLE001 - a bad proposal is data
            proposal_failures += 1
            history.append(
                {"generation": generation, "parent_id": incumbent,
                 "proposal_error": str(exc)}
            )
            if proposal_failures >= max_proposal_failures:
                stop_reason = "proposal-failures"
                break
            continue

        run_child_fn(child_id)
        child_reward = summary_fn()[child_id]["reward_mean"]
        improved = child_reward > incumbent_reward
        meta = dict(
            meta,
            generation=generation,
            parent_id=incumbent,
            parent_reward_mean=incumbent_reward,
            reward_mean=child_reward,
            improved=improved,
        )
        history.append(meta)
        non_improving = 0 if improved else non_improving + 1
        if non_improving >= plateau_limit:
            stop_reason = "plateau"
            break

    final_summary = summary_fn()
    return {
        "stop_reason": stop_reason,
        "history": history,
        "best_id": best_candidate(final_summary),
        "spend_known_usd": round(known_spend(final_summary, optimizer_costs), 4),
    }
