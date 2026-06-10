#!/usr/bin/env python3
"""Comparison report over experiment run records.

Usage:
    runner/report.py runs/<exp-id> [more dirs or .jsonl files] [--out report.md]

Aggregates trial records by candidate, prints a per-task reward grid, totals,
the Pareto set over (reward mean ↑, cost ↓, wall ↓), and a recommendation.
Reference candidates appear in the grid but never in the Pareto set or
recommendation: null (floor), oracle (ceiling), and any oneshot-kind probe
(saturation detector — comparisons here are always agent vs agent, so a
one-shot can never be the recommended deliverable).
"""

import argparse
import json
from pathlib import Path

REFERENCE_KINDS = {"null", "oracle", "oneshot"}
COSTLESS_KINDS = {"null", "oracle"}  # cost is structurally absent, not unknown


def load_records(paths):
    records = []
    for p in map(Path, paths):
        f = p / "trials.jsonl" if p.is_dir() else p
        records += [json.loads(line) for line in f.read_text().splitlines()]
    return records


def aggregate(records):
    cands = {}
    for r in records:
        c = cands.setdefault(
            r["candidate_id"],
            {
                "id": r["candidate_id"],
                "kind": r.get("candidate_kind"),
                "model": r.get("model"),
                "hash": r.get("composition_hash"),
                "tasks": {},
                "trials": 0,
                "voided": 0,
                "cost": 0.0,
                "cost_known": True,
                "walls": [],
            },
        )
        c["trials"] += 1
        if r.get("error"):
            c["voided"] += 1
        c["tasks"].setdefault(r["task_id"], []).append(r["reward"])
        c["walls"].append(r["wall_ms"])
        if r.get("cost_usd") is None:
            if r.get("candidate_kind") not in COSTLESS_KINDS:
                c["cost_known"] = False
        else:
            c["cost"] += r["cost_usd"]
    for c in cands.values():
        rewards = [x for rs in c["tasks"].values() for x in rs]
        c["reward_mean"] = round(sum(rewards) / len(rewards), 4)
        c["wall_mean"] = round(sum(c["walls"]) / len(c["walls"]) / 1000, 1)
        c["cost"] = round(c["cost"], 4) if c["cost_known"] else None
        # Dominance and recommendation compare cost per trial, never totals:
        # a front candidate that earned extra holdout trials must not be
        # penalized for having been tested more (live bug, capstone run
        # 20260610T160533Z).
        c["cost_per_trial"] = (
            round(c["cost"] / c["trials"], 6) if c["cost"] is not None else None
        )
    return cands


def _dominates(b, a):
    """b dominates a: no worse on all three objectives, better on one."""
    b_cost = b["cost_per_trial"] if b["cost_per_trial"] is not None else float("inf")
    a_cost = a["cost_per_trial"] if a["cost_per_trial"] is not None else float("inf")
    no_worse = (
        b["reward_mean"] >= a["reward_mean"]
        and b_cost <= a_cost
        and b["wall_mean"] <= a["wall_mean"]
    )
    better = (
        b["reward_mean"] > a["reward_mean"]
        or b_cost < a_cost
        or b["wall_mean"] < a["wall_mean"]
    )
    return no_worse and better


def pareto_front(cands):
    pts = [c for c in cands.values() if c["kind"] not in REFERENCE_KINDS]
    return sorted(
        (a["id"] for a in pts if not any(_dominates(b, a) for b in pts if b is not a)),
        key=lambda cid: -cands[cid]["reward_mean"],
    )


def recommend(cands, front, eligible=None):
    """Best mean reward; within 0.05 of the best, cheapest per trial wins.
    When `eligible` is given (certification: candidates with enough trials),
    only those may be recommended — a lucky low-n mean can rank but never
    ship. Falls back to the whole front if nothing eligible made it."""
    pool = [cid for cid in front if eligible is None or cid in eligible] or front
    if not pool:
        return None
    best = max(cands[cid]["reward_mean"] for cid in pool)
    close = [cid for cid in pool if cands[cid]["reward_mean"] >= best - 0.05]
    return min(
        close,
        key=lambda cid: (
            cands[cid]["cost_per_trial"]
            if cands[cid]["cost_per_trial"] is not None
            else float("inf")
        ),
    )


def render(cands, front, pick):
    tasks = sorted({t for c in cands.values() for t in c["tasks"]})
    order = sorted(cands.values(), key=lambda c: -c["reward_mean"])
    lines = ["# Experiment comparison", ""]

    lines += ["## Compositions", ""]
    lines.append("| candidate | kind | model | hash | trials | voided |")
    lines.append("|---|---|---|---|---|---|")
    for c in order:
        lines.append(
            f"| {c['id']} | {c['kind']} | {c['model'] or '—'} "
            f"| {c['hash'] or '—'} | {c['trials']} | {c['voided']} |"
        )

    lines += ["", "## Mean reward per task (n trials in parentheses)", ""]
    lines.append("| candidate | " + " | ".join(tasks) + " | **overall** |")
    lines.append("|---|" + "---|" * (len(tasks) + 1))
    for c in order:
        cells = []
        for t in tasks:
            rs = c["tasks"].get(t)
            cells.append(f"{sum(rs)/len(rs):.2f} ({len(rs)})" if rs else "—")
        lines.append(
            f"| {c['id']} | " + " | ".join(cells) + f" | **{c['reward_mean']:.4f}** |"
        )

    lines += ["", "## Cost and latency", ""]
    lines.append("| candidate | cost/trial | total cost | mean wall/task |")
    lines.append("|---|---|---|---|")
    for c in order:
        cost = f"${c['cost']:.4f}" if c["cost"] is not None else "unknown"
        per = (
            f"${c['cost_per_trial']:.4f}"
            if c["cost_per_trial"] is not None
            else "unknown"
        )
        lines.append(f"| {c['id']} | {per} | {cost} | {c['wall_mean']}s |")

    lines += ["", "## Pareto set (reward ↑, cost ↓, latency ↓)", ""]
    lines += [f"- {cid}" for cid in front] or ["- (no non-reference candidates)"]

    lines += ["", "## Recommendation", ""]
    if pick:
        c = cands[pick]
        per = (
            f"${c['cost_per_trial']:.4f}/trial"
            if c["cost_per_trial"] is not None
            else "unknown cost"
        )
        lines.append(
            f"**{pick}** — mean reward {c['reward_mean']:.4f} at {per} "
            f"({c['wall_mean']}s mean wall). Within-0.05 reward ties resolve "
            "to the cheapest candidate per trial."
        )
    else:
        lines.append("No non-reference candidates to recommend.")
    lines.append("")
    lines.append(
        "_References are excluded from Pareto and recommendation: oracle/null "
        "bound the verifier; the one-shot probe only detects arena saturation. "
        "Every recommendable candidate is an agent composition._"
    )
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", help="experiment dirs or trials .jsonl")
    parser.add_argument("--out", help="write report.md here (default: first dir)")
    args = parser.parse_args()

    cands = aggregate(load_records(args.paths))
    front = pareto_front(cands)
    pick = recommend(cands, front)
    text = render(cands, front, pick)

    first = Path(args.paths[0])
    out = Path(args.out) if args.out else (
        first / "report.md" if first.is_dir() else first.with_suffix(".report.md")
    )
    out.write_text(text)
    pareto_path = out.parent / "pareto.json"
    pareto_path.write_text(
        json.dumps(
            [
                {
                    "candidate_id": cid,
                    "composition_hash": cands[cid]["hash"],
                    "reward_mean": cands[cid]["reward_mean"],
                    "cost_usd_total": cands[cid]["cost"],
                    "cost_usd_per_trial": cands[cid]["cost_per_trial"],
                    "wall_mean_s": cands[cid]["wall_mean"],
                    "recommended": cid == pick,
                }
                for cid in front
            ],
            indent=2,
        )
    )
    print(text)
    print(f"wrote {out} and {pareto_path}", flush=True)


if __name__ == "__main__":
    main()
