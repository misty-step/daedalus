//! Cerberus reviewer-config packet export.
//!
//! This is a downstream handoff view over an existing measured Threshold
//! delivery. It deliberately does not deploy, approve, or mutate Cerberus
//! defaults; Cerberus' own validator/importer is the acceptance oracle.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::pycompat::utc_now_iso;
use crate::validate::SchemaVersion;

#[derive(Debug)]
pub struct CerberusExportError(pub String);

impl std::fmt::Display for CerberusExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CerberusExportError {}

pub fn export_reviewer_config_packet(
    delivery_dir: &Path,
    spec: &Value,
    out: &Path,
    generated_ts: Option<&str>,
    repo_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let packet = build_reviewer_config_packet(delivery_dir, spec, generated_ts, repo_root)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(&packet)?;
    text.push('\n');
    std::fs::write(out, text)?;
    Ok(out.to_path_buf())
}

pub fn build_reviewer_config_packet(
    delivery_dir: &Path,
    spec: &Value,
    generated_ts: Option<&str>,
    repo_root: &Path,
) -> Result<Value, Box<dyn std::error::Error>> {
    let candidate = crate::run::load_candidate(&delivery_dir.join("agent.toml"), repo_root)?;
    let contract = load_contract(delivery_dir)?;
    let run_dir = contract_string(&contract, &["evidence", "run_dir"])?;
    let run_dir_path = resolve_repo_path(&run_dir, repo_root);
    let summary = load_json(&run_dir_path.join("summary.json"))?;
    let pareto = load_json(&run_dir_path.join("pareto.json"))?;
    let candidate_id = contract_string(&contract, &["agent"])?;
    let candidate_summary = summary
        .get(&candidate_id)
        .ok_or_else(|| {
            CerberusExportError(format!(
                "summary.json has no candidate entry for {candidate_id}"
            ))
        })?
        .clone();
    let pareto_entry = find_pareto_entry(&pareto, &candidate_id)?;

    let generated_at = generated_ts
        .map(str::to_string)
        .or_else(|| contract_get_string(&contract, &["generated"]))
        .unwrap_or_else(utc_now_iso);
    let delivery_id = repo_relative(delivery_dir, repo_root);
    let composition_hash = contract_string(&contract, &["composition_hash"])?;
    let agent = value_string(&candidate, "id")?;
    let kind = value_string(&candidate, "kind")?;
    let provider = value_string(&candidate, "provider_name")?;
    let model = value_string(&candidate, "model")?;
    let harness_version = contract_get_string(&contract, &["composition", "harness_version"]);
    let harness_id = format!("{kind}-{provider}");
    let prompt_hash = format!(
        "sha256:{}",
        sha256_bytes(value_string(&candidate, "_packet_text")?.as_bytes())
    );
    let config = review_config(&agent, &provider, &model);
    let config_hash = digest_json(&config)?;

    let arena_ref = contract_get_string(&contract, &["observability", "arena"])
        .or_else(|| spec_get_string(spec, &["inputs", "fixtures"]))
        .ok_or_else(|| CerberusExportError("missing arena reference".to_string()))?;
    let arena = load_arena(&arena_ref, repo_root)?;
    let arena_id = arena
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_else(|| arena_name(&arena_ref));
    let arena_version = arena
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let task_count = arena_task_count(&arena_ref, repo_root).unwrap_or_else(|| {
        candidate_summary
            .get("tasks")
            .and_then(Value::as_object)
            .map(|tasks| tasks.len() as u64)
            .unwrap_or(1)
    });
    let suite_id =
        strip_version_suffix(spec.get("id").and_then(Value::as_str).unwrap_or_else(|| {
            delivery_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("delivery")
        }));

    let packet = json!({
        "schema_version": SchemaVersion::REVIEWER_CONFIG_PACKET,
        "packet_id": format!(
            "threshold-{}-{}-{}-sandbox",
            sanitize(&suite_id),
            sanitize(&composition_hash),
            sanitize(&agent)
        ),
        "producer": {
            "system": "threshold",
            "delivery_id": delivery_id,
            "generated_at": generated_at,
            "sandbox_only": true,
            "signature": null
        },
        "benchmark": {
            "benchmark_id": arena_id,
            "suite_id": suite_id,
            "arena_version": arena_version,
            "run_id": run_dir,
            "task_count": task_count,
            "score_distribution": score_distribution(&candidate_summary, &pareto_entry)
        },
        "promotion": {
            "status": "sandbox_only",
            "gates": promotion_gates(&contract),
            "rationale": format!("Threshold G2 accepted measured composition_hash={composition_hash} only for sandboxed handoff; G3/G4/G5 remain pending.")
        },
        "rollback": {
            "baseline_config_id": "default-fake-review-panel",
            "rollback_command": "restore ReviewConfig.v1 defaults from cerberus-core::default_config",
            "reason": "Sandbox-only Threshold handoff must be reversible before any Cerberus defaults change.",
            "previous_packet_id": null
        },
        "cost": {
            "measured_cost_usd": number_or(&pareto_entry, "cost_usd_per_trial", 0.0),
            "max_cost_usd": contract_number(&contract, &["budgets", "max_cost_usd_per_run"]).unwrap_or(0.0),
            "measured_wall_sec": number_or(&pareto_entry, "wall_mean_s", 0.0),
            "max_wall_sec": contract_number(&contract, &["budgets", "max_wall_sec"]).unwrap_or(0.0)
        },
        "harnesses": [{
            "harness_id": harness_id,
            "kind": kind,
            "provider_name": provider,
            "command": value_string(&candidate, "kind")?,
            "version": harness_version,
            "execution_mode": "sandbox"
        }],
        "models": [{
            "reviewer_id": "pr_review",
            "harness_id": harness_id,
            "provider": provider,
            "model": model,
            "prompt_hash": prompt_hash,
            "context_length": context_length_for_model(&model, repo_root)
        }],
        "prompt_hashes": {
            "pr_review": prompt_hash
        },
        "config_hash": config_hash,
        "config": config
    });
    Ok(packet)
}

fn load_contract(delivery_dir: &Path) -> Result<toml::Value, Box<dyn std::error::Error>> {
    let path = delivery_dir.join("contract.toml");
    let text = std::fs::read_to_string(&path).map_err(|error| {
        CerberusExportError(format!("failed to read {}: {error}", path.display()))
    })?;
    toml::from_str(&text).map_err(|error| {
        CerberusExportError(format!("failed to parse {}: {error}", path.display())).into()
    })
}

fn load_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        CerberusExportError(format!("failed to read {}: {error}", path.display()))
    })?;
    serde_json::from_str(&text).map_err(|error| {
        CerberusExportError(format!("failed to parse {}: {error}", path.display())).into()
    })
}

fn load_arena(arena_ref: &str, repo_root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let path = resolve_repo_path(arena_ref, repo_root).join("arena.toml");
    let text = std::fs::read_to_string(&path).map_err(|error| {
        CerberusExportError(format!("failed to read {}: {error}", path.display()))
    })?;
    let toml: toml::Value = toml::from_str(&text).map_err(|error| {
        CerberusExportError(format!("failed to parse {}: {error}", path.display()))
    })?;
    Ok(crate::run::toml_to_json(toml))
}

fn review_config(agent: &str, provider: &str, model: &str) -> Value {
    json!({
        "schema_version": SchemaVersion::REVIEW_CONFIG,
        "config_id": format!("threshold-sandbox-{}", sanitize(agent)),
        "reviewers": [{
            "id": "pr_review",
            "perspective": "correctness",
            "model": format!("{provider}:{model}"),
            "fake_behavior": "directive"
        }],
        "confidence_min": 0.7
    })
}

fn promotion_gates(contract: &toml::Value) -> Vec<Value> {
    let g3 = contract_get_string(contract, &["approval", "g3_approval"])
        .unwrap_or_else(|| "approvals/G3-pending.md".to_string());
    vec![
        json!({
            "name": "G2",
            "status": "waived",
            "evidence": "approvals/G2-pr-review-v2.md",
            "waiver": "accepted with sandbox-only waivers; not a production quality claim"
        }),
        json!({
            "name": "G3",
            "status": "pending",
            "evidence": g3
        }),
        json!({
            "name": "G4",
            "status": "pending",
            "evidence": "approvals/G4-pr-review-write-authority.md"
        }),
        json!({
            "name": "G5",
            "status": "pending",
            "evidence": "approvals/G5-pr-review-production-reingestion.md"
        }),
    ]
}

fn score_distribution(candidate_summary: &Value, pareto_entry: &Value) -> Value {
    let mut rewards = Vec::new();
    if let Some(tasks) = candidate_summary.get("tasks").and_then(Value::as_object) {
        for task in tasks.values() {
            if let Some(values) = task.get("rewards").and_then(Value::as_array) {
                rewards.extend(values.iter().filter_map(Value::as_f64));
            }
        }
    }
    rewards.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (min, median, max) = if rewards.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        let min = *rewards.first().unwrap();
        let max = *rewards.last().unwrap();
        let mid = rewards.len() / 2;
        let median = if rewards.len() % 2 == 0 {
            (rewards[mid - 1] + rewards[mid]) / 2.0
        } else {
            rewards[mid]
        };
        (min, median, max)
    };
    let certified_trials = candidate_summary
        .get("tasks")
        .and_then(Value::as_object)
        .and_then(|tasks| {
            tasks
                .values()
                .filter_map(|task| task.get("rewards").and_then(Value::as_array).map(Vec::len))
                .min()
        })
        .unwrap_or(0);
    json!({
        "min": min,
        "mean": number_or(pareto_entry, "reward_mean", 0.0),
        "median": median,
        "max": max,
        "certified_trials": certified_trials as u64
    })
}

fn find_pareto_entry(pareto: &Value, candidate_id: &str) -> Result<Value, CerberusExportError> {
    let entries = pareto
        .as_array()
        .ok_or_else(|| CerberusExportError("pareto.json must be an array".to_string()))?;
    entries
        .iter()
        .find(|entry| {
            entry
                .get("candidate_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == candidate_id)
        })
        .cloned()
        .ok_or_else(|| {
            CerberusExportError(format!(
                "pareto.json has no candidate entry for {candidate_id}"
            ))
        })
}

fn digest_json(value: &Value) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn value_string(candidate: &Map<String, Value>, key: &str) -> Result<String, CerberusExportError> {
    candidate
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CerberusExportError(format!("candidate missing string field {key}")))
}

fn contract_string(contract: &toml::Value, path: &[&str]) -> Result<String, CerberusExportError> {
    contract_get_string(contract, path).ok_or_else(|| {
        CerberusExportError(format!("contract missing string field {}", path.join(".")))
    })
}

fn contract_get_string(contract: &toml::Value, path: &[&str]) -> Option<String> {
    let value = descend_toml(contract, path)?;
    value.as_str().map(str::to_string)
}

fn contract_number(contract: &toml::Value, path: &[&str]) -> Option<f64> {
    let value = descend_toml(contract, path)?;
    match value {
        toml::Value::Integer(value) => Some(*value as f64),
        toml::Value::Float(value) => Some(*value),
        _ => None,
    }
}

fn descend_toml<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn spec_get_string(spec: &Value, path: &[&str]) -> Option<String> {
    let mut current = spec;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn number_or(value: &Value, key: &str, fallback: f64) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

fn repo_relative(path: &Path, repo_root: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let repo = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    abs.strip_prefix(&repo)
        .unwrap_or(&abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn resolve_repo_path(path: &str, repo_root: &Path) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn arena_name(arena_ref: &str) -> &str {
    arena_ref
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(arena_ref)
}

fn arena_task_count(arena_ref: &str, repo_root: &Path) -> Option<u64> {
    let tasks = resolve_repo_path(arena_ref, repo_root).join("tasks");
    let entries = std::fs::read_dir(tasks).ok()?;
    Some(
        entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .count() as u64,
    )
}

fn strip_version_suffix(value: &str) -> String {
    value
        .strip_suffix("-v0")
        .or_else(|| value.strip_suffix("-v1"))
        .or_else(|| value.strip_suffix("-v2"))
        .unwrap_or(value)
        .to_string()
}

fn sanitize(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn context_length_for_model(model: &str, repo_root: &Path) -> Option<u64> {
    let primitives = std::fs::read_to_string(repo_root.join("docs/primitives.md")).ok()?;
    for line in primitives.lines() {
        if line.contains(&format!("`{model}`")) && line.contains("1M") {
            return Some(1_048_576);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("repo root")
            .to_path_buf()
    }

    fn spec() -> Value {
        let text = std::fs::read_to_string(repo_root().join("specs/pr-review/taskspec.toml"))
            .expect("spec readable");
        let toml: toml::Value = toml::from_str(&text).expect("spec toml");
        crate::run::toml_to_json(toml)
    }

    #[test]
    fn export_pr_review_packet_preserves_measured_evidence() {
        let repo = repo_root();
        let packet = build_reviewer_config_packet(
            &repo.join("deliveries/pr-review"),
            &spec(),
            Some("2026-06-12T00:18:48Z"),
            &repo,
        )
        .expect("packet");

        assert_eq!(packet["schema_version"], "reviewer-config-packet.v1");
        assert_eq!(
            packet["packet_id"],
            "threshold-pr-review-4a73f1fd213aa1a5-seed4-qwen3-7-plus-checklist-sandbox"
        );
        assert_eq!(packet["producer"]["system"], "threshold");
        assert_eq!(packet["producer"]["sandbox_only"], true);
        assert_eq!(packet["benchmark"]["benchmark_id"], "pr-review-v2");
        assert_eq!(packet["benchmark"]["suite_id"], "pr-review");
        assert_eq!(packet["benchmark"]["arena_version"], "0.2.0");
        assert_eq!(packet["benchmark"]["task_count"], 10);
        assert_eq!(packet["benchmark"]["score_distribution"]["mean"], 0.5714);
        assert_eq!(
            packet["benchmark"]["score_distribution"]["certified_trials"],
            5
        );
        assert_eq!(packet["promotion"]["status"], "sandbox_only");
        assert!(packet["promotion"]["rationale"]
            .as_str()
            .unwrap()
            .contains("composition_hash=4a73f1fd213aa1a5"));
        assert_eq!(packet["promotion"]["gates"][0]["name"], "G2");
        assert_eq!(packet["promotion"]["gates"][0]["status"], "waived");
        assert_eq!(packet["promotion"]["gates"][1]["status"], "pending");
        assert_eq!(
            packet["promotion"]["gates"][2]["evidence"],
            "approvals/G4-pr-review-write-authority.md"
        );
        assert_eq!(
            packet["promotion"]["gates"][3]["evidence"],
            "approvals/G5-pr-review-production-reingestion.md"
        );
        assert_eq!(packet["cost"]["measured_cost_usd"], 0.017009);
        assert_eq!(packet["cost"]["measured_wall_sec"], 70.7);
        assert_eq!(packet["harnesses"][0]["version"], "0.78.1");
        assert_eq!(packet["models"][0]["model"], "qwen/qwen3.7-plus");
        assert_eq!(
            packet["prompt_hashes"]["pr_review"],
            "sha256:4ce0f7d61af3b5b3ac6f58db7dae9e1e9278a61d169249ce8e932e3711eb9198"
        );
        assert_eq!(packet["config"]["reviewers"].as_array().unwrap().len(), 1);
        assert_eq!(packet["config"]["reviewers"][0]["id"], "pr_review");
        assert_eq!(
            packet["config"]["reviewers"][0]["fake_behavior"],
            "directive"
        );
        assert_eq!(
            packet["config_hash"],
            digest_json(&packet["config"]).unwrap()
        );
    }

    #[test]
    fn export_writes_pretty_json_packet() {
        let repo = repo_root();
        let out = std::env::temp_dir().join(format!(
            "threshold-cerberus-export-{}.json",
            std::process::id()
        ));
        export_reviewer_config_packet(
            &repo.join("deliveries/pr-review"),
            &spec(),
            &out,
            Some("2026-06-12T00:18:48Z"),
            &repo,
        )
        .expect("export");
        let raw = std::fs::read_to_string(&out).expect("packet file");
        assert!(raw.ends_with('\n'));
        let packet: Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(
            packet["packet_id"],
            "threshold-pr-review-4a73f1fd213aa1a5-seed4-qwen3-7-plus-checklist-sandbox"
        );
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn score_distribution_uses_recorded_rewards() {
        let summary = json!({
            "tasks": {
                "a": {"rewards": [0.0, 1.0]},
                "b": {"rewards": [0.5, 1.0]}
            }
        });
        let pareto = json!({"reward_mean": 0.625});
        let dist = score_distribution(&summary, &pareto);
        assert_eq!(dist["min"], 0.0);
        assert_eq!(dist["mean"], 0.625);
        assert_eq!(dist["median"], 0.75);
        assert_eq!(dist["max"], 1.0);
        assert_eq!(dist["certified_trials"], 2);
    }
}
