import json
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import workbench  # noqa: E402

REPO = Path(__file__).resolve().parent.parent


def write_probe_run(tmp_path, name="probe"):
    run = tmp_path / name
    run.mkdir()
    (run / "summary.json").write_text(json.dumps({
        "oracle": {"kind": "oracle", "reward_mean": 1.0},
        "null": {"kind": "null", "reward_mean": 0.0},
        "probe-oneshot": {"kind": "oneshot", "reward_mean": 0.0},
    }))
    return run


def test_correctness_specialist_spec_points_at_taxonomy_arena(tmp_path):
    spec = tomllib.loads(
        (REPO / "specs/pr-review-correctness/taskspec.toml").read_text()
    )
    assert spec["inputs"]["fixtures"] == "arenas/pr-review-correctness-v0"
    assert spec["lens"]["owned_categories"] == [
        "logic-invariant",
        "runtime-crash",
    ]
    assert "py-formatter-missing-crash" in spec["lens"]["authored_tasks"]

    report = workbench.validate_arena(
        REPO / spec["inputs"]["fixtures"],
        probe_run=write_probe_run(tmp_path),
        holdout_burn=9,
    )
    assert report.ok, report.messages
    assert report.oracle_mean == 1.0
    assert report.null_mean == 0.25
    assert report.holdout_counts == {
        "py-export-clear": 8,
        "py-plugin-cache": 8,
    }

    burned = workbench.validate_arena(
        REPO / spec["inputs"]["fixtures"],
        probe_run=write_probe_run(tmp_path, "burned-probe"),
    )
    assert not burned.ok
    assert any("holdout task burned" in message for message in burned.messages)


def test_security_specialist_spec_points_at_taxonomy_arena(tmp_path):
    spec = tomllib.loads(
        (REPO / "specs/pr-review-security/taskspec.toml").read_text()
    )
    assert spec["inputs"]["fixtures"] == "arenas/pr-review-security-v0"
    assert spec["lens"]["owned_categories"] == [
        "credential-exposure",
        "authz-bypass",
        "injection",
    ]

    report = workbench.validate_arena(
        REPO / spec["inputs"]["fixtures"],
        probe_run=write_probe_run(tmp_path),
    )
    assert report.ok, report.messages
    assert report.oracle_mean == 1.0
    assert report.null_mean == 0.3333
