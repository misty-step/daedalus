//! Convert an experiment's committed run records into OTel-GenAI-shaped trace
//! JSON for a trace sink (Langfuse OTLP, or any OTel backend).
//!
//! Port of `runner/trace.py`. JSONL run records stay canonical; tracing is a
//! *view* produced at export time. One experiment = one trace; one trial = one
//! span. Pure function of the run dir, testable offline. Object key order and
//! `None`/falsy filtering follow Python dict semantics (hence `preserve_order`).

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::pycompat::{is_truthy, py_str, round_half_even};

/// semconv pinned at export, not at runtime.
pub const OTEL_GENAI_VERSION: &str = "1.30.0-draft";

fn records(exp_dir: &Path) -> Vec<Value> {
    let path = exp_dir.join("trials.jsonl");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// One trial → one OTel span: GenAI semantic-convention attributes plus
/// threshold fields. `None`-valued attributes are dropped, not emitted as null.
pub fn trial_span(record: &Value) -> Value {
    let get = |key: &str| record.get(key).cloned().unwrap_or(Value::Null);

    // `record.get("provider_served") or "openrouter"` — Python `or` falls
    // through on any falsy value, not just None.
    let system = match record.get("provider_served") {
        Some(v) if is_truthy(v) => v.clone(),
        _ => json!("openrouter"),
    };

    let mut attrs = Map::new();
    attrs.insert("gen_ai.system".into(), system);
    attrs.insert("gen_ai.request.model".into(), get("model"));
    attrs.insert("gen_ai.usage.input_tokens".into(), get("tokens_prompt"));
    attrs.insert(
        "gen_ai.usage.output_tokens".into(),
        get("tokens_completion"),
    );
    attrs.insert("gen_ai.usage.cost_usd".into(), get("cost_usd"));
    attrs.insert("threshold.candidate_id".into(), get("candidate_id"));
    attrs.insert("threshold.candidate_kind".into(), get("candidate_kind"));
    attrs.insert("threshold.composition_hash".into(), get("composition_hash"));
    attrs.insert("threshold.task_id".into(), get("task_id"));
    attrs.insert("threshold.trial".into(), get("trial"));
    attrs.insert("threshold.reward".into(), get("reward"));
    attrs.insert("threshold.false_positives".into(), get("false_positives"));
    attrs.insert("threshold.harness_version".into(), get("harness_version"));
    let attrs: Map<String, Value> = attrs.into_iter().filter(|(_, v)| !v.is_null()).collect();

    let name = format!(
        "{}/{}/t{}",
        py_str(&get("candidate_id")),
        py_str(&get("task_id")),
        py_str(&get("trial"))
    );
    let status = if record.get("error").map(is_truthy).unwrap_or(false) {
        "ERROR"
    } else {
        "OK"
    };

    let mut span = Map::new();
    span.insert("name".into(), json!(name));
    span.insert("span_id".into(), get("run_id"));
    span.insert("start_time".into(), get("ts_start"));
    span.insert("end_time".into(), get("ts_end"));
    span.insert("status".into(), json!(status));
    span.insert("status_message".into(), get("error"));
    span.insert("attributes".into(), Value::Object(attrs));
    Value::Object(span)
}

/// One experiment → one trace: an OTLP-ish dict ready for a sink adapter.
pub fn experiment_trace(exp_dir: &Path) -> Value {
    let recs = records(exp_dir);
    let spans: Vec<Value> = recs.iter().map(trial_span).collect();

    // sum(r.get("cost_usd") or 0): falsy costs contribute 0.
    let total_cost: f64 = recs
        .iter()
        .map(|r| match r.get("cost_usd") {
            Some(v) if is_truthy(v) => v.as_f64().unwrap_or(0.0),
            _ => 0.0,
        })
        .sum();

    // len(sorted({candidate_id})): distinct candidate ids.
    let mut seen = std::collections::HashSet::new();
    for r in &recs {
        seen.insert(
            r.get("candidate_id")
                .cloned()
                .unwrap_or(Value::Null)
                .to_string(),
        );
    }
    let candidate_count = seen.len();

    let name = exp_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let attributes = json!({
        "threshold.experiment": name,
        "threshold.candidate_count": candidate_count,
        "threshold.trial_count": recs.len(),
        "threshold.cost_usd_total": round_half_even(total_cost, 6),
    });

    let mut trace = Map::new();
    trace.insert(
        "schema".into(),
        json!(format!("otel-genai/{OTEL_GENAI_VERSION}")),
    );
    trace.insert("trace_id".into(), json!(name));
    trace.insert("name".into(), json!(format!("threshold experiment {name}")));
    trace.insert("attributes".into(), attributes);
    trace.insert("spans".into(), Value::Array(spans));
    Value::Object(trace)
}

/// Write `trace.otel.json` into the experiment dir; returns the path written.
pub fn write_trace(exp_dir: &Path) -> std::io::Result<PathBuf> {
    let trace = experiment_trace(exp_dir);
    let out = exp_dir.join("trace.otel.json");
    std::fs::write(&out, serde_json::to_string_pretty(&trace)?)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_span_carries_attrs() {
        let record = json!({
            "run_id": "r1", "candidate_id": "seed1", "task_id": "x", "trial": 1,
            "model": "z-ai/glm-5", "provider_served": "openrouter",
            "tokens_prompt": 100, "cost_usd": 0.01, "reward": 1.0, "error": null,
        });
        let span = trial_span(&record);
        assert_eq!(
            span["attributes"]["gen_ai.request.model"],
            json!("z-ai/glm-5")
        );
        assert_eq!(span["attributes"]["threshold.reward"], json!(1.0));
        assert_eq!(span["status"], json!("OK"));
        assert_eq!(span["name"], json!("seed1/x/t1"));
    }

    #[test]
    fn error_span_drops_null_attrs() {
        let record = json!({
            "run_id": "r", "candidate_id": "c", "task_id": "x", "trial": 1,
            "error": "pi exited 1", "cost_usd": null, "reward": 0.0,
        });
        let span = trial_span(&record);
        assert_eq!(span["status"], json!("ERROR"));
        assert_eq!(span["status_message"], json!("pi exited 1"));
        assert!(span["attributes"].get("gen_ai.usage.cost_usd").is_none());
    }
}
