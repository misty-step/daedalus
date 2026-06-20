use std::path::Path;

use serde_json::{json, Value};

use super::{
    request_digest, string_for_markdown, string_value, value_at, value_f64, value_opt_f64,
    value_u64, ArenaInfo, ImportOptions,
};

pub(super) struct SummaryArtifactPaths<'a> {
    pub(super) request: &'a Path,
    pub(super) artifact: &'a Path,
    pub(super) findings: &'a Path,
    pub(super) score: &'a Path,
    pub(super) report: &'a Path,
    pub(super) transcript: Option<&'a Path>,
    pub(super) receipt: Option<&'a Path>,
}

pub(super) fn build_summary(
    options: &ImportOptions,
    request: &Value,
    artifact: &Value,
    arena: &ArenaInfo,
    score: &Value,
    paths: SummaryArtifactPaths<'_>,
) -> Value {
    let run_id = options
        .out_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cerberus-lab-import");
    let cost = artifact
        .get("run")
        .and_then(|run| run.get("cost_usd"))
        .cloned()
        .unwrap_or(Value::Null);
    json!({
        "schema_version": "cerberus-lab-import.v1",
        "run_id": run_id,
        "candidate": {
            "candidate_id": options.candidate_id,
            "substrate": options.substrate,
            "model": options.model
        },
        "arena": {
            "path": options.arena.to_string_lossy(),
            "id": arena.id,
            "version": arena.version,
            "task_id": options.task_id
        },
        "request": {
            "path": paths.request.to_string_lossy(),
            "source_path": options.request.to_string_lossy(),
            "request_id": value_at(request, &["request_id"]),
            "request_digest": request_digest(request).unwrap_or_else(|err| format!("error:{err}")),
            "source_kind": value_at(request, &["source", "kind"])
        },
        "artifact": {
            "path": paths.artifact.to_string_lossy(),
            "source_path": options.artifact.to_string_lossy(),
            "artifact_id": value_at(artifact, &["artifact_id"]),
            "request_id": value_at(artifact, &["request_id"]),
            "lifecycle_state": value_at(artifact, &["lifecycle_state"]),
            "verdict": value_at(artifact, &["verdict"]),
            "context_capabilities": artifact.get("context_capabilities").cloned().unwrap_or(Value::Null),
            "valid": true
        },
        "run": {
            "engine_version": value_at(artifact, &["run", "engine_version"]),
            "duration_ms": value_at(artifact, &["run", "duration_ms"]),
            "cost_usd": cost
        },
        "score": score,
        "outputs": {
            "request": paths.request.to_string_lossy(),
            "artifact": paths.artifact.to_string_lossy(),
            "findings": paths.findings.to_string_lossy(),
            "score": paths.score.to_string_lossy(),
            "summary": options.out_dir.join("summary.json").to_string_lossy(),
            "report": paths.report.to_string_lossy(),
            "transcript": paths.transcript.map(|p| p.to_string_lossy().into_owned()),
            "receipt": paths.receipt.map(|p| p.to_string_lossy().into_owned())
        }
    })
}

pub(super) fn render_report(summary: &Value, artifact: &Value, score: &Value) -> String {
    let mut out = String::new();
    out.push_str("# Cerberus Lab Import Report\n\n");
    out.push_str("## Candidate\n\n");
    out.push_str(&format!(
        "- Candidate: `{}`\n- Substrate: `{}`\n- Model: `{}`\n\n",
        string_value(summary, &["candidate", "candidate_id"]),
        string_value(summary, &["candidate", "substrate"]),
        string_value(summary, &["candidate", "model"])
    ));
    out.push_str("## Artifact\n\n");
    out.push_str(&format!(
        "- Artifact: `{}`\n- Lifecycle: `{}`\n- Verdict: `{}`\n- Validation: passed\n\n",
        string_value(summary, &["artifact", "artifact_id"]),
        string_value(summary, &["artifact", "lifecycle_state"]),
        string_value(summary, &["artifact", "verdict"])
    ));
    out.push_str("## Score\n\n");
    if score.get("status").and_then(Value::as_str) == Some("scored") {
        let result = score.get("result").unwrap_or(&Value::Null);
        out.push_str(&format!(
            "- Task: `{}`\n- Reward: `{}`\n- Recall: `{}`\n- False positives: `{}`\n- Matched: `{}`\n\n",
            string_value(score, &["task_id"]),
            string_value(result, &["reward"]),
            string_value(result, &["recall"]),
            string_value(result, &["false_positives"]),
            string_value(result, &["matched"])
        ));
    } else {
        out.push_str("- Status: not scored; no `--task-id` supplied.\n\n");
    }
    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "{}\n\n{}\n\n",
        string_value(artifact, &["summary", "title"]),
        string_value(artifact, &["summary", "body"])
    ));
    let residual = artifact
        .get("summary")
        .and_then(|s| s.get("residual_risk"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !residual.is_empty() {
        out.push_str("## Residual Risk\n\n");
        for item in residual {
            out.push_str(&format!("- {}\n", string_for_markdown(&item)));
        }
        out.push('\n');
    }
    out.push_str("## Evidence\n\n");
    for key in [
        "request", "artifact", "findings", "score", "summary", "report",
    ] {
        out.push_str(&format!(
            "- {key}: `{}`\n",
            string_value(summary, &["outputs", key])
        ));
    }
    out
}

pub(super) struct ArtifactIndexPaths<'a> {
    pub(super) request: &'a Path,
    pub(super) artifact: &'a Path,
    pub(super) findings: &'a Path,
    pub(super) score: &'a Path,
    pub(super) summary: &'a Path,
    pub(super) report: &'a Path,
    pub(super) transcript: Option<&'a Path>,
    pub(super) receipt: Option<&'a Path>,
}

pub(super) fn render_artifact_index(paths: ArtifactIndexPaths<'_>) -> String {
    let mut lines = vec![
        format!("request {}", paths.request.display()),
        format!("artifact {}", paths.artifact.display()),
        format!("findings {}", paths.findings.display()),
        format!("score {}", paths.score.display()),
        format!("summary {}", paths.summary.display()),
        format!("report {}", paths.report.display()),
    ];
    if let Some(path) = paths.transcript {
        lines.push(format!("transcript {}", path.display()));
    }
    if let Some(path) = paths.receipt {
        lines.push(format!("receipt {}", path.display()));
    }
    lines.push(String::new());
    lines.join("\n")
}

pub(super) fn compare_candidate(run_dir: &Path, summary: &Value) -> Value {
    json!({
        "run_dir": run_dir.to_string_lossy(),
        "run_id": value_at(summary, &["run_id"]),
        "candidate_id": value_at(summary, &["candidate", "candidate_id"]),
        "substrate": value_at(summary, &["candidate", "substrate"]),
        "model": value_at(summary, &["candidate", "model"]),
        "artifact_valid": value_at(summary, &["artifact", "valid"]),
        "lifecycle_state": value_at(summary, &["artifact", "lifecycle_state"]),
        "verdict": value_at(summary, &["artifact", "verdict"]),
        "reward": value_at(summary, &["score", "result", "reward"]),
        "recall": value_at(summary, &["score", "result", "recall"]),
        "false_positives": value_at(summary, &["score", "result", "false_positives"]),
        "cost_usd": value_at(summary, &["run", "cost_usd"]),
        "duration_ms": value_at(summary, &["run", "duration_ms"]),
        "report": value_at(summary, &["outputs", "report"])
    })
}

pub(super) fn comparison_scope(candidates: &[Value]) -> &'static str {
    if candidates.iter().any(is_live_candidate) {
        "live_fixture_comparison"
    } else {
        "fixture_only"
    }
}

fn is_live_candidate(candidate: &Value) -> bool {
    if is_fixture_substrate(candidate) {
        return false;
    }
    let candidate_id = candidate
        .get("candidate_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let run_id = candidate
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let model_present = !candidate.get("model").unwrap_or(&Value::Null).is_null();
    model_present || has_live_component(candidate_id) || has_live_component(run_id)
}

fn has_live_component(value: &str) -> bool {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|component| component.eq_ignore_ascii_case("live"))
}

pub(super) fn best_live_candidate(candidates: &[Value]) -> Value {
    candidates
        .iter()
        .find(|candidate| !is_fixture_substrate(candidate) && is_live_candidate(candidate))
        .cloned()
        .unwrap_or(Value::Null)
}

pub(super) fn compare_candidate_order(left: &Value, right: &Value) -> std::cmp::Ordering {
    let left_valid = left
        .get("artifact_valid")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let right_valid = right
        .get("artifact_valid")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    right_valid
        .cmp(&left_valid)
        .then_with(|| {
            value_f64(right, "reward")
                .partial_cmp(&value_f64(left, "reward"))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| compare_optional_cost(left.get("cost_usd"), right.get("cost_usd")))
        .then_with(|| is_fixture_substrate(left).cmp(&is_fixture_substrate(right)))
        .then_with(|| value_u64(left, "duration_ms").cmp(&value_u64(right, "duration_ms")))
        .then_with(|| {
            string_value(left, &["candidate_id"]).cmp(&string_value(right, &["candidate_id"]))
        })
}

fn is_fixture_substrate(candidate: &Value) -> bool {
    candidate.get("substrate").and_then(Value::as_str) == Some("fixture")
}

fn compare_optional_cost(left: Option<&Value>, right: Option<&Value>) -> std::cmp::Ordering {
    match (value_opt_f64(left), value_opt_f64(right)) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub(super) fn render_comparison_report(comparison: &Value) -> String {
    let mut out = String::new();
    out.push_str("# Cerberus Lab Comparison Report\n\n");
    let scope = comparison
        .get("recommendation_scope")
        .and_then(Value::as_str)
        .unwrap_or("fixture_only");
    if scope == "live_fixture_comparison" {
        out.push_str("Scope: live imported Cerberus artifacts plus fixture reference. This is sandbox evidence, not a production default change.\n\n");
    } else {
        out.push_str("Scope: fixture-only imported Cerberus artifacts. This is sandbox evidence, not a production default change.\n\n");
    }
    out.push_str("| candidate | substrate | valid | lifecycle | verdict | reward | recall | false positives | cost | duration ms |\n");
    out.push_str("|---|---|---:|---|---|---:|---:|---:|---:|---:|\n");
    for candidate in comparison
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |\n",
            string_value(candidate, &["candidate_id"]),
            string_value(candidate, &["substrate"]),
            string_value(candidate, &["artifact_valid"]),
            string_value(candidate, &["lifecycle_state"]),
            string_value(candidate, &["verdict"]),
            string_value(candidate, &["reward"]),
            string_value(candidate, &["recall"]),
            string_value(candidate, &["false_positives"]),
            string_value(candidate, &["cost_usd"]),
            string_value(candidate, &["duration_ms"])
        ));
    }
    out.push_str("\n## Ordering\n\n");
    let best = comparison.get("best_candidate").unwrap_or(&Value::Null);
    if scope == "live_fixture_comparison" {
        let best_live = comparison
            .get("best_live_candidate")
            .unwrap_or(&Value::Null);
        out.push_str(&format!(
            "Best live substrate under this fixture objective: `{}` on `{}` with reward `{}` and cost `{}`. The fixture reference remains the oracle ceiling and is not a deployable substrate.\n\n",
            string_value(best_live, &["candidate_id"]),
            string_value(best_live, &["substrate"]),
            string_value(best_live, &["reward"]),
            string_value(best_live, &["cost_usd"])
        ));
        out.push_str(&format!(
            "`{}` is first under the sandbox ordering: valid artifacts first, reward descending, known lower cost, fixture references after live candidates on ties, then lower latency. Pi is not included because current Pi runs emit Daedalus candidate findings, not Cerberus `ReviewArtifact.v1` lifecycle receipts, so it is incomparable in this adapter proof.\n\n",
            string_value(best, &["candidate_id"])
        ));
    } else {
        out.push_str(&format!(
            "`{}` is first under the fixture-only ordering: valid artifacts first, reward descending, known lower cost, then lower latency. This is not a substrate recommendation; live Cerberus OpenCode/OMP runs and Pi comparability remain required before any sandbox recommendation.\n\n",
            string_value(best, &["candidate_id"])
        ));
    }
    out.push_str("## Evidence\n\n");
    for candidate in comparison
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.push_str(&format!(
            "- `{}`: `{}`\n",
            string_value(candidate, &["candidate_id"]),
            string_value(candidate, &["report"])
        ));
    }
    out
}
