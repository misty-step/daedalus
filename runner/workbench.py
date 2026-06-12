"""Arena authoring and calibration helpers.

The workbench is intentionally file-first: it creates Harbor-shaped task
placeholders, validates frozen arena surfaces, records human adjudications,
and reports scoring disagreements without mutating scorer constants.
"""

import json
import re
import tempfile
import tomllib
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

from score import score


VERIFY_SH = """#!/usr/bin/env sh
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
WORKDIR=${1:-$PWD}
python3 "$HERE/../../../../../runner/score.py" "$WORKDIR/findings.json" "$HERE/expected.json"
"""

DEFAULT_TEMPLATE = """{intent}

Return ONLY findings.json with this shape:
{"findings": [{"file": "...", "line": 1, "category": "...", "description": "..."}]}
"""


class WorkbenchError(RuntimeError):
    """Raised for invalid authoring/calibration operations."""


@dataclass
class ValidationReport:
    arena_id: str
    arena_version: str
    ok: bool = True
    messages: list[str] = field(default_factory=list)
    oracle_mean: float | None = None
    null_mean: float | None = None
    probe_mean: float | None = None
    holdout_counts: dict[str, int] = field(default_factory=dict)

    def fail(self, message):
        self.ok = False
        self.messages.append(message)


def _load_toml(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def _task_dirs(arena_dir):
    tasks = arena_dir / "tasks"
    return sorted(d for d in tasks.iterdir() if d.is_dir())


def _version_tuple(version):
    parts = re.findall(r"\d+", version)
    return tuple(int(p) for p in parts)


def scaffold_task(arena_dir, task_id, taskspec="specs/TODO/taskspec.toml"):
    """Create a new Harbor-format task scaffold and minimal arena metadata."""
    arena_dir = Path(arena_dir)
    task_dir = arena_dir / "tasks" / task_id
    if task_dir.exists():
        raise WorkbenchError(f"task already exists: {task_dir}")

    arena_dir.mkdir(parents=True, exist_ok=True)
    arena_id = arena_dir.name
    arena_toml = arena_dir / "arena.toml"
    if not arena_toml.exists():
        arena_toml.write_text(
            f"""id = "{arena_id}"
version = "0.1.0"
taskspec = "{taskspec}"

[template]
file = "template.md"

[risk]
class = "low"
notes = "Scaffold placeholder; review before any candidate run."

[split]
train = []
validation = []
holdout = []
"""
        )
    template = arena_dir / "template.md"
    if not template.exists():
        template.write_text(DEFAULT_TEMPLATE)

    (task_dir / "environment").mkdir(parents=True)
    (task_dir / "environment" / "README.md").write_text(
        "Replace this placeholder with the candidate-visible fixture files.\n"
    )
    (task_dir / "tests").mkdir()
    (task_dir / "tests" / "expected.json").write_text('{"defects": []}\n')
    test_sh = task_dir / "tests" / "test.sh"
    test_sh.write_text(VERIFY_SH)
    test_sh.chmod(0o755)
    (task_dir / "solution").mkdir()
    (task_dir / "solution" / "findings.json").write_text('{"findings": []}\n')
    (task_dir / "intent.md").write_text(
        "Describe the task-specific review intent here.\n"
    )
    (task_dir / "task.toml").write_text(
        f'id = "{task_id}"\n\n[agent]\ntimeout_sec = 600\n\n'
        "[verifier]\ntimeout_sec = 60\n"
    )
    return task_dir


def _validate_expected_shape(path):
    try:
        defects = json.loads(path.read_text())["defects"]
    except Exception as exc:  # noqa: BLE001
        raise WorkbenchError(f"{path}: invalid expected.json: {exc}") from exc
    if not isinstance(defects, list):
        raise WorkbenchError(f"{path}: defects must be a list")
    required = {"id", "file", "line_start", "line_end", "category"}
    for i, defect in enumerate(defects, start=1):
        if not isinstance(defect, dict):
            raise WorkbenchError(f"{path}: defect {i} must be an object")
        missing = required - set(defect)
        if missing:
            raise WorkbenchError(
                f"{path}: defect {i} missing {', '.join(sorted(missing))}"
            )
        if int(defect["line_start"]) > int(defect["line_end"]):
            raise WorkbenchError(f"{path}: defect {defect['id']} has inverted span")
    return defects


def _validate_no_symlinks(task_dir):
    for p in task_dir.rglob("*"):
        if p.is_symlink():
            raise WorkbenchError(f"fixture contains symlink: {p}")


def _validate_splits(arena, task_ids, report):
    split = arena.get("split") or {}
    buckets = {
        name: set(split.get(name) or [])
        for name in ("train", "validation", "holdout")
    }
    assigned = {}
    for name, ids in buckets.items():
        for task_id in ids:
            if task_id in assigned:
                report.fail(
                    f"task assigned to multiple splits: {task_id} "
                    f"({assigned[task_id]}, {name})"
                )
            assigned[task_id] = name
            if task_id not in task_ids:
                report.fail(f"split references missing task: {task_id}")
    missing = sorted(task_ids - set(assigned))
    if missing:
        report.fail("not assigned to any split: " + ", ".join(missing))
    return buckets


def _validate_probe_run(probe_run, report):
    if probe_run is None:
        report.fail("one-shot probe not checked: pass --probe-run")
        return
    summary_path = Path(probe_run) / "summary.json"
    if not summary_path.exists():
        report.fail(f"one-shot probe summary missing: {summary_path}")
        return
    summary = json.loads(summary_path.read_text())
    oracle = next(
        (v for v in summary.values() if v.get("kind") == "oracle"),
        summary.get("oracle"),
    )
    probe = next(
        (v for v in summary.values() if v.get("kind") == "oneshot"),
        summary.get("probe-oneshot"),
    )
    if not oracle or not probe:
        report.fail("probe run must include oracle and one-shot records")
        return
    report.probe_mean = float(probe["reward_mean"])
    oracle_mean = float(oracle["reward_mean"])
    if report.probe_mean >= oracle_mean - 0.1:
        report.fail(
            "one-shot probe saturates the arena: "
            f"{report.probe_mean:.4f} >= oracle {oracle_mean:.4f} - 0.1"
        )


def _holdout_counts(arena_dir, holdout_tasks):
    counts = {task: 0 for task in holdout_tasks}
    ledger = arena_dir / "holdout-ledger.md"
    if not holdout_tasks or not ledger.exists():
        return counts
    for line in ledger.read_text().splitlines():
        if not line.startswith("|") or "---" in line or "tasks" in line:
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 4:
            continue
        tasks_cell = cells[3]
        for task in holdout_tasks:
            if task not in tasks_cell:
                continue
            count = 1
            match = re.search(rf"{re.escape(task)}\s*[x×]\s*(\d+)", tasks_cell)
            if match:
                count = int(match.group(1))
            counts[task] += count
    return counts


def validate_arena(arena_dir, probe_run=None, holdout_burn=5):
    """Validate an arena freeze gate without spending model budget."""
    arena_dir = Path(arena_dir)
    arena = _load_toml(arena_dir / "arena.toml")
    report = ValidationReport(arena["id"], arena["version"])
    task_dirs = _task_dirs(arena_dir)
    task_ids = {d.name for d in task_dirs}
    split_buckets = _validate_splits(arena, task_ids, report)

    oracle_rewards = []
    null_rewards = []
    with tempfile.TemporaryDirectory() as td:
        null_findings = Path(td) / "findings.json"
        null_findings.write_text('{"findings": []}\n')
        for task_dir in task_dirs:
            try:
                _validate_no_symlinks(task_dir)
                defects = _validate_expected_shape(task_dir / "tests" / "expected.json")
            except WorkbenchError as exc:
                report.fail(str(exc))
                continue
            oracle = score(
                task_dir / "solution" / "findings.json",
                task_dir / "tests" / "expected.json",
            )
            oracle_rewards.append(oracle["reward"])
            if oracle["reward"] != 1.0:
                report.fail(f"oracle is not 1.0 on {task_dir.name}")
            null = score(null_findings, task_dir / "tests" / "expected.json")
            null_rewards.append(null["reward"])
            expected_null = 1.0 if not defects else 0.0
            if null["reward"] != expected_null:
                report.fail(
                    f"null floor mismatch on {task_dir.name}: "
                    f"{null['reward']} != {expected_null}"
                )

    if oracle_rewards:
        report.oracle_mean = round(sum(oracle_rewards) / len(oracle_rewards), 4)
    if null_rewards:
        report.null_mean = round(sum(null_rewards) / len(null_rewards), 4)
    _validate_probe_run(probe_run, report)

    holdout_tasks = sorted(split_buckets.get("holdout") or [])
    report.holdout_counts = _holdout_counts(arena_dir, holdout_tasks)
    if holdout_tasks and not (arena_dir / "holdout-ledger.md").exists():
        report.fail("holdout ledger missing")
    for task, count in report.holdout_counts.items():
        if count >= holdout_burn:
            report.fail(
                f"holdout task burned: {task} has {count} exposures "
                f"(threshold {holdout_burn})"
            )
    return report


def render_validation_report(report):
    status = "PASS" if report.ok else "FAIL"
    lines = [
        f"# Arena freeze report: {report.arena_id} {report.arena_version}",
        "",
        f"Status: **{status}**",
        "",
        "| check | value |",
        "|---|---|",
        f"| oracle mean | `{report.oracle_mean}` |",
        f"| null mean | `{report.null_mean}` |",
        f"| one-shot probe mean | `{report.probe_mean}` |",
        f"| holdout exposures | `{json.dumps(report.holdout_counts, sort_keys=True)}` |",
        "",
    ]
    if report.messages:
        lines += ["## Findings", ""]
        lines += [f"- {m}" for m in report.messages]
        lines.append("")
    return "\n".join(lines)


def _replace_version(arena_toml, old, new):
    text = arena_toml.read_text()
    pattern = f'version = "{old}"'
    if pattern not in text:
        raise WorkbenchError(f"could not find version line for {old}")
    arena_toml.write_text(text.replace(pattern, f'version = "{new}"', 1))


def record_adjudication(
    arena_dir,
    task,
    finding,
    ruling,
    rationale,
    new_version=None,
    baseline_run=None,
):
    """Append a human adjudication and enforce ACCEPT version discipline."""
    arena_dir = Path(arena_dir)
    arena_toml = arena_dir / "arena.toml"
    arena = _load_toml(arena_toml)
    current_version = arena["version"]
    ruling = ruling.upper()
    if ruling not in {"ACCEPT", "OUT-OF-SCOPE"}:
        raise WorkbenchError("ruling must be ACCEPT or OUT-OF-SCOPE")
    version_note = current_version
    if ruling == "ACCEPT":
        if not new_version:
            raise WorkbenchError("ACCEPT requires --new-version")
        if _version_tuple(new_version) <= _version_tuple(current_version):
            raise WorkbenchError("ACCEPT requires a version bump")
        if not baseline_run:
            raise WorkbenchError("ACCEPT requires --baseline-run")
        baseline_report = validate_arena(arena_dir, probe_run=baseline_run)
        if not baseline_report.ok:
            raise WorkbenchError(
                "baseline rerun failed: " + "; ".join(baseline_report.messages)
            )
        _replace_version(arena_toml, current_version, new_version)
        version_note = f"{current_version} -> {new_version}"

    path = arena_dir / "adjudications.md"
    if not path.exists():
        path.write_text(
            f"# Answer-key adjudications - {arena['id']}\n\n"
            "| id | date | task | finding | ruling |\n"
            "|---|---|---|---|---|\n"
        )
    existing = len(re.findall(r"\| ADJ-\d+ \|", path.read_text()))
    adj_id = f"ADJ-{existing + 1}"
    date = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    with open(path, "a") as f:
        f.write(
            f"| {adj_id} | {date} | {task} | {finding} | **{ruling}** |\n\n"
            f"## {adj_id} - {task} ({ruling})\n\n"
            f"- **Rationale:** {rationale}\n"
            f"- **Arena version:** {version_note}\n"
        )
        if baseline_run:
            f.write(f"- **Baseline run:** `{baseline_run}`\n")
        f.write("\n")
    return path


def disagreements(findings_path, expected_path):
    """Report category/span misses without changing scorer constants."""
    findings = json.loads(Path(findings_path).read_text())["findings"]
    defects = json.loads(Path(expected_path).read_text())["defects"]
    rows = []
    for finding in findings:
        file = finding.get("file")
        try:
            line = int(finding.get("line"))
        except (TypeError, ValueError):
            continue
        category = finding.get("category")
        exact = [
            d for d in defects
            if d["file"] == file
            and d["category"] == category
            and int(d["line_start"]) <= line <= int(d["line_end"])
        ]
        if exact:
            continue
        in_span = [
            d for d in defects
            if d["file"] == file and int(d["line_start"]) <= line <= int(d["line_end"])
        ]
        if in_span:
            d = in_span[0]
            rows.append({
                "kind": "category",
                "finding": finding,
                "defect_id": d["id"],
                "expected_category": d["category"],
            })
            continue
        same_category = [
            d for d in defects if d["file"] == file and d["category"] == category
        ]
        if same_category:
            d = same_category[0]
            rows.append({
                "kind": "span",
                "finding": finding,
                "defect_id": d["id"],
                "expected_span": [d["line_start"], d["line_end"]],
            })
    return rows
