import hashlib
import sys
import tomllib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import launch  # noqa: E402


def build_delivery(tmp_path):
    prompt = tmp_path / "packets" / "packet.md"
    prompt.parent.mkdir()
    prompt.write_text("Measured review prompt.\n")
    (tmp_path / "contract.toml").write_text(
        f"""
contract = 1
agent = "demo-agent"
composition_hash = "abc123"
taskspec = "demo"
mode = "threshold-then-cheap"

[composition]
harness = "pi"
harness_version = "9.9.9"
provider = "openrouter"
model = "z-ai/glm-5"
thinking = "low"
tools = ["read", "bash"]
prompt_packet = "{prompt}"
timeout_sec = 600

[approval]
g3_signed = false
g3_approval = "approvals/G3-demo-agent.md"
note = "unsigned"
"""
    )
    return tmp_path, prompt


def test_unsigned_contract_refuses_deploy_packet_by_default(tmp_path):
    delivery, _ = build_delivery(tmp_path)
    with pytest.raises(launch.UnsignedLaunchError):
        launch.write_import_packet(
            delivery,
            plane="bitter-blossom",
            dry_run=False,
            generated="2026-06-11T00:00:00Z",
        )


def test_unsigned_contract_can_emit_sandbox_dry_run_packet(tmp_path):
    delivery, prompt = build_delivery(tmp_path)
    out = launch.write_import_packet(
        delivery,
        plane="bitter-blossom",
        dry_run=True,
        generated="2026-06-11T00:00:00Z",
    )
    packet = tomllib.loads(out.read_text())
    assert packet["plane"] == "bitter-blossom"
    assert packet["mode"] == "dry-run"
    assert packet["deployable"] is False
    assert packet["sandbox_required"] is True
    assert packet["primary_reviewer_allowed"] is False
    assert packet["refusal_reason"] == "G3 approval is unsigned"
    assert packet["prompt_packet_sha256"] == hashlib.sha256(
        prompt.read_bytes()
    ).hexdigest()


def test_signed_contract_still_requires_signed_g3_file(tmp_path):
    delivery, _ = build_delivery(tmp_path)
    contract_path = delivery / "contract.toml"
    contract_path.write_text(contract_path.read_text().replace(
        "g3_signed = false", "g3_signed = true"
    ))
    with pytest.raises(launch.UnsignedLaunchError, match="approval file"):
        launch.write_import_packet(
            delivery,
            plane="bitter-blossom",
            dry_run=False,
            generated="2026-06-11T00:00:00Z",
        )
