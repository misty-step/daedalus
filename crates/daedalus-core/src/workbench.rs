//! Arena authoring and calibration helpers.
//!
//! Port of `runner/workbench.py`. The workbench is intentionally file-first:
//! it creates Harbor-shaped task placeholders, validates frozen arena surfaces,
//! records human adjudications, and reports scoring disagreements without
//! mutating scorer constants.
//!
//! All public functions faithfully replicate the Python semantics, including:
//! - dict insertion order in generated files (serde_json with preserve_order)
//! - `round(x, 4)` via `pycompat::round_half_even`
//! - version tuple comparison via `version_tuple`
//! - `None`/absent handling for probe_run / optional fields
//! - stable sort order matching Python's `sorted()`

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use toml::Value as TomlValue;

use crate::pycompat::round_half_even;
use crate::score::score;

// ---------------------------------------------------------------------------
// Template constants — must match runner/workbench.py exactly.
// ---------------------------------------------------------------------------

const VERIFY_SH: &str = "#!/usr/bin/env sh
set -eu
HERE=$(cd \"$(dirname \"$0\")\" && pwd)
WORKDIR=${1:-$PWD}
daedalus score \"$WORKDIR/findings.json\" \"$HERE/expected.json\"
";

const DEFAULT_TEMPLATE: &str = "{intent}

Return ONLY findings.json with this shape:
{\"findings\": [{\"file\": \"...\", \"line\": 1, \"category\": \"...\", \"description\": \"...\"}]}
";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Raised for invalid authoring/calibration operations.
#[derive(Debug, Clone)]
pub struct WorkbenchError(pub String);

impl std::fmt::Display for WorkbenchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WorkbenchError {}

// ---------------------------------------------------------------------------
// ValidationReport
// ---------------------------------------------------------------------------

/// Result of validating an arena freeze gate.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub arena_id: String,
    pub arena_version: String,
    pub ok: bool,
    pub messages: Vec<String>,
    /// Non-failing advisories (e.g. contamination: a source is public).
    pub warnings: Vec<String>,
    pub oracle_mean: Option<f64>,
    pub null_mean: Option<f64>,
    pub probe_mean: Option<f64>,
    pub probe_errors: Option<i64>,
    pub probe_trials: Option<i64>,
    pub probe_verdict: Option<String>,
    pub holdout_counts: HashMap<String, i64>,
}

impl ValidationReport {
    fn new(arena_id: &str, arena_version: &str) -> Self {
        ValidationReport {
            arena_id: arena_id.to_string(),
            arena_version: arena_version.to_string(),
            ok: true,
            messages: Vec::new(),
            warnings: Vec::new(),
            oracle_mean: None,
            null_mean: None,
            probe_mean: None,
            probe_errors: None,
            probe_trials: None,
            probe_verdict: None,
            holdout_counts: HashMap::new(),
        }
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.ok = false;
        self.messages.push(message.into());
    }

    /// A non-failing advisory.
    pub fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }
}

// ---------------------------------------------------------------------------
// contamination record (backlog 040 item 1)
// ---------------------------------------------------------------------------

/// One upstream source an arena's fixtures are drawn from.
#[derive(Debug, Clone, Deserialize)]
pub struct ContaminationSource {
    pub repo: String,
    #[serde(default)]
    pub r#ref: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    /// Publicly indexable → plausibly in model training data.
    #[serde(default)]
    pub public: bool,
}

/// `contamination.toml` beside `arena.toml`: which upstream code the fixtures
/// come from, and whether the planted defects are novel.
#[derive(Debug, Clone, Deserialize)]
pub struct Contamination {
    /// All planted defects are authored for this arena (not upstream bugs).
    #[serde(default)]
    pub defects_novel: bool,
    #[serde(default)]
    pub source: Vec<ContaminationSource>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Load `<arena>/contamination.toml`. `Ok(None)` when absent.
pub fn load_contamination(arena_dir: &Path) -> Result<Option<Contamination>, WorkbenchError> {
    let path = arena_dir.join("contamination.toml");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| WorkbenchError(format!("contamination.toml: {e}")))?;
    toml::from_str(&text)
        .map(Some)
        .map_err(|e| WorkbenchError(format!("contamination.toml: {e}")))
}

/// Validate the contamination record (040 items 1 & 4). A real-repo arena —
/// any task declaring a `source_repo` — MUST carry a record; a synthetic arena
/// need not, but if it has one it is validated too. The record must list its
/// sources and assert the defects are novel. Public sources are surfaced as
/// contamination advisories; an all-private record is blessed as a
/// contamination-resistant holdout.
pub fn validate_contamination(arena_dir: &Path, report: &mut ValidationReport) {
    let labeled = task_dirs(arena_dir)
        .iter()
        .any(|td| crate::run::source_repo(td).is_some());
    match load_contamination(arena_dir) {
        Ok(None) => {
            if labeled {
                report.fail(
                    "real-repo arena (tasks declare source_repo) is missing contamination.toml (040 item 1)",
                );
            }
            // Unlabeled and no record → synthetic arena, nothing to validate.
        }
        Ok(Some(c)) => {
            if c.source.is_empty() {
                report.fail("contamination.toml lists no [[source]] entries");
            }
            if !c.defects_novel {
                report.fail(
                    "contamination.toml must assert defects_novel = true (planted defects are authored, not upstream bugs)",
                );
            }
            let all_private = !c.source.is_empty() && c.source.iter().all(|s| !s.public);
            if all_private {
                report.warn(
                    "contamination-resistant: all sources are private/synthetic — suitable as a holdout (040 item 4)",
                );
            } else {
                for s in &c.source {
                    if s.public {
                        report.warn(format!(
                            "contamination: source {} is public — plausibly in model training data; \
                             pair with a contamination-resistant holdout before trusting rankings",
                            s.repo
                        ));
                    }
                }
            }
        }
        Err(e) => report.fail(e.0),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn load_toml(path: &Path) -> Result<TomlValue, WorkbenchError> {
    let text =
        fs::read_to_string(path).map_err(|e| WorkbenchError(format!("{}: {e}", path.display())))?;
    text.parse::<TomlValue>()
        .map_err(|e| WorkbenchError(format!("{}: {e}", path.display())))
}

/// Return sorted task subdirectories from `<arena_dir>/tasks/`.
fn task_dirs(arena_dir: &Path) -> Vec<PathBuf> {
    let tasks = arena_dir.join("tasks");
    let mut dirs: Vec<PathBuf> = match fs::read_dir(&tasks) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect(),
        Err(_) => Vec::new(),
    };
    dirs.sort();
    dirs
}

/// Replicate Python's `_version_tuple`: extract numeric parts and convert to
/// a comparable tuple.
///
/// ```text
/// "1.2.3" -> [1, 2, 3]
/// "0.10.0" -> [0, 10, 0]
/// ```
pub fn version_tuple(version: &str) -> Vec<u64> {
    let re = Regex::new(r"\d+").expect("static regex");
    re.find_iter(version)
        .map(|m| m.as_str().parse::<u64>().unwrap_or(0))
        .collect()
}

// ---------------------------------------------------------------------------
// scaffold_task
// ---------------------------------------------------------------------------

/// Create a new Harbor-format task scaffold and minimal arena metadata.
///
/// Port of Python `scaffold_task(arena_dir, task_id, taskspec)`.
pub fn scaffold_task(
    arena_dir: &Path,
    task_id: &str,
    taskspec: &str,
) -> Result<PathBuf, WorkbenchError> {
    let task_dir = arena_dir.join("tasks").join(task_id);
    if task_dir.exists() {
        return Err(WorkbenchError(format!(
            "task already exists: {}",
            task_dir.display()
        )));
    }

    fs::create_dir_all(arena_dir).map_err(|e| WorkbenchError(format!("create arena_dir: {e}")))?;
    let arena_id = arena_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arena");
    let arena_toml = arena_dir.join("arena.toml");
    if !arena_toml.exists() {
        fs::write(
            &arena_toml,
            format!(
                "id = \"{arena_id}\"\nversion = \"0.1.0\"\ntaskspec = \"{taskspec}\"\n\n[template]\nfile = \"template.md\"\n\n[risk]\nclass = \"low\"\nnotes = \"Scaffold placeholder; review before any candidate run.\"\n\n[split]\ntrain = []\nvalidation = []\nholdout = []\n"
            ),
        )
        .map_err(|e| WorkbenchError(format!("write arena.toml: {e}")))?;
    }
    let template = arena_dir.join("template.md");
    if !template.exists() {
        fs::write(&template, DEFAULT_TEMPLATE)
            .map_err(|e| WorkbenchError(format!("write template.md: {e}")))?;
    }

    // task_dir/environment/README.md
    fs::create_dir_all(task_dir.join("environment"))
        .map_err(|e| WorkbenchError(format!("create environment/: {e}")))?;
    fs::write(
        task_dir.join("environment").join("README.md"),
        "Replace this placeholder with the candidate-visible fixture files.\n",
    )
    .map_err(|e| WorkbenchError(format!("write environment/README.md: {e}")))?;

    // task_dir/tests/
    fs::create_dir_all(task_dir.join("tests"))
        .map_err(|e| WorkbenchError(format!("create tests/: {e}")))?;
    fs::write(
        task_dir.join("tests").join("expected.json"),
        "{\"defects\": []}\n",
    )
    .map_err(|e| WorkbenchError(format!("write expected.json: {e}")))?;
    let test_sh = task_dir.join("tests").join("test.sh");
    fs::write(&test_sh, VERIFY_SH).map_err(|e| WorkbenchError(format!("write test.sh: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&test_sh, fs::Permissions::from_mode(0o755))
            .map_err(|e| WorkbenchError(format!("chmod test.sh: {e}")))?;
    }

    // task_dir/solution/findings.json
    fs::create_dir_all(task_dir.join("solution"))
        .map_err(|e| WorkbenchError(format!("create solution/: {e}")))?;
    fs::write(
        task_dir.join("solution").join("findings.json"),
        "{\"findings\": []}\n",
    )
    .map_err(|e| WorkbenchError(format!("write findings.json: {e}")))?;

    // task_dir/intent.md
    fs::write(
        task_dir.join("intent.md"),
        "Describe the task-specific review intent here.\n",
    )
    .map_err(|e| WorkbenchError(format!("write intent.md: {e}")))?;

    // task_dir/task.toml
    fs::write(
        task_dir.join("task.toml"),
        format!(
            "id = \"{task_id}\"\n\n[agent]\ntimeout_sec = 600\n\n[verifier]\ntimeout_sec = 60\n"
        ),
    )
    .map_err(|e| WorkbenchError(format!("write task.toml: {e}")))?;

    Ok(task_dir)
}

// ---------------------------------------------------------------------------
// validate_expected_shape
// ---------------------------------------------------------------------------

/// Validate the shape of an `expected.json` file.
///
/// Returns the list of defects on success, or raises `WorkbenchError`.
/// Port of Python `_validate_expected_shape(path)`.
pub fn validate_expected_shape(path: &Path) -> Result<Vec<Value>, WorkbenchError> {
    let text = fs::read_to_string(path)
        .map_err(|e| WorkbenchError(format!("{}: invalid expected.json: {e}", path.display())))?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| WorkbenchError(format!("{}: invalid expected.json: {e}", path.display())))?;
    let defects = root
        .get("defects")
        .ok_or_else(|| {
            WorkbenchError(format!(
                "{}: invalid expected.json: missing 'defects' key",
                path.display()
            ))
        })?
        .clone();
    let defects = match defects {
        Value::Array(arr) => arr,
        _ => {
            return Err(WorkbenchError(format!(
                "{}: defects must be a list",
                path.display()
            )))
        }
    };
    let required = ["id", "file", "line_start", "line_end", "category"];
    for (i, defect) in defects.iter().enumerate() {
        let idx = i + 1; // Python's enumerate(start=1)
        let obj = match defect.as_object() {
            Some(o) => o,
            None => {
                return Err(WorkbenchError(format!(
                    "{}: defect {idx} must be an object",
                    path.display()
                )))
            }
        };
        let present: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
        let mut missing: Vec<&str> = required
            .iter()
            .copied()
            .filter(|k| !present.contains(k))
            .collect();
        if !missing.is_empty() {
            missing.sort();
            return Err(WorkbenchError(format!(
                "{}: defect {idx} missing {}",
                path.display(),
                missing.join(", ")
            )));
        }
        // Check inverted span
        let line_start = defect["line_start"]
            .as_i64()
            .or_else(|| defect["line_start"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        let line_end = defect["line_end"]
            .as_i64()
            .or_else(|| defect["line_end"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        if line_start > line_end {
            let id = defect["id"].as_str().unwrap_or("?");
            return Err(WorkbenchError(format!(
                "{}: defect {id} has inverted span",
                path.display()
            )));
        }
    }
    Ok(defects)
}

// ---------------------------------------------------------------------------
// validate_no_symlinks
// ---------------------------------------------------------------------------

/// Check that a task directory contains no symlinks.
///
/// Port of Python `_validate_no_symlinks(task_dir)`.
pub fn validate_no_symlinks(task_dir: &Path) -> Result<(), WorkbenchError> {
    fn rglob(dir: &Path) -> Result<(), WorkbenchError> {
        for entry in fs::read_dir(dir)
            .map_err(|e| WorkbenchError(format!("read_dir {}: {e}", dir.display())))?
        {
            let entry = entry.map_err(|e| WorkbenchError(format!("dir entry: {e}")))?;
            let path = entry.path();
            // is_symlink checks the path itself (not following symlink)
            if path.is_symlink() {
                return Err(WorkbenchError(format!(
                    "fixture contains symlink: {}",
                    path.display()
                )));
            }
            let ft = entry
                .file_type()
                .map_err(|e| WorkbenchError(format!("file_type: {e}")))?;
            if ft.is_dir() {
                rglob(&path)?;
            }
        }
        Ok(())
    }
    rglob(task_dir)
}

// ---------------------------------------------------------------------------
// validate_splits
// ---------------------------------------------------------------------------

/// Validate split membership.
///
/// Returns a map of split name → set of task IDs.
/// Port of Python `_validate_splits(arena, task_ids, report)`.
pub fn validate_splits(
    arena: &TomlValue,
    task_ids: &BTreeSet<String>,
    report: &mut ValidationReport,
) -> HashMap<String, BTreeSet<String>> {
    let split = arena.get("split");
    let bucket_names = ["train", "validation", "holdout"];
    let mut buckets: HashMap<String, BTreeSet<String>> = HashMap::new();
    for name in &bucket_names {
        let ids = split
            .and_then(|s| s.get(name))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<BTreeSet<String>>()
            })
            .unwrap_or_default();
        buckets.insert(name.to_string(), ids);
    }

    let mut assigned: HashMap<String, String> = HashMap::new(); // task_id -> split_name
                                                                // Python iterates buckets in insertion order: train, validation, holdout
    for name in &bucket_names {
        let ids = buckets.get(*name).cloned().unwrap_or_default();
        // Python iterates a set; we iterate a BTreeSet (sorted) to be deterministic
        // Python does not guarantee order here but we need determinism for tests
        for task_id in &ids {
            if let Some(existing) = assigned.get(task_id) {
                report.fail(format!(
                    "task assigned to multiple splits: {task_id} ({existing}, {name})"
                ));
            }
            assigned.insert(task_id.clone(), name.to_string());
            if !task_ids.contains(task_id) {
                report.fail(format!("split references missing task: {task_id}"));
            }
        }
    }
    // Python: sorted(task_ids - set(assigned))
    let mut missing: Vec<String> = task_ids
        .iter()
        .filter(|id| !assigned.contains_key(*id))
        .cloned()
        .collect();
    if !missing.is_empty() {
        missing.sort();
        report.fail(format!("not assigned to any split: {}", missing.join(", ")));
    }
    buckets
}

// ---------------------------------------------------------------------------
// saturation probe verdict (backlog 040)
// ---------------------------------------------------------------------------

/// Outcome of the one-shot saturation probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeVerdict {
    /// The probe scored near the oracle — the arena cannot rank agents.
    Saturated,
    /// The probe cleanly scored low — the arena discriminates skill.
    Unsaturated,
    /// The probe errored (context overflow, HTTP failure). Its low score is an
    /// artifact, not evidence — it says nothing about saturation either way.
    Inconclusive,
}

/// Classify the saturation probe, distinguishing "probe errored" from "probe
/// genuinely scored low" (backlog 040).
///
/// An errored probe records reward 0.0, which deflates `probe_mean` toward the
/// "unsaturated" side and would silently pass a meaningless arena (pr-review-v2
/// errored to 0.0 on context overflow and was read as a pass). So an errored
/// probe must NOT count as evidence of non-saturation:
/// - every probe trial errored, or no trials → `Inconclusive`;
/// - scored near the oracle (≥ oracle − 0.1) → `Saturated` (high despite any
///   errors is still saturated);
/// - scored low with *some* errors → `Inconclusive` (the low may be the errors,
///   not genuine skill-floor);
/// - scored low with no errors → `Unsaturated` (a clean signal).
pub fn probe_saturation_verdict(
    probe_mean: f64,
    oracle_mean: f64,
    probe_errors: i64,
    probe_trials: i64,
) -> ProbeVerdict {
    if probe_trials <= 0 || probe_errors >= probe_trials {
        return ProbeVerdict::Inconclusive;
    }
    if probe_mean >= oracle_mean - 0.1 {
        ProbeVerdict::Saturated
    } else if probe_errors > 0 {
        ProbeVerdict::Inconclusive
    } else {
        ProbeVerdict::Unsaturated
    }
}

// ---------------------------------------------------------------------------
// validate_probe_run
// ---------------------------------------------------------------------------

/// Validate the one-shot probe run summary.
///
/// Port of Python `_validate_probe_run(probe_run, report)`.
pub fn validate_probe_run(probe_run: Option<&Path>, report: &mut ValidationReport) {
    let probe_run = match probe_run {
        None => {
            report.fail("one-shot probe not checked: pass --probe-run");
            return;
        }
        Some(p) => p,
    };
    let summary_path = probe_run.join("summary.json");
    if !summary_path.exists() {
        report.fail(format!(
            "one-shot probe summary missing: {}",
            summary_path.display()
        ));
        return;
    }
    let text = match fs::read_to_string(&summary_path) {
        Ok(t) => t,
        Err(e) => {
            report.fail(format!(
                "one-shot probe summary missing: {}: {e}",
                summary_path.display()
            ));
            return;
        }
    };
    let summary: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            report.fail(format!("probe run summary parse error: {e}"));
            return;
        }
    };

    // Find oracle record: first value where kind=="oracle", else key "oracle"
    let oracle = summary
        .as_object()
        .and_then(|obj| {
            obj.values()
                .find(|v| v.get("kind").and_then(|k| k.as_str()) == Some("oracle"))
        })
        .or_else(|| summary.get("oracle"));

    // Find probe record: first value where kind=="oneshot", else key "probe-oneshot"
    let probe = summary
        .as_object()
        .and_then(|obj| {
            obj.values()
                .find(|v| v.get("kind").and_then(|k| k.as_str()) == Some("oneshot"))
        })
        .or_else(|| summary.get("probe-oneshot"));

    let (oracle, probe) = match (oracle, probe) {
        (Some(o), Some(p)) if !o.is_null() && !p.is_null() => (o, p),
        _ => {
            report.fail("probe run must include oracle and one-shot records");
            return;
        }
    };

    let probe_mean = match probe.get("reward_mean").and_then(|v| v.as_f64()) {
        Some(f) => f,
        None => {
            report.fail("probe run must include oracle and one-shot records");
            return;
        }
    };
    let oracle_mean = match oracle.get("reward_mean").and_then(|v| v.as_f64()) {
        Some(f) => f,
        None => {
            report.fail("probe run must include oracle and one-shot records");
            return;
        }
    };

    report.probe_mean = Some(probe_mean);
    let probe_errors = probe.get("errors").and_then(|v| v.as_i64()).unwrap_or(0);
    let probe_trials = probe.get("trials").and_then(|v| v.as_i64()).unwrap_or(0);
    report.probe_errors = Some(probe_errors);
    report.probe_trials = Some(probe_trials);
    let verdict = probe_saturation_verdict(probe_mean, oracle_mean, probe_errors, probe_trials);
    report.probe_verdict = Some(
        match verdict {
            ProbeVerdict::Saturated => "saturated",
            ProbeVerdict::Unsaturated => "unsaturated",
            ProbeVerdict::Inconclusive => "inconclusive",
        }
        .to_string(),
    );
    match verdict {
        ProbeVerdict::Saturated => report.fail(format!(
            "one-shot probe saturates the arena: {probe_mean:.4} >= oracle {oracle_mean:.4} - 0.1"
        )),
        ProbeVerdict::Inconclusive => report.fail(format!(
            "one-shot probe inconclusive: {probe_errors}/{probe_trials} trials errored, so its \
             {probe_mean:.4} mean is not evidence the arena is unsaturated"
        )),
        ProbeVerdict::Unsaturated => {}
    }
}

// ---------------------------------------------------------------------------
// holdout_counts
// ---------------------------------------------------------------------------

/// Count per-task holdout exposures from the ledger file.
///
/// Port of Python `_holdout_counts(arena_dir, holdout_tasks, arena_version)`.
pub fn holdout_counts(
    arena_dir: &Path,
    holdout_tasks: &[String],
    arena_version: Option<&str>,
) -> HashMap<String, i64> {
    let mut counts: HashMap<String, i64> = holdout_tasks.iter().map(|t| (t.clone(), 0)).collect();
    if holdout_tasks.is_empty() {
        return counts;
    }
    let ledger = arena_dir.join("holdout-ledger.md");
    if !ledger.exists() {
        return counts;
    }
    let text = match fs::read_to_string(&ledger) {
        Ok(t) => t,
        Err(_) => return counts,
    };
    let version_re = Regex::new(r"^\d+\.\d+\.\d+$").expect("static regex");
    for line in text.lines() {
        if !line.starts_with('|') || line.contains("---") || line.contains("tasks") {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        if cells.len() < 4 {
            continue;
        }
        // Check version column
        let version_cell = if version_re.is_match(cells[1]) {
            Some(cells[1])
        } else {
            None
        };
        if let (Some(av), Some(vc)) = (arena_version, version_cell) {
            if vc != av {
                continue;
            }
        }
        let tasks_cell = cells[cells.len() - 1];
        for task in holdout_tasks {
            if !tasks_cell.contains(task.as_str()) {
                continue;
            }
            // Check for multiplier: task x3 or task ×3
            let pattern = format!(r"{}\s*[x×]\s*(\d+)", regex::escape(task));
            let mul_re = Regex::new(&pattern).expect("dynamic regex");
            let count = if let Some(caps) = mul_re.find(tasks_cell).and_then(|_| {
                Regex::new(&pattern)
                    .ok()
                    .and_then(|re| re.captures(tasks_cell))
            }) {
                caps[1].parse::<i64>().unwrap_or(1)
            } else {
                1
            };
            *counts.entry(task.clone()).or_insert(0) += count;
        }
    }
    counts
}

// ---------------------------------------------------------------------------
// format_holdout_ledger_row
// ---------------------------------------------------------------------------

/// Format a holdout ledger row for appending to `holdout-ledger.md`.
///
/// Port of Python `format_holdout_ledger_row(stamp, run_name, candidates,
/// holdout_tasks, trials_per_candidate, arena_version)`.
pub fn format_holdout_ledger_row(
    stamp: &str,
    run_name: &str,
    candidates: &[&str],
    holdout_tasks: &[&str],
    trials_per_candidate: usize,
    arena_version: Option<&str>,
) -> String {
    // Parse date from first 8 chars of stamp: "20260612T220412Z" → "2026-06-12"
    let date = if stamp.len() >= 8 {
        let y = &stamp[0..4];
        let m = &stamp[4..6];
        let d = &stamp[6..8];
        format!("{y}-{m}-{d}")
    } else {
        stamp.to_string()
    };
    let exposure_count = candidates.len() * trials_per_candidate;
    let holdout_cell: Vec<String> = holdout_tasks
        .iter()
        .map(|t| format!("{t} x{exposure_count}"))
        .collect();
    let holdout_cell = holdout_cell.join(", ");
    if let Some(version) = arena_version {
        format!(
            "| {date} | {version} | {run_name} | holdout final: {} | {holdout_cell} |\n",
            candidates.join(", ")
        )
    } else {
        format!(
            "| {date} | {run_name} | {} | {holdout_cell} |\n",
            candidates.join(", ")
        )
    }
}

// ---------------------------------------------------------------------------
// validate_arena
// ---------------------------------------------------------------------------

/// Validate an arena freeze gate without spending model budget.
///
/// Port of Python `validate_arena(arena_dir, probe_run, holdout_burn)`.
pub fn validate_arena(
    arena_dir: &Path,
    probe_run: Option<&Path>,
    holdout_burn: i64,
) -> Result<ValidationReport, WorkbenchError> {
    let arena = load_toml(&arena_dir.join("arena.toml"))?;
    let arena_id = arena
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let arena_version = arena
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut report = ValidationReport::new(&arena_id, &arena_version);

    let task_dir_list = task_dirs(arena_dir);
    let task_ids: BTreeSet<String> = task_dir_list
        .iter()
        .filter_map(|d| {
            d.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .collect();

    let split_buckets = validate_splits(&arena, &task_ids, &mut report);

    let mut oracle_rewards: Vec<f64> = Vec::new();
    let mut null_rewards: Vec<f64> = Vec::new();

    // Create a temporary null findings file. The name carries a process-wide
    // atomic sequence so concurrent validations (e.g. parallel tests) cannot
    // collide on the same timestamp and delete each other's scratch dir.
    static SCRATCH_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp_dir = std::env::temp_dir().join(format!(
        "daedalus-workbench-{}-{}-{}",
        std::process::id(),
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        },
        SCRATCH_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&tmp_dir).map_err(|e| WorkbenchError(format!("create tmpdir: {e}")))?;
    let null_findings = tmp_dir.join("findings.json");
    fs::write(&null_findings, "{\"findings\": []}\n")
        .map_err(|e| WorkbenchError(format!("write null findings: {e}")))?;

    for task_dir in &task_dir_list {
        let task_name = task_dir.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        // _validate_no_symlinks
        if let Err(e) = validate_no_symlinks(task_dir) {
            report.fail(e.0);
            continue;
        }
        // _validate_expected_shape
        let expected_path = task_dir.join("tests").join("expected.json");
        let defects = match validate_expected_shape(&expected_path) {
            Ok(d) => d,
            Err(e) => {
                report.fail(e.0);
                continue;
            }
        };

        // Oracle score
        let oracle_findings = task_dir.join("solution").join("findings.json");
        match score(&oracle_findings, &expected_path) {
            Ok(r) => {
                oracle_rewards.push(r.reward);
                if r.reward != 1.0 {
                    report.fail(format!("oracle is not 1.0 on {task_name}"));
                }
            }
            Err(e) => {
                report.fail(format!("score error on {task_name}: {e}"));
            }
        }

        // Null score
        match score(&null_findings, &expected_path) {
            Ok(r) => {
                null_rewards.push(r.reward);
                let expected_null = if defects.is_empty() { 1.0 } else { 0.0 };
                if r.reward != expected_null {
                    report.fail(format!(
                        "null floor mismatch on {task_name}: {} != {expected_null}",
                        r.reward
                    ));
                }
            }
            Err(e) => {
                report.fail(format!("null score error on {task_name}: {e}"));
            }
        }
    }

    let _ = fs::remove_dir_all(&tmp_dir);

    // Compute means using Python's round(sum/len, 4)
    if !oracle_rewards.is_empty() {
        let mean = oracle_rewards.iter().sum::<f64>() / oracle_rewards.len() as f64;
        report.oracle_mean = Some(round_half_even(mean, 4));
    }
    if !null_rewards.is_empty() {
        let mean = null_rewards.iter().sum::<f64>() / null_rewards.len() as f64;
        report.null_mean = Some(round_half_even(mean, 4));
    }

    validate_probe_run(probe_run, &mut report);

    // Holdout counts
    let holdout_set = split_buckets.get("holdout").cloned().unwrap_or_default();
    let mut holdout_tasks_sorted: Vec<String> = holdout_set.into_iter().collect();
    holdout_tasks_sorted.sort();

    report.holdout_counts = holdout_counts(arena_dir, &holdout_tasks_sorted, Some(&arena_version));

    if !holdout_tasks_sorted.is_empty() && !arena_dir.join("holdout-ledger.md").exists() {
        report.fail("holdout ledger missing");
    }

    for task in &holdout_tasks_sorted {
        let count = report.holdout_counts.get(task).copied().unwrap_or(0);
        if count >= holdout_burn {
            report.fail(format!(
                "holdout task burned: {task} has {count} exposures (threshold {holdout_burn})"
            ));
        }
    }

    // Backlog 040 item 1: real-repo arenas must carry a contamination record.
    validate_contamination(arena_dir, &mut report);

    Ok(report)
}

// ---------------------------------------------------------------------------
// render_validation_report
// ---------------------------------------------------------------------------

/// Render a validation report as a Markdown string.
///
/// Port of Python `render_validation_report(report)`.
pub fn render_validation_report(report: &ValidationReport) -> String {
    let status = if report.ok { "PASS" } else { "FAIL" };
    // holdout_counts: sort keys like Python's json.dumps(sort_keys=True)
    let holdout_json = {
        let mut sorted: Vec<(&String, &i64)> = report.holdout_counts.iter().collect();
        sorted.sort_by_key(|(k, _)| *k);
        let pairs: Vec<String> = sorted
            .iter()
            .map(|(k, v)| format!("\"{k}\": {v}"))
            .collect();
        format!("{{{}}}", pairs.join(", "))
    };
    let mut lines: Vec<String> = vec![
        format!(
            "# Arena freeze report: {} {}",
            report.arena_id, report.arena_version
        ),
        String::new(),
        format!("Status: **{status}**"),
        String::new(),
        "| check | value |".to_string(),
        "|---|---|".to_string(),
        format!("| oracle mean | `{}` |", format_opt_f64(report.oracle_mean)),
        format!("| null mean | `{}` |", format_opt_f64(report.null_mean)),
        format!(
            "| one-shot probe mean | `{}` |",
            format_opt_f64(report.probe_mean)
        ),
        format!(
            "| one-shot probe verdict | `{}` |",
            report.probe_verdict.as_deref().unwrap_or("None")
        ),
        format!(
            "| one-shot probe errors | `{}` |",
            format_opt_i64(report.probe_errors)
        ),
        format!(
            "| one-shot probe trials | `{}` |",
            format_opt_i64(report.probe_trials)
        ),
        format!("| holdout exposures | `{holdout_json}` |"),
        String::new(),
    ];
    if !report.messages.is_empty() {
        lines.push("## Findings".to_string());
        lines.push(String::new());
        for m in &report.messages {
            lines.push(format!("- {m}"));
        }
        lines.push(String::new());
    }
    if !report.warnings.is_empty() {
        lines.push("## Advisories".to_string());
        lines.push(String::new());
        for w in &report.warnings {
            lines.push(format!("- {w}"));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

/// Format an `Option<f64>` as Python's `repr(None)` / `str(float)`.
///
/// Python's `f"{v}"` for a float always produces at least one decimal digit:
/// `1.0` → `"1.0"`, `0.5` → `"0.5"`, `0.0` → `"0.0"`. Rust's `{}` for
/// `1.0_f64` produces `"1"`. We replicate Python by detecting integer-valued
/// floats and appending `.0`.
fn format_opt_f64(v: Option<f64>) -> String {
    match v {
        None => "None".to_string(),
        Some(f) => format_py_float(f),
    }
}

fn format_opt_i64(v: Option<i64>) -> String {
    v.map(|n| n.to_string())
        .unwrap_or_else(|| "None".to_string())
}

/// Format an f64 the way Python's str()/f-string does: always at least one
/// decimal digit (e.g. `1.0` → `"1.0"`, `0.5` → `"0.5"`).
fn format_py_float(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 {
        format!("{f}.0")
    } else {
        format!("{f}")
    }
}

// ---------------------------------------------------------------------------
// replace_version (internal)
// ---------------------------------------------------------------------------

fn replace_version(arena_toml: &Path, old: &str, new: &str) -> Result<(), WorkbenchError> {
    let text = fs::read_to_string(arena_toml)
        .map_err(|e| WorkbenchError(format!("read arena.toml: {e}")))?;
    let pattern = format!("version = \"{old}\"");
    if !text.contains(&pattern) {
        return Err(WorkbenchError(format!(
            "could not find version line for {old}"
        )));
    }
    let new_text = text.replacen(&pattern, &format!("version = \"{new}\""), 1);
    fs::write(arena_toml, &new_text)
        .map_err(|e| WorkbenchError(format!("write arena.toml: {e}")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// record_adjudication
// ---------------------------------------------------------------------------

/// Append a human adjudication and enforce ACCEPT version discipline.
///
/// Port of Python `record_adjudication(arena_dir, task, finding, ruling,
/// rationale, new_version, baseline_run)`.
pub fn record_adjudication(
    arena_dir: &Path,
    task: &str,
    finding: &str,
    ruling: &str,
    rationale: &str,
    new_version: Option<&str>,
    baseline_run: Option<&Path>,
) -> Result<PathBuf, WorkbenchError> {
    let arena_toml = arena_dir.join("arena.toml");
    let arena = load_toml(&arena_toml)?;
    let current_version = arena
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let arena_id = arena
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ruling = ruling.to_uppercase();
    if ruling != "ACCEPT" && ruling != "OUT-OF-SCOPE" {
        return Err(WorkbenchError(
            "ruling must be ACCEPT or OUT-OF-SCOPE".to_string(),
        ));
    }
    let mut version_note = current_version.clone();
    if ruling == "ACCEPT" {
        let new_ver = new_version
            .ok_or_else(|| WorkbenchError("ACCEPT requires --new-version".to_string()))?;
        if version_tuple(new_ver) <= version_tuple(&current_version) {
            return Err(WorkbenchError("ACCEPT requires a version bump".to_string()));
        }
        if baseline_run.is_none() {
            return Err(WorkbenchError("ACCEPT requires --baseline-run".to_string()));
        }
        let baseline_report = validate_arena(
            arena_dir,
            baseline_run,
            5, // default holdout_burn
        )?;
        if !baseline_report.ok {
            return Err(WorkbenchError(format!(
                "baseline rerun failed: {}",
                baseline_report.messages.join("; ")
            )));
        }
        replace_version(&arena_toml, &current_version, new_ver)?;
        version_note = format!("{current_version} -> {new_ver}");
    }

    let path = arena_dir.join("adjudications.md");
    if !path.exists() {
        fs::write(
            &path,
            format!(
                "# Answer-key adjudications - {arena_id}\n\n| id | date | task | finding | ruling |\n|---|---|---|---|---|\n"
            ),
        )
        .map_err(|e| WorkbenchError(format!("write adjudications.md: {e}")))?;
    }
    // Count existing ADJ entries
    let existing_text = fs::read_to_string(&path)
        .map_err(|e| WorkbenchError(format!("read adjudications.md: {e}")))?;
    let re = Regex::new(r"\| ADJ-\d+ \|").expect("static regex");
    let existing = re.find_iter(&existing_text).count();
    let adj_id = format!("ADJ-{}", existing + 1);

    // Date in UTC: Python uses datetime.now(timezone.utc).strftime("%Y-%m-%d")
    let date = utc_date_today();

    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .map_err(|e| WorkbenchError(format!("open adjudications.md: {e}")))?;
    write!(
        f,
        "| {adj_id} | {date} | {task} | {finding} | **{ruling}** |\n\n\
         ## {adj_id} - {task} ({ruling})\n\n\
         - **Rationale:** {rationale}\n\
         - **Arena version:** {version_note}\n"
    )
    .map_err(|e| WorkbenchError(format!("append adjudications.md: {e}")))?;
    if let Some(br) = baseline_run {
        writeln!(f, "- **Baseline run:** `{}`", br.display())
            .map_err(|e| WorkbenchError(format!("append baseline run: {e}")))?;
    }
    writeln!(f).map_err(|e| WorkbenchError(format!("append newline: {e}")))?;

    Ok(path)
}

/// Return today's UTC date as `%Y-%m-%d`.
///
/// Mirrors Python `datetime.now(timezone.utc).strftime("%Y-%m-%d")`.
fn utc_date_today() -> String {
    use crate::pycompat::civil_from_days;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------
// disagreements
// ---------------------------------------------------------------------------

/// Report category/span misses without changing scorer constants.
///
/// Port of Python `disagreements(findings_path, expected_path)`.
/// Returns a list of disagreement objects, each with at minimum:
/// - `kind`: "category" or "span"
/// - `finding`: the original finding object
/// - `defect_id`: the matched defect's id
/// - `expected_category` (for "category" kind)
/// - `expected_span`: [line_start, line_end] (for "span" kind)
pub fn disagreements(
    findings_path: &Path,
    expected_path: &Path,
) -> Result<Vec<Value>, WorkbenchError> {
    let findings_text = fs::read_to_string(findings_path)
        .map_err(|e| WorkbenchError(format!("read findings: {e}")))?;
    let findings_root: Value = serde_json::from_str(&findings_text)
        .map_err(|e| WorkbenchError(format!("parse findings: {e}")))?;
    let findings = findings_root
        .get("findings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let expected_text = fs::read_to_string(expected_path)
        .map_err(|e| WorkbenchError(format!("read expected: {e}")))?;
    let expected_root: Value = serde_json::from_str(&expected_text)
        .map_err(|e| WorkbenchError(format!("parse expected: {e}")))?;
    let defects = expected_root
        .get("defects")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut rows: Vec<Value> = Vec::new();

    for finding in &findings {
        let file = match finding.get("file").and_then(|v| v.as_str()) {
            Some(f) => f,
            None => continue,
        };
        // Python: int(finding.get("line")) — TypeError/ValueError → continue
        let line = match finding.get("line").and_then(coerce_line_int) {
            Some(l) => l,
            None => continue,
        };
        let category = match finding.get("category").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => continue,
        };

        // Exact match: file == d.file AND category == d.category AND line in [start,end]
        let exact: Vec<&Value> = defects
            .iter()
            .filter(|d| {
                d.get("file").and_then(|v| v.as_str()) == Some(file)
                    && d.get("category").and_then(|v| v.as_str()) == Some(category)
                    && defect_line_start(d) <= line
                    && line <= defect_line_end(d)
            })
            .collect();
        if !exact.is_empty() {
            continue;
        }

        // In-span match: file == d.file AND line in [start,end] (different category)
        let in_span: Vec<&Value> = defects
            .iter()
            .filter(|d| {
                d.get("file").and_then(|v| v.as_str()) == Some(file)
                    && defect_line_start(d) <= line
                    && line <= defect_line_end(d)
            })
            .collect();
        if !in_span.is_empty() {
            let d = in_span[0];
            let defect_id = d
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let expected_cat = d
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            rows.push(serde_json::json!({
                "kind": "category",
                "finding": finding,
                "defect_id": defect_id,
                "expected_category": expected_cat,
            }));
            continue;
        }

        // Same-category match: file == d.file AND category == d.category (different span)
        let same_cat: Vec<&Value> = defects
            .iter()
            .filter(|d| {
                d.get("file").and_then(|v| v.as_str()) == Some(file)
                    && d.get("category").and_then(|v| v.as_str()) == Some(category)
            })
            .collect();
        if !same_cat.is_empty() {
            let d = same_cat[0];
            let defect_id = d
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let line_start = d.get("line_start").cloned().unwrap_or(Value::Null);
            let line_end = d.get("line_end").cloned().unwrap_or(Value::Null);
            rows.push(serde_json::json!({
                "kind": "span",
                "finding": finding,
                "defect_id": defect_id,
                "expected_span": [line_start, line_end],
            }));
        }
    }

    Ok(rows)
}

/// Coerce a JSON value to `i64` matching Python's `int(x)` semantics for
/// the `line` field in findings — TypeError/ValueError → None.
fn coerce_line_int(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u as i64))
            .or_else(|| n.as_f64().map(|f| f.trunc() as i64)),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        Value::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

fn defect_line_start(d: &Value) -> i64 {
    d.get("line_start")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(i64::MAX)
}

fn defect_line_end(d: &Value) -> i64 {
    d.get("line_end")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(i64::MIN)
}

// ---------------------------------------------------------------------------
// Unit tests — port of tests/test_workbench.py
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn errored_probe_is_inconclusive_not_unsaturated() {
        // The pr-review-v2 bug: the probe errored to reward 0.0 on every trial
        // (context overflow). That must NOT read as "unsaturated".
        assert_eq!(
            probe_saturation_verdict(0.0, 1.0, 3, 3),
            ProbeVerdict::Inconclusive
        );
        // No probe trials at all → also inconclusive.
        assert_eq!(
            probe_saturation_verdict(0.0, 1.0, 0, 0),
            ProbeVerdict::Inconclusive
        );
    }

    #[test]
    fn high_probe_is_saturated_even_with_some_errors() {
        // 0.95 ≥ oracle 1.0 − 0.1: scoring high despite errors is still saturated.
        assert_eq!(
            probe_saturation_verdict(0.95, 1.0, 1, 3),
            ProbeVerdict::Saturated
        );
    }

    #[test]
    fn low_probe_with_errors_is_inconclusive_but_clean_low_is_unsaturated() {
        // Low score with some errors: the low may be the errors, not skill-floor.
        assert_eq!(
            probe_saturation_verdict(0.2, 1.0, 1, 3),
            ProbeVerdict::Inconclusive
        );
        // Low score, no errors: a clean signal that the arena discriminates.
        assert_eq!(
            probe_saturation_verdict(0.2, 1.0, 0, 3),
            ProbeVerdict::Unsaturated
        );
    }

    fn write_labeled_task(arena: &Path, source_repo: Option<&str>) {
        let task = arena.join("tasks").join("t1");
        std::fs::create_dir_all(&task).unwrap();
        let toml = match source_repo {
            Some(r) => format!("id = \"t1\"\nsource_repo = \"{r}\"\n"),
            None => "id = \"t1\"\n".to_string(),
        };
        std::fs::write(task.join("task.toml"), toml).unwrap();
    }

    #[test]
    fn validate_contamination_requires_a_record_for_real_repo_arenas() {
        let arena = tmpdir("contam-labeled");
        std::fs::create_dir_all(&arena).unwrap();
        write_labeled_task(&arena, Some("rich"));

        // Labeled arena, no record → fail.
        let mut r1 = ValidationReport::new("a", "0");
        validate_contamination(&arena, &mut r1);
        assert!(!r1.ok);
        assert!(r1.messages.iter().any(|m| m.contains("contamination.toml")));

        // Add a valid record with a public source → passes with an advisory.
        std::fs::write(
            arena.join("contamination.toml"),
            "defects_novel = true\n[[source]]\nrepo = \"rich\"\npublic = true\n",
        )
        .unwrap();
        let mut r2 = ValidationReport::new("a", "0");
        validate_contamination(&arena, &mut r2);
        assert!(r2.ok);
        assert!(r2.warnings.iter().any(|w| w.contains("public")));
        let _ = std::fs::remove_dir_all(&arena);
    }

    #[test]
    fn validate_contamination_requires_defects_novel() {
        let arena = tmpdir("contam-novel");
        std::fs::create_dir_all(&arena).unwrap();
        write_labeled_task(&arena, Some("rich"));
        std::fs::write(
            arena.join("contamination.toml"),
            "defects_novel = false\n[[source]]\nrepo = \"rich\"\n",
        )
        .unwrap();
        let mut report = ValidationReport::new("a", "0");
        validate_contamination(&arena, &mut report);
        assert!(!report.ok);
        assert!(report.messages.iter().any(|m| m.contains("defects_novel")));
        let _ = std::fs::remove_dir_all(&arena);
    }

    #[test]
    fn validate_contamination_skips_synthetic_arenas() {
        let arena = tmpdir("contam-synth");
        std::fs::create_dir_all(&arena).unwrap();
        write_labeled_task(&arena, None); // no source_repo → synthetic
        let mut report = ValidationReport::new("a", "0");
        validate_contamination(&arena, &mut report);
        assert!(report.ok); // no contamination record required
        let _ = std::fs::remove_dir_all(&arena);
    }

    #[test]
    fn validate_contamination_blesses_an_all_private_holdout() {
        let arena = tmpdir("contam-private");
        std::fs::create_dir_all(&arena).unwrap();
        write_labeled_task(&arena, None); // synthetic
        std::fs::write(
            arena.join("contamination.toml"),
            "defects_novel = true\n[[source]]\nrepo = \"synthetic\"\npublic = false\n",
        )
        .unwrap();
        let mut report = ValidationReport::new("a", "0");
        validate_contamination(&arena, &mut report);
        assert!(report.ok); // a valid all-private record passes
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("contamination-resistant")));
        let _ = std::fs::remove_dir_all(&arena);
    }

    #[test]
    fn validate_probe_run_fails_the_freeze_gate_on_an_errored_probe() {
        let dir = tmpdir("errored-probe");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("summary.json"),
            serde_json::to_string(&serde_json::json!({
                "oracle": {"kind": "oracle", "reward_mean": 1.0, "trials": 3, "errors": 0},
                // probe errored to 0.0 on every trial (the pr-review-v2 bug).
                "probe-oneshot": {"kind": "oneshot", "reward_mean": 0.0, "trials": 3, "errors": 3},
            }))
            .unwrap(),
        )
        .unwrap();
        let mut report = ValidationReport::new("a", "0.0.0");
        validate_probe_run(Some(&dir), &mut report);
        assert!(!report.ok, "errored probe must not pass the freeze gate");
        assert_eq!(report.probe_errors, Some(3));
        assert_eq!(report.probe_trials, Some(3));
        assert_eq!(report.probe_verdict.as_deref(), Some("inconclusive"));
        let rendered = render_validation_report(&report);
        assert!(rendered.contains("| one-shot probe errors | `3` |"));
        assert!(rendered.contains("| one-shot probe trials | `3` |"));
        assert!(report.messages.iter().any(|m| m.contains("inconclusive")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmpdir(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let d =
            std::env::temp_dir().join(format!("daedalus-wb-{}-{n}-{label}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Build a minimal valid arena (mirrors test_workbench.py::write_arena).
    fn write_arena(tmp: &Path) -> PathBuf {
        let arena = tmp.join("arena");
        let buggy = arena.join("tasks").join("buggy");
        let clean = arena.join("tasks").join("clean");

        // buggy task
        fs::create_dir_all(buggy.join("environment")).unwrap();
        fs::write(buggy.join("environment").join("app.py"), "print('bug')\n").unwrap();
        fs::write(buggy.join("intent.md"), "Find the bug.\n").unwrap();
        fs::create_dir_all(buggy.join("tests")).unwrap();
        fs::write(
            buggy.join("tests").join("expected.json"),
            serde_json::to_string(&serde_json::json!({
                "defects": [{
                    "id": "bug",
                    "file": "app.py",
                    "line_start": 1,
                    "line_end": 1,
                    "category": "correctness",
                    "note": "seeded"
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(buggy.join("tests").join("test.sh"), "#!/usr/bin/env sh\n").unwrap();
        fs::create_dir_all(buggy.join("solution")).unwrap();
        fs::write(
            buggy.join("solution").join("findings.json"),
            serde_json::to_string(&serde_json::json!({
                "findings": [{
                    "file": "app.py",
                    "line": 1,
                    "category": "correctness",
                    "description": "bug"
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            buggy.join("task.toml"),
            "id = \"buggy\"\n\n[agent]\ntimeout_sec = 600\n\n[verifier]\ntimeout_sec = 60\n",
        )
        .unwrap();

        // clean task
        fs::create_dir_all(clean.join("environment")).unwrap();
        fs::write(clean.join("environment").join("app.py"), "print('ok')\n").unwrap();
        fs::write(clean.join("intent.md"), "Confirm clean.\n").unwrap();
        fs::create_dir_all(clean.join("tests")).unwrap();
        fs::write(
            clean.join("tests").join("expected.json"),
            "{\"defects\": []}\n",
        )
        .unwrap();
        fs::write(clean.join("tests").join("test.sh"), "#!/usr/bin/env sh\n").unwrap();
        fs::create_dir_all(clean.join("solution")).unwrap();
        fs::write(
            clean.join("solution").join("findings.json"),
            "{\"findings\": []}\n",
        )
        .unwrap();
        fs::write(
            clean.join("task.toml"),
            "id = \"clean\"\n\n[agent]\ntimeout_sec = 600\n\n[verifier]\ntimeout_sec = 60\n",
        )
        .unwrap();

        fs::write(
            arena.join("template.md"),
            "{intent}\nReturn findings.json.\n",
        )
        .unwrap();
        fs::write(
            arena.join("arena.toml"),
            "\nid = \"sample\"\nversion = \"0.1.0\"\ntaskspec = \"specs/sample/taskspec.toml\"\n\n[template]\nfile = \"template.md\"\n\n[risk]\nclass = \"low\"\n\n[split]\ntrain = [\"buggy\"]\nvalidation = [\"clean\"]\nholdout = []\n",
        )
        .unwrap();
        arena
    }

    fn write_probe_run(tmp: &Path) -> PathBuf {
        let run = tmp.join("run");
        fs::create_dir_all(&run).unwrap();
        fs::write(
            run.join("summary.json"),
            serde_json::to_string(&serde_json::json!({
                "oracle": {"kind": "oracle", "reward_mean": 1.0},
                "null": {"kind": "null", "reward_mean": 0.5},
                "probe-oneshot": {"kind": "oneshot", "reward_mean": 0.0, "trials": 1, "errors": 0},
            }))
            .unwrap(),
        )
        .unwrap();
        run
    }

    #[test]
    fn scaffold_task_creates_harbor_placeholders() {
        let tmp = tmpdir("scaffold");
        let arena = tmp.join("new-arena");
        let task = scaffold_task(&arena, "new-task", "specs/x.toml").expect("scaffold_task failed");
        assert!(arena.join("arena.toml").exists());
        assert!(arena.join("template.md").exists());
        assert!(task.join("intent.md").exists());
        assert!(task.join("environment").join("README.md").exists());
        let expected: Value = serde_json::from_str(
            &fs::read_to_string(task.join("tests").join("expected.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(expected, serde_json::json!({"defects": []}));
        let findings: Value = serde_json::from_str(
            &fs::read_to_string(task.join("solution").join("findings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(findings, serde_json::json!({"findings": []}));
        let test_sh = fs::read_to_string(task.join("tests").join("test.sh")).unwrap();
        assert!(test_sh.contains("daedalus score"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn validate_arena_checks_oracle_null_probe_and_splits() {
        let tmp = tmpdir("validate");
        let arena = write_arena(&tmp);
        let probe = write_probe_run(&tmp);
        let report = validate_arena(&arena, Some(&probe), 5).expect("validate_arena failed");
        assert!(report.ok, "{:?}", report.messages);
        assert_eq!(report.oracle_mean, Some(1.0));
        assert_eq!(report.null_mean, Some(0.5));
        assert_eq!(report.probe_mean, Some(0.0));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn validate_arena_reports_missing_split_membership() {
        let tmp = tmpdir("split");
        let arena = write_arena(&tmp);
        let text = fs::read_to_string(arena.join("arena.toml")).unwrap();
        let new_text = text.replace("validation = [\"clean\"]", "validation = []");
        fs::write(arena.join("arena.toml"), &new_text).unwrap();
        let probe = write_probe_run(&tmp);
        let report = validate_arena(&arena, Some(&probe), 5).expect("validate_arena failed");
        assert!(!report.ok);
        assert!(
            report
                .messages
                .iter()
                .any(|m| m.contains("not assigned to any split: clean")),
            "messages: {:?}",
            report.messages
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn holdout_ledger_version_column_scopes_burn_count() {
        let tmp = tmpdir("ledger");
        let arena = write_arena(&tmp);
        let text = fs::read_to_string(arena.join("arena.toml")).unwrap();
        let text = text
            .replace("version = \"0.1.0\"", "version = \"0.2.0\"")
            .replace("train = [\"buggy\"]", "train = []")
            .replace("holdout = []", "holdout = [\"buggy\"]");
        fs::write(arena.join("arena.toml"), &text).unwrap();
        fs::write(
            arena.join("holdout-ledger.md"),
            "| date | arena version | run | purpose | tasks |\n\
             |---|---|---|---|---|\n\
             | 2026-06-12 | 0.1.0 | old-run | old baseline | buggy x9 |\n\
             | 2026-06-12 | 0.2.0 | new-run | new baseline | buggy x1 |\n",
        )
        .unwrap();
        let probe = write_probe_run(&tmp);
        let report = validate_arena(&arena, Some(&probe), 5).expect("validate_arena failed");
        assert!(report.ok, "{:?}", report.messages);
        assert_eq!(report.holdout_counts.get("buggy").copied(), Some(1));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn format_holdout_ledger_row_records_version_and_exposure_count() {
        let row = format_holdout_ledger_row(
            "20260612T220412Z",
            "runs-search",
            &["cand-a", "cand-b"],
            &["holdout-a", "holdout-b"],
            3,
            Some("0.2.0"),
        );
        assert_eq!(
            row,
            "| 2026-06-12 | 0.2.0 | runs-search | holdout final: cand-a, cand-b | holdout-a x6, holdout-b x6 |\n"
        );
    }

    #[test]
    fn adjudicate_accept_requires_version_bump_and_baselines() {
        let tmp = tmpdir("adjudicate");
        let arena = write_arena(&tmp);
        // Should fail: no new_version
        let err = record_adjudication(
            &arena,
            "buggy",
            "candidate found missing issue",
            "ACCEPT",
            "key missed it",
            None,
            None,
        )
        .unwrap_err();
        assert!(err.0.contains("--new-version"), "err: {}", err.0);

        // Should succeed with version + baseline
        let probe = write_probe_run(&tmp);
        record_adjudication(
            &arena,
            "buggy",
            "candidate found missing issue",
            "ACCEPT",
            "key missed it",
            Some("0.2.0"),
            Some(&probe),
        )
        .expect("record_adjudication failed");

        let toml_text = fs::read_to_string(arena.join("arena.toml")).unwrap();
        let parsed: TomlValue = toml::from_str(&toml_text).unwrap();
        assert_eq!(
            parsed.get("version").and_then(|v| v.as_str()),
            Some("0.2.0")
        );
        let adj_text = fs::read_to_string(arena.join("adjudications.md")).unwrap();
        assert!(adj_text.contains("ACCEPT"));
        assert!(adj_text.contains("0.1.0 -> 0.2.0"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn disagreements_report_category_and_span_misses() {
        let tmp = tmpdir("disagree");
        let expected = tmp.join("expected.json");
        fs::write(
            &expected,
            serde_json::to_string(&serde_json::json!({
                "defects": [{
                    "id": "escape",
                    "file": "app.py",
                    "line_start": 10,
                    "line_end": 12,
                    "category": "security"
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        let findings = tmp.join("findings.json");
        fs::write(
            &findings,
            serde_json::to_string(&serde_json::json!({
                "findings": [
                    {"file": "app.py", "line": 11, "category": "correctness"},
                    {"file": "app.py", "line": 14, "category": "security"},
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let rows = disagreements(&findings, &expected).expect("disagreements failed");
        let kinds: Vec<&str> = rows
            .iter()
            .map(|r| r.get("kind").and_then(|v| v.as_str()).unwrap_or(""))
            .collect();
        assert_eq!(kinds, vec!["category", "span"]);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn version_tuple_parses_correctly() {
        assert_eq!(version_tuple("0.1.0"), vec![0, 1, 0]);
        assert_eq!(version_tuple("1.10.3"), vec![1, 10, 3]);
        assert_eq!(version_tuple("0.2.0"), vec![0, 2, 0]);
        assert!(version_tuple("0.2.0") > version_tuple("0.1.0"));
    }

    #[test]
    fn render_validation_report_pass() {
        let mut report = ValidationReport::new("my-arena", "0.1.0");
        report.oracle_mean = Some(1.0);
        report.null_mean = Some(0.5);
        report.probe_mean = Some(0.0);
        let text = render_validation_report(&report);
        assert!(text.contains("PASS"));
        assert!(text.contains("oracle mean"));
        assert!(!text.contains("## Findings"));
    }

    #[test]
    fn render_validation_report_fail_with_messages() {
        let mut report = ValidationReport::new("my-arena", "0.1.0");
        report.fail("something went wrong");
        let text = render_validation_report(&report);
        assert!(text.contains("FAIL"));
        assert!(text.contains("## Findings"));
        assert!(text.contains("- something went wrong"));
    }
}
