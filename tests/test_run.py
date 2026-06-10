"""Runner tests: helper units plus an offline end-to-end integration pass
(oracle ceiling and null floor over the real arena, no network)."""

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

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


def test_candidate_env_withholds_unrelated_secrets(monkeypatch):
    monkeypatch.setenv("GITHUB_TOKEN", "sekret")
    monkeypatch.setenv("OPENAI_API_KEY", "sekret2")
    monkeypatch.setenv("OPENROUTER_API_KEY", "or-key")
    env = runner.candidate_env({})
    assert "GITHUB_TOKEN" not in env
    assert "OPENAI_API_KEY" not in env
    assert env["OPENROUTER_API_KEY"] == "or-key"
    assert "PATH" in env


def test_candidate_env_respects_manifest_allowlist(monkeypatch):
    monkeypatch.setenv("CUSTOM_KEY", "v")
    monkeypatch.setenv("OPENROUTER_API_KEY", "or-key")
    env = runner.candidate_env({"env_allowlist": ["CUSTOM_KEY"]})
    assert env["CUSTOM_KEY"] == "v"
    assert "OPENROUTER_API_KEY" not in env


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


def invoke_runner(candidate, runs_dir, *extra):
    env = dict(os.environ, DAEDALUS_RUNS_DIR=str(runs_dir))
    return subprocess.run(
        [
            sys.executable,
            str(REPO / "runner" / "run.py"),
            "--candidate",
            str(REPO / "candidates" / f"{candidate}.toml"),
            "--arena",
            str(REPO / "arenas" / "pr-review-v0"),
            *extra,
        ],
        capture_output=True,
        text=True,
        env=env,
        cwd=REPO,
        timeout=120,
    )


def run_candidate(candidate, runs_dir, *extra):
    proc = invoke_runner(candidate, runs_dir, "--final", *extra)
    assert proc.returncode == 0, proc.stderr
    records = []
    for f in runs_dir.glob("*/trials.jsonl"):
        records += [json.loads(line) for line in f.read_text().splitlines()]
    return [r for r in records if r["candidate_id"] == candidate]


REQUIRED_FIELDS = {
    "run_id", "ts_start", "ts_end", "wall_ms", "runner_version", "arena_id",
    "arena_version", "task_id", "trial", "candidate_id", "candidate_kind",
    "composition_hash", "harness_version", "artifacts", "model",
    "provider_served", "tokens_prompt", "tokens_completion", "tokens_cached",
    "cost_usd", "reward", "recall", "matched", "false_positives", "error",
    "scorer_error",
}


def test_oracle_scores_one_everywhere_offline(tmp_path):
    records = run_candidate("oracle", tmp_path)
    assert len(records) == 6
    assert all(r["reward"] == 1.0 for r in records)
    for r in records:
        missing = REQUIRED_FIELDS - set(r)
        assert not missing, f"run record missing fields: {missing}"
        assert not any(k.startswith("_") for k in r), "private keys leaked"
    exp_dir = next(tmp_path.iterdir())
    snapshot = json.loads((exp_dir / "compositions" / "oracle.json").read_text())
    assert snapshot["composition_hash"] == records[0]["composition_hash"]
    summary = json.loads((exp_dir / "summary.json").read_text())
    assert summary["oracle"]["reward_mean"] == 1.0
    assert summary["oracle"]["tasks"]["py-auth-sqli"]["rewards"] == [1.0]
    art = exp_dir / records[0]["artifacts"]
    assert (art / "findings.json").exists()


def test_null_scores_exactly_clean_fraction_offline(tmp_path):
    records = run_candidate("null", tmp_path)
    assert len(records) == 6
    rewards = {r["task_id"]: r["reward"] for r in records}
    assert rewards["js-clean-rename"] == 1.0
    assert all(v == 0.0 for k, v in rewards.items() if k != "js-clean-rename")


def run_candidate_arena(candidate, arena, runs_dir):
    env = dict(os.environ, DAEDALUS_RUNS_DIR=str(runs_dir))
    proc = subprocess.run(
        [
            sys.executable, str(REPO / "runner" / "run.py"),
            "--candidate", str(REPO / "candidates" / f"{candidate}.toml"),
            "--arena", str(REPO / "arenas" / arena), "--final",
        ],
        capture_output=True, text=True, env=env, cwd=REPO, timeout=120,
    )
    assert proc.returncode == 0, proc.stderr
    records = []
    for f in runs_dir.glob("*/trials.jsonl"):
        records += [json.loads(line) for line in f.read_text().splitlines()]
    return records


@pytest.mark.parametrize("arena", ["pr-review-v0", "pr-review-v1"])
def test_arena_rig_oracle_ceiling_and_null_floor(arena, tmp_path):
    """Every arena must pass its rig: oracle 1.0 everywhere, null scores only
    the clean (empty-key) tasks. This is the gate that protects the grader."""
    oracle = run_candidate_arena("oracle", arena, tmp_path / "o")
    assert oracle and all(r["reward"] == 1.0 for r in oracle)
    null = run_candidate_arena("null", arena, tmp_path / "n")
    for r in null:
        # null reports nothing; it scores 1.0 only where the answer key is empty.
        expected = 1.0 if r["expected_defects"] == 0 else 0.0
        assert r["reward"] == expected, (arena, r["task_id"], r["reward"])


def test_holdout_requires_final_flag(tmp_path):
    proc = invoke_runner("null", tmp_path, "--split", "holdout")
    assert proc.returncode != 0
    assert "holdout" in proc.stderr


def test_full_arena_without_final_is_refused(tmp_path):
    proc = invoke_runner("null", tmp_path)
    assert proc.returncode != 0
    assert "holdout" in proc.stderr


def test_train_split_runs_without_final(tmp_path):
    proc = invoke_runner("null", tmp_path, "--split", "train")
    assert proc.returncode == 0, proc.stderr
    records = []
    for f in tmp_path.glob("*/trials.jsonl"):
        records += [json.loads(line) for line in f.read_text().splitlines()]
    assert sorted(r["task_id"] for r in records) == [
        "js-cart-total",
        "js-clean-rename",
        "py-auth-sqli",
    ]


def test_instruction_composed_from_template_and_intent():
    arena_dir = REPO / "arenas" / "pr-review-v0"
    arena = runner.load_toml(arena_dir / "arena.toml")
    text = runner.task_instruction(
        arena_dir, arena, arena_dir / "tasks" / "py-pagination"
    )
    assert "pagination helper" in text
    assert "{intent}" not in text
    assert "findings.json" in text


def test_composition_hash_tracks_prompt_packet(tmp_path):
    packet = tmp_path / "packet.md"
    packet.write_text("Review carefully.")
    manifest = tmp_path / "cand.toml"
    manifest.write_text(
        f'id = "x"\nkind = "oneshot"\nmodel = "m"\nprompt_packet = "{packet}"\n'
    )
    h1 = runner.load_candidate(manifest)["_hash"]
    packet.write_text("Review very carefully.")
    h2 = runner.load_candidate(manifest)["_hash"]
    assert h1 != h2
    packet.write_text("Review carefully.")
    assert runner.load_candidate(manifest)["_hash"] == h1
