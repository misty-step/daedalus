//! Cerberus R&D lab artifact import.
//!
//! This is a narrow adapter over Cerberus `ReviewRequest.v1` and
//! `ReviewArtifact.v1`. It validates the artifact boundary, maps findings into
//! Daedalus' existing review-finding shape, optionally scores against one arena
//! task, and leaves an inspectable evidence packet. It does not run models,
//! post comments, or mutate Cerberus defaults.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::score::{self, ScoreResult};

mod digest;
mod report;

use digest::{accepted_request_digests, request_digest, sha256_digest};
use report::{
    best_live_candidate, build_summary, compare_candidate, compare_candidate_order,
    comparison_scope, render_artifact_index, render_comparison_report, render_report,
    ArtifactIndexPaths, SummaryArtifactPaths,
};

const REQUEST_SCHEMA: &str = "cerberus.review_request.v1";
const ARTIFACT_SCHEMA: &str = "cerberus.review_artifact.v1";

#[derive(Debug)]
pub struct CerberusLabError(pub String);

impl std::fmt::Display for CerberusLabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CerberusLabError {}

#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub arena: PathBuf,
    pub request: PathBuf,
    pub artifact: PathBuf,
    pub candidate_id: String,
    pub substrate: String,
    pub model: Option<String>,
    pub task_id: Option<String>,
    pub transcript: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
    pub out_dir: PathBuf,
    pub repo_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub out_dir: PathBuf,
    pub report: PathBuf,
    pub summary: PathBuf,
    pub findings: PathBuf,
    pub score: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CompareOptions {
    pub run_dirs: Vec<PathBuf>,
    pub out_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CompareResult {
    pub out_dir: PathBuf,
    pub summary: PathBuf,
    pub report: PathBuf,
}

pub fn import_review_artifact(
    options: &ImportOptions,
) -> Result<ImportResult, Box<dyn std::error::Error>> {
    validate_import_options(options)?;

    let request = load_json(&options.request)?;
    let artifact = load_json(&options.artifact)?;
    validate_request(&request)?;
    validate_artifact_for_request(&artifact, &request)?;

    if let Some(model) = &options.model {
        validate_model_in_pool(model, &options.repo_root)?;
    }

    let findings_value = cerberus_findings_to_daedalus(&artifact)?;
    let arena_info = load_arena_info(&options.arena)?;
    let score_result = score_if_requested(options, &findings_value)?;

    std::fs::create_dir_all(&options.out_dir)?;
    let request_out = options.out_dir.join("request.json");
    let artifact_out = options.out_dir.join("artifact.json");
    let findings_out = options.out_dir.join("findings.json");
    let score_out = options.out_dir.join("score.json");
    let summary_out = options.out_dir.join("summary.json");
    let report_out = options.out_dir.join("report.md");
    let index_out = options.out_dir.join("artifacts.index");

    write_json(&request_out, &request)?;
    write_json(&artifact_out, &artifact)?;
    write_json(&findings_out, &findings_value)?;

    let score_record = match &score_result {
        Some(score) => json!({
            "status": "scored",
            "task_id": options.task_id.as_deref().unwrap(),
            "result": score
        }),
        None => json!({
            "status": "not_scored",
            "reason": "no --task-id supplied"
        }),
    };
    write_json(&score_out, &score_record)?;

    let copied_transcript = copy_optional_artifact(&options.transcript, &options.out_dir)?;
    let copied_receipt = copy_optional_artifact(&options.receipt, &options.out_dir)?;

    let summary = build_summary(
        options,
        &request,
        &artifact,
        &arena_info,
        &score_record,
        SummaryArtifactPaths {
            request: &request_out,
            artifact: &artifact_out,
            findings: &findings_out,
            score: &score_out,
            report: &report_out,
            transcript: copied_transcript.as_deref(),
            receipt: copied_receipt.as_deref(),
        },
    );
    write_json(&summary_out, &summary)?;
    std::fs::write(
        &report_out,
        render_report(&summary, &artifact, &score_record),
    )?;
    std::fs::write(
        &index_out,
        render_artifact_index(ArtifactIndexPaths {
            request: &request_out,
            artifact: &artifact_out,
            findings: &findings_out,
            score: &score_out,
            summary: &summary_out,
            report: &report_out,
            transcript: copied_transcript.as_deref(),
            receipt: copied_receipt.as_deref(),
        }),
    )?;

    Ok(ImportResult {
        out_dir: options.out_dir.clone(),
        report: report_out,
        summary: summary_out,
        findings: findings_out,
        score: score_out,
    })
}

pub fn compare_imports(
    options: &CompareOptions,
) -> Result<CompareResult, Box<dyn std::error::Error>> {
    if options.run_dirs.len() < 2 {
        return Err(
            CerberusLabError("at least two --run-dir values are required".to_string()).into(),
        );
    }

    let mut candidates = Vec::new();
    for run_dir in &options.run_dirs {
        let summary_path = run_dir.join("summary.json");
        let summary = load_json(&summary_path)?;
        if summary.get("schema_version").and_then(Value::as_str) != Some("cerberus-lab-import.v1") {
            return Err(CerberusLabError(format!(
                "{} is not a cerberus-lab import summary",
                summary_path.display()
            ))
            .into());
        }
        candidates.push(compare_candidate(run_dir, &summary));
    }

    candidates.sort_by(compare_candidate_order);
    let recommendation_scope = comparison_scope(&candidates);
    std::fs::create_dir_all(&options.out_dir)?;
    let summary_path = options.out_dir.join("summary.json");
    let report_path = options.out_dir.join("report.md");
    let comparison = json!({
        "schema_version": "cerberus-lab-comparison.v1",
        "candidate_count": candidates.len(),
        "recommendation_scope": recommendation_scope,
        "best_candidate": candidates.first().cloned().unwrap_or(Value::Null),
        "best_live_candidate": best_live_candidate(&candidates),
        "candidates": candidates,
        "outputs": {
            "summary": summary_path.to_string_lossy(),
            "report": report_path.to_string_lossy()
        }
    });
    write_json(&summary_path, &comparison)?;
    std::fs::write(&report_path, render_comparison_report(&comparison))?;

    Ok(CompareResult {
        out_dir: options.out_dir.clone(),
        summary: summary_path,
        report: report_path,
    })
}

fn validate_import_options(options: &ImportOptions) -> Result<(), CerberusLabError> {
    if options.candidate_id.trim().is_empty() {
        return Err(CerberusLabError("--candidate-id is required".to_string()));
    }
    if options.substrate.trim().is_empty() {
        return Err(CerberusLabError("--substrate is required".to_string()));
    }
    if !options.arena.join("arena.toml").is_file() {
        return Err(CerberusLabError(format!(
            "arena.toml not found under {}",
            options.arena.display()
        )));
    }
    Ok(())
}

fn load_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| CerberusLabError(format!("failed to read {}: {err}", path.display())))?;
    serde_json::from_str(&text).map_err(|err| {
        CerberusLabError(format!("failed to parse {}: {err}", path.display())).into()
    })
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    std::fs::write(path, text)?;
    Ok(())
}

fn validate_request(request: &Value) -> Result<(), CerberusLabError> {
    require_string(request, &["schema_version"], "request schema_version").and_then(|schema| {
        if schema == REQUEST_SCHEMA {
            Ok(())
        } else {
            Err(CerberusLabError(format!(
                "unsupported request schema: {schema}"
            )))
        }
    })?;
    require_nonempty_string(request, &["request_id"], "request_id")?;
    require_nonempty_string(request, &["change", "title"], "change.title")?;
    require_nonempty_string(request, &["change", "diff", "body"], "change.diff.body")?;
    let format = optional_string(request, &["change", "diff", "format"]).unwrap_or("unified");
    if format != "unified" {
        return Err(CerberusLabError(format!(
            "unsupported diff format: {format}"
        )));
    }
    if let Some(actual) = optional_string(request, &["change", "diff", "digest"]) {
        let body = require_string(request, &["change", "diff", "body"], "change.diff.body")?;
        let expected = sha256_digest(body.as_bytes());
        if actual != expected {
            return Err(CerberusLabError(format!(
                "diff digest mismatch: expected {expected}, got {actual}"
            )));
        }
    }
    Ok(())
}

fn validate_artifact_for_request(
    artifact: &Value,
    request: &Value,
) -> Result<(), CerberusLabError> {
    let schema = require_string(artifact, &["schema_version"], "artifact schema_version")?;
    if schema != ARTIFACT_SCHEMA {
        return Err(CerberusLabError(format!(
            "unsupported artifact schema: {schema}"
        )));
    }

    let request_id = require_string(request, &["request_id"], "request_id")?;
    let artifact_request_id = require_string(artifact, &["request_id"], "artifact.request_id")?;
    if request_id != artifact_request_id {
        return Err(CerberusLabError(format!(
            "artifact request id mismatch: expected {request_id}, got {artifact_request_id}"
        )));
    }

    let expected_digests = accepted_request_digests(request)?;
    let actual_digest = require_string(artifact, &["request_digest"], "artifact.request_digest")?;
    if !expected_digests
        .iter()
        .any(|digest| digest == actual_digest)
    {
        return Err(CerberusLabError(format!(
            "artifact request digest mismatch: expected one of {}, got {actual_digest}",
            expected_digests.join(", ")
        )));
    }

    let expected_capabilities = context_capabilities_from_request(request);
    let actual_capabilities = artifact
        .get("context_capabilities")
        .ok_or_else(|| CerberusLabError("artifact context_capabilities is required".to_string()))?;
    if actual_capabilities != &expected_capabilities {
        return Err(CerberusLabError(
            "artifact context capabilities overstate the request".to_string(),
        ));
    }

    validate_artifact_references(artifact, request)
}

fn validate_artifact_references(artifact: &Value, request: &Value) -> Result<(), CerberusLabError> {
    let changed_paths = changed_paths(request);
    let findings = array_or_empty(artifact, "findings");
    let comments = array_or_empty(artifact, "comments");
    let citations = array_or_empty(artifact, "citations");
    let suggested_fixes = array_or_empty(artifact, "suggested_fixes");

    let finding_ids: HashSet<String> = findings
        .iter()
        .filter_map(|finding| optional_string(finding, &["id"]).map(str::to_string))
        .collect();
    let citation_ids: HashSet<String> = citations
        .iter()
        .filter_map(|citation| optional_string(citation, &["id"]).map(str::to_string))
        .collect();
    let fix_ids: HashSet<String> = suggested_fixes
        .iter()
        .filter_map(|fix| optional_string(fix, &["id"]).map(str::to_string))
        .collect();

    let mut attached_fixes: HashSet<String> = HashSet::new();
    for finding in findings {
        let finding_id = require_string(finding, &["id"], "finding.id")?;
        let anchors = array_or_empty(finding, "anchors");
        if anchors.is_empty() {
            return Err(CerberusLabError(format!(
                "finding is missing an evidence anchor: {finding_id}"
            )));
        }
        for anchor in anchors {
            validate_anchor_path(anchor, &changed_paths)?;
        }
        for citation_id in string_array(finding, "citations") {
            if !citation_ids.contains(&citation_id) {
                return Err(CerberusLabError(format!(
                    "finding references unknown citation id: {citation_id}"
                )));
            }
        }
        for fix_id in string_array(finding, "suggested_fixes") {
            if !fix_ids.contains(&fix_id) {
                return Err(CerberusLabError(format!(
                    "finding references unknown suggested fix id: {fix_id}"
                )));
            }
            attached_fixes.insert(fix_id);
        }
    }

    for comment in comments {
        let comment_id = require_string(comment, &["id"], "comment.id")?;
        if let Some(finding_id) = optional_string(comment, &["finding_id"]) {
            if !finding_ids.contains(finding_id) {
                return Err(CerberusLabError(format!(
                    "comment {comment_id} references unknown finding id {finding_id}"
                )));
            }
        }
        if let Some(anchor) = comment.get("anchor") {
            validate_anchor_path(anchor, &changed_paths)?;
        }
        for fix_id in string_array(comment, "suggested_fixes") {
            if !fix_ids.contains(&fix_id) {
                return Err(CerberusLabError(format!(
                    "comment references unknown suggested fix id: {fix_id}"
                )));
            }
            attached_fixes.insert(fix_id);
        }
    }

    for citation in citations {
        let citation_id = require_string(citation, &["id"], "citation.id")?;
        for finding_id in string_array(citation, "used_by") {
            if !finding_ids.contains(&finding_id) {
                return Err(CerberusLabError(format!(
                    "citation {citation_id} references unknown finding id {finding_id}"
                )));
            }
        }
    }

    for fix in suggested_fixes {
        let fix_id = require_string(fix, &["id"], "suggested_fix.id")?;
        if let Some(finding_id) = optional_string(fix, &["finding_id"]) {
            if !finding_ids.contains(finding_id) {
                return Err(CerberusLabError(format!(
                    "suggested fix {fix_id} references unknown finding id {finding_id}"
                )));
            }
            attached_fixes.insert(fix_id.to_string());
        }
        if !attached_fixes.contains(fix_id) {
            return Err(CerberusLabError(format!(
                "top-level suggested fix is not attached to any finding or comment: {fix_id}"
            )));
        }
    }

    Ok(())
}

fn validate_anchor_path(
    anchor: &Value,
    changed_paths: &HashSet<String>,
) -> Result<(), CerberusLabError> {
    let Some(kind) = optional_string(anchor, &["kind"]) else {
        return Ok(());
    };
    if !matches!(kind, "inline" | "file") {
        return Ok(());
    }
    let Some(path) = optional_string(anchor, &["path"]) else {
        return Err(CerberusLabError(format!("{kind} anchor path is missing")));
    };
    if !changed_paths.contains(path) {
        return Err(CerberusLabError(format!(
            "{kind} anchor points outside changed files: {path}"
        )));
    }
    Ok(())
}

fn cerberus_findings_to_daedalus(artifact: &Value) -> Result<Value, CerberusLabError> {
    let mut mapped = Vec::new();
    for finding in array_or_empty(artifact, "findings") {
        let id = require_string(finding, &["id"], "finding.id")?;
        let category = require_string(finding, &["category"], "finding.category")?;
        let description =
            require_string(finding, &["description"], "finding.description")?.to_string();
        let title = optional_string(finding, &["title"]).unwrap_or(id);
        let evidence = optional_string(finding, &["evidence"]).unwrap_or("");
        let (file, line) = finding_location(finding).ok_or_else(|| {
            CerberusLabError(format!(
                "finding {id} has no inline/file anchor with path and line"
            ))
        })?;
        mapped.push(json!({
            "file": file,
            "line": line,
            "category": category,
            "description": format!("{title}: {description}\nEvidence: {evidence}"),
            "severity": daedalus_severity(optional_string(finding, &["severity"]).unwrap_or(""))
        }));
    }
    Ok(json!({ "findings": mapped }))
}

fn finding_location(finding: &Value) -> Option<(String, u64)> {
    for anchor in array_or_empty(finding, "anchors") {
        let kind = optional_string(anchor, &["kind"])?;
        if !matches!(kind, "inline" | "file") {
            continue;
        }
        let path = optional_string(anchor, &["path"])?;
        let line = anchor
            .get("line")
            .or_else(|| anchor.get("start_line"))
            .and_then(Value::as_u64)?;
        return Some((path.to_string(), line));
    }
    None
}

fn daedalus_severity(cerberus_severity: &str) -> &'static str {
    match cerberus_severity {
        "critical" => "blocking",
        "major" => "serious",
        "minor" | "info" => "minor",
        _ => "minor",
    }
}

fn score_if_requested(
    options: &ImportOptions,
    findings_value: &Value,
) -> Result<Option<ScoreResult>, Box<dyn std::error::Error>> {
    let Some(task_id) = options.task_id.as_deref() else {
        return Ok(None);
    };
    let task_dir = options.arena.join("tasks").join(task_id);
    let expected = task_dir.join("tests/expected.json");
    if !expected.is_file() {
        return Err(CerberusLabError(format!(
            "expected answer key not found for task {task_id}: {}",
            expected.display()
        ))
        .into());
    }

    let tmp = std::env::temp_dir().join(format!(
        "daedalus-cerberus-lab-score-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp)?;
    let findings = tmp.join("findings.json");
    write_json(&findings, findings_value)?;
    let result = score::score(&findings, &expected)?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(Some(result))
}

#[derive(Debug, Clone)]
struct ArenaInfo {
    id: String,
    version: String,
}

fn load_arena_info(arena: &Path) -> Result<ArenaInfo, Box<dyn std::error::Error>> {
    let path = arena.join("arena.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|err| CerberusLabError(format!("failed to read {}: {err}", path.display())))?;
    let parsed: toml::Value = toml::from_str(&text)
        .map_err(|err| CerberusLabError(format!("failed to parse {}: {err}", path.display())))?;
    Ok(ArenaInfo {
        id: parsed
            .get("id")
            .and_then(toml::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        version: parsed
            .get("version")
            .and_then(toml::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
    })
}

fn validate_model_in_pool(model: &str, repo_root: &Path) -> Result<(), CerberusLabError> {
    let primitives =
        std::fs::read_to_string(repo_root.join("docs/primitives.md")).map_err(|err| {
            CerberusLabError(format!(
                "failed to read docs/primitives.md for model validation: {err}"
            ))
        })?;
    let needle = format!("`{model}`");
    if primitives.contains(&needle) {
        Ok(())
    } else {
        Err(CerberusLabError(format!(
            "model {model} is not present in docs/primitives.md"
        )))
    }
}

fn copy_optional_artifact(
    source: &Option<PathBuf>,
    out_dir: &Path,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let Some(source) = source else {
        return Ok(None);
    };
    let file_name = source
        .file_name()
        .ok_or_else(|| CerberusLabError(format!("path has no file name: {}", source.display())))?;
    let out = out_dir.join(file_name);
    std::fs::copy(source, &out)?;
    Ok(Some(out))
}

fn context_capabilities_from_request(request: &Value) -> Value {
    let diff = optional_string(request, &["change", "diff", "body"])
        .map(|body| !body.trim().is_empty())
        .unwrap_or(false);
    let context = request.get("context").unwrap_or(&Value::Null);
    let workspaces = context.get("workspaces").unwrap_or(&Value::Null);
    let local_runtime = context
        .get("local_runtime")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let remote_runtime = context
        .get("remote_runtime")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    json!({
        "diff": diff,
        "repo_head": workspaces.get("head").is_some(),
        "repo_base": workspaces.get("base").is_some(),
        "local_runtime": local_runtime,
        "remote_runtime": remote_runtime,
        "external_research": optional_string(request, &["policy", "external_research"]).unwrap_or("forbid")
    })
}

fn changed_paths(request: &Value) -> HashSet<String> {
    request
        .get("change")
        .and_then(|change| change.get("files"))
        .and_then(Value::as_array)
        .map(|files| {
            files
                .iter()
                .filter_map(|file| optional_string(file, &["path"]).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn require_nonempty_string<'a>(
    value: &'a Value,
    path: &[&str],
    label: &str,
) -> Result<&'a str, CerberusLabError> {
    let text = require_string(value, path, label)?;
    if text.trim().is_empty() {
        Err(CerberusLabError(format!("{label} is required")))
    } else {
        Ok(text)
    }
}

fn require_string<'a>(
    value: &'a Value,
    path: &[&str],
    label: &str,
) -> Result<&'a str, CerberusLabError> {
    optional_string(value, path).ok_or_else(|| CerberusLabError(format!("{label} is required")))
}

fn optional_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn value_at(value: &Value, path: &[&str]) -> Value {
    let mut current = value;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return Value::Null;
        };
        current = next;
    }
    current.clone()
}

fn array_or_empty<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn string_value(value: &Value, path: &[&str]) -> String {
    let value = value_at(value, path);
    string_for_markdown(&value)
}

fn string_for_markdown(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn value_f64(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(f64::MIN)
}

fn value_opt_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.parse().ok(),
        _ => None,
    }
}

fn value_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn imports_and_scores_valid_cerberus_fixture() {
        let root = tempdir("cerberus-lab-valid");
        let arena = write_arena(&root);
        let request = sample_request();
        let artifact = sample_artifact(&request, "completed", "WARN");
        let request_path = root.join("request.json");
        let artifact_path = root.join("artifact.json");
        write_json(&request_path, &request).unwrap();
        write_json(&artifact_path, &artifact).unwrap();

        let result = import_review_artifact(&ImportOptions {
            arena,
            request: request_path.clone(),
            artifact: artifact_path.clone(),
            candidate_id: "fixture-self-review".to_string(),
            substrate: "fixture".to_string(),
            model: None,
            task_id: Some("ratio-zero".to_string()),
            transcript: None,
            receipt: None,
            out_dir: root.join("runs/cerberus-lab"),
            repo_root: root.clone(),
        })
        .unwrap();

        let summary = load_json(&result.summary).unwrap();
        assert_eq!(summary["artifact"]["valid"], true);
        assert_eq!(summary["score"]["result"]["reward"], 1.0);
        assert!(result.report.is_file());
        assert!(result.findings.is_file());
        assert!(result.out_dir.join("artifacts.index").is_file());
        let report = std::fs::read_to_string(result.report).unwrap();
        assert!(report.contains("Validation: passed"));

        let other = import_review_artifact(&ImportOptions {
            arena: root.join("arenas/ratio"),
            request: request_path,
            artifact: artifact_path,
            candidate_id: "opencode-fixture-review".to_string(),
            substrate: "opencode".to_string(),
            model: None,
            task_id: Some("ratio-zero".to_string()),
            transcript: None,
            receipt: None,
            out_dir: root.join("runs/cerberus-lab-opencode"),
            repo_root: root.clone(),
        })
        .unwrap();
        let comparison = compare_imports(&CompareOptions {
            run_dirs: vec![result.out_dir, other.out_dir],
            out_dir: root.join("runs/cerberus-lab-comparison"),
        })
        .unwrap();
        let comparison_report = std::fs::read_to_string(comparison.report).unwrap();
        assert!(comparison_report.contains("fixture-only imported Cerberus artifacts"));
        assert!(comparison.summary.is_file());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_wrong_request_digest() {
        let root = tempdir("cerberus-lab-digest");
        let arena = write_arena(&root);
        let request = sample_request();
        let mut artifact = sample_artifact(&request, "completed", "WARN");
        artifact["request_digest"] = json!("sha256:bad");
        let request_path = root.join("request.json");
        let artifact_path = root.join("artifact.json");
        write_json(&request_path, &request).unwrap();
        write_json(&artifact_path, &artifact).unwrap();

        let err = import_review_artifact(&ImportOptions {
            arena,
            request: request_path,
            artifact: artifact_path,
            candidate_id: "fixture-self-review".to_string(),
            substrate: "fixture".to_string(),
            model: None,
            task_id: Some("ratio-zero".to_string()),
            transcript: None,
            receipt: None,
            out_dir: root.join("runs/cerberus-lab"),
            repo_root: root.clone(),
        })
        .unwrap_err()
        .to_string();

        assert!(err.contains("artifact request digest mismatch"), "{err}");
        assert!(!root.join("runs/cerberus-lab/report.md").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_legacy_request_digest() {
        let root = tempdir("cerberus-lab-legacy-digest");
        let arena = write_arena(&root);
        let request = sample_request();
        let mut artifact = sample_artifact(&request, "completed", "WARN");
        artifact["request_digest"] = json!(digest::legacy_request_digest(&request).unwrap());
        let request_path = root.join("request.json");
        let artifact_path = root.join("artifact.json");
        write_json(&request_path, &request).unwrap();
        write_json(&artifact_path, &artifact).unwrap();

        let result = import_review_artifact(&ImportOptions {
            arena,
            request: request_path,
            artifact: artifact_path,
            candidate_id: "fixture-self-review".to_string(),
            substrate: "fixture".to_string(),
            model: None,
            task_id: Some("ratio-zero".to_string()),
            transcript: None,
            receipt: None,
            out_dir: root.join("runs/cerberus-lab"),
            repo_root: root.clone(),
        })
        .unwrap();

        assert!(result.summary.is_file());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_context_overclaim() {
        let root = tempdir("cerberus-lab-capability");
        let arena = write_arena(&root);
        let request = sample_request();
        let mut artifact = sample_artifact(&request, "completed", "WARN");
        artifact["context_capabilities"]["repo_head"] = json!(true);
        let request_path = root.join("request.json");
        let artifact_path = root.join("artifact.json");
        write_json(&request_path, &request).unwrap();
        write_json(&artifact_path, &artifact).unwrap();

        let err = import_review_artifact(&ImportOptions {
            arena,
            request: request_path,
            artifact: artifact_path,
            candidate_id: "fixture-self-review".to_string(),
            substrate: "fixture".to_string(),
            model: None,
            task_id: None,
            transcript: None,
            receipt: None,
            out_dir: root.join("runs/cerberus-lab"),
            repo_root: root.clone(),
        })
        .unwrap_err()
        .to_string();

        assert!(
            err.contains("context capabilities overstate the request"),
            "{err}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn live_comparison_report_marks_pi_incomparable() {
        let comparison = json!({
            "recommendation_scope": "live_fixture_comparison",
            "best_candidate": {"candidate_id": "opencode-live-review"},
            "best_live_candidate": {
                "candidate_id": "opencode-live-review",
                "substrate": "opencode",
                "reward": 0.8,
                "cost_usd": null
            },
            "candidates": [
                {
                    "candidate_id": "opencode-live-review",
                    "substrate": "opencode",
                    "artifact_valid": true,
                    "lifecycle_state": "completed",
                    "verdict": "WARN",
                    "reward": 0.8,
                    "recall": 1.0,
                    "false_positives": 1,
                    "cost_usd": null,
                    "duration_ms": 0,
                    "report": "runs/cerberus-rd-lab-live-opencode/report.md"
                }
            ]
        });

        let report = render_comparison_report(&comparison);

        assert!(report.contains("live imported Cerberus artifacts"));
        assert!(report.contains("Pi is not included"));
    }

    fn sample_request() -> Value {
        json!({
            "schema_version": REQUEST_SCHEMA,
            "request_id": "fixture-diff-only-001",
            "source": {"kind": "fixture", "external_id": "fixture-001", "repo": "example/fixture", "metadata": {}},
            "change": {
                "title": "Avoid divide by zero in ratio helper",
                "diff": {"format": "unified", "body": "@@ -1,3 +1,6 @@\n fn ratio(numerator: f64, denominator: f64) -> f64 {\n+    if denominator == 0.0 {\n+        return 0.0;\n+    }\n     numerator / denominator\n }\n"},
                "files": [{"path": "src/ratio.rs", "status": "modified", "additions": 3, "deletions": 0}]
            },
            "context": {},
            "policy": {"allow_degraded": true, "timeout_ms": 120000, "external_research": "forbid", "render_targets": ["json"], "allowed_env": []}
        })
    }

    fn sample_artifact(request: &Value, lifecycle_state: &str, verdict: &str) -> Value {
        json!({
            "schema_version": ARTIFACT_SCHEMA,
            "artifact_id": "artifact-test",
            "request_id": request["request_id"],
            "request_digest": request_digest(request).unwrap(),
            "lifecycle_state": lifecycle_state,
            "verdict": verdict,
            "context_capabilities": context_capabilities_from_request(request),
            "summary": {
                "title": "Diff-only review found one behavioral concern",
                "body": "The guard avoids division by zero, but returning 0.0 silently changes the mathematical meaning.",
                "analysis": "Fixture evidence only.",
                "residual_risk": ["No surrounding call sites were available."]
            },
            "findings": [{
                "id": "finding-001",
                "severity": "major",
                "category": "correctness",
                "title": "Silent zero return may mask invalid denominator",
                "description": "The new branch returns 0.0 for every zero denominator.",
                "evidence": "The changed hunk adds a zero denominator branch.",
                "confidence": 0.76,
                "anchors": [{"kind": "inline", "path": "src/ratio.rs", "line": 3}],
                "citations": [],
                "suggested_fixes": ["fix-001"]
            }],
            "comments": [],
            "suggested_fixes": [{
                "id": "fix-001",
                "finding_id": "finding-001",
                "applicability": "needs_review",
                "format": "instructions",
                "edits": [],
                "diff": null
            }],
            "citations": [],
            "receipts": [{"id": "receipt-master", "role": "master", "harness": "fixture", "status": "completed", "verdict": verdict, "summary": "fixture"}],
            "run": {
                "engine_version": "cerberus-fixture",
                "config_digest": "sha256:fixture",
                "started_at": "0",
                "finished_at": "0",
                "duration_ms": 1,
                "cost_usd": null,
                "coverage": {"files_reviewed": ["src/ratio.rs"], "files_with_findings": ["src/ratio.rs"]}
            },
            "errors": []
        })
    }

    fn write_arena(root: &Path) -> PathBuf {
        let arena = root.join("arenas/ratio");
        std::fs::create_dir_all(arena.join("tasks/ratio-zero/tests")).unwrap();
        std::fs::write(
            arena.join("arena.toml"),
            "id = \"ratio\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            arena.join("tasks/ratio-zero/tests/expected.json"),
            serde_json::to_string_pretty(&json!({
                "defects": [{
                    "id": "ratio-zero",
                    "file": "src/ratio.rs",
                    "line_start": 1,
                    "line_end": 5,
                    "category": "correctness",
                    "severity": "serious",
                    "note": "Zero denominator should not silently return a plausible ratio."
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        arena
    }

    fn tempdir(prefix: &str) -> PathBuf {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("{prefix}-{}-{nanos}-{counter}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
