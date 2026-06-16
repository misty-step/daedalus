//! End-to-end parity oracle for `run_arena` vs `runner/run.py`.
//!
//! Runs both Python and Rust pipelines against `candidates/null.toml` and
//! `candidates/oracle.toml` on `arenas/pr-review-v0`, then compares the
//! DETERMINISTIC fields of `trials.jsonl` and `summary.json` record-by-record.
//!
//! **No model spend** — null and oracle candidates do only local file I/O.
//!
//! Skips (does not fail) if `python3` is not on PATH.
//!
//! ## Deterministic fields compared
//!
//! `arena_id`, `arena_version`, `taskspec`, `task_id`, `trial`,
//! `candidate_id`, `candidate_kind`, `composition_hash`, `model`, `error`,
//! `reward`, `recall`, `matched`, `false_positives`, `expected_defects`,
//! `scorer_error`, `findings`
//!
//! ## Excluded (inherently non-deterministic)
//!
//! `run_id`, `ts_start`, `ts_end`, `wall_ms`, `artifacts`, `harness_version`,
//! `runner_version`, `provider_served`, `tokens_prompt`, `tokens_completion`,
//! `tokens_cached`, `cost_usd`, `agent_exit_code`

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use daedalus_core::run::{run_arena, ArenaInputs};
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root two levels above crates/daedalus-core")
        .to_path_buf()
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// The deterministic fields we assert equal between Python and Rust records.
const DET_FIELDS: &[&str] = &[
    "arena_id",
    "arena_version",
    "taskspec",
    "task_id",
    "trial",
    "candidate_id",
    "candidate_kind",
    "composition_hash",
    "model",
    "error",
    "reward",
    "recall",
    "matched",
    "false_positives",
    "expected_defects",
    "scorer_error",
    "findings",
];

/// Parse a `trials.jsonl` file into a list of JSON objects keyed by
/// `(task_id, trial)` — order-independent lookup for comparison.
fn parse_trials(path: &Path) -> Vec<Value> {
    let text = std::fs::read_to_string(path).expect("read trials.jsonl");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("parse trials.jsonl line"))
        .collect()
}

/// Run the Python runner and return the experiment directory.
fn run_python(repo: &Path, candidate_rel: &str, arena_rel: &str, runs_dir: &Path) -> PathBuf {
    let out = Command::new("python3")
        .current_dir(repo)
        .arg("runner/run.py")
        .arg("--candidate")
        .arg(candidate_rel)
        .arg("--arena")
        .arg(arena_rel)
        .arg("--trials")
        .arg("1")
        // Use split=all with --final to include all tasks without
        // triggering the holdout guard.
        .arg("--split")
        .arg("all")
        .arg("--final")
        .env("DAEDALUS_RUNS_DIR", runs_dir)
        .output()
        .expect("spawn python3 runner/run.py");

    assert!(
        out.status.success(),
        "Python runner failed for {candidate_rel}:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    // Find the single exp dir created under runs_dir.
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(runs_dir)
        .expect("read runs_dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    assert_eq!(
        dirs.len(),
        1,
        "expected exactly one experiment dir under {}: found {:?}",
        runs_dir.display(),
        dirs
    );
    dirs.remove(0)
}

/// Run the Rust `run_arena` and return the experiment directory.
fn run_rust(repo: &Path, candidate_rel: &str, arena_rel: &str, runs_dir: &Path) -> PathBuf {
    let inputs = ArenaInputs {
        candidate_path: repo.join(candidate_rel),
        arena_dir: repo.join(arena_rel),
        task_filter: None,
        trials: 1,
        exp_dir: None,
        split: "all".to_string(),
        is_final: true, // mirror --final so holdout tasks are included
        max_errors: None,
        repo_root: repo.to_path_buf(),
        runs_root: runs_dir.to_path_buf(),
    };
    run_arena(inputs).expect("run_arena failed")
}

/// Compare the deterministic fields of two sets of trial records.
///
/// Matches records by `(task_id, trial)` pair, then asserts each deterministic
/// field is equal. Missing or extra records are reported as failures.
fn assert_trials_parity(py_trials: &[Value], rs_trials: &[Value], label: &str) {
    // Index by (task_id, trial).
    let py_map: HashMap<(String, u64), &Value> = py_trials
        .iter()
        .map(|r| {
            let tid = r["task_id"].as_str().unwrap_or("").to_string();
            let tn = r["trial"].as_u64().unwrap_or(0);
            ((tid, tn), r)
        })
        .collect();

    let rs_map: HashMap<(String, u64), &Value> = rs_trials
        .iter()
        .map(|r| {
            let tid = r["task_id"].as_str().unwrap_or("").to_string();
            let tn = r["trial"].as_u64().unwrap_or(0);
            ((tid, tn), r)
        })
        .collect();

    // Same set of keys.
    let mut py_keys: Vec<_> = py_map.keys().cloned().collect();
    let mut rs_keys: Vec<_> = rs_map.keys().cloned().collect();
    py_keys.sort();
    rs_keys.sort();
    assert_eq!(
        py_keys, rs_keys,
        "[{label}] trial (task_id, trial) key sets differ:\n  python={py_keys:?}\n  rust={rs_keys:?}"
    );

    // Per-record, per-field comparison.
    for key in &py_keys {
        let py_rec = py_map[key];
        let rs_rec = rs_map[key];
        for &field in DET_FIELDS {
            let py_val = py_rec.get(field).cloned().unwrap_or(Value::Null);
            let rs_val = rs_rec.get(field).cloned().unwrap_or(Value::Null);
            assert_eq!(
                py_val,
                rs_val,
                "[{label}] field `{field}` mismatch for task={} trial={}:\n  python={}\n  rust={}",
                key.0,
                key.1,
                serde_json::to_string_pretty(&py_val).unwrap(),
                serde_json::to_string_pretty(&rs_val).unwrap(),
            );
        }
    }
}

/// Compare the deterministic fields of two summary.json maps.
///
/// For each candidate in the Python summary, asserts:
/// - `composition_hash`, `kind`, `trials`, `errors`
/// - `reward_mean` (per round — both round to 4dp)
/// - per-task `mean`, `min`, `max` (not `rewards`/`wall_ms` arrays, which are
///   value-equal but order depends on task iteration order)
fn assert_summary_parity(py_exp: &Path, rs_exp: &Path, label: &str) {
    let py_raw =
        std::fs::read_to_string(py_exp.join("summary.json")).expect("read python summary.json");
    let rs_raw =
        std::fs::read_to_string(rs_exp.join("summary.json")).expect("read rust summary.json");

    let py_sum: Value = serde_json::from_str(&py_raw).expect("parse python summary.json");
    let rs_sum: Value = serde_json::from_str(&rs_raw).expect("parse rust summary.json");

    // Same candidate keys.
    let mut py_cands: Vec<_> = py_sum
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let mut rs_cands: Vec<_> = rs_sum
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    py_cands.sort();
    rs_cands.sort();
    assert_eq!(
        py_cands, rs_cands,
        "[{label}] summary candidate key sets differ"
    );

    for cand_id in &py_cands {
        let py_c = &py_sum[cand_id];
        let rs_c = &rs_sum[cand_id];

        for field in &[
            "composition_hash",
            "kind",
            "trials",
            "errors",
            "reward_mean",
        ] {
            let py_v = py_c.get(*field).cloned().unwrap_or(Value::Null);
            let rs_v = rs_c.get(*field).cloned().unwrap_or(Value::Null);
            assert_eq!(
                py_v,
                rs_v,
                "[{label}] summary[{cand_id}].{field} mismatch:\n  python={}\n  rust={}",
                serde_json::to_string_pretty(&py_v).unwrap(),
                serde_json::to_string_pretty(&rs_v).unwrap(),
            );
        }

        // Per-task aggregates.
        let py_tasks = py_c["tasks"].as_object().expect("tasks is object");
        let rs_tasks = rs_c["tasks"].as_object().expect("tasks is object");

        let mut py_tids: Vec<_> = py_tasks.keys().cloned().collect();
        let mut rs_tids: Vec<_> = rs_tasks.keys().cloned().collect();
        py_tids.sort();
        rs_tids.sort();
        assert_eq!(
            py_tids, rs_tids,
            "[{label}] summary[{cand_id}].tasks key sets differ"
        );

        for tid in &py_tids {
            let py_t = &py_tasks[tid];
            let rs_t = &rs_tasks[tid];
            for field in &["mean", "min", "max"] {
                let py_v = py_t.get(*field).cloned().unwrap_or(Value::Null);
                let rs_v = rs_t.get(*field).cloned().unwrap_or(Value::Null);
                assert_eq!(
                    py_v, rs_v,
                    "[{label}] summary[{cand_id}].tasks[{tid}].{field} mismatch:\n  python={}\n  rust={}",
                    serde_json::to_string_pretty(&py_v).unwrap(),
                    serde_json::to_string_pretty(&rs_v).unwrap(),
                );
            }
        }
    }
}

/// Shared test body: run both Python and Rust, compare trials + summary.
fn run_e2e_parity(candidate_rel: &str, arena_rel: &str) {
    if !python_available() {
        eprintln!("skipping e2e parity ({candidate_rel}): python3 not available");
        return;
    }

    // Use a thread-local unique suffix so concurrent test threads don't collide.
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);

    let repo = repo_root();
    let tmp = std::env::temp_dir().join(format!(
        "daedalus-e2e-{}-{n}-{}",
        std::process::id(),
        candidate_rel.replace(['/', '.'], "-")
    ));

    let py_runs = tmp.join("py");
    let rs_runs = tmp.join("rs");
    std::fs::create_dir_all(&py_runs).unwrap();
    std::fs::create_dir_all(&rs_runs).unwrap();

    let py_exp = run_python(&repo, candidate_rel, arena_rel, &py_runs);
    let rs_exp = run_rust(&repo, candidate_rel, arena_rel, &rs_runs);

    let label = format!("{candidate_rel} vs {arena_rel}");

    let py_trials = parse_trials(&py_exp.join("trials.jsonl"));
    let rs_trials = parse_trials(&rs_exp.join("trials.jsonl"));

    assert_eq!(
        py_trials.len(),
        rs_trials.len(),
        "[{label}] trials.jsonl record count differs: python={} rust={}",
        py_trials.len(),
        rs_trials.len()
    );

    assert_trials_parity(&py_trials, &rs_trials, &label);
    assert_summary_parity(&py_exp, &rs_exp, &label);

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn parity_e2e_null_candidate() {
    run_e2e_parity("candidates/null.toml", "arenas/pr-review-v0");
}

#[test]
fn parity_e2e_oracle_candidate() {
    run_e2e_parity("candidates/oracle.toml", "arenas/pr-review-v0");
}
