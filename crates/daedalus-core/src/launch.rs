//! Approval-aware launch/import packet helpers.
//!
//! Port of `runner/launch.py`. These helpers deliberately stop before deployment.
//! Unsigned contracts can produce sandbox review artifacts, but any
//! runtime-facing packet requires G3.
//!
//! ## Timestamp handling
//!
//! The Python `_generated(value=None)` idiom becomes an `Option<&str>` param
//! that falls back to [`pycompat::utc_now_iso`]. The parity test always passes
//! an explicit timestamp so results are deterministic; the fallback is
//! wall-clock only.

use std::path::{Path, PathBuf};

use crate::pycompat::utc_now_iso;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Raised when a launch/import path needs G3 but the contract is unsigned.
/// Mirrors Python's `UnsignedLaunchError(RuntimeError)`.
#[derive(Debug)]
pub struct UnsignedLaunchError(pub String);

impl std::fmt::Display for UnsignedLaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UnsignedLaunchError {}

/// Raised when a launch contract is malformed or over-authorized.
/// Mirrors Python's `ContractValidationError(RuntimeError)`.
#[derive(Debug)]
pub struct ContractValidationError(pub String);

impl std::fmt::Display for ContractValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ContractValidationError {}

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

/// Return `value` if `Some` and non-empty; otherwise call [`utc_now_iso`].
/// Mirrors `_generated(value=None)`.
fn generated(value: Option<&str>) -> String {
    match value {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => utc_now_iso(),
    }
}

// ---------------------------------------------------------------------------
// Path resolution — mirrors Python's `_resolve_contract_path`
// ---------------------------------------------------------------------------

/// Resolve a ref string: absolute → use as-is; relative → try delivery_dir
/// first, fall back to repo root.
/// Mirrors `_resolve_contract_path(ref, delivery_dir)`.
fn resolve_contract_path(ref_: &str, delivery_dir: &Path, repo: &Path) -> PathBuf {
    let path = Path::new(ref_);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let local = delivery_dir.join(path);
    if local.exists() {
        return local;
    }
    repo.join(path)
}

// ---------------------------------------------------------------------------
// Require helpers — mirror Python's `_require_*` family
// ---------------------------------------------------------------------------

fn require_keys(
    table: &toml::Table,
    keys: &[&str],
    label: &str,
) -> Result<(), ContractValidationError> {
    let missing: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|k| !table.contains_key(*k))
        .collect();
    if !missing.is_empty() {
        return Err(ContractValidationError(format!(
            "{label} missing required field(s): {}",
            missing.join(", ")
        )));
    }
    Ok(())
}

fn require_string(
    value: Option<&toml::Value>,
    label: &str,
) -> Result<String, ContractValidationError> {
    match value {
        Some(toml::Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(toml::Value::String(_)) => Err(ContractValidationError(format!(
            "{label} must not be empty"
        ))),
        _ => Err(ContractValidationError(format!("{label} must be str"))),
    }
}

fn require_number(
    value: Option<&toml::Value>,
    label: &str,
) -> Result<f64, ContractValidationError> {
    match value {
        Some(toml::Value::Integer(i)) => Ok(*i as f64),
        Some(toml::Value::Float(f)) => Ok(*f),
        _ => Err(ContractValidationError(format!(
            "{label} must be int|float"
        ))),
    }
}

fn require_bool(value: Option<&toml::Value>, label: &str) -> Result<bool, ContractValidationError> {
    match value {
        Some(toml::Value::Boolean(b)) => Ok(*b),
        _ => Err(ContractValidationError(format!("{label} must be bool"))),
    }
}

fn require_array<'a>(
    value: Option<&'a toml::Value>,
    label: &str,
) -> Result<&'a toml::value::Array, ContractValidationError> {
    match value {
        Some(toml::Value::Array(a)) => Ok(a),
        _ => Err(ContractValidationError(format!("{label} must be list"))),
    }
}

// ---------------------------------------------------------------------------
// Approval file check — mirrors Python's `_approval_file_approved`
// ---------------------------------------------------------------------------

/// Check that a file exists and contains an approval marker.
/// Mirrors `_approval_file_approved(path)`.
fn approval_file_approved(path: &str, repo: &Path) -> bool {
    let approval_path = Path::new(path);
    let resolved = if approval_path.is_absolute() {
        approval_path.to_path_buf()
    } else {
        repo.join(approval_path)
    };
    if !resolved.exists() {
        return false;
    }
    match std::fs::read_to_string(&resolved) {
        Ok(text) => text.contains("**Status:** approved") || text.contains("**Status:** signed"),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// `validate_contract` — mirrors Python's `validate_contract`
// ---------------------------------------------------------------------------

/// Validate contract.v1 before any import packet consumes it.
/// Mirrors `validate_contract(contract, delivery_dir)`.
pub fn validate_contract(
    contract: &toml::Value,
    delivery_dir: &Path,
    repo: &Path,
) -> Result<(), ContractValidationError> {
    let root = contract
        .as_table()
        .ok_or_else(|| ContractValidationError("contract must be a table".to_string()))?;

    require_keys(
        root,
        &["contract", "agent", "composition_hash", "taskspec", "mode"],
        "contract",
    )?;

    // contract == 1
    match root.get("contract") {
        Some(toml::Value::Integer(1)) => {}
        _ => {
            return Err(ContractValidationError(
                "contract must be version 1".to_string(),
            ))
        }
    }

    // agent, composition_hash, taskspec, mode must be non-empty strings
    for key in &["agent", "composition_hash", "taskspec", "mode"] {
        require_string(root.get(*key), key)?;
    }

    // [composition]
    let composition = root
        .get("composition")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ContractValidationError("composition must be a table".to_string()))?;

    require_keys(
        composition,
        &[
            "harness",
            "harness_version",
            "provider",
            "model",
            "prompt_packet",
            "timeout_sec",
        ],
        "composition",
    )?;

    for key in &[
        "harness",
        "harness_version",
        "provider",
        "model",
        "prompt_packet",
    ] {
        require_string(composition.get(*key), &format!("composition.{key}"))?;
    }
    require_number(composition.get("timeout_sec"), "composition.timeout_sec")?;

    let prompt_ref = composition
        .get("prompt_packet")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let prompt = resolve_contract_path(prompt_ref, delivery_dir, repo);
    if !prompt.is_file() {
        return Err(ContractValidationError(format!(
            "prompt_packet does not exist: {prompt_ref}"
        )));
    }

    // [permissions]
    let permissions = root
        .get("permissions")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ContractValidationError("permissions must be a table".to_string()))?;

    require_keys(
        permissions,
        &["workspace", "env", "write_actions"],
        "permissions",
    )?;
    require_array(permissions.get("env"), "permissions.env")?;

    let write_actions = permissions
        .get("write_actions")
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_lowercase();

    let approval = root.get("approval").and_then(toml::Value::as_table);
    let approval_table = approval.unwrap_or(&EMPTY_TABLE_STATIC);

    if write_actions != "none" {
        let g4_signed = approval_table
            .get("g4_signed")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        if !g4_signed {
            return Err(ContractValidationError(
                "contract grants write authority before G4 approval".to_string(),
            ));
        }
        // g4_approval required
        require_keys(approval_table, &["g4_approval"], "approval")?;
        let g4_path = require_string(approval_table.get("g4_approval"), "approval.g4_approval")?;
        if !approval_file_approved(&g4_path, repo) {
            return Err(ContractValidationError(
                "G4 approval file is missing or unsigned".to_string(),
            ));
        }
    }

    // [budgets]
    let budgets = root
        .get("budgets")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ContractValidationError("budgets must be a table".to_string()))?;

    require_keys(
        budgets,
        &["max_cost_usd_per_run", "max_wall_sec"],
        "budgets",
    )?;
    let max_cost = require_number(
        budgets.get("max_cost_usd_per_run"),
        "budgets.max_cost_usd_per_run",
    )?;
    let max_wall = require_number(budgets.get("max_wall_sec"), "budgets.max_wall_sec")?;
    if max_cost < 0.0 || max_wall <= 0.0 {
        return Err(ContractValidationError(
            "budgets must be positive".to_string(),
        ));
    }

    // [observability]
    let observability = root
        .get("observability")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ContractValidationError("observability must be a table".to_string()))?;

    require_keys(
        observability,
        &["arena", "trace_destination"],
        "observability",
    )?;

    // [evidence]
    let evidence = root
        .get("evidence")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ContractValidationError("evidence must be a table".to_string()))?;

    require_keys(evidence, &["run_dir", "trials"], "evidence")?;

    for key in &["run_dir", "trials"] {
        let ref_ = require_string(evidence.get(*key), &format!("evidence.{key}"))?;
        let path = resolve_contract_path(&ref_, delivery_dir, repo);
        if *key == "run_dir" && !path.is_dir() {
            return Err(ContractValidationError(format!(
                "evidence.run_dir does not exist: {path}",
                path = path.display()
            )));
        }
        if *key == "trials" && !path.is_file() {
            return Err(ContractValidationError(format!(
                "evidence.trials does not exist: {path}",
                path = path.display()
            )));
        }
    }

    // [approval] — g3 fields (always required)
    require_keys(approval_table, &["g3_signed", "g3_approval"], "approval")?;
    require_bool(approval_table.get("g3_signed"), "approval.g3_signed")?;
    require_string(approval_table.get("g3_approval"), "approval.g3_approval")?;
    if let Some(v) = approval_table.get("g4_signed") {
        require_bool(Some(v), "approval.g4_signed")?;
    }

    Ok(())
}

/// A static empty toml::Table for use as a default when optional tables are absent.
static EMPTY_TABLE_STATIC: std::sync::LazyLock<toml::Table> =
    std::sync::LazyLock::new(toml::Table::new);

// ---------------------------------------------------------------------------
// `load_contract` — mirrors Python's `load_contract`
// ---------------------------------------------------------------------------

/// Load and validate a contract.toml from delivery_dir.
/// Mirrors `load_contract(delivery_dir)`.
pub fn load_contract(
    delivery_dir: &Path,
    repo: &Path,
) -> Result<toml::Value, ContractValidationError> {
    let path = delivery_dir.join("contract.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| ContractValidationError(format!("{}: {e}", path.display())))?;
    let contract: toml::Value = toml::from_str(&text)
        .map_err(|e| ContractValidationError(format!("{}: invalid TOML: {e}", path.display())))?;
    validate_contract(&contract, delivery_dir, repo)?;
    Ok(contract)
}

// ---------------------------------------------------------------------------
// `_prompt_hash` — mirrors Python's `_prompt_hash`
// ---------------------------------------------------------------------------

fn prompt_hash(
    contract: &toml::Table,
    delivery_dir: &Path,
    repo: &Path,
) -> Result<String, ContractValidationError> {
    let prompt_ref = contract
        .get("composition")
        .and_then(toml::Value::as_table)
        .and_then(|t| t.get("prompt_packet"))
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let prompt = resolve_contract_path(prompt_ref, delivery_dir, repo);
    let bytes = std::fs::read(&prompt)
        .map_err(|e| ContractValidationError(format!("{}: {e}", prompt.display())))?;
    use sha2::Digest;
    let hash = sha2::Sha256::digest(&bytes);
    Ok(format!("{hash:x}"))
}

// ---------------------------------------------------------------------------
// `require_g3` — mirrors Python's `require_g3`
// ---------------------------------------------------------------------------

/// Raise UnsignedLaunchError if G3 approval is missing or file is unsigned.
/// Mirrors `require_g3(contract)`.
pub fn require_g3(contract: &toml::Table, repo: &Path) -> Result<(), UnsignedLaunchError> {
    let approval = contract.get("approval").and_then(toml::Value::as_table);

    let g3_signed = approval
        .and_then(|a| a.get("g3_signed"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);

    if !g3_signed {
        return Err(UnsignedLaunchError("G3 approval is unsigned".to_string()));
    }

    let g3_approval = approval
        .and_then(|a| a.get("g3_approval"))
        .and_then(toml::Value::as_str)
        .unwrap_or("");

    if !approval_file_approved(g3_approval, repo) {
        return Err(UnsignedLaunchError(
            "G3 approval file is missing or unsigned".to_string(),
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// `_load_swarm_if_present` — mirrors Python's `_load_swarm_if_present`
// ---------------------------------------------------------------------------

/// Load swarm contract if swarm-contract.toml exists; return None otherwise.
/// Re-raises SwarmValidationError as ContractValidationError.
/// Mirrors `_load_swarm_if_present(delivery_dir)`.
fn load_swarm_if_present(
    delivery_dir: &Path,
) -> Result<Option<toml::Value>, ContractValidationError> {
    let path = delivery_dir.join("swarm-contract.toml");
    if !path.exists() {
        return Ok(None);
    }
    crate::swarm::load_swarm_contract(delivery_dir)
        .map(Some)
        .map_err(|e| ContractValidationError(e.to_string()))
}

// ---------------------------------------------------------------------------
// `render_import_packet` — mirrors Python's `render_import_packet`
// ---------------------------------------------------------------------------

/// Render a control-plane import packet for a single-agent delivery.
///
/// Non-dry-run packets require G3. Dry-run packets are explicitly marked as
/// non-deployable, sandbox-only, and never primary-reviewer-capable.
///
/// Mirrors `render_import_packet(delivery_dir, plane, dry_run=False, generated=None)`.
pub fn render_import_packet(
    delivery_dir: &Path,
    plane: &str,
    dry_run: bool,
    ts: Option<&str>,
    repo: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    // Swarm path: if swarm-contract.toml is present, delegate.
    if let Some(swarm_contract) = load_swarm_if_present(delivery_dir)? {
        return render_swarm_import_packet(delivery_dir, &swarm_contract, plane, dry_run, ts)
            .map_err(Into::into);
    }

    let contract = load_contract(delivery_dir, repo)?;
    let contract_table = contract.as_table().expect("contract is a table");

    let approval = contract_table
        .get("approval")
        .and_then(toml::Value::as_table);

    let g3_signed = approval
        .and_then(|a| a.get("g3_signed"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);

    let refusal: String;
    if !dry_run {
        // require_g3 raises on failure
        require_g3(contract_table, repo).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        refusal = String::new();
    } else if !g3_signed {
        refusal = "G3 approval is unsigned".to_string();
    } else {
        refusal = String::new();
    }

    let deployable = !dry_run && g3_signed;
    let mode = if dry_run { "dry-run" } else { "deployable" };
    let sandbox = if dry_run { "true" } else { "false" };
    let primary_reviewer_allowed = approval
        .and_then(|a| a.get("primary_reviewer_allowed"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);
    let primary_allowed = deployable && primary_reviewer_allowed;
    let primary = if primary_allowed { "true" } else { "false" };

    let agent = contract_table
        .get("agent")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let composition_hash = contract_table
        .get("composition_hash")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let prompt_packet_ref = contract_table
        .get("composition")
        .and_then(toml::Value::as_table)
        .and_then(|t| t.get("prompt_packet"))
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let sha = prompt_hash(contract_table, delivery_dir, repo)?;

    let g3_approval_val = approval
        .and_then(|a| a.get("g3_approval"))
        .and_then(toml::Value::as_str)
        .unwrap_or("");

    let deployable_str = if deployable { "true" } else { "false" };

    Ok(format!(
        "\
packet = 1
generated = {gen}
plane = {plane_str}
mode = {mode_str}
source_contract = \"contract.toml\"
agent = {agent_str}
composition_hash = {hash_str}
prompt_packet = {pp_str}
prompt_packet_sha256 = {sha_str}
deployable = {deployable_str}
sandbox_required = {sandbox}
primary_reviewer_allowed = {primary}
refusal_reason = {refusal_str}

[gates]
g3_signed = {g3_signed_str}
g3_approval = {g3_approval_str}
g4_required_for_write_authority = true
g5_required_for_prod_data_reingestion = true

[constraints]
write_authority = \"none\"
posting = \"control-plane dry run only before G3\"
",
        gen = toml_str(&generated(ts)),
        plane_str = toml_str(plane),
        mode_str = toml_str(mode),
        agent_str = toml_str(agent),
        hash_str = toml_str(composition_hash),
        pp_str = toml_str(prompt_packet_ref),
        sha_str = toml_str(&sha),
        deployable_str = deployable_str,
        sandbox = sandbox,
        primary = primary,
        refusal_str = toml_str(&refusal),
        g3_signed_str = if g3_signed { "true" } else { "false" },
        g3_approval_str = toml_str(g3_approval_val),
    ))
}

// ---------------------------------------------------------------------------
// `render_swarm_import_packet` — mirrors Python's `render_swarm_import_packet`
// ---------------------------------------------------------------------------

/// Render a control-plane import packet for a swarm delivery.
/// Mirrors `render_swarm_import_packet(delivery_dir, contract, plane, dry_run, generated)`.
pub fn render_swarm_import_packet(
    _delivery_dir: &Path,
    contract: &toml::Value,
    plane: &str,
    dry_run: bool,
    ts: Option<&str>,
) -> Result<String, UnsignedLaunchError> {
    let contract_table = contract.as_table().expect("swarm contract is a table");
    let approval = contract_table
        .get("approval")
        .and_then(toml::Value::as_table);

    let g3_signed = approval
        .and_then(|a| a.get("g3_signed"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);

    let g3_approval = approval
        .and_then(|a| a.get("g3_approval"))
        .and_then(toml::Value::as_str)
        .unwrap_or("");

    if !dry_run && !g3_signed {
        return Err(UnsignedLaunchError("G3 approval is unsigned".to_string()));
    }

    let refusal = if dry_run && !g3_signed {
        "G3 approval is unsigned"
    } else {
        ""
    };

    let deployable = !dry_run && g3_signed;
    let deployable_str = if deployable { "true" } else { "false" };
    let sandbox_str = if dry_run { "true" } else { "false" };

    let suite = contract_table
        .get("suite")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let handoff_mode = contract_table
        .get("handoff_mode")
        .and_then(toml::Value::as_str)
        .unwrap_or("");

    // members.required and members.optional
    let members_table = contract_table
        .get("members")
        .and_then(toml::Value::as_table);
    let required: String = members_table
        .and_then(|m| m.get("required"))
        .and_then(toml::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(toml::Value::as_str)
                .map(toml_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let optional: String = members_table
        .and_then(|m| m.get("optional"))
        .and_then(toml::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(toml::Value::as_str)
                .map(toml_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let g3_signed_str = if g3_signed { "true" } else { "false" };

    Ok(format!(
        "\
packet = 1
generated = {gen}
plane = {plane_str}
mode = {mode_str}
source_contract = \"swarm-contract.toml\"
suite = {suite_str}
handoff_mode = {handoff_str}
deployable = {deployable_str}
sandbox_required = {sandbox_str}
primary_reviewer_allowed = false
refusal_reason = {refusal_str}

[members]
required = [{required}]
optional = [{optional}]

[gates]
g3_signed = {g3_signed_str}
g3_approval = {g3_approval_str}
g4_required_for_write_authority = true
g5_required_for_prod_data_reingestion = true

[constraints]
member_posting = \"none\"
review_posting = \"control-plane dry run only before G3\"
write_authority = \"none\"
",
        gen = toml_str(&generated(ts)),
        plane_str = toml_str(plane),
        mode_str = toml_str(if dry_run { "dry-run" } else { "deployable" }),
        suite_str = toml_str(suite),
        handoff_str = toml_str(handoff_mode),
        deployable_str = deployable_str,
        sandbox_str = sandbox_str,
        refusal_str = toml_str(refusal),
        required = required,
        optional = optional,
        g3_signed_str = g3_signed_str,
        g3_approval_str = toml_str(g3_approval),
    ))
}

// ---------------------------------------------------------------------------
// `render_swarm_import_packet_with_repo` — internal wrapper that checks the file
// ---------------------------------------------------------------------------

/// Like `render_swarm_import_packet` but performs the G3 approval-file check.
fn render_swarm_import_packet_with_repo(
    delivery_dir: &Path,
    contract: &toml::Value,
    plane: &str,
    dry_run: bool,
    ts: Option<&str>,
    repo: &Path,
) -> Result<String, UnsignedLaunchError> {
    let contract_table = contract.as_table().expect("swarm contract is a table");
    let approval = contract_table
        .get("approval")
        .and_then(toml::Value::as_table);

    let g3_signed = approval
        .and_then(|a| a.get("g3_signed"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);

    let g3_approval = approval
        .and_then(|a| a.get("g3_approval"))
        .and_then(toml::Value::as_str)
        .unwrap_or("");

    if !dry_run {
        if !g3_signed {
            return Err(UnsignedLaunchError("G3 approval is unsigned".to_string()));
        }
        if !approval_file_approved(g3_approval, repo) {
            return Err(UnsignedLaunchError(
                "G3 approval file is missing or unsigned".to_string(),
            ));
        }
    }

    render_swarm_import_packet(delivery_dir, contract, plane, dry_run, ts)
}

// ---------------------------------------------------------------------------
// `write_import_packet` — mirrors Python's `write_import_packet`
// ---------------------------------------------------------------------------

/// Render and write an import packet, returning the written path.
/// Mirrors `write_import_packet(delivery_dir, plane, dry_run=False, generated=None, out_dir=None)`.
pub fn write_import_packet(
    delivery_dir: &Path,
    plane: &str,
    dry_run: bool,
    ts: Option<&str>,
    out_dir: Option<&Path>,
    repo: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Swarm path: if swarm-contract.toml is present, delegate.
    let text = if delivery_dir.join("swarm-contract.toml").exists() {
        let swarm_contract =
            load_swarm_if_present(delivery_dir)?.expect("swarm-contract.toml exists");
        render_swarm_import_packet_with_repo(
            delivery_dir,
            &swarm_contract,
            plane,
            dry_run,
            ts,
            repo,
        )
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
    } else {
        render_import_packet(delivery_dir, plane, dry_run, ts, repo)?
    };

    // Validate that the rendered text is valid TOML
    toml::from_str::<toml::Value>(&text).map_err(|e| {
        Box::new(ContractValidationError(format!(
            "rendered packet is invalid TOML: {e}"
        ))) as Box<dyn std::error::Error>
    })?;

    let default_dir = if dry_run {
        "launch-dry-run"
    } else {
        "launch-pack"
    };
    let out = match out_dir {
        Some(d) => d.to_path_buf(),
        None => delivery_dir.join(default_dir),
    };
    std::fs::create_dir_all(&out).map_err(|e| {
        Box::new(ContractValidationError(format!(
            "cannot create {}: {e}",
            out.display()
        ))) as Box<dyn std::error::Error>
    })?;

    let path = out.join(format!("{plane}.import-packet.toml"));
    std::fs::write(&path, &text).map_err(|e| {
        Box::new(ContractValidationError(format!(
            "cannot write {}: {e}",
            path.display()
        ))) as Box<dyn std::error::Error>
    })?;

    Ok(path)
}

// ---------------------------------------------------------------------------
// Unit tests (port of tests/test_launch.py assertions)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
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
            "daedalus-launch-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn build_delivery(tmp_path: &Path) -> (PathBuf, PathBuf) {
        let prompt = tmp_path.join("packets").join("packet.md");
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(&prompt, "Measured review prompt.\n").unwrap();

        let evidence = tmp_path.join("runs").join("demo");
        std::fs::create_dir_all(&evidence).unwrap();
        for name in &["report.md", "lineage.md", "pareto.json", "trials.jsonl"] {
            std::fs::write(evidence.join(name), "evidence\n").unwrap();
        }

        let prompt_str = prompt.display().to_string();
        let evidence_str = evidence.display().to_string();
        let trials_str = evidence.join("trials.jsonl").display().to_string();

        let contract_text = format!(
            r#"
contract = 1
agent = "demo-agent"
composition_hash = "abc123"
taskspec = "demo"
mode = "threshold-then-cheap"

[composition]
harness = "pi"
harness_version = "9.9.9"
provider = "openrouter"
model = "z-ai/glm-5"
thinking = "low"
tools = ["read", "bash"]
prompt_packet = "{prompt_str}"
timeout_sec = 600

[permissions]
workspace = "read-only checkout"
env = ["OPENROUTER_API_KEY"]
write_actions = "none"

[budgets]
max_cost_usd_per_run = 0.5
max_wall_sec = 600

[observability]
arena = "arenas/pr-review-v2"
trace_destination = "JSONL-only waiver"

[evidence]
run_dir = "{evidence_str}"
report = "{evidence_str}/report.md"
lineage = "{evidence_str}/lineage.md"
pareto = "{evidence_str}/pareto.json"
trials = "{trials_str}"

[approval]
g3_signed = false
g3_approval = "approvals/G3-demo-agent.md"
note = "unsigned"
"#
        );
        std::fs::write(tmp_path.join("contract.toml"), &contract_text).unwrap();

        (tmp_path.to_path_buf(), prompt)
    }

    #[test]
    fn unsigned_contract_refuses_deploy_packet_by_default() {
        let d = tmpdir("unsigned-deploy");
        let (delivery, _) = build_delivery(&d);
        let repo = repo_root();
        let result = write_import_packet(
            &delivery,
            "bitter-blossom",
            false,
            Some("2026-06-11T00:00:00Z"),
            None,
            &repo,
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("G3 approval is unsigned"),
            "expected UnsignedLaunchError, got: {err_str}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unsigned_contract_can_emit_sandbox_dry_run_packet() {
        let d = tmpdir("unsigned-dry-run");
        let (delivery, prompt) = build_delivery(&d);
        let repo = repo_root();
        let out_path = write_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            None,
            &repo,
        )
        .expect("dry run should succeed");

        let text = std::fs::read_to_string(&out_path).unwrap();
        let packet: toml::Value = toml::from_str(&text).unwrap();
        let p = packet.as_table().unwrap();

        assert_eq!(p["plane"].as_str(), Some("bitter-blossom"));
        assert_eq!(p["mode"].as_str(), Some("dry-run"));
        assert_eq!(p["deployable"].as_bool(), Some(false));
        assert_eq!(p["sandbox_required"].as_bool(), Some(true));
        assert_eq!(p["primary_reviewer_allowed"].as_bool(), Some(false));
        assert_eq!(
            p["refusal_reason"].as_str(),
            Some("G3 approval is unsigned")
        );

        // verify sha256
        use sha2::Digest;
        let expected_sha = format!(
            "{:x}",
            sha2::Sha256::digest(std::fs::read(&prompt).unwrap())
        );
        assert_eq!(
            p["prompt_packet_sha256"].as_str(),
            Some(expected_sha.as_str())
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn signed_contract_still_requires_signed_g3_file() {
        let d = tmpdir("signed-g3-file");
        let (delivery, _) = build_delivery(&d);
        let repo = repo_root();

        // Flip g3_signed to true
        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace("g3_signed = false", "g3_signed = true");
        std::fs::write(&contract_path, &new_text).unwrap();

        let result = write_import_packet(
            &delivery,
            "bitter-blossom",
            false,
            Some("2026-06-11T00:00:00Z"),
            None,
            &repo,
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("approval file"),
            "expected 'approval file' in: {err_str}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn contract_schema_is_validated_before_dry_run_import() {
        let d = tmpdir("schema-validate");
        let (delivery, _) = build_delivery(&d);
        let repo = repo_root();

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace("composition_hash = \"abc123\"\n", "");
        std::fs::write(&contract_path, &new_text).unwrap();

        let result = write_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            None,
            &repo,
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("composition_hash"),
            "expected 'composition_hash' in: {err_str}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn contract_write_authority_requires_g4_before_import() {
        let d = tmpdir("write-auth-g4");
        let (delivery, _) = build_delivery(&d);
        let repo = repo_root();

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace(
            "write_actions = \"none\"",
            "write_actions = \"post PR comments\"",
        );
        std::fs::write(&contract_path, &new_text).unwrap();

        let result = write_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            None,
            &repo,
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("G4"), "expected 'G4' in: {err_str}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn contract_write_authority_none_prefix_does_not_bypass_g4() {
        let d = tmpdir("write-auth-none-prefix");
        let (delivery, _) = build_delivery(&d);
        let repo = repo_root();

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace(
            "write_actions = \"none\"",
            "write_actions = \"none, except PR comments\"",
        );
        std::fs::write(&contract_path, &new_text).unwrap();

        let result = write_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            None,
            &repo,
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("G4"), "expected 'G4' in: {err_str}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn contract_write_authority_requires_signed_g4_file() {
        let d = tmpdir("write-auth-g4-file");
        let (delivery, _) = build_delivery(&d);
        let repo = repo_root();

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text
            .replace(
                "write_actions = \"none\"",
                "write_actions = \"post PR comments\"",
            )
            .replace(
                "g3_approval = \"approvals/G3-demo-agent.md\"",
                "g3_approval = \"approvals/G3-demo-agent.md\"\ng4_signed = true\ng4_approval = \"approvals/G4-demo-agent.md\"",
            );
        std::fs::write(&contract_path, &new_text).unwrap();

        let result = write_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            None,
            &repo,
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("G4 approval file"),
            "expected 'G4 approval file' in: {err_str}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
