"""Review-swarm delivery contracts.

`swarm-contract.v1` is the suite-level counterpart to a single-agent launch
contract. It does not deploy agents and it does not invent run evidence: export
requires a summary artifact that records measured cost, wall time, master
replay status, and handoff mode.
"""

import json
import tomllib
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SWARM_CONTRACT_VERSION = 1


class SwarmValidationError(RuntimeError):
    """Raised when a review-swarm delivery is malformed."""


def _toml_str(value):
    return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"') + '"'


def _generated(value=None):
    return value or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _load_toml(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def _load_json(path):
    try:
        return json.loads(Path(path).read_text())
    except Exception as exc:  # noqa: BLE001
        raise SwarmValidationError(f"{path}: invalid JSON: {exc}") from exc


def _resolve_ref(ref, delivery_dir):
    path = Path(ref)
    if path.is_absolute():
        return path
    if delivery_dir is not None:
        local = Path(delivery_dir) / path
        if local.exists():
            return local
    return REPO / path


def _require_number(value, label):
    if not isinstance(value, (int, float)):
        raise SwarmValidationError(f"{label} must be numeric")
    return float(value)


def _require_text(value, label):
    if not isinstance(value, str) or not value:
        raise SwarmValidationError(f"{label} must be a non-empty string")
    return value


def _require_table(value, label):
    if not isinstance(value, dict):
        raise SwarmValidationError(f"{label} must be a table")
    return value


def _require_existing_file(ref, label, delivery_dir):
    ref = _require_text(ref, label)
    path = _resolve_ref(ref, delivery_dir)
    if not path.is_file():
        raise SwarmValidationError(f"{label} does not exist: {ref}")
    return path


def _require_existing_dir(ref, label, delivery_dir):
    ref = _require_text(ref, label)
    path = _resolve_ref(ref, delivery_dir)
    if not path.is_dir():
        raise SwarmValidationError(f"{label} does not exist: {ref}")
    return path


def _validate_composition_hash(record, run_dir, label):
    expected = record["composition_hash"]
    comp_dir = run_dir / "compositions"
    if not comp_dir.is_dir():
        raise SwarmValidationError(f"{label}.evidence.run_dir has no compositions/")
    for path in comp_dir.glob("*.json"):
        try:
            payload = json.loads(path.read_text())
        except json.JSONDecodeError:
            continue
        if payload.get("composition_hash") == expected:
            return
    raise SwarmValidationError(
        f"{label}.composition_hash not found in run compositions"
    )


def _validate_evidence_record(record, label, delivery_dir):
    evidence = _require_table(record.get("evidence"), f"{label}.evidence")
    run_dir = _require_existing_dir(
        evidence.get("run_dir"),
        f"{label}.evidence.run_dir",
        delivery_dir,
    )
    _require_existing_file(
        evidence.get("trials"),
        f"{label}.evidence.trials",
        delivery_dir,
    )
    _validate_composition_hash(record, run_dir, label)


def _validate_member_records(summary, suite_spec, delivery_dir):
    spec_suite = suite_spec.get("suite") or {}
    members = _require_table(summary.get("members"), "members")
    required = spec_suite.get("required_members") or []
    optional = spec_suite.get("optional_members") or []
    known = set(required) | set(optional)
    for member_id in required:
        if member_id not in members:
            raise SwarmValidationError(f"required member missing: {member_id}")
    for member_id, record in members.items():
        if member_id not in known:
            raise SwarmValidationError(f"unknown member in summary: {member_id}")
        record = _require_table(record, f"members.{member_id}")
        _require_text(record.get("contract"), f"members.{member_id}.contract")
        _require_text(
            record.get("composition_hash"),
            f"members.{member_id}.composition_hash",
        )
        _validate_evidence_record(record, f"members.{member_id}", delivery_dir)


def _validate_master_record(summary, delivery_dir):
    master = _require_table(summary.get("master"), "master")
    _require_text(master.get("contract"), "master.contract")
    _require_text(master.get("composition_hash"), "master.composition_hash")
    _validate_evidence_record(master, "master", delivery_dir)
    replay = _require_table(master.get("real_member_replay"), "master.real_member_replay")
    if not isinstance(replay.get("passed"), bool):
        raise SwarmValidationError("master.real_member_replay.passed must be bool")
    _require_existing_file(
        replay.get("evidence"),
        "master.real_member_replay.evidence",
        delivery_dir,
    )
    return master


def _validate_quality_thresholds(summary, thresholds, mode):
    metrics = _require_table(summary.get("metrics"), "metrics")
    checks = [
        ("master_recall", "master_recall_min", ">="),
        ("blocking_recall", "blocking_recall_min", ">="),
        ("false_positive_carry", "false_positive_carry_max", "<="),
        ("duplicate_collapse", "duplicate_collapse_min", ">="),
    ]
    measured = {}
    for metric_key, threshold_key, direction in checks:
        value = _require_number(metrics.get(metric_key), f"metrics.{metric_key}")
        threshold = _require_number(
            thresholds.get(threshold_key),
            f"suite.thresholds.{threshold_key}",
        )
        measured[metric_key] = value
        if mode == "member-only":
            continue
        failed = value < threshold if direction == ">=" else value > threshold
        if failed:
            raise SwarmValidationError(
                f"metrics.{metric_key} does not satisfy {threshold_key}"
            )
    return measured


def validate_summary(summary, suite_spec, delivery_dir=None):
    """Validate suite summary against the launchability envelope."""
    suite = _require_table(summary.get("suite"), "suite")
    waivers = summary.get("waivers") or {}
    handoff = _require_table(summary.get("handoff"), "handoff")
    spec_suite = suite_spec.get("suite") or {}
    thresholds = spec_suite.get("thresholds") or {}

    total_cost = _require_number(suite.get("total_cost_usd"), "suite.total_cost_usd")
    total_wall = _require_number(suite.get("total_wall_sec"), "suite.total_wall_sec")
    cost_ceiling = float(spec_suite.get("cost_ceiling_usd", 2.0))
    wall_ceiling = float(spec_suite.get("wall_ceiling_sec", 1200))
    if total_cost > cost_ceiling and waivers.get("cost_ceiling") is not True:
        raise SwarmValidationError("suite exceeds cost ceiling without waiver")
    if total_wall > wall_ceiling and waivers.get("wall_time") is not True:
        raise SwarmValidationError("suite exceeds wall-time ceiling without waiver")

    mode = handoff.get("mode")
    if mode not in {"full-swarm", "member-only"}:
        raise SwarmValidationError("handoff.mode must be full-swarm or member-only")

    _validate_member_records(summary, suite_spec, delivery_dir)
    master = _validate_master_record(summary, delivery_dir)
    replay = master["real_member_replay"]
    if replay.get("passed") is not True and mode != "member-only":
        raise SwarmValidationError(
            "full-swarm handoff requires passing real-member replay"
        )
    measured = _validate_quality_thresholds(summary, thresholds, mode)
    return {
        "total_cost_usd": total_cost,
        "total_wall_sec": total_wall,
        "handoff_mode": mode,
        "metrics": measured,
        "master_contract": master["contract"],
    }


def render_swarm_contract(suite_spec, summary, generated=None, delivery_dir=None):
    suite = suite_spec.get("suite") or {}
    thresholds = suite.get("thresholds") or {}
    summary_info = validate_summary(summary, suite_spec, delivery_dir=delivery_dir)
    required = ", ".join(_toml_str(v) for v in suite.get("required_members") or [])
    optional = ", ".join(_toml_str(v) for v in suite.get("optional_members") or [])
    return f"""\
# Swarm contract - generated by daedalus export-suite.
swarm_contract = {SWARM_CONTRACT_VERSION}
generated = {_toml_str(_generated(generated))}
suite = {_toml_str(suite_spec.get("id", ""))}
mode = {_toml_str(suite_spec.get("mode", ""))}
taxonomy = {_toml_str(suite.get("taxonomy", ""))}
handoff_mode = {_toml_str(summary_info["handoff_mode"])}

[members]
required = [{required}]
optional = [{optional}]

[thresholds]
master_recall_min = {thresholds["master_recall_min"]}
blocking_recall_min = {thresholds["blocking_recall_min"]}
false_positive_carry_max = {thresholds["false_positive_carry_max"]}
duplicate_collapse_min = {thresholds["duplicate_collapse_min"]}

[budgets]
cost_ceiling_usd = {suite.get("cost_ceiling_usd", 2.0)}
cost_waiver_usd = {suite.get("cost_waiver_usd", 3.0)}
wall_ceiling_sec = {suite.get("wall_ceiling_sec", 1200)}
wall_waiver_sec = {suite.get("wall_waiver_sec", 1800)}
measured_cost_usd = {summary_info["total_cost_usd"]}
measured_wall_sec = {summary_info["total_wall_sec"]}

[evidence]
summary = "summary.json"
master_contract = {_toml_str(summary_info["master_contract"])}

[approval]
g3_signed = false
g3_approval = "approvals/G3-pr-review-swarm.md"
note = "Do not deploy as a primary reviewer until G3 is signed by a human; unsigned suite contracts may only produce sandbox dry-run packets."
"""


def export_suite(delivery_dir, suite_spec, generated=None):
    delivery_dir = Path(delivery_dir)
    summary_path = delivery_dir / "summary.json"
    if not summary_path.exists():
        raise SwarmValidationError(f"suite summary missing: {summary_path}")
    summary = _load_json(summary_path)
    contract_text = render_swarm_contract(
        suite_spec,
        summary,
        generated=generated,
        delivery_dir=delivery_dir,
    )
    contract = tomllib.loads(contract_text)
    delivery_dir.mkdir(parents=True, exist_ok=True)
    contract_path = delivery_dir / "swarm-contract.toml"
    contract_path.write_text(contract_text)
    handoff_path = delivery_dir / "plane-handoff.md"
    handoff_path.write_text(render_handoff(contract, summary))
    return {
        "contract": contract_path,
        "handoff": handoff_path,
        "summary": summary_path,
    }


def render_handoff(contract, summary):
    mode = contract["handoff_mode"]
    return "\n".join([
        f"# Review-swarm handoff: {contract['suite']}",
        "",
        "Lab evidence is not launch approval. G3/G4/G5 still gate deployment,",
        "write authority, and production-data re-ingestion.",
        "",
        "## Suite",
        "",
        f"- Mode: `{mode}`",
        f"- Required members: `{', '.join(contract['members']['required'])}`",
        f"- Optional members: `{', '.join(contract['members']['optional'])}`",
        f"- Measured cost: `${contract['budgets']['measured_cost_usd']}`",
        f"- Measured wall time: `{contract['budgets']['measured_wall_sec']}s`",
        "",
        "## Import Boundary",
        "",
        "- member agents write artifacts only.",
        "- The master/control plane owns synthesis and any later posting.",
        "- Unsigned use is sandbox-only and non-primary.",
        "",
        "## Residual Evidence",
        "",
        f"- Master replay: `{json.dumps(summary.get('master', {}), sort_keys=True)}`",
    ]) + "\n"


def validate_swarm_contract(contract, delivery_dir):
    if contract.get("swarm_contract") != SWARM_CONTRACT_VERSION:
        raise SwarmValidationError("swarm_contract must be version 1")
    for key in ("suite", "mode", "taxonomy", "handoff_mode"):
        if not isinstance(contract.get(key), str) or not contract.get(key):
            raise SwarmValidationError(f"{key} must not be empty")
    for table in ("members", "thresholds", "budgets", "evidence", "approval"):
        if not isinstance(contract.get(table), dict):
            raise SwarmValidationError(f"{table} table is required")
    for key in ("required", "optional"):
        if not isinstance(contract["members"].get(key), list):
            raise SwarmValidationError(f"members.{key} must be a list")
    summary = Path(delivery_dir) / contract["evidence"].get("summary", "")
    if not summary.is_file():
        raise SwarmValidationError(f"summary evidence does not exist: {summary}")
    if not isinstance(contract["approval"].get("g3_signed"), bool):
        raise SwarmValidationError("approval.g3_signed must be bool")


def load_swarm_contract(delivery_dir):
    delivery_dir = Path(delivery_dir)
    path = delivery_dir / "swarm-contract.toml"
    contract = _load_toml(path)
    validate_swarm_contract(contract, delivery_dir)
    return contract
