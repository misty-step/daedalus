//! Reflective mutation step: propose exactly one single-slot change to a
//! composition, grounded in its worst trials.
//!
//! Port of `runner/mutate.py`. The module is split into:
//!
//! - **Deterministic core** — `proposal_instructions`, `worst_trials`,
//!   `evidence_block`, `build_prompt`, `parse_proposal`, `validate_proposal`,
//!   `normalize_predicted_effect`, `resolve_donor`, `build_child`,
//!   `write_manifest`, and `propose`. These are parity-tested in
//!   `tests/parity_mutate.rs`.
//!
//! - **LLM boundary** — `call_optimizer` is a thin `std::process::Command`
//!   wrapper (structural port of the `urllib.request` retry loop). It is NOT
//!   parity-tested because it requires a live OpenRouter endpoint; any pure
//!   helpers it relies on are unit-tested below.
//!
//! ## Injected `call`
//!
//! `propose` takes an injected `call: FnMut(&str, &str) -> (String, Option<f64>)`
//! so the full step is testable offline with a fake proposer — the same
//! pattern used in `judge.rs`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value};

use crate::prompt_packet::is_sane_prompt_packet;
use crate::pycompat::py_json_dumps;

/// Render a `&[&str]` as Python's `repr(sorted_list)`: `['a', 'b', 'c']`.
/// Used wherever Python f-strings embed `sorted(some_set_or_list)` in error messages.
fn py_repr_list(items: &[&str]) -> String {
    let inner: Vec<String> = items.iter().map(|s| format!("'{s}'")).collect();
    format!("[{}]", inner.join(", "))
}

// ---------------------------------------------------------------------------
// Constants — mirror Python module-level sets
// ---------------------------------------------------------------------------

/// Slots that the optimizer is allowed to propose.
///
/// Frozen slots (temperature, max_tokens, env_allowlist, kind, …) are excluded
/// by design: pi exposes no flags for them, so "mutating" them would change the
/// composition hash without changing behavior (false attribution by construction).
const MUTABLE_SLOTS: &[&str] = &[
    "prompt_packet",
    "model",
    "thinking",
    "tools",
    "system_prompt_mode",
    "agents_md",
    "skills",
];

const THINKING_LEVELS: &[&str] = &["high", "low", "medium", "minimal", "off", "xhigh"];
const SYSTEM_PROMPT_MODES: &[&str] = &["append", "replace"];
const PREDICTED_REWARD: &[&str] = &["hold", "up"];
const PREDICTED_COST: &[&str] = &["down", "hold", "up"];

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

// ---------------------------------------------------------------------------
// normalize_predicted_effect
// ---------------------------------------------------------------------------

/// The proposer's testable prediction, scored later against measurement.
///
/// A missing prediction defaults to `{reward: up, cost: hold}` and is flagged
/// as defaulted. Returns `(predicted_effect_map, defaulted)`.
pub fn normalize_predicted_effect(
    proposal: &Map<String, Value>,
) -> Result<(Map<String, Value>, bool), String> {
    let pe = proposal.get("predicted_effect");
    if pe.is_none() || pe == Some(&Value::Null) {
        let mut m = Map::new();
        m.insert("reward".into(), Value::String("up".into()));
        m.insert("cost".into(), Value::String("hold".into()));
        return Ok((m, true));
    }
    let pe = pe.unwrap();
    let obj = pe.as_object().ok_or_else(|| {
        "predicted_effect must be {\"reward\": \"up|hold\", \"cost\": \"down|hold|up\"}".to_string()
    })?;
    let reward = obj.get("reward").and_then(Value::as_str).unwrap_or("");
    let cost = obj.get("cost").and_then(Value::as_str).unwrap_or("");
    if !PREDICTED_REWARD.contains(&reward) || !PREDICTED_COST.contains(&cost) {
        return Err(
            "predicted_effect must be {\"reward\": \"up|hold\", \"cost\": \"down|hold|up\"}"
                .to_string(),
        );
    }
    let mut m = Map::new();
    m.insert("reward".into(), Value::String(reward.into()));
    m.insert("cost".into(), Value::String(cost.into()));
    Ok((m, false))
}

// ---------------------------------------------------------------------------
// resolve_donor
// ---------------------------------------------------------------------------

/// Transplant operator: take the proposed slot's value from a named archive
/// candidate.
///
/// Returns a new proposal map with `"value"` filled in from the donor.
pub fn resolve_donor(
    proposal: &Map<String, Value>,
    archive_manifests: &HashMap<String, Map<String, Value>>,
    _parent_manifest: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let donor = proposal
        .get("donor")
        .and_then(Value::as_str)
        .ok_or_else(|| "proposal has no donor field".to_string())?;
    let slot = proposal
        .get("slot")
        .and_then(Value::as_str)
        .ok_or_else(|| "proposal has no slot field".to_string())?;

    let snap = archive_manifests
        .get(donor)
        .ok_or_else(|| format!("unknown transplant donor '{donor}'"))?;

    let value = if slot == "prompt_packet" {
        snap.get("prompt_packet_text").cloned()
    } else if slot == "agents_md" {
        snap.get("agents_md_text").cloned()
    } else {
        snap.get(slot).cloned()
    };

    let value = value.ok_or_else(|| format!("donor '{donor}' has no value for slot '{slot}'"))?;

    let mut out = proposal.clone();
    out.insert("value".into(), value);
    Ok(out)
}

// ---------------------------------------------------------------------------
// proposal_instructions
// ---------------------------------------------------------------------------

/// Compose the slot menu from the declared search space.
///
/// The brief asks for the highest-information experiment under the declared
/// mode — no slot is privileged. Mirrors `proposal_instructions(...)` exactly,
/// including `json.dumps(sorted(...))` for list rendering.
pub fn proposal_instructions(
    tool_policies: Option<&HashMap<String, Vec<String>>>,
    allowed_models: Option<&[String]>,
    allowed_thinking: Option<&[String]>,
    avoid_slots: &[String],
    skill_sets: Option<&HashMap<String, Vec<String>>>,
    mode: Option<&str>,
    donors: Option<&[String]>,
) -> String {
    let mut lines: Vec<String> = vec![
        "You are the search step of an agent-optimization loop. Your job: propose".into(),
        "EXACTLY ONE change to ONE slot of the candidate composition below — the".into(),
        "highest-information single-variable experiment available given the evidence".into(),
        format!(
            "and the objective mode ({}). Never change two things.",
            mode.unwrap_or("max-quality")
        ),
        "".into(),
        "Mutable slots and value rules:".into(),
        "- \"prompt_packet\": value is the FULL replacement packet text (system-prompt".into(),
        "  guidance for the review agent).".into(),
    ];

    if let Some(models) = allowed_models {
        let mut sorted = models.to_vec();
        sorted.sort();
        let sorted_val: Value =
            Value::Array(sorted.iter().map(|s| Value::String(s.clone())).collect());
        let json_list = py_json_dumps(&sorted_val, false);
        lines.push(format!("- \"model\": one of {json_list}."));
    } else {
        lines.push("- \"model\": an OpenRouter model id string.".into());
    }

    if let Some(thinking) = allowed_thinking {
        let mut sorted = thinking.to_vec();
        sorted.sort();
        let sorted_val: Value =
            Value::Array(sorted.iter().map(|s| Value::String(s.clone())).collect());
        let json_list = py_json_dumps(&sorted_val, false);
        lines.push(format!("- \"thinking\": one of {json_list}."));
    } else {
        lines.push("- \"thinking\": one of off|minimal|low|medium|high|xhigh.".into());
    }

    lines.push(
        "- \"system_prompt_mode\": \"append\" (packet added to pi's default \
coding prompt) or \"replace\" (packet IS the whole system prompt)."
            .into(),
    );
    lines.push(
        "- \"agents_md\": the FULL text of an AGENTS.md placed in the agent's \
workspace root (repo-context briefing it reads on startup)."
            .into(),
    );

    if let Some(policies) = tool_policies {
        let mut sorted_keys: Vec<&str> = policies.keys().map(|s| s.as_str()).collect();
        sorted_keys.sort_unstable();
        let sorted_val: Value = Value::Array(
            sorted_keys
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        );
        let json_list = py_json_dumps(&sorted_val, false);
        lines.push(format!(
            "- \"tools\": one of the named tool policies \
{json_list} (value is the policy name)."
        ));
    }

    if let Some(sets) = skill_sets {
        let mut sorted_keys: Vec<&str> = sets.keys().map(|s| s.as_str()).collect();
        sorted_keys.sort_unstable();
        let sorted_val: Value = Value::Array(
            sorted_keys
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        );
        let json_list = py_json_dumps(&sorted_val, false);
        lines.push(format!(
            "- \"skills\": one of the named skill sets \
{json_list} (value is the set name)."
        ));
    }

    if let Some(donors) = donors {
        if !donors.is_empty() {
            let mut sorted_donors = donors.to_vec();
            sorted_donors.sort();
            let sorted_val: Value = Value::Array(
                sorted_donors
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            );
            let donors_json = py_json_dumps(&sorted_val, false);
            lines.push("".into());
            lines.push(
                "You may instead TRANSPLANT one slot's value from another archive candidate".into(),
            );
            lines.push(format!(
                "(donors: {donors_json}) by adding \"donor\": \"<candidate_id>\""
            ));
            lines.push(
                "and omitting \"value\" — e.g. move a strong packet onto a cheaper model, or a"
                    .into(),
            );
            lines.push("cheap model under a winning packet. Still exactly one slot.".into());
        }
    }

    if !avoid_slots.is_empty() {
        let unique: HashSet<&str> = avoid_slots.iter().map(|s| s.as_str()).collect();
        let mut sorted_avoid: Vec<&str> = unique.into_iter().collect();
        sorted_avoid.sort_unstable();
        lines.push("".into());
        lines.push(format!(
            "Competing hypotheses this generation already target: {}. Propose a DIFFERENT slot.",
            py_repr_list(&sorted_avoid)
        ));
    }

    lines.extend([
        "".into(),
        "Respond with ONLY a JSON object:".into(),
        "{\"slot\": \"<slot>\", \"value\": <value> (or \"donor\": \"<candidate_id>\"),".into(),
        " \"hypothesis\": \"<one or two sentences: what evidence this addresses and why>\",".into(),
        " \"predicted_effect\": {\"reward\": \"up|hold\", \"cost\": \"down|hold|up\"}}".into(),
        "".into(),
        "predicted_effect is your testable prediction; it will be scored against the".into(),
        "measured outcome and recorded in the lab notebook.".into(),
    ]);

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// worst_trials
// ---------------------------------------------------------------------------

/// The candidate's lowest-reward trials, worst first.
///
/// Mirrors `sorted(own, key=lambda r: (r["reward"], -r["wall_ms"]))[:n]`.
pub fn worst_trials(
    records: &[Map<String, Value>],
    candidate_id: &str,
    n: usize,
) -> Vec<Map<String, Value>> {
    let mut own: Vec<&Map<String, Value>> = records
        .iter()
        .filter(|r| r.get("candidate_id").and_then(Value::as_str) == Some(candidate_id))
        .collect();
    // Sort by (reward ASC, wall_ms DESC)
    own.sort_by(|a, b| {
        let ra = a.get("reward").and_then(Value::as_f64).unwrap_or(0.0);
        let rb = b.get("reward").and_then(Value::as_f64).unwrap_or(0.0);
        let wa = a.get("wall_ms").and_then(Value::as_f64).unwrap_or(0.0);
        let wb = b.get("wall_ms").and_then(Value::as_f64).unwrap_or(0.0);
        ra.partial_cmp(&rb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(wb.partial_cmp(&wa).unwrap_or(std::cmp::Ordering::Equal))
    });
    own.into_iter().take(n).cloned().collect()
}

// ---------------------------------------------------------------------------
// evidence_block
// ---------------------------------------------------------------------------

/// Build the worst-trial evidence block for the optimizer prompt.
///
/// Mirrors `evidence_block(trials, exp_dir, transcript_chars=3000)`.
/// `exp_dir` is `Option<&Path>`; when `None`, transcript tails are skipped.
pub fn evidence_block(trials: &[Map<String, Value>], exp_dir: Option<&Path>) -> String {
    let transcript_chars: usize = 3000;
    let mut parts: Vec<String> = Vec::new();

    for r in trials {
        let matched = r
            .get("matched")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        let expected = r.get("expected_defects");
        let run_id = r.get("run_id").and_then(Value::as_str).unwrap_or("");
        let task_id = r.get("task_id").and_then(Value::as_str).unwrap_or("");
        let reward = r
            .get("reward")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let false_positives = r
            .get("false_positives")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let error = r
            .get("error")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let findings = r.get("findings");
        let findings_str = findings
            .map(|v| {
                let s = serde_json::to_string(v).unwrap_or_default();
                // match [:800] slice
                let chars: Vec<char> = s.chars().collect();
                let end = chars.len().min(800);
                chars[..end].iter().collect::<String>()
            })
            .unwrap_or_else(|| "null".into());

        // expected_defects rendered as Python's str of the value
        let expected_str = expected
            .map(|v| match v {
                Value::Null => "None".into(),
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| "None".into());

        parts.push(format!(
            "### Trial {run_id}\ntask: {task_id}  reward: {reward}  \
verdict: matched {matched} of {expected_str} seeded defects, \
{false_positives} finding(s) penalized as false positives  error: {error}\n\
findings: {findings_str}\n"
        ));

        // Transcript tail
        if let (Some(art), Some(dir)) = (r.get("artifacts").and_then(Value::as_str), exp_dir) {
            let transcript_path = dir.join(art).join("transcript.txt");
            if transcript_path.exists() {
                if let Ok(text) = std::fs::read_to_string(&transcript_path) {
                    let chars: Vec<char> = text.chars().collect();
                    let start = chars.len().saturating_sub(transcript_chars);
                    let tail: String = chars[start..].iter().collect();
                    parts.push(format!("transcript tail:\n```\n{tail}\n```\n"));
                }
            }
        }
    }

    parts.join("\n")
}

// ---------------------------------------------------------------------------
// build_prompt
// ---------------------------------------------------------------------------

/// Build the full optimizer prompt from task, parent, evidence, and archive.
///
/// Mirrors `build_prompt(taskspec, parent_snapshot, trials_evidence, archive_summary, instructions)`.
/// `json.dumps(slots, indent=2)` → `serde_json::to_string_pretty`.
/// `json.dumps(archive_summary, indent=2)[:2000]` → pretty + char-truncate to 2000.
pub fn build_prompt(
    taskspec: &Map<String, Value>,
    parent_snapshot: &Map<String, Value>,
    trials_evidence: &str,
    archive_summary: &Value,
    instructions: Option<&str>,
) -> String {
    // Build the slots sub-object exactly as Python does:
    // {k: parent_snapshot.get(k) for k in ("model","thinking","tools","kind")}
    let mut slots = Map::new();
    for key in &["model", "thinking", "tools", "kind"] {
        slots.insert(
            (*key).to_string(),
            parent_snapshot.get(*key).cloned().unwrap_or(Value::Null),
        );
    }
    let slots_json = serde_json::to_string_pretty(&Value::Object(slots)).unwrap_or_default();

    let archive_json = serde_json::to_string_pretty(archive_summary).unwrap_or_default();
    // [:2000] on characters
    let archive_chars: Vec<char> = archive_json.chars().collect();
    let archive_end = archive_chars.len().min(2000);
    let archive_truncated: String = archive_chars[..archive_end].iter().collect();

    let goal = taskspec.get("goal").and_then(Value::as_str).unwrap_or("");
    let mode = taskspec.get("mode").and_then(Value::as_str).unwrap_or("");
    let prompt_packet_text = parent_snapshot
        .get("prompt_packet_text")
        .and_then(Value::as_str)
        .unwrap_or("(none)");

    // Use the default instructions string when none provided
    let instructions_str: String;
    let instr: &str = if let Some(s) = instructions {
        s
    } else {
        instructions_str = proposal_instructions(None, None, None, &[], None, None, None);
        &instructions_str
    };

    format!(
        "{instr}\n## Task\ngoal: {goal}\nmode: {mode}\n\
\n## Candidate composition (parent)\n{slots_json}\n\
\ncurrent prompt_packet text:\n---\n{prompt_packet_text}\n---\n\
\n## Archive (what has been tried)\n{archive_truncated}\
\n\n## Worst-trial evidence\n{trials_evidence}"
    )
}

// ---------------------------------------------------------------------------
// parse_proposal
// ---------------------------------------------------------------------------

/// Extract the first parseable JSON object from LLM output.
///
/// Mirrors the Python brace-scan loop exactly: advance `start` on each failure,
/// track depth + string escape state, attempt JSON parse at depth==0. Raises
/// `Err` when no parseable proposal is found.
pub fn parse_proposal(text: &str) -> Result<Map<String, Value>, String> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut start = 0;

    // find next '{' from position `start`
    while start < len {
        // Find next '{'
        let found = chars[start..].iter().position(|&c| c == '{');
        let abs_start = match found {
            Some(rel) => start + rel,
            None => break,
        };

        // Scan from abs_start tracking depth + string state
        let mut depth = 0i32;
        let mut in_str = false;
        let mut escape = false;
        let mut i = abs_start;

        while i < len {
            let ch = chars[i];
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_str = !in_str;
            } else if !in_str {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        // Attempt parse on text[abs_start..=i]
                        let candidate: String = chars[abs_start..=i].iter().collect();
                        match serde_json::from_str::<Value>(&candidate) {
                            Ok(Value::Object(m)) => return Ok(m),
                            _ => break,
                        }
                    }
                }
            }
            i += 1;
        }

        start = abs_start + 1;
    }

    Err("optimizer returned no parseable proposal".to_string())
}

// ---------------------------------------------------------------------------
// validate_proposal
// ---------------------------------------------------------------------------

/// Reject anything that is not a well-formed single-slot mutation drawn from
/// the declared search space.
///
/// Returns `(slot, value, hypothesis)` on success.
#[allow(clippy::too_many_arguments)] // faithful port of Python kwargs signature
pub fn validate_proposal(
    proposal: &Map<String, Value>,
    parent_manifest: &Map<String, Value>,
    tool_policies: Option<&HashMap<String, Vec<String>>>,
    allowed_models: Option<&[String]>,
    allowed_thinking: Option<&[String]>,
    avoid_slots: &[String],
    skill_sets: Option<&HashMap<String, Vec<String>>>,
    donor: Option<&str>,
) -> Result<(String, Value, String), String> {
    let slot = proposal.get("slot").and_then(Value::as_str).unwrap_or("");
    let value = proposal.get("value");
    let hypothesis = proposal.get("hypothesis");

    // Check slot is mutable
    if !MUTABLE_SLOTS.contains(&slot) {
        let mut sorted = MUTABLE_SLOTS.to_vec();
        sorted.sort_unstable();
        return Err(format!(
            "slot '{slot}' is not mutable (allowed: {})",
            py_repr_list(&sorted)
        ));
    }

    // Check avoid_slots
    if avoid_slots.contains(&slot.to_string()) {
        return Err(format!(
            "slot '{slot}' already targeted by a competing hypothesis this generation"
        ));
    }

    // Check hypothesis is non-empty
    let hyp = hypothesis
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if hyp.is_empty() {
        return Err("proposal missing hypothesis".to_string());
    }

    let value_owned = value.cloned().unwrap_or(Value::Null);

    match slot {
        "prompt_packet" => {
            let text = value_owned.as_str().unwrap_or("");
            if !is_sane_prompt_packet(text) {
                return Err("prompt_packet value failed sanity check".to_string());
            }
        }
        "model" => {
            let model = value_owned.as_str().unwrap_or("");
            if !model.contains('/') {
                return Err("model value must be an OpenRouter model id".to_string());
            }
            if let Some(models) = allowed_models {
                if !models.contains(&model.to_string()) {
                    return Err(format!(
                        "model '{model}' is outside the declared search space"
                    ));
                }
            }
            if parent_manifest.get("model").and_then(Value::as_str) == Some(model) {
                return Err("model mutation must differ from parent".to_string());
            }
        }
        "thinking" => {
            let level = value_owned.as_str().unwrap_or("");
            if !THINKING_LEVELS.contains(&level) {
                let mut sorted = THINKING_LEVELS.to_vec();
                sorted.sort_unstable();
                return Err(format!("thinking must be one of {}", py_repr_list(&sorted)));
            }
            if let Some(allowed) = allowed_thinking {
                if !allowed.contains(&level.to_string()) {
                    return Err(format!(
                        "thinking '{level}' is outside the declared search space"
                    ));
                }
            }
            if parent_manifest.get("thinking").and_then(Value::as_str) == Some(level) {
                return Err("thinking mutation must differ from parent".to_string());
            }
        }
        "tools" => {
            if donor.is_some() {
                // Transplant: value is already resolved to a list
                let new_list = value_as_str_vec(&value_owned);
                let parent_list = value_as_str_vec(
                    parent_manifest
                        .get("tools")
                        .unwrap_or(&Value::Array(vec![])),
                );
                if new_list == parent_list {
                    return Err("tools transplant must differ from parent".to_string());
                }
            } else {
                let policy_name = value_owned.as_str().unwrap_or("");
                let policies = tool_policies
                    .ok_or_else(|| "tools mutation requires declared tool_policies".to_string())?;
                if !policies.contains_key(policy_name) {
                    let mut sorted_keys: Vec<&str> = policies.keys().map(|s| s.as_str()).collect();
                    sorted_keys.sort_unstable();
                    return Err(format!(
                        "tools value must be a policy name from {}",
                        py_repr_list(&sorted_keys)
                    ));
                }
                let policy_list = policies
                    .get(policy_name)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let parent_list = value_as_str_vec(
                    parent_manifest
                        .get("tools")
                        .unwrap_or(&Value::Array(vec![])),
                );
                if policy_list == parent_list.as_slice() {
                    return Err("tools mutation must differ from parent".to_string());
                }
            }
        }
        "system_prompt_mode" => {
            let mode = value_owned.as_str().unwrap_or("");
            if !SYSTEM_PROMPT_MODES.contains(&mode) {
                let mut sorted = SYSTEM_PROMPT_MODES.to_vec();
                sorted.sort_unstable();
                return Err(format!(
                    "system_prompt_mode must be one of {}",
                    py_repr_list(&sorted)
                ));
            }
            // Default is "append" when key is absent
            let parent_mode = parent_manifest
                .get("system_prompt_mode")
                .and_then(Value::as_str)
                .unwrap_or("append");
            if mode == parent_mode {
                return Err("system_prompt_mode mutation must differ from parent".to_string());
            }
        }
        "agents_md" => {
            let text = value_owned.as_str().unwrap_or("");
            if text.trim().len() < 20 {
                return Err("agents_md value must be substantial briefing text".to_string());
            }
        }
        "skills" => {
            if donor.is_some() {
                // Transplant: value is already resolved to a list
                let new_list = value_as_str_vec(&value_owned);
                let parent_list = value_as_str_vec(
                    parent_manifest
                        .get("skills")
                        .unwrap_or(&Value::Array(vec![])),
                );
                if new_list == parent_list {
                    return Err("skills transplant must differ from parent".to_string());
                }
            } else {
                let set_name = value_owned.as_str().unwrap_or("");
                let sets = skill_sets
                    .ok_or_else(|| "skills mutation requires declared skill_sets".to_string())?;
                if !sets.contains_key(set_name) {
                    let mut sorted_keys: Vec<&str> = sets.keys().map(|s| s.as_str()).collect();
                    sorted_keys.sort_unstable();
                    return Err(format!(
                        "skills value must be a set name from {}",
                        py_repr_list(&sorted_keys)
                    ));
                }
                let set_list = sets.get(set_name).map(|v| v.as_slice()).unwrap_or(&[]);
                let parent_list = value_as_str_vec(
                    parent_manifest
                        .get("skills")
                        .unwrap_or(&Value::Array(vec![])),
                );
                if set_list == parent_list.as_slice() {
                    return Err("skills mutation must differ from parent".to_string());
                }
            }
        }
        _ => {}
    }

    Ok((slot.to_string(), value_owned, hyp))
}

/// Helper: extract a Vec<String> from a JSON array-of-strings, exactly
/// reproducing `list(value)` where value is already a Python list.
fn value_as_str_vec(v: &Value) -> Vec<String> {
    match v {
        Value::Array(a) => a
            .iter()
            .filter_map(|e| e.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// build_child
// ---------------------------------------------------------------------------

/// Materialize the child manifest (and any mutated text file).
///
/// `tools` and `skills` mutations carry the *name* of a declared set; the
/// manifest gets the resolved list. `prompt_packet` and `agents_md` carry full
/// text, written to versioned files under `packets_dir`.
pub fn build_child(
    parent_manifest: &Map<String, Value>,
    slot: &str,
    value: &Value,
    child_id: &str,
    packets_dir: &Path,
    tool_policies: Option<&HashMap<String, Vec<String>>>,
    skill_sets: Option<&HashMap<String, Vec<String>>>,
) -> Result<Map<String, Value>, String> {
    // Copy all non-underscore, non-id keys from parent
    let mut child: Map<String, Value> = parent_manifest
        .iter()
        .filter(|(k, _)| !k.starts_with('_') && k.as_str() != "id")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    child.insert("id".into(), Value::String(child_id.to_string()));

    match slot {
        "prompt_packet" => {
            let text = value.as_str().unwrap_or("");
            let path = write_text_slot(packets_dir, child_id, ".md", text)?;
            child.insert("prompt_packet".into(), Value::String(path));
        }
        "agents_md" => {
            let text = value.as_str().unwrap_or("");
            let path = write_text_slot(packets_dir, child_id, "-agents.md", text)?;
            child.insert("agents_md".into(), Value::String(path));
        }
        "tools" => {
            let resolved: Vec<Value> = if value.is_array() {
                // Transplanted (already a list)
                value.as_array().cloned().unwrap_or_default()
            } else if let Some(name) = value.as_str() {
                tool_policies
                    .and_then(|p| p.get(name))
                    .map(|v| v.iter().map(|s| Value::String(s.clone())).collect())
                    .unwrap_or_default()
            } else {
                vec![]
            };
            child.insert("tools".into(), Value::Array(resolved));
        }
        "skills" => {
            let resolved: Vec<Value> = if value.is_array() {
                // Transplanted (already a list)
                value.as_array().cloned().unwrap_or_default()
            } else if let Some(name) = value.as_str() {
                skill_sets
                    .and_then(|s| s.get(name))
                    .map(|v| v.iter().map(|s| Value::String(s.clone())).collect())
                    .unwrap_or_default()
            } else {
                vec![]
            };
            child.insert("skills".into(), Value::Array(resolved));
        }
        _ => {
            child.insert(slot.to_string(), value.clone());
        }
    }

    Ok(child)
}

/// Write text to `<packets_dir>/<child_id><suffix>`, appending `\n` if needed.
/// Returns the string path (mirroring Python's `str(path)`).
fn write_text_slot(
    packets_dir: &Path,
    child_id: &str,
    suffix: &str,
    text: &str,
) -> Result<String, String> {
    std::fs::create_dir_all(packets_dir)
        .map_err(|e| format!("create_dir_all {}: {e}", packets_dir.display()))?;
    let path = packets_dir.join(format!("{child_id}{suffix}"));
    let content = if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    };
    std::fs::write(&path, content).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// write_manifest
// ---------------------------------------------------------------------------

/// Serialize a child manifest as a bespoke TOML-like file.
///
/// Mirrors Python's `write_manifest(child, path)` EXACTLY (string-templated,
/// not serialized through a TOML library):
///   - `bool`  → `true` / `false`
///   - `int`/`float` → `json.dumps(val)` (numbers)
///   - `list`  → `json.dumps(val)`
///   - anything else → `json.dumps(str(val))` (quoted string)
///
/// One key=value per line, trailing newline.
pub fn write_manifest(child: &Map<String, Value>, path: &Path) -> Result<PathBuf, String> {
    let mut lines: Vec<String> = Vec::new();
    for (key, val) in child {
        // Python write_manifest:
        //   bool  → "true"/"false"  (not json.dumps, just str(val).lower())
        //   int/float → json.dumps(val)   (numbers: no difference from serde compact)
        //   list  → json.dumps(val)       (uses ", " separator — need py_json_dumps)
        //   else  → json.dumps(str(val))  (string quoting; str(None) → "None")
        let rendered = match val {
            Value::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Value::Number(_) => {
                // json.dumps of a number: same as compact serialization for int/float
                py_json_dumps(val, false)
            }
            Value::Array(_) => {
                // json.dumps(list) uses ", " between elements — use py_json_dumps
                py_json_dumps(val, false)
            }
            // Null → json.dumps(str(None)) → json.dumps("None") → "\"None\""
            Value::Null => "\"None\"".to_string(),
            Value::String(s) => {
                // json.dumps(str(s)) — the string is already a str, so just dump it
                py_json_dumps(&Value::String(s.clone()), false)
            }
            Value::Object(_) => {
                // Python: json.dumps(str(val)) where str(dict) gives Python repr.
                // We approximate with json.dumps(json.dumps(val)) since dicts
                // in manifests are not expected in practice.
                let as_str = py_json_dumps(val, false);
                py_json_dumps(&Value::String(as_str), false)
            }
        };
        lines.push(format!("{key} = {rendered}"));
    }
    let content = lines.join("\n") + "\n";
    std::fs::write(path, &content).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path.to_path_buf())
}

// ---------------------------------------------------------------------------
// call_optimizer  (LLM boundary — NOT parity-tested)
// ---------------------------------------------------------------------------

/// Call the optimizer model via the OpenRouter HTTP API, with exponential
/// backoff retries for transient errors.
///
/// ## LLM boundary
///
/// This function requires a live `OPENROUTER_API_KEY` env var and a running
/// OpenRouter endpoint. It is NOT parity-tested. The argv/payload structure
/// mirrors `runner/mutate.py::call_optimizer` exactly.
///
/// Returns `(content, Option<cost_usd>)`.
pub fn call_optimizer(
    prompt: &str,
    model: &str,
    timeout_secs: u64,
    retries: u32,
) -> Result<(String, Option<f64>), String> {
    let key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| "OPENROUTER_API_KEY is not set".to_string())?;

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.7,
        "max_tokens": 16384,
        "usage": {"include": true},
    });
    let body_str =
        serde_json::to_string(&body).map_err(|e| format!("serialize request body: {e}"))?;

    let mut last_err = String::new();
    for attempt in 0..retries {
        // Use curl as the subprocess transport (no external Rust HTTP dep)
        let mut cmd = Command::new("curl");
        cmd.arg("-s")
            .arg("-m")
            .arg(timeout_secs.to_string())
            .arg("-X")
            .arg("POST")
            .arg(OPENROUTER_URL)
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-H")
            .arg(format!("Authorization: Bearer {key}"))
            .arg("-d")
            .arg(&body_str);

        match cmd.output() {
            Ok(out) if out.status.success() => {
                let payload: Value = serde_json::from_slice(&out.stdout)
                    .map_err(|e| format!("parse optimizer response: {e}"))?;
                let choice = payload
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .ok_or_else(|| "optimizer response missing choices".to_string())?;
                let msg = choice
                    .get("message")
                    .ok_or_else(|| "optimizer choice missing message".to_string())?;
                let content = msg
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let content = if content.trim().is_empty() {
                    msg.get("reasoning")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                } else {
                    content
                };
                if content.trim().is_empty() {
                    let finish_reason = choice
                        .get("finish_reason")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    last_err =
                        format!("optimizer returned empty content (finish_reason={finish_reason})");
                } else {
                    let cost = payload
                        .get("usage")
                        .and_then(|u| u.get("cost"))
                        .and_then(Value::as_f64);
                    return Ok((content, cost));
                }
            }
            Ok(out) => {
                last_err = format!(
                    "curl exited {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Err(e) => {
                last_err = format!("curl spawn failed: {e}");
            }
        }

        if attempt + 1 < retries {
            std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
        }
    }

    Err(format!(
        "optimizer call failed after {retries} attempts: {last_err}"
    ))
}

// ---------------------------------------------------------------------------
// propose  (full step with injected call — testable offline)
// ---------------------------------------------------------------------------

/// Full mutation step: evidence → LLM proposal → validation → child on disk.
///
/// `call` is the injected optimizer call:
///   `FnMut(prompt: &str, model: &str) -> Result<(content: String, cost: Option<f64>), String>`
///
/// Returns `(manifest_path, metadata_map)`.
#[allow(clippy::too_many_arguments)] // faithful port of Python kwargs signature
pub fn propose<F>(
    taskspec: &Map<String, Value>,
    parent_snapshot: &Map<String, Value>,
    parent_manifest: &Map<String, Value>,
    records: &[Map<String, Value>],
    exp_dir: Option<&Path>,
    child_id: &str,
    optimizer_model: &str,
    packets_dir: &Path,
    manifests_dir: &Path,
    archive_summary: Option<&Value>,
    tool_policies: Option<&HashMap<String, Vec<String>>>,
    allowed_models: Option<&[String]>,
    allowed_thinking: Option<&[String]>,
    avoid_slots: &[String],
    skill_sets: Option<&HashMap<String, Vec<String>>>,
    archive_manifests: Option<&HashMap<String, Map<String, Value>>>,
    mode: Option<&str>,
    mut call: F,
) -> Result<(PathBuf, Map<String, Value>), String>
where
    F: FnMut(&str, &str) -> Result<(String, Option<f64>), String>,
{
    let parent_id = parent_snapshot
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("");

    let trials = worst_trials(records, parent_id, 3);
    let empty_archive: HashMap<String, Map<String, Value>> = HashMap::new();
    let archive_ref = archive_manifests.unwrap_or(&empty_archive);

    let mut donors: Vec<String> = archive_ref
        .keys()
        .filter(|cid| cid.as_str() != parent_id)
        .cloned()
        .collect();
    donors.sort();

    let donors_opt: Option<&[String]> = if donors.is_empty() {
        None
    } else {
        Some(&donors)
    };

    let instr = proposal_instructions(
        tool_policies,
        allowed_models,
        allowed_thinking,
        avoid_slots,
        skill_sets,
        mode,
        donors_opt,
    );

    let default_archive = Value::Object(Map::new());
    let archive_val = archive_summary.unwrap_or(&default_archive);
    let prompt = build_prompt(
        taskspec,
        parent_snapshot,
        &evidence_block(&trials, exp_dir),
        archive_val,
        Some(&instr),
    );

    let (content, cost) = call(&prompt, optimizer_model)?;
    let proposal = parse_proposal(&content)?;
    let (predicted_effect, pe_defaulted) = normalize_predicted_effect(&proposal)?;
    let donor = proposal
        .get("donor")
        .and_then(Value::as_str)
        .map(str::to_string);

    let resolved_proposal = if donor.is_some() {
        resolve_donor(&proposal, archive_ref, parent_manifest)?
    } else {
        proposal.clone()
    };

    let donor_str = donor.as_deref();
    let (slot, value, hypothesis) = validate_proposal(
        &resolved_proposal,
        parent_manifest,
        tool_policies,
        allowed_models,
        allowed_thinking,
        avoid_slots,
        skill_sets,
        donor_str,
    )?;

    let child = build_child(
        parent_manifest,
        &slot,
        &value,
        child_id,
        packets_dir,
        tool_policies,
        skill_sets,
    )?;

    std::fs::create_dir_all(manifests_dir).map_err(|e| format!("create manifests_dir: {e}"))?;
    let manifest_path = write_manifest(&child, &manifests_dir.join(format!("{child_id}.toml")))?;

    let value_summary: Value = if slot == "prompt_packet" || slot == "agents_md" {
        Value::String("(new text)".into())
    } else {
        value.clone()
    };

    let mut meta: Map<String, Value> = Map::new();
    meta.insert("child_id".into(), Value::String(child_id.to_string()));
    meta.insert("parent_id".into(), Value::String(parent_id.to_string()));
    meta.insert("slot_changed".into(), Value::String(slot.clone()));
    meta.insert("value_summary".into(), value_summary);
    meta.insert("hypothesis".into(), Value::String(hypothesis));
    meta.insert("predicted_effect".into(), Value::Object(predicted_effect));
    meta.insert(
        "optimizer_cost_usd".into(),
        cost.map(Value::from).unwrap_or(Value::Null),
    );
    if pe_defaulted {
        meta.insert("predicted_effect_defaulted".into(), Value::Bool(true));
    }
    if let Some(d) = donor {
        meta.insert("donor".into(), Value::String(d));
    }

    Ok((manifest_path, meta))
}

// ---------------------------------------------------------------------------
// Unit tests (port of tests/test_mutate.py; fake `call` for propose)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn parent() -> Map<String, Value> {
        serde_json::from_str(
            r#"{
            "composition": 1,
            "id": "pi-kimi",
            "kind": "pi",
            "model": "moonshotai/kimi-k2.6",
            "prompt_packet": "packets/reviewer-v1.md",
            "thinking": "medium",
            "tools": ["read", "bash", "edit", "write"],
            "timeout_sec": 600
        }"#,
        )
        .unwrap()
    }

    fn policies() -> HashMap<String, Vec<String>> {
        let mut m = HashMap::new();
        m.insert(
            "full".into(),
            vec!["read".into(), "bash".into(), "edit".into(), "write".into()],
        );
        m.insert("explore".into(), vec!["read".into(), "bash".into()]);
        m
    }

    // parse_proposal

    #[test]
    fn parse_proposal_with_fences_and_braces_in_strings() {
        let text = r#"Reasoning...
```json
{"slot": "prompt_packet", "value": "Use {curly} braces and review carefully always.", "hypothesis": "more specific"}
```"#;
        let p = parse_proposal(text).unwrap();
        assert_eq!(p.get("slot").and_then(Value::as_str), Some("prompt_packet"));
        assert!(p
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("{curly}"));
    }

    #[test]
    fn parse_proposal_recovers_from_reasoning_style_text() {
        let text = r#"Let me think. The agent missed cross-file context, so:

{"slot": "model", "value": "anthropic/claude-x", "hypothesis": "stronger model for cross-file reasoning"}

That should help."#;
        let p = parse_proposal(text).unwrap();
        assert_eq!(p.get("slot").and_then(Value::as_str), Some("model"));
        assert_eq!(
            p.get("value").and_then(Value::as_str),
            Some("anthropic/claude-x")
        );
    }

    // validate_proposal — unknown/frozen slots

    #[test]
    fn validate_rejects_unknown_and_frozen_slots() {
        for slot in &[
            "kind",
            "env_allowlist",
            "temperature",
            "max_tokens",
            "nonsense",
        ] {
            let proposal: Map<String, Value> = serde_json::from_str(&format!(
                r#"{{"slot": "{slot}", "value": "x", "hypothesis": "h"}}"#
            ))
            .unwrap();
            let err = validate_proposal(&proposal, &parent(), None, None, None, &[], None, None)
                .unwrap_err();
            assert!(
                err.contains("not mutable"),
                "expected 'not mutable' for slot={slot}, got: {err}"
            );
        }
    }

    #[test]
    fn validate_rejects_no_op_mutations() {
        let thinking_noop: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "thinking", "value": "medium", "hypothesis": "h"
        }))
        .unwrap();
        let err = validate_proposal(&thinking_noop, &parent(), None, None, None, &[], None, None)
            .unwrap_err();
        assert!(err.contains("differ from parent"), "{err}");

        let model_noop: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "model", "value": "moonshotai/kimi-k2.6", "hypothesis": "h"
        }))
        .unwrap();
        let err = validate_proposal(&model_noop, &parent(), None, None, None, &[], None, None)
            .unwrap_err();
        assert!(err.contains("differ from parent"), "{err}");
    }

    #[test]
    fn validate_rejects_thin_packet_and_missing_hypothesis() {
        let thin: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "prompt_packet", "value": "be good", "hypothesis": "h"
        }))
        .unwrap();
        let err =
            validate_proposal(&thin, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("sanity"), "{err}");

        let no_hyp: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "thinking", "value": "high", "hypothesis": " "
        }))
        .unwrap();
        let err =
            validate_proposal(&no_hyp, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("hypothesis"), "{err}");
    }

    #[test]
    fn validate_rejects_degenerate_packet_mutation() {
        let value = format!("The{}", "!".repeat(5000));
        let proposal: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "prompt_packet", "value": value,
            "hypothesis": "try an optimizer-authored replacement packet"
        }))
        .unwrap();
        let err =
            validate_proposal(&proposal, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("sanity"), "{err}");
    }

    #[test]
    fn validate_bounds_thinking() {
        let proposal: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "thinking", "value": "ultra", "hypothesis": "h"
        }))
        .unwrap();
        assert!(
            validate_proposal(&proposal, &parent(), None, None, None, &[], None, None).is_err()
        );
    }

    #[test]
    fn validate_model_must_be_in_search_space() {
        let not_in_space: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "model", "value": "made-up/model", "hypothesis": "h"
        }))
        .unwrap();
        let allowed = vec!["z-ai/glm-5".to_string(), "moonshotai/kimi-k2.6".to_string()];
        let err = validate_proposal(
            &not_in_space,
            &parent(),
            None,
            Some(&allowed),
            None,
            &[],
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("search space"), "{err}");

        let in_space: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "model", "value": "z-ai/glm-5", "hypothesis": "h"
        }))
        .unwrap();
        let (slot, value, _) = validate_proposal(
            &in_space,
            &parent(),
            None,
            Some(&allowed),
            None,
            &[],
            None,
            None,
        )
        .unwrap();
        assert_eq!(slot, "model");
        assert_eq!(value.as_str(), Some("z-ai/glm-5"));
    }

    #[test]
    fn validate_thinking_must_be_in_search_space() {
        let not_in: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "thinking", "value": "high", "hypothesis": "h"
        }))
        .unwrap();
        let allowed = vec!["off".to_string(), "low".to_string(), "medium".to_string()];
        let err = validate_proposal(
            &not_in,
            &parent(),
            None,
            None,
            Some(&allowed),
            &[],
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("search space"), "{err}");

        let in_space: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "thinking", "value": "low", "hypothesis": "h"
        }))
        .unwrap();
        let (slot, value, _) = validate_proposal(
            &in_space,
            &parent(),
            None,
            None,
            Some(&allowed),
            &[],
            None,
            None,
        )
        .unwrap();
        assert_eq!((slot.as_str(), value.as_str()), ("thinking", Some("low")));
    }

    #[test]
    fn proposal_instructions_list_declared_thinking_levels() {
        let allowed = vec!["off".to_string(), "low".to_string(), "medium".to_string()];
        let text = proposal_instructions(None, None, Some(&allowed), &[], None, None, None);
        assert!(!text.contains("\"high\""));
        assert!(text.contains("\"medium\""));
    }

    #[test]
    fn validate_tools_policy_mutation() {
        let p = policies();

        let unknown: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "tools", "value": "yolo", "hypothesis": "h"
        }))
        .unwrap();
        let err = validate_proposal(&unknown, &parent(), Some(&p), None, None, &[], None, None)
            .unwrap_err();
        assert!(err.contains("policy name"), "{err}");

        // "full" is the same as parent → rejected
        let same: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "tools", "value": "full", "hypothesis": "h"
        }))
        .unwrap();
        let err =
            validate_proposal(&same, &parent(), Some(&p), None, None, &[], None, None).unwrap_err();
        assert!(err.contains("differ from parent"), "{err}");

        // No tool_policies → error
        let no_pol: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "tools", "value": "explore", "hypothesis": "h"
        }))
        .unwrap();
        let err =
            validate_proposal(&no_pol, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("tool_policies"), "{err}");

        let ok: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "tools", "value": "explore", "hypothesis": "h"
        }))
        .unwrap();
        let (slot, value, _) =
            validate_proposal(&ok, &parent(), Some(&p), None, None, &[], None, None).unwrap();
        assert_eq!((slot.as_str(), value.as_str()), ("tools", Some("explore")));
    }

    #[test]
    fn validate_system_prompt_mode_mutation() {
        let bad: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "system_prompt_mode", "value": "yolo", "hypothesis": "h"
        }))
        .unwrap();
        let err =
            validate_proposal(&bad, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("append"), "{err}");

        // parent has no explicit mode → defaults to append
        let noop: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "system_prompt_mode", "value": "append", "hypothesis": "h"
        }))
        .unwrap();
        let err =
            validate_proposal(&noop, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("differ from parent"), "{err}");

        let ok: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "system_prompt_mode", "value": "replace", "hypothesis": "h"
        }))
        .unwrap();
        let (slot, value, _) =
            validate_proposal(&ok, &parent(), None, None, None, &[], None, None).unwrap();
        assert_eq!(
            (slot.as_str(), value.as_str()),
            ("system_prompt_mode", Some("replace"))
        );
    }

    #[test]
    fn validate_agents_md_and_build_child_writes_file() {
        let tmp = tempfile_dir();
        let thin: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "agents_md", "value": "hi", "hypothesis": "h"
        }))
        .unwrap();
        let err =
            validate_proposal(&thin, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("substantial"), "{err}");

        let text = "Workspace briefing: trace callers across modules before judging.";
        let ok: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "agents_md", "value": text, "hypothesis": "h"
        }))
        .unwrap();
        let (slot, value, _) =
            validate_proposal(&ok, &parent(), None, None, None, &[], None, None).unwrap();
        let child = build_child(&parent(), &slot, &value, "gen9", &tmp, None, None).unwrap();
        let agents_path = child.get("agents_md").and_then(Value::as_str).unwrap();
        let contents = std::fs::read_to_string(agents_path).unwrap();
        assert!(contents.contains("trace callers"));
        assert_eq!(
            child.get("prompt_packet").and_then(Value::as_str),
            parent().get("prompt_packet").and_then(Value::as_str)
        );
    }

    #[test]
    fn validate_skills_mutation_against_declared_sets() {
        let tmp = tempfile_dir();
        let mut sets = HashMap::new();
        sets.insert(
            "review-pack".to_string(),
            vec!["packets/skill-a.md".to_string()],
        );
        sets.insert("bare".to_string(), vec![]);

        let no_sets: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "skills", "value": "review-pack", "hypothesis": "h"
        }))
        .unwrap();
        let err =
            validate_proposal(&no_sets, &parent(), None, None, None, &[], None, None).unwrap_err();
        assert!(err.contains("skill_sets"), "{err}");

        let bad_name: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "skills", "value": "nonsense", "hypothesis": "h"
        }))
        .unwrap();
        let err = validate_proposal(
            &bad_name,
            &parent(),
            None,
            None,
            None,
            &[],
            Some(&sets),
            None,
        )
        .unwrap_err();
        assert!(err.contains("set name"), "{err}");

        let ok: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "skills", "value": "review-pack", "hypothesis": "h"
        }))
        .unwrap();
        let (slot, value, _) =
            validate_proposal(&ok, &parent(), None, None, None, &[], Some(&sets), None).unwrap();
        let child =
            build_child(&parent(), &slot, &value, "gen10", &tmp, None, Some(&sets)).unwrap();
        assert_eq!(
            child.get("skills").and_then(Value::as_array).unwrap(),
            &vec![Value::String("packets/skill-a.md".into())]
        );
    }

    #[test]
    fn validate_rejects_slot_taken_by_competing_hypothesis() {
        let proposal: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "thinking", "value": "high", "hypothesis": "h"
        }))
        .unwrap();
        let avoid = vec!["thinking".to_string()];
        let err = validate_proposal(&proposal, &parent(), None, None, None, &avoid, None, None)
            .unwrap_err();
        assert!(err.contains("competing"), "{err}");
    }

    #[test]
    fn validate_accepts_good_packet_mutation() {
        let proposal: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "prompt_packet",
            "value": "You are a meticulous reviewer. Always cross-check callers.",
            "hypothesis": "missed cross-file defects"
        }))
        .unwrap();
        let (slot, _value, hyp) =
            validate_proposal(&proposal, &parent(), None, None, None, &[], None, None).unwrap();
        assert_eq!(slot, "prompt_packet");
        assert_eq!(hyp, "missed cross-file defects");
    }

    #[test]
    fn build_child_packet_mutation_roundtrips() {
        let tmp = tempfile_dir();
        let packets_dir = tmp.join("packets");
        let child = build_child(
            &parent(),
            "prompt_packet",
            &Value::String("New packet text that is long enough to be substantial.".into()),
            "gen1-pi-kimi",
            &packets_dir,
            None,
            None,
        )
        .unwrap();
        let manifest_path = write_manifest(&child, &tmp.join("gen1.toml")).unwrap();
        let loaded_str = std::fs::read_to_string(&manifest_path).unwrap();
        // Parse back via Python-compat TOML reader (basic key=value)
        let mut loaded: HashMap<String, String> = HashMap::new();
        for line in loaded_str.lines() {
            if let Some((k, v)) = line.split_once(" = ") {
                loaded.insert(k.trim().to_string(), v.trim_matches('"').to_string());
            }
        }
        assert_eq!(loaded.get("id").map(|s| s.as_str()), Some("gen1-pi-kimi"));
        assert_eq!(loaded.get("kind").map(|s| s.as_str()), Some("pi"));
        let packet_path_str = child.get("prompt_packet").and_then(Value::as_str).unwrap();
        let packet = std::path::Path::new(packet_path_str);
        assert!(packet.exists());
        assert!(std::fs::read_to_string(packet)
            .unwrap()
            .contains("substantial"));
    }

    #[test]
    fn build_child_scalar_mutation() {
        let tmp = tempfile_dir();
        let child = build_child(
            &parent(),
            "thinking",
            &Value::String("high".into()),
            "gen2",
            &tmp,
            None,
            None,
        )
        .unwrap();
        assert_eq!(child.get("thinking").and_then(Value::as_str), Some("high"));
        assert_eq!(
            child.get("prompt_packet").and_then(Value::as_str),
            parent().get("prompt_packet").and_then(Value::as_str)
        );
    }

    #[test]
    fn build_child_tools_mutation_resolves_policy_name() {
        let tmp = tempfile_dir();
        let p = policies();
        let child = build_child(
            &parent(),
            "tools",
            &Value::String("explore".into()),
            "gen3",
            &tmp,
            Some(&p),
            None,
        )
        .unwrap();
        let tools: Vec<&str> = child
            .get("tools")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(tools, vec!["read", "bash"]);
        assert_eq!(
            child.get("model").and_then(Value::as_str),
            parent().get("model").and_then(Value::as_str)
        );
    }

    #[test]
    fn predicted_effect_normalization() {
        // Missing → defaults
        let empty: Map<String, Value> = Map::new();
        let (pe, defaulted) = normalize_predicted_effect(&empty).unwrap();
        assert!(defaulted);
        assert_eq!(pe.get("reward").and_then(Value::as_str), Some("up"));
        assert_eq!(pe.get("cost").and_then(Value::as_str), Some("hold"));

        // Provided
        let with_pe: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "predicted_effect": {"reward": "hold", "cost": "down"}
        }))
        .unwrap();
        let (pe, defaulted) = normalize_predicted_effect(&with_pe).unwrap();
        assert!(!defaulted);
        assert_eq!(pe.get("reward").and_then(Value::as_str), Some("hold"));
        assert_eq!(pe.get("cost").and_then(Value::as_str), Some("down"));

        // Bad value → error
        let bad: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "predicted_effect": {"reward": "sideways", "cost": "down"}
        }))
        .unwrap();
        assert!(normalize_predicted_effect(&bad).is_err());
    }

    #[test]
    fn transplant_resolves_donor_value() {
        let mut archive: HashMap<String, Map<String, Value>> = HashMap::new();
        archive.insert(
            "seed2-cheap".to_string(),
            serde_json::from_value(serde_json::json!({
                "id": "seed2-cheap",
                "kind": "pi",
                "model": "z-ai/glm-4.7-flash",
                "tools": ["read", "bash"],
                "prompt_packet_text": "A winning packet with plenty of substance about tracing callers and citing evidence."
            })).unwrap(),
        );

        let prop: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "model", "donor": "seed2-cheap", "hypothesis": "h"
        }))
        .unwrap();
        let resolved = resolve_donor(&prop, &archive, &parent()).unwrap();
        assert_eq!(
            resolved.get("value").and_then(Value::as_str),
            Some("z-ai/glm-4.7-flash")
        );

        // Packet transplant pulls donor's resolved text
        let prop2: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "prompt_packet", "donor": "seed2-cheap", "hypothesis": "h"
        }))
        .unwrap();
        let resolved2 = resolve_donor(&prop2, &archive, &parent()).unwrap();
        assert!(resolved2
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("winning packet"));

        // Unknown donor → error
        let bad: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "model", "donor": "ghost", "hypothesis": "h"
        }))
        .unwrap();
        let err = resolve_donor(&bad, &archive, &parent()).unwrap_err();
        assert!(err.contains("unknown transplant donor"), "{err}");
    }

    #[test]
    fn transplanted_tools_list_validates_and_builds() {
        let tmp = tempfile_dir();
        let proposal: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "tools", "value": ["read", "bash"], "hypothesis": "h"
        }))
        .unwrap();
        let (slot, value, _) = validate_proposal(
            &proposal,
            &parent(),
            None,
            None,
            None,
            &[],
            None,
            Some("seed2-cheap"),
        )
        .unwrap();
        let child = build_child(&parent(), &slot, &value, "gen11", &tmp, None, None).unwrap();
        let tools: Vec<&str> = child
            .get("tools")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(tools, vec!["read", "bash"]);

        // Transplanting same value as parent → rejected
        let same: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "slot": "tools", "value": ["read", "bash", "edit", "write"], "hypothesis": "h"
        }))
        .unwrap();
        let err = validate_proposal(
            &same,
            &parent(),
            None,
            None,
            None,
            &[],
            None,
            Some("seed2-cheap"),
        )
        .unwrap_err();
        assert!(err.contains("differ from parent"), "{err}");
    }

    #[test]
    fn worst_trials_orders_by_reward() {
        let records: Vec<Map<String, Value>> = serde_json::from_value(serde_json::json!([
            {"candidate_id": "x", "reward": 1.0, "wall_ms": 1, "run_id": "a"},
            {"candidate_id": "x", "reward": 0.0, "wall_ms": 1, "run_id": "b"},
            {"candidate_id": "y", "reward": 0.0, "wall_ms": 1, "run_id": "c"},
            {"candidate_id": "x", "reward": 0.5, "wall_ms": 1, "run_id": "d"}
        ]))
        .unwrap();
        let worst = worst_trials(&records, "x", 2);
        let ids: Vec<&str> = worst
            .iter()
            .filter_map(|r| r.get("run_id").and_then(Value::as_str))
            .collect();
        assert_eq!(ids, vec!["b", "d"]);
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "daedalus-mutate-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
