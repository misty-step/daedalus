import json
import subprocess
import sys
import tomllib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import swarm  # noqa: E402

REPO = Path(__file__).resolve().parent.parent


def suite_spec():
    return tomllib.loads((REPO / "specs" / "pr-review-suite" / "taskspec.toml").read_text())


def write_run_evidence(delivery, hashes):
    run_dir = delivery / "evidence" / "run"
    (run_dir / "compositions").mkdir(parents=True, exist_ok=True)
    (run_dir / "trials.jsonl").write_text('{"candidate_id":"suite"}\n')
    for i, composition_hash in enumerate(hashes, start=1):
        (run_dir / "compositions" / f"c{i}.json").write_text(
            json.dumps({"composition_hash": composition_hash})
        )
    (delivery / "evidence" / "replay.json").write_text(
        json.dumps({"passed": True, "source": "test"})
    )


def write_summary(delivery, **overrides):
    delivery.mkdir(parents=True, exist_ok=True)
    run_dir = "evidence/run"
    hashes = ["generalhash", "correcthash", "securityhash", "masterhash"]
    write_run_evidence(delivery, hashes)
    summary = {
        "suite": {"total_cost_usd": 1.25, "total_wall_sec": 900},
        "waivers": {},
        "metrics": {
            "master_recall": 0.95,
            "blocking_recall": 1.0,
            "false_positive_carry": 1,
            "duplicate_collapse": 0.95,
        },
        "members": {
            "general": {
                "contract": "members/general/contract.toml",
                "composition_hash": "generalhash",
                "evidence": {"run_dir": run_dir, "trials": f"{run_dir}/trials.jsonl"},
            },
            "correctness": {
                "contract": "members/correctness/contract.toml",
                "composition_hash": "correcthash",
                "evidence": {"run_dir": run_dir, "trials": f"{run_dir}/trials.jsonl"},
            },
            "security": {
                "contract": "members/security/contract.toml",
                "composition_hash": "securityhash",
                "evidence": {"run_dir": run_dir, "trials": f"{run_dir}/trials.jsonl"},
            },
        },
        "master": {
            "contract": "master/contract.toml",
            "composition_hash": "masterhash",
            "evidence": {"run_dir": run_dir, "trials": f"{run_dir}/trials.jsonl"},
            "real_member_replay": {"passed": True, "evidence": "evidence/replay.json"},
        },
        "handoff": {"mode": "full-swarm"},
    }
    for key, value in overrides.items():
        summary[key] = value
    (delivery / "summary.json").write_text(json.dumps(summary))
    return summary


def test_export_suite_writes_swarm_contract_and_handoff(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    write_summary(delivery)

    paths = swarm.export_suite(
        delivery,
        suite_spec(),
        generated="2026-06-12T00:00:00Z",
    )

    contract = tomllib.loads(paths["contract"].read_text())
    assert contract["swarm_contract"] == 1
    assert contract["suite"] == "pr-review-suite"
    assert contract["members"]["required"] == ["general", "correctness", "security"]
    assert contract["budgets"]["measured_cost_usd"] == 1.25
    assert contract["budgets"]["measured_wall_sec"] == 900.0
    assert contract["evidence"]["master_contract"] == "master/contract.toml"
    assert contract["approval"]["g3_signed"] is False
    assert "member agents write artifacts only" in paths["handoff"].read_text()


def test_export_suite_requires_cost_waiver_above_ceiling(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    write_summary(delivery, suite={"total_cost_usd": 2.5, "total_wall_sec": 900})
    with pytest.raises(swarm.SwarmValidationError, match="cost ceiling"):
        swarm.export_suite(delivery, suite_spec())
    write_summary(
        delivery,
        suite={"total_cost_usd": 2.5, "total_wall_sec": 900},
        waivers={"cost_ceiling": True},
    )
    assert swarm.export_suite(delivery, suite_spec())["contract"].exists()


def test_export_suite_member_only_when_master_replay_fails(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    summary = write_summary(delivery)
    summary["master"]["real_member_replay"]["passed"] = False
    summary["handoff"] = {"mode": "full-swarm"}
    write_summary(
        delivery,
        master=summary["master"],
        handoff={"mode": "full-swarm"},
    )
    with pytest.raises(swarm.SwarmValidationError, match="real-member replay"):
        swarm.export_suite(delivery, suite_spec())
    summary["handoff"] = {"mode": "member-only"}
    write_summary(
        delivery,
        master=summary["master"],
        handoff={"mode": "member-only"},
    )
    contract = tomllib.loads(swarm.export_suite(delivery, suite_spec())["contract"].read_text())
    assert contract["handoff_mode"] == "member-only"


def test_export_suite_requires_member_contract_evidence(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    summary = write_summary(delivery)
    del summary["members"]["security"]
    write_summary(delivery, members=summary["members"])
    with pytest.raises(swarm.SwarmValidationError, match="required member missing"):
        swarm.export_suite(delivery, suite_spec())


def test_export_suite_rejects_full_swarm_below_quality_threshold(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    write_summary(
        delivery,
        metrics={
            "master_recall": 0.5,
            "blocking_recall": 1.0,
            "false_positive_carry": 1,
            "duplicate_collapse": 0.95,
        },
    )
    with pytest.raises(swarm.SwarmValidationError, match="master_recall"):
        swarm.export_suite(delivery, suite_spec())


@pytest.mark.parametrize(
    ("metric", "value"),
    [
        ("master_recall", 0.5),
        ("blocking_recall", 0.5),
        ("false_positive_carry", 2),
        ("duplicate_collapse", 0.5),
    ],
)
def test_export_suite_rejects_each_full_swarm_quality_threshold(tmp_path, metric, value):
    delivery = tmp_path / "pr-review-swarm"
    summary = write_summary(delivery)
    summary["metrics"][metric] = value
    write_summary(delivery, metrics=summary["metrics"])
    with pytest.raises(swarm.SwarmValidationError, match=metric):
        swarm.export_suite(delivery, suite_spec())


def test_export_suite_requires_real_evidence_paths_and_hashes(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    summary = write_summary(delivery)
    summary["members"]["general"]["composition_hash"] = "fabricated"
    write_summary(delivery, members=summary["members"])
    with pytest.raises(swarm.SwarmValidationError, match="composition_hash"):
        swarm.export_suite(delivery, suite_spec())


def test_cli_export_suite_and_launch_pack_dry_run(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    write_summary(delivery)
    export_proc = subprocess.run(
        [
            sys.executable,
            str(REPO / "bin" / "daedalus"),
            "export-suite",
            str(delivery),
            "--suite",
            "specs/pr-review-suite/taskspec.toml",
        ],
        cwd=REPO,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert export_proc.returncode == 0, export_proc.stderr
    assert "contract:" in export_proc.stdout

    launch_proc = subprocess.run(
        [
            sys.executable,
            str(REPO / "bin" / "daedalus"),
            "launch-pack",
            str(delivery),
            "--plane",
            "olympus",
            "--dry-run",
        ],
        cwd=REPO,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert launch_proc.returncode == 0, launch_proc.stderr
    packet_path = Path(launch_proc.stdout.strip().split(": ", 1)[1])
    packet = tomllib.loads(packet_path.read_text())
    assert packet["source_contract"] == "swarm-contract.toml"
    assert packet["sandbox_required"] is True
    assert packet["deployable"] is False
    assert packet["primary_reviewer_allowed"] is False


def test_cli_launch_pack_refuses_unsigned_swarm_without_dry_run(tmp_path):
    delivery = tmp_path / "pr-review-swarm"
    write_summary(delivery)
    export_proc = subprocess.run(
        [
            sys.executable,
            str(REPO / "bin" / "daedalus"),
            "export-suite",
            str(delivery),
            "--suite",
            "specs/pr-review-suite/taskspec.toml",
        ],
        cwd=REPO,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert export_proc.returncode == 0, export_proc.stderr

    launch_proc = subprocess.run(
        [
            sys.executable,
            str(REPO / "bin" / "daedalus"),
            "launch-pack",
            str(delivery),
            "--plane",
            "olympus",
        ],
        cwd=REPO,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert launch_proc.returncode != 0
    assert "G3 approval is unsigned" in launch_proc.stderr
