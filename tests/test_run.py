"""Runner tests: helper units plus an offline end-to-end integration pass
(oracle ceiling and null floor over the real arena, no network)."""

import json
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import run as runner  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
PI_TRANSCRIPT = """\
{"type":"session","version":3,"id":"x","timestamp":"t","cwd":"/tmp"}
{"type":"message_end","message":{"role":"user","content":[]}}
{"type":"message_end","message":{"role":"assistant","provider":"openrouter","usage":{"input":397,"output":26,"cacheRead":10,"cacheWrite":0,"totalTokens":423,"cost":{"input":0.0002,"output":0.00008,"total":0.00028}}}}
{"type":"message_end","message":{"role":"assistant","provider":"openrouter","usage":{"input":500,"output":40,"cacheRead":0,"cacheWrite":0,"totalTokens":540,"cost":{"total":0.0005}}}}
"""


def test_extract_json_object_plain():
    assert runner.extract_json_object('{"findings": []}') == {"findings": []}


def test_extract_json_object_with_prose_and_fences():
    text = 'Sure! Here you go:\n```json\n{"findings": [{"a": 1}]}\n```\nDone.'
    assert runner.extract_json_object(text) == {"findings": [{"a": 1}]}


def test_extract_json_object_skips_broken_prefix():
    text = '{broken {"findings": []}'
    assert runner.extract_json_object(text) == {"findings": []}


def test_extract_json_object_raises_when_absent():
    try:
        runner.extract_json_object("no json here")
    except ValueError:
        return
    raise AssertionError("expected ValueError")


def test_extract_pi_usage_sums_assistant_message_ends():
    usage = runner.extract_pi_usage(PI_TRANSCRIPT)
    assert usage["tokens_prompt"] == 897
    assert usage["tokens_completion"] == 66
    assert usage["tokens_cached"] == 10
    assert usage["cost_usd"] == 0.00078
    assert usage["provider_served"] == "openrouter"


def test_extract_pi_usage_empty_on_no_events():
    assert runner.extract_pi_usage("plain text\n{\"type\":\"other\"}") == {}


def test_tree_digest_detects_tampering(tmp_path):
    (tmp_path / "tests").mkdir()
    key = tmp_path / "tests" / "expected.json"
    key.write_text("{}")
    before = runner.tree_digest(tmp_path / "tests")
    assert runner.tree_digest(tmp_path / "tests") == before
    key.write_text('{"defects": []}')
    assert runner.tree_digest(tmp_path / "tests") != before


def test_validate_task_dir_rejects_symlinks(tmp_path):
    (tmp_path / "environment").mkdir()
    (tmp_path / "environment" / "evil").symlink_to("/etc/passwd")
    try:
        runner.validate_task_dir(tmp_path)
    except RuntimeError:
        return
    raise AssertionError("expected RuntimeError on symlinked fixture")


def run_candidate(candidate, runs_dir):
    env = dict(os.environ, DAEDALUS_RUNS_DIR=str(runs_dir))
    proc = subprocess.run(
        [
            sys.executable,
            str(REPO / "runner" / "run.py"),
            "--candidate",
            str(REPO / "candidates" / f"{candidate}.toml"),
            "--arena",
            str(REPO / "arenas" / "pr-review-v0"),
        ],
        capture_output=True,
        text=True,
        env=env,
        cwd=REPO,
        timeout=120,
    )
    assert proc.returncode == 0, proc.stderr
    records = []
    for f in runs_dir.glob("*.jsonl"):
        records += [json.loads(line) for line in f.read_text().splitlines()]
    return [r for r in records if r["candidate_id"] == candidate]


REQUIRED_FIELDS = {
    "run_id", "ts_start", "ts_end", "wall_ms", "runner_version", "arena_id",
    "arena_version", "task_id", "trial", "candidate_id", "candidate_kind",
    "model", "provider_served", "tokens_prompt", "tokens_completion",
    "tokens_cached", "cost_usd", "reward", "recall", "matched",
    "false_positives", "error", "scorer_error",
}


def test_oracle_scores_one_everywhere_offline(tmp_path):
    records = run_candidate("oracle", tmp_path)
    assert len(records) == 6
    assert all(r["reward"] == 1.0 for r in records)
    for r in records:
        missing = REQUIRED_FIELDS - set(r)
        assert not missing, f"run record missing fields: {missing}"


def test_null_scores_exactly_clean_fraction_offline(tmp_path):
    records = run_candidate("null", tmp_path)
    assert len(records) == 6
    rewards = {r["task_id"]: r["reward"] for r in records}
    assert rewards["js-clean-rename"] == 1.0
    assert all(v == 0.0 for k, v in rewards.items() if k != "js-clean-rename")
