//! Review-swarm taxonomy validation.
//!
//! Port of `runner/taxonomy.py`. The taxonomy is a human-readable Markdown
//! document with one fenced TOML block as the machine contract. The suite spec
//! names the required/optional members and thresholds. This validator checks
//! that the two agree before any arena fixtures are authored.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use regex::Regex;
use toml::Value as TomlValue;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Raised (as an error string) when the taxonomy document cannot be parsed.
#[derive(Debug)]
pub struct TaxonomyError(pub String);

impl fmt::Display for TaxonomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Mirrors `TaxonomyReport` from `runner/taxonomy.py`.
#[derive(Debug, Default)]
pub struct TaxonomyReport {
    pub ok: bool,
    pub messages: Vec<String>,
    pub lenses: Vec<String>,
    pub categories: Vec<String>,
}

impl TaxonomyReport {
    pub fn new() -> Self {
        Self {
            ok: true,
            ..Default::default()
        }
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.ok = false;
        self.messages.push(message.into());
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Load a TOML file (binary mode) — mirrors Python's `tomllib.load`.
fn load_toml(path: &Path) -> Result<TomlValue, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    text.parse::<TomlValue>()
        .map_err(|e| format!("{}: {}", path.display(), e))
}

/// Extract the fenced TOML block with `schema = "review-swarm-taxonomy.v1"`.
///
/// Replicates `re.findall(r"```toml\n(.*?)\n```", text, flags=re.DOTALL)`.
/// The `(?s)` flag in the Rust regex activates DOTALL (`.` matches newlines),
/// `.*?` is non-greedy exactly as in Python.
fn taxonomy_block(path: &Path) -> Result<TomlValue, TaxonomyError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| TaxonomyError(format!("{}: {}", path.display(), e)))?;
    // DOTALL (?s) + non-greedy .*? — same semantics as Python re.DOTALL
    let re = Regex::new(r"(?s)```toml\n(.*?)\n```").expect("valid regex");
    for cap in re.captures_iter(&text) {
        let block = cap.get(1).expect("group 1 always present").as_str();
        if block.contains(r#"schema = "review-swarm-taxonomy.v1""#) {
            return block.parse::<TomlValue>().map_err(|e| {
                TaxonomyError(format!(
                    "{}: invalid TOML in fenced block: {}",
                    path.display(),
                    e
                ))
            });
        }
    }
    Err(TaxonomyError(format!(
        "{}: missing fenced TOML block with schema review-swarm-taxonomy.v1",
        path.display()
    )))
}

/// `_require_list` — validate that `data[key]` is a non-empty list of
/// non-empty strings. Returns the list on success, empty on failure.
fn require_list(
    data: &TomlValue,
    key: &str,
    report: &mut TaxonomyReport,
    label: &str,
) -> Vec<String> {
    match data.get(key) {
        Some(TomlValue::Array(arr)) if !arr.is_empty() => {
            let strs: Option<Vec<String>> = arr
                .iter()
                .map(|v| match v {
                    TomlValue::String(s) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                })
                .collect();
            match strs {
                Some(v) => v,
                None => {
                    report.fail(format!("{label} must be a non-empty string list"));
                    vec![]
                }
            }
        }
        _ => {
            report.fail(format!("{label} must be a non-empty string list"));
            vec![]
        }
    }
}

/// `_require_thresholds`
fn require_thresholds(suite: &TomlValue, report: &mut TaxonomyReport) {
    let suite_table = suite
        .get("suite")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    for key in &["cost_ceiling_usd", "wall_ceiling_sec"] {
        let value = suite_table.get(*key);
        let is_numeric = matches!(
            value,
            Some(TomlValue::Integer(_)) | Some(TomlValue::Float(_))
        );
        if !is_numeric {
            report.fail(format!("suite.{key} must be numeric"));
        }
    }
    let thresholds = suite_table
        .get("thresholds")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    for key in &[
        "master_recall_min",
        "blocking_recall_min",
        "false_positive_carry_max",
        "duplicate_collapse_min",
    ] {
        if !thresholds.contains_key(*key) {
            report.fail(format!("suite.thresholds missing {key}"));
        }
    }
}

/// `_validate_member_artifact`
fn validate_member_artifact(
    suite: &TomlValue,
    taxonomy_data: &TomlValue,
    report: &mut TaxonomyReport,
) {
    let member = suite
        .get("member_artifact")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let schema = member.get("schema").and_then(|v| v.as_str());
    if schema != Some("review-swarm-member-artifact.v1") {
        report.fail("member_artifact.schema must be review-swarm-member-artifact.v1");
    }
    // severity_levels from taxonomy
    let severity_levels: Vec<String> = taxonomy_data
        .get("severity")
        .and_then(|v| v.get("levels"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let member_severities: Vec<String> = member
        .get("severities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if member_severities != severity_levels {
        report.fail("member_artifact.severities must match taxonomy severity levels");
    }
    for key in &["statuses", "confidences"] {
        let value = member.get(*key).and_then(|v| v.as_array());
        match value {
            Some(arr) if !arr.is_empty() => {}
            _ => {
                report.fail(format!("member_artifact.{key} must be a non-empty list"));
            }
        }
    }
}

/// `_repo_root_for_paths` — walk parents for the repo root, marked by
/// `AGENTS.md` + the workspace `Cargo.toml`. (The old `runner/` marker was
/// deleted with the Python, which silently broke this post-migration: it fell
/// back to `current_dir()`, passing locally only because of a leftover
/// `runner/` dir and failing in a clean CI checkout.)
fn repo_root_for_paths(suite_path: &Path) -> PathBuf {
    let suite_path = suite_path
        .canonicalize()
        .unwrap_or_else(|_| suite_path.to_path_buf());
    let start = suite_path.parent().unwrap_or(&suite_path).to_path_buf();
    let mut current = start.clone();
    loop {
        if current.join("AGENTS.md").exists() && current.join("Cargo.toml").exists() {
            return current;
        }
        match current.parent() {
            Some(p) if p != current => current = p.to_path_buf(),
            _ => break,
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// `_scaffold_only`
fn scaffold_only(spec: &TomlValue) -> bool {
    spec.get("scaffold")
        .and_then(|s| s.get("runnable"))
        .and_then(|v| v.as_bool())
        == Some(false)
}

/// `_validate_scaffold` — returns `true` if the spec is scaffold-only.
fn validate_scaffold(spec: &TomlValue, report: &mut TaxonomyReport, label: &str) -> bool {
    if !scaffold_only(spec) {
        return false;
    }
    let scaffold = spec
        .get("scaffold")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    if spec.get("search").is_some() {
        report.fail(format!(
            "{label} is scaffold-only and must not declare [search]"
        ));
    }
    let blocked_on = scaffold
        .get("blocked_on")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if blocked_on.is_empty() {
        report.fail(format!("{label}.scaffold.blocked_on must be non-empty"));
    }
    true
}

/// `_validate_base_packet`
fn validate_base_packet(spec: &TomlValue, base: &Path, report: &mut TaxonomyReport, label: &str) {
    if scaffold_only(spec) {
        return;
    }
    let packet = spec
        .get("search")
        .and_then(|s| s.get("base_packet"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if packet.is_empty() {
        report.fail(format!(
            "{label}.search.base_packet must be a non-empty path"
        ));
    } else if !base.join(packet).exists() {
        report.fail(format!(
            "{label}.search.base_packet does not exist: {packet}"
        ));
    }
}

/// `_validate_lens_adapter`
fn validate_lens_adapter(spec: &TomlValue, base: &Path, report: &mut TaxonomyReport, label: &str) {
    let lens = spec
        .get("lens")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    if lens.is_empty() {
        return;
    }
    if scaffold_only(spec) {
        let blocked_on = lens
            .get("blocked_on")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if blocked_on.is_empty() {
            report.fail(format!("{label}.lens.blocked_on must be non-empty"));
        }
        return;
    }
    let adapted_from = lens
        .get("adapted_from")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if adapted_from.is_empty() {
        report.fail(format!(
            "{label}.lens.adapted_from must be a non-empty path"
        ));
        return;
    }
    let arena = base.join(adapted_from);
    if !arena.is_dir() {
        report.fail(format!(
            "{label}.lens.adapted_from does not exist: {adapted_from}"
        ));
        return;
    }
    let tasks: Vec<&str> = lens
        .get("adapted_tasks")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    if tasks.is_empty() {
        report.fail(format!(
            "{label}.lens.adapted_tasks must be a non-empty list"
        ));
        return;
    }
    for task in &tasks {
        if task.is_empty() {
            report.fail(format!(
                "{label}.lens.adapted_tasks contains a non-string task"
            ));
        } else if !arena.join("tasks").join(task).is_dir() {
            report.fail(format!("{label}.lens.adapted_tasks missing task: {task}"));
        }
    }
    let authored: Vec<&str> = lens
        .get("authored_tasks")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    if !authored.is_empty() {
        let fixtures = spec
            .get("inputs")
            .and_then(|i| i.get("fixtures"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fixture_arena = if !fixtures.is_empty() {
            Some(base.join(fixtures))
        } else {
            None
        };
        let fixture_arena = match fixture_arena.filter(|p| p.is_dir()) {
            Some(p) => p,
            None => {
                report.fail(format!(
                    "{label}.inputs.fixtures must point at an arena for authored_tasks"
                ));
                return;
            }
        };
        for task in &authored {
            if task.is_empty() {
                report.fail(format!(
                    "{label}.lens.authored_tasks contains a non-string task"
                ));
            } else if !fixture_arena.join("tasks").join(task).is_dir() {
                report.fail(format!("{label}.lens.authored_tasks missing task: {task}"));
            }
        }
    }
}

/// `_validate_suite_paths`
fn validate_suite_paths(
    suite: &TomlValue,
    suite_path: &Path,
    required_members: &[String],
    optional_members: &[String],
    report: &mut TaxonomyReport,
) {
    let base = repo_root_for_paths(suite_path);
    validate_base_packet(suite, &base, report, "suite");

    let suite_table = suite
        .get("suite")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();

    let master_spec = suite_table
        .get("master_spec")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if master_spec.is_empty() {
        report.fail("suite.master_spec must be a non-empty path");
    } else if !base.join(master_spec).exists() {
        report.fail(format!("suite.master_spec does not exist: {master_spec}"));
    }

    let member_tables = suite_table.get("members");
    let member_tables = match member_tables {
        Some(TomlValue::Table(t)) => Some(t.clone()),
        Some(_) => {
            report.fail("suite.members must be a table");
            None
        }
        None => Some(toml::value::Table::new()),
    };
    let member_tables = member_tables.unwrap_or_default();

    for member in required_members {
        if !member_tables.contains_key(member.as_str()) {
            report.fail(format!("suite.members missing required member: {member}"));
        }
    }
    let allowed: HashSet<&str> = required_members
        .iter()
        .chain(optional_members.iter())
        .map(String::as_str)
        .collect();

    for (member, table) in &member_tables {
        if !allowed.contains(member.as_str()) {
            report.fail(format!("suite.members contains unknown member: {member}"));
            continue;
        }
        let t = match table {
            TomlValue::Table(t) => t,
            _ => {
                report.fail(format!("suite.members.{member} must be a table"));
                continue;
            }
        };
        for key in &["spec", "role", "status", "evidence"] {
            let v = t.get(*key).and_then(|v| v.as_str()).unwrap_or("");
            if v.is_empty() {
                report.fail(format!("suite.members.{member}.{key} must be non-empty"));
            }
        }
        for key in &["spec", "evidence"] {
            let r#ref = t.get(*key).and_then(|v| v.as_str()).unwrap_or("");
            if !r#ref.is_empty() && !base.join(r#ref).exists() {
                report.fail(format!(
                    "suite.members.{member}.{key} does not exist: {ref}"
                ));
            }
        }
        let spec_ref = t.get("spec").and_then(|v| v.as_str()).unwrap_or("");
        if !spec_ref.is_empty() && base.join(spec_ref).exists() {
            match load_toml(&base.join(spec_ref)) {
                Err(e) => {
                    report.fail(format!("suite.members.{member}.spec is invalid TOML: {e}"));
                    continue;
                }
                Ok(member_spec) => {
                    let label = format!("suite.members.{member}.spec");
                    validate_scaffold(&member_spec, report, &label);
                    validate_base_packet(&member_spec, &base, report, &label);
                    validate_lens_adapter(&member_spec, &base, report, &label);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate taxonomy doc + suite taskspec. Mirrors `validate_taxonomy`.
pub fn validate_taxonomy(taxonomy_path: &Path, suite_path: &Path) -> TaxonomyReport {
    let mut report = TaxonomyReport::new();

    let taxonomy_data = match taxonomy_block(taxonomy_path) {
        Ok(v) => v,
        Err(e) => {
            report.fail(e.to_string());
            return report;
        }
    };
    let suite = match load_toml(suite_path) {
        Ok(v) => v,
        Err(e) => {
            report.fail(format!("{}: invalid suite spec: {e}", suite_path.display()));
            return report;
        }
    };

    // Schema check
    let schema = taxonomy_data
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if schema != "review-swarm-taxonomy.v1" {
        report.fail("taxonomy schema must be review-swarm-taxonomy.v1");
    }

    let lenses = require_list(&taxonomy_data, "lenses", &mut report, "lenses");
    let required_lenses = require_list(
        &taxonomy_data,
        "required_lenses",
        &mut report,
        "required_lenses",
    );
    let optional_lenses = require_list(
        &taxonomy_data,
        "optional_lenses",
        &mut report,
        "optional_lenses",
    );
    report.lenses = lenses.clone();
    let lens_set: HashSet<&str> = lenses.iter().map(String::as_str).collect();

    for lens in required_lenses.iter().chain(optional_lenses.iter()) {
        if !lens_set.contains(lens.as_str()) {
            report.fail(format!("declared lens not present in lenses: {lens}"));
        }
    }

    let suite_table = suite
        .get("suite")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let required_members: Vec<String> = suite_table
        .get("required_members")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let optional_members: Vec<String> = suite_table
        .get("optional_members")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    for member in &required_members {
        if !lens_set.contains(member.as_str()) {
            report.fail(format!(
                "required member missing from taxonomy lenses: {member}"
            ));
        }
    }
    for member in &optional_members {
        if !lens_set.contains(member.as_str()) {
            report.fail(format!(
                "optional member missing from taxonomy lenses: {member}"
            ));
        }
    }
    let req_set: HashSet<&str> = required_members.iter().map(String::as_str).collect();
    let opt_set: HashSet<&str> = optional_members.iter().map(String::as_str).collect();
    let overlap_set: HashSet<&&str> = req_set.intersection(&opt_set).collect();
    if !overlap_set.is_empty() {
        let mut sorted: Vec<&str> = overlap_set.iter().map(|&&s| s).collect();
        sorted.sort_unstable();
        report.fail(format!(
            "members cannot be both required and optional: {}",
            sorted.join(", ")
        ));
    }

    // Severity
    let severity = taxonomy_data
        .get("severity")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let levels: Vec<String> = severity
        .get("levels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let expected_levels = ["blocking", "serious", "minor"];
    if levels.iter().map(String::as_str).collect::<Vec<_>>() != expected_levels {
        report.fail("severity.levels must be blocking, serious, minor");
    }
    let blocking_rule = severity
        .get("blocking_rule")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if blocking_rule.is_empty() {
        report.fail("severity.blocking_rule must not be empty");
    }

    // Categories: [[category]] in TOML is an array of tables
    let categories = taxonomy_data
        .get("category")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if categories.is_empty() {
        report.fail("at least one [[category]] is required");
    }
    let mut seen_categories: HashSet<String> = HashSet::new();
    let mut categories_by_lens: HashMap<&str, usize> =
        lenses.iter().map(|l| (l.as_str(), 0)).collect();

    for category in &categories {
        let cid = category.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let lens_val = category.get("lens").and_then(|v| v.as_str()).unwrap_or("");
        if cid.is_empty() {
            report.fail("category missing id");
            continue;
        }
        if seen_categories.contains(cid) {
            report.fail(format!("duplicate category id: {cid}"));
        }
        seen_categories.insert(cid.to_string());
        report.categories.push(cid.to_string());
        if !lens_set.contains(lens_val) {
            report.fail(format!("category {cid} uses unknown lens: {lens_val}"));
        } else if let Some(cnt) = categories_by_lens.get_mut(lens_val) {
            *cnt += 1;
        }
        for key in &["description", "blocking_rule"] {
            let v = category.get(*key).and_then(|v| v.as_str()).unwrap_or("");
            if v.is_empty() {
                report.fail(format!("category {cid} missing {key}"));
            }
        }
        let overlaps_val = category.get("allowed_overlaps");
        let overlaps: Vec<&str> = match overlaps_val {
            Some(TomlValue::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
            Some(_) => {
                report.fail(format!("category {cid} allowed_overlaps must be a list"));
                vec![]
            }
            None => vec![],
        };
        for item in &overlaps {
            if !lens_set.contains(*item) {
                report.fail(format!("category {cid} uses unknown overlap lens: {item}"));
            }
        }
    }
    for member in &required_members {
        if categories_by_lens
            .get(member.as_str())
            .copied()
            .unwrap_or(0)
            == 0
        {
            report.fail(format!(
                "required member has no taxonomy category: {member}"
            ));
        }
    }

    // Overlaps: [[overlap]]
    let overlaps = taxonomy_data
        .get("overlap")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut seen_overlaps: HashSet<String> = HashSet::new();
    for item in &overlaps {
        let oid = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if oid.is_empty() {
            report.fail("overlap missing id");
            continue;
        }
        if seen_overlaps.contains(oid) {
            report.fail(format!("duplicate overlap id: {oid}"));
        }
        seen_overlaps.insert(oid.to_string());
        let owner = item.get("owner").and_then(|v| v.as_str()).unwrap_or("");
        if !lens_set.contains(owner) {
            report.fail(format!("overlap {oid} owner uses unknown lens: {owner}"));
        }
        let overlap_lenses: Vec<&str> = item
            .get("lenses")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        if overlap_lenses.len() < 2 {
            report.fail(format!("overlap {oid} must name at least two lenses"));
        }
        for lens in &overlap_lenses {
            if !lens_set.contains(*lens) {
                report.fail(format!("overlap {oid} uses unknown lens: {lens}"));
            }
        }
        if !owner.is_empty() && !overlap_lenses.contains(&owner) {
            report.fail(format!("overlap {oid} owner must be one of its lenses"));
        }
        let rule = item.get("rule").and_then(|v| v.as_str()).unwrap_or("");
        if rule.is_empty() {
            report.fail(format!("overlap {oid} missing rule"));
        }
    }

    require_thresholds(&suite, &mut report);
    validate_member_artifact(&suite, &taxonomy_data, &mut report);
    validate_suite_paths(
        &suite,
        suite_path,
        &required_members,
        &optional_members,
        &mut report,
    );

    report
}

/// Render the validation report. Mirrors `render_report`.
pub fn render_report(report: &TaxonomyReport) -> String {
    let status = if report.ok { "PASS" } else { "FAIL" };
    let lenses_str = if report.lenses.is_empty() {
        "-".to_string()
    } else {
        report.lenses.join(", ")
    };
    let categories_str = if report.categories.is_empty() {
        "-".to_string()
    } else {
        report.categories.join(", ")
    };
    let mut lines = vec![
        format!("Taxonomy validation: {status}"),
        format!("lenses: {lenses_str}"),
        format!("categories: {categories_str}"),
    ];
    if !report.messages.is_empty() {
        lines.push("findings:".to_string());
        for msg in &report.messages {
            lines.push(format!("- {msg}"));
        }
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Internal unit tests (port of tests/test_taxonomy.py assertions)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("repo root")
            .to_path_buf()
    }

    fn write_taxonomy(dir: &Path, body: Option<&str>) -> PathBuf {
        let path = dir.join("taxonomy.md");
        let content = match body {
            Some(s) => s.to_string(),
            None => std::fs::read_to_string(repo_root().join("docs/review-swarm-taxonomy.md"))
                .expect("taxonomy file"),
        };
        std::fs::write(&path, content).unwrap();
        path
    }

    fn write_suite(dir: &Path, extra: &str) -> PathBuf {
        let path = dir.join("suite.toml");
        std::fs::write(
            &path,
            format!(
                r#"
id = "pr-review-suite"
mode = "threshold-then-cheap"

[suite]
master_spec = "specs/pr-review-master/taskspec.toml"
required_members = ["general", "correctness", "security"]
optional_members = ["verification", "simplification", "product"]
cost_ceiling_usd = 2.0
wall_ceiling_sec = 1200

[suite.thresholds]
master_recall_min = 0.9
blocking_recall_min = 1.0
false_positive_carry_max = 1
duplicate_collapse_min = 0.9

[suite.members.general]
spec = "specs/pr-review/taskspec.toml"
role = "baseline"
status = "ready"
evidence = "deliveries/pr-review/DELIVERY.md"

[suite.members.correctness]
spec = "specs/pr-review-correctness/taskspec.toml"
role = "correctness"
status = "ready"
evidence = "docs/review-swarm-vertical-slice.md"

[suite.members.security]
spec = "specs/pr-review-security/taskspec.toml"
role = "security"
status = "ready"
evidence = "docs/review-swarm-vertical-slice.md"

[member_artifact]
schema = "review-swarm-member-artifact.v1"
statuses = ["ok", "error", "timeout", "truncated"]
severities = ["blocking", "serious", "minor"]
confidences = ["high", "medium", "low"]

[search]
base_packet = "packets/reviewer-v1.md"
{extra}
"#
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn review_swarm_taxonomy_validates_against_suite_spec() {
        let root = repo_root();
        let report = validate_taxonomy(
            &root.join("docs/review-swarm-taxonomy.md"),
            &root.join("specs/pr-review-suite/taskspec.toml"),
        );
        assert!(report.ok, "{:?}", report.messages);
        assert_eq!(
            report.lenses,
            [
                "general",
                "correctness",
                "security",
                "verification",
                "simplification",
                "product"
            ]
        );
    }

    #[test]
    fn taxonomy_rejects_missing_required_lens() {
        let dir = tempdir();
        let source =
            std::fs::read_to_string(repo_root().join("docs/review-swarm-taxonomy.md")).unwrap();
        let broken = source.replace(r#""security", "#, "");
        let tax_path = write_taxonomy(&dir, Some(&broken));
        let suite_path = write_suite(&dir, "");
        let report = validate_taxonomy(&tax_path, &suite_path);
        assert!(!report.ok);
        assert!(
            report
                .messages
                .iter()
                .any(|m| m.contains("required member missing from taxonomy lenses: security")),
            "{:?}",
            report.messages
        );
    }

    #[test]
    fn taxonomy_rejects_category_for_unknown_lens() {
        let dir = tempdir();
        let source =
            std::fs::read_to_string(repo_root().join("docs/review-swarm-taxonomy.md")).unwrap();
        // Replace only the first occurrence (matches Python's replace(..., 1))
        let broken = source.replacen(r#"lens = "security""#, r#"lens = "compliance""#, 1);
        let tax_path = write_taxonomy(&dir, Some(&broken));
        let suite_path = write_suite(&dir, "");
        let report = validate_taxonomy(&tax_path, &suite_path);
        assert!(!report.ok);
        assert!(
            report
                .messages
                .iter()
                .any(|m| m.contains("category credential-exposure uses unknown lens: compliance")),
            "{:?}",
            report.messages
        );
    }

    #[test]
    fn taxonomy_rejects_suite_without_thresholds() {
        let dir = tempdir();
        let tax_path = write_taxonomy(&dir, None);
        let suite_path = write_suite(&dir, "");
        // Remove master_recall_min line
        let text = std::fs::read_to_string(&suite_path).unwrap();
        let patched = text.replace("master_recall_min = 0.9\n", "");
        std::fs::write(&suite_path, patched).unwrap();
        let report = validate_taxonomy(&tax_path, &suite_path);
        assert!(!report.ok);
        assert!(
            report
                .messages
                .iter()
                .any(|m| m.contains("suite.thresholds missing master_recall_min")),
            "{:?}",
            report.messages
        );
    }

    #[test]
    fn taxonomy_rejects_missing_member_spec_path() {
        let dir = tempdir();
        let tax_path = write_taxonomy(&dir, None);
        let suite_path = write_suite(&dir, "");
        let text = std::fs::read_to_string(&suite_path).unwrap();
        let patched = text.replace(
            r#"spec = "specs/pr-review-security/taskspec.toml""#,
            r#"spec = "specs/pr-review-security/MISSING.toml""#,
        );
        std::fs::write(&suite_path, patched).unwrap();
        let report = validate_taxonomy(&tax_path, &suite_path);
        assert!(!report.ok);
        assert!(
            report
                .messages
                .iter()
                .any(|m| m.contains("suite.members.security.spec does not exist")),
            "{:?}",
            report.messages
        );
    }

    #[test]
    fn scaffold_only_specs_must_not_declare_search() {
        let mut report = TaxonomyReport::new();
        let spec: TomlValue = r#"
[scaffold]
runnable = false
blocked_on = "fixtures"
[search]
base_packet = "packets/reviewer-v1.md"
"#
        .parse()
        .unwrap();
        validate_scaffold(&spec, &mut report, "scaffold-spec");
        assert!(!report.ok);
        assert!(
            report
                .messages
                .iter()
                .any(|m| m.contains("must not declare [search]")),
            "{:?}",
            report.messages
        );
    }

    #[test]
    fn scaffold_only_specs_skip_base_packet_requirement() {
        let mut report = TaxonomyReport::new();
        let spec: TomlValue = r#"
[scaffold]
runnable = false
blocked_on = "fixtures"
"#
        .parse()
        .unwrap();
        let root = repo_root();
        validate_base_packet(&spec, &root, &mut report, "scaffold-spec");
        assert!(report.ok, "{:?}", report.messages);
    }

    fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "threshold-taxonomy-test-{}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
