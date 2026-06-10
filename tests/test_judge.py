"""Judge-family tests: 0–5 rubric scoring, response parsing, Spearman, and
the calibration gate. No network — the judge call is injected."""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import judge  # noqa: E402

RUBRIC = """\
# Review-quality rubric
Score each 0–5:
- evidence: every finding cites file/line and explains the defect
- actionability: a developer could act on the finding without guessing
- severity_calibration: severity matches real impact
"""


def fake_call(scores):
    def _call(prompt, model):
        import json
        return json.dumps({"scores": scores}), 0.0009
    return _call


def test_normalize_0_5_to_unit():
    assert judge.normalize({"a": 5, "b": 5}) == 1.0
    assert judge.normalize({"a": 0, "b": 0}) == 0.0
    assert judge.normalize({"a": 3, "b": 0}) == 0.3  # mean 1.5 / 5
    assert judge.normalize({}) == 0.0


def test_parse_judge_response_variants():
    assert judge.parse_judge_response('{"scores": {"evidence": 4}}') == {
        "evidence": 4
    }
    # bare object without the scores wrapper
    assert judge.parse_judge_response('prose {"evidence": 5, "x": 2} tail') == {
        "evidence": 5, "x": 2
    }
    # out-of-range and non-numeric entries are dropped
    assert judge.parse_judge_response('{"a": 3, "b": 9, "c": "hi"}') == {"a": 3}
    with pytest.raises(ValueError):
        judge.parse_judge_response("no json")
    with pytest.raises(ValueError):
        judge.parse_judge_response('{"a": 7}')  # all out of range


def test_judge_score_normalizes_and_meters_cost():
    out = judge.judge_score(
        RUBRIC, "review the diff", {"findings": []},
        "anthropic/claude-x", fake_call({"evidence": 4, "actionability": 5,
                                         "severity_calibration": 3}),
    )
    assert out["score"] == round((4 + 5 + 3) / 3 / 5, 4)
    assert out["cost_usd"] == 0.0009
    assert out["rubric_hash"] == judge.rubric_hash(RUBRIC)


def test_spearman_concordant_discordant_and_constant():
    assert judge.spearman([1, 2, 3, 4], [1, 2, 3, 4]) == 1.0
    assert judge.spearman([1, 2, 3, 4], [4, 3, 2, 1]) == -1.0
    assert judge.spearman([1, 1, 1], [1, 2, 3]) is None  # constant series
    assert judge.spearman([1], [1]) is None


def test_calibration_gate_passes_when_judges_and_key_agree():
    a = [0.2, 0.4, 0.6, 0.8, 1.0]
    b = [0.3, 0.45, 0.55, 0.75, 0.95]   # same ranking
    key = [0.0, 0.5, 0.5, 1.0, 1.0]     # concordant with a
    gate = judge.calibration_gate(a, b, key)
    assert gate["passed"]
    assert gate["inter_judge_spearman"] >= 0.8


def test_calibration_gate_fails_on_judge_disagreement():
    a = [0.1, 0.3, 0.5, 0.7, 0.9]
    b = [0.9, 0.2, 0.8, 0.1, 0.5]       # scrambled vs a
    gate = judge.calibration_gate(a, b, [])
    assert not gate["passed"]
    assert any("inter-judge" in r for r in gate["reasons"])


def test_calibration_gate_fails_when_judge_contradicts_key():
    a = [0.2, 0.4, 0.6, 0.8, 1.0]
    b = [0.25, 0.45, 0.55, 0.85, 0.95]  # agrees with a
    key = [1.0, 1.0, 0.5, 0.0, 0.0]     # opposite ranking to the judges
    gate = judge.calibration_gate(a, b, key)
    assert not gate["passed"]
    assert any("judge-vs-key" in r for r in gate["reasons"])


def test_load_scorer_families_defaults_to_deterministic(tmp_path):
    (tmp_path / "tests").mkdir()
    fam = judge.load_scorer_families(tmp_path)
    assert fam == {"families": ["deterministic"]}
    (tmp_path / "tests" / "scoring.toml").write_text(
        'families = ["deterministic", "judge"]\n'
        '[judge]\nrubric = "rubric.md"\n'
        'models = ["anthropic/claude-x", "openai/gpt-y"]\n'
    )
    fam = judge.load_scorer_families(tmp_path)
    assert fam["families"] == ["deterministic", "judge"]
    assert fam["judge"]["rubric"] == "rubric.md"
