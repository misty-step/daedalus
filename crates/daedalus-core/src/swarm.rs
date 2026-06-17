//! Review-swarm delivery contracts.
//!
//! Port of `runner/swarm.py`. `swarm-contract.v1` is the suite-level
//! counterpart to a single-agent launch contract. It does not deploy agents and
//! it does not invent run evidence: export requires a summary artifact that
//! records measured cost, wall time, master replay status, and handoff mode.
//!
//! ## Timestamp handling
//!
//! The Python `_generated(value=None)` idiom — fall back to
//! `datetime.now(timezone.utc).strftime(…)` — becomes an `Option<&str>` param
//! that falls back to [`pycompat::utc_now_iso`]. The parity test always passes
//! an explicit timestamp so results are deterministic; the fallback is
//! wall-clock only.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::pycompat::utc_now_iso;

/// Contract schema version constant (mirrors `SWARM_CONTRACT_VERSION = 1`).
pub const SWARM_CONTRACT_VERSION: u64 = 1;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Raised when a review-swarm delivery is malformed.
/// Mirrors Python's `SwarmValidationError(RuntimeError)`.
#[derive(Debug)]
pub struct SwarmValidationError(pub String);

impl std::fmt::Display for SwarmValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SwarmValidationError {}

macro_rules! bail {
    ($($arg:tt)*) => {
        return Err(SwarmValidationError(format!($($arg)*)))
    };
}

// ---------------------------------------------------------------------------
// TOML string escaping — mirrors Python's `_toml_str`
// ---------------------------------------------------------------------------

/// Replicate Python's `_toml_str(value)`:
/// `'"' + str(value).replace('\\', '\\\\').replace('"', '\\"') + '"'`
fn toml_str(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

// ---------------------------------------------------------------------------
// Timestamp fallback — mirrors Python's `_generated(value=None)`
// ---------------------------------------------------------------------------

/// Return `value` if `Some`; otherwise call [`utc_now_iso`].
/// Mirrors `_generated(value=None)`.
fn generated(value: Option<&str>) -> String {
    match value {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => utc_now_iso(),
    }
}

// ---------------------------------------------------------------------------
// Path resolution — mirrors Python's `_resolve_ref`
// ---------------------------------------------------------------------------

/// Resolve a ref string against the repo root and optional delivery dir.
/// Mirrors `_resolve_ref(ref, delivery_dir)`.
///
/// `repo` is passed in explicitly instead of using `REPO` (a compile-time
/// constant cannot refer to `__file__`). Callers compute it via
/// `env!("CARGO_MANIFEST_DIR")` two levels up.
fn resolve_ref(ref_: &str, delivery_dir: Option<&Path>, repo: &Path) -> PathBuf {
    let path = Path::new(ref_);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if let Some(dd) = delivery_dir {
        let local = dd.join(path);
        if local.exists() {
            return local;
        }
    }
    repo.join(path)
}

// ---------------------------------------------------------------------------
// Require helpers — mirror Python's `_require_*` family
// ---------------------------------------------------------------------------

fn require_number(value: Option<&Value>, label: &str) -> Result<f64, SwarmValidationError> {
    match value {
        Some(Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                return Ok(f);
            }
            bail!("{label} must be numeric")
        }
        _ => bail!("{label} must be numeric"),
    }
}

fn require_text<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<&'a str, SwarmValidationError> {
    match value {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.as_str()),
        _ => bail!("{label} must be a non-empty string"),
    }
}

fn require_table<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<&'a serde_json::Map<String, Value>, SwarmValidationError> {
    match value {
        Some(Value::Object(m)) => Ok(m),
        _ => bail!("{label} must be a table"),
    }
}

fn require_existing_file(
    ref_val: Option<&Value>,
    label: &str,
    delivery_dir: Option<&Path>,
    repo: &Path,
) -> Result<PathBuf, SwarmValidationError> {
    let ref_str = require_text(ref_val, label)?;
    let path = resolve_ref(ref_str, delivery_dir, repo);
    if !path.is_file() {
        bail!("{label} does not exist: {ref_str}");
    }
    Ok(path)
}

fn require_existing_dir(
    ref_val: Option<&Value>,
    label: &str,
    delivery_dir: Option<&Path>,
    repo: &Path,
) -> Result<PathBuf, SwarmValidationError> {
    let ref_str = require_text(ref_val, label)?;
    let path = resolve_ref(ref_str, delivery_dir, repo);
    if !path.is_dir() {
        bail!("{label} does not exist: {ref_str}");
    }
    Ok(path)
}

// ---------------------------------------------------------------------------
// Composition-hash validation — mirrors `_validate_composition_hash`
// ---------------------------------------------------------------------------

fn validate_composition_hash(
    record: &serde_json::Map<String, Value>,
    run_dir: &Path,
    label: &str,
) -> Result<(), SwarmValidationError> {
    let expected = require_text(
        record.get("composition_hash"),
        &format!("{label}.composition_hash"),
    )?;
    let comp_dir = run_dir.join("compositions");
    if !comp_dir.is_dir() {
        bail!("{label}.evidence.run_dir has no compositions/");
    }
    let entries = std::fs::read_dir(&comp_dir)
        .map_err(|e| SwarmValidationError(format!("cannot read {}: {e}", comp_dir.display())))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(payload) = serde_json::from_str::<Value>(&text) {
                if payload.get("composition_hash").and_then(Value::as_str) == Some(expected) {
                    return Ok(());
                }
            }
        }
    }
    bail!("{label}.composition_hash not found in run compositions");
}

// ---------------------------------------------------------------------------
// Evidence-record validation — mirrors `_validate_evidence_record`
// ---------------------------------------------------------------------------

fn validate_evidence_record(
    record: &serde_json::Map<String, Value>,
    label: &str,
    delivery_dir: Option<&Path>,
    repo: &Path,
) -> Result<(), SwarmValidationError> {
    let evidence = require_table(record.get("evidence"), &format!("{label}.evidence"))?;
    let run_dir = require_existing_dir(
        evidence.get("run_dir"),
        &format!("{label}.evidence.run_dir"),
        delivery_dir,
        repo,
    )?;
    require_existing_file(
        evidence.get("trials"),
        &format!("{label}.evidence.trials"),
        delivery_dir,
        repo,
    )?;
    validate_composition_hash(record, &run_dir, label)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Member-records validation — mirrors `_validate_member_records`
// ---------------------------------------------------------------------------

fn validate_member_records(
    summary: &serde_json::Map<String, Value>,
    suite_spec: &serde_json::Map<String, Value>,
    delivery_dir: Option<&Path>,
    repo: &Path,
) -> Result<(), SwarmValidationError> {
    let spec_suite = suite_spec
        .get("suite")
        .and_then(Value::as_object)
        .map(|m| m as &serde_json::Map<String, Value>);

    let members = require_table(summary.get("members"), "members")?;

    let required: Vec<String> = spec_suite
        .and_then(|s| s.get("required_members"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let optional: Vec<String> = spec_suite
        .and_then(|s| s.get("optional_members"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let known: std::collections::HashSet<&str> = required
        .iter()
        .chain(optional.iter())
        .map(String::as_str)
        .collect();

    for member_id in &required {
        if !members.contains_key(member_id) {
            bail!("required member missing: {member_id}");
        }
    }

    for (member_id, record_val) in members.iter() {
        if !known.contains(member_id.as_str()) {
            bail!("unknown member in summary: {member_id}");
        }
        let record = require_table(Some(record_val), &format!("members.{member_id}"))?;
        require_text(
            record.get("contract"),
            &format!("members.{member_id}.contract"),
        )?;
        require_text(
            record.get("composition_hash"),
            &format!("members.{member_id}.composition_hash"),
        )?;
        validate_evidence_record(record, &format!("members.{member_id}"), delivery_dir, repo)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Master-record validation — mirrors `_validate_master_record`
// ---------------------------------------------------------------------------

fn validate_master_record(
    summary: &serde_json::Map<String, Value>,
    delivery_dir: Option<&Path>,
    repo: &Path,
) -> Result<serde_json::Map<String, Value>, SwarmValidationError> {
    let master = require_table(summary.get("master"), "master")?;
    require_text(master.get("contract"), "master.contract")?;
    require_text(master.get("composition_hash"), "master.composition_hash")?;
    validate_evidence_record(master, "master", delivery_dir, repo)?;

    let replay = require_table(
        master.get("real_member_replay"),
        "master.real_member_replay",
    )?;

    match replay.get("passed") {
        Some(Value::Bool(_)) => {}
        _ => bail!("master.real_member_replay.passed must be bool"),
    }

    require_existing_file(
        replay.get("evidence"),
        "master.real_member_replay.evidence",
        delivery_dir,
        repo,
    )?;

    Ok(master.clone())
}

// ---------------------------------------------------------------------------
// Quality-threshold validation — mirrors `_validate_quality_thresholds`
// ---------------------------------------------------------------------------

fn validate_quality_thresholds(
    summary: &serde_json::Map<String, Value>,
    thresholds: &serde_json::Map<String, Value>,
    mode: &str,
) -> Result<serde_json::Map<String, Value>, SwarmValidationError> {
    let metrics = require_table(summary.get("metrics"), "metrics")?;

    let checks: &[(&str, &str, &str)] = &[
        ("master_recall", "master_recall_min", ">="),
        ("blocking_recall", "blocking_recall_min", ">="),
        ("false_positive_carry", "false_positive_carry_max", "<="),
        ("duplicate_collapse", "duplicate_collapse_min", ">="),
    ];

    let mut measured: serde_json::Map<String, Value> = serde_json::Map::new();

    for (metric_key, threshold_key, direction) in checks {
        let value = require_number(metrics.get(*metric_key), &format!("metrics.{metric_key}"))?;
        let threshold = require_number(
            thresholds.get(*threshold_key),
            &format!("suite.thresholds.{threshold_key}"),
        )?;
        measured.insert(metric_key.to_string(), Value::from(value));

        if mode == "member-only" {
            continue;
        }

        let failed = if *direction == ">=" {
            value < threshold
        } else {
            value > threshold
        };

        if failed {
            bail!("metrics.{metric_key} does not satisfy {threshold_key}");
        }
    }

    Ok(measured)
}

// ---------------------------------------------------------------------------
// `validate_summary` — public, mirrors Python's
// ---------------------------------------------------------------------------

/// Validated summary info returned by `validate_summary`.
pub struct SummaryInfo {
    pub total_cost_usd: f64,
    pub total_wall_sec: f64,
    pub handoff_mode: String,
    pub metrics: serde_json::Map<String, Value>,
    pub master_contract: String,
}

/// Validate suite summary against the launchability envelope.
/// Mirrors `validate_summary(summary, suite_spec, delivery_dir=None)`.
pub fn validate_summary(
    summary: &Value,
    suite_spec: &Value,
    delivery_dir: Option<&Path>,
    repo: &Path,
) -> Result<SummaryInfo, SwarmValidationError> {
    let summary = match summary.as_object() {
        Some(m) => m,
        None => bail!("summary must be a table"),
    };
    let suite_spec_obj = suite_spec
        .as_object()
        .ok_or_else(|| SwarmValidationError("suite_spec must be a table".to_string()))?;

    let suite = require_table(summary.get("suite"), "suite")?;
    let waivers: Option<&serde_json::Map<String, Value>> =
        summary.get("waivers").and_then(Value::as_object);
    let handoff = require_table(summary.get("handoff"), "handoff")?;

    let spec_suite = suite_spec_obj.get("suite").and_then(Value::as_object);
    static EMPTY_THRESHOLDS: std::sync::OnceLock<serde_json::Map<String, Value>> =
        std::sync::OnceLock::new();
    let thresholds: &serde_json::Map<String, Value> = spec_suite
        .and_then(|s| s.get("thresholds"))
        .and_then(Value::as_object)
        .unwrap_or_else(|| EMPTY_THRESHOLDS.get_or_init(serde_json::Map::new));

    let total_cost = require_number(suite.get("total_cost_usd"), "suite.total_cost_usd")?;
    let total_wall = require_number(suite.get("total_wall_sec"), "suite.total_wall_sec")?;

    let cost_ceiling: f64 = spec_suite
        .and_then(|s| s.get("cost_ceiling_usd"))
        .and_then(Value::as_f64)
        .unwrap_or(2.0);
    let wall_ceiling: f64 = spec_suite
        .and_then(|s| s.get("wall_ceiling_sec"))
        .and_then(Value::as_f64)
        .unwrap_or(1200.0);

    let cost_waiver = waivers
        .and_then(|w| w.get("cost_ceiling"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let wall_waiver = waivers
        .and_then(|w| w.get("wall_time"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if total_cost > cost_ceiling && !cost_waiver {
        bail!("suite exceeds cost ceiling without waiver");
    }
    if total_wall > wall_ceiling && !wall_waiver {
        bail!("suite exceeds wall-time ceiling without waiver");
    }

    let mode = match handoff.get("mode").and_then(Value::as_str) {
        Some("full-swarm") => "full-swarm",
        Some("member-only") => "member-only",
        _ => bail!("handoff.mode must be full-swarm or member-only"),
    };

    validate_member_records(summary, suite_spec_obj, delivery_dir, repo)?;
    let master = validate_master_record(summary, delivery_dir, repo)?;

    let replay = master
        .get("real_member_replay")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            SwarmValidationError("master.real_member_replay must be a table".to_string())
        })?;

    if replay.get("passed").and_then(Value::as_bool) != Some(true) && mode != "member-only" {
        bail!("full-swarm handoff requires passing real-member replay");
    }

    let measured = validate_quality_thresholds(summary, thresholds, mode)?;

    let master_contract = master
        .get("contract")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Ok(SummaryInfo {
        total_cost_usd: total_cost,
        total_wall_sec: total_wall,
        handoff_mode: mode.to_string(),
        metrics: measured,
        master_contract,
    })
}

// ---------------------------------------------------------------------------
// `render_swarm_contract` — mirrors Python's string-template function
// ---------------------------------------------------------------------------

/// Render the TOML-formatted swarm contract text.
/// Mirrors `render_swarm_contract(suite_spec, summary, generated=None, delivery_dir=None)`.
pub fn render_swarm_contract(
    suite_spec: &Value,
    summary: &Value,
    ts: Option<&str>,
    delivery_dir: Option<&Path>,
    repo: &Path,
) -> Result<String, SwarmValidationError> {
    static EMPTY_MAP: std::sync::OnceLock<serde_json::Map<String, Value>> =
        std::sync::OnceLock::new();
    let empty_map = EMPTY_MAP.get_or_init(serde_json::Map::new);

    let spec_obj = suite_spec.as_object().unwrap_or(empty_map);

    // Extract suite sub-table (may be absent)
    let suite = spec_obj
        .get("suite")
        .and_then(Value::as_object)
        .unwrap_or(empty_map);

    let thresholds = suite
        .get("thresholds")
        .and_then(Value::as_object)
        .unwrap_or(empty_map);

    let summary_info = validate_summary(summary, suite_spec, delivery_dir, repo)?;

    // required = ", ".join(_toml_str(v) for v in suite.get("required_members") or [])
    let required: String = suite
        .get("required_members")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(toml_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let optional: String = suite
        .get("optional_members")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(toml_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    // Numbers from thresholds — replicate Python's bare number formatting.
    // Python writes e.g. `master_recall_min = 0.9` (no trailing zero);
    // these come straight from the TOML, so we preserve as-is via f64 Display.
    let master_recall_min = require_number(
        thresholds.get("master_recall_min"),
        "suite.thresholds.master_recall_min",
    )?;
    let blocking_recall_min = require_number(
        thresholds.get("blocking_recall_min"),
        "suite.thresholds.blocking_recall_min",
    )?;
    let false_positive_carry_max = require_number(
        thresholds.get("false_positive_carry_max"),
        "suite.thresholds.false_positive_carry_max",
    )?;
    let duplicate_collapse_min = require_number(
        thresholds.get("duplicate_collapse_min"),
        "suite.thresholds.duplicate_collapse_min",
    )?;

    let cost_ceiling_usd: f64 = suite
        .get("cost_ceiling_usd")
        .and_then(Value::as_f64)
        .unwrap_or(2.0);
    let cost_waiver_usd: f64 = suite
        .get("cost_waiver_usd")
        .and_then(Value::as_f64)
        .unwrap_or(3.0);
    let wall_ceiling_sec: f64 = suite
        .get("wall_ceiling_sec")
        .and_then(Value::as_f64)
        .unwrap_or(1200.0);
    let wall_waiver_sec: f64 = suite
        .get("wall_waiver_sec")
        .and_then(Value::as_f64)
        .unwrap_or(1800.0);

    let suite_id = spec_obj.get("id").and_then(Value::as_str).unwrap_or("");
    let mode = spec_obj.get("mode").and_then(Value::as_str).unwrap_or("");
    let taxonomy = suite.get("taxonomy").and_then(Value::as_str).unwrap_or("");

    // Python uses f-string which calls `str()` on the float for thresholds and
    // budget numbers. The exact output depends on the Python f64→str format:
    // integers print as e.g. `1` (not `1.0`) via Python's `1` representation of
    // TOML integers. We replicate by formatting as Python would.
    // Python TOML: integers stay integers (e.g. 1200, 1, 2); floats stay float.
    // We need to print them the same way Python's f-string would. The Python
    // values come from TOML which distinguishes int vs float.
    // Since we read them as f64, we must print to match: Python's `str(1.0)` →
    // `"1.0"`, `str(1200.0)` → `"1200.0"`. But Python TOML parses `1200` as int,
    // so `str(1200)` → `"1200"` (no decimal). We can't recover that from f64.
    // Solution: check whether the original JSON value was an integer.
    let fmt_num = |v: &Value, fallback: f64| -> String {
        match v {
            Value::Number(n) if n.as_i64().is_some() => n.as_i64().unwrap().to_string(),
            Value::Number(n) => {
                let f = n.as_f64().unwrap_or(fallback);
                // Python str(float) — minimal representation
                py_float_display(f)
            }
            _ => py_float_display(fallback),
        }
    };

    // For threshold/budget values, we need the original Value references.
    let thr_mr = thresholds.get("master_recall_min");
    let thr_br = thresholds.get("blocking_recall_min");
    let thr_fp = thresholds.get("false_positive_carry_max");
    let thr_dc = thresholds.get("duplicate_collapse_min");

    let suite_cc = suite.get("cost_ceiling_usd");
    let suite_cw = suite.get("cost_waiver_usd");
    let suite_wc = suite.get("wall_ceiling_sec");
    let suite_ww = suite.get("wall_waiver_sec");

    let s = format!(
        "\
# Swarm contract - generated by daedalus export-suite.
swarm_contract = {SWARM_CONTRACT_VERSION}
generated = {gen}
suite = {suite_id_str}
mode = {mode_str}
taxonomy = {taxonomy_str}
handoff_mode = {handoff_mode_str}

[members]
required = [{required}]
optional = [{optional}]

[thresholds]
master_recall_min = {mr}
blocking_recall_min = {br}
false_positive_carry_max = {fp}
duplicate_collapse_min = {dc}

[budgets]
cost_ceiling_usd = {cc}
cost_waiver_usd = {cw}
wall_ceiling_sec = {wc}
wall_waiver_sec = {ww}
measured_cost_usd = {measured_cost}
measured_wall_sec = {measured_wall}

[evidence]
summary = \"summary.json\"
master_contract = {mc}

[approval]
g3_signed = false
g3_approval = \"approvals/G3-pr-review-swarm.md\"
note = \"Do not deploy as a primary reviewer until G3 is signed by a human; unsigned suite contracts may only produce sandbox dry-run packets.\"
",
        SWARM_CONTRACT_VERSION = SWARM_CONTRACT_VERSION,
        gen = toml_str(&generated(ts)),
        suite_id_str = toml_str(suite_id),
        mode_str = toml_str(mode),
        taxonomy_str = toml_str(taxonomy),
        handoff_mode_str = toml_str(&summary_info.handoff_mode),
        required = required,
        optional = optional,
        mr = thr_mr.map(|v| fmt_num(v, master_recall_min)).unwrap_or_else(|| py_float_display(master_recall_min)),
        br = thr_br.map(|v| fmt_num(v, blocking_recall_min)).unwrap_or_else(|| py_float_display(blocking_recall_min)),
        fp = thr_fp.map(|v| fmt_num(v, false_positive_carry_max)).unwrap_or_else(|| py_float_display(false_positive_carry_max)),
        dc = thr_dc.map(|v| fmt_num(v, duplicate_collapse_min)).unwrap_or_else(|| py_float_display(duplicate_collapse_min)),
        cc = suite_cc.map(|v| fmt_num(v, cost_ceiling_usd)).unwrap_or_else(|| py_float_display(cost_ceiling_usd)),
        cw = suite_cw.map(|v| fmt_num(v, cost_waiver_usd)).unwrap_or_else(|| py_float_display(cost_waiver_usd)),
        wc = suite_wc.map(|v| fmt_num(v, wall_ceiling_sec)).unwrap_or_else(|| py_float_display(wall_ceiling_sec)),
        ww = suite_ww.map(|v| fmt_num(v, wall_waiver_sec)).unwrap_or_else(|| py_float_display(wall_waiver_sec)),
        measured_cost = py_float_display(summary_info.total_cost_usd),
        measured_wall = py_float_display(summary_info.total_wall_sec),
        mc = toml_str(&summary_info.master_contract),
    );

    Ok(s)
}

/// Format a float the way Python's `str(float)` or f-string does for TOML
/// number values: minimal digits, always includes decimal point for non-integer
/// values.
fn py_float_display(x: f64) -> String {
    // If the value is a whole number, Python's TOML integer representation
    // would not include ".0". However, when the value was a TOML float, Python
    // prints e.g. `2.0`. We cannot distinguish origin here, so we use Rust's
    // default float display which gives "2.0" for 2.0f64 — matching Python's
    // str(2.0) == "2.0".
    // For integers like 1200.0, Python gives "1200.0" when it's a float.
    // Rust's `{x}` gives "1200" for 1200.0; we need "1200.0".
    // Use format that always shows decimal.
    if x.fract() == 0.0 && x.is_finite() {
        format!("{x:.1}")
    } else {
        // Strip trailing zeros after the decimal point but keep at least one
        let s = format!("{x}");
        s
    }
}

// ---------------------------------------------------------------------------
// `export_suite` — mirrors Python's
// ---------------------------------------------------------------------------

/// Result of a successful `export_suite` call.
#[derive(Debug)]
pub struct ExportResult {
    pub contract: PathBuf,
    pub handoff: PathBuf,
    pub summary: PathBuf,
}

/// Export the swarm contract and handoff document to `delivery_dir`.
/// Mirrors `export_suite(delivery_dir, suite_spec, generated=None)`.
pub fn export_suite(
    delivery_dir: &Path,
    suite_spec: &Value,
    ts: Option<&str>,
    repo: &Path,
) -> Result<ExportResult, SwarmValidationError> {
    let summary_path = delivery_dir.join("summary.json");
    if !summary_path.exists() {
        bail!("suite summary missing: {}", summary_path.display());
    }

    let summary_text = std::fs::read_to_string(&summary_path)
        .map_err(|e| SwarmValidationError(format!("{}: {e}", summary_path.display())))?;
    let summary: Value = serde_json::from_str(&summary_text).map_err(|e| {
        SwarmValidationError(format!("{}: invalid JSON: {e}", summary_path.display()))
    })?;

    let contract_text = render_swarm_contract(suite_spec, &summary, ts, Some(delivery_dir), repo)?;

    // Parse contract_text as TOML to get the contract object for render_handoff
    let contract_toml: toml::Value = toml::from_str(&contract_text)
        .map_err(|e| SwarmValidationError(format!("generated contract is invalid TOML: {e}")))?;

    std::fs::create_dir_all(delivery_dir).map_err(|e| {
        SwarmValidationError(format!("cannot create {}: {e}", delivery_dir.display()))
    })?;

    let contract_path = delivery_dir.join("swarm-contract.toml");
    std::fs::write(&contract_path, &contract_text).map_err(|e| {
        SwarmValidationError(format!("cannot write {}: {e}", contract_path.display()))
    })?;

    let handoff_text = render_handoff(&contract_toml, &summary);
    let handoff_path = delivery_dir.join("plane-handoff.md");
    std::fs::write(&handoff_path, &handoff_text).map_err(|e| {
        SwarmValidationError(format!("cannot write {}: {e}", handoff_path.display()))
    })?;

    Ok(ExportResult {
        contract: contract_path,
        handoff: handoff_path,
        summary: summary_path,
    })
}

// ---------------------------------------------------------------------------
// `render_handoff` — mirrors Python's
// ---------------------------------------------------------------------------

/// Render the handoff Markdown document.
/// Mirrors `render_handoff(contract, summary)`.
pub fn render_handoff(contract: &toml::Value, summary: &Value) -> String {
    let mode = contract
        .get("handoff_mode")
        .and_then(toml::Value::as_str)
        .unwrap_or("");

    let suite_name = contract
        .get("suite")
        .and_then(toml::Value::as_str)
        .unwrap_or("");

    let required_members: Vec<&str> = contract
        .get("members")
        .and_then(|m| m.get("required"))
        .and_then(toml::Value::as_array)
        .map(|a| a.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();

    let optional_members: Vec<&str> = contract
        .get("members")
        .and_then(|m| m.get("optional"))
        .and_then(toml::Value::as_array)
        .map(|a| a.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();

    // budgets: measured_cost_usd and measured_wall_sec
    let measured_cost = contract
        .get("budgets")
        .and_then(|b| b.get("measured_cost_usd"))
        .and_then(toml::Value::as_float)
        .unwrap_or(0.0);
    let measured_wall = contract
        .get("budgets")
        .and_then(|b| b.get("measured_wall_sec"))
        .and_then(toml::Value::as_float)
        .unwrap_or(0.0);

    // The master replay line uses json.dumps(summary.get('master', {}), sort_keys=True)
    let master_obj = summary
        .get("master")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let master_json = json_dumps_sort_keys(&master_obj);

    let lines: Vec<String> = vec![
        format!("# Review-swarm handoff: {suite_name}"),
        "".to_string(),
        "Lab evidence is not launch approval. G3/G4/G5 still gate deployment,".to_string(),
        "write authority, and production-data re-ingestion.".to_string(),
        "".to_string(),
        "## Suite".to_string(),
        "".to_string(),
        format!("- Mode: `{mode}`"),
        format!("- Required members: `{}`", required_members.join(", ")),
        format!("- Optional members: `{}`", optional_members.join(", ")),
        format!("- Measured cost: `${}`", py_float_display(measured_cost)),
        format!(
            "- Measured wall time: `{}s`",
            py_float_display(measured_wall)
        ),
        "".to_string(),
        "## Import Boundary".to_string(),
        "".to_string(),
        "- member agents write artifacts only.".to_string(),
        "- The master/control plane owns synthesis and any later posting.".to_string(),
        "- Unsigned use is sandbox-only and non-primary.".to_string(),
        "".to_string(),
        "## Residual Evidence".to_string(),
        "".to_string(),
        format!("- Master replay: `{master_json}`"),
    ];

    lines.join("\n") + "\n"
}

/// Replicate Python's `json.dumps(obj, sort_keys=True)`.
/// Recursively serializes the JSON value with sorted object keys.
fn json_dumps_sort_keys(value: &Value) -> String {
    match value {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            let pairs: Vec<String> = keys
                .iter()
                .map(|k| {
                    let v = json_dumps_sort_keys(&m[*k]);
                    format!("{}: {}", serde_json::to_string(k).unwrap(), v)
                })
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        Value::Array(a) => {
            let elems: Vec<String> = a.iter().map(json_dumps_sort_keys).collect();
            format!("[{}]", elems.join(", "))
        }
        _ => serde_json::to_string(value).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// `validate_swarm_contract` — mirrors Python's
// ---------------------------------------------------------------------------

/// Validate a loaded swarm contract.
/// Mirrors `validate_swarm_contract(contract, delivery_dir)`.
pub fn validate_swarm_contract(
    contract: &toml::Value,
    delivery_dir: &Path,
) -> Result<(), SwarmValidationError> {
    // swarm_contract version check
    match contract.get("swarm_contract") {
        Some(toml::Value::Integer(v)) if *v == SWARM_CONTRACT_VERSION as i64 => {}
        _ => bail!("swarm_contract must be version 1"),
    }

    for key in ["suite", "mode", "taxonomy", "handoff_mode"] {
        match contract.get(key) {
            Some(toml::Value::String(s)) if !s.is_empty() => {}
            _ => bail!("{key} must not be empty"),
        }
    }

    for table in ["members", "thresholds", "budgets", "evidence", "approval"] {
        match contract.get(table) {
            Some(toml::Value::Table(_)) => {}
            _ => bail!("{table} table is required"),
        }
    }

    for key in ["required", "optional"] {
        let members = contract.get("members").and_then(toml::Value::as_table);
        match members.and_then(|m| m.get(key)) {
            Some(toml::Value::Array(_)) => {}
            _ => bail!("members.{key} must be a list"),
        }
    }

    // evidence.summary file must exist
    let summary_ref = contract
        .get("evidence")
        .and_then(toml::Value::as_table)
        .and_then(|e| e.get("summary"))
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let summary_path = delivery_dir.join(summary_ref);
    if !summary_path.is_file() {
        bail!(
            "summary evidence does not exist: {}",
            summary_path.display()
        );
    }

    // approval.g3_signed must be bool
    match contract
        .get("approval")
        .and_then(toml::Value::as_table)
        .and_then(|a| a.get("g3_signed"))
    {
        Some(toml::Value::Boolean(_)) => {}
        _ => bail!("approval.g3_signed must be bool"),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// `load_swarm_contract` — mirrors Python's
// ---------------------------------------------------------------------------

/// Load and validate a swarm contract from `delivery_dir/swarm-contract.toml`.
/// Mirrors `load_swarm_contract(delivery_dir)`.
pub fn load_swarm_contract(delivery_dir: &Path) -> Result<toml::Value, SwarmValidationError> {
    let path = delivery_dir.join("swarm-contract.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| SwarmValidationError(format!("{}: {e}", path.display())))?;
    let contract: toml::Value = toml::from_str(&text)
        .map_err(|e| SwarmValidationError(format!("{}: invalid TOML: {e}", path.display())))?;
    validate_swarm_contract(&contract, delivery_dir)?;
    Ok(contract)
}

// ---------------------------------------------------------------------------
// Unit tests (port of tests/test_swarm.py assertions)
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

    fn suite_spec() -> Value {
        let repo = repo_root();
        let text = std::fs::read_to_string(repo.join("specs/pr-review-suite/taskspec.toml"))
            .expect("taskspec.toml");
        let tv: toml::Value = toml::from_str(&text).expect("parse toml");
        toml_value_to_json(tv)
    }

    fn write_run_evidence(delivery: &Path, hashes: &[&str]) {
        let run_dir = delivery.join("evidence/run");
        let comp_dir = run_dir.join("compositions");
        std::fs::create_dir_all(&comp_dir).unwrap();
        std::fs::write(
            run_dir.join("trials.jsonl"),
            "{\"candidate_id\":\"suite\"}\n",
        )
        .unwrap();
        for (i, hash) in hashes.iter().enumerate() {
            std::fs::write(
                comp_dir.join(format!("c{}.json", i + 1)),
                serde_json::to_string(&json!({"composition_hash": hash})).unwrap(),
            )
            .unwrap();
        }
        std::fs::write(
            delivery.join("evidence/replay.json"),
            serde_json::to_string(&json!({"passed": true, "source": "test"})).unwrap(),
        )
        .unwrap();
    }

    fn write_summary(delivery: &Path, overrides: Option<Value>) -> Value {
        std::fs::create_dir_all(delivery).unwrap();
        let run_dir = "evidence/run";
        let hashes = vec!["generalhash", "correcthash", "securityhash", "masterhash"];
        write_run_evidence(delivery, &hashes);
        let mut summary = json!({
            "suite": {"total_cost_usd": 1.25, "total_wall_sec": 900},
            "waivers": {},
            "metrics": {
                "master_recall": 0.95,
                "blocking_recall": 1.0,
                "false_positive_carry": 1,
                "duplicate_collapse": 0.95
            },
            "members": {
                "general": {
                    "contract": "members/general/contract.toml",
                    "composition_hash": "generalhash",
                    "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")}
                },
                "correctness": {
                    "contract": "members/correctness/contract.toml",
                    "composition_hash": "correcthash",
                    "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")}
                },
                "security": {
                    "contract": "members/security/contract.toml",
                    "composition_hash": "securityhash",
                    "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")}
                }
            },
            "master": {
                "contract": "master/contract.toml",
                "composition_hash": "masterhash",
                "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")},
                "real_member_replay": {"passed": true, "evidence": "evidence/replay.json"}
            },
            "handoff": {"mode": "full-swarm"}
        });

        if let Some(ov) = overrides {
            if let (Some(s_obj), Some(ov_obj)) = (summary.as_object_mut(), ov.as_object()) {
                for (k, v) in ov_obj {
                    s_obj.insert(k.clone(), v.clone());
                }
            }
        }

        std::fs::write(
            delivery.join("summary.json"),
            serde_json::to_string(&summary).unwrap(),
        )
        .unwrap();

        summary
    }

    fn tmpdir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("daedalus-swarm-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Convert a toml::Value to serde_json::Value (best-effort; no datetime support needed here)
    pub fn toml_value_to_json(tv: toml::Value) -> Value {
        match tv {
            toml::Value::String(s) => Value::String(s),
            toml::Value::Integer(i) => Value::from(i),
            toml::Value::Float(f) => Value::from(f),
            toml::Value::Boolean(b) => Value::Bool(b),
            toml::Value::Array(a) => Value::Array(a.into_iter().map(toml_value_to_json).collect()),
            toml::Value::Table(t) => {
                let mut m = serde_json::Map::new();
                for (k, v) in t {
                    m.insert(k, toml_value_to_json(v));
                }
                Value::Object(m)
            }
            toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        }
    }

    #[test]
    fn export_suite_writes_swarm_contract_and_handoff() {
        let delivery = tmpdir("export-basic");
        write_summary(&delivery, None);
        let spec = suite_spec();
        let repo = repo_root();

        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &repo)
            .expect("export_suite ok");

        let contract_text = std::fs::read_to_string(&result.contract).unwrap();
        let contract: toml::Value = toml::from_str(&contract_text).unwrap();

        assert_eq!(
            contract
                .get("swarm_contract")
                .and_then(toml::Value::as_integer),
            Some(1)
        );
        assert_eq!(
            contract.get("suite").and_then(toml::Value::as_str),
            Some("pr-review-suite")
        );
        let req = contract
            .get("members")
            .and_then(|m| m.get("required"))
            .and_then(toml::Value::as_array)
            .unwrap();
        let req_strs: Vec<&str> = req.iter().filter_map(toml::Value::as_str).collect();
        assert_eq!(req_strs, vec!["general", "correctness", "security"]);
        assert_eq!(
            contract
                .get("budgets")
                .and_then(|b| b.get("measured_cost_usd"))
                .and_then(toml::Value::as_float),
            Some(1.25)
        );
        assert_eq!(
            contract
                .get("budgets")
                .and_then(|b| b.get("measured_wall_sec"))
                .and_then(toml::Value::as_float),
            Some(900.0)
        );
        assert_eq!(
            contract
                .get("evidence")
                .and_then(|e| e.get("master_contract"))
                .and_then(toml::Value::as_str),
            Some("master/contract.toml")
        );
        assert_eq!(
            contract
                .get("approval")
                .and_then(|a| a.get("g3_signed"))
                .and_then(toml::Value::as_bool),
            Some(false)
        );

        let handoff_text = std::fs::read_to_string(&result.handoff).unwrap();
        assert!(handoff_text.contains("member agents write artifacts only"));

        let _ = std::fs::remove_dir_all(&delivery);
    }

    #[test]
    fn export_suite_requires_cost_waiver_above_ceiling() {
        let delivery = tmpdir("export-cost-waiver");
        let spec = suite_spec();
        let repo = repo_root();

        write_summary(
            &delivery,
            Some(json!({"suite": {"total_cost_usd": 2.5, "total_wall_sec": 900}})),
        );
        let err =
            export_suite(&delivery, &spec, None, &repo).expect_err("should fail without waiver");
        assert!(
            err.0.contains("cost ceiling"),
            "expected 'cost ceiling' in: {}",
            err.0
        );

        write_summary(
            &delivery,
            Some(json!({
                "suite": {"total_cost_usd": 2.5, "total_wall_sec": 900},
                "waivers": {"cost_ceiling": true}
            })),
        );
        export_suite(&delivery, &spec, None, &repo).expect("should succeed with waiver");

        let _ = std::fs::remove_dir_all(&delivery);
    }

    #[test]
    fn export_suite_member_only_when_master_replay_fails() {
        let delivery = tmpdir("export-member-only");
        let spec = suite_spec();
        let repo = repo_root();

        // full-swarm with failed replay → error
        write_summary(
            &delivery,
            Some(json!({
                "master": {
                    "contract": "master/contract.toml",
                    "composition_hash": "masterhash",
                    "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"},
                    "real_member_replay": {"passed": false, "evidence": "evidence/replay.json"}
                },
                "handoff": {"mode": "full-swarm"}
            })),
        );
        let err = export_suite(&delivery, &spec, None, &repo).expect_err("should fail");
        assert!(err.0.contains("real-member replay"), "{}", err.0);

        // member-only with failed replay → ok
        write_summary(
            &delivery,
            Some(json!({
                "master": {
                    "contract": "master/contract.toml",
                    "composition_hash": "masterhash",
                    "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"},
                    "real_member_replay": {"passed": false, "evidence": "evidence/replay.json"}
                },
                "handoff": {"mode": "member-only"}
            })),
        );
        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &repo)
            .expect("member-only should succeed");
        let contract_text = std::fs::read_to_string(&result.contract).unwrap();
        let contract: toml::Value = toml::from_str(&contract_text).unwrap();
        assert_eq!(
            contract.get("handoff_mode").and_then(toml::Value::as_str),
            Some("member-only")
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    #[test]
    fn export_suite_requires_member_contract_evidence() {
        let delivery = tmpdir("export-missing-member");
        let spec = suite_spec();
        let repo = repo_root();

        // Remove security member
        write_summary(
            &delivery,
            Some(json!({
                "members": {
                    "general": {
                        "contract": "members/general/contract.toml",
                        "composition_hash": "generalhash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    },
                    "correctness": {
                        "contract": "members/correctness/contract.toml",
                        "composition_hash": "correcthash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    }
                }
            })),
        );
        let err = export_suite(&delivery, &spec, None, &repo).expect_err("should fail");
        assert!(err.0.contains("required member missing"), "{}", err.0);

        let _ = std::fs::remove_dir_all(&delivery);
    }

    #[test]
    fn export_suite_rejects_full_swarm_below_quality_threshold() {
        let delivery = tmpdir("export-quality");
        let spec = suite_spec();
        let repo = repo_root();

        write_summary(
            &delivery,
            Some(json!({
                "metrics": {
                    "master_recall": 0.5,
                    "blocking_recall": 1.0,
                    "false_positive_carry": 1,
                    "duplicate_collapse": 0.95
                }
            })),
        );
        let err = export_suite(&delivery, &spec, None, &repo).expect_err("should fail");
        assert!(err.0.contains("master_recall"), "{}", err.0);

        let _ = std::fs::remove_dir_all(&delivery);
    }

    #[test]
    fn export_suite_requires_real_evidence_paths_and_hashes() {
        let delivery = tmpdir("export-hash");
        let spec = suite_spec();
        let repo = repo_root();

        write_summary(
            &delivery,
            Some(json!({
                "members": {
                    "general": {
                        "contract": "members/general/contract.toml",
                        "composition_hash": "fabricated",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    },
                    "correctness": {
                        "contract": "members/correctness/contract.toml",
                        "composition_hash": "correcthash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    },
                    "security": {
                        "contract": "members/security/contract.toml",
                        "composition_hash": "securityhash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    }
                }
            })),
        );
        let err = export_suite(&delivery, &spec, None, &repo).expect_err("should fail");
        assert!(err.0.contains("composition_hash"), "{}", err.0);

        let _ = std::fs::remove_dir_all(&delivery);
    }
}
