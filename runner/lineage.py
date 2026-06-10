#!/usr/bin/env python3
"""Render a run's experiment lineage: the traceable story of how the final
agent contract was discovered — rig, sampled landscape, every hypothesis
with its measured outcome and decision, alarms, certification, outcome.
A pure function of the run's own artifacts (rig.json, seed.json, loop.json,
pareto.json, trials.jsonl), so it works retroactively on any recorded
experiment.

Usage:
    runner/lineage.py runs/<exp-dir>    # writes <exp-dir>/lineage.md
"""

import json
import re
import sys
from pathlib import Path


def _load_json(path, default):
    return json.loads(path.read_text()) if path.exists() else default


def _records(exp_dir):
    path = exp_dir / "trials.jsonl"
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines()]


def _candidate_stats(records):
    out = {}
    for r in records:
        c = out.setdefault(
            r["candidate_id"],
            {"rewards": [], "cost": 0.0, "kind": r.get("candidate_kind")},
        )
        c["rewards"].append(r["reward"])
        c["cost"] += r.get("cost_usd") or 0
    for c in out.values():
        c["mean"] = sum(c["rewards"]) / len(c["rewards"])
        c["n"] = len(c["rewards"])
    return out


def _seed_index(candidate_id):
    m = re.match(r"seed(\d+)-", candidate_id)
    return int(m.group(1)) if m else None


def hypothesis_verdict(h):
    """Score a generation's predicted_effect against its measurement.
    Returns (label, detail) or None when no structured prediction exists."""
    pe = h.get("predicted_effect")
    if not pe:
        return None
    axes = []
    delta = h.get("mean_task_delta")
    if delta is not None:
        ok = delta > 0.02 if pe.get("reward") == "up" else delta >= -0.05
        axes.append(("reward", pe.get("reward"), ok, f"Δ{delta:+.3f}"))
    pcpt = h.get("parent_cost_per_trial")
    ccpt = h.get("child_cost_per_trial")
    if pcpt and ccpt is not None:
        ratio = ccpt / pcpt
        ok = {"down": ratio <= 0.9,
              "hold": 0.9 < ratio < 1.1,
              "up": True}.get(pe.get("cost"), True)
        axes.append(("cost", pe.get("cost"), ok, f"×{ratio:.2f}"))
    if not axes:
        return None
    oks = [ok for _, _, ok, _ in axes]
    label = ("prediction confirmed" if all(oks)
             else "prediction refuted" if not any(oks)
             else "prediction partially confirmed")
    detail = ", ".join(
        f"{axis} {pred}: {'✓' if ok else '✗'} ({meas})"
        for axis, pred, ok, meas in axes
    )
    return label, detail


def render(exp_dir):
    exp_dir = Path(exp_dir)
    rig = _load_json(exp_dir / "rig.json", {})
    seedj = _load_json(exp_dir / "seed.json", {})
    loopj = _load_json(exp_dir / "loop.json", {})
    pareto = _load_json(exp_dir / "pareto.json", [])
    stats = _candidate_stats(_records(exp_dir))

    lines = [f"# Experiment lineage — {exp_dir.name}", ""]

    lines += ["## Rig", ""]
    if rig:
        verdict = ("**saturated**" if rig.get("saturated")
                   else "arena discriminates")
        lines.append(
            f"- oracle {rig.get('oracle_mean')} · null {rig.get('null_mean')}"
            f" · one-shot probe {rig.get('probe_mean')} — {verdict}"
        )
    else:
        lines.append("- (no rig.json recorded)")

    lines += ["", "## Landscape scan (seed population)", ""]
    if seedj:
        lines.append(
            f"rng_seed {seedj.get('rng_seed')} · packet stances: "
            + ", ".join(seedj.get("packet_stances", []))
        )
        combos = seedj.get("combos", [])
        seeds = sorted(
            (c for c in stats if _seed_index(c) is not None),
            key=_seed_index,
        )
        if seeds:
            lines += ["",
                      "| seed | model | thinking | tools | mean | n | cost |",
                      "|---|---|---|---|---|---|---|"]
            for sid in seeds:
                i = _seed_index(sid) - 1
                combo = combos[i] if i < len(combos) else {}
                s = stats[sid]
                lines.append(
                    f"| {sid} | {combo.get('model', '?')} "
                    f"| {combo.get('thinking', '?')} "
                    f"| {combo.get('policy_name', '?')} "
                    f"| {s['mean']:.3f} | {s['n']} | ${s['cost']:.3f} |"
                )
    else:
        lines.append("- (no seed.json recorded)")

    lines += ["", "## Generations (hypothesis → measurement → decision)", ""]
    history = loopj.get("history", [])
    if not history:
        lines.append("- (no search generations recorded)")
    for h in history:
        gen = f"g{h.get('generation')}.{h.get('attempt', '?')}"
        if "proposal_error" in h:
            lines.append(
                f"- {gen} parent `{h.get('parent_id')}` — **proposal "
                f"rejected**: {h['proposal_error']}"
            )
            continue
        verdict = ("**improvement — kept as a direction**"
                   if h.get("improved")
                   else "no improvement — direction discarded")
        donor = f" (transplant from `{h['donor']}`)" if h.get("donor") else ""
        lines += [
            f"- {gen} `{h.get('child_id')}` ← `{h.get('parent_id')}` "
            f"(slot `{h.get('slot_changed')}`){donor}",
            f"  - hypothesis: {h.get('hypothesis')}",
            f"  - measured: reward {h.get('reward_mean')} vs parent "
            f"{h.get('parent_reward_mean')} (paired Δ "
            f"{h.get('mean_task_delta')}) → {verdict}",
        ]
        scored = hypothesis_verdict(h)
        if scored:
            label, detail = scored
            lines.append(f"  - {label}: {detail}")

    alarms = loopj.get("alarms", [])
    if alarms:
        lines += ["", "## Meta-eval alarms", ""]
        lines += [f"- **{a.get('kind')}**: {a.get('detail')}" for a in alarms]

    lines += ["", "## Outcome", ""]
    lines.append(
        f"- stop: {loopj.get('stop_reason')} · generations "
        f"{loopj.get('generations')} · known spend "
        f"${loopj.get('spend_known_usd')}"
    )
    certified = loopj.get("certified")
    if certified is not None:
        lines.append(f"- certified: {', '.join(certified) or 'none'}")
    for e in pareto:
        mark = " ← **recommended**" if e.get("recommended") else ""
        cpt = e.get("cost_usd_per_trial")
        cost = f", ${cpt:.4f}/trial" if cpt is not None else ""
        lines.append(
            f"- {e['candidate_id']} (hash {e.get('composition_hash')}): "
            f"reward {e.get('reward_mean')}{cost}{mark}"
        )

    lines += ["", "## What this run taught us", ""]
    taught = False
    for h in history:
        if "proposal_error" in h:
            continue
        taught = True
        scored = hypothesis_verdict(h)
        if scored:
            label, detail = scored
            tag = f"{label.replace('prediction ', '')}: {detail}"
        else:
            tag = ("confirmed" if h.get("improved")
                   else f"not confirmed (Δ {h.get('mean_task_delta')})")
        lines.append(f"- [{tag}] {h.get('hypothesis')}")
    for a in alarms:
        taught = True
        lines.append(f"- [arena] {a.get('detail')}")
    if not taught:
        lines.append("- (none recorded)")
    return "\n".join(lines) + "\n"


def notebook_entry(exp_dir, spec, arena_cfg):
    """A short committed lab-notebook entry; lineage.md holds the full story."""
    exp_dir = Path(exp_dir)
    loopj = _load_json(exp_dir / "loop.json", {})
    pareto = _load_json(exp_dir / "pareto.json", [])
    pick = next((e for e in pareto if e.get("recommended")), None)
    lines = [
        f"\n## {exp_dir.name}",
        "",
        f"- spec `{spec.get('id')}` (mode {spec.get('mode')}) on arena "
        f"`{arena_cfg.get('id')}` v{arena_cfg.get('version')}",
        f"- stop: {loopj.get('stop_reason')} · spend "
        f"${loopj.get('spend_known_usd')} · generations "
        f"{loopj.get('generations')}",
    ]
    if pick:
        lines.append(
            f"- recommended: `{pick['candidate_id']}` (hash "
            f"{pick.get('composition_hash')}, reward {pick.get('reward_mean')}"
            f", certified={pick.get('certified')})"
        )
    confirmed = [h for h in loopj.get("history", []) if h.get("improved")]
    if confirmed:
        lines.append("- confirmed hypotheses: " + "; ".join(
            (h.get("hypothesis") or "")[:110] for h in confirmed
        ))
    for a in loopj.get("alarms", [])[:3]:
        lines.append(f"- alarm: {a.get('kind')} — {(a.get('detail') or '')[:140]}")
    lines.append(f"- full story: {exp_dir.name}/lineage.md")
    return "\n".join(lines) + "\n"


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    exp_dir = Path(sys.argv[1])
    text = render(exp_dir)
    (exp_dir / "lineage.md").write_text(text)
    print(f"wrote {exp_dir / 'lineage.md'}")


if __name__ == "__main__":
    main()
