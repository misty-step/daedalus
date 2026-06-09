"""Grader regression tests. A silent scoring bug poisons every future
experiment; these are the highest-leverage assertions in the repo."""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
from score import score  # noqa: E402


def write(tmp_path, name, payload):
    p = tmp_path / name
    p.write_text(json.dumps(payload))
    return p


def expected_two_defects(tmp_path):
    return write(
        tmp_path,
        "expected.json",
        {
            "defects": [
                {
                    "id": "d1",
                    "file": "a.py",
                    "line_start": 5,
                    "line_end": 10,
                    "category": "security",
                },
                {
                    "id": "d2",
                    "file": "a.py",
                    "line_start": 20,
                    "line_end": 22,
                    "category": "correctness",
                },
            ]
        },
    )


def expected_clean(tmp_path):
    return write(tmp_path, "expected.json", {"defects": []})


def findings(tmp_path, items):
    return write(tmp_path, "findings.json", {"findings": items})


def test_perfect_recall_no_fp(tmp_path):
    f = findings(
        tmp_path,
        [
            {"file": "a.py", "line": 7, "category": "security"},
            {"file": "a.py", "line": 21, "category": "correctness"},
        ],
    )
    r = score(f, expected_two_defects(tmp_path))
    assert r["reward"] == 1.0
    assert sorted(r["matched"]) == ["d1", "d2"]
    assert r["false_positives"] == 0


def test_half_recall_plus_one_fp_is_point_three(tmp_path):
    f = findings(
        tmp_path,
        [
            {"file": "a.py", "line": 7, "category": "security"},
            {"file": "a.py", "line": 99, "category": "concurrency"},
        ],
    )
    r = score(f, expected_two_defects(tmp_path))
    assert r["reward"] == 0.3
    assert r["recall"] == 0.5
    assert r["false_positives"] == 1


def test_empty_findings_on_defective_task_scores_zero(tmp_path):
    f = findings(tmp_path, [])
    r = score(f, expected_two_defects(tmp_path))
    assert r["reward"] == 0.0
    assert r["recall"] == 0.0


def test_clean_task_silence_scores_one(tmp_path):
    f = findings(tmp_path, [])
    r = score(f, expected_clean(tmp_path))
    assert r["reward"] == 1.0


def test_clean_task_any_invented_finding_is_hard_zero(tmp_path):
    f = findings(
        tmp_path, [{"file": "a.py", "line": 1, "category": "correctness"}]
    )
    r = score(f, expected_clean(tmp_path))
    assert r["reward"] == 0.0
    assert r["false_positives"] == 1


def test_category_mismatch_is_fp_not_match(tmp_path):
    f = findings(tmp_path, [{"file": "a.py", "line": 7, "category": "correctness"}])
    r = score(f, expected_two_defects(tmp_path))
    assert r["matched"] == []
    assert r["false_positives"] == 1


def test_line_outside_range_is_fp(tmp_path):
    f = findings(tmp_path, [{"file": "a.py", "line": 11, "category": "security"}])
    r = score(f, expected_two_defects(tmp_path))
    assert r["matched"] == []
    assert r["false_positives"] == 1


def test_one_defect_matches_at_most_once(tmp_path):
    f = findings(
        tmp_path,
        [
            {"file": "a.py", "line": 6, "category": "security"},
            {"file": "a.py", "line": 8, "category": "security"},
        ],
    )
    r = score(f, expected_two_defects(tmp_path))
    assert r["matched"] == ["d1"]
    assert r["false_positives"] == 1
    assert r["reward"] == 0.3


def test_missing_findings_file_scores_zero_with_error(tmp_path):
    r = score(tmp_path / "nope.json", expected_two_defects(tmp_path))
    assert r["reward"] == 0.0
    assert r["error"]


def test_malformed_findings_scores_zero_with_error(tmp_path):
    p = tmp_path / "findings.json"
    p.write_text("not json {")
    r = score(p, expected_two_defects(tmp_path))
    assert r["reward"] == 0.0
    assert r["error"]


def test_findings_not_a_list_scores_zero(tmp_path):
    p = tmp_path / "findings.json"
    p.write_text(json.dumps({"findings": "lots of issues"}))
    r = score(p, expected_two_defects(tmp_path))
    assert r["reward"] == 0.0
    assert r["error"]


def test_finding_missing_fields_counts_as_fp(tmp_path):
    f = findings(tmp_path, [{"file": "a.py"}, {"line": 7, "category": "security"}])
    r = score(f, expected_two_defects(tmp_path))
    assert r["false_positives"] == 2
    assert r["reward"] == 0.0


def test_reward_never_negative(tmp_path):
    f = findings(
        tmp_path,
        [
            {"file": "z.py", "line": i, "category": "correctness"}
            for i in range(1, 9)
        ],
    )
    r = score(f, expected_two_defects(tmp_path))
    assert r["reward"] == 0.0
    assert r["false_positives"] == 8
