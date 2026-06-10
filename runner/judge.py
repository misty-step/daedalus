#!/usr/bin/env python3
"""The judge scorer family: a calibrated 0–5 rubric judge for qualities a
seeded-defect key cannot capture (finding quality, severity calibration,
actionability). The deterministic scorer (score.py) stays primary and is
never replaced; a judge score only counts toward keep/discard after it has
passed a calibration gate (DESIGN.md meta-eval; the red line is "never the
only oracle").

This module is pure scoring + calibration logic. The judge LLM call is
injected so the family is testable offline. Rubric files are versioned and
hashed into run records; judge cost is metered like any other spend.
"""

import hashlib
import json
import statistics
from pathlib import Path

# 0–5 per-criterion scale: research consensus that a 6-point scale maximizes
# human–LLM agreement vs binary or 0–10. Normalized to [0,1] for the reward.
SCALE_MAX = 5


def rubric_hash(rubric_text):
    return hashlib.sha256(rubric_text.encode()).hexdigest()[:16]


def normalize(criterion_scores):
    """Mean of per-criterion 0–5 scores, normalized to [0,1]."""
    vals = list(criterion_scores.values())
    if not vals:
        return 0.0
    return round(sum(vals) / len(vals) / SCALE_MAX, 4)


def parse_judge_response(text):
    """Pull {criterion: score} out of a judge model's JSON reply. Scores
    must be ints in [0,5]; anything else is a malformed judgment."""
    start = text.find("{")
    while start != -1:
        depth = 0
        for i in range(start, len(text)):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    try:
                        obj = json.loads(text[start : i + 1])
                    except json.JSONDecodeError:
                        break
                    scores = obj.get("scores", obj)
                    out = {}
                    for k, v in scores.items():
                        if isinstance(v, bool) or not isinstance(v, (int, float)):
                            continue
                        if 0 <= v <= SCALE_MAX:
                            out[k] = int(v)
                    if out:
                        return out
                    break
        start = text.find("{", start + 1)
    raise ValueError("no parseable 0–5 rubric scores in judge response")


def build_judge_prompt(rubric_text, instruction, findings):
    return (
        rubric_text
        + "\n\n## Task the agent was reviewing\n"
        + instruction
        + "\n\n## The agent's findings to score\n"
        + json.dumps(findings, indent=2)
        + "\n\nScore each rubric criterion from 0 to 5. Respond with ONLY a "
        'JSON object: {"scores": {"<criterion>": <0-5>, ...}}.'
    )


def judge_score(rubric_text, instruction, findings, judge_model, call):
    """One judge model's normalized score for a set of findings. `call` is
    the injected optimizer/judge LLM call returning (text, cost_usd)."""
    prompt = build_judge_prompt(rubric_text, instruction, findings)
    text, cost = call(prompt, judge_model)
    criterion_scores = parse_judge_response(text)
    return {
        "score": normalize(criterion_scores),
        "criterion_scores": criterion_scores,
        "judge_model": judge_model,
        "rubric_hash": rubric_hash(rubric_text),
        "cost_usd": cost,
    }


def spearman(xs, ys):
    """Spearman rank correlation; 1.0 for perfectly concordant rankings.
    Ties get average ranks. Undefined (returns None) when either series is
    constant — there is no ranking to agree on."""
    def ranks(vals):
        order = sorted(range(len(vals)), key=lambda i: vals[i])
        r = [0.0] * len(vals)
        i = 0
        while i < len(order):
            j = i
            while j + 1 < len(order) and vals[order[j + 1]] == vals[order[i]]:
                j += 1
            avg = (i + j) / 2 + 1
            for k in range(i, j + 1):
                r[order[k]] = avg
            i = j + 1
        return r

    if len(xs) != len(ys) or len(xs) < 2:
        return None
    if len(set(xs)) == 1 or len(set(ys)) == 1:
        return None
    rx, ry = ranks(xs), ranks(ys)
    n = len(xs)
    mean_rx, mean_ry = statistics.mean(rx), statistics.mean(ry)
    num = sum((rx[i] - mean_rx) * (ry[i] - mean_ry) for i in range(n))
    den = (
        sum((rx[i] - mean_rx) ** 2 for i in range(n))
        * sum((ry[i] - mean_ry) ** 2 for i in range(n))
    ) ** 0.5
    if den == 0:
        return None
    return round(num / den, 4)


def calibration_gate(judge_a, judge_b, deterministic, *, min_agreement=0.8):
    """Decide whether a judge family may count toward keep/discard.

    judge_a, judge_b: equal-length lists of the two judges' normalized scores
        over the SAME calibration outputs (inter-judge agreement).
    deterministic: deterministic reward over the SAME outputs that have a
        seeded key (judge-vs-key ranking agreement); pass [] when no keyed
        outputs exist.

    Passes only when two independent judges agree (Spearman ≥ min_agreement)
    AND, where a key exists, the judge ranking agrees with it. This is the
    "never the only oracle / calibrate before it counts" gate.
    """
    inter = spearman(judge_a, judge_b)
    vs_key = spearman(judge_a, deterministic) if deterministic else None
    reasons = []
    if inter is None:
        reasons.append("inter-judge agreement undefined (constant scores)")
    elif inter < min_agreement:
        reasons.append(f"inter-judge Spearman {inter} < {min_agreement}")
    if deterministic:
        if vs_key is None:
            reasons.append("judge-vs-key agreement undefined")
        elif vs_key < min_agreement:
            reasons.append(f"judge-vs-key Spearman {vs_key} < {min_agreement}")
    return {
        "passed": not reasons,
        "inter_judge_spearman": inter,
        "judge_vs_key_spearman": vs_key,
        "min_agreement": min_agreement,
        "reasons": reasons,
    }


def load_scorer_families(task_dir):
    """A task declares which families apply in scoring.toml; absence means
    deterministic-only (every existing task). Judge family names a rubric
    file (under the task's tests/) and the judge models."""
    cfg_path = Path(task_dir) / "tests" / "scoring.toml"
    if not cfg_path.exists():
        return {"families": ["deterministic"]}
    import tomllib
    return tomllib.loads(cfg_path.read_text())
