import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
from run import load_toml, select_tasks  # noqa: E402
from score import score  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
ARENA = REPO / "arenas" / "launch-contract-v0"


def task_dirs():
    return sorted((ARENA / "tasks").iterdir())


def test_launch_contract_oracle_scores_every_task():
    for task in task_dirs():
        result = score(
            task / "solution" / "findings.json",
            task / "tests" / "expected.json",
        )
        assert result["reward"] == 1.0, task.name
        assert result["false_positives"] == 0, task.name


def test_launch_contract_null_floor_matches_clean_fraction(tmp_path):
    null = tmp_path / "findings.json"
    null.write_text('{"findings": []}\n')
    rewards = [
        score(null, task / "tests" / "expected.json")["reward"]
        for task in task_dirs()
    ]
    clean = sum(
        1 for task in task_dirs()
        if not json.loads((task / "tests" / "expected.json").read_text())["defects"]
    )
    assert sum(rewards) / len(rewards) == pytest.approx(clean / len(task_dirs()))
    assert clean == 1


def test_launch_contract_uses_arena_defined_categories():
    task = ARENA / "tasks" / "bb-unsigned-primary"
    finding = task / "solution" / "findings.json"
    result = score(finding, task / "tests" / "expected.json")
    assert result["matched"] == ["unsigned-primary"]


def test_launch_contract_holdout_requires_final_flag():
    arena = load_toml(ARENA / "arena.toml")
    with pytest.raises(SystemExit):
        select_tasks(ARENA, arena, "holdout", task_filter=None, final=False)
    tasks = select_tasks(ARENA, arena, "holdout", task_filter=None, final=True)
    assert [t.name for t in tasks] == ["absolute-prompt-path"]
