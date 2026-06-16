//! Parity oracle for the lineage port: the Rust `render` and `notebook_entry`
//! must agree byte-for-byte with Python's over crafted fixtures. Skips when
//! python3 is unavailable, mirroring `bin/gate`.
//!
//! Cases exercised:
//!   - full run (all artifacts present, seeds, hypothesis verdicts, alarms,
//!     certified list, pareto recommendation, proposal errors)
//!   - missing all optional artifacts (bare directory)
//!   - multiple seeds sorted by index
//!   - error trials (cost_usd null) contributing zero cost
//!   - hypothesis verdict variants: confirmed, refuted, partially confirmed
//!   - donor transplant field
//!   - empty certified list (renders "none")
//!   - notebook_entry with whitespace-collapsed hypothesis
//!   - notebook_entry with no recommended pareto entry
//!   - alarm truncation (more than 3 alarms → only first 3 in notebook)

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::lineage::{hypothesis_verdict, notebook_entry, render};
use serde_json::{json, Value};

static COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn fresh_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "daedalus-parity-lineage-{}-{n}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run Python `lineage.render(exp_dir)` and return the output string.
fn py_render(root: &Path, exp_dir: &Path) -> String {
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(
            "import sys; sys.path.insert(0, 'runner'); import lineage; \
             print(lineage.render(sys.argv[1]), end='')",
        )
        .arg(exp_dir)
        .output()
        .expect("run python lineage.render");
    assert!(
        out.status.success(),
        "python lineage.render failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("python lineage.render output was not utf-8")
}

/// Run Python `lineage.notebook_entry(exp_dir, spec, arena_cfg)`.
fn py_notebook_entry(root: &Path, exp_dir: &Path, spec: &Value, arena_cfg: &Value) -> String {
    let code = format!(
        "import sys, json; sys.path.insert(0, 'runner'); import lineage; \
         print(lineage.notebook_entry(sys.argv[1], {}, {}), end='')",
        serde_json::to_string(spec).unwrap(),
        serde_json::to_string(arena_cfg).unwrap(),
    );
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&code)
        .arg(exp_dir)
        .output()
        .expect("run python lineage.notebook_entry");
    assert!(
        out.status.success(),
        "python lineage.notebook_entry failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("python notebook_entry output was not utf-8")
}

/// Run Python `lineage.hypothesis_verdict(h)` — returns the JSON-encoded tuple
/// or null when Python returns None.
/// We pass `h` via a JSON file to avoid shell-quoting issues with null/true/false.
fn py_hypothesis_verdict(root: &Path, h: &Value) -> Value {
    let dir = std::env::temp_dir().join(format!("daedalus-hv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let json_path = dir.join("h.json");
    std::fs::write(&json_path, serde_json::to_string(h).unwrap()).unwrap();

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(
            "import sys, json; sys.path.insert(0, 'runner'); import lineage; \
             h = json.loads(open(sys.argv[1]).read()); \
             r = lineage.hypothesis_verdict(h); \
             print(json.dumps(list(r) if r else None))",
        )
        .arg(&json_path)
        .output()
        .expect("run python hypothesis_verdict");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "python hypothesis_verdict failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("python hypothesis_verdict emitted json")
}

fn write_json(dir: &Path, name: &str, value: &Value) {
    std::fs::write(dir.join(name), serde_json::to_string(value).unwrap()).unwrap();
}

fn write_trials(dir: &Path, records: &[Value]) {
    let body: String = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("trials.jsonl"), body + "\n").unwrap();
}

/// Build the standard full-run fixture (mirrors `tests/test_lineage.py:build_run`).
fn build_full_run(parent: &Path) -> PathBuf {
    let exp = parent.join("20260610T000000Z-search-demo");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(
        &exp,
        "rig.json",
        &json!({
            "oracle_mean": 1.0, "null_mean": 0.25, "probe_mean": 0.0,
            "saturated": false
        }),
    );
    write_json(
        &exp,
        "seed.json",
        &json!({
            "rng_seed": 7, "seed_count": 2,
            "packet_stances": ["spec-first", "skeptic"],
            "optimizer_costs": [0.01],
            "combos": [
                {"model": "z-ai/glm-5", "thinking": "high", "policy_name": "full"},
                {"model": "openai/gpt-5-mini", "thinking": "low", "policy_name": "explore"}
            ]
        }),
    );
    write_json(
        &exp,
        "loop.json",
        &json!({
            "stop_reason": "plateau", "mode": "threshold-then-cheap",
            "generations": 2, "spend_known_usd": 1.23,
            "certified": ["seed1-glm-5-spec-first"],
            "alarms": [{"kind": "saturation-at-top",
                        "detail": "seed1 at ceiling; cost search only"}],
            "history": [
                {
                    "generation": 1, "attempt": 0,
                    "child_id": "g1a-seed1", "parent_id": "seed1-glm-5-spec-first",
                    "slot_changed": "thinking",
                    "hypothesis": "medium thinking keeps reward and cuts cost",
                    "predicted_effect": {"reward": "hold", "cost": "down"},
                    "parent_cost_per_trial": 0.0171, "child_cost_per_trial": 0.0138,
                    "parent_reward_mean": 1.0, "reward_mean": 1.0,
                    "mean_task_delta": 0.0, "improved": true
                },
                {
                    "generation": 2, "attempt": 0,
                    "child_id": "g2a-g1a", "parent_id": "g1a-seed1",
                    "slot_changed": "prompt_packet",
                    "hypothesis": "stop instruction reduces spend",
                    "parent_reward_mean": 1.0, "reward_mean": 0.66,
                    "mean_task_delta": -0.33, "improved": false
                },
                {
                    "generation": 2, "attempt": 1,
                    "parent_id": "g1a-seed1", "proposal_error": "slot not mutable"
                }
            ]
        }),
    );
    write_json(
        &exp,
        "pareto.json",
        &json!([{
            "candidate_id": "g1a-seed1", "composition_hash": "abc123",
            "reward_mean": 1.0, "cost_usd_per_trial": 0.0138,
            "certified": true, "recommended": true
        }]),
    );
    write_trials(
        &exp,
        &[
            json!({"candidate_id": "seed1-glm-5-spec-first", "candidate_kind": "pi",
                   "task_id": "t1", "reward": 1.0, "cost_usd": 0.02}),
            json!({"candidate_id": "seed2-gpt-5-mini-skeptic", "candidate_kind": "pi",
                   "task_id": "t1", "reward": 0.2, "cost_usd": 0.01}),
            json!({"candidate_id": "oracle", "candidate_kind": "oracle",
                   "task_id": "t1", "reward": 1.0, "cost_usd": null}),
        ],
    );
    exp
}

// ─────────────────────────────────────────────────────────────────────────────
// parity tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn render_matches_python_full_run() {
    if !python_available() {
        eprintln!("skipping lineage parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = build_full_run(&dir);

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(py, rust, "render output differs\npy={py}\nrust={rust}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_matches_python_bare_directory() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("bare-exp");
    std::fs::create_dir_all(&exp).unwrap();

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(py, rust, "bare render differs\npy={py}\nrust={rust}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_matches_python_multiple_seeds_sorted() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-multi-seed");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(
        &exp,
        "rig.json",
        &json!({"oracle_mean": 0.8, "null_mean": 0.1, "probe_mean": 0.3, "saturated": false}),
    );
    write_json(
        &exp,
        "seed.json",
        &json!({
            "rng_seed": 42, "seed_count": 3,
            "packet_stances": ["alpha", "beta", "gamma"],
            "combos": [
                {"model": "m1", "thinking": "high", "policy_name": "p1"},
                {"model": "m2", "thinking": "low",  "policy_name": "p2"},
                {"model": "m3", "thinking": "mid",  "policy_name": "p3"}
            ]
        }),
    );
    write_json(&exp, "loop.json", &json!({"history": [], "alarms": []}));
    write_json(&exp, "pareto.json", &json!([]));
    // Write trials out of order (seed3, seed1, seed2) to test stable sort
    write_trials(
        &exp,
        &[
            json!({"candidate_id": "seed3-m3-gamma", "task_id": "t1", "reward": 0.5, "cost_usd": 0.03}),
            json!({"candidate_id": "seed1-m1-alpha", "task_id": "t1", "reward": 0.9, "cost_usd": 0.01}),
            json!({"candidate_id": "seed2-m2-beta",  "task_id": "t1", "reward": 0.7, "cost_usd": 0.02}),
        ],
    );

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(py, rust, "multi-seed render differs\npy={py}\nrust={rust}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_matches_python_error_trials_null_cost() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-err-trials");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(
        &exp,
        "rig.json",
        &json!({"oracle_mean": 1.0, "null_mean": 0.0, "probe_mean": 0.5, "saturated": false}),
    );
    write_json(
        &exp,
        "seed.json",
        &json!({"rng_seed": 1, "packet_stances": [], "combos": [{"model": "m", "thinking": "x", "policy_name": "y"}]}),
    );
    write_json(&exp, "loop.json", &json!({"history": [], "alarms": []}));
    write_json(&exp, "pareto.json", &json!([]));
    write_trials(
        &exp,
        &[
            json!({"candidate_id": "seed1-m-y", "task_id": "t1", "reward": 1.0, "cost_usd": null}),
            json!({"candidate_id": "seed1-m-y", "task_id": "t2", "reward": 0.0, "cost_usd": null}),
        ],
    );

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(
        py, rust,
        "error-trials render differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_matches_python_hypothesis_verdicts() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-verdicts");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(
        &exp,
        "rig.json",
        &json!({"oracle_mean": 1.0, "null_mean": 0.0, "probe_mean": 0.5, "saturated": false}),
    );
    write_json(
        &exp,
        "seed.json",
        &json!({"rng_seed": 1, "packet_stances": [], "combos": []}),
    );
    write_json(
        &exp,
        "loop.json",
        &json!({
            "history": [
                // confirmed (reward hold, cost down)
                {
                    "generation": 1, "attempt": 0,
                    "child_id": "g1", "parent_id": "p1", "slot_changed": "s",
                    "hypothesis": "h-confirmed",
                    "predicted_effect": {"reward": "hold", "cost": "down"},
                    "parent_cost_per_trial": 0.02, "child_cost_per_trial": 0.015,
                    "parent_reward_mean": 0.9, "reward_mean": 0.9,
                    "mean_task_delta": 0.0, "improved": true
                },
                // refuted (reward up failed, cost down failed)
                {
                    "generation": 1, "attempt": 1,
                    "child_id": "g1b", "parent_id": "p1", "slot_changed": "s",
                    "hypothesis": "h-refuted",
                    "predicted_effect": {"reward": "up", "cost": "down"},
                    "parent_cost_per_trial": 0.02, "child_cost_per_trial": 0.025,
                    "parent_reward_mean": 0.9, "reward_mean": 0.8,
                    "mean_task_delta": -0.1, "improved": false
                },
                // partial (reward up ok, cost down failed)
                {
                    "generation": 2, "attempt": 0,
                    "child_id": "g2", "parent_id": "g1", "slot_changed": "s",
                    "hypothesis": "h-partial",
                    "predicted_effect": {"reward": "up", "cost": "down"},
                    "parent_cost_per_trial": 0.02, "child_cost_per_trial": 0.025,
                    "parent_reward_mean": 0.9, "reward_mean": 1.0,
                    "mean_task_delta": 0.1, "improved": true
                },
                // no structured prediction (legacy)
                {
                    "generation": 3, "attempt": 0,
                    "child_id": "g3", "parent_id": "g2", "slot_changed": "t",
                    "hypothesis": "h-legacy",
                    "parent_reward_mean": 1.0, "reward_mean": 0.5,
                    "mean_task_delta": -0.5, "improved": false
                }
            ],
            "alarms": []
        }),
    );
    write_json(&exp, "pareto.json", &json!([]));

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(
        py, rust,
        "hypothesis-verdicts render differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_matches_python_donor_transplant() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-donor");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(&exp, "rig.json", &json!({}));
    write_json(&exp, "seed.json", &json!({}));
    write_json(
        &exp,
        "loop.json",
        &json!({
            "history": [{
                "generation": 1, "attempt": 0,
                "child_id": "g1", "parent_id": "p1", "slot_changed": "model",
                "donor": "seed2-some-model",
                "hypothesis": "transplant donor model improves reward",
                "parent_reward_mean": 0.5, "reward_mean": 0.8,
                "mean_task_delta": 0.3, "improved": true
            }],
            "alarms": []
        }),
    );
    write_json(&exp, "pareto.json", &json!([]));

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(
        py, rust,
        "donor transplant render differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_matches_python_empty_certified_list() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-no-cert");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(&exp, "rig.json", &json!({}));
    write_json(&exp, "seed.json", &json!({}));
    write_json(
        &exp,
        "loop.json",
        &json!({"certified": [], "history": [], "alarms": []}),
    );
    write_json(&exp, "pareto.json", &json!([]));

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(
        py, rust,
        "empty-certified render differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_matches_python_saturated_rig() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-saturated");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(
        &exp,
        "rig.json",
        &json!({"oracle_mean": 1.0, "null_mean": 1.0, "probe_mean": 1.0, "saturated": true}),
    );
    write_json(&exp, "seed.json", &json!({}));
    write_json(&exp, "loop.json", &json!({"history": [], "alarms": []}));
    write_json(&exp, "pareto.json", &json!([]));

    let py = py_render(&root, &exp);
    let rust = render(&exp);
    assert_eq!(
        py, rust,
        "saturated-rig render differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn notebook_entry_matches_python_full_run() {
    if !python_available() {
        eprintln!("skipping lineage parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = build_full_run(&dir);

    let spec = json!({"id": "pr-review", "mode": "threshold-then-cheap"});
    let arena_cfg = json!({"id": "pr-review-v2", "version": "0.1.0"});

    let py = py_notebook_entry(&root, &exp, &spec, &arena_cfg);
    let rust = notebook_entry(&exp, &spec, &arena_cfg);
    assert_eq!(py, rust, "notebook_entry differs\npy={py}\nrust={rust}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn notebook_entry_matches_python_whitespace_collapse() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = build_full_run(&dir);

    // Overwrite loop.json with multiline hypothesis
    let loop_path = exp.join("loop.json");
    let mut loopj: Value =
        serde_json::from_str(&std::fs::read_to_string(&loop_path).unwrap()).unwrap();
    loopj["history"][0]["hypothesis"] = json!("first line\nsecond line   ");
    std::fs::write(&loop_path, serde_json::to_string(&loopj).unwrap()).unwrap();

    let spec = json!({"id": "pr-review", "mode": "threshold-then-cheap"});
    let arena_cfg = json!({"id": "pr-review-v2", "version": "0.1.0"});

    let py = py_notebook_entry(&root, &exp, &spec, &arena_cfg);
    let rust = notebook_entry(&exp, &spec, &arena_cfg);
    assert_eq!(
        py, rust,
        "whitespace-collapse notebook_entry differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn notebook_entry_matches_python_no_recommended() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-no-pick");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(
        &exp,
        "loop.json",
        &json!({
            "stop_reason": "budget", "spend_known_usd": 0.5, "generations": 1,
            "history": [], "alarms": []
        }),
    );
    write_json(
        &exp,
        "pareto.json",
        &json!([{
            "candidate_id": "seed1-x", "composition_hash": "h1",
            "reward_mean": 0.5, "recommended": false
        }]),
    );

    let spec = json!({"id": "spec-x", "mode": "explore"});
    let arena_cfg = json!({"id": "arena-x", "version": "0.2.0"});

    let py = py_notebook_entry(&root, &exp, &spec, &arena_cfg);
    let rust = notebook_entry(&exp, &spec, &arena_cfg);
    assert_eq!(
        py, rust,
        "no-recommended notebook_entry differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn notebook_entry_matches_python_alarm_truncation() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    let dir = fresh_dir();
    let exp = dir.join("20260610T000000Z-alarms");
    std::fs::create_dir_all(&exp).unwrap();

    write_json(
        &exp,
        "loop.json",
        &json!({
            "stop_reason": "budget", "spend_known_usd": 1.0, "generations": 3,
            "history": [],
            "alarms": [
                {"kind": "alarm1", "detail": "detail one"},
                {"kind": "alarm2", "detail": "detail two"},
                {"kind": "alarm3", "detail": "detail three"},
                {"kind": "alarm4", "detail": "detail four should not appear"}
            ]
        }),
    );
    write_json(&exp, "pareto.json", &json!([]));

    let spec = json!({"id": "s", "mode": "m"});
    let arena_cfg = json!({"id": "a", "version": "0.1"});

    let py = py_notebook_entry(&root, &exp, &spec, &arena_cfg);
    let rust = notebook_entry(&exp, &spec, &arena_cfg);
    assert_eq!(
        py, rust,
        "alarm-truncation notebook_entry differs\npy={py}\nrust={rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hypothesis_verdict_matches_python() {
    if !python_available() {
        eprintln!("skipping lineage parity: python3 not available");
        return;
    }
    let root = repo_root();

    let cases: Vec<(&str, Value)> = vec![
        ("no-prediction", json!({"improved": true})),
        ("empty-predicted-effect", json!({"predicted_effect": {}})),
        (
            "confirmed-hold-down",
            json!({
                "predicted_effect": {"reward": "hold", "cost": "down"},
                "mean_task_delta": 0.0,
                "parent_cost_per_trial": 0.0171, "child_cost_per_trial": 0.0138
            }),
        ),
        (
            "refuted-up-up",
            json!({
                "predicted_effect": {"reward": "up", "cost": "down"},
                "mean_task_delta": -0.2,
                "parent_cost_per_trial": 0.01, "child_cost_per_trial": 0.02
            }),
        ),
        (
            "partial-up-ok-down-failed",
            json!({
                "predicted_effect": {"reward": "up", "cost": "down"},
                "mean_task_delta": 0.3,
                "parent_cost_per_trial": 0.01, "child_cost_per_trial": 0.02
            }),
        ),
        (
            "reward-only-no-cost-keys",
            json!({
                "predicted_effect": {"reward": "up"},
                "mean_task_delta": 0.05
            }),
        ),
        (
            "cost-only-no-delta",
            json!({
                "predicted_effect": {"cost": "down"},
                "parent_cost_per_trial": 0.01, "child_cost_per_trial": 0.008
            }),
        ),
        (
            "cost-hold",
            json!({
                "predicted_effect": {"reward": "hold", "cost": "hold"},
                "mean_task_delta": 0.01,
                "parent_cost_per_trial": 0.01, "child_cost_per_trial": 0.0105
            }),
        ),
        (
            "cost-up-always-ok",
            json!({
                "predicted_effect": {"cost": "up"},
                "parent_cost_per_trial": 0.01, "child_cost_per_trial": 0.05
            }),
        ),
        (
            "pcpt-zero-skips-cost-axis",
            json!({
                "predicted_effect": {"cost": "down"},
                "parent_cost_per_trial": 0.0, "child_cost_per_trial": 0.0
            }),
        ),
        (
            "ccpt-null-skips-cost-axis",
            json!({
                "predicted_effect": {"cost": "down"},
                "parent_cost_per_trial": 0.01, "child_cost_per_trial": null
            }),
        ),
        (
            "delta-exactly-0.02-not-up",
            json!({
                "predicted_effect": {"reward": "up"},
                "mean_task_delta": 0.02
            }),
        ),
        (
            "delta-just-above-0.02-is-up",
            json!({
                "predicted_effect": {"reward": "up"},
                "mean_task_delta": 0.021
            }),
        ),
        (
            "unknown-cost-pred-defaults-true",
            json!({
                "predicted_effect": {"cost": "sideways"},
                "parent_cost_per_trial": 0.01, "child_cost_per_trial": 0.02
            }),
        ),
    ];

    for (label, h) in &cases {
        let py = py_hypothesis_verdict(&root, h);
        let rust = hypothesis_verdict(h)
            .map(|(l, d)| json!([l, d]))
            .unwrap_or(Value::Null);
        assert_eq!(
            py, rust,
            "[{label}] hypothesis_verdict differs\npy={py}\nrust={rust}"
        );
    }
}
