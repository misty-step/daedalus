#!/usr/bin/env python3
"""Convert an experiment's committed run records into OTel-GenAI-shaped trace
JSON for a trace sink (Langfuse OTLP, or any OTel backend).

Design (DESIGN.md): JSONL run records stay canonical; tracing is a *view*
produced at export time, so the lab never takes a hard dependency on a moving
semantic-convention spec or on a running collector. One experiment = one
trace; one trial = one span, with GenAI attributes (model, tokens, cost) and
the daedalus-specific reward/candidate fields. Pure function of the run dir,
testable offline; emission to a live endpoint is a thin, separable step.
"""

import json
import sys
from pathlib import Path

OTEL_GENAI_VERSION = "1.30.0-draft"  # semconv pinned at export, not at runtime


def _records(exp_dir):
    path = Path(exp_dir) / "trials.jsonl"
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines()]


def trial_span(record):
    """One trial → one OTel span. GenAI semantic-convention attributes plus
    daedalus fields; cost/tokens are first-class so a sink shows cost/trace."""
    attrs = {
        "gen_ai.system": record.get("provider_served") or "openrouter",
        "gen_ai.request.model": record.get("model"),
        "gen_ai.usage.input_tokens": record.get("tokens_prompt"),
        "gen_ai.usage.output_tokens": record.get("tokens_completion"),
        "gen_ai.usage.cost_usd": record.get("cost_usd"),
        "daedalus.candidate_id": record.get("candidate_id"),
        "daedalus.candidate_kind": record.get("candidate_kind"),
        "daedalus.composition_hash": record.get("composition_hash"),
        "daedalus.task_id": record.get("task_id"),
        "daedalus.trial": record.get("trial"),
        "daedalus.reward": record.get("reward"),
        "daedalus.false_positives": record.get("false_positives"),
        "daedalus.harness_version": record.get("harness_version"),
    }
    return {
        "name": f"{record.get('candidate_id')}/{record.get('task_id')}"
                f"/t{record.get('trial')}",
        "span_id": record.get("run_id"),
        "start_time": record.get("ts_start"),
        "end_time": record.get("ts_end"),
        "status": "ERROR" if record.get("error") else "OK",
        "status_message": record.get("error"),
        "attributes": {k: v for k, v in attrs.items() if v is not None},
    }


def experiment_trace(exp_dir):
    """One experiment → one trace: a tree of per-candidate groups, each with
    its trial spans. Returns an OTLP-ish dict ready for a sink adapter."""
    exp_dir = Path(exp_dir)
    records = _records(exp_dir)
    spans = [trial_span(r) for r in records]
    total_cost = sum(r.get("cost_usd") or 0 for r in records)
    candidates = sorted({r.get("candidate_id") for r in records})
    return {
        "schema": f"otel-genai/{OTEL_GENAI_VERSION}",
        "trace_id": exp_dir.name,
        "name": f"daedalus experiment {exp_dir.name}",
        "attributes": {
            "daedalus.experiment": exp_dir.name,
            "daedalus.candidate_count": len(candidates),
            "daedalus.trial_count": len(records),
            "daedalus.cost_usd_total": round(total_cost, 6),
        },
        "spans": spans,
    }


def write_trace(exp_dir):
    exp_dir = Path(exp_dir)
    trace = experiment_trace(exp_dir)
    out = exp_dir / "trace.otel.json"
    out.write_text(json.dumps(trace, indent=2))
    return out


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: trace.py runs/<exp-dir>")
    out = write_trace(sys.argv[1])
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
