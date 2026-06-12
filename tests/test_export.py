"""Export tests: launch contract + persona are a pure, parseable function of
the delivery dir, and the persona embeds the measured packet byte-for-byte."""

import sys
import tomllib
import subprocess
from pathlib import Path

import pytest

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
    packet = tmp_path / "runs" / "20260611T000000Z-demo" / "packets" / "packet.md"
    packet.parent.mkdir(parents=True)
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
    assert contract["approval"]["g3_approval"] == (
        "approvals/G3-pr-review-demo-agent.md"
    )
    assert contract["evidence"]["run_dir"].endswith("runs/20260611T000000Z-demo")
    assert contract["evidence"]["report"].endswith(
        "runs/20260611T000000Z-demo/report.md"
    )
    assert contract["observability"]["trace_artifact"].endswith(
        "runs/20260611T000000Z-demo/trace.otel.json"
    )
    assert "JSONL-only waiver" in contract["observability"]["trace_destination"]

    persona = paths["persona"].read_text()
    head, _, body = persona.partition("---\n\n")
    assert "name: demo-agent" in head
    assert "model: openrouter/z-ai/glm-5" in head
    assert f"composition_hash: {cand['_hash']}" in head
    # The deployed system prompt is byte-identical to the measured packet.
    assert body == cand["_packet_text"]
    handoff = paths["handoff"].read_text()
    assert "Bitter Blossom import shape" in handoff
    assert "Olympus AgentSpec import shape" in handoff
    assert f"composition hash | `{cand['_hash']}`" in handoff
    assert "prompt_ref: deliveries/" in handoff
    assert "Lab evidence is not launch approval" in handoff


def test_export_is_deterministic(tmp_path):
    delivery = build_delivery(tmp_path)
    a = export.export_delivery(delivery, SPEC, harness_version="9.9.9",
                               generated="2026-06-10T00:00:00Z")
    first = a["contract"].read_text()
    first_handoff = a["handoff"].read_text()
    b = export.export_delivery(delivery, SPEC, harness_version="9.9.9",
                               generated="2026-06-10T00:00:00Z")
    assert b["contract"].read_text() == first
    assert b["handoff"].read_text() == first_handoff


def test_export_requires_evidence_backed_prompt_packet(tmp_path):
    delivery = build_delivery(tmp_path)
    packet = tmp_path / "loose-packet.md"
    packet.write_text("No run evidence.\n")
    (delivery / "agent.toml").write_text(
        (delivery / "agent.toml").read_text().replace(
            str(delivery / "runs" / "20260611T000000Z-demo" / "packets" / "packet.md"),
            str(packet),
        )
    )
    with pytest.raises(ValueError, match="evidence pointers"):
        export.export_delivery(delivery, SPEC, harness_version="9.9.9")


def test_export_handoff_includes_incumbent_comparison(tmp_path):
    delivery = build_delivery(tmp_path)
    (delivery / "plane-incumbents.toml").write_text(
        """
[bitter_blossom]
agent = "review-coordinator"
version = "2"
model = "moonshotai/kimi-k2.6"
harness = "pi"
posting = "agent posts one PR comment directly through gh"
config_paths = [
  "plane/agents/review-coordinator.toml",
  "plane/tasks/review/task.toml",
  "plane/tasks/review/card.md",
]
notes = ["budgeted webhook task", "direct-post red line"]

[olympus]
agent = "charon"
version = "2"
model = "~moonshotai/kimi-latest"
harness = "pi"
posting = "strict JSON artifact; orchestrator validates and posts"
config_paths = [
  "orchestrator/agent-specs/charon.yaml",
  "orchestrator/prompts/charon-review.md",
]
notes = ["activation gated", "orchestrator-side posting"]
"""
    )
    paths = export.export_delivery(delivery, SPEC, harness_version="9.9.9",
                                   generated="2026-06-10T00:00:00Z")
    text = paths["handoff"].read_text()
    assert "review-coordinator v2" in text
    assert "moonshotai/kimi-k2.6" in text
    assert "charon v2" in text
    assert "~moonshotai/kimi-latest" in text
    assert "plane/agents/review-coordinator.toml" in text
    assert "orchestrator/agent-specs/charon.yaml" in text
    assert "G3/G4/G5" in text


def test_export_uses_delivery_and_task_identity_for_non_pr_review(tmp_path):
    delivery = build_delivery(tmp_path / "launch-contract")
    spec = {
        **SPEC,
        "id": "launch-contract-v0",
        "goal": "Review launch contracts.",
        "inputs": {"description": "launch packet", "fixtures": "arenas/launch-contract-v0"},
        "trigger": {"intent": "Manual launch-contract review before G3/G4/G5 approval"},
    }
    paths = export.export_delivery(delivery, spec, harness_version="9.9.9",
                                   generated="2026-06-10T00:00:00Z")
    contract = tomllib.loads(paths["contract"].read_text())
    assert contract["approval"]["g3_approval"] == (
        "approvals/G3-launch-contract-demo-agent.md"
    )
    handoff = paths["handoff"].read_text()
    assert "prompt_ref: deliveries/launch-contract/persona.md" in handoff
    assert "contract_ref: deliveries/launch-contract/contract.toml" in handoff


def test_pi_version_accepts_stderr(monkeypatch):
    def fake_run(*args, **kwargs):
        return subprocess.CompletedProcess(args[0], 0, stdout="", stderr="0.78.1\n")

    monkeypatch.setattr(export.subprocess, "run", fake_run)
    assert export.pi_version() == "0.78.1"
