import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import taxonomy  # noqa: E402

REPO = Path(__file__).resolve().parent.parent


def write_taxonomy(tmp_path, body=None):
    path = tmp_path / "taxonomy.md"
    path.write_text(body or (REPO / "docs" / "review-swarm-taxonomy.md").read_text())
    return path


def write_suite(tmp_path, extra=""):
    path = tmp_path / "suite.toml"
    path.write_text(
        textwrap.dedent(
            f"""
            id = "pr-review-suite"
            mode = "threshold-then-cheap"

            [suite]
            master_spec = "specs/pr-review-master/taskspec.toml"
            required_members = ["general", "correctness", "security"]
            optional_members = ["verification", "simplification", "product"]
            cost_ceiling_usd = 2.0
            wall_ceiling_sec = 1200

            [suite.thresholds]
            master_recall_min = 0.9
            blocking_recall_min = 1.0
            false_positive_carry_max = 1
            duplicate_collapse_min = 0.9

            [suite.members.general]
            spec = "specs/pr-review/taskspec.toml"
            role = "baseline"
            status = "ready"
            evidence = "deliveries/pr-review/DELIVERY.md"

            [suite.members.correctness]
            spec = "specs/pr-review-correctness/taskspec.toml"
            role = "correctness"
            status = "ready"
            evidence = "docs/review-swarm-vertical-slice.md"

            [suite.members.security]
            spec = "specs/pr-review-security/taskspec.toml"
            role = "security"
            status = "ready"
            evidence = "docs/review-swarm-vertical-slice.md"

            [member_artifact]
            schema = "review-swarm-member-artifact.v1"
            statuses = ["ok", "error", "timeout", "truncated"]
            severities = ["blocking", "serious", "minor"]
            confidences = ["high", "medium", "low"]

            [search]
            base_packet = "packets/reviewer-v1.md"
            {extra}
            """
        )
    )
    return path


def test_review_swarm_taxonomy_validates_against_suite_spec(tmp_path):
    report = taxonomy.validate_taxonomy(
        REPO / "docs" / "review-swarm-taxonomy.md",
        REPO / "specs" / "pr-review-suite" / "taskspec.toml",
    )
    assert report.ok, report.messages
    assert report.lenses == [
        "general",
        "correctness",
        "security",
        "verification",
        "simplification",
        "product",
    ]


def test_taxonomy_rejects_missing_required_lens(tmp_path):
    source = (REPO / "docs" / "review-swarm-taxonomy.md").read_text()
    broken = source.replace('"security", ', "")
    report = taxonomy.validate_taxonomy(write_taxonomy(tmp_path, broken), write_suite(tmp_path))
    assert not report.ok
    assert any("required member missing from taxonomy lenses: security" in m for m in report.messages)


def test_taxonomy_rejects_category_for_unknown_lens(tmp_path):
    source = (REPO / "docs" / "review-swarm-taxonomy.md").read_text()
    broken = source.replace('lens = "security"', 'lens = "compliance"', 1)
    report = taxonomy.validate_taxonomy(write_taxonomy(tmp_path, broken), write_suite(tmp_path))
    assert not report.ok
    assert any("category credential-exposure uses unknown lens: compliance" in m for m in report.messages)


def test_taxonomy_rejects_suite_without_thresholds(tmp_path):
    suite = write_suite(tmp_path)
    suite.write_text(suite.read_text().replace("master_recall_min = 0.9\n", ""))
    report = taxonomy.validate_taxonomy(write_taxonomy(tmp_path), suite)
    assert not report.ok
    assert any("suite.thresholds missing master_recall_min" in m for m in report.messages)


def test_taxonomy_rejects_missing_member_spec_path(tmp_path):
    suite = write_suite(tmp_path)
    suite.write_text(suite.read_text().replace(
        'spec = "specs/pr-review-security/taskspec.toml"',
        'spec = "specs/pr-review-security/MISSING.toml"',
    ))
    report = taxonomy.validate_taxonomy(write_taxonomy(tmp_path), suite)
    assert not report.ok
    assert any(
        "suite.members.security.spec does not exist" in m
        for m in report.messages
    )


def test_cli_taxonomy_validate_writes_report():
    proc = subprocess.run(
        [
            sys.executable,
            str(REPO / "bin" / "daedalus"),
            "taxonomy-validate",
            "docs/review-swarm-taxonomy.md",
            "--suite",
            "specs/pr-review-suite/taskspec.toml",
        ],
        capture_output=True,
        text=True,
        cwd=REPO,
        timeout=120,
    )
    assert proc.returncode == 0, proc.stderr
    assert "Taxonomy validation: PASS" in proc.stdout
