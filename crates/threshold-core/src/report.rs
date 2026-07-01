//! Comparison report over experiment run records.
//!
//! Port of `runner/report.py`. Aggregates trial records by candidate, computes
//! a per-task reward grid, totals, the Pareto set over (reward mean ↑, cost ↓,
//! wall ↓), and a recommendation.
//!
//! Reference candidates appear in the grid but never in the Pareto set or
//! recommendation: null (floor), oracle (ceiling), and any oneshot-kind probe
//! (saturation detector).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::pycompat::round_half_even;

// "incumbent" (055): a real-trial reference (the deployed config), excluded
// from the Pareto front and recommendation like the other references.
const REFERENCE_KINDS: &[&str] = &["null", "oracle", "oneshot", "incumbent"];
const COSTLESS_KINDS: &[&str] = &["null", "oracle"];

pub fn is_reference_kind(kind: Option<&str>) -> bool {
    match kind {
        Some(k) => REFERENCE_KINDS.contains(&k),
        None => false,
    }
}

/// The canonical leaderboard reading order, keyed by `(is_reference,
/// reward_mean)`: non-reference candidates first, then by descending mean reward
/// (NaN sorts as equal). The single source of this rule — `report_html` and
/// `view` both order their rows by it, so the live view and the HTML report
/// never disagree on who leads. (This crate's markdown `render` deliberately
/// sorts reward-desc only, for `runner/report.py` parity, and does not use it.)
pub fn cmp_leaderboard(a: (bool, f64), b: (bool, f64)) -> std::cmp::Ordering {
    a.0.cmp(&b.0)
        .then(b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
}

fn is_costless_kind(kind: Option<&str>) -> bool {
    match kind {
        Some(k) => COSTLESS_KINDS.contains(&k),
        None => false,
    }
}

/// Replicate Python's `str(float)` when float has been rounded to `ndigits`
/// decimal places. Python's `str(1.0)` → `"1.0"`, not `"1"`. We use a fixed
/// number of decimal places matching the Python call site.
fn py_float_str_1(x: f64) -> String {
    // wall_mean is rounded to 1 decimal place; Python str() always shows it.
    format!("{x:.1}")
}

/// Load records from a list of paths (directories containing trials.jsonl, or
/// .jsonl files directly). Mirrors `load_records(paths)` in report.py.
pub fn load_records(paths: &[impl AsRef<Path>]) -> Vec<Value> {
    let mut records = Vec::new();
    for p in paths {
        let p = p.as_ref();
        let file: PathBuf = if p.is_dir() {
            p.join("trials.jsonl")
        } else {
            p.to_path_buf()
        };
        if let Ok(text) = std::fs::read_to_string(&file) {
            for line in text.lines() {
                if !line.is_empty() {
                    if let Ok(v) = serde_json::from_str(line) {
                        records.push(v);
                    }
                }
            }
        }
    }
    records
}

/// Aggregate trial records into a candidate map. Mirrors `aggregate(records)`.
///
/// Returns a `Map` with preserve_order so insertion order is kept (matching
/// Python dict semantics). Each value is a JSON object with fields in Python
/// dict insertion order.
pub fn aggregate(records: &[Value]) -> Map<String, Value> {
    // Per-candidate mutable state, kept as parallel Vecs to preserve order.
    struct Cand {
        id: String,
        kind: Option<String>,
        model: Option<String>,
        hash: Option<String>,
        // tasks: insertion-ordered pairs of (task_id, rewards)
        tasks: Vec<(String, Vec<f64>)>,
        task_index: HashMap<String, usize>,
        trials: u64,
        voided: u64,
        cost: f64,
        cost_known: bool,
        walls: Vec<f64>,
    }

    let mut cand_order: Vec<String> = Vec::new();
    let mut cands: HashMap<String, Cand> = HashMap::new();

    for r in records {
        let candidate_id = match r.get("candidate_id").and_then(Value::as_str) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let task_id = match r.get("task_id").and_then(Value::as_str) {
            Some(s) => s.to_string(),
            None => continue,
        };

        // Python setdefault: insert only if absent, preserving first-seen order.
        if !cands.contains_key(&candidate_id) {
            cand_order.push(candidate_id.clone());
            cands.insert(
                candidate_id.clone(),
                Cand {
                    id: candidate_id.clone(),
                    kind: r
                        .get("candidate_kind")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    model: r.get("model").and_then(Value::as_str).map(str::to_string),
                    hash: r
                        .get("composition_hash")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    tasks: Vec::new(),
                    task_index: HashMap::new(),
                    trials: 0,
                    voided: 0,
                    cost: 0.0,
                    cost_known: true,
                    walls: Vec::new(),
                },
            );
        }

        let c = cands.get_mut(&candidate_id).unwrap();
        c.trials += 1;

        // if r.get("error"): any truthy value
        let errored = r
            .get("error")
            .map(|v| match v {
                Value::Null => false,
                Value::Bool(b) => *b,
                Value::String(s) => !s.is_empty(),
                Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
                _ => true,
            })
            .unwrap_or(false);
        if errored {
            c.voided += 1;
        }

        // tasks.setdefault(task_id, []).append(reward)
        let reward = r.get("reward").and_then(Value::as_f64).unwrap_or(0.0);
        if let Some(&idx) = c.task_index.get(&task_id) {
            c.tasks[idx].1.push(reward);
        } else {
            let idx = c.tasks.len();
            c.task_index.insert(task_id.clone(), idx);
            c.tasks.push((task_id, vec![reward]));
        }

        // walls.append(r["wall_ms"])
        let wall_ms = r.get("wall_ms").and_then(Value::as_f64).unwrap_or(0.0);
        c.walls.push(wall_ms);

        // cost handling: if r.get("cost_usd") is None ...
        let cost_usd = r.get("cost_usd");
        let cost_is_none = matches!(cost_usd, None | Some(Value::Null));
        if cost_is_none {
            // if r.get("candidate_kind") not in COSTLESS_KINDS: cost_known = False
            if !is_costless_kind(r.get("candidate_kind").and_then(Value::as_str)) {
                c.cost_known = false;
            }
        } else if let Some(v) = cost_usd {
            c.cost += v.as_f64().unwrap_or(0.0);
        }
    }

    // Build result Map in cand_order (Python insertion order).
    let mut result: Map<String, Value> = Map::new();

    for cid in &cand_order {
        let c = cands.remove(cid).unwrap();

        // Compute aggregates
        let all_rewards: Vec<f64> = c
            .tasks
            .iter()
            .flat_map(|(_, rs)| rs.iter().copied())
            .collect();
        let reward_mean = round_half_even(
            all_rewards.iter().sum::<f64>() / all_rewards.len() as f64,
            4,
        );
        let wall_mean = round_half_even(
            c.walls.iter().sum::<f64>() / c.walls.len() as f64 / 1000.0,
            1,
        );
        let cost_val: Option<f64> = if c.cost_known {
            Some(round_half_even(c.cost, 4))
        } else {
            None
        };
        let cost_per_trial: Option<f64> =
            cost_val.map(|cv| round_half_even(cv / c.trials as f64, 6));

        // Build tasks object (preserving task insertion order).
        let mut tasks_obj: Map<String, Value> = Map::new();
        for (tid, rewards) in &c.tasks {
            let arr: Vec<Value> = rewards.iter().map(|&r| Value::from(r)).collect();
            tasks_obj.insert(tid.clone(), Value::Array(arr));
        }

        // Build candidate object with Python field insertion order.
        let mut obj: Map<String, Value> = Map::new();
        obj.insert("id".into(), Value::String(c.id));
        obj.insert(
            "kind".into(),
            match c.kind {
                Some(k) => Value::String(k),
                None => Value::Null,
            },
        );
        obj.insert(
            "model".into(),
            match c.model {
                Some(m) => Value::String(m),
                None => Value::Null,
            },
        );
        obj.insert(
            "hash".into(),
            match c.hash {
                Some(h) => Value::String(h),
                None => Value::Null,
            },
        );
        obj.insert("tasks".into(), Value::Object(tasks_obj));
        obj.insert("trials".into(), Value::from(c.trials));
        obj.insert("voided".into(), Value::from(c.voided));
        obj.insert(
            "cost".into(),
            match cost_val {
                Some(cv) => Value::from(cv),
                None => Value::Null,
            },
        );
        obj.insert("cost_known".into(), Value::Bool(c.cost_known));
        obj.insert("walls".into(), {
            let arr: Vec<Value> = c.walls.iter().map(|&w| Value::from(w)).collect();
            Value::Array(arr)
        });
        obj.insert("reward_mean".into(), Value::from(reward_mean));
        obj.insert("wall_mean".into(), Value::from(wall_mean));
        obj.insert(
            "cost_per_trial".into(),
            match cost_per_trial {
                Some(cpt) => Value::from(cpt),
                None => Value::Null,
            },
        );

        result.insert(cid.clone(), Value::Object(obj));
    }

    result
}

/// Returns true if `b` dominates `a`: no worse on all three objectives, better
/// on one. Mirrors `_dominates(b, a)` in report.py.
fn dominates(b: &Value, a: &Value) -> bool {
    let cost_of = |c: &Value| -> f64 {
        // None cost → float("inf")
        match c.get("cost_per_trial") {
            Some(Value::Null) | None => f64::INFINITY,
            Some(v) => v.as_f64().unwrap_or(f64::INFINITY),
        }
    };

    let b_reward = b.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
    let a_reward = a.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
    let b_cost = cost_of(b);
    let a_cost = cost_of(a);
    let b_wall = b.get("wall_mean").and_then(Value::as_f64).unwrap_or(0.0);
    let a_wall = a.get("wall_mean").and_then(Value::as_f64).unwrap_or(0.0);

    let no_worse = b_reward >= a_reward && b_cost <= a_cost && b_wall <= a_wall;
    let better = b_reward > a_reward || b_cost < a_cost || b_wall < a_wall;
    no_worse && better
}

/// Return sorted list of Pareto-front candidate ids (non-reference,
/// non-dominated), sorted by descending reward_mean.
/// Mirrors `pareto_front(cands)` in report.py.
pub fn pareto_front(cands: &Map<String, Value>) -> Vec<String> {
    // pts = [c for c in cands.values() if c["kind"] not in REFERENCE_KINDS]
    let pts: Vec<&Value> = cands
        .values()
        .filter(|c| !is_reference_kind(c.get("kind").and_then(Value::as_str)))
        .collect();

    // a is on the front iff no other b (b is not a) dominates a
    let mut front: Vec<String> = pts
        .iter()
        .filter(|&&a| {
            let a_id = a.get("id");
            !pts.iter().any(|&b| b.get("id") != a_id && dominates(b, a))
        })
        .filter_map(|c| c.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();

    // sorted(..., key=lambda cid: -cands[cid]["reward_mean"]) — stable sort
    front.sort_by(|a_id, b_id| {
        let a_r = cands[a_id]
            .get("reward_mean")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let b_r = cands[b_id]
            .get("reward_mean")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        b_r.partial_cmp(&a_r).unwrap_or(std::cmp::Ordering::Equal)
    });

    front
}

/// Best mean reward; within 0.05 of the best, cheapest per trial wins.
/// When `eligible` is Some, only those ids may be recommended (even if they
/// are not on the overall Pareto front).
/// Mirrors `recommend(cands, front, eligible=None)` in report.py.
pub fn recommend(
    cands: &Map<String, Value>,
    front: &[String],
    eligible: Option<&HashSet<String>>,
) -> Option<String> {
    let pool: Vec<String> = match eligible {
        None => front.to_vec(),
        Some(elig) => {
            // pool = [cid for cid in sorted(eligible)
            //         if cid in cands and cands[cid]["kind"] not in REFERENCE_KINDS]
            let mut sorted_elig: Vec<&String> = elig.iter().collect();
            sorted_elig.sort(); // Python sorted() on a set uses lexicographic order
            sorted_elig
                .into_iter()
                .filter(|cid| {
                    cands
                        .get(*cid)
                        .is_some_and(|c| !is_reference_kind(c.get("kind").and_then(Value::as_str)))
                })
                .cloned()
                .collect()
        }
    };

    if pool.is_empty() {
        return None;
    }

    let best: f64 = pool
        .iter()
        .filter_map(|cid| {
            cands
                .get(cid)
                .and_then(|c| c.get("reward_mean"))
                .and_then(Value::as_f64)
        })
        .fold(f64::NEG_INFINITY, f64::max);

    // close = [cid for cid in pool if cands[cid]["reward_mean"] >= best - 0.05]
    let close: Vec<&String> = pool
        .iter()
        .filter(|cid| {
            cands
                .get(*cid)
                .and_then(|c| c.get("reward_mean"))
                .and_then(Value::as_f64)
                .is_some_and(|r| r >= best - 0.05)
        })
        .collect();

    // min(..., key=lambda cid: cost_per_trial or inf)
    close
        .into_iter()
        .min_by(|a_id, b_id| {
            let cost_of = |cid: &&String| -> f64 {
                match cands.get(*cid).and_then(|c| c.get("cost_per_trial")) {
                    Some(Value::Null) | None => f64::INFINITY,
                    Some(v) => v.as_f64().unwrap_or(f64::INFINITY),
                }
            };
            cost_of(a_id)
                .partial_cmp(&cost_of(b_id))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

/// Render a markdown comparison report. Returns the text string (ends with
/// `\n`). Mirrors `render(cands, front, pick)` in report.py.
pub fn render(cands: &Map<String, Value>, front: &[String], pick: Option<&str>) -> String {
    // tasks = sorted({t for c in cands.values() for t in c["tasks"]})
    let mut task_set: HashSet<String> = HashSet::new();
    for c in cands.values() {
        if let Some(Value::Object(tasks)) = c.get("tasks") {
            for tid in tasks.keys() {
                task_set.insert(tid.clone());
            }
        }
    }
    let mut tasks: Vec<String> = task_set.into_iter().collect();
    tasks.sort();

    // order = sorted(cands.values(), key=lambda c: -c["reward_mean"])
    let mut order: Vec<&Value> = cands.values().collect();
    order.sort_by(|a, b| {
        let ar = a.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
        let br = b.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
        br.partial_cmp(&ar).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines: Vec<String> = vec!["# Experiment comparison".to_string(), "".to_string()];

    // ## Compositions
    lines.push("## Compositions".to_string());
    lines.push("".to_string());
    lines.push("| candidate | kind | model | hash | trials | voided |".to_string());
    lines.push("|---|---|---|---|---|---|".to_string());
    for c in &order {
        let id = c.get("id").and_then(Value::as_str).unwrap_or("");
        // f"{c['kind']}" — None becomes "None" in Python f-strings
        let kind = match c.get("kind") {
            Some(Value::String(s)) => s.as_str(),
            Some(Value::Null) | None => "None",
            _ => "None",
        };
        // c['model'] or '—' — None/falsy → '—'
        let model_str = c
            .get("model")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("—");
        let hash_str = c
            .get("hash")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("—");
        let trials = c.get("trials").and_then(Value::as_u64).unwrap_or(0);
        let voided = c.get("voided").and_then(Value::as_u64).unwrap_or(0);
        lines.push(format!(
            "| {id} | {kind} | {model_str} | {hash_str} | {trials} | {voided} |"
        ));
    }

    // ## Mean reward per task
    lines.push("".to_string());
    lines.push("## Mean reward per task (n trials in parentheses)".to_string());
    lines.push("".to_string());
    lines.push(format!(
        "| candidate | {} | **overall** |",
        tasks.join(" | ")
    ));
    // Python: "|---|" + "---|" * (len(tasks) + 1)
    // Each "---|" already ends with |, so no trailing | needed.
    lines.push(format!("|---|{}", "---|".repeat(tasks.len() + 1)));
    for c in &order {
        let id = c.get("id").and_then(Value::as_str).unwrap_or("");
        let reward_mean = c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
        let mut cells: Vec<String> = Vec::new();
        for t in &tasks {
            let rs = c
                .get("tasks")
                .and_then(|ts| ts.get(t))
                .and_then(Value::as_array);
            match rs {
                Some(arr) if !arr.is_empty() => {
                    let vals: Vec<f64> = arr.iter().filter_map(Value::as_f64).collect();
                    let n = vals.len();
                    let mean = vals.iter().sum::<f64>() / n as f64;
                    cells.push(format!("{mean:.2} ({n})"));
                }
                _ => cells.push("—".to_string()),
            }
        }
        lines.push(format!(
            "| {id} | {} | **{reward_mean:.4}** |",
            cells.join(" | ")
        ));
    }

    // ## Cost and latency
    lines.push("".to_string());
    lines.push("## Cost and latency".to_string());
    lines.push("".to_string());
    lines.push("| candidate | cost/trial | total cost | mean wall/task |".to_string());
    lines.push("|---|---|---|---|".to_string());
    for c in &order {
        let id = c.get("id").and_then(Value::as_str).unwrap_or("");
        // cost = f"${c['cost']:.4f}" if c["cost"] is not None else "unknown"
        let cost_str = match c.get("cost") {
            Some(Value::Null) | None => "unknown".to_string(),
            Some(v) => format!("${:.4}", v.as_f64().unwrap_or(0.0)),
        };
        // per = f"${c['cost_per_trial']:.4f}" if c["cost_per_trial"] is not None else "unknown"
        let per_str = match c.get("cost_per_trial") {
            Some(Value::Null) | None => "unknown".to_string(),
            Some(v) => format!("${:.4}", v.as_f64().unwrap_or(0.0)),
        };
        // f"... {c['wall_mean']}s ..." — Python str(float) for a 1-decimal value
        let wall_mean = c.get("wall_mean").and_then(Value::as_f64).unwrap_or(0.0);
        lines.push(format!(
            "| {id} | {per_str} | {cost_str} | {}s |",
            py_float_str_1(wall_mean)
        ));
    }

    // ## Pareto set
    lines.push("".to_string());
    lines.push("## Pareto set (reward ↑, cost ↓, latency ↓)".to_string());
    lines.push("".to_string());
    if front.is_empty() {
        lines.push("- (no non-reference candidates)".to_string());
    } else {
        for cid in front {
            lines.push(format!("- {cid}"));
        }
    }

    // ## Recommendation
    lines.push("".to_string());
    lines.push("## Recommendation".to_string());
    lines.push("".to_string());
    match pick {
        Some(pick_id) => {
            let c = &cands[pick_id];
            let reward_mean = c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
            let wall_mean = c.get("wall_mean").and_then(Value::as_f64).unwrap_or(0.0);
            // per = f"${c['cost_per_trial']:.4f}/trial" if ... is not None else "unknown cost"
            let per_str = match c.get("cost_per_trial") {
                Some(Value::Null) | None => "unknown cost".to_string(),
                Some(v) => format!("${:.4}/trial", v.as_f64().unwrap_or(0.0)),
            };
            lines.push(format!(
                "**{pick_id}** — mean reward {reward_mean:.4} at {per_str} \
({ws}s mean wall). Within-0.05 reward ties resolve \
to the cheapest candidate per trial.",
                ws = py_float_str_1(wall_mean)
            ));
        }
        None => {
            lines.push("No certified recommendation.".to_string());
        }
    }
    lines.push("".to_string());
    if cands
        .values()
        .any(|c| c.get("kind").and_then(Value::as_str) == Some("incumbent"))
    {
        lines.push(
            "_References are excluded from Pareto and recommendation: oracle/null \
bound the verifier; the one-shot probe only detects arena saturation; the \
incumbent is the baseline-to-beat. Every recommendable candidate is an agent \
composition._"
                .to_string(),
        );
    } else {
        lines.push(
            "_References are excluded from Pareto and recommendation: oracle/null \
bound the verifier; the one-shot probe only detects arena saturation. \
Every recommendable candidate is an agent composition._"
                .to_string(),
        );
    }

    lines.join("\n") + "\n"
}

/// Render the search verdict — WHY the search stopped, and what it concluded —
/// as a `report.md` section, from the run's `loop.json`. Returns an empty string
/// when `loop.json` carries no `stop_reason`, so callers can append
/// unconditionally (mirrors `stats::delta_ci_markdown` / `consistency_markdown`).
///
/// 041: the stop reason previously lived only in `loop.json`, stdout, and the
/// HTML report — never in `report.md`. A `--plateau` early-stop in particular is
/// invisible to an operator reading the markdown. This surfaces it, plus the
/// recommended/certified summary and known spend, without grepping prose.
///
/// Spend prints only when `loop.json` carries `spend_known_usd`; an absent key
/// prints nothing rather than `$0` (AGENTS: unknown cost is null, never an
/// estimate).
pub fn verdict_markdown(loop_json: &Value) -> String {
    let stop = match loop_json.get("stop_reason").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s,
        _ => return String::new(),
    };
    // Plain-language gloss so the stop reason is self-explaining in the report.
    let gloss = match stop {
        "budget" => "the run hit its cost budget",
        "plateau" => "consecutive non-improving generations exhausted the plateau limit",
        "max-candidates" => "the search reached its candidate budget",
        "proposal-failures" => "the optimizer could not propose valid candidates",
        s if s.starts_with("arena-saturated") => {
            "the arena saturated at the top — rewards can no longer rank candidates"
        }
        _ => "see loop.json for detail",
    };

    let mut s = String::new();
    s.push_str(&format!(
        "\n## Verdict\n\nThe search stopped because **{stop}** — {gloss}.\n\n"
    ));

    let recommended = loop_json.get("recommended").and_then(Value::as_str);
    let certified: Vec<&str> = loop_json
        .get("certified")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    match recommended {
        Some(r) => {
            let cert = if certified.contains(&r) {
                " (certified)"
            } else {
                " (UNCERTIFIED — no provable win over the null floor)"
            };
            s.push_str(&format!("- **Recommended:** `{r}`{cert}\n"));
        }
        None => {
            s.push_str(
                "- **Recommended:** none — no candidate is provably better than the floor.\n",
            );
        }
    }
    s.push_str(&format!(
        "- **Certified:** {}\n",
        if certified.is_empty() {
            "none".to_string()
        } else {
            certified
                .iter()
                .map(|c| format!("`{c}`"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    // Known spend only when recorded; never fabricate a dollar figure.
    if let Some(spend) = loop_json.get("spend_known_usd").and_then(Value::as_f64) {
        s.push_str(&format!("- **Known spend:** ${spend:.4}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(
        cand: &str,
        task: &str,
        reward: f64,
        cost: Option<f64>,
        wall_ms: f64,
        error: Option<&str>,
        kind: &str,
    ) -> Value {
        json!({
            "candidate_id": cand,
            "candidate_kind": kind,
            "composition_hash": format!("hash-{cand}"),
            "model": "m",
            "task_id": task,
            "reward": reward,
            "cost_usd": cost,
            "wall_ms": wall_ms,
            "error": error,
        })
    }

    fn pi_record(cand: &str, task: &str, reward: f64, cost: f64, wall_ms: f64) -> Value {
        record(cand, task, reward, Some(cost), wall_ms, None, "pi")
    }

    #[test]
    fn pareto_excludes_dominated_candidate() {
        let records = vec![
            pi_record("good", "t1", 1.0, 0.01, 1000.0),
            pi_record("good", "t2", 0.8, 0.01, 1000.0),
            pi_record("worse", "t1", 0.5, 0.05, 5000.0),
            pi_record("worse", "t2", 0.4, 0.05, 5000.0),
            pi_record("cheap", "t1", 0.6, 0.001, 500.0),
            pi_record("cheap", "t2", 0.6, 0.001, 500.0),
        ];
        let cands = aggregate(&records);
        let front = pareto_front(&cands);
        assert!(front.contains(&"good".to_string()));
        assert!(front.contains(&"cheap".to_string()));
        assert!(!front.contains(&"worse".to_string()));
    }

    #[test]
    fn reference_candidates_never_in_front_or_pick() {
        let records = vec![
            record("oracle", "t1", 1.0, None, 1.0, None, "oracle"),
            record("null", "t1", 0.0, None, 1.0, None, "null"),
            pi_record("real", "t1", 0.7, 0.02, 2000.0),
        ];
        let cands = aggregate(&records);
        let front = pareto_front(&cands);
        assert_eq!(front, vec!["real".to_string()]);
        assert_eq!(recommend(&cands, &front, None), Some("real".to_string()));
    }

    #[test]
    fn oneshot_probe_excluded_even_when_it_wins() {
        let records = vec![
            record(
                "probe-oneshot",
                "t1",
                1.0,
                Some(0.001),
                500.0,
                None,
                "oneshot",
            ),
            pi_record("agent", "t1", 0.7, 0.02, 2000.0),
        ];
        let cands = aggregate(&records);
        let front = pareto_front(&cands);
        assert_eq!(front, vec!["agent".to_string()]);
        assert_eq!(recommend(&cands, &front, None), Some("agent".to_string()));
    }

    #[test]
    fn recommendation_breaks_near_ties_by_cost() {
        let records = vec![
            pi_record("pricey", "t1", 0.90, 0.50, 1000.0),
            pi_record("frugal", "t1", 0.88, 0.05, 1200.0),
        ];
        let cands = aggregate(&records);
        let pick = recommend(&cands, &pareto_front(&cands), None);
        assert_eq!(pick, Some("frugal".to_string()));
    }

    #[test]
    fn recommendation_prefers_clear_winner_despite_cost() {
        let records = vec![
            pi_record("strong", "t1", 0.95, 0.50, 1000.0),
            pi_record("weak", "t1", 0.60, 0.01, 500.0),
        ];
        let cands = aggregate(&records);
        let pick = recommend(&cands, &pareto_front(&cands), None);
        assert_eq!(pick, Some("strong".to_string()));
    }

    #[test]
    fn render_and_reward_mean() {
        let records = vec![
            pi_record("a", "t1", 1.0, 0.01, 1000.0),
            pi_record("a", "t1", 0.5, 0.01, 1100.0),
            record("b", "t1", 0.2, Some(0.30), 9000.0, Some("boom"), "pi"),
        ];
        let cands = aggregate(&records);
        assert_eq!(
            cands["a"].get("reward_mean").and_then(Value::as_f64),
            Some(0.75)
        );
        assert_eq!(cands["a"]["tasks"]["t1"], json!([1.0, 0.5]));
        assert_eq!(cands["b"].get("voided").and_then(Value::as_u64), Some(1));
        let front = pareto_front(&cands);
        let text = render(&cands, &front, Some("a"));
        assert!(text.contains("0.75 (2)"), "missing 0.75 (2) in: {text}");
        assert!(text.contains("**a**"), "missing **a** in: {text}");
        assert!(text.contains("hash-a"), "missing hash-a in: {text}");
    }

    #[test]
    fn extra_holdout_trials_do_not_penalize_recommendation() {
        let records = vec![
            pi_record("untested", "t1", 1.0, 0.0171, 65000.0),
            pi_record("untested", "t2", 1.0, 0.0171, 65000.0),
            pi_record("proven", "t1", 1.0, 0.0138, 61000.0),
            pi_record("proven", "t2", 1.0, 0.0138, 61000.0),
            pi_record("proven", "holdout", 1.0, 0.0138, 61000.0),
            pi_record("proven", "holdout", 1.0, 0.0138, 61000.0),
        ];
        let cands = aggregate(&records);
        let proven_cost = cands["proven"]
            .get("cost")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let untested_cost = cands["untested"]
            .get("cost")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!(proven_cost > untested_cost);
        let pick = recommend(&cands, &pareto_front(&cands), None);
        assert_eq!(pick, Some("proven".to_string()));
    }

    #[test]
    fn recommend_restricted_to_certified_candidates() {
        let mut records = vec![pi_record("lucky", "t1", 1.0, 0.001, 500.0)];
        for _ in 0..5 {
            records.push(pi_record("steady", "t1", 0.9, 0.0005, 800.0));
        }
        let cands = aggregate(&records);
        let front = pareto_front(&cands);
        assert!(front.contains(&"lucky".to_string()));
        assert_eq!(recommend(&cands, &front, None), Some("lucky".to_string()));
        let eligible_steady: HashSet<String> = ["steady".to_string()].into_iter().collect();
        assert_eq!(
            recommend(&cands, &front, Some(&eligible_steady)),
            Some("steady".to_string())
        );
        let eligible_ghost: HashSet<String> = ["ghost".to_string()].into_iter().collect();
        assert_eq!(recommend(&cands, &front, Some(&eligible_ghost)), None);
    }

    #[test]
    fn unknown_cost_treated_as_worst_in_dominance() {
        let records = vec![
            pi_record("known", "t1", 0.8, 0.01, 1000.0),
            record("mystery", "t1", 0.8, None, 1000.0, None, "pi"),
        ];
        let cands = aggregate(&records);
        let front = pareto_front(&cands);
        assert!(front.contains(&"known".to_string()));
        assert!(!front.contains(&"mystery".to_string()));
    }

    #[test]
    fn verdict_markdown_surfaces_stop_reason_and_recommendation() {
        // Fixture mirrors loop.json: a plateau stop with a recommended +
        // certified pick. The operator must read WHY the search stopped and what
        // it concluded without grepping prose (041 / ticket oracle).
        let loop_json = json!({
            "stop_reason": "plateau",
            "mode": "threshold-then-cheap",
            "recommended": "seed1-glm-5-spec-first",
            "certified": ["seed1-glm-5-spec-first", "g1b-seed1-glm-5-spec-first"],
            "spend_known_usd": 3.027,
        });
        let md = verdict_markdown(&loop_json);
        assert!(md.starts_with("\n## Verdict\n"), "section header: {md}");
        assert!(md.contains("plateau"), "stop reason missing: {md}");
        assert!(md.contains("non-improving"), "plateau gloss missing: {md}");
        assert!(
            md.contains("seed1-glm-5-spec-first"),
            "recommendation missing: {md}"
        );
        assert!(md.contains("$3.0270"), "spend missing: {md}");
    }

    #[test]
    fn verdict_markdown_honest_when_nothing_certified() {
        let loop_json = json!({
            "stop_reason": "max-candidates",
            "recommended": Value::Null,
            "certified": [],
        });
        let md = verdict_markdown(&loop_json);
        assert!(md.contains("max-candidates"), "stop reason: {md}");
        assert!(
            md.contains("provably better than the floor"),
            "uncertified honesty missing: {md}"
        );
        // No spend key → no fabricated dollar figure (AGENTS: unknown cost is null).
        assert!(!md.contains('$'), "fabricated spend: {md}");
    }

    #[test]
    fn verdict_markdown_empty_without_stop_reason() {
        // No verdict to render (e.g. a malformed/absent loop.json) → empty string
        // so the caller can append unconditionally, mirroring stats::*_markdown.
        assert_eq!(verdict_markdown(&json!({})), "");
    }
}
