"""Trace-view tests: run records → OTel-GenAI trace JSON, offline. Validated
against the committed capstone experiment so the converter works on real
records, not just fixtures."""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "runner"))
import trace  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
CAPSTONE = REPO / "runs" / "20260610T160533Z-search-pr-review-v0"


def test_trial_span_carries_genai_and_daedalus_attrs():
    record = {
        "run_id": "r1", "ts_start": "t0", "ts_end": "t1",
        "candidate_id": "seed1", "candidate_kind": "pi", "task_id": "x",
        "trial": 1, "model": "z-ai/glm-5", "provider_served": "openrouter",
        "composition_hash": "abc", "tokens_prompt": 100,
        "tokens_completion": 20, "cost_usd": 0.01, "reward": 1.0,
        "false_positives": 0, "error": None,
    }
    span = trace.trial_span(record)
    a = span["attributes"]
    assert a["gen_ai.request.model"] == "z-ai/glm-5"
    assert a["gen_ai.usage.input_tokens"] == 100
    assert a["gen_ai.usage.cost_usd"] == 0.01
    assert a["daedalus.reward"] == 1.0
    assert span["status"] == "OK"


def test_error_trial_is_error_span_with_no_null_attrs():
    record = {"run_id": "r", "candidate_id": "c", "task_id": "x", "trial": 1,
              "error": "pi exited 1", "cost_usd": None, "reward": 0.0}
    span = trace.trial_span(record)
    assert span["status"] == "ERROR"
    assert span["status_message"] == "pi exited 1"
    # null-valued attributes are dropped, not emitted as null
    assert "gen_ai.usage.cost_usd" not in span["attributes"]


def test_experiment_trace_over_real_capstone_records():
    if not (CAPSTONE / "trials.jsonl").exists():
        return  # records optional in a stripped checkout
    tr = trace.experiment_trace(CAPSTONE)
    assert tr["schema"].startswith("otel-genai/")
    assert tr["trace_id"] == CAPSTONE.name
    assert tr["attributes"]["daedalus.trial_count"] == len(tr["spans"])
    assert tr["attributes"]["daedalus.cost_usd_total"] > 0
    # spans reference real candidates and carry reward
    assert any(s["attributes"].get("daedalus.candidate_id", "").startswith("seed")
               for s in tr["spans"])


def test_write_trace_roundtrips(tmp_path):
    exp = tmp_path / "exp"
    exp.mkdir()
    (exp / "trials.jsonl").write_text(json.dumps({
        "run_id": "r1", "candidate_id": "c", "task_id": "t", "trial": 1,
        "model": "m", "cost_usd": 0.02, "reward": 0.5, "error": None,
    }) + "\n")
    out = trace.write_trace(exp)
    loaded = json.loads(out.read_text())
    assert loaded["attributes"]["daedalus.cost_usd_total"] == 0.02
    assert len(loaded["spans"]) == 1
