use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::{value_at, CerberusLabError};

pub(super) fn request_digest(request: &Value) -> Result<String, CerberusLabError> {
    current_request_digest(request)
}

pub(super) fn accepted_request_digests(request: &Value) -> Result<Vec<String>, CerberusLabError> {
    let mut digests = vec![current_request_digest(request)?];
    let legacy = legacy_request_digest(request)?;
    if !digests.contains(&legacy) {
        digests.push(legacy);
    }
    Ok(digests)
}

fn current_request_digest(request: &Value) -> Result<String, CerberusLabError> {
    let canonical = canonical_request_for_digest(request, RequestDigestVersion::Current);
    serde_json::to_vec(&canonical)
        .map(sha256_digest)
        .map_err(|err| CerberusLabError(format!("request is not serializable: {err}")))
}

pub(super) fn legacy_request_digest(request: &Value) -> Result<String, CerberusLabError> {
    let canonical = canonical_request_for_digest(request, RequestDigestVersion::Legacy);
    serde_json::to_vec(&canonical)
        .map(sha256_digest)
        .map_err(|err| CerberusLabError(format!("request is not serializable: {err}")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestDigestVersion {
    Current,
    Legacy,
}

fn canonical_request_for_digest(request: &Value, version: RequestDigestVersion) -> Value {
    let mut out = Map::new();
    out.insert(
        "schema_version".to_string(),
        value_at(request, &["schema_version"]),
    );
    out.insert("request_id".to_string(), value_at(request, &["request_id"]));
    out.insert(
        "source".to_string(),
        canonical_source(request.get("source")),
    );
    out.insert(
        "change".to_string(),
        canonical_change(request.get("change")),
    );
    out.insert(
        "context".to_string(),
        canonical_context(request.get("context")),
    );
    out.insert(
        "policy".to_string(),
        canonical_policy(request.get("policy"), version),
    );
    Value::Object(out)
}

fn canonical_source(source: Option<&Value>) -> Value {
    let source = source.unwrap_or(&Value::Null);
    let mut out = Map::new();
    out.insert("kind".to_string(), value_at(source, &["kind"]));
    insert_if_present(&mut out, source, "external_id");
    insert_if_present(&mut out, source, "repo");
    insert_if_present(&mut out, source, "uri");
    out.insert(
        "metadata".to_string(),
        source.get("metadata").cloned().unwrap_or(Value::Null),
    );
    Value::Object(out)
}

fn canonical_change(change: Option<&Value>) -> Value {
    let change = change.unwrap_or(&Value::Null);
    let mut out = Map::new();
    out.insert("title".to_string(), value_at(change, &["title"]));
    insert_if_present(&mut out, change, "description");
    insert_if_present(&mut out, change, "base_ref");
    insert_if_present(&mut out, change, "head_ref");
    insert_if_present(&mut out, change, "head_sha");
    out.insert("diff".to_string(), canonical_diff(change.get("diff")));
    out.insert(
        "files".to_string(),
        Value::Array(
            change
                .get("files")
                .and_then(Value::as_array)
                .map(|files| files.iter().map(canonical_changed_file).collect())
                .unwrap_or_default(),
        ),
    );
    Value::Object(out)
}

fn canonical_diff(diff: Option<&Value>) -> Value {
    let diff = diff.unwrap_or(&Value::Null);
    let mut out = Map::new();
    out.insert(
        "format".to_string(),
        diff.get("format")
            .cloned()
            .unwrap_or_else(|| json!("unified")),
    );
    out.insert("body".to_string(), value_at(diff, &["body"]));
    insert_if_present(&mut out, diff, "digest");
    Value::Object(out)
}

fn canonical_changed_file(file: &Value) -> Value {
    let mut out = Map::new();
    out.insert("path".to_string(), value_at(file, &["path"]));
    out.insert("status".to_string(), value_at(file, &["status"]));
    insert_if_present(&mut out, file, "old_path");
    insert_if_present(&mut out, file, "additions");
    insert_if_present(&mut out, file, "deletions");
    Value::Object(out)
}

fn canonical_context(context: Option<&Value>) -> Value {
    let context = context.unwrap_or(&Value::Null);
    let mut out = Map::new();
    insert_if_present(&mut out, context, "summary");
    out.insert(
        "acceptance".to_string(),
        context
            .get("acceptance")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    out.insert(
        "instructions".to_string(),
        context
            .get("instructions")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    out.insert(
        "artifacts".to_string(),
        Value::Array(
            context
                .get("artifacts")
                .and_then(Value::as_array)
                .map(|artifacts| artifacts.iter().map(canonical_context_artifact).collect())
                .unwrap_or_default(),
        ),
    );
    out.insert(
        "workspaces".to_string(),
        canonical_workspaces(context.get("workspaces")),
    );
    out.insert(
        "local_runtime".to_string(),
        Value::Array(
            context
                .get("local_runtime")
                .and_then(Value::as_array)
                .map(|targets| targets.iter().map(canonical_runtime_target).collect())
                .unwrap_or_default(),
        ),
    );
    out.insert(
        "remote_runtime".to_string(),
        Value::Array(
            context
                .get("remote_runtime")
                .and_then(Value::as_array)
                .map(|targets| targets.iter().map(canonical_remote_target).collect())
                .unwrap_or_default(),
        ),
    );
    out.insert(
        "metadata".to_string(),
        context.get("metadata").cloned().unwrap_or(Value::Null),
    );
    Value::Object(out)
}

fn canonical_context_artifact(artifact: &Value) -> Value {
    let mut out = Map::new();
    out.insert("kind".to_string(), value_at(artifact, &["kind"]));
    out.insert("uri".to_string(), value_at(artifact, &["uri"]));
    insert_if_present(&mut out, artifact, "digest");
    Value::Object(out)
}

fn canonical_workspaces(workspaces: Option<&Value>) -> Value {
    let workspaces = workspaces.unwrap_or(&Value::Null);
    let mut out = Map::new();
    if let Some(head) = workspaces.get("head") {
        out.insert("head".to_string(), canonical_workspace_ref(head));
    }
    if let Some(base) = workspaces.get("base") {
        out.insert("base".to_string(), canonical_workspace_ref(base));
    }
    Value::Object(out)
}

fn canonical_workspace_ref(workspace: &Value) -> Value {
    let mut out = Map::new();
    out.insert("kind".to_string(), value_at(workspace, &["kind"]));
    out.insert("path".to_string(), value_at(workspace, &["path"]));
    insert_if_present(&mut out, workspace, "ref_name");
    insert_if_present(&mut out, workspace, "sha");
    Value::Object(out)
}

fn canonical_runtime_target(target: &Value) -> Value {
    let mut out = Map::new();
    out.insert("kind".to_string(), value_at(target, &["kind"]));
    out.insert("command".to_string(), value_at(target, &["command"]));
    out.insert(
        "args".to_string(),
        target.get("args").cloned().unwrap_or_else(|| json!([])),
    );
    insert_if_present(&mut out, target, "cwd");
    Value::Object(out)
}

fn canonical_remote_target(target: &Value) -> Value {
    let mut out = Map::new();
    out.insert("name".to_string(), value_at(target, &["name"]));
    out.insert("url".to_string(), value_at(target, &["url"]));
    out.insert(
        "allowed_methods".to_string(),
        target
            .get("allowed_methods")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    Value::Object(out)
}

fn canonical_policy(policy: Option<&Value>, version: RequestDigestVersion) -> Value {
    let policy = policy.unwrap_or(&Value::Null);
    let mut out = Map::new();
    out.insert(
        "allow_degraded".to_string(),
        policy
            .get("allow_degraded")
            .cloned()
            .unwrap_or_else(|| json!(true)),
    );
    out.insert(
        "timeout_ms".to_string(),
        policy
            .get("timeout_ms")
            .cloned()
            .unwrap_or_else(|| json!(120_000)),
    );
    if version == RequestDigestVersion::Current {
        out.insert(
            "allow_local_runtime".to_string(),
            policy
                .get("allow_local_runtime")
                .cloned()
                .unwrap_or_else(|| json!(false)),
        );
    }
    out.insert(
        "external_research".to_string(),
        policy
            .get("external_research")
            .cloned()
            .unwrap_or_else(|| json!("forbid")),
    );
    out.insert(
        "render_targets".to_string(),
        policy
            .get("render_targets")
            .cloned()
            .unwrap_or_else(|| json!(["json", "markdown"])),
    );
    out.insert(
        "allowed_env".to_string(),
        policy
            .get("allowed_env")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    Value::Object(out)
}

fn insert_if_present(out: &mut Map<String, Value>, source: &Value, key: &str) {
    if let Some(value) = source.get(key) {
        out.insert(key.to_string(), value.clone());
    }
}

pub(super) fn sha256_digest(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}
