//! Deterministic core of `runner/run.py`, ported to Rust.
//!
//! **Scope** — pure, deterministic helpers only. Live-I/O glue
//! (`run_oneshot`, `run_pi`, `harness_version`, `main`) is lead-owned and not
//! included here. The boundary is the same as the Python module: these
//! functions are callable without a network, subprocess, or temp-dir side
//! effect.
//!
//! Every public function is parity-tested in `tests/parity_run.rs` against the
//! Python original. See `docs/rust-migration.md` for migration status.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::pycompat::{py_json_dumps, round_half_even};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Raised when an arena requires isolation stronger than temp dirs.
#[derive(Debug)]
pub struct LocalExecutionRefused(pub String);

impl std::fmt::Display for LocalExecutionRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for LocalExecutionRefused {}

/// Raised when candidate-visible text exposes hidden grader paths.
#[derive(Debug)]
pub struct GraderPathLeak(pub String);

impl std::fmt::Display for GraderPathLeak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for GraderPathLeak {}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LOCAL_SAFE_RISK_CLASSES: &[&str] = &["low"];

/// Maps TOML risk flag keys → human-readable labels.
const LOCAL_REFUSAL_FLAGS: &[(&str, &str)] = &[
    ("needs_network", "network"),
    ("needs_secrets", "secrets"),
    ("adversarial_fixtures", "adversarial fixtures"),
    ("user_data", "user data"),
];

// ---------------------------------------------------------------------------
// tree_digest
// ---------------------------------------------------------------------------

/// Stable hash over file paths + contents, for grader tamper detection.
///
/// Mirrors:
/// ```python
/// h = hashlib.sha256()
/// for root in roots:
///     for f in sorted(Path(root).rglob("*")):
///         if f.is_file():
///             h.update(str(f.relative_to(root)).encode())
///             h.update(f.read_bytes())
/// return h.hexdigest()
/// ```
///
/// Python's `Path.rglob("*")` returns all entries; `sorted()` sorts by their
/// string representation lexicographically. Only files (not directories)
/// contribute to the hash — same as Python's `if f.is_file()` guard.
pub fn tree_digest(roots: &[&Path]) -> String {
    let mut hasher = Sha256::new();
    for &root in roots {
        // Collect all entries recursively, then sort by relative path string.
        let mut files: Vec<(String, PathBuf)> = Vec::new();
        collect_files(root, root, &mut files);
        // Python sorts Path objects lexicographically (by their string form).
        files.sort_by(|a, b| a.0.cmp(&b.0));
        for (rel_str, abs_path) in &files {
            if abs_path.is_file() {
                hasher.update(rel_str.as_bytes());
                if let Ok(bytes) = std::fs::read(abs_path) {
                    hasher.update(&bytes);
                }
            }
        }
    }
    format!("{:x}", hasher.finalize())
}

/// Recursively walk `dir`, collecting (relative_path_str, absolute_path) for
/// every entry (files and dirs alike — the `is_file()` filter is in the caller,
/// matching Python's `if f.is_file()` guard). Uses forward slashes on all
/// platforms so the sort order matches Python's `str(f.relative_to(root))`.
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Build the relative path string exactly as Python does:
        // `str(f.relative_to(root))` uses forward slashes on POSIX.
        let rel = path
            .strip_prefix(root)
            .expect("path under root")
            .to_string_lossy()
            .replace('\\', "/");
        out.push((rel, path.clone()));
        if path.is_dir() {
            collect_files(root, &path, out);
        }
    }
}

// ---------------------------------------------------------------------------
// validate_task_dir
// ---------------------------------------------------------------------------

/// Reject fixture trees that could leak paths outside the task directory.
pub fn validate_task_dir(task_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    validate_task_dir_inner(task_dir)
}

fn validate_task_dir_inner(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_symlink() {
            return Err(format!("fixture contains symlink: {}", path.display()).into());
        }
        if path.is_dir() {
            validate_task_dir_inner(&path)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_arena_for_local_execution
// ---------------------------------------------------------------------------

/// Refuse arena classes that must run behind Harbor/Docker isolation.
///
/// Mirrors Python's `validate_arena_for_local_execution(arena)` exactly,
/// including the error message strings.
pub fn validate_arena_for_local_execution(arena: &Value) -> Result<(), LocalExecutionRefused> {
    let risk = match arena.get("risk") {
        Some(r) => r,
        None => {
            return Err(LocalExecutionRefused(
                "arena risk requires Harbor/Docker isolation; local temp-dir \
runner refused (missing [risk] metadata). Add reviewed low-risk \
metadata or use bin/harbor-run for isolated execution."
                    .to_string(),
            ));
        }
    };
    let risk_class = risk
        .get("class")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();

    let mut risky_flags: Vec<String> = Vec::new();
    for &(key, label) in LOCAL_REFUSAL_FLAGS {
        if py_is_truthy_json(risk.get(key)) {
            risky_flags.push(label.to_string());
        }
    }
    if !LOCAL_SAFE_RISK_CLASSES.contains(&risk_class.as_str()) {
        risky_flags.insert(0, format!("class={risk_class}"));
    }
    if !risky_flags.is_empty() {
        // dict.fromkeys(risky_flags) deduplicates while preserving insertion order.
        let mut seen = std::collections::HashSet::new();
        let deduped: Vec<&String> = risky_flags
            .iter()
            .filter(|f| seen.insert(f.as_str()))
            .collect();
        let detail = deduped
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(LocalExecutionRefused(format!(
            "arena risk requires Harbor/Docker isolation; local temp-dir \
runner refused ({detail}). Use bin/harbor-run for isolated \
execution, or lower the arena risk metadata only after review."
        )));
    }
    Ok(())
}

/// Python `bool(risk.get(key))` — Value::Null, Value::Bool(false), zero, empty
/// string/array/object are all falsy. Missing key is None → falsy.
fn py_is_truthy_json(v: Option<&Value>) -> bool {
    match v {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

// ---------------------------------------------------------------------------
// _read_visible_texts
// ---------------------------------------------------------------------------

/// Iterator over (source_label, text) pairs that are visible to the candidate.
/// Mirrors Python's `_read_visible_texts(candidate, instruction, env_dir)`.
pub fn read_visible_texts<'a>(
    candidate: &'a Map<String, Value>,
    instruction: &'a str,
    env_dir: &'a Path,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    out.push(("instruction".to_string(), instruction.to_string()));

    if let Some(Value::String(text)) = candidate.get("_packet_text") {
        if !text.is_empty() {
            out.push(("prompt_packet".to_string(), text.clone()));
        }
    }
    if let Some(Value::String(text)) = candidate.get("_agents_md_text") {
        out.push(("agents_md".to_string(), text.clone()));
    }
    if let Some(Value::Array(skills)) = candidate.get("_skills_texts") {
        for (i, skill) in skills.iter().enumerate() {
            if let Value::String(text) = skill {
                out.push((format!("skill[{}]", i + 1), text.clone()));
            }
        }
    }

    // sorted(Path(env_dir).rglob("*")) — files only, text files only
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_files(env_dir, env_dir, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    for (rel_str, abs_path) in files {
        if !abs_path.is_file() {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&abs_path) {
            out.push((rel_str, text));
        }
        // Non-UTF-8 files are silently skipped (UnicodeDecodeError in Python)
    }

    out
}

// ---------------------------------------------------------------------------
// validate_no_hidden_absolute_paths
// ---------------------------------------------------------------------------

/// Reject candidate-visible absolute paths into tests/ or solution/.
pub fn validate_no_hidden_absolute_paths(
    candidate: &Map<String, Value>,
    task_dir: &Path,
    instruction: &str,
    env_dir: &Path,
) -> Result<(), GraderPathLeak> {
    let hidden_roots = [
        task_dir
            .join("tests")
            .canonicalize()
            .unwrap_or_else(|_| task_dir.join("tests"))
            .to_string_lossy()
            .into_owned(),
        task_dir
            .join("solution")
            .canonicalize()
            .unwrap_or_else(|_| task_dir.join("solution"))
            .to_string_lossy()
            .into_owned(),
    ];
    for (source, text) in read_visible_texts(candidate, instruction, env_dir) {
        for root in &hidden_roots {
            if text.contains(root.as_str()) {
                return Err(GraderPathLeak(format!(
                    "candidate-visible {source} exposes hidden grader path: {root}"
                )));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// task_instruction
// ---------------------------------------------------------------------------

/// Compose the task instruction from the arena template + task intent, or fall
/// back to a per-task instruction.md for template-less arenas.
///
/// Mirrors Python:
/// ```python
/// template_ref = (arena.get("template") or {}).get("file")
/// if template_ref:
///     template = (arena_dir / template_ref).read_text()
///     intent = (task_dir / "intent.md").read_text().strip()
///     return template.replace("{intent}", intent)
/// return (task_dir / "instruction.md").read_text()
/// ```
pub fn task_instruction(
    arena_dir: &Path,
    arena: &Value,
    task_dir: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let template_ref = arena
        .get("template")
        .and_then(|t| t.get("file"))
        .and_then(Value::as_str);
    if let Some(tref) = template_ref {
        let template = std::fs::read_to_string(arena_dir.join(tref))?;
        let intent = std::fs::read_to_string(task_dir.join("intent.md"))?;
        let intent = intent.trim();
        return Ok(template.replace("{intent}", intent));
    }
    Ok(std::fs::read_to_string(task_dir.join("instruction.md"))?)
}

// ---------------------------------------------------------------------------
// select_tasks
// ---------------------------------------------------------------------------

/// Resolve task dirs honoring split selection and the holdout guard.
///
/// Mirrors Python's `select_tasks(arena_dir, arena, split, task_filter, final)`.
/// On a bad split, calls `sys.exit(...)` in Python; here we return `Err`.
pub fn select_tasks(
    arena_dir: &Path,
    arena: &Value,
    split: &str,
    task_filter: Option<&std::collections::HashSet<String>>,
    is_final: bool,
) -> Result<Vec<PathBuf>, String> {
    let split_cfg = arena.get("split").and_then(Value::as_object);
    let allowed: Option<std::collections::HashSet<String>> = if split != "all" {
        match split_cfg.and_then(|c| c.get(split)) {
            None => return Err(format!("arena declares no '{split}' split")),
            Some(Value::Array(ids)) => Some(
                ids.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect(),
            ),
            _ => return Err(format!("arena declares no '{split}' split")),
        }
    } else {
        None
    };

    // sorted((arena_dir / "tasks").iterdir()) — directories only
    let tasks_dir = arena_dir.join("tasks");
    let mut task_dirs: Vec<PathBuf> = match std::fs::read_dir(&tasks_dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .filter(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                allowed.as_ref().is_none_or(|a| a.contains(name))
                    && task_filter.is_none_or(|f| f.contains(name))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    task_dirs.sort();

    let holdout: std::collections::HashSet<String> = split_cfg
        .and_then(|c| c.get("holdout"))
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let mut touched_holdout: Vec<String> = task_dirs
        .iter()
        .filter_map(|d| {
            d.file_name()
                .and_then(|n| n.to_str())
                .filter(|n| holdout.contains(*n))
                .map(String::from)
        })
        .collect();
    touched_holdout.sort();

    if !touched_holdout.is_empty() && !is_final {
        return Err(format!(
            "holdout tasks require --final (anti-overfitting guard): {}",
            touched_holdout.join(", ")
        ));
    }
    Ok(task_dirs)
}

// ---------------------------------------------------------------------------
// load_toml
// ---------------------------------------------------------------------------

/// Load a TOML file and return it as a `serde_json::Value` (recursive
/// toml→json conversion). Mirrors Python's `load_toml(path)` return type
/// as used in the codebase: a plain dict.
pub fn load_toml(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    let tv: toml::Value = toml::from_str(&text)?;
    Ok(toml_to_json(tv))
}

/// Recursively convert a `toml::Value` to a `serde_json::Value`.
/// Integer → Number(i64); Float → Number(f64); Boolean → Bool; String →
/// String; Array → Array; Table → Object; Datetime → String.
pub fn toml_to_json(v: toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(serde_json::Number::from(i)),
        toml::Value::Float(f) => {
            Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into()))
        }
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut m: Map<String, Value> = Map::new();
            for (k, val) in t {
                m.insert(k, toml_to_json(val));
            }
            Value::Object(m)
        }
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
    }
}

// ---------------------------------------------------------------------------
// _resolve_ref
// ---------------------------------------------------------------------------

/// Resolve a relative or absolute file reference against the repo root.
/// Mirrors `_resolve_ref(ref)` from Python:
/// ```python
/// path = Path(ref)
/// return path if path.is_absolute() else REPO / path
/// ```
/// where `REPO` is the repository root (two levels above `runner/run.py`).
pub fn resolve_ref(ref_str: &str, repo_root: &Path) -> PathBuf {
    let p = Path::new(ref_str);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        repo_root.join(p)
    }
}

// ---------------------------------------------------------------------------
// load_candidate
// ---------------------------------------------------------------------------

/// Load a manifest, resolve its file-referenced slots (prompt packet, skills,
/// agents_md), compute the composition hash, and return the candidate map.
///
/// Private keys (underscore-prefixed) are included as `_packet_text`,
/// `_skill_paths`, `_skills_texts`, `_agents_md_text`, `_hash`.
///
/// The composition hash mirrors Python exactly:
/// ```python
/// basis = {k: v for k, v in candidate.items() if not k.startswith("_")}
/// basis["prompt_packet_text"] = candidate["_packet_text"]
/// basis["skills_texts"] = candidate["_skills_texts"]
/// basis["agents_md_text"] = candidate["_agents_md_text"]
/// candidate["_hash"] = hashlib.sha256(
///     json.dumps(basis, sort_keys=True).encode()
/// ).hexdigest()[:16]
/// ```
pub fn load_candidate(
    path: &Path,
    repo_root: &Path,
) -> Result<Map<String, Value>, Box<dyn std::error::Error>> {
    let raw = load_toml(path)?;
    let mut candidate = match raw {
        Value::Object(m) => m,
        _ => return Err("candidate manifest is not a TOML table".into()),
    };

    // Resolve prompt_packet
    let packet_text: Option<String> = match candidate.get("prompt_packet") {
        Some(Value::String(ref_str)) => {
            let p = resolve_ref(ref_str, repo_root);
            Some(std::fs::read_to_string(&p)?)
        }
        _ => None,
    };
    candidate.insert(
        "_packet_text".to_string(),
        match packet_text {
            Some(ref t) => Value::String(t.clone()),
            None => Value::Null,
        },
    );

    // Resolve skills
    let skill_paths: Vec<PathBuf> = match candidate.get("skills") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| resolve_ref(s, repo_root))
            .collect(),
        _ => Vec::new(),
    };
    candidate.insert(
        "_skill_paths".to_string(),
        Value::Array(
            skill_paths
                .iter()
                .map(|p| Value::String(p.to_string_lossy().into_owned()))
                .collect(),
        ),
    );
    let skills_texts: Vec<Value> = skill_paths
        .iter()
        .map(|p| std::fs::read_to_string(p).map(Value::String))
        .collect::<Result<_, _>>()?;
    candidate.insert("_skills_texts".to_string(), Value::Array(skills_texts));

    // Resolve agents_md
    let agents_md_text: Option<String> = match candidate.get("agents_md") {
        Some(Value::String(ref_str)) => {
            let p = resolve_ref(ref_str, repo_root);
            Some(std::fs::read_to_string(&p)?)
        }
        _ => None,
    };
    candidate.insert(
        "_agents_md_text".to_string(),
        match agents_md_text {
            Some(ref t) => Value::String(t.clone()),
            None => Value::Null,
        },
    );

    // Validate system_prompt_mode
    let mode = candidate
        .get("system_prompt_mode")
        .and_then(Value::as_str)
        .unwrap_or("append");
    if mode != "append" && mode != "replace" {
        return Err(format!("system_prompt_mode must be append|replace, got {mode:?}").into());
    }

    // Build basis for the composition hash: non-underscore keys + three
    // resolved texts.
    let mut basis: Map<String, Value> = Map::new();
    for (k, v) in &candidate {
        if !k.starts_with('_') {
            basis.insert(k.clone(), v.clone());
        }
    }
    // Python inserts these keys AFTER the manifest keys, in this exact order.
    basis.insert(
        "prompt_packet_text".to_string(),
        candidate["_packet_text"].clone(),
    );
    basis.insert(
        "skills_texts".to_string(),
        candidate["_skills_texts"].clone(),
    );
    basis.insert(
        "agents_md_text".to_string(),
        candidate["_agents_md_text"].clone(),
    );

    // sha256(json.dumps(basis, sort_keys=True).encode()).hexdigest()[:16]
    let dumped = py_json_dumps(&Value::Object(basis), true);
    let mut h = Sha256::new();
    h.update(dumped.as_bytes());
    let hex = format!("{:x}", h.finalize());
    let hash = hex[..16].to_string();
    candidate.insert("_hash".to_string(), Value::String(hash));

    Ok(candidate)
}

// ---------------------------------------------------------------------------
// workspace_listing
// ---------------------------------------------------------------------------

/// Build a workspace listing string for oneshot prompts.
///
/// Mirrors:
/// ```python
/// parts = []
/// for f in sorted(workdir.rglob("*")):
///     if f.is_file() and f.name != "findings.json":
///         rel = f.relative_to(workdir)
///         parts.append(f"\n### {rel}\n```\n{f.read_text()}```\n")
/// return "".join(parts)
/// ```
pub fn workspace_listing(workdir: &Path) -> String {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_files(workdir, workdir, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut parts = String::new();
    for (rel_str, abs_path) in files {
        if !abs_path.is_file() {
            continue;
        }
        let name = abs_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "findings.json" {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&abs_path) {
            parts.push_str(&format!("\n### {rel_str}\n```\n{text}```\n"));
        }
    }
    parts
}

// ---------------------------------------------------------------------------
// extract_json_object
// ---------------------------------------------------------------------------

/// Pull the first parseable top-level JSON object out of model output.
///
/// Mirrors Python's brace-depth scan exactly:
/// ```python
/// start = text.find("{")
/// while start != -1:
///     depth = 0
///     for i in range(start, len(text)):
///         if text[i] == "{": depth += 1
///         elif text[i] == "}":
///             depth -= 1
///             if depth == 0:
///                 try: return json.loads(text[start : i + 1])
///                 except json.JSONDecodeError: break
///     start = text.find("{", start + 1)
/// raise ValueError("no JSON object found in model output")
/// ```
pub fn extract_json_object(text: &str) -> Result<Value, String> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut start = 0usize;

    while let Some(rel) = text[start..].find('{') {
        let abs_start = start + rel;
        let mut depth: i32 = 0;
        let mut i = abs_start;
        while i < len {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        let slice = &text[abs_start..=i];
                        if let Ok(v) = serde_json::from_str::<Value>(slice) {
                            return Ok(v);
                        }
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        start = abs_start + 1;
    }
    Err("no JSON object found in model output".to_string())
}

// ---------------------------------------------------------------------------
// candidate_env
// ---------------------------------------------------------------------------

/// Baseline process environment for candidate subprocesses.
///
/// Mirrors:
/// ```python
/// BASE_ENV_VARS = ("PATH", "HOME", "TERM", "LANG", "LC_ALL")
/// allow = candidate.get("env_allowlist", ["OPENROUTER_API_KEY"])
/// return {k: os.environ[k] for k in (*BASE_ENV_VARS, *allow) if k in os.environ}
/// ```
pub fn candidate_env(
    candidate: &Map<String, Value>,
    env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let base: &[&str] = &["PATH", "HOME", "TERM", "LANG", "LC_ALL"];
    let allow: Vec<String> = match candidate.get("env_allowlist") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect(),
        _ => vec!["OPENROUTER_API_KEY".to_string()],
    };
    base.iter()
        .map(|k| k.to_string())
        .chain(allow)
        .filter_map(|k| env.get(&k).map(|v| (k, v.clone())))
        .collect()
}

// ---------------------------------------------------------------------------
// extract_pi_usage
// ---------------------------------------------------------------------------

/// Sum usage across assistant `message_end` events in `pi --mode json` output.
///
/// Mirrors Python exactly: sums `input`/`output`/`cacheRead` as `int(x or 0)`,
/// sums `cost.total` as `float`, returns `round(cost, 6)` via
/// `pycompat::round_half_even`. Provider is taken from the last seen assistant
/// message. Returns empty map if no qualifying events found.
pub fn extract_pi_usage(stdout_text: &str) -> Map<String, Value> {
    let mut tokens_in: i64 = 0;
    let mut tokens_out: i64 = 0;
    let mut cached: i64 = 0;
    let mut cost: f64 = 0.0;
    let mut provider: Option<String> = None;
    let mut found = false;

    for line in stdout_text.lines() {
        let line = line.trim();
        if !line.starts_with(r#"{"type":"message_end""#) {
            continue;
        }
        let event: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let msg = match event.get("message") {
            Some(m) => m,
            None => continue,
        };
        if msg.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        found = true;
        // provider: msg.get("provider") or previous provider
        if let Some(p) = msg.get("provider").and_then(Value::as_str) {
            provider = Some(p.to_string());
        }
        let usage = msg.get("usage").and_then(Value::as_object);
        if let Some(u) = usage {
            tokens_in += as_int_or_0(u.get("input"));
            tokens_out += as_int_or_0(u.get("output"));
            cached += as_int_or_0(u.get("cacheRead"));
            let total = u
                .get("cost")
                .and_then(|c| c.get("total"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            cost += total;
        }
    }

    if !found {
        return Map::new();
    }

    let mut out = Map::new();
    out.insert(
        "provider_served".to_string(),
        match provider {
            Some(p) => Value::String(p),
            None => Value::Null,
        },
    );
    out.insert(
        "tokens_prompt".to_string(),
        Value::Number(serde_json::Number::from(tokens_in)),
    );
    out.insert(
        "tokens_completion".to_string(),
        Value::Number(serde_json::Number::from(tokens_out)),
    );
    out.insert(
        "tokens_cached".to_string(),
        Value::Number(serde_json::Number::from(cached)),
    );
    out.insert(
        "cost_usd".to_string(),
        Value::Number(
            serde_json::Number::from_f64(round_half_even(cost, 6)).unwrap_or_else(|| 0.into()),
        ),
    );
    out
}

/// Python's `int(usage.get(key) or 0)` — coerce falsy to 0.
fn as_int_or_0(v: Option<&Value>) -> i64 {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => 0,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// build_pi_cmd
// ---------------------------------------------------------------------------

/// Compose the pi argv from the candidate's slots.
///
/// Mirrors Python's `build_pi_cmd(candidate)`:
/// - `--no-skills` unless `_skill_paths` is non-empty
/// - `--no-context-files` unless `_agents_md_text` is non-None
/// - `--append-system-prompt` or `--system-prompt` depending on
///   `system_prompt_mode`
pub fn build_pi_cmd(candidate: &Map<String, Value>) -> Vec<String> {
    let mut cmd = vec![
        "pi".to_string(),
        "-p".to_string(),
        "--mode".to_string(),
        "json".to_string(),
        "--no-session".to_string(),
        "--no-extensions".to_string(),
        "--no-prompt-templates".to_string(),
        "--no-themes".to_string(),
        "--provider".to_string(),
        candidate
            .get("provider_name")
            .and_then(Value::as_str)
            .unwrap_or("openrouter")
            .to_string(),
        "--model".to_string(),
        candidate
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    ];

    // Skills: non-empty _skill_paths → add --skill flags; else --no-skills
    let has_skills = match candidate.get("_skill_paths") {
        Some(Value::Array(arr)) => !arr.is_empty(),
        _ => false,
    };
    if has_skills {
        if let Some(Value::Array(paths)) = candidate.get("_skill_paths") {
            for p in paths {
                if let Value::String(s) = p {
                    cmd.push("--skill".to_string());
                    cmd.push(s.clone());
                }
            }
        }
    } else {
        cmd.push("--no-skills".to_string());
    }

    // Agents MD: None → --no-context-files
    let has_agents_md = !matches!(candidate.get("_agents_md_text"), Some(Value::Null) | None);
    if !has_agents_md {
        cmd.push("--no-context-files".to_string());
    }

    // Thinking flag
    if let Some(v) = candidate.get("thinking").and_then(Value::as_str) {
        cmd.push("--thinking".to_string());
        cmd.push(v.to_string());
    }

    // Tools flag
    if let Some(Value::Array(tools)) = candidate.get("tools") {
        let tool_str: Vec<&str> = tools.iter().filter_map(Value::as_str).collect();
        if !tool_str.is_empty() {
            cmd.push("--tools".to_string());
            cmd.push(tool_str.join(","));
        }
    }

    // Prompt packet: system-prompt or append-system-prompt
    let packet_text = candidate
        .get("_packet_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !packet_text.is_empty() {
        let mode = candidate
            .get("system_prompt_mode")
            .and_then(Value::as_str)
            .unwrap_or("append");
        let flag = if mode == "replace" {
            "--system-prompt"
        } else {
            "--append-system-prompt"
        };
        cmd.push(flag.to_string());
        cmd.push(packet_text.to_string());
    }

    cmd
}

// ---------------------------------------------------------------------------
// prepare_workspace
// ---------------------------------------------------------------------------

/// Slot-driven workspace composition beyond the fixture copy.
///
/// Mirrors Python:
/// ```python
/// if candidate.get("_agents_md_text") is not None:
///     (workdir / "AGENTS.md").write_text(candidate["_agents_md_text"])
/// ```
pub fn prepare_workspace(
    candidate: &Map<String, Value>,
    workdir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match candidate.get("_agents_md_text") {
        Some(Value::String(text)) => {
            std::fs::write(workdir.join("AGENTS.md"), text)?;
        }
        Some(Value::Null) | None => {}
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// summarize
// ---------------------------------------------------------------------------

/// Per-candidate, per-task reward distributions from a trials file.
///
/// Mirrors Python's `summarize(trials_path)` exactly, including:
/// - `round(sum/len, 4)` via `pycompat::round_half_even`
/// - field insertion order (composition_hash, kind, tasks, trials, errors,
///   cost_usd_total, cost_known)
/// - per-task `mean`, `min`, `max` and full `rewards`/`wall_ms` arrays
pub fn summarize(trials_path: &Path) -> Result<Map<String, Value>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(trials_path)?;
    let records: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;

    // summary: candidate_id → candidate_summary
    let mut summary: Map<String, Value> = Map::new();

    for r in &records {
        let cand_id = r["candidate_id"].as_str().unwrap_or("").to_string();
        let task_id = r["task_id"].as_str().unwrap_or("").to_string();
        let reward = r["reward"].as_f64().unwrap_or(0.0);
        // Preserve the original JSON type of wall_ms (int or float) so that
        // serialisation matches Python's json.dumps (e.g. 9 stays 9, not 9.0).
        let wall_ms_val = r.get("wall_ms").cloned().unwrap_or(Value::Null);
        let _wall_ms = wall_ms_val.as_f64().unwrap_or(0.0);
        let cost_usd = r.get("cost_usd");
        let error = r.get("error");

        let c = summary.entry(cand_id).or_insert_with(|| {
            // Build the initial candidate struct in Python insertion order.
            let mut m = Map::new();
            m.insert(
                "composition_hash".to_string(),
                r.get("composition_hash").cloned().unwrap_or(Value::Null),
            );
            m.insert(
                "kind".to_string(),
                r.get("candidate_kind").cloned().unwrap_or(Value::Null),
            );
            m.insert("tasks".to_string(), Value::Object(Map::new()));
            m.insert(
                "trials".to_string(),
                Value::Number(serde_json::Number::from(0i64)),
            );
            m.insert(
                "errors".to_string(),
                Value::Number(serde_json::Number::from(0i64)),
            );
            m.insert(
                "cost_usd_total".to_string(),
                Value::Number(serde_json::Number::from_f64(0.0).unwrap()),
            );
            m.insert("cost_known".to_string(), Value::Bool(true));
            Value::Object(m)
        });

        let c = c.as_object_mut().unwrap();

        // trials += 1
        let trials = c["trials"].as_i64().unwrap_or(0) + 1;
        c.insert(
            "trials".to_string(),
            Value::Number(serde_json::Number::from(trials)),
        );

        // errors += 1 if error is truthy
        let has_error = match error {
            Some(Value::Null) | None => false,
            Some(Value::Bool(false)) => false,
            Some(Value::String(s)) if s.is_empty() => false,
            _ => true,
        };
        if has_error {
            let errors = c["errors"].as_i64().unwrap_or(0) + 1;
            c.insert(
                "errors".to_string(),
                Value::Number(serde_json::Number::from(errors)),
            );
        }

        // tasks[task_id].rewards.append(reward), .wall_ms.append(wall_ms)
        let tasks = c["tasks"].as_object_mut().unwrap();
        let t = tasks
            .entry(task_id)
            .or_insert_with(|| {
                let mut m = Map::new();
                m.insert("rewards".to_string(), Value::Array(vec![]));
                m.insert("wall_ms".to_string(), Value::Array(vec![]));
                Value::Object(m)
            })
            .as_object_mut()
            .unwrap();
        t["rewards"]
            .as_array_mut()
            .unwrap()
            .push(Value::Number(serde_json::Number::from_f64(reward).unwrap()));
        // Push the original value (preserving int vs float type from JSONL).
        t["wall_ms"].as_array_mut().unwrap().push(wall_ms_val);

        // cost tracking
        match cost_usd {
            Some(Value::Null) | None => {
                c.insert("cost_known".to_string(), Value::Bool(false));
            }
            Some(v) => {
                let cost_val = v.as_f64().unwrap_or(0.0);
                let current = c["cost_usd_total"].as_f64().unwrap_or(0.0);
                c.insert(
                    "cost_usd_total".to_string(),
                    Value::Number(
                        serde_json::Number::from_f64(current + cost_val)
                            .unwrap_or_else(|| 0.into()),
                    ),
                );
            }
        }
    }

    // Post-processing: compute per-candidate and per-task aggregates.
    for c in summary.values_mut() {
        let c = c.as_object_mut().unwrap();

        // All rewards across all tasks
        let all_rewards: Vec<f64> = c["tasks"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|t| {
                t["rewards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_f64)
            })
            .collect();

        let n = all_rewards.len() as f64;
        let sum_r: f64 = all_rewards.iter().sum();
        let reward_mean = round_half_even(sum_r / n, 4);
        c.insert(
            "reward_mean".to_string(),
            Value::Number(serde_json::Number::from_f64(reward_mean).unwrap()),
        );

        let total_cost = c["cost_usd_total"].as_f64().unwrap_or(0.0);
        c.insert(
            "cost_usd_total".to_string(),
            Value::Number(
                serde_json::Number::from_f64(round_half_even(total_cost, 6))
                    .unwrap_or_else(|| 0.into()),
            ),
        );

        // Per-task: mean, min, max
        let tasks = c["tasks"].as_object_mut().unwrap();
        for t in tasks.values_mut() {
            let t = t.as_object_mut().unwrap();
            let rewards: Vec<f64> = t["rewards"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_f64)
                .collect();
            let nt = rewards.len() as f64;
            let st: f64 = rewards.iter().sum();
            let mean_r = round_half_even(st / nt, 4);
            let min_r: f64 = rewards.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_r: f64 = rewards.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            t.insert(
                "mean".to_string(),
                Value::Number(serde_json::Number::from_f64(mean_r).unwrap()),
            );
            t.insert(
                "min".to_string(),
                Value::Number(serde_json::Number::from_f64(min_r).unwrap()),
            );
            t.insert(
                "max".to_string(),
                Value::Number(serde_json::Number::from_f64(max_r).unwrap()),
            );
        }
    }

    Ok(summary)
}

// ---------------------------------------------------------------------------
// run_null / run_oracle (deterministic halves of the executor functions)
// ---------------------------------------------------------------------------

/// Write an empty findings file. Mirrors `run_null` without the `workdir`
/// I/O being the test surface — returns the JSON string the function would
/// write, so the caller can assert it or write it.
pub fn null_findings_json() -> &'static str {
    "{\"findings\": []}\n"
}

/// Copy oracle findings from `task_dir/solution/findings.json` to
/// `workdir/findings.json`. Returns the findings JSON string.
pub fn oracle_findings(task_dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let src = task_dir.join("solution").join("findings.json");
    Ok(std::fs::read_to_string(src)?)
}

// ---------------------------------------------------------------------------
// Unit tests (porting tests/test_run.py inline assertions)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const PI_TRANSCRIPT: &str = r#"{"type":"session","version":3,"id":"x","timestamp":"t","cwd":"/tmp"}
{"type":"message_end","message":{"role":"user","content":[]}}
{"type":"message_end","message":{"role":"assistant","provider":"openrouter","usage":{"input":397,"output":26,"cacheRead":10,"cacheWrite":0,"totalTokens":423,"cost":{"input":0.0002,"output":0.00008,"total":0.00028}}}}
{"type":"message_end","message":{"role":"assistant","provider":"openrouter","usage":{"input":500,"output":40,"cacheRead":0,"cacheWrite":0,"totalTokens":540,"cost":{"total":0.0005}}}}
"#;

    // -----------------------------------------------------------------------
    // extract_json_object
    // -----------------------------------------------------------------------

    #[test]
    fn extract_json_object_plain() {
        let v = extract_json_object(r#"{"findings": []}"#).unwrap();
        assert_eq!(v, serde_json::json!({"findings": []}));
    }

    #[test]
    fn extract_json_object_with_prose_and_fences() {
        let text = "Sure! Here you go:\n```json\n{\"findings\": [{\"a\": 1}]}\n```\nDone.";
        let v = extract_json_object(text).unwrap();
        assert_eq!(v, serde_json::json!({"findings": [{"a": 1}]}));
    }

    #[test]
    fn extract_json_object_skips_broken_prefix() {
        let text = r#"{broken {"findings": []}"#;
        let v = extract_json_object(text).unwrap();
        assert_eq!(v, serde_json::json!({"findings": []}));
    }

    #[test]
    fn extract_json_object_raises_when_absent() {
        assert!(extract_json_object("no json here").is_err());
    }

    // -----------------------------------------------------------------------
    // extract_pi_usage
    // -----------------------------------------------------------------------

    #[test]
    fn extract_pi_usage_sums_assistant_message_ends() {
        let usage = extract_pi_usage(PI_TRANSCRIPT);
        assert_eq!(usage["tokens_prompt"].as_i64().unwrap(), 897);
        assert_eq!(usage["tokens_completion"].as_i64().unwrap(), 66);
        assert_eq!(usage["tokens_cached"].as_i64().unwrap(), 10);
        assert!(
            (usage["cost_usd"].as_f64().unwrap() - 0.00078).abs() < 1e-9,
            "cost_usd = {}",
            usage["cost_usd"]
        );
        assert_eq!(usage["provider_served"].as_str().unwrap(), "openrouter");
    }

    #[test]
    fn extract_pi_usage_empty_on_no_events() {
        let usage = extract_pi_usage("plain text\n{\"type\":\"other\"}");
        assert!(usage.is_empty());
    }

    // -----------------------------------------------------------------------
    // candidate_env
    // -----------------------------------------------------------------------

    #[test]
    fn candidate_env_withholds_unrelated_secrets() {
        let mut env = HashMap::new();
        env.insert("GITHUB_TOKEN".to_string(), "sekret".to_string());
        env.insert("OPENAI_API_KEY".to_string(), "sekret2".to_string());
        env.insert("OPENROUTER_API_KEY".to_string(), "or-key".to_string());
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        let candidate = Map::new();
        let result = candidate_env(&candidate, &env);
        assert!(!result.contains_key("GITHUB_TOKEN"));
        assert!(!result.contains_key("OPENAI_API_KEY"));
        assert_eq!(result["OPENROUTER_API_KEY"], "or-key");
        assert!(result.contains_key("PATH"));
    }

    #[test]
    fn candidate_env_respects_manifest_allowlist() {
        let mut env = HashMap::new();
        env.insert("CUSTOM_KEY".to_string(), "v".to_string());
        env.insert("OPENROUTER_API_KEY".to_string(), "or-key".to_string());
        let mut candidate = Map::new();
        candidate.insert(
            "env_allowlist".to_string(),
            Value::Array(vec![Value::String("CUSTOM_KEY".to_string())]),
        );
        let result = candidate_env(&candidate, &env);
        assert_eq!(result["CUSTOM_KEY"], "v");
        assert!(!result.contains_key("OPENROUTER_API_KEY"));
    }

    // -----------------------------------------------------------------------
    // tree_digest
    // -----------------------------------------------------------------------

    #[test]
    fn tree_digest_detects_tampering() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-test-{}", std::process::id()));
        let tests_dir = tmp.join("tests");
        std::fs::create_dir_all(&tests_dir).unwrap();
        let key = tests_dir.join("expected.json");
        std::fs::write(&key, "{}").unwrap();
        let before = tree_digest(&[&tests_dir]);
        assert_eq!(tree_digest(&[&tests_dir]), before);
        std::fs::write(&key, "{\"defects\": []}").unwrap();
        assert_ne!(tree_digest(&[&tests_dir]), before);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // validate_task_dir
    // -----------------------------------------------------------------------

    #[test]
    fn validate_task_dir_rejects_symlinks() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-sym-{}", std::process::id()));
        let env_dir = tmp.join("environment");
        std::fs::create_dir_all(&env_dir).unwrap();
        let evil = env_dir.join("evil");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", &evil).unwrap();
        #[cfg(unix)]
        {
            let result = validate_task_dir(&tmp);
            assert!(result.is_err(), "expected Err for symlink in fixture");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // validate_arena_for_local_execution
    // -----------------------------------------------------------------------

    #[test]
    fn validate_arena_missing_risk_is_refused() {
        let arena = serde_json::json!({"id": "x", "version": "0.1.0"});
        let err = validate_arena_for_local_execution(&arena).unwrap_err();
        assert!(err.0.contains("missing [risk] metadata"));
        assert!(err.0.contains("Harbor/Docker isolation"));
    }

    #[test]
    fn validate_arena_sensitive_class_is_refused() {
        let arena = serde_json::json!({"risk": {"class": "sensitive", "needs_network": true}});
        let err = validate_arena_for_local_execution(&arena).unwrap_err();
        assert!(err.0.contains("Harbor/Docker isolation"));
        assert!(err.0.contains("class=sensitive"));
        assert!(err.0.contains("network"));
    }

    #[test]
    fn validate_arena_low_class_no_flags_passes() {
        let arena = serde_json::json!({"risk": {"class": "low"}});
        assert!(validate_arena_for_local_execution(&arena).is_ok());
    }

    // -----------------------------------------------------------------------
    // build_pi_cmd
    // -----------------------------------------------------------------------

    #[test]
    fn build_pi_cmd_default_isolation_and_append() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-cmd-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let packet = tmp.join("p.md");
        std::fs::write(&packet, "Review carefully and thoroughly.").unwrap();
        let manifest = tmp.join("c.toml");
        std::fs::write(
            &manifest,
            format!(
                "id = \"x\"\nkind = \"pi\"\nmodel = \"m\"\nprompt_packet = \"{}\"\n",
                packet.display()
            ),
        )
        .unwrap();
        // Use absolute path as repo_root to enable resolution
        let cand = load_candidate(&manifest, &tmp).unwrap();
        let cmd = build_pi_cmd(&cand);
        assert!(cmd.contains(&"--no-skills".to_string()));
        assert!(cmd.contains(&"--no-context-files".to_string()));
        assert!(cmd.contains(&"--append-system-prompt".to_string()));
        assert!(
            !cmd.contains(&"--system-prompt".to_string())
                || cmd
                    .iter()
                    .position(|s| s == "--system-prompt")
                    .map(|i| cmd[i] != "--system-prompt")
                    .unwrap_or(true)
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn build_pi_cmd_replace_mode_uses_system_prompt_flag() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-cmd2-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let packet = tmp.join("p.md");
        std::fs::write(&packet, "You are the whole system prompt.").unwrap();
        let skill = tmp.join("skill.md");
        std::fs::write(&skill, "# a pi skill").unwrap();
        let agents = tmp.join("agents.md");
        std::fs::write(&agents, "Repo briefing: run bin/gate before claiming done.").unwrap();
        let manifest = tmp.join("c.toml");
        std::fs::write(
            &manifest,
            format!(
                "id = \"x\"\nkind = \"pi\"\nmodel = \"m\"\n\
prompt_packet = \"{packet}\"\nsystem_prompt_mode = \"replace\"\n\
skills = [\"{skill}\"]\nagents_md = \"{agents}\"\n",
                packet = packet.display(),
                skill = skill.display(),
                agents = agents.display(),
            ),
        )
        .unwrap();
        let cand = load_candidate(&manifest, &tmp).unwrap();
        let cmd = build_pi_cmd(&cand);
        assert!(cmd.contains(&"--skill".to_string()));
        assert!(!cmd.contains(&"--no-skills".to_string()));
        assert!(!cmd.contains(&"--no-context-files".to_string()));
        assert!(cmd.contains(&"--system-prompt".to_string()));
        assert!(!cmd.contains(&"--append-system-prompt".to_string()));
        let workdir = tmp.join("ws");
        std::fs::create_dir_all(&workdir).unwrap();
        prepare_workspace(&cand, &workdir).unwrap();
        let agents_text = std::fs::read_to_string(workdir.join("AGENTS.md")).unwrap();
        assert!(agents_text.contains("bin/gate"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // load_candidate / composition hash
    // -----------------------------------------------------------------------

    #[test]
    fn hash_tracks_agents_md_and_skills_content() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-hash-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let skill = tmp.join("skill.md");
        std::fs::write(&skill, "v1").unwrap();
        let agents = tmp.join("agents.md");
        std::fs::write(&agents, "briefing v1").unwrap();
        let manifest = tmp.join("c.toml");
        std::fs::write(
            &manifest,
            format!(
                "id = \"x\"\nkind = \"pi\"\nmodel = \"m\"\n\
skills = [\"{skill}\"]\nagents_md = \"{agents}\"\n",
                skill = skill.display(),
                agents = agents.display(),
            ),
        )
        .unwrap();
        let h1 = load_candidate(&manifest, &tmp).unwrap()["_hash"]
            .as_str()
            .unwrap()
            .to_string();
        std::fs::write(&skill, "v2").unwrap();
        let h2 = load_candidate(&manifest, &tmp).unwrap()["_hash"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(h1, h2);
        std::fs::write(&skill, "v1").unwrap();
        std::fs::write(&agents, "briefing v2").unwrap();
        let h3 = load_candidate(&manifest, &tmp).unwrap()["_hash"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(h3, h1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn invalid_system_prompt_mode_rejected() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-mode-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let manifest = tmp.join("c.toml");
        std::fs::write(
            &manifest,
            "id = \"x\"\nkind = \"pi\"\nmodel = \"m\"\nsystem_prompt_mode = \"yolo\"\n",
        )
        .unwrap();
        let result = load_candidate(&manifest, &tmp);
        assert!(
            result.is_err(),
            "expected Err for invalid system_prompt_mode"
        );
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("system_prompt_mode"), "msg={msg}");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn composition_hash_tracks_prompt_packet() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-packet-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let packet = tmp.join("packet.md");
        std::fs::write(&packet, "Review carefully.").unwrap();
        let manifest = tmp.join("cand.toml");
        std::fs::write(
            &manifest,
            format!(
                "id = \"x\"\nkind = \"oneshot\"\nmodel = \"m\"\nprompt_packet = \"{}\"\n",
                packet.display()
            ),
        )
        .unwrap();
        let h1 = load_candidate(&manifest, &tmp).unwrap()["_hash"]
            .as_str()
            .unwrap()
            .to_string();
        std::fs::write(&packet, "Review very carefully.").unwrap();
        let h2 = load_candidate(&manifest, &tmp).unwrap()["_hash"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(h1, h2);
        std::fs::write(&packet, "Review carefully.").unwrap();
        let h3 = load_candidate(&manifest, &tmp).unwrap()["_hash"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(h1, h3);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // select_tasks
    // -----------------------------------------------------------------------

    fn write_minimal_arena(tmp: &Path, risk_block: &str) -> (PathBuf, Value) {
        let arena_dir = tmp.join("arena");
        let task = arena_dir.join("tasks").join("sample");
        let env = task.join("environment");
        std::fs::create_dir_all(&env).unwrap();
        std::fs::write(env.join("PR.diff"), "diff --git a/x b/x\n").unwrap();
        std::fs::create_dir_all(task.join("tests")).unwrap();
        std::fs::write(
            task.join("tests").join("expected.json"),
            "{\"defects\": []}\n",
        )
        .unwrap();
        std::fs::create_dir_all(task.join("solution")).unwrap();
        std::fs::write(
            task.join("solution").join("findings.json"),
            "{\"findings\": []}\n",
        )
        .unwrap();
        std::fs::write(task.join("instruction.md"), "Review this.").unwrap();
        let toml_content = format!(
            "id = \"sample\"\nversion = \"0.1.0\"\ntaskspec = \"specs/sample/taskspec.toml\"\n\
{risk_block}\n[split]\ntrain = [\"sample\"]\nvalidation = []\nholdout = []\n"
        );
        std::fs::write(arena_dir.join("arena.toml"), &toml_content).unwrap();
        let arena = load_toml(&arena_dir.join("arena.toml")).unwrap();
        (arena_dir, arena)
    }

    #[test]
    fn select_tasks_train_split_returns_matching_tasks() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-select-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let (arena_dir, arena) = write_minimal_arena(&tmp, "[risk]\nclass = \"low\"\n");
        let tasks = select_tasks(&arena_dir, &arena, "train", None, false).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].file_name().unwrap().to_str().unwrap(), "sample");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn select_tasks_unknown_split_errors() {
        let tmp =
            std::env::temp_dir().join(format!("daedalus-run-badsplit-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let (arena_dir, arena) = write_minimal_arena(&tmp, "[risk]\nclass = \"low\"\n");
        let result = select_tasks(&arena_dir, &arena, "nonsense", None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonsense"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn select_tasks_empty_holdout_returns_no_tasks() {
        // holdout = [] → valid, selects nothing
        let tmp = std::env::temp_dir().join(format!("daedalus-run-holdout-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let (arena_dir, arena) = write_minimal_arena(&tmp, "[risk]\nclass = \"low\"\n");
        let tasks = select_tasks(&arena_dir, &arena, "holdout", None, true).unwrap();
        assert_eq!(tasks.len(), 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // task_instruction
    // -----------------------------------------------------------------------

    #[test]
    fn task_instruction_template_replaces_intent() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-instr-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let task_dir = tmp.join("tasks").join("t1");
        std::fs::create_dir_all(&task_dir).unwrap();
        std::fs::write(task_dir.join("intent.md"), "  find the bug  ").unwrap();
        std::fs::write(tmp.join("template.md"), "Review: {intent}\nEnd.").unwrap();
        let arena = serde_json::json!({"template": {"file": "template.md"}});
        let result = task_instruction(&tmp, &arena, &task_dir).unwrap();
        assert_eq!(result, "Review: find the bug\nEnd.");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn task_instruction_falls_back_to_instruction_md() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-instr2-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let task_dir = tmp.join("tasks").join("t1");
        std::fs::create_dir_all(&task_dir).unwrap();
        std::fs::write(task_dir.join("instruction.md"), "Direct instruction.").unwrap();
        let arena = serde_json::json!({"id": "x"});
        let result = task_instruction(&tmp, &arena, &task_dir).unwrap();
        assert_eq!(result, "Direct instruction.");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // summarize
    // -----------------------------------------------------------------------

    #[test]
    fn summarize_basic_aggregates() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-summ-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("trials.jsonl");
        let records = [
            serde_json::json!({"candidate_id": "oracle", "candidate_kind": "oracle", "composition_hash": "abc", "task_id": "t1", "reward": 1.0, "cost_usd": null, "wall_ms": 100, "error": null}),
            serde_json::json!({"candidate_id": "oracle", "candidate_kind": "oracle", "composition_hash": "abc", "task_id": "t2", "reward": 1.0, "cost_usd": null, "wall_ms": 200, "error": null}),
        ];
        let body: String = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&path, body).unwrap();
        let summary = summarize(&path).unwrap();
        let oracle = &summary["oracle"];
        assert_eq!(oracle["trials"].as_i64().unwrap(), 2);
        assert_eq!(oracle["reward_mean"].as_f64().unwrap(), 1.0);
        assert!(!oracle["cost_known"].as_bool().unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn summarize_task_level_min_max_mean() {
        let tmp = std::env::temp_dir().join(format!("daedalus-run-summ2-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("trials.jsonl");
        let records = [
            serde_json::json!({"candidate_id": "a", "candidate_kind": "pi", "composition_hash": "h", "task_id": "t1", "reward": 0.0, "cost_usd": 0.01, "wall_ms": 1000, "error": null}),
            serde_json::json!({"candidate_id": "a", "candidate_kind": "pi", "composition_hash": "h", "task_id": "t1", "reward": 1.0, "cost_usd": 0.01, "wall_ms": 2000, "error": null}),
        ];
        let body: String = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&path, body).unwrap();
        let summary = summarize(&path).unwrap();
        let a = &summary["a"];
        let t1 = &a["tasks"]["t1"];
        assert_eq!(t1["min"].as_f64().unwrap(), 0.0);
        assert_eq!(t1["max"].as_f64().unwrap(), 1.0);
        assert_eq!(t1["mean"].as_f64().unwrap(), 0.5);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
