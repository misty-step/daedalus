//! Crucible-backed optimization target import and headroom probe.
//!
//! This is the first narrow slice of backlog 061: pull a Crucible
//! `crucible.eval_spec.v1` key-recall eval into Threshold, summarize the
//! existing freeze evidence as a bounded headroom probe, and leave a
//! Bitterblossom/Sprites trial request + receipt surface for the remote runner
//! seam. It intentionally does not mutate graders, arenas, or Bitterblossom
//! plane files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CRUCIBLE_EVAL_SCHEMA: &str = "crucible.eval_spec.v1";
const TARGET_SCHEMA: &str = "threshold.optimization_target.v1";
const HEADROOM_SCHEMA: &str = "threshold.headroom_probe.v1";
const GUARDRAILS_SCHEMA: &str = "threshold.optimizer_guardrails.v1";
const SPRITE_REQUEST_SCHEMA: &str = "threshold.sprite_trial_request.v1";
const SPRITE_RECEIPT_SCHEMA: &str = "threshold.sprite_trial_receipt.v1";

#[derive(Debug, Clone)]
pub struct OptimizationTargetError(pub String);

impl std::fmt::Display for OptimizationTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for OptimizationTargetError {}

#[derive(Debug, Clone)]
pub struct HeadroomProbeOptions {
    pub eval_spec: PathBuf,
    pub out_dir: PathBuf,
    pub budget_usd: f64,
    pub bb_config: Option<PathBuf>,
    pub bb_task: String,
    pub bb_bin: PathBuf,
    pub bb_repo: String,
    pub bb_rev: Option<String>,
    pub bb_change: Option<String>,
    pub dispatch_bitterblossom: bool,
}

#[derive(Debug, Clone)]
pub struct HeadroomProbeResult {
    pub out_dir: PathBuf,
    pub target: PathBuf,
    pub rig: PathBuf,
    pub headroom_probe: PathBuf,
    pub guardrails: PathBuf,
    pub trials: PathBuf,
    pub report: PathBuf,
    pub sprite_request: PathBuf,
    pub sprite_receipt: PathBuf,
    pub verdict: String,
    pub probe_point: Option<f64>,
}

#[derive(Debug, Clone)]
struct EvalInfo {
    spec: Value,
    id: String,
    decision: Option<String>,
    baselines: Vec<String>,
    runner_kind: String,
    source_trials: PathBuf,
    arena_dir: Option<PathBuf>,
    candidate_id: String,
    tasks: Vec<String>,
    eval_digest: String,
    trials_digest: String,
}

#[derive(Debug, Clone)]
struct CandidateSummary {
    candidate_id: String,
    candidate_kind: String,
    model: Option<String>,
    composition_hash: String,
    successes: u64,
    n: u64,
    point: Option<f64>,
    lower: Option<f64>,
    upper: Option<f64>,
    cost_usd_total: Option<f64>,
    wall_ms_total: u64,
    trials: u64,
    expected_defects: u64,
    false_positives: u64,
    reward_mean: Option<f64>,
    task_results: Vec<Value>,
}

pub fn run_headroom_probe(
    options: &HeadroomProbeOptions,
) -> Result<HeadroomProbeResult, Box<dyn std::error::Error>> {
    if !(options.budget_usd.is_finite() && options.budget_usd > 0.0) {
        return Err(OptimizationTargetError("--budget-usd must be positive".to_string()).into());
    }
    if options.bb_task.trim().is_empty() {
        return Err(OptimizationTargetError("--bb-task must not be empty".to_string()).into());
    }
    if options.dispatch_bitterblossom {
        if options.bb_repo.trim().is_empty() {
            return Err(OptimizationTargetError(
                "--bb-repo must not be empty when dispatching".to_string(),
            )
            .into());
        }
        if options.bb_rev.as_deref().unwrap_or("").trim().is_empty() {
            return Err(OptimizationTargetError(
                "--bb-rev must resolve to a fetchable revision when dispatching".to_string(),
            )
            .into());
        }
    }

    let eval_info = load_eval_info(&options.eval_spec)?;
    let all_records = read_jsonl_values(&eval_info.source_trials)?;
    let candidates = candidate_ids(&eval_info);
    let summaries = summarize_candidates(&eval_info, &all_records, &candidates)?;
    let selected_records = selected_trial_records(&eval_info, &all_records, &candidates);

    std::fs::create_dir_all(&options.out_dir)?;
    let target_path = options.out_dir.join("optimization-target.json");
    let rig_path = options.out_dir.join("rig.json");
    let headroom_path = options.out_dir.join("headroom-probe.json");
    let guardrails_path = options.out_dir.join("guardrails.json");
    let trials_path = options.out_dir.join("trials.jsonl");
    let report_path = options.out_dir.join("report.md");
    let request_path = options.out_dir.join("sprite-trial-request.json");
    let receipt_path = options.out_dir.join("sprite-trial-receipt.json");

    let target = build_target(&eval_info, options);
    let rig = build_rig(&eval_info, options, &candidates);
    let headroom = build_headroom_probe(&eval_info, options, &summaries);
    let guardrails = build_guardrails(&eval_info, &headroom);
    let request = build_sprite_request(&eval_info, options, &summaries, &target);

    write_json(&target_path, &target)?;
    write_json(&rig_path, &rig)?;
    write_json(&headroom_path, &headroom)?;
    write_json(&guardrails_path, &guardrails)?;
    write_trials_jsonl(&trials_path, &selected_records, &eval_info)?;
    write_json(&request_path, &request)?;

    let receipt = if options.dispatch_bitterblossom {
        dispatch_bitterblossom(options, &request_path, &request)
    } else {
        build_pending_receipt(&request)
    };
    write_json(&receipt_path, &receipt)?;

    let report = render_report(&eval_info, options, &summaries, &headroom, &receipt);
    std::fs::write(&report_path, report)?;

    let verdict = headroom
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let probe_point = summaries
        .iter()
        .find(|s| s.candidate_id == eval_info.candidate_id)
        .and_then(|s| s.point);

    Ok(HeadroomProbeResult {
        out_dir: options.out_dir.clone(),
        target: target_path,
        rig: rig_path,
        headroom_probe: headroom_path,
        guardrails: guardrails_path,
        trials: trials_path,
        report: report_path,
        sprite_request: request_path,
        sprite_receipt: receipt_path,
        verdict,
        probe_point,
    })
}

fn load_eval_info(eval_spec: &Path) -> Result<EvalInfo, Box<dyn std::error::Error>> {
    let spec_text = std::fs::read_to_string(eval_spec).map_err(|err| {
        OptimizationTargetError(format!("read Crucible eval {}: {err}", eval_spec.display()))
    })?;
    let spec: Value = serde_json::from_str(&spec_text).map_err(|err| {
        OptimizationTargetError(format!(
            "parse Crucible eval {}: {err}",
            eval_spec.display()
        ))
    })?;
    if spec.get("schema_version").and_then(Value::as_str) != Some(CRUCIBLE_EVAL_SCHEMA) {
        return Err(OptimizationTargetError(format!(
            "{} is not {CRUCIBLE_EVAL_SCHEMA}",
            eval_spec.display()
        ))
        .into());
    }
    let id = required_str(&spec, &["id"], "eval id")?.to_string();
    let runner = spec
        .get("runner")
        .and_then(Value::as_object)
        .ok_or_else(|| OptimizationTargetError("runner is required".to_string()))?;
    let runner_kind = runner
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| OptimizationTargetError("runner.kind is required".to_string()))?
        .to_string();
    if runner_kind != "key_recall" {
        return Err(OptimizationTargetError(format!(
            "unsupported Crucible runner.kind {runner_kind:?}; only key_recall is supported"
        ))
        .into());
    }
    let corpus = runner
        .get("corpus")
        .and_then(Value::as_object)
        .ok_or_else(|| OptimizationTargetError("runner.corpus is required".to_string()))?;
    let base = eval_spec.parent().unwrap_or_else(|| Path::new("."));
    let source_trials = resolve_existing(base, required_str_in(corpus, "trials_jsonl")?)?;
    let arena_dir = corpus
        .get("arena_dir")
        .and_then(Value::as_str)
        .map(|p| resolve_existing(base, p))
        .transpose()?;
    let candidate_id = required_str_in(corpus, "candidate_id")?.to_string();
    let tasks = corpus
        .get("tasks")
        .and_then(Value::as_array)
        .ok_or_else(|| OptimizationTargetError("runner.corpus.tasks is required".to_string()))?
        .iter()
        .map(|v| {
            v.as_str().map(str::to_string).ok_or_else(|| {
                OptimizationTargetError("runner.corpus.tasks entries must be strings".to_string())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if tasks.is_empty() {
        return Err(
            OptimizationTargetError("runner.corpus.tasks must not be empty".to_string()).into(),
        );
    }
    let baselines = spec
        .get("baselines")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["null".to_string(), "oracle".to_string()]);
    let eval_digest = format!("sha256:{}", sha256_hex(spec_text.as_bytes()));
    let trials_bytes = std::fs::read(&source_trials)?;
    let trials_digest = format!("sha256:{}", sha256_hex(&trials_bytes));
    let decision = spec
        .get("decision")
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(EvalInfo {
        spec,
        id,
        decision,
        baselines,
        runner_kind,
        source_trials,
        arena_dir,
        candidate_id,
        tasks,
        eval_digest,
        trials_digest,
    })
}

fn summarize_candidates(
    eval: &EvalInfo,
    records: &[Value],
    candidates: &[String],
) -> Result<Vec<CandidateSummary>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for candidate_id in candidates {
        let mut by_task: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for record in records {
            let rec_candidate = record.get("candidate_id").and_then(Value::as_str);
            let task = record.get("task_id").and_then(Value::as_str);
            if rec_candidate == Some(candidate_id.as_str()) {
                if let Some(task) = task {
                    if eval.tasks.iter().any(|t| t == task) {
                        by_task
                            .entry(task.to_string())
                            .or_default()
                            .push(record.clone());
                    }
                }
            }
        }
        let missing: Vec<String> = eval
            .tasks
            .iter()
            .filter(|task| !by_task.contains_key(task.as_str()))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Err(OptimizationTargetError(format!(
                "source trials missing candidate {candidate_id} for task(s): {}",
                missing.join(", ")
            ))
            .into());
        }
        let mut successes = 0_u64;
        let mut n = 0_u64;
        let mut known_cost = true;
        let mut cost_total = 0.0_f64;
        let mut wall_total = 0_u64;
        let mut false_positives = 0_u64;
        let mut reward_values = Vec::new();
        let mut task_results = Vec::new();
        let mut trial_count = 0_u64;
        let mut candidate_kind = String::new();
        let mut model = None;
        let mut composition_hash = String::new();

        for task in &eval.tasks {
            let task_records = by_task.get(task).expect("checked above");
            let mut task_successes = 0_u64;
            let mut task_n = 0_u64;
            let mut task_false_positives = 0_u64;
            let mut task_rewards = Vec::new();
            let mut source_run_ids = Vec::new();
            for record in task_records {
                let record_kind = record
                    .get("candidate_kind")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if candidate_kind.is_empty() {
                    candidate_kind = record_kind.to_string();
                }
                if model.is_none() {
                    model = record
                        .get("model")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                if composition_hash.is_empty() {
                    composition_hash = record
                        .get("composition_hash")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                }
                let expected = record
                    .get("expected_defects")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let matched = record
                    .get("matched")
                    .and_then(Value::as_array)
                    .map(|a| a.len() as u64)
                    .unwrap_or(0)
                    .min(expected);
                let fps = record
                    .get("false_positives")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let reward = record.get("reward").and_then(Value::as_f64);
                successes += matched;
                n += expected;
                task_successes += matched;
                task_n += expected;
                false_positives += fps;
                task_false_positives += fps;
                if let Some(reward) = reward {
                    reward_values.push(reward);
                    task_rewards.push(reward);
                }
                match record.get("cost_usd") {
                    Some(Value::Number(num)) => {
                        cost_total += num.as_f64().unwrap_or(0.0);
                    }
                    Some(Value::Null) | None if is_costless_kind(record_kind) => {}
                    _ => known_cost = false,
                }
                wall_total += record.get("wall_ms").and_then(Value::as_u64).unwrap_or(0);
                trial_count += 1;
                source_run_ids.push(record.get("run_id").cloned().unwrap_or(Value::Null));
            }
            let task_reward_mean = if task_rewards.is_empty() {
                None
            } else {
                Some(task_rewards.iter().sum::<f64>() / task_rewards.len() as f64)
            };
            task_results.push(json!({
                "task_id": task,
                "successes": task_successes,
                "n": task_n,
                "recall": if task_n > 0 { Some(task_successes as f64 / task_n as f64) } else { None },
                "reward_mean": task_reward_mean,
                "false_positives": task_false_positives,
                "trials": task_records.len(),
                "source_run_ids": source_run_ids
            }));
        }
        let point = if n > 0 {
            Some(successes as f64 / n as f64)
        } else {
            None
        };
        let (lower, upper) = match (successes, n) {
            (_, 0) => (None, None),
            (s, n) => {
                let (lo, hi) = wilson_interval(s, n);
                (Some(lo), Some(hi))
            }
        };
        let reward_mean = if reward_values.is_empty() {
            None
        } else {
            Some(reward_values.iter().sum::<f64>() / reward_values.len() as f64)
        };
        out.push(CandidateSummary {
            candidate_id: candidate_id.clone(),
            candidate_kind,
            model,
            composition_hash,
            successes,
            n,
            point,
            lower,
            upper,
            cost_usd_total: known_cost.then_some(cost_total),
            wall_ms_total: wall_total,
            trials: trial_count,
            expected_defects: n,
            false_positives,
            reward_mean,
            task_results,
        });
    }
    Ok(out)
}

fn selected_trial_records(eval: &EvalInfo, records: &[Value], candidates: &[String]) -> Vec<Value> {
    let candidate_set: BTreeSet<&str> = candidates.iter().map(String::as_str).collect();
    let task_set: BTreeSet<&str> = eval.tasks.iter().map(String::as_str).collect();
    records
        .iter()
        .filter(|record| {
            let candidate = record.get("candidate_id").and_then(Value::as_str);
            let task = record.get("task_id").and_then(Value::as_str);
            candidate.is_some_and(|c| candidate_set.contains(c))
                && task.is_some_and(|t| task_set.contains(t))
        })
        .cloned()
        .collect()
}

fn build_target(eval: &EvalInfo, options: &HeadroomProbeOptions) -> Value {
    json!({
        "schema": TARGET_SCHEMA,
        "id": eval.id,
        "source": "crucible",
        "mode": "threshold-then-cheap",
        "decision": eval.decision,
        "eval": {
            "schema": CRUCIBLE_EVAL_SCHEMA,
            "spec_ref": eval.spec.get("id").and_then(Value::as_str).unwrap_or(""),
            "spec_path": path_string(&options.eval_spec),
            "digest": eval.eval_digest,
            "harbor_package": eval.arena_dir.as_ref().map(|p| path_string(p)),
            "fixtures_digest": Value::Null,
            "answer_key_digest": Value::Null,
            "oracle_solution_digest": Value::Null,
            "source_trials_jsonl": path_string(&eval.source_trials),
            "source_trials_digest": eval.trials_digest,
            "version": eval.spec.get("version").cloned().unwrap_or(Value::Null)
        },
        "scorer": {
            "kind": "threshold-score",
            "metric": "pr_review_key_recall",
            "authoritative_until": "crucible-backlog-008-grade-reward-parity",
            "note": "This first slice imports Crucible key-recall evidence from a frozen Threshold run; full Harbor bundle digests remain a guardrail gap."
        },
        "splits": {
            "train": Value::Null,
            "validation": "runner.corpus.tasks",
            "holdout": Value::Null,
            "holdout_policy": "not declared by this Crucible key-recall eval"
        },
        "baselines": eval.baselines,
        "runner": {
            "default": eval.runner_kind,
            "remote": "bitter-blossom-sprites",
            "request_schema": SPRITE_REQUEST_SCHEMA,
            "receipt_schema": SPRITE_RECEIPT_SCHEMA,
            "bb_task": options.bb_task
        },
        "gates": {
            "g1": {"state": "unsigned", "meaning": "operator approval required before paid search beyond this bounded probe"},
            "g2": {"state": "unsigned", "meaning": "eval quality/headroom not fully approved; this probe is evidence for that decision"},
            "g3": {"state": "unsigned", "meaning": "launch approval required before downstream deployment"},
            "g5": {"state": "not_applicable", "meaning": "authored Crucible eval, not production trace re-ingestion"}
        }
    })
}

fn build_rig(eval: &EvalInfo, options: &HeadroomProbeOptions, candidates: &[String]) -> Value {
    json!({
        "schema_version": "threshold.optimization_rig.v1",
        "eval_id": eval.id,
        "eval_digest": eval.eval_digest,
        "source_trials": path_string(&eval.source_trials),
        "source_trials_digest": eval.trials_digest,
        "arena_dir": eval.arena_dir.as_ref().map(|p| path_string(p)),
        "runner_kind": eval.runner_kind,
        "tasks": eval.tasks,
        "candidates": candidates,
        "headroom_budget_usd": options.budget_usd,
        "remote_runner": {
            "plane": "bitterblossom",
            "substrate": "sprites",
            "config": options.bb_config.as_ref().map(|p| path_string(p)),
            "task": options.bb_task,
            "repo": options.bb_repo,
            "rev": options.bb_rev,
            "change": options.bb_change,
            "dispatch_requested": options.dispatch_bitterblossom
        }
    })
}

fn build_headroom_probe(
    eval: &EvalInfo,
    options: &HeadroomProbeOptions,
    summaries: &[CandidateSummary],
) -> Value {
    let oracle = summaries.iter().find(|s| s.candidate_id == "oracle");
    let null = summaries.iter().find(|s| s.candidate_id == "null");
    let probe = summaries
        .iter()
        .find(|s| s.candidate_id == eval.candidate_id);
    let oracle_point = oracle.and_then(|s| s.point);
    let null_point = null.and_then(|s| s.point);
    let probe_point = probe.and_then(|s| s.point);
    let saturated = matches!((oracle_point, probe_point), (Some(o), Some(p)) if p >= o - 0.1);
    let oracle_pass = oracle_point.is_some_and(|p| (p - 1.0).abs() < 1e-9);
    let null_pass = null_point.is_some_and(|p| p <= 0.000_001);
    let ranks = oracle_point.is_some()
        && null_point.is_some()
        && probe_point.is_some()
        && !saturated
        && oracle_point.unwrap_or(0.0) > null_point.unwrap_or(0.0);
    let verdict = if oracle_pass && null_pass && ranks {
        "pass"
    } else if saturated {
        "saturated"
    } else {
        "needs-review"
    };
    let candidate_values: Vec<Value> = summaries.iter().map(candidate_to_value).collect();
    let known_spend = summaries
        .iter()
        .try_fold(0.0_f64, |acc, c| c.cost_usd_total.map(|v| acc + v));
    json!({
        "schema": HEADROOM_SCHEMA,
        "eval_id": eval.id,
        "budget_usd": options.budget_usd,
        "spend_known_usd": known_spend,
        "metric": "defect_level_key_recall",
        "uncertainty": {
            "method": "wilson",
            "confidence": 0.95
        },
        "verdict": verdict,
        "checks": {
            "oracle_reaches_ceiling": oracle_pass,
            "null_is_floor": null_pass,
            "oneshot_not_saturated": !saturated,
            "reference_spread_exists": ranks,
            "under_budget": known_spend.is_none_or(|s| s <= options.budget_usd)
        },
        "candidates": candidate_values
    })
}

fn build_guardrails(eval: &EvalInfo, headroom: &Value) -> Value {
    json!({
        "schema": GUARDRAILS_SCHEMA,
        "eval_id": eval.id,
        "overfitting": {
            "status": "partial",
            "evidence": "imported key-recall probe uses the Crucible-declared task subset; no GEPA mutation or holdout reuse occurs in this slice",
            "holdout_feedback_blocked": true
        },
        "judge_gaming": {
            "status": "pass",
            "evidence": "objective is deterministic key recall from Threshold trial records; no judge-only winner is selected"
        },
        "non_stationarity": {
            "status": "pass",
            "eval_digest": eval.eval_digest,
            "source_trials_digest": eval.trials_digest,
            "model_provider_ids_recorded": true
        },
        "scorer_parity": {
            "status": "caveat",
            "evidence": "Crucible eval points at Threshold freeze evidence; Crucible grade parity remains outside this slice"
        },
        "harbor_bundle": {
            "status": if eval.arena_dir.is_some() { "partial" } else { "missing" },
            "evidence": "Crucible key-recall eval declares source trials and arena_dir, but not full answer-key/scorer/oracle digests"
        },
        "headroom_verdict": headroom.get("verdict").cloned().unwrap_or(Value::Null)
    })
}

fn build_sprite_request(
    eval: &EvalInfo,
    options: &HeadroomProbeOptions,
    summaries: &[CandidateSummary],
    target: &Value,
) -> Value {
    let incumbent = summaries
        .iter()
        .find(|s| s.candidate_id == eval.candidate_id)
        .or_else(|| {
            summaries
                .iter()
                .find(|s| !is_costless_kind(&s.candidate_kind))
        });
    let composition_hash = incumbent
        .map(|s| s.composition_hash.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| sha256_hex(format!("{}:{}", eval.id, options.bb_task).as_bytes()));
    let model = incumbent
        .and_then(|s| s.model.clone())
        .unwrap_or_else(|| "deepseek/deepseek-v4-pro".to_string());
    let trial_id = format!(
        "{}:{}:headroom-probe:{}",
        eval.id,
        options.bb_task,
        short_hash(&composition_hash)
    );
    let submission_id = format!("threshold-{trial_id}");
    let context = format!(
        "Threshold {} headroom/Sprites seam probe. Eval digest: {}; source trials digest: {}. Threshold-specific trial contract is embedded in this EVENT.json under schema={}.",
        eval.id, eval.eval_digest, eval.trials_digest, SPRITE_REQUEST_SCHEMA
    );
    json!({
        "schema": SPRITE_REQUEST_SCHEMA,
        "submission": submission_id,
        "repo": options.bb_repo,
        "rev": options.bb_rev,
        "change": options.bb_change,
        "context": context,
        "experiment_id": eval.id,
        "trial_id": trial_id,
        "task_id": options.bb_task,
        "candidate": {
            "composition_hash": format_hash(&composition_hash),
            "model": model,
            "thinking": Value::Null,
            "prompt_packet_digest": target.get("eval").and_then(|e| e.get("digest")).cloned().unwrap_or(Value::Null),
            "tool_policy": "bitterblossom-correctness"
        },
        "workspace": {
            "harbor_package": eval.arena_dir.as_ref().map(|p| path_string(p)),
            "candidate_visible_paths": ["RUN.json", "EVENT.json", "PR.diff", "environment/"],
            "hidden_paths": ["tests/", "solution/"]
        },
        "output_contract": "REPORT.json containing the Bitterblossom correctness verdict JSON; Threshold scores returned findings locally when available",
        "secret_names": ["OPENROUTER_API_KEY", "GH_TOKEN"],
        "env_allowlist": ["OPENROUTER_API_KEY", "GH_TOKEN"],
        "budget": {
            "max_cost_usd": 0.60,
            "timeout_seconds": 2700
        },
        "threshold": {
            "target_schema": TARGET_SCHEMA,
            "eval_id": eval.id,
            "eval_digest": eval.eval_digest,
            "source_trials_digest": eval.trials_digest,
            "score_owner": "threshold"
        },
        "threshold_submission": {
            "repo": options.bb_repo,
            "rev": options.bb_rev,
            "change": options.bb_change,
            "context": "Threshold backlog 061 first Sprites runner seam probe for Crucible pr-review-key-recall-v0"
        }
    })
}

fn dispatch_bitterblossom(
    options: &HeadroomProbeOptions,
    request_path: &Path,
    request: &Value,
) -> Value {
    let started = now_iso();
    let idempotency = format!(
        "threshold-{}-{}",
        request
            .get("trial_id")
            .and_then(Value::as_str)
            .unwrap_or("trial")
            .replace([':', '/'], "-"),
        short_hash(&sha256_hex(
            serde_json::to_string(request)
                .unwrap_or_default()
                .as_bytes()
        ))
    );
    let mut cmd = Command::new(&options.bb_bin);
    if let Some(config) = &options.bb_config {
        cmd.arg("--config").arg(config);
        if let Some(root) = bb_repo_root(config) {
            cmd.current_dir(root);
        }
    }
    cmd.arg("run")
        .arg(&options.bb_task)
        .arg("--idempotency-key")
        .arg(&idempotency)
        .arg("--payload-file")
        .arg(request_path)
        .arg("--json");

    let result = cmd.output();
    let ended = now_iso();
    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout_json = serde_json::from_str::<Value>(&stdout).unwrap_or(Value::Null);
            let run_id = extract_bb_run_id(&stdout_json);
            let status = if output.status.success() {
                "ok"
            } else {
                "failed"
            };
            let digest_basis = format!("{stdout}\n---stderr---\n{stderr}");
            json!({
                "schema": SPRITE_RECEIPT_SCHEMA,
                "trial_id": request.get("trial_id").cloned().unwrap_or(Value::Null),
                "task_id": request.get("task_id").cloned().unwrap_or(Value::Null),
                "composition_hash": request.get("candidate").and_then(|c| c.get("composition_hash")).cloned().unwrap_or(Value::Null),
                "bitter_blossom_run_id": run_id,
                "substrate": "sprites",
                "status": status,
                "artifact_refs": {
                    "findings": Value::Null,
                    "transcript": Value::Null,
                    "bb_stdout": stdout_json
                },
                "model_served": request.get("candidate").and_then(|c| c.get("model")).cloned().unwrap_or(Value::Null),
                "tokens_prompt": Value::Null,
                "tokens_completion": Value::Null,
                "cost_usd": Value::Null,
                "wall_ms": Value::Null,
                "error": if output.status.success() {
                    Value::Null
                } else {
                    json!({
                        "exit_code": output.status.code(),
                        "stderr_tail": tail(&stderr, 4000),
                        "stdout_tail": tail(&stdout, 4000)
                    })
                },
                "command": {
                    "bb_bin": path_string(&options.bb_bin),
                    "config": options.bb_config.as_ref().map(|p| path_string(p)),
                    "task": options.bb_task,
                    "idempotency_key": idempotency
                },
                "started_at": started,
                "ended_at": ended,
                "receipt_digest": format!("sha256:{}", sha256_hex(digest_basis.as_bytes()))
            })
        }
        Err(err) => json!({
            "schema": SPRITE_RECEIPT_SCHEMA,
            "trial_id": request.get("trial_id").cloned().unwrap_or(Value::Null),
            "task_id": request.get("task_id").cloned().unwrap_or(Value::Null),
            "composition_hash": request.get("candidate").and_then(|c| c.get("composition_hash")).cloned().unwrap_or(Value::Null),
            "bitter_blossom_run_id": Value::Null,
            "substrate": "sprites",
            "status": "failed",
            "artifact_refs": {
                "findings": Value::Null,
                "transcript": Value::Null
            },
            "model_served": request.get("candidate").and_then(|c| c.get("model")).cloned().unwrap_or(Value::Null),
            "tokens_prompt": Value::Null,
            "tokens_completion": Value::Null,
            "cost_usd": Value::Null,
            "wall_ms": Value::Null,
            "error": format!("failed to execute bb: {err}"),
            "started_at": started,
            "ended_at": ended,
            "receipt_digest": format!("sha256:{}", sha256_hex(err.to_string().as_bytes()))
        }),
    }
}

fn build_pending_receipt(request: &Value) -> Value {
    json!({
        "schema": SPRITE_RECEIPT_SCHEMA,
        "trial_id": request.get("trial_id").cloned().unwrap_or(Value::Null),
        "task_id": request.get("task_id").cloned().unwrap_or(Value::Null),
        "composition_hash": request.get("candidate").and_then(|c| c.get("composition_hash")).cloned().unwrap_or(Value::Null),
        "bitter_blossom_run_id": Value::Null,
        "substrate": "sprites",
        "status": "not_dispatched",
        "artifact_refs": {
            "findings": Value::Null,
            "transcript": Value::Null
        },
        "model_served": request.get("candidate").and_then(|c| c.get("model")).cloned().unwrap_or(Value::Null),
        "tokens_prompt": Value::Null,
        "tokens_completion": Value::Null,
        "cost_usd": Value::Null,
        "wall_ms": 0,
        "error": "run with --dispatch-bitterblossom to call bb run",
        "receipt_digest": Value::Null
    })
}

fn render_report(
    eval: &EvalInfo,
    options: &HeadroomProbeOptions,
    summaries: &[CandidateSummary],
    headroom: &Value,
    receipt: &Value,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Optimizer Headroom Probe: {}\n\n", eval.id));
    out.push_str(&format!(
        "- Verdict: `{}`\n",
        headroom
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    ));
    out.push_str(&format!("- Budget cap: `${:.2}`\n", options.budget_usd));
    out.push_str(&format!("- Crucible eval digest: `{}`\n", eval.eval_digest));
    out.push_str(&format!(
        "- Source trials digest: `{}`\n",
        eval.trials_digest
    ));
    out.push_str("\n| candidate | key recall | Wilson 95% CI | defects | reward mean | known cost | wall |\n");
    out.push_str("|---|---:|---:|---:|---:|---:|---:|\n");
    for summary in summaries {
        out.push_str(&format!(
            "| {} | {} | {} | {}/{} | {} | {} | {} ms |\n",
            summary.candidate_id,
            fmt_opt4(summary.point),
            fmt_ci(summary.lower, summary.upper),
            summary.successes,
            summary.n,
            fmt_opt4(summary.reward_mean),
            summary
                .cost_usd_total
                .map(|c| format!("${c:.4}"))
                .unwrap_or_else(|| "unknown".to_string()),
            summary.wall_ms_total
        ));
    }
    out.push_str("\n## Sprites Dispatch\n\n");
    out.push_str(&format!(
        "- Requested: `{}`\n",
        options.dispatch_bitterblossom
    ));
    out.push_str(&format!(
        "- Receipt status: `{}`\n",
        receipt
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    ));
    if let Some(run_id) = receipt
        .get("bitter_blossom_run_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!("- Bitterblossom run id: `{run_id}`\n"));
    }
    if let Some(error) = receipt.get("error") {
        if !error.is_null() {
            out.push_str(&format!("- Error: `{}`\n", compact_json(error)));
        }
    }
    out.push_str("\n## Guardrail Read\n\n");
    out.push_str("- This slice uses deterministic key-recall evidence; no judge-only objective decides a winner.\n");
    out.push_str("- Full Crucible Harbor bundle digests are not present in this eval spec, so this is a transitional target import, not a final G2 approval packet.\n");
    out.push_str("- Search remains blocked until G1/G2 approval; the probe only establishes whether the target has headroom.\n");
    out
}

fn candidate_to_value(summary: &CandidateSummary) -> Value {
    json!({
        "candidate_id": summary.candidate_id,
        "candidate_kind": summary.candidate_kind,
        "composition_hash": summary.composition_hash,
        "model": summary.model,
        "successes": summary.successes,
        "n": summary.n,
        "point": summary.point,
        "lower": summary.lower,
        "upper": summary.upper,
        "expected_defects": summary.expected_defects,
        "false_positives": summary.false_positives,
        "reward_mean": summary.reward_mean,
        "cost_usd_total": summary.cost_usd_total,
        "wall_ms_total": summary.wall_ms_total,
        "trials": summary.trials,
        "task_results": summary.task_results
    })
}

fn write_trials_jsonl(
    path: &Path,
    records: &[Value],
    eval: &EvalInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut text = String::new();
    for record in records {
        let mut cloned = record.clone();
        if let Some(obj) = cloned.as_object_mut() {
            obj.insert(
                "optimization_eval_id".to_string(),
                Value::String(eval.id.clone()),
            );
            obj.insert(
                "optimization_eval_digest".to_string(),
                Value::String(eval.eval_digest.clone()),
            );
        }
        text.push_str(&serde_json::to_string(&cloned)?);
        text.push('\n');
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    std::fs::write(path, text)?;
    Ok(())
}

fn read_jsonl_values(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| OptimizationTargetError(format!("read {}: {err}", path.display())))?;
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str::<Value>(line).map_err(|err| {
            OptimizationTargetError(format!("parse {} line {}: {err}", path.display(), idx + 1))
        })?);
    }
    Ok(out)
}

fn candidate_ids(eval: &EvalInfo) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for candidate in eval
        .baselines
        .iter()
        .chain(std::iter::once(&eval.candidate_id))
    {
        if seen.insert(candidate.clone()) {
            out.push(candidate.clone());
        }
    }
    out
}

fn required_str<'a>(
    value: &'a Value,
    path: &[&str],
    label: &str,
) -> Result<&'a str, OptimizationTargetError> {
    let mut current = value;
    for segment in path {
        current = current
            .get(*segment)
            .ok_or_else(|| OptimizationTargetError(format!("{label} is required")))?;
    }
    current
        .as_str()
        .ok_or_else(|| OptimizationTargetError(format!("{label} must be string")))
}

fn required_str_in<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, OptimizationTargetError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| OptimizationTargetError(format!("runner.corpus.{key} is required")))
}

fn resolve_existing(base: &Path, raw: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = Path::new(raw);
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    std::fs::canonicalize(&joined).map_err(|err| {
        OptimizationTargetError(format!(
            "resolve {} relative to {}: {err}",
            raw,
            base.display()
        ))
        .into()
    })
}

fn is_costless_kind(kind: &str) -> bool {
    matches!(kind, "oracle" | "null")
}

fn wilson_interval(successes: u64, n: u64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 0.0);
    }
    let z = 1.959_963_984_540_054_f64;
    let nf = n as f64;
    let phat = successes as f64 / nf;
    let z2 = z * z;
    let denom = 1.0 + z2 / nf;
    let center = (phat + z2 / (2.0 * nf)) / denom;
    let margin = z * ((phat * (1.0 - phat) + z2 / (4.0 * nf)) / nf).sqrt() / denom;
    ((center - margin).max(0.0), (center + margin).min(1.0))
}

fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_ref());
    format!("{:x}", hasher.finalize())
}

fn short_hash(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(12)
        .collect()
}

fn format_hash(hash: &str) -> String {
    if hash.starts_with("sha256:") {
        hash.to_string()
    } else {
        format!("sha256:{hash}")
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn now_iso() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs() as i64;
            let days = secs.div_euclid(86_400);
            let rem = secs.rem_euclid(86_400);
            let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
            let (y, mo, dy) = crate::pycompat::civil_from_days(days);
            format!("{y:04}-{mo:02}-{dy:02}T{h:02}:{m:02}:{s:02}+00:00")
        })
        .unwrap_or_else(|_| "1970-01-01T00:00:00+00:00".to_string())
}

fn bb_repo_root(config: &Path) -> Option<PathBuf> {
    let plane_dir = config.parent()?;
    plane_dir.parent().map(Path::to_path_buf)
}

fn extract_bb_run_id(stdout_json: &Value) -> Value {
    for key in ["run_id", "id"] {
        if let Some(v) = stdout_json.get(key).cloned() {
            return v;
        }
    }
    if let Some(v) = stdout_json
        .get("run")
        .and_then(|r| r.get("id").or_else(|| r.get("run_id")))
        .cloned()
    {
        return v;
    }
    Value::Null
}

fn tail(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}

fn fmt_opt4(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.4}"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn fmt_ci(lo: Option<f64>, hi: Option<f64>) -> String {
    match (lo, hi) {
        (Some(lo), Some(hi)) => format!("[{lo:.4}, {hi:.4}]"),
        _ => "n/a".to_string(),
    }
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "unprintable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_key_recall_eval_and_writes_headroom_probe() {
        let root = fresh_tmp("import");
        let eval_dir = root.join("evals");
        let run_dir = root.join("runs").join("freeze");
        std::fs::create_dir_all(&eval_dir).unwrap();
        std::fs::create_dir_all(&run_dir).unwrap();
        let trials = run_dir.join("trials.jsonl");
        std::fs::write(
            &trials,
            [
                TrialRow::new("oracle", "oracle", "t1", 2, &["a", "b"]).to_jsonl(),
                TrialRow::new("oracle", "oracle", "t2", 0, &[])
                    .reward(1.0)
                    .to_jsonl(),
                TrialRow::new("null", "null", "t1", 2, &[]).to_jsonl(),
                TrialRow::new("null", "null", "t2", 0, &[])
                    .reward(1.0)
                    .to_jsonl(),
                TrialRow::new("probe-oneshot", "oneshot", "t1", 2, &["a"])
                    .false_positives(1)
                    .reward(0.3)
                    .cost(0.25)
                    .to_jsonl(),
                TrialRow::new("probe-oneshot", "oneshot", "t1", 2, &[])
                    .reward(0.0)
                    .cost(0.02)
                    .to_jsonl(),
                TrialRow::new("probe-oneshot", "oneshot", "t2", 0, &[])
                    .reward(1.0)
                    .cost(0.05)
                    .to_jsonl(),
            ]
            .join("\n")
                + "\n",
        )
        .unwrap();
        let eval = eval_dir.join("pr-review-key-recall-v0.json");
        std::fs::write(
            &eval,
            json!({
                "schema_version": CRUCIBLE_EVAL_SCHEMA,
                "id": "pr-review-key-recall-v0",
                "task": "pr-review-key-recall",
                "baselines": ["null", "oracle"],
                "aggregation": "proportion",
                "runner": {
                    "kind": "key_recall",
                    "corpus": {
                        "trials_jsonl": "../runs/freeze/trials.jsonl",
                        "candidate_id": "probe-oneshot",
                        "tasks": ["t1", "t2"]
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        let out_dir = root.join("out");
        let result = run_headroom_probe(&HeadroomProbeOptions {
            eval_spec: eval,
            out_dir: out_dir.clone(),
            budget_usd: 5.0,
            bb_config: None,
            bb_task: "correctness".to_string(),
            bb_bin: PathBuf::from("bb"),
            bb_repo: "misty-step/threshold".to_string(),
            bb_rev: Some("abc123".to_string()),
            bb_change: Some("threshold-optimizer-061".to_string()),
            dispatch_bitterblossom: false,
        })
        .unwrap();
        assert_eq!(result.verdict, "pass");
        assert_eq!(result.probe_point, Some(0.25));
        let headroom: Value =
            serde_json::from_str(&std::fs::read_to_string(result.headroom_probe).unwrap()).unwrap();
        assert_eq!(headroom["schema"], HEADROOM_SCHEMA);
        assert_eq!(headroom["checks"]["oracle_reaches_ceiling"], true);
        assert_eq!(headroom["checks"]["oneshot_not_saturated"], true);
        assert_eq!(headroom["spend_known_usd"], json!(0.32));
        assert_eq!(headroom["candidates"][2]["successes"], json!(1));
        assert_eq!(headroom["candidates"][2]["n"], json!(4));
        assert_eq!(headroom["candidates"][2]["trials"], json!(3));
        let target: Value =
            serde_json::from_str(&std::fs::read_to_string(result.target).unwrap()).unwrap();
        assert_eq!(target["schema"], TARGET_SCHEMA);
        let request: Value =
            serde_json::from_str(&std::fs::read_to_string(result.sprite_request).unwrap()).unwrap();
        assert_eq!(request["schema"], SPRITE_REQUEST_SCHEMA);
        assert_eq!(request["repo"], "misty-step/threshold");
        assert_eq!(request["rev"], "abc123");
        let receipt: Value =
            serde_json::from_str(&std::fs::read_to_string(result.sprite_receipt).unwrap()).unwrap();
        assert_eq!(receipt["status"], "not_dispatched");
    }

    #[test]
    fn saturated_probe_returns_saturated_verdict() {
        let root = fresh_tmp("saturated");
        let eval_dir = root.join("evals");
        let run_dir = root.join("runs");
        std::fs::create_dir_all(&eval_dir).unwrap();
        std::fs::create_dir_all(&run_dir).unwrap();
        let trials = run_dir.join("trials.jsonl");
        std::fs::write(
            &trials,
            [
                TrialRow::new("oracle", "oracle", "t1", 2, &["a", "b"]).to_jsonl(),
                TrialRow::new("null", "null", "t1", 2, &[]).to_jsonl(),
                TrialRow::new("probe-oneshot", "oneshot", "t1", 2, &["a", "b"])
                    .reward(1.0)
                    .cost(0.1)
                    .to_jsonl(),
            ]
            .join("\n")
                + "\n",
        )
        .unwrap();
        let eval = eval_dir.join("eval.json");
        std::fs::write(
            &eval,
            json!({
                "schema_version": CRUCIBLE_EVAL_SCHEMA,
                "id": "eval",
                "baselines": ["null", "oracle"],
                "runner": {"kind": "key_recall", "corpus": {
                    "trials_jsonl": "../runs/trials.jsonl",
                    "candidate_id": "probe-oneshot",
                    "tasks": ["t1"]
                }}
            })
            .to_string(),
        )
        .unwrap();
        let result = run_headroom_probe(&HeadroomProbeOptions {
            eval_spec: eval,
            out_dir: root.join("out"),
            budget_usd: 5.0,
            bb_config: None,
            bb_task: "correctness".to_string(),
            bb_bin: PathBuf::from("bb"),
            bb_repo: "misty-step/threshold".to_string(),
            bb_rev: Some("abc123".to_string()),
            bb_change: Some("threshold-optimizer-061".to_string()),
            dispatch_bitterblossom: false,
        })
        .unwrap();
        assert_eq!(result.verdict, "saturated");
    }

    fn fresh_tmp(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "threshold-optimization-target-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    struct TrialRow<'a> {
        candidate_id: &'a str,
        candidate_kind: &'a str,
        task_id: &'a str,
        expected: u64,
        matched: &'a [&'a str],
        false_positives: u64,
        reward: f64,
        cost: Option<f64>,
    }

    impl<'a> TrialRow<'a> {
        fn new(
            candidate_id: &'a str,
            candidate_kind: &'a str,
            task_id: &'a str,
            expected: u64,
            matched: &'a [&'a str],
        ) -> Self {
            TrialRow {
                candidate_id,
                candidate_kind,
                task_id,
                expected,
                matched,
                false_positives: 0,
                reward: if expected == matched.len() as u64 {
                    1.0
                } else {
                    0.0
                },
                cost: None,
            }
        }

        fn false_positives(mut self, false_positives: u64) -> Self {
            self.false_positives = false_positives;
            self
        }

        fn reward(mut self, reward: f64) -> Self {
            self.reward = reward;
            self
        }

        fn cost(mut self, cost: f64) -> Self {
            self.cost = Some(cost);
            self
        }

        fn to_jsonl(&self) -> String {
            serde_json::to_string(&json!({
                "run_id": format!("{}-{}", self.candidate_id, self.task_id),
                "candidate_id": self.candidate_id,
                "candidate_kind": self.candidate_kind,
                "composition_hash": format!("{}-hash", self.candidate_id),
                "model": if self.candidate_kind == "oneshot" { Value::String("deepseek/deepseek-v4-pro".to_string()) } else { Value::Null },
                "task_id": self.task_id,
                "trial": 1,
                "cost_usd": self.cost,
                "wall_ms": 10,
                "reward": self.reward,
                "matched": self.matched,
                "false_positives": self.false_positives,
                "expected_defects": self.expected
            }))
            .unwrap()
        }
    }
}
