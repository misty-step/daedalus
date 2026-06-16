//! Parity oracle for the trace port: the Rust `experiment_trace` /
//! `write_trace` must agree with Python's over crafted fixtures and, when
//! present, the committed real capstone experiment. Skips when python3 is
//! unavailable, mirroring `bin/gate`.

use std::path::{Path, PathBuf};
use std::process::Command;

use daedalus_core::trace::{experiment_trace, write_trace};
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/daedalus-core")
        .to_path_buf()
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Python `trace.experiment_trace(dir)` as a JSON Value.
fn py_experiment_trace(root: &Path, exp_dir: &Path) -> Value {
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(
            "import sys, json; sys.path.insert(0, 'runner'); import trace; \
             print(json.dumps(trace.experiment_trace(sys.argv[1])))",
        )
        .arg(exp_dir)
        .output()
        .expect("run python trace");
    assert!(
        out.status.success(),
        "python trace failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("python trace emitted json")
}

/// Python `trace.write_trace(dir)` then read the bytes it wrote.
fn py_write_trace_bytes(root: &Path, exp_dir: &Path) -> Vec<u8> {
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(
            "import sys; sys.path.insert(0, 'runner'); import trace; \
             trace.write_trace(sys.argv[1])",
        )
        .arg(exp_dir)
        .output()
        .expect("run python write_trace");
    assert!(out.status.success(), "python write_trace failed");
    std::fs::read(exp_dir.join("trace.otel.json")).unwrap()
}

fn write_trials(dir: &Path, records: &[Value]) {
    let body: String = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("trials.jsonl"), body + "\n").unwrap();
}

#[test]
fn experiment_trace_matches_python_on_fixtures() {
    if !python_available() {
        eprintln!("skipping trace parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = std::env::temp_dir().join(format!("daedalus-trace-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    write_trials(
        &dir,
        &[
            serde_json::json!({
                "run_id": "r1", "ts_start": "t0", "ts_end": "t1",
                "candidate_id": "seed1", "candidate_kind": "pi", "task_id": "alpha",
                "trial": 1, "model": "z-ai/glm-5", "provider_served": "openrouter",
                "composition_hash": "abc", "tokens_prompt": 100,
                "tokens_completion": 20, "cost_usd": 0.011, "reward": 1.0,
                "false_positives": 0, "error": null,
            }),
            serde_json::json!({
                "run_id": "r2", "candidate_id": "seed2", "task_id": "beta",
                "trial": 1, "error": "pi exited 1", "cost_usd": null, "reward": 0.0,
            }),
            serde_json::json!({
                "run_id": "r3", "candidate_id": "seed1", "task_id": "beta",
                "trial": 2, "model": "m", "cost_usd": 0.0245, "reward": 0.5,
            }),
        ],
    );

    // semantic agreement
    let py = py_experiment_trace(&root, &dir);
    let rust = serde_json::to_value(experiment_trace(&dir)).unwrap();
    assert_eq!(py, rust, "experiment_trace differs\npy={py}\nrust={rust}");

    // byte agreement of the written artifact (python writes, then rust overwrites)
    let py_bytes = py_write_trace_bytes(&root, &dir);
    write_trace(&dir).unwrap();
    let rust_bytes = std::fs::read(dir.join("trace.otel.json")).unwrap();
    assert_eq!(
        py_bytes,
        rust_bytes,
        "trace.otel.json bytes differ\npy={}\nrust={}",
        String::from_utf8_lossy(&py_bytes),
        String::from_utf8_lossy(&rust_bytes)
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn experiment_trace_matches_python_on_real_capstone() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let capstone = root.join("runs/20260610T160533Z-search-pr-review-v0");
    if !capstone.join("trials.jsonl").exists() {
        eprintln!("skipping capstone parity: records not present in this checkout");
        return;
    }
    let py = py_experiment_trace(&root, &capstone);
    let rust = serde_json::to_value(experiment_trace(&capstone)).unwrap();
    assert_eq!(py, rust, "capstone experiment_trace differs");
}
