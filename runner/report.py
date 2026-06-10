#!/usr/bin/env python3
"""Comparison report over experiment run records.

Usage:
    runner/report.py runs/<exp-id> [more dirs or .jsonl files] [--out report.md]

Aggregates trial records by candidate, prints a per-task reward grid, totals,
the Pareto set over (reward mean ↑, cost ↓, wall ↓), and a recommendation.
Reference candidates (null, oracle) appear in the grid but never in the
Pareto set or recommendation.
"""

import argparse
import json
from pathlib import Path

REFERENCE = {"null", "oracle"}


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
            if r["candidate_id"] not in REFERENCE:
                c["cost_known"] = False
        else:
            c["cost"] += r["cost_usd"]
    for c in cands.values():
        rewards = [x for rs in c["tasks"].values() for x in rs]
        c["reward_mean"] = round(sum(rewards) / len(rewards), 4)
        c["wall_mean"] = round(sum(c["walls"]) / len(c["walls"]) / 1000, 1)
        c["cost"] = round(c["cost"], 4) if c["cost_known"] else None
    return cands


def _dominates(b, a):
    """b dominates a: no worse on all three objectives, better on one."""
    b_cost = b["cost"] if b["cost"] is not None else float("inf")
    a_cost = a["cost"] if a["cost"] is not None else float("inf")
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
    pts = [c for c in cands.values() if c["id"] not in REFERENCE]
    return sorted(
        (a["id"] for a in pts if not any(_dominates(b, a) for b in pts if b is not a)),
        key=lambda cid: -cands[cid]["reward_mean"],
    )


def recommend(cands, front):
    """Best mean reward; within 0.05 of the best, cheapest wins."""
    if not front:
        return None
    best = max(cands[cid]["reward_mean"] for cid in front)
    close = [cid for cid in front if cands[cid]["reward_mean"] >= best - 0.05]
    return min(
        close,
        key=lambda cid: (
            cands[cid]["cost"] if cands[cid]["cost"] is not None else float("inf")
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
    lines.append("| candidate | total cost | mean wall/task |")
    lines.append("|---|---|---|")
    for c in order:
        cost = f"${c['cost']:.4f}" if c["cost"] is not None else "unknown"
        lines.append(f"| {c['id']} | {cost} | {c['wall_mean']}s |")

    lines += ["", "## Pareto set (reward ↑, cost ↓, latency ↓)", ""]
    lines += [f"- {cid}" for cid in front] or ["- (no non-reference candidates)"]

    lines += ["", "## Recommendation", ""]
    if pick:
        c = cands[pick]
        cost = f"${c['cost']:.4f}" if c["cost"] is not None else "unknown cost"
        lines.append(
            f"**{pick}** — mean reward {c['reward_mean']:.4f} at {cost} "
            f"({c['wall_mean']}s mean wall). Within-0.05 reward ties resolve "
            "to the cheapest candidate."
        )
    else:
        lines.append("No non-reference candidates to recommend.")
    lines.append("")
    lines.append(
        "_Reference candidates (oracle/null) bound the verifier; they are "
        "excluded from Pareto and recommendation._"
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
