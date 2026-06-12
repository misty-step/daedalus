import subprocess
import sys
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import doctor  # noqa: E402

REPO = Path(__file__).resolve().parent.parent


def write_minimal_repo(tmp_path, primitive_date="2026-06-10", harness="0.78.1"):
    docs = tmp_path / "docs"
    docs.mkdir()
    (docs / "primitives.md").write_text(
        f"Verified live against the OpenRouter `/api/v1/models` endpoint on\n"
        f"**{primitive_date}**.\n\n"
        "Run pi trials **sequentially** per machine; parallel pi can deadlock.\n"
    )
    delivery = tmp_path / "deliveries" / "demo"
    delivery.mkdir(parents=True)
    approvals = tmp_path / "approvals"
    approvals.mkdir()
    (approvals / "G3-demo.md").write_text("**Status:** pending\n")
    (delivery / "contract.toml").write_text(
        f"""
contract = 1
agent = "demo"

[composition]
harness = "pi"
harness_version = "{harness}"

[approval]
g3_signed = false
g3_approval = "approvals/G3-demo.md"
"""
    )
    runs = tmp_path / "runs"
    runs.mkdir()
    return tmp_path


def status_by_name(checks):
    return {c.name: c.status for c in checks}


def test_doctor_passes_fresh_primitives_and_known_harness(tmp_path):
    repo = write_minimal_repo(tmp_path)
    checks = doctor.run_checks(repo, today=date(2026, 6, 12), use_git=False)
    statuses = status_by_name(checks)
    assert statuses["model-primitives"] == "ok"
    assert statuses["harness-versions"] == "ok"
    assert statuses["parallel-pi"] == "ok"
    assert statuses["approvals"] == "warn"


def test_doctor_flags_stale_primitives_and_unknown_harness(tmp_path):
    repo = write_minimal_repo(
        tmp_path, primitive_date="2026-04-01", harness="unknown"
    )
    checks = doctor.run_checks(repo, today=date(2026, 6, 12), use_git=False)
    statuses = status_by_name(checks)
    assert statuses["model-primitives"] == "fail"
    assert statuses["harness-versions"] == "fail"


def test_doctor_flags_dirty_run_artifacts_without_git(tmp_path):
    repo = write_minimal_repo(tmp_path)
    (repo / "runs" / "exp" / "artifacts").mkdir(parents=True)
    (repo / "runs" / "exp" / "artifacts" / "response.txt").write_text("raw\n")
    checks = doctor.run_checks(repo, today=date(2026, 6, 12), use_git=False)
    assert status_by_name(checks)["run-artifacts"] == "warn"


def test_cli_doctor_renders_summary():
    proc = subprocess.run(
        [sys.executable, str(REPO / "bin" / "daedalus"), "doctor"],
        capture_output=True,
        text=True,
        cwd=REPO,
        timeout=120,
    )
    assert proc.returncode == 0, proc.stderr
    assert "Daedalus doctor" in proc.stdout
    assert "model-primitives" in proc.stdout
    assert "approvals" in proc.stdout


def test_cold_start_surfaces_link_operator_sop():
    sop = "docs/operator-sop.md"
    paths = [
        REPO / "README.md",
        REPO / "ROADMAP.md",
        REPO / ".agents" / "skills" / "daedalus" / "SKILL.md",
        REPO / "deliveries" / "pr-review" / "DELIVERY.md",
        REPO / "deliveries" / "pr-review" / "plane-handoff.md",
    ]
    for path in paths:
        assert sop in path.read_text(), path
