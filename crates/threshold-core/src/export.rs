//! Control-plane export helpers.
//!
//! Port of `runner/export.py`. Renders a machine-readable `contract.toml`, a
//! Bitter Blossom sprite `persona.md`, and an advisory `plane-handoff.md` from
//! a delivery dir + taskspec.  All render functions are pure (no subprocess
//! calls) so they are fully parity-testable offline.
//!
//! ## Harness version
//!
//! `pi --version` is a live subprocess call (boundary). Callers that need
//! offline parity pass an explicit `harness_version`; the `export_delivery`
//! entry-point accepts `Option<&str>` and falls back to `pi_version()` only
//! when `None`.
//!
//! ## Timestamp handling
//!
//! The Python `generated or datetime.now(...)` idiom becomes
//! `Option<&str>` that falls back to [`pycompat::utc_now_iso`].  The parity
//! test always passes an explicit timestamp so results are deterministic.

use std::path::{Path, PathBuf};

use crate::pycompat::utc_now_iso;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CONTRACT_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Raised when a prompt_packet does not live inside a `runs/<run-id>/` tree,
/// so the export cannot carry evidence pointers.
/// Mirrors Python's `ValueError("delivery prompt_packet must point inside …")`.
#[derive(Debug)]
pub struct ExportError(pub String);

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExportError {}

// ---------------------------------------------------------------------------
// TOML string escaping — mirrors Python's `_toml_str`
// ---------------------------------------------------------------------------

/// `'"' + str(value).replace('\\', '\\\\').replace('"', '\\"') + '"'`
fn toml_str(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

// ---------------------------------------------------------------------------
// Timestamp — mirrors `generated or datetime.now(timezone.utc).strftime(...)`
// ---------------------------------------------------------------------------

fn generated(value: Option<&str>) -> String {
    match value {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => utc_now_iso(),
    }
}

// ---------------------------------------------------------------------------
// `_as_list` — mirrors Python's `_as_list(value)`
// ---------------------------------------------------------------------------

/// Return value as a list: None → []; scalar → [scalar]; list → list.
fn as_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        None | Some(serde_json::Value::Null) => vec![],
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect(),
        Some(other) => {
            let s = other.as_str().unwrap_or("").to_string();
            vec![s]
        }
    }
}

// ---------------------------------------------------------------------------
// `_md_join` — mirrors Python's `_md_join(values)`
// ---------------------------------------------------------------------------

/// Join values as a Markdown-safe comma-separated string, or "-" if empty.
fn md_join(value: Option<&serde_json::Value>) -> String {
    let vals: Vec<String> = as_list(value)
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    if vals.is_empty() {
        "-".to_string()
    } else {
        vals.join(", ")
    }
}

// ---------------------------------------------------------------------------
// `_repo_relative` — mirrors Python's `_repo_relative(path)`
// ---------------------------------------------------------------------------

/// Try to express `path` relative to `cwd`, then relative to `repo_root`,
/// falling back to the string representation.  Mirrors:
/// ```python
/// try: return str(path.resolve().relative_to(Path.cwd().resolve()))
/// except ValueError:
///     try: return str(path.resolve().relative_to(repo_root))
///     except ValueError: return str(path)
/// ```
fn repo_relative(path_str: &str, cwd: &Path, repo_root: &Path) -> String {
    let p = Path::new(path_str);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    let abs = abs.canonicalize().unwrap_or(abs);
    let cwd_abs = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let repo_abs = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    if let Ok(rel) = abs.strip_prefix(&cwd_abs) {
        return rel.to_string_lossy().into_owned();
    }
    if let Ok(rel) = abs.strip_prefix(&repo_abs) {
        return rel.to_string_lossy().into_owned();
    }
    path_str.to_string()
}

// ---------------------------------------------------------------------------
// `evidence_paths` — mirrors Python's `evidence_paths(candidate)`
// ---------------------------------------------------------------------------

/// Infer committed run-record pointers from the measured prompt path.
/// Returns an empty map when the packet is not inside a `runs/<run-id>/` tree.
fn evidence_paths(
    candidate: &serde_json::Map<String, serde_json::Value>,
    cwd: &Path,
    repo_root: &Path,
) -> std::collections::HashMap<String, String> {
    let packet = match candidate.get("prompt_packet").and_then(|v| v.as_str()) {
        Some(p) if !p.is_empty() => p,
        _ => return std::collections::HashMap::new(),
    };

    let path = Path::new(packet);
    let parts: Vec<&str> = path
        .components()
        .map(|c| c.as_os_str().to_str().unwrap_or(""))
        .collect();

    let idx = match parts.iter().position(|&c| c == "runs") {
        Some(i) => i,
        None => return std::collections::HashMap::new(),
    };

    if idx + 1 >= parts.len() {
        return std::collections::HashMap::new();
    }

    // Reconstruct run_dir from components 0..=idx+1
    let run_dir_path: PathBuf = parts[..=idx + 1].iter().collect();
    let rel = repo_relative(&run_dir_path.to_string_lossy(), cwd, repo_root);

    let mut m = std::collections::HashMap::new();
    m.insert("run_dir".to_string(), rel.clone());
    m.insert("report".to_string(), format!("{rel}/report.md"));
    m.insert("lineage".to_string(), format!("{rel}/lineage.md"));
    m.insert("pareto".to_string(), format!("{rel}/pareto.json"));
    m.insert("trials".to_string(), format!("{rel}/trials.jsonl"));
    m.insert("trace".to_string(), format!("{rel}/trace.otel.json"));
    m
}

// ---------------------------------------------------------------------------
// `delivery_name` — mirrors Python's `delivery_name(delivery_dir)`
// ---------------------------------------------------------------------------

fn delivery_name(delivery_dir: &Path) -> String {
    delivery_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".")
        .to_string()
}

// ---------------------------------------------------------------------------
// `approval_prefix` — mirrors Python's `approval_prefix(spec, delivery_dir)`
// ---------------------------------------------------------------------------

fn approval_prefix(spec: &serde_json::Value, delivery_dir: &Path) -> String {
    let task_id = spec
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let task_id = task_id.strip_suffix("-v0").unwrap_or(&task_id).to_string();
    if !task_id.is_empty() {
        task_id
    } else {
        delivery_name(delivery_dir)
    }
}

// ---------------------------------------------------------------------------
// `_incumbent_name` — mirrors Python's `_incumbent_name(data)`
// ---------------------------------------------------------------------------

fn incumbent_name(data: &serde_json::Value) -> String {
    if data.is_null() {
        return "not recorded".to_string();
    }
    let agent = data
        .get("agent")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    match data.get("version").and_then(|v| v.as_str()) {
        Some(v) if !v.is_empty() => format!("{agent} v{v}"),
        _ => agent.to_string(),
    }
}

// ---------------------------------------------------------------------------
// `load_incumbents` — mirrors Python's `load_incumbents(delivery_dir)`
// ---------------------------------------------------------------------------

/// Load optional `plane-incumbents.toml`; return empty object if absent.
pub fn load_incumbents(delivery_dir: &Path) -> serde_json::Value {
    let path = delivery_dir.join("plane-incumbents.toml");
    if !path.exists() {
        return serde_json::Value::Object(serde_json::Map::new());
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str::<toml::Value>(&text) {
            Ok(tv) => crate::run::toml_to_json(tv),
            Err(_) => serde_json::Value::Object(serde_json::Map::new()),
        },
        Err(_) => serde_json::Value::Object(serde_json::Map::new()),
    }
}

// ---------------------------------------------------------------------------
// `pi_version` — mirrors Python's `pi_version()`
// ---------------------------------------------------------------------------

/// Best-effort capture of `pi --version`; returns "unknown" on any failure.
/// This is a live-I/O boundary; parity tests inject an explicit version.
pub fn pi_version() -> String {
    match std::process::Command::new("pi").arg("--version").output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if !stdout.is_empty() {
                stdout
            } else if !stderr.is_empty() {
                stderr
            } else {
                "unknown".to_string()
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// `render_contract` — mirrors Python's `render_contract`
// ---------------------------------------------------------------------------

/// Render the machine-readable `contract.toml` text.
///
/// `harness_version`: injected by caller; pass `pi_version()` for live use.
/// `generated_ts`: `None` falls back to [`utc_now_iso`].
/// `cwd` / `repo_root`: used to relativize evidence paths.
pub fn render_contract(
    candidate: &serde_json::Map<String, serde_json::Value>,
    spec: &serde_json::Value,
    harness_version: &str,
    generated_ts: Option<&str>,
    delivery_dir: &Path,
    cwd: &Path,
    repo_root: &Path,
) -> Result<String, ExportError> {
    let budget = spec
        .get("budget")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let trigger = spec
        .get("trigger")
        .and_then(|t| t.get("intent"))
        .and_then(|v| v.as_str())
        .unwrap_or("manual runs");
    let inputs = spec
        .get("inputs")
        .and_then(|i| i.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let arena = spec
        .get("inputs")
        .and_then(|i| i.get("fixtures"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let output = spec
        .get("output")
        .and_then(|o| o.get("contract"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let env: Vec<String> = match candidate.get("env_allowlist") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect(),
        _ => vec!["OPENROUTER_API_KEY".to_string()],
    };

    let evidence = evidence_paths(candidate, cwd, repo_root);
    if evidence.is_empty() {
        return Err(ExportError(
            "delivery prompt_packet must point inside runs/<run-id>/ so \
the launch contract can carry evidence pointers"
                .to_string(),
        ));
    }

    let gen = generated(generated_ts);

    let tools: String = match candidate.get("tools") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(toml_str)
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };

    let env_s: String = env
        .iter()
        .map(|e| toml_str(e))
        .collect::<Vec<_>>()
        .join(", ");

    let g3_approval = format!(
        "approvals/G3-{}-{}.md",
        approval_prefix(spec, delivery_dir),
        candidate.get("id").and_then(|v| v.as_str()).unwrap_or("")
    );

    let max_cost = budget
        .get("max_cost_per_trial_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let max_wall = budget
        .get("max_wall_per_trial_sec")
        .and_then(|v| v.as_f64())
        .unwrap_or(600.0);

    // Python: str(0.5) = "0.5", str(600) = "600" (int), str(600.0) = "600.0"
    let max_cost_str = format_py_number(
        spec.get("budget")
            .and_then(|b| b.get("max_cost_per_trial_usd")),
        max_cost,
    );
    let max_wall_str = format_py_number(
        spec.get("budget")
            .and_then(|b| b.get("max_wall_per_trial_sec")),
        max_wall,
    );

    let agent_id = candidate.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let hash = candidate
        .get("_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let taskspec_id = spec.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let mode = spec.get("mode").and_then(|v| v.as_str()).unwrap_or("");
    let kind = candidate.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    let provider = candidate
        .get("provider_name")
        .and_then(|v| v.as_str())
        .unwrap_or("openrouter");
    let model = candidate
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let thinking = candidate
        .get("thinking")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let prompt_packet = candidate
        .get("prompt_packet")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let prompt_packet_ref = repo_relative(prompt_packet, cwd, repo_root);
    let system_prompt_mode = candidate
        .get("system_prompt_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("append");
    let timeout_sec = candidate
        .get("timeout_sec")
        .and_then(|v| v.as_i64())
        .unwrap_or(600);

    let ev_run_dir = evidence.get("run_dir").map(String::as_str).unwrap_or("");
    let ev_report = evidence.get("report").map(String::as_str).unwrap_or("");
    let ev_lineage = evidence.get("lineage").map(String::as_str).unwrap_or("");
    let ev_pareto = evidence.get("pareto").map(String::as_str).unwrap_or("");
    let ev_trials = evidence.get("trials").map(String::as_str).unwrap_or("");
    let ev_trace = evidence.get("trace").map(String::as_str).unwrap_or("");

    Ok(format!(
        "\
# Launch contract — generated by threshold export; do not hand-edit fields
# that are pinned to evidence (composition_hash, harness_version).
contract = {CONTRACT_VERSION}
generated = {gen_s}
agent = {agent_s}
composition_hash = {hash_s}
taskspec = {taskspec_s}
mode = {mode_s}

[composition]
harness = {kind_s}
harness_version = {harness_s}
provider = {provider_s}
model = {model_s}
thinking = {thinking_s}
tools = [{tools}]
prompt_packet = {prompt_packet_s}
system_prompt_mode = {spm_s}
timeout_sec = {timeout_sec}

[trigger]
intent = {trigger_s}

[inputs]
description = {inputs_s}

[output]
contract = {output_s}

[permissions]
workspace = \"read-only checkout; writes only the output artifact in a throwaway workdir\"
env = [{env_s}]
write_actions = \"none\"

[budgets]
max_cost_usd_per_run = {max_cost_str}
max_wall_sec = {max_wall_str}

[escalation]
on_malformed_output = \"emit nothing; flag the run for human review\"
on_timeout = \"emit nothing; flag the run for human review\"

[observability]
regression_eval = \"re-run the arena holdout on any packet/model/harness change, and on a monthly cadence\"
arena = {arena_s}
trace_artifact = {trace_s}
trace_destination = \"JSONL-only waiver: committed trials.jsonl remains canonical; runner/trace.py emits trace.otel.json until a live sink is signed.\"

[evidence]
run_dir = {ev_run_dir_s}
report = {ev_report_s}
lineage = {ev_lineage_s}
pareto = {ev_pareto_s}
trials = {ev_trials_s}

[approval]
g3_signed = false
g3_approval = {g3_approval_s}
note = \"Do not deploy as a primary reviewer until G3 is signed by a human; unsigned contracts may only produce sandbox dry-run packets.\"
",
        gen_s = toml_str(&gen),
        agent_s = toml_str(agent_id),
        hash_s = toml_str(hash),
        taskspec_s = toml_str(taskspec_id),
        mode_s = toml_str(mode),
        kind_s = toml_str(kind),
        harness_s = toml_str(harness_version),
        provider_s = toml_str(provider),
        model_s = toml_str(model),
        thinking_s = toml_str(thinking),
        tools = tools,
        prompt_packet_s = toml_str(&prompt_packet_ref),
        spm_s = toml_str(system_prompt_mode),
        timeout_sec = timeout_sec,
        trigger_s = toml_str(trigger),
        inputs_s = toml_str(inputs),
        output_s = toml_str(output),
        env_s = env_s,
        max_cost_str = max_cost_str,
        max_wall_str = max_wall_str,
        arena_s = toml_str(arena),
        trace_s = toml_str(ev_trace),
        ev_run_dir_s = toml_str(ev_run_dir),
        ev_report_s = toml_str(ev_report),
        ev_lineage_s = toml_str(ev_lineage),
        ev_pareto_s = toml_str(ev_pareto),
        ev_trials_s = toml_str(ev_trials),
        g3_approval_s = toml_str(&g3_approval),
    ))
}

/// Format a numeric JSON value the same way Python's `str(value)` does:
/// integer stays as integer ("600"), float stays as float ("0.5").
fn format_py_number(raw: Option<&serde_json::Value>, fallback: f64) -> String {
    match raw {
        Some(serde_json::Value::Number(n)) => {
            if n.is_i64() || n.is_u64() {
                n.to_string()
            } else {
                match n.as_f64() {
                    Some(f) => format_float_py(f),
                    None => fallback.to_string(),
                }
            }
        }
        _ => format_float_py(fallback),
    }
}

/// Format a float the way Python's `str(float)` does.
/// Python always shows at least one decimal: 0.5 → "0.5", 600.0 → "600.0".
fn format_float_py(f: f64) -> String {
    let s = format!("{f}");
    if s.contains('.')
        || s.contains('e')
        || s.contains('E')
        || s.contains("nan")
        || s.contains("inf")
    {
        s
    } else {
        format!("{s}.0")
    }
}

// ---------------------------------------------------------------------------
// `render_persona` — mirrors Python's `render_persona`
// ---------------------------------------------------------------------------

/// Render the Bitter Blossom sprite `persona.md`.
pub fn render_persona(
    candidate: &serde_json::Map<String, serde_json::Value>,
    spec: &serde_json::Value,
) -> String {
    let goal = spec
        .get("goal")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let description = if goal.len() <= 160 {
        goal.clone()
    } else {
        format!("{}...", &goal[..157])
    };

    let skills_lines: Vec<String> = match candidate.get("skills") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| format!("  - {s}"))
            .collect(),
        _ => vec![],
    };
    let skills_block = if skills_lines.is_empty() {
        "skills: []".to_string()
    } else {
        format!("skills:\n{}", skills_lines.join("\n"))
    };

    let agent_id = candidate.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let provider = candidate
        .get("provider_name")
        .and_then(|v| v.as_str())
        .unwrap_or("openrouter");
    let model = candidate
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let hash = candidate
        .get("_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let body = candidate
        .get("_packet_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    format!(
        "---\nname: {agent_id}\ndescription: \"{description}\"\nmodel: {provider}/{model}\n{skills_block}\nthreshold:\n  composition_hash: {hash}\n  contract: contract.toml\n---\n\n{body}"
    )
}

// ---------------------------------------------------------------------------
// `render_handoff` — mirrors Python's `render_handoff`
// ---------------------------------------------------------------------------

/// Render the human-reviewable `plane-handoff.md`.
pub fn render_handoff(
    candidate: &serde_json::Map<String, serde_json::Value>,
    spec: &serde_json::Value,
    harness_version: &str,
    generated_ts: Option<&str>,
    incumbents: &serde_json::Value,
    delivery_dir: &Path,
) -> String {
    let gen = generated(generated_ts);

    let budget = spec
        .get("budget")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let output = spec
        .get("output")
        .and_then(|o| o.get("contract"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let trigger = spec
        .get("trigger")
        .and_then(|t| t.get("intent"))
        .and_then(|v| v.as_str())
        .unwrap_or("manual runs");

    let tools: String = match candidate.get("tools") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };

    let env: Vec<String> = match candidate.get("env_allowlist") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect(),
        _ => vec!["OPENROUTER_API_KEY".to_string()],
    };
    let env_str = env.join(", ");

    let delivery_ref = format!("deliveries/{}", delivery_name(delivery_dir));

    let bb = incumbents
        .get("bitter_blossom")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let olympus = incumbents
        .get("olympus")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let agent_id = candidate.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let hash = candidate
        .get("_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let taskspec_id = spec.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let mode = spec.get("mode").and_then(|v| v.as_str()).unwrap_or("");
    let kind = candidate.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    let provider = candidate
        .get("provider_name")
        .and_then(|v| v.as_str())
        .unwrap_or("openrouter");
    let model = candidate
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let thinking = candidate
        .get("thinking")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let prompt_packet = candidate
        .get("prompt_packet")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let max_cost = budget
        .get("max_cost_per_trial_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let max_wall = budget
        .get("max_wall_per_trial_sec")
        .and_then(|v| v.as_i64())
        .unwrap_or(600);

    let bb_name = incumbent_name(&bb);
    let bb_model = bb.get("model").and_then(|v| v.as_str()).unwrap_or("-");
    let bb_posting = bb.get("posting").and_then(|v| v.as_str()).unwrap_or("-");
    let bb_config = md_join(bb.get("config_paths"));

    let ol_name = incumbent_name(&olympus);
    let ol_model = olympus.get("model").and_then(|v| v.as_str()).unwrap_or("-");
    let ol_posting = olympus
        .get("posting")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let ol_config = md_join(olympus.get("config_paths"));

    let bb_tools: String = match candidate.get("tools") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(toml_str)
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };
    let bb_secrets: String = env
        .iter()
        .map(|e| toml_str(e))
        .collect::<Vec<_>>()
        .join(", ");

    let mut lines: Vec<String> = vec![
        format!("# Cross-plane handoff: {agent_id}"),
        String::new(),
        format!("Generated: `{gen}`"),
        String::new(),
        "Lab evidence is not launch approval. This packet is import guidance \
for humans and control-plane dry runs; G3/G4/G5 approval still gates \
deployment, write authority, and production-data re-ingestion. \
Unsigned use is sandbox-only and must not operate as a primary \
reviewer."
            .to_string(),
        String::new(),
        "## Certified composition identity".to_string(),
        String::new(),
        "| field | value |".to_string(),
        "|---|---|".to_string(),
        format!("| agent | `{agent_id}` |"),
        format!("| composition hash | `{hash}` |"),
        format!("| taskspec | `{taskspec_id}` |"),
        format!("| mode | `{mode}` |"),
        format!("| harness | `{kind}` (`{harness_version}`) |"),
        format!("| provider/model | `{provider}/{model}` |"),
        format!("| thinking | `{thinking}` |"),
        format!("| tools | `{tools}` |"),
        format!("| prompt packet | `{prompt_packet}` |"),
        format!("| output contract | `{output}` |"),
        format!("| trigger intent | `{trigger}` |"),
        format!("| budget | `${max_cost}` and `{max_wall}s` per run |"),
        format!("| env | `{env_str}` |"),
        "| approval | `g3_signed = false` until a human signs the launch gate |".to_string(),
        String::new(),
        "## Incumbent comparison".to_string(),
        String::new(),
        "| plane | current incumbent | model | posting / output boundary | \
config surfaces | import delta |"
            .to_string(),
        "|---|---|---|---|---|---|".to_string(),
        format!("| Bitter Blossom | {bb_name} | `{bb_model}` | {bb_posting} | {bb_config} | Replace or overlay agent/persona fields from this packet; preserve task filters, dedupe, budgets, and HMAC ingress. |"),
        format!("| Olympus | {ol_name} | `{ol_model}` | {ol_posting} | {ol_config} | Replace or overlay AgentSpec runtime/model/prompt fields from this packet; preserve activation gating, strict artifact validation, duplicate suppression, and orchestrator-side posting. |"),
        String::new(),
        "## Bitter Blossom import shape".to_string(),
        String::new(),
        "Map the measured composition into `plane/agents/` and keep the review \
task's existing trigger/filter/budget guardrails unless a G3 launch \
approval says otherwise."
            .to_string(),
        String::new(),
        "```toml".to_string(),
        format!("# plane/agents/{agent_id}.toml"),
        format!("id = \"{agent_id}\""),
        "version = 1".to_string(),
        "harness = \"pi\"".to_string(),
        format!("provider = \"{provider}\""),
        format!("model = \"{model}\""),
        format!("thinking = \"{thinking}\""),
        format!("composition_hash = \"{hash}\""),
        "contract = \"contract.toml\"".to_string(),
        "persona = \"persona.md\"".to_string(),
        format!("tools = [{bb_tools}]"),
        format!("secrets = [{bb_secrets}]"),
        "```".to_string(),
        String::new(),
        "- If Bitter Blossom keeps direct posting or workflow side effects, \
the task card must retain the no-approval/no-write red lines and the \
measured prompt packet must remain byte-identical."
            .to_string(),
        "- Preferred safer import: keep the measured review persona, have the \
agent emit the structured findings contract, and let the plane own \
formatting/posting after G3."
            .to_string(),
        "- Before G3, any Bitter Blossom run must be sandboxed and secondary \
to the existing review path; it is evidence for Threshold, not an \
enterprise-ready reviewer deployment."
            .to_string(),
        String::new(),
        "## Olympus AgentSpec import shape".to_string(),
        String::new(),
        "Map the same measured composition into an AgentSpec without \
weakening Olympus' control-plane-owned validation/posting \
boundary."
            .to_string(),
        String::new(),
        "```yaml".to_string(),
        "id: <target-agent-id>".to_string(),
        "version: <human-bumped>".to_string(),
        "runtime: pi".to_string(),
        format!("model: {model}"),
        format!("provider: {provider}"),
        format!("thinking: {thinking}"),
        format!("prompt_ref: {delivery_ref}/persona.md"),
        format!("composition_hash: {hash}"),
        format!("contract_ref: {delivery_ref}/contract.toml"),
        "output_contract: strict findings artifact, then orchestrator review \
posting"
            .to_string(),
        "budgets:".to_string(),
        format!("  max_cost_usd_per_run: {max_cost}"),
        format!("  max_wall_sec: {max_wall}"),
        "activation:".to_string(),
        "  g3_signed: false".to_string(),
        "```".to_string(),
        String::new(),
        "- Preserve pinned input checkout, untrusted event metadata handling, \
output caps, artifact validation, duplicate suppression, and \
control-plane posting."
            .to_string(),
        "- Treat this packet as an AgentSpec overlay candidate, not an \
automatic replacement for the live Charon config."
            .to_string(),
        String::new(),
        "## Residual risks and next gates".to_string(),
        String::new(),
        "- Exact replay against incumbents may be impossible when their prompts, \
posting contract, runtime wrappers, or model aliases do not map to a \
Threshold composition slot; record that in the run report instead of \
pretending parity."
            .to_string(),
        "- G3 decides whether either plane imports this packet.".to_string(),
        "- G4 is required before any production write authority expands beyond \
advisory review output."
            .to_string(),
        "- G5 is required before production traces or PR data flow back into \
arena fixtures."
            .to_string(),
        "- This handoff is not a public benchmark-quality claim; keep the \
G2 calibration caveats attached until a stronger arena version \
supersedes them."
            .to_string(),
        String::new(),
    ];

    // Append incumbent notes if present — mirrors Python's loop over
    // (("bitter_blossom", "Bitter Blossom"), ("olympus", "Olympus"))
    for (key, title) in &[("bitter_blossom", "Bitter Blossom"), ("olympus", "Olympus")] {
        let notes = as_list(incumbents.get(*key).and_then(|v| v.get("notes")));
        let notes: Vec<String> = notes.into_iter().filter(|n| !n.is_empty()).collect();
        if !notes.is_empty() {
            lines.push(format!("### {title} incumbent notes"));
            lines.push(String::new());
            for n in &notes {
                lines.push(format!("- {n}"));
            }
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// `export_delivery` — mirrors Python's `export_delivery`
// ---------------------------------------------------------------------------

/// Write control-plane artifacts next to the delivery's `agent.toml`.
///
/// Returns `{contract, persona, handoff}` paths.
///
/// `harness_version`: `None` falls back to [`pi_version`].
/// `generated_ts`: `None` falls back to [`utc_now_iso`].
/// `repo_root`: the repository root used for evidence-path relativization and
///   `load_candidate` file resolution.
pub fn export_delivery(
    delivery_dir: &Path,
    spec: &serde_json::Value,
    harness_version: Option<&str>,
    generated_ts: Option<&str>,
    repo_root: &Path,
) -> Result<std::collections::HashMap<String, PathBuf>, Box<dyn std::error::Error>> {
    let hv_owned;
    let hv = match harness_version {
        Some(v) => v,
        None => {
            hv_owned = pi_version();
            &hv_owned
        }
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| repo_root.to_path_buf());

    // mirrors Python: candidate = load_candidate(delivery_dir / "agent.toml")
    let manifest = delivery_dir.join("agent.toml");
    let candidate = crate::run::load_candidate(&manifest, repo_root)?;

    let contract_text = render_contract(
        &candidate,
        spec,
        hv,
        generated_ts,
        delivery_dir,
        &cwd,
        repo_root,
    )?;

    // mirrors Python: tomllib.loads(contract_text)  # the exported artifact must parse
    toml::from_str::<toml::Value>(&contract_text)
        .map_err(|e| ExportError(format!("rendered contract.toml is invalid TOML: {e}")))?;

    let contract_path = delivery_dir.join("contract.toml");
    std::fs::write(&contract_path, &contract_text)?;

    let persona_path = delivery_dir.join("persona.md");
    std::fs::write(&persona_path, render_persona(&candidate, spec))?;

    let incumbents = load_incumbents(delivery_dir);
    let handoff_path = delivery_dir.join("plane-handoff.md");
    std::fs::write(
        &handoff_path,
        render_handoff(
            &candidate,
            spec,
            hv,
            generated_ts,
            &incumbents,
            delivery_dir,
        ),
    )?;

    let mut paths = std::collections::HashMap::new();
    paths.insert("contract".to_string(), contract_path);
    paths.insert("persona".to_string(), persona_path);
    paths.insert("handoff".to_string(), handoff_path);
    Ok(paths)
}

// ---------------------------------------------------------------------------
// Unit tests (port of tests/test_export.py)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("repo root")
            .to_path_buf()
    }

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "threshold-export-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const SPEC_JSON: &str = r#"{
        "id": "pr-review-v0",
        "goal": "Find the real defects a change introduces.",
        "mode": "threshold-then-cheap",
        "inputs": {"description": "post-change repo + PR.diff", "fixtures": "arenas/pr-review-v2"},
        "output": {"contract": "findings.json"},
        "budget": {"max_cost_per_trial_usd": 0.5, "max_wall_per_trial_sec": 600},
        "trigger": {"intent": "GitHub PR webhook"}
    }"#;

    fn build_delivery(tmp_path: &Path) -> PathBuf {
        let packet = tmp_path
            .join("runs")
            .join("20260611T000000Z-demo")
            .join("packets")
            .join("packet.md");
        std::fs::create_dir_all(packet.parent().unwrap()).unwrap();
        std::fs::write(&packet, "Review with evidence. Cite file and line. Stop.\n").unwrap();

        std::fs::write(
            tmp_path.join("agent.toml"),
            format!(
                "composition = 1\nid = \"demo-agent\"\nkind = \"pi\"\n\
provider_name = \"openrouter\"\nmodel = \"z-ai/glm-5\"\n\
prompt_packet = \"{}\"\nthinking = \"medium\"\n\
tools = [\"read\", \"bash\"]\ntimeout_sec = 600\n",
                packet.display()
            ),
        )
        .unwrap();
        tmp_path.to_path_buf()
    }

    #[test]
    fn export_writes_parseable_contract_and_faithful_persona() {
        let d = tmpdir("write-contract");
        let delivery = build_delivery(&d);
        let spec: serde_json::Value = serde_json::from_str(SPEC_JSON).unwrap();
        let repo = repo_root();

        let paths = export_delivery(
            &delivery,
            &spec,
            Some("9.9.9"),
            Some("2026-06-10T00:00:00Z"),
            &repo,
        )
        .expect("export_delivery should succeed");

        let contract_text = std::fs::read_to_string(&paths["contract"]).unwrap();
        let contract: toml::Value = toml::from_str(&contract_text).unwrap();
        let c = contract.as_table().unwrap();

        assert_eq!(c["contract"].as_integer(), Some(1));
        assert_eq!(c["agent"].as_str(), Some("demo-agent"));
        assert_eq!(c["composition"]["harness_version"].as_str(), Some("9.9.9"));
        assert_eq!(c["composition"]["model"].as_str(), Some("z-ai/glm-5"));
        assert!((c["budgets"]["max_cost_usd_per_run"].as_float().unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(c["trigger"]["intent"].as_str(), Some("GitHub PR webhook"));
        assert_eq!(c["approval"]["g3_signed"].as_bool(), Some(false));
        assert_eq!(
            c["approval"]["g3_approval"].as_str(),
            Some("approvals/G3-pr-review-demo-agent.md")
        );

        let ev_run_dir = c["evidence"]["run_dir"].as_str().unwrap();
        assert!(
            ev_run_dir.ends_with("runs/20260611T000000Z-demo"),
            "evidence.run_dir = {ev_run_dir}"
        );
        let ev_report = c["evidence"]["report"].as_str().unwrap();
        assert!(
            ev_report.ends_with("runs/20260611T000000Z-demo/report.md"),
            "evidence.report = {ev_report}"
        );
        let trace = c["observability"]["trace_artifact"].as_str().unwrap();
        assert!(
            trace.ends_with("runs/20260611T000000Z-demo/trace.otel.json"),
            "trace_artifact = {trace}"
        );
        assert!(c["observability"]["trace_destination"]
            .as_str()
            .unwrap()
            .contains("JSONL-only waiver"));

        let persona = std::fs::read_to_string(&paths["persona"]).unwrap();
        assert!(persona.contains("name: demo-agent"));
        assert!(persona.contains("model: openrouter/z-ai/glm-5"));
        let cand = crate::run::load_candidate(&delivery.join("agent.toml"), &repo).unwrap();
        let hash = cand["_hash"].as_str().unwrap();
        assert!(persona.contains(&format!("composition_hash: {hash}")));
        let (_, body) = persona.split_once("---\n\n").unwrap();
        let expected_body = cand["_packet_text"].as_str().unwrap();
        assert_eq!(body, expected_body);

        let handoff = std::fs::read_to_string(&paths["handoff"]).unwrap();
        assert!(handoff.contains("Bitter Blossom import shape"));
        assert!(handoff.contains("Olympus AgentSpec import shape"));
        assert!(handoff.contains(&format!("composition hash | `{hash}`")));
        assert!(handoff.contains("prompt_ref: deliveries/"));
        assert!(handoff.contains("Lab evidence is not launch approval"));

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn render_contract_relativizes_repo_local_prompt_packet() {
        let d = tmpdir("relative-prompt-packet");
        let packet = d
            .join("runs")
            .join("20260611T000000Z-demo")
            .join("packets")
            .join("packet.md");
        std::fs::create_dir_all(packet.parent().unwrap()).unwrap();
        std::fs::write(&packet, "Review with evidence. Cite file and line. Stop.\n").unwrap();

        let candidate = json!({
            "id": "demo-agent",
            "_hash": "abc123",
            "kind": "pi",
            "provider_name": "openrouter",
            "model": "z-ai/glm-5",
            "thinking": "medium",
            "tools": ["read", "bash"],
            "prompt_packet": packet.to_str().unwrap(),
            "system_prompt_mode": "append",
            "timeout_sec": 600
        });
        let spec: serde_json::Value = serde_json::from_str(SPEC_JSON).unwrap();
        let text = render_contract(
            candidate.as_object().unwrap(),
            &spec,
            "9.9.9",
            Some("2026-06-10T00:00:00Z"),
            &d,
            &d,
            &d,
        )
        .unwrap();
        let contract: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(
            contract["composition"]["prompt_packet"].as_str(),
            Some("runs/20260611T000000Z-demo/packets/packet.md")
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn export_is_deterministic() {
        let d = tmpdir("deterministic");
        let delivery = build_delivery(&d);
        let spec: serde_json::Value = serde_json::from_str(SPEC_JSON).unwrap();
        let repo = repo_root();

        let a = export_delivery(
            &delivery,
            &spec,
            Some("9.9.9"),
            Some("2026-06-10T00:00:00Z"),
            &repo,
        )
        .unwrap();
        let first_contract = std::fs::read_to_string(&a["contract"]).unwrap();
        let first_handoff = std::fs::read_to_string(&a["handoff"]).unwrap();

        let b = export_delivery(
            &delivery,
            &spec,
            Some("9.9.9"),
            Some("2026-06-10T00:00:00Z"),
            &repo,
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&b["contract"]).unwrap(),
            first_contract
        );
        assert_eq!(
            std::fs::read_to_string(&b["handoff"]).unwrap(),
            first_handoff
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn export_requires_evidence_backed_prompt_packet() {
        let d = tmpdir("no-evidence");
        let delivery = build_delivery(&d);
        let spec: serde_json::Value = serde_json::from_str(SPEC_JSON).unwrap();
        let repo = repo_root();

        let loose = d.join("loose-packet.md");
        std::fs::write(&loose, "No run evidence.\n").unwrap();
        let old = std::fs::read_to_string(delivery.join("agent.toml")).unwrap();
        let runs_packet = delivery
            .join("runs")
            .join("20260611T000000Z-demo")
            .join("packets")
            .join("packet.md");
        let new_content = old.replace(runs_packet.to_str().unwrap(), loose.to_str().unwrap());
        std::fs::write(delivery.join("agent.toml"), &new_content).unwrap();

        let result = export_delivery(&delivery, &spec, Some("9.9.9"), None, &repo);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("evidence pointers"),
            "expected 'evidence pointers' in: {msg}"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn export_handoff_includes_incumbent_comparison() {
        let d = tmpdir("incumbents");
        let delivery = build_delivery(&d);
        let spec: serde_json::Value = serde_json::from_str(SPEC_JSON).unwrap();
        let repo = repo_root();

        std::fs::write(
            delivery.join("plane-incumbents.toml"),
            r#"
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
"#,
        )
        .unwrap();

        let paths = export_delivery(
            &delivery,
            &spec,
            Some("9.9.9"),
            Some("2026-06-10T00:00:00Z"),
            &repo,
        )
        .unwrap();
        let text = std::fs::read_to_string(&paths["handoff"]).unwrap();

        assert!(text.contains("review-coordinator v2"), "text = {text}");
        assert!(text.contains("moonshotai/kimi-k2.6"));
        assert!(text.contains("charon v2"));
        assert!(text.contains("~moonshotai/kimi-latest"));
        assert!(text.contains("plane/agents/review-coordinator.toml"));
        assert!(text.contains("orchestrator/agent-specs/charon.yaml"));
        assert!(text.contains("G3/G4/G5"));

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn export_uses_delivery_and_task_identity_for_non_pr_review() {
        let d = tmpdir("non-pr-review");
        let delivery = build_delivery(&d.join("launch-contract"));
        let spec: serde_json::Value = json!({
            "id": "launch-contract-v0",
            "goal": "Review launch contracts.",
            "mode": "threshold-then-cheap",
            "inputs": {"description": "launch packet", "fixtures": "arenas/launch-contract-v0"},
            "output": {"contract": "findings.json"},
            "budget": {"max_cost_per_trial_usd": 0.5, "max_wall_per_trial_sec": 600},
            "trigger": {"intent": "Manual launch-contract review before G3/G4/G5 approval"}
        });
        let repo = repo_root();

        let paths = export_delivery(
            &delivery,
            &spec,
            Some("9.9.9"),
            Some("2026-06-10T00:00:00Z"),
            &repo,
        )
        .unwrap();

        let contract: toml::Value =
            toml::from_str(&std::fs::read_to_string(&paths["contract"]).unwrap()).unwrap();
        assert_eq!(
            contract["approval"]["g3_approval"].as_str(),
            Some("approvals/G3-launch-contract-demo-agent.md")
        );

        let handoff = std::fs::read_to_string(&paths["handoff"]).unwrap();
        assert!(handoff.contains("prompt_ref: deliveries/launch-contract/persona.md"));
        assert!(handoff.contains("contract_ref: deliveries/launch-contract/contract.toml"));

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn toml_str_escapes_backslash_and_quote() {
        assert_eq!(toml_str("hello"), r#""hello""#);
        assert_eq!(toml_str(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(toml_str(r"back\slash"), r#""back\\slash""#);
    }

    #[test]
    fn format_float_py_matches_python() {
        assert_eq!(format_float_py(0.5), "0.5");
        assert_eq!(format_float_py(600.0), "600.0");
        assert_eq!(format_float_py(1.0), "1.0");
    }
}
