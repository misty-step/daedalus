//! Parity oracle for the search loop: drive the real Python `loop.run_search`
//! and the Rust port over identical scripted worlds and assert the full result
//! (stop_reason, history with per-child parent_id, best_id, spend) agrees. The
//! multi-parent scenario is the real test of the `PyRandom` shuffle integration:
//! parent order — and thus the trajectory — depends on `random.Random(0)`.
//!
//! Skips when python3 is unavailable.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use daedalus_core::pyrandom::PyRandom;
use daedalus_core::search_loop::{run_search, SearchParams, SearchWorld};
use serde_json::{json, Map, Value};

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Build a summary entry from `{task: [rewards]}` (mirrors test_loop.stats and
/// the Python driver below).
fn stats(task_rewards: &Value, cost: f64, kind: &str) -> Value {
    let mut tasks = Map::new();
    let mut flat: Vec<f64> = Vec::new();
    for (t, rs_v) in task_rewards.as_object().unwrap() {
        let rs: Vec<f64> = rs_v
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let mean = rs.iter().sum::<f64>() / rs.len() as f64;
        let min = rs.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        tasks.insert(t.clone(), json!({"rewards": rs, "mean": mean, "min": min, "max": max, "wall_ms": vec![1000.0; rs.len()]}));
        flat.extend(rs);
    }
    json!({
        "kind": kind,
        "tasks": Value::Object(tasks),
        "trials": flat.len(),
        "reward_mean": daedalus_core::pycompat::round_half_even(flat.iter().sum::<f64>() / flat.len() as f64, 4),
        "cost_usd_total": cost,
    })
}

struct FakeWorld {
    summary: Map<String, Value>,
    script: Vec<Value>,
    pending: Map<String, Value>,
}

impl FakeWorld {
    fn new(candidates: &Value, script: &Value) -> Self {
        let mut summary = Map::new();
        for (cid, c) in candidates.as_object().unwrap() {
            summary.insert(
                cid.clone(),
                stats(
                    &c["task_rewards"],
                    c["cost"].as_f64().unwrap(),
                    c.get("kind").and_then(Value::as_str).unwrap_or("pi"),
                ),
            );
        }
        FakeWorld {
            summary,
            script: script.as_array().unwrap().clone(),
            pending: Map::new(),
        }
    }
}

impl SearchWorld for FakeWorld {
    fn summary(&mut self) -> Map<String, Value> {
        self.summary.clone()
    }
    fn propose(
        &mut self,
        _p: &str,
        _g: u64,
        _a: usize,
        _av: &[String],
    ) -> Result<(String, Value), String> {
        if self.script.is_empty() {
            return Err("script exhausted".to_string());
        }
        let item = self.script.remove(0);
        if item == json!("fail") {
            return Err("optimizer returned garbage".to_string());
        }
        let id = item["id"].as_str().unwrap().to_string();
        self.pending.insert(
            id.clone(),
            stats(&item["task_rewards"], item["cost"].as_f64().unwrap(), "pi"),
        );
        Ok((
            id.clone(),
            json!({"child_id": id, "slot_changed": "prompt_packet", "hypothesis": "h"}),
        ))
    }
    fn run_child(&mut self, child_id: &str) {
        let v = self.pending.remove(child_id).unwrap();
        self.summary.insert(child_id.to_string(), v);
    }
}

const PY_DRIVER: &str = r#"
import sys, json
sys.path.insert(0, 'runner')
import loop

def stats(task_rewards, cost, kind):
    tasks = {t: {"rewards": rs, "mean": sum(rs)/len(rs), "min": min(rs), "max": max(rs), "wall_ms": [1000]*len(rs)} for t, rs in task_rewards.items()}
    flat = [x for rs in task_rewards.values() for x in rs]
    return {"kind": kind, "tasks": tasks, "trials": len(flat), "reward_mean": round(sum(flat)/len(flat), 4), "cost_usd_total": cost}

class FakeWorld:
    def __init__(self, candidates, script):
        self.summary = {cid: stats(c["task_rewards"], c["cost"], c.get("kind", "pi")) for cid, c in candidates.items()}
        self.script = list(script); self.pending = {}
    def summary_fn(self): return {k: dict(v) for k, v in self.summary.items()}
    def propose_fn(self, parent, gen, attempt, avoid):
        if not self.script: raise ValueError("script exhausted")
        item = self.script.pop(0)
        if item == "fail": raise ValueError("optimizer returned garbage")
        cid = item["id"]
        self.pending[cid] = stats(item["task_rewards"], item["cost"], "pi")
        return cid, {"child_id": cid, "slot_changed": "prompt_packet", "hypothesis": "h"}
    def run_child_fn(self, cid): self.summary[cid] = self.pending.pop(cid)

scn = json.load(sys.stdin)
w = FakeWorld(scn["candidates"], scn["script"])
out = loop.run_search(summary_fn=w.summary_fn, propose_fn=w.propose_fn, run_child_fn=w.run_child_fn, optimizer_costs=[], **scn["kwargs"])
print(json.dumps(out))
"#;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/daedalus-core")
        .to_path_buf()
}

fn py_run(scenario: &Value) -> Value {
    let mut child = Command::new("python3")
        .current_dir(repo_root())
        .arg("-c")
        .arg(PY_DRIVER)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn python");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(scenario).unwrap().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "python loop failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("python loop emitted json")
}

fn params_from(kwargs: &Value) -> SearchParams {
    let mut p = SearchParams::default();
    if let Some(v) = kwargs.get("max_children").and_then(Value::as_u64) {
        p.max_children = v as usize;
    }
    if let Some(v) = kwargs.get("budget_usd").and_then(Value::as_f64) {
        p.budget_usd = Some(v);
    }
    if let Some(v) = kwargs.get("plateau_limit").and_then(Value::as_u64) {
        p.plateau_limit = v as usize;
    }
    if let Some(v) = kwargs
        .get("children_per_generation")
        .and_then(Value::as_u64)
    {
        p.children_per_generation = v as usize;
    }
    if let Some(v) = kwargs.get("mode").and_then(Value::as_str) {
        p.mode = v.to_string();
    }
    p
}

fn check(label: &str, scenario: Value) {
    let py = py_run(&scenario);
    let mut world = FakeWorld::new(&scenario["candidates"], &scenario["script"]);
    let params = params_from(&scenario["kwargs"]);
    let rust = run_search(&mut world, &params, &mut PyRandom::new(0));
    assert_eq!(
        py, rust,
        "[{label}] loop result differs\npy={py}\nrust={rust}"
    );
}

#[test]
fn loop_matches_python_across_scenarios() {
    if !python_available() {
        eprintln!("skipping loop parity: python3 not available");
        return;
    }

    // single-parent plateau (BASE-like: only `base` is non-reference)
    check(
        "plateau-single-parent",
        json!({
            "candidates": {
                "oracle": {"task_rewards": {"t1": [1.0], "t2": [1.0]}, "cost": 0.0, "kind": "oracle"},
                "null": {"task_rewards": {"t1": [0.0], "t2": [0.5]}, "cost": 0.0, "kind": "null"},
                "base": {"task_rewards": {"t1": [0.6, 0.6], "t2": [0.5, 0.5]}, "cost": 0.05}
            },
            "script": [
                {"id": "c1", "task_rewards": {"t1": [0.58], "t2": [0.5]}, "cost": 0.01},
                {"id": "c2", "task_rewards": {"t1": [0.55], "t2": [0.5]}, "cost": 0.01},
                {"id": "c3", "task_rewards": {"t1": [0.59], "t2": [0.5]}, "cost": 0.01},
                {"id": "c4", "task_rewards": {"t1": [0.3], "t2": [0.2]}, "cost": 0.01}
            ],
            "kwargs": {"max_children": 10, "budget_usd": 100.0}
        }),
    );

    // MULTI-parent: pool = [a, b, c]; rng.shuffle drives which parent each child
    // gets, so the per-child parent_id is the real PyRandom cross-check.
    check(
        "multi-parent-trajectory",
        json!({
            "candidates": {
                "null": {"task_rewards": {"t1": [0.0], "t2": [0.0]}, "cost": 0.0, "kind": "null"},
                "oracle": {"task_rewards": {"t1": [1.0], "t2": [1.0]}, "cost": 0.0, "kind": "oracle"},
                "a": {"task_rewards": {"t1": [0.9, 0.9], "t2": [0.1, 0.1]}, "cost": 0.05},
                "b": {"task_rewards": {"t1": [0.1, 0.1], "t2": [0.9, 0.9]}, "cost": 0.05},
                "c": {"task_rewards": {"t1": [0.5, 0.5], "t2": [0.5, 0.5]}, "cost": 0.02}
            },
            "script": [
                {"id": "k1", "task_rewards": {"t1": [0.5], "t2": [0.5]}, "cost": 0.01},
                {"id": "k2", "task_rewards": {"t1": [0.55], "t2": [0.5]}, "cost": 0.01},
                {"id": "k3", "task_rewards": {"t1": [0.5], "t2": [0.52]}, "cost": 0.01},
                {"id": "k4", "task_rewards": {"t1": [0.51], "t2": [0.5]}, "cost": 0.01}
            ],
            "kwargs": {"max_children": 4, "budget_usd": 100.0, "plateau_limit": 99}
        }),
    );

    // budget stop before the second generation's spend
    check(
        "budget-stop",
        json!({
            "candidates": {
                "oracle": {"task_rewards": {"t1": [1.0]}, "cost": 0.0, "kind": "oracle"},
                "null": {"task_rewards": {"t1": [0.0]}, "cost": 0.0, "kind": "null"},
                "base": {"task_rewards": {"t1": [0.6, 0.6], "t2": [0.5, 0.5]}, "cost": 0.05}
            },
            "script": [
                {"id": "c1", "task_rewards": {"t1": [0.9], "t2": [0.9]}, "cost": 3.0},
                {"id": "c2", "task_rewards": {"t1": [0.95], "t2": [0.95]}, "cost": 3.0}
            ],
            "kwargs": {"max_children": 10, "budget_usd": 3.0, "children_per_generation": 1}
        }),
    );

    // proposal failures stop
    check(
        "proposal-failures",
        json!({
            "candidates": {
                "oracle": {"task_rewards": {"t1": [1.0]}, "cost": 0.0, "kind": "oracle"},
                "null": {"task_rewards": {"t1": [0.0]}, "cost": 0.0, "kind": "null"},
                "base": {"task_rewards": {"t1": [0.6, 0.6]}, "cost": 0.05}
            },
            "script": ["fail", "fail"],
            "kwargs": {"max_children": 10, "budget_usd": 100.0}
        }),
    );
}
