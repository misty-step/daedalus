#!/usr/bin/env python3
"""Approval-aware launch/import packet helpers.

These helpers deliberately stop before deployment. Unsigned contracts can
produce sandbox review artifacts, but any runtime-facing packet requires G3.
"""

import hashlib
import tomllib
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


class UnsignedLaunchError(RuntimeError):
    """Raised when a launch/import path needs G3 but the contract is unsigned."""


def _toml_str(value):
    return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"') + '"'


def _generated(value=None):
    return value or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def load_contract(delivery_dir):
    path = Path(delivery_dir) / "contract.toml"
    with open(path, "rb") as f:
        return tomllib.load(f)


def _prompt_hash(contract):
    prompt = Path(contract["composition"]["prompt_packet"])
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
prompt_packet_sha256 = {_toml_str(_prompt_hash(contract))}
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
