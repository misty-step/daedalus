#!/usr/bin/env python3
"""Score review findings against a seeded-defect answer key.

Contract (DESIGN.md): reward = max(0, recall - 0.2 * false_positives), except
on a clean task (empty answer key) where any finding at all scores 0 --
inventing defects on a sound change fails the task's whole point.

A finding matches a defect when `file` and `category` are equal and the
finding's `line` falls inside the defect's [line_start, line_end]. Each defect
matches at most once (greedy, in finding order). Findings that match nothing
count as false positives. A missing or malformed findings.json scores 0 --
failing to follow the output contract is a failure.
"""

import json
import sys
from pathlib import Path

FP_PENALTY = 0.2


def score(findings_path, expected_path):
    expected = json.loads(Path(expected_path).read_text())["defects"]
    result = {
        "reward": 0.0,
        "recall": 0.0,
        "matched": [],
        "false_positives": 0,
        "expected_defects": len(expected),
        "error": None,
    }
    try:
        findings = json.loads(Path(findings_path).read_text())["findings"]
        if not isinstance(findings, list):
            raise ValueError("findings is not a list")
    except Exception as exc:  # noqa: BLE001 - any malformed output scores 0
        result["error"] = f"invalid findings: {exc}"
        return result

    unmatched = {d["id"]: d for d in expected}
    false_positives = 0
    for finding in findings:
        try:
            file = finding["file"]
            line = int(finding["line"])
            category = finding["category"]
        except (KeyError, TypeError, ValueError):
            false_positives += 1
            continue
        hit = next(
            (
                defect_id
                for defect_id, d in unmatched.items()
                if d["file"] == file
                and d["category"] == category
                and d["line_start"] <= line <= d["line_end"]
            ),
            None,
        )
        if hit is None:
            false_positives += 1
        else:
            result["matched"].append(hit)
            del unmatched[hit]

    recall = 1.0 if not expected else len(result["matched"]) / len(expected)
    result["recall"] = round(recall, 4)
    result["false_positives"] = false_positives
    if not expected and false_positives:
        result["reward"] = 0.0
    else:
        result["reward"] = round(
            max(0.0, recall - FP_PENALTY * false_positives), 4
        )
    return result


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: score.py <findings.json> <expected.json>")
    print(json.dumps(score(sys.argv[1], sys.argv[2]), indent=2))
