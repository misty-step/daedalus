"""Export tests: launch contract + persona are a pure, parseable function of
the delivery dir, and the persona embeds the measured packet byte-for-byte."""

import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import export  # noqa: E402
import run as runner  # noqa: E402

SPEC = {
    "id": "pr-review-v0",
    "goal": "Find the real defects a change introduces.",
    "mode": "threshold-then-cheap",
    "inputs": {"description": "post-change repo + PR.diff",
               "fixtures": "arenas/pr-review-v2"},
    "output": {"contract": "findings.json"},
    "budget": {"max_cost_per_trial_usd": 0.5, "max_wall_per_trial_sec": 600},
    "trigger": {"intent": "GitHub PR webhook"},
}


def build_delivery(tmp_path):
    packet = tmp_path / "packet.md"
    packet.write_text("Review with evidence. Cite file and line. Stop.\n")
    (tmp_path / "agent.toml").write_text(
        'composition = 1\nid = "demo-agent"\nkind = "pi"\n'
        'provider_name = "openrouter"\nmodel = "z-ai/glm-5"\n'
        f'prompt_packet = "{packet}"\nthinking = "medium"\n'
        'tools = ["read", "bash"]\ntimeout_sec = 600\n'
    )
    return tmp_path


def test_export_writes_parseable_contract_and_faithful_persona(tmp_path):
    delivery = build_delivery(tmp_path)
    paths = export.export_delivery(delivery, SPEC, harness_version="9.9.9",
                                   generated="2026-06-10T00:00:00Z")
    contract = tomllib.loads(paths["contract"].read_text())
    cand = runner.load_candidate(delivery / "agent.toml")
    assert contract["contract"] == 1
    assert contract["agent"] == "demo-agent"
    assert contract["composition_hash"] == cand["_hash"]
    assert contract["composition"]["harness_version"] == "9.9.9"
    assert contract["composition"]["model"] == "z-ai/glm-5"
    assert contract["budgets"]["max_cost_usd_per_run"] == 0.5
    assert contract["trigger"]["intent"] == "GitHub PR webhook"
    assert contract["approval"]["g3_signed"] is False

    persona = paths["persona"].read_text()
    head, _, body = persona.partition("---\n\n")
    assert "name: demo-agent" in head
    assert "model: openrouter/z-ai/glm-5" in head
    assert f"composition_hash: {cand['_hash']}" in head
    # The deployed system prompt is byte-identical to the measured packet.
    assert body == cand["_packet_text"]


def test_export_is_deterministic(tmp_path):
    delivery = build_delivery(tmp_path)
    a = export.export_delivery(delivery, SPEC, harness_version="9.9.9",
                               generated="2026-06-10T00:00:00Z")
    first = a["contract"].read_text()
    b = export.export_delivery(delivery, SPEC, harness_version="9.9.9",
                               generated="2026-06-10T00:00:00Z")
    assert b["contract"].read_text() == first
