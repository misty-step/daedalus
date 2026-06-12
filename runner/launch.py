#!/usr/bin/env python3
"""Approval-aware launch/import packet helpers.

These helpers deliberately stop before deployment. Unsigned contracts can
produce sandbox review artifacts, but any runtime-facing packet requires G3.
"""

import hashlib
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import swarm as swarm_mod  # noqa: E402


class UnsignedLaunchError(RuntimeError):
    """Raised when a launch/import path needs G3 but the contract is unsigned."""


class ContractValidationError(RuntimeError):
    """Raised when a launch contract is malformed or over-authorized."""


def _toml_str(value):
    return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"') + '"'


def _generated(value=None):
    return value or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def load_contract(delivery_dir):
    path = Path(delivery_dir) / "contract.toml"
    with open(path, "rb") as f:
        contract = tomllib.load(f)
    validate_contract(contract, delivery_dir)
    return contract


def _resolve_contract_path(ref, delivery_dir):
    path = Path(ref)
    if path.is_absolute():
        return path
    local = Path(delivery_dir) / path
    if local.exists():
        return local
    return REPO / path


def _require_keys(table, keys, label):
    missing = [k for k in keys if k not in table]
    if missing:
        raise ContractValidationError(
            f"{label} missing required field(s): {', '.join(missing)}"
        )


def _require_type(value, expected, label):
    if not isinstance(value, expected):
        names = (
            expected.__name__
            if isinstance(expected, type)
            else "|".join(t.__name__ for t in expected)
        )
        raise ContractValidationError(f"{label} must be {names}")


def validate_contract(contract, delivery_dir):
    """Validate contract.v1 before any import packet consumes it."""
    _require_keys(
        contract,
        ["contract", "agent", "composition_hash", "taskspec", "mode"],
        "contract",
    )
    if contract["contract"] != 1:
        raise ContractValidationError("contract must be version 1")
    for key in ("agent", "composition_hash", "taskspec", "mode"):
        _require_type(contract[key], str, key)
        if not contract[key]:
            raise ContractValidationError(f"{key} must not be empty")

    composition = contract.get("composition") or {}
    _require_keys(
        composition,
        [
            "harness",
            "harness_version",
            "provider",
            "model",
            "prompt_packet",
            "timeout_sec",
        ],
        "composition",
    )
    for key in ("harness", "harness_version", "provider", "model", "prompt_packet"):
        _require_type(composition[key], str, f"composition.{key}")
        if not composition[key]:
            raise ContractValidationError(f"composition.{key} must not be empty")
    _require_type(composition["timeout_sec"], (int, float), "composition.timeout_sec")
    prompt = _resolve_contract_path(composition["prompt_packet"], delivery_dir)
    if not prompt.is_file():
        raise ContractValidationError(f"prompt_packet does not exist: {prompt}")

    permissions = contract.get("permissions") or {}
    _require_keys(permissions, ["workspace", "env", "write_actions"], "permissions")
    _require_type(permissions["env"], list, "permissions.env")
    write_actions = str(permissions["write_actions"]).strip().lower()
    approval = contract.get("approval") or {}
    if write_actions != "none":
        if not approval.get("g4_signed"):
            raise ContractValidationError(
                "contract grants write authority before G4 approval"
            )
        _require_keys(approval, ["g4_approval"], "approval")
        _require_type(approval["g4_approval"], str, "approval.g4_approval")
        if not _approval_file_approved(approval["g4_approval"]):
            raise ContractValidationError(
                "G4 approval file is missing or unsigned"
            )

    budgets = contract.get("budgets") or {}
    _require_keys(budgets, ["max_cost_usd_per_run", "max_wall_sec"], "budgets")
    _require_type(
        budgets["max_cost_usd_per_run"], (int, float), "budgets.max_cost_usd_per_run"
    )
    _require_type(budgets["max_wall_sec"], (int, float), "budgets.max_wall_sec")
    if budgets["max_cost_usd_per_run"] < 0 or budgets["max_wall_sec"] <= 0:
        raise ContractValidationError("budgets must be positive")

    observability = contract.get("observability") or {}
    _require_keys(
        observability,
        ["arena", "trace_destination"],
        "observability",
    )

    evidence = contract.get("evidence") or {}
    _require_keys(evidence, ["run_dir", "trials"], "evidence")
    for key in ("run_dir", "trials"):
        _require_type(evidence[key], str, f"evidence.{key}")
        path = _resolve_contract_path(evidence[key], delivery_dir)
        if key == "run_dir" and not path.is_dir():
            raise ContractValidationError(f"evidence.run_dir does not exist: {path}")
        if key == "trials" and not path.is_file():
            raise ContractValidationError(f"evidence.trials does not exist: {path}")

    _require_keys(approval, ["g3_signed", "g3_approval"], "approval")
    _require_type(approval["g3_signed"], bool, "approval.g3_signed")
    _require_type(approval["g3_approval"], str, "approval.g3_approval")
    if "g4_signed" in approval:
        _require_type(approval["g4_signed"], bool, "approval.g4_signed")


def _prompt_hash(contract, delivery_dir):
    prompt = _resolve_contract_path(
        contract["composition"]["prompt_packet"], delivery_dir
    )
    return hashlib.sha256(prompt.read_bytes()).hexdigest()


def _approval(contract):
    return contract.get("approval") or {}


def _approval_file_approved(path):
    approval_path = Path(path)
    if not approval_path.is_absolute():
        approval_path = REPO / approval_path
    if not approval_path.exists():
        return False
    text = approval_path.read_text()
    return "**Status:** approved" in text or "**Status:** signed" in text


def _load_swarm_if_present(delivery_dir):
    path = Path(delivery_dir) / "swarm-contract.toml"
    if not path.exists():
        return None
    try:
        return swarm_mod.load_swarm_contract(delivery_dir)
    except swarm_mod.SwarmValidationError as exc:
        raise ContractValidationError(str(exc)) from exc


def require_g3(contract):
    approval = _approval(contract)
    if not approval.get("g3_signed"):
        raise UnsignedLaunchError("G3 approval is unsigned")
    if not _approval_file_approved(approval.get("g3_approval", "")):
        raise UnsignedLaunchError("G3 approval file is missing or unsigned")


def render_import_packet(delivery_dir, plane, dry_run=False, generated=None):
    """Render a control-plane import packet.

    Non-dry-run packets require G3. Dry-run packets are explicitly marked as
    non-deployable, sandbox-only, and never primary-reviewer-capable.
    """
    delivery_dir = Path(delivery_dir)
    swarm_contract = _load_swarm_if_present(delivery_dir)
    if swarm_contract is not None:
        return render_swarm_import_packet(
            delivery_dir,
            swarm_contract,
            plane=plane,
            dry_run=dry_run,
            generated=generated,
        )
    contract = load_contract(delivery_dir)
    refusal = ""
    if not dry_run:
        require_g3(contract)
    elif not _approval(contract).get("g3_signed"):
        refusal = "G3 approval is unsigned"

    approval = _approval(contract)
    deployable = not dry_run and bool(approval.get("g3_signed"))
    mode = "dry-run" if dry_run else "deployable"
    sandbox = "true" if dry_run else "false"
    primary_allowed = deployable and bool(approval.get("primary_reviewer_allowed"))
    primary = str(primary_allowed).lower()
    return f"""\
packet = 1
generated = {_toml_str(_generated(generated))}
plane = {_toml_str(plane)}
mode = {_toml_str(mode)}
source_contract = "contract.toml"
agent = {_toml_str(contract["agent"])}
composition_hash = {_toml_str(contract["composition_hash"])}
prompt_packet = {_toml_str(contract["composition"]["prompt_packet"])}
prompt_packet_sha256 = {_toml_str(_prompt_hash(contract, delivery_dir))}
deployable = {str(deployable).lower()}
sandbox_required = {sandbox}
primary_reviewer_allowed = {primary}
refusal_reason = {_toml_str(refusal)}

[gates]
g3_signed = {str(bool(approval.get("g3_signed"))).lower()}
g3_approval = {_toml_str(approval.get("g3_approval", ""))}
g4_required_for_write_authority = true
g5_required_for_prod_data_reingestion = true

[constraints]
write_authority = "none"
posting = "control-plane dry run only before G3"
"""


def render_swarm_import_packet(delivery_dir, contract, plane, dry_run=False,
                               generated=None):
    approval = contract.get("approval") or {}
    if not dry_run:
        if not approval.get("g3_signed"):
            raise UnsignedLaunchError("G3 approval is unsigned")
        if not _approval_file_approved(approval.get("g3_approval", "")):
            raise UnsignedLaunchError("G3 approval file is missing or unsigned")
    refusal = "G3 approval is unsigned" if dry_run and not approval.get("g3_signed") else ""
    deployable = not dry_run and bool(approval.get("g3_signed"))
    return f"""\
packet = 1
generated = {_toml_str(_generated(generated))}
plane = {_toml_str(plane)}
mode = {_toml_str("dry-run" if dry_run else "deployable")}
source_contract = "swarm-contract.toml"
suite = {_toml_str(contract["suite"])}
handoff_mode = {_toml_str(contract["handoff_mode"])}
deployable = {str(deployable).lower()}
sandbox_required = {str(dry_run).lower()}
primary_reviewer_allowed = false
refusal_reason = {_toml_str(refusal)}

[members]
required = [{", ".join(_toml_str(v) for v in contract["members"]["required"])}]
optional = [{", ".join(_toml_str(v) for v in contract["members"]["optional"])}]

[gates]
g3_signed = {str(bool(approval.get("g3_signed"))).lower()}
g3_approval = {_toml_str(approval.get("g3_approval", ""))}
g4_required_for_write_authority = true
g5_required_for_prod_data_reingestion = true

[constraints]
member_posting = "none"
review_posting = "control-plane dry run only before G3"
write_authority = "none"
"""


def write_import_packet(delivery_dir, plane, dry_run=False, generated=None,
                        out_dir=None):
    delivery_dir = Path(delivery_dir)
    text = render_import_packet(
        delivery_dir, plane=plane, dry_run=dry_run, generated=generated
    )
    tomllib.loads(text)
    default_dir = "launch-dry-run" if dry_run else "launch-pack"
    out = Path(out_dir) if out_dir else delivery_dir / default_dir
    out.mkdir(parents=True, exist_ok=True)
    path = out / f"{plane}.import-packet.toml"
    path.write_text(text)
    return path
