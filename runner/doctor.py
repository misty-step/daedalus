"""Cold-start readiness checks for Daedalus operators."""

import subprocess
import tomllib
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path


@dataclass
class Check:
    name: str
    status: str
    message: str


def _primitive_date(text):
    marker = "Verified live"
    if marker not in text:
        return None
    tail = text[text.find(marker):]
    start = tail.find("**")
    if start == -1:
        return None
    end = tail.find("**", start + 2)
    if end == -1:
        return None
    try:
        return datetime.strptime(tail[start + 2:end], "%Y-%m-%d").date()
    except ValueError:
        return None


def _check_primitives(repo, today, stale_days):
    path = repo / "docs" / "primitives.md"
    if not path.exists():
        return Check("model-primitives", "fail", "docs/primitives.md missing")
    verified = _primitive_date(path.read_text())
    if verified is None:
        return Check(
            "model-primitives",
            "fail",
            "could not find verified model-primitives date",
        )
    age = (today - verified).days
    if age > stale_days:
        return Check(
            "model-primitives",
            "fail",
            f"model primitives are stale: {verified} ({age} days old)",
        )
    return Check(
        "model-primitives",
        "ok",
        f"model primitives verified {verified} ({age} days old)",
    )


def _delivery_contracts(repo):
    deliveries = repo / "deliveries"
    if not deliveries.exists():
        return []
    return sorted(deliveries.glob("*/contract.toml"))


def _load_toml(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def _check_approvals(repo):
    missing = []
    unsigned = []
    for path in _delivery_contracts(repo):
        contract = _load_toml(path)
        approval = contract.get("approval") or {}
        ref = approval.get("g3_approval")
        if not ref:
            missing.append(str(path.relative_to(repo)))
            continue
        approval_path = Path(ref)
        if not approval_path.is_absolute():
            approval_path = repo / approval_path
        if not approval_path.exists():
            missing.append(ref)
        if not approval.get("g3_signed"):
            unsigned.append(ref)
    if missing:
        return Check("approvals", "fail", "missing approval file(s): " + ", ".join(missing))
    if unsigned:
        return Check(
            "approvals",
            "warn",
            "launch approval unsigned; lab evidence only: " + ", ".join(unsigned),
        )
    return Check("approvals", "ok", "delivery launch approvals are signed")


def _check_harness_versions(repo):
    unknown = []
    for path in _delivery_contracts(repo):
        contract = _load_toml(path)
        version = str((contract.get("composition") or {}).get("harness_version", ""))
        if not version or version == "unknown":
            unknown.append(str(path.relative_to(repo)))
    if unknown:
        return Check(
            "harness-versions",
            "fail",
            "unknown harness version in " + ", ".join(unknown),
        )
    return Check("harness-versions", "ok", "delivery harness versions are pinned")


def _check_parallel_pi(repo):
    text = (repo / "docs" / "primitives.md").read_text()
    if "sequential" not in text.lower() or "deadlock" not in text.lower():
        return Check(
            "parallel-pi",
            "fail",
            "docs/primitives.md does not warn about sequential pi trials",
        )
    return Check("parallel-pi", "ok", "pi concurrency constraint is documented")


def _check_run_artifacts(repo, use_git):
    artifact_files = sorted((repo / "runs").glob("*/artifacts/**/*"))
    artifact_files = [p for p in artifact_files if p.is_file()]
    if artifact_files:
        sample = ", ".join(str(p.relative_to(repo)) for p in artifact_files[:3])
        return Check("run-artifacts", "warn", "local run artifacts present: " + sample)
    if use_git:
        proc = subprocess.run(
            ["git", "-C", str(repo), "status", "--short", "--untracked-files=all", "--", "runs"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if proc.stdout.strip():
            return Check(
                "run-artifacts",
                "fail",
                "dirty run records/artifacts:\n" + proc.stdout.strip(),
            )
    return Check("run-artifacts", "ok", "no dirty run artifacts detected")


def run_checks(repo, today=None, stale_days=30, use_git=True):
    repo = Path(repo)
    today = today or date.today()
    return [
        _check_primitives(repo, today, stale_days),
        _check_approvals(repo),
        _check_harness_versions(repo),
        _check_parallel_pi(repo),
        _check_run_artifacts(repo, use_git),
    ]


def render(checks):
    lines = ["# Daedalus doctor", "", "| check | status | message |", "|---|---|---|"]
    for check in checks:
        lines.append(f"| {check.name} | {check.status} | {check.message} |")
    return "\n".join(lines) + "\n"


def has_failures(checks):
    return any(c.status == "fail" for c in checks)
