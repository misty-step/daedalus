import json
import subprocess
import sys
import tomllib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import workbench  # noqa: E402

REPO = Path(__file__).resolve().parent.parent


def write_arena(tmp_path):
    arena = tmp_path / "arena"
    task = arena / "tasks" / "buggy"
    (task / "environment").mkdir(parents=True)
    (task / "environment" / "app.py").write_text("print('bug')\n")
    (task / "intent.md").write_text("Find the bug.\n")
    (task / "tests").mkdir()
    (task / "tests" / "expected.json").write_text(json.dumps({
        "defects": [
            {
                "id": "bug",
                "file": "app.py",
                "line_start": 1,
                "line_end": 1,
                "category": "correctness",
                "note": "seeded",
            }
        ]
    }))
    (task / "tests" / "test.sh").write_text("#!/usr/bin/env sh\n")
    (task / "solution").mkdir()
    (task / "solution" / "findings.json").write_text(json.dumps({
        "findings": [
            {
                "file": "app.py",
                "line": 1,
                "category": "correctness",
                "description": "bug",
            }
        ]
    }))
    (task / "task.toml").write_text(
        'id = "buggy"\n\n[agent]\ntimeout_sec = 600\n\n'
        '[verifier]\ntimeout_sec = 60\n'
    )
    clean = arena / "tasks" / "clean"
    (clean / "environment").mkdir(parents=True)
    (clean / "environment" / "app.py").write_text("print('ok')\n")
    (clean / "intent.md").write_text("Confirm clean.\n")
    (clean / "tests").mkdir()
    (clean / "tests" / "expected.json").write_text('{"defects": []}\n')
    (clean / "tests" / "test.sh").write_text("#!/usr/bin/env sh\n")
    (clean / "solution").mkdir()
    (clean / "solution" / "findings.json").write_text('{"findings": []}\n')
    (clean / "task.toml").write_text(
        'id = "clean"\n\n[agent]\ntimeout_sec = 600\n\n'
        '[verifier]\ntimeout_sec = 60\n'
    )
    (arena / "template.md").write_text("{intent}\nReturn findings.json.\n")
    (arena / "arena.toml").write_text(
        f"""
id = "sample"
version = "0.1.0"
taskspec = "specs/sample/taskspec.toml"

[template]
file = "template.md"

[risk]
class = "low"

[split]
train = ["buggy"]
validation = ["clean"]
holdout = []
"""
    )
    return arena


def write_probe_run(tmp_path):
    run = tmp_path / "run"
    run.mkdir()
    (run / "summary.json").write_text(json.dumps({
        "oracle": {"kind": "oracle", "reward_mean": 1.0},
        "null": {"kind": "null", "reward_mean": 0.5},
        "probe-oneshot": {"kind": "oneshot", "reward_mean": 0.0},
    }))
    return run


def test_scaffold_task_creates_harbor_placeholders(tmp_path):
    arena = tmp_path / "new-arena"
    task = workbench.scaffold_task(arena, "new-task", taskspec="specs/x.toml")
    assert (arena / "arena.toml").exists()
    assert (arena / "template.md").exists()
    assert (task / "intent.md").exists()
    assert (task / "environment" / "README.md").exists()
    assert json.loads((task / "tests" / "expected.json").read_text()) == {
        "defects": []
    }
    assert json.loads((task / "solution" / "findings.json").read_text()) == {
        "findings": []
    }
    assert "runner/score.py" in (task / "tests" / "test.sh").read_text()


def test_validate_arena_checks_oracle_null_probe_and_splits(tmp_path):
    arena = write_arena(tmp_path)
    report = workbench.validate_arena(arena, probe_run=write_probe_run(tmp_path))
    assert report.ok, report.messages
    assert report.oracle_mean == 1.0
    assert report.null_mean == 0.5
    assert report.probe_mean == 0.0


def test_validate_arena_reports_missing_split_membership(tmp_path):
    arena = write_arena(tmp_path)
    text = (arena / "arena.toml").read_text()
    (arena / "arena.toml").write_text(text.replace('validation = ["clean"]', "validation = []"))
    report = workbench.validate_arena(arena, probe_run=write_probe_run(tmp_path))
    assert not report.ok
    assert any("not assigned to any split: clean" in m for m in report.messages)


def test_holdout_ledger_version_column_scopes_burn_count(tmp_path):
    arena = write_arena(tmp_path)
    text = (arena / "arena.toml").read_text()
    text = text.replace('version = "0.1.0"', 'version = "0.2.0"')
    text = text.replace('train = ["buggy"]', "train = []")
    text = text.replace("holdout = []", 'holdout = ["buggy"]')
    (arena / "arena.toml").write_text(text)
    (arena / "holdout-ledger.md").write_text(
        "| date | arena version | run | purpose | tasks |\n"
        "|---|---|---|---|---|\n"
        "| 2026-06-12 | 0.1.0 | old-run | old baseline | buggy x9 |\n"
        "| 2026-06-12 | 0.2.0 | new-run | new baseline | buggy x1 |\n"
    )
    report = workbench.validate_arena(arena, probe_run=write_probe_run(tmp_path))
    assert report.ok, report.messages
    assert report.holdout_counts == {"buggy": 1}


def test_format_holdout_ledger_row_records_version_and_exposure_count():
    row = workbench.format_holdout_ledger_row(
        "20260612T220412Z",
        "runs-search",
        ["cand-a", "cand-b"],
        ["holdout-a", "holdout-b"],
        trials_per_candidate=3,
        arena_version="0.2.0",
    )
    assert row == (
        "| 2026-06-12 | 0.2.0 | runs-search | "
        "holdout final: cand-a, cand-b | holdout-a x6, holdout-b x6 |\n"
    )


def test_adjudicate_accept_requires_version_bump_and_baselines(tmp_path):
    arena = write_arena(tmp_path)
    with pytest.raises(workbench.WorkbenchError, match="--new-version"):
        workbench.record_adjudication(
            arena,
            task="buggy",
            finding="candidate found missing issue",
            ruling="ACCEPT",
            rationale="key missed it",
        )
    workbench.record_adjudication(
        arena,
        task="buggy",
        finding="candidate found missing issue",
        ruling="ACCEPT",
        rationale="key missed it",
        new_version="0.2.0",
        baseline_run=write_probe_run(tmp_path),
    )
    assert tomllib.loads((arena / "arena.toml").read_text())["version"] == "0.2.0"
    text = (arena / "adjudications.md").read_text()
    assert "ACCEPT" in text
    assert "0.1.0 -> 0.2.0" in text


def test_disagreements_report_category_and_span_misses(tmp_path):
    expected = tmp_path / "expected.json"
    expected.write_text(json.dumps({
        "defects": [
            {
                "id": "escape",
                "file": "app.py",
                "line_start": 10,
                "line_end": 12,
                "category": "security",
            }
        ]
    }))
    findings = tmp_path / "findings.json"
    findings.write_text(json.dumps({
        "findings": [
            {"file": "app.py", "line": 11, "category": "correctness"},
            {"file": "app.py", "line": 14, "category": "security"},
        ]
    }))
    rows = workbench.disagreements(findings, expected)
    assert [r["kind"] for r in rows] == ["category", "span"]


def test_cli_validate_writes_freeze_report(tmp_path):
    arena = write_arena(tmp_path)
    report_path = tmp_path / "freeze.md"
    proc = subprocess.run(
        [
            sys.executable,
            str(REPO / "bin" / "daedalus"),
            "arena-validate",
            str(arena),
            "--probe-run",
            str(write_probe_run(tmp_path)),
            "--report",
            str(report_path),
        ],
        capture_output=True,
        text=True,
        cwd=REPO,
        timeout=120,
    )
    assert proc.returncode == 0, proc.stderr
    assert "freeze report" in proc.stdout
    assert "one-shot probe" in report_path.read_text()
