//! Search-loop core v2: seed archive → race competing hypotheses → keep what
//! clears the noise → stop.
//!
//! Port of `runner/loop.py` (named `search_loop` because `loop` is a Rust
//! keyword). Pure orchestration with an injected [`SearchWorld`] so the policy
//! is testable offline; the CLI wires in the real runner subprocess and the LLM
//! mutation step. Parent shuffling uses [`crate::pyrandom::PyRandom`] so the
//! default `random.Random(0)` trajectory matches CPython exactly.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::{json, Map, Value};

use crate::pycompat::round_half_even;
use crate::pyrandom::PyRandom;

const REFERENCE_IDS: &[&str] = &["null", "oracle"];
const REFERENCE_KINDS: &[&str] = &["null", "oracle", "oneshot"];
const COST_SENSITIVE_MODES: &[&str] = &["threshold-then-cheap", "pareto"];
const LATENCY_SENSITIVE_MODES: &[&str] = &["fast-enough", "pareto"];

/// The injected boundary: the real world runs trials and proposes mutations;
/// tests script it. `run_search` calls these sequentially (never re-entrant).
pub trait SearchWorld {
    /// Current `{candidate_id: stats}` summary (a fresh copy per call).
    fn summary(&mut self) -> Map<String, Value>;
    /// Propose a child of `parent`; `Ok((child_id, meta))` or `Err(message)`
    /// (a bad proposal is data, recorded as `proposal_error`).
    fn propose(
        &mut self,
        parent: &str,
        generation: u64,
        attempt: usize,
        avoid_slots: &[String],
    ) -> Result<(String, Value), String>;
    /// Run the proposed child's trials (they land in the summary).
    fn run_child(&mut self, child_id: &str);
    /// Meta-eval monitor; an alarm carrying `"stop"` halts the search. Default:
    /// no monitor (equivalent to Python `monitor_fn=None`).
    fn monitor(&mut self, _summary: &Map<String, Value>, _generation: u64) -> Vec<Value> {
        Vec::new()
    }
    /// Persist one history entry the instant it is produced — the incremental
    /// hypothesis log. Default: a no-op (offline policy and tests don't need the
    /// stream). The CLI world appends it to `loop.history.jsonl` so `daedalus
    /// view` can tail the optimizer's hypotheses live, instead of them only
    /// appearing in `loop.json` at completion. The `history` that [`run_search`]
    /// returns is unchanged — this is a pure addition.
    fn record_history(&mut self, _entry: &Value) {}
}

/// Tunables for [`run_search`] (Python keyword args, same defaults).
pub struct SearchParams {
    pub max_children: usize,
    pub budget_usd: Option<f64>,
    pub optimizer_costs: Vec<Option<f64>>,
    pub plateau_limit: usize,
    pub max_proposal_failures: usize,
    pub children_per_generation: usize,
    pub mode: String,
}

impl Default for SearchParams {
    fn default() -> Self {
        SearchParams {
            max_children: 10,
            budget_usd: None,
            optimizer_costs: Vec::new(),
            plateau_limit: 2,
            max_proposal_failures: 2,
            children_per_generation: 2,
            mode: "max-quality".to_string(),
        }
    }
}

fn reward_mean(stats: &Value) -> f64 {
    stats
        .get("reward_mean")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

/// `cost_usd_total / trials`, or `None` if either is missing/falsy.
pub fn cost_per_trial(stats: &Value) -> Option<f64> {
    let cost = match stats.get("cost_usd_total") {
        None | Some(Value::Null) => return None,
        Some(c) => c.as_f64()?,
    };
    let trials = stats.get("trials").and_then(Value::as_f64).unwrap_or(0.0);
    if trials == 0.0 {
        return None;
    }
    Some(cost / trials)
}

/// Mean wall-clock across every trial of every task, or `None` if none recorded.
pub fn wall_mean_ms(stats: &Value) -> Option<f64> {
    let mut walls: Vec<f64> = Vec::new();
    if let Some(tasks) = stats.get("tasks").and_then(Value::as_object) {
        for t in tasks.values() {
            if let Some(ws) = t.get("wall_ms").and_then(Value::as_array) {
                walls.extend(ws.iter().filter_map(Value::as_f64));
            }
        }
    }
    if walls.is_empty() {
        None
    } else {
        Some(walls.iter().sum::<f64>() / walls.len() as f64)
    }
}

/// References (floor, ceiling, saturation probe) never compete.
pub fn is_reference(cid: &str, stats: &Value) -> bool {
    REFERENCE_IDS.contains(&cid)
        || stats
            .get("kind")
            .and_then(Value::as_str)
            .map(|k| REFERENCE_KINDS.contains(&k))
            .unwrap_or(false)
}

fn real_candidates(summary: &Map<String, Value>) -> Vec<(&String, &Value)> {
    summary
        .iter()
        .filter(|(cid, v)| !is_reference(cid, v))
        .collect()
}

/// Highest mean reward among non-reference candidates; ties go to the cheapest
/// (unknown cost ranks worst). First candidate wins an exact tie (Python `max`).
pub fn best_candidate(summary: &Map<String, Value>) -> String {
    let mut best: Option<(&String, (f64, f64))> = None;
    let mut any = false;
    for (cid, v) in summary {
        if is_reference(cid, v) {
            continue;
        }
        any = true;
        let neg_cost = -v
            .get("cost_usd_total")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        let key = (reward_mean(v), neg_cost);
        let better = match &best {
            None => true,
            Some((_, bk)) => key.0 > bk.0 || (key.0 == bk.0 && key.1 > bk.1),
        };
        if better {
            best = Some((cid, key));
        }
    }
    assert!(any, "no non-reference candidates in summary");
    best.unwrap().0.clone()
}

/// Archive-eligible parents: best-on-mean, every per-task winner, and the
/// cheapest within a near-tie of the best reward. Returned sorted.
pub fn parent_pool(summary: &Map<String, Value>) -> Vec<String> {
    let real = real_candidates(summary);
    assert!(!real.is_empty(), "no non-reference candidates in summary");
    let real_map: HashMap<&str, &Value> = real.iter().map(|(k, v)| (k.as_str(), *v)).collect();

    let mut pool: BTreeSet<String> = BTreeSet::new();
    pool.insert(best_candidate(summary));

    // every per-task winner (contenders sorted; first wins an exact tie)
    let mut tasks: BTreeSet<String> = BTreeSet::new();
    for (_, v) in &real {
        if let Some(t) = v.get("tasks").and_then(Value::as_object) {
            tasks.extend(t.keys().cloned());
        }
    }
    for task in &tasks {
        let mut contenders: Vec<&str> = real
            .iter()
            .filter(|(_, v)| {
                v.get("tasks")
                    .and_then(Value::as_object)
                    .map(|t| t.contains_key(task))
                    .unwrap_or(false)
            })
            .map(|(c, _)| c.as_str())
            .collect();
        contenders.sort();
        if let Some(winner) = contenders.iter().copied().reduce(|best, c| {
            let m = |id: &str| {
                real_map[id]["tasks"][task]
                    .get("mean")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            };
            if m(c) > m(best) {
                c
            } else {
                best
            }
        }) {
            pool.insert(winner.to_string());
        }
    }

    // cost frontier: cheapest within 0.05 of the best reward
    let best_r = reward_mean(real_map[best_candidate(summary).as_str()]);
    let mut near: Vec<&str> = real
        .iter()
        .map(|(c, _)| c.as_str())
        .filter(|c| {
            reward_mean(real_map[c]) >= best_r - 0.05 && cost_per_trial(real_map[c]).is_some()
        })
        .collect();
    near.sort();
    if let Some(cheapest) = near.iter().copied().reduce(|best, c| {
        let cpt = |id: &str| cost_per_trial(real_map[id]).unwrap_or(f64::INFINITY);
        if cpt(c) < cpt(best) {
            c
        } else {
            best
        }
    }) {
        pool.insert(cheapest.to_string());
    }

    pool.into_iter().collect()
}

/// Half the mean within-task reward range: the observable trial-noise radius.
pub fn trial_noise(stats: &Value) -> f64 {
    let mut spreads: Vec<f64> = Vec::new();
    if let Some(tasks) = stats.get("tasks").and_then(Value::as_object) {
        for t in tasks.values() {
            let n = t
                .get("rewards")
                .and_then(Value::as_array)
                .map(|a| a.len())
                .unwrap_or(0);
            if n >= 2 {
                let max = t.get("max").and_then(Value::as_f64).unwrap_or(0.0);
                let min = t.get("min").and_then(Value::as_f64).unwrap_or(0.0);
                spreads.push(max - min);
            }
        }
    }
    if spreads.is_empty() {
        0.0
    } else {
        spreads.iter().sum::<f64>() / spreads.len() as f64 / 2.0
    }
}

/// Variance- and mode-aware keep rule. Returns `(improved, rounded_mean_delta)`.
pub fn improved_over(child: &Value, parent: &Value, mode: &str, epsilon: f64) -> (bool, f64) {
    let child_tasks: BTreeSet<String> = child
        .get("tasks")
        .and_then(Value::as_object)
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default();
    let parent_tasks: BTreeSet<String> = parent
        .get("tasks")
        .and_then(Value::as_object)
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default();
    let common: Vec<&String> = child_tasks.intersection(&parent_tasks).collect();
    if common.is_empty() {
        return (false, 0.0);
    }
    let task_mean = |v: &Value, t: &str| {
        v["tasks"][t]
            .get("mean")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    };
    let deltas: Vec<f64> = common
        .iter()
        .map(|t| task_mean(child, t) - task_mean(parent, t))
        .collect();
    let mean_delta = deltas.iter().sum::<f64>() / deltas.len() as f64;
    let rounded = round_half_even(mean_delta, 4);
    let band = trial_noise(child).max(trial_noise(parent)).max(epsilon);
    if mean_delta > band {
        return (true, rounded);
    }
    if mean_delta >= -band {
        if COST_SENSITIVE_MODES.contains(&mode) {
            if let (Some(cc), Some(pc)) = (cost_per_trial(child), cost_per_trial(parent)) {
                if cc <= pc * 0.9 {
                    return (true, rounded);
                }
            }
        }
        if LATENCY_SENSITIVE_MODES.contains(&mode) {
            if let (Some(cw), Some(pw)) = (wall_mean_ms(child), wall_mean_ms(parent)) {
                if cw <= pw * 0.9 {
                    return (true, rounded);
                }
            }
        }
    }
    (false, rounded)
}

/// Total known spend: summed trial cost plus optimizer-call costs.
pub fn known_spend(summary: &Map<String, Value>, optimizer_costs: &[Option<f64>]) -> f64 {
    let trial: f64 = summary
        .values()
        .map(|v| {
            v.get("cost_usd_total")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        })
        .sum();
    let opt: f64 = optimizer_costs.iter().flatten().sum();
    trial + opt
}

/// Drive the search. Returns `{stop_reason, mode, history, alarms, generations,
/// best_id, spend_known_usd}` (Python dict key order preserved).
pub fn run_search(world: &mut dyn SearchWorld, params: &SearchParams, rng: &mut PyRandom) -> Value {
    let mut history: Vec<Value> = Vec::new();
    let mut alarms: Vec<Value> = Vec::new();
    let mut seen_alarms: HashSet<(Option<String>, Option<String>)> = HashSet::new();
    let mut non_improving_gens = 0usize;
    let mut proposal_failures = 0usize;
    let mut children_built = 0usize;
    let mut generation: u64 = 0;
    let mut stop_reason = "max-candidates".to_string();
    let mut stopped = false;

    while children_built < params.max_children && !stopped {
        generation += 1;
        let summary = world.summary();
        let spent = known_spend(&summary, &params.optimizer_costs);
        if let Some(budget) = params.budget_usd {
            if spent >= budget {
                stop_reason = "budget".to_string();
                break;
            }
        }

        for alarm in world.monitor(&summary, generation) {
            let key = (
                alarm
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                alarm
                    .get("detail")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            );
            if !seen_alarms.contains(&key) {
                seen_alarms.insert(key);
                alarms.push(alarm.clone());
            }
            if alarm
                .get("stop")
                .map(crate::pycompat::is_truthy)
                .unwrap_or(false)
            {
                stop_reason = alarm["stop"].as_str().unwrap_or_default().to_string();
                stopped = true;
            }
        }
        if stopped {
            break;
        }

        let mut parents = parent_pool(&summary);
        rng.shuffle(&mut parents);
        let k = params
            .children_per_generation
            .min(params.max_children - children_built);
        let mut proposed_slots: HashMap<String, Vec<Value>> = HashMap::new();
        let mut ran_any = false;
        let mut improved_any = false;

        for attempt in 0..k {
            let parent = parents[attempt % parents.len()].clone();
            let avoid: Vec<String> = proposed_slots
                .get(&parent)
                .map(|slots| {
                    slots
                        .iter()
                        .filter(|s| crate::pycompat::is_truthy(s))
                        .map(|s| s.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default();

            match world.propose(&parent, generation, attempt, &avoid) {
                Err(exc) => {
                    proposal_failures += 1;
                    let mut h = Map::new();
                    h.insert("generation".to_string(), json!(generation));
                    h.insert("attempt".to_string(), json!(attempt));
                    h.insert("parent_id".to_string(), json!(parent));
                    h.insert("proposal_error".to_string(), json!(exc));
                    let entry = Value::Object(h);
                    world.record_history(&entry);
                    history.push(entry);
                    if proposal_failures >= params.max_proposal_failures {
                        stop_reason = "proposal-failures".to_string();
                        stopped = true;
                        break;
                    }
                    continue;
                }
                Ok((child_id, meta)) => {
                    let slot = meta.get("slot_changed").cloned().unwrap_or(Value::Null);
                    proposed_slots.entry(parent.clone()).or_default().push(slot);
                    world.run_child(&child_id);
                    children_built += 1;
                    let after = world.summary();
                    let child_stats = after.get(&child_id).cloned().unwrap_or(Value::Null);
                    let parent_stats = after.get(&parent).cloned().unwrap_or(Value::Null);
                    let (improved, delta) =
                        improved_over(&child_stats, &parent_stats, &params.mode, 0.01);
                    let ccpt = cost_per_trial(&child_stats);
                    let pcpt = cost_per_trial(&parent_stats);

                    let mut h = meta.as_object().cloned().unwrap_or_default();
                    h.insert("generation".to_string(), json!(generation));
                    h.insert("attempt".to_string(), json!(attempt));
                    h.insert("parent_id".to_string(), json!(parent));
                    h.insert(
                        "parent_reward_mean".to_string(),
                        parent_stats
                            .get("reward_mean")
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                    h.insert(
                        "reward_mean".to_string(),
                        child_stats
                            .get("reward_mean")
                            .cloned()
                            .unwrap_or(Value::Null),
                    );
                    h.insert("mean_task_delta".to_string(), json!(delta));
                    h.insert(
                        "parent_cost_per_trial".to_string(),
                        pcpt.map(|x| json!(round_half_even(x, 6)))
                            .unwrap_or(Value::Null),
                    );
                    h.insert(
                        "child_cost_per_trial".to_string(),
                        ccpt.map(|x| json!(round_half_even(x, 6)))
                            .unwrap_or(Value::Null),
                    );
                    h.insert("improved".to_string(), json!(improved));
                    let entry = Value::Object(h);
                    world.record_history(&entry);
                    history.push(entry);
                    ran_any = true;
                    improved_any = improved_any || improved;
                }
            }
        }

        if stopped {
            break;
        }
        if ran_any {
            non_improving_gens = if improved_any {
                0
            } else {
                non_improving_gens + 1
            };
            if non_improving_gens >= params.plateau_limit {
                stop_reason = "plateau".to_string();
                break;
            }
        }
    }

    let final_summary = world.summary();
    json!({
        "stop_reason": stop_reason,
        "mode": params.mode,
        "history": history,
        "alarms": alarms,
        "generations": generation,
        "best_id": best_candidate(&final_summary),
        "spend_known_usd": round_half_even(known_spend(&final_summary, &params.optimizer_costs), 4),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a summary entry from `{task: [rewards]}` (mirrors test_loop.stats).
    fn stats(task_rewards: &[(&str, &[f64])], cost: f64, kind: &str, wall_ms: f64) -> Value {
        let mut tasks = Map::new();
        let mut flat: Vec<f64> = Vec::new();
        for (t, rs) in task_rewards {
            let mean = rs.iter().sum::<f64>() / rs.len() as f64;
            let min = rs.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = rs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            tasks.insert(
                t.to_string(),
                json!({
                    "rewards": rs, "mean": mean, "min": min, "max": max,
                    "wall_ms": vec![wall_ms; rs.len()],
                }),
            );
            flat.extend_from_slice(rs);
        }
        json!({
            "kind": kind,
            "tasks": Value::Object(tasks),
            "trials": flat.len(),
            "reward_mean": round_half_even(flat.iter().sum::<f64>() / flat.len() as f64, 4),
            "cost_usd_total": cost,
        })
    }

    fn s(task_rewards: &[(&str, &[f64])], cost: f64) -> Value {
        stats(task_rewards, cost, "pi", 1000.0)
    }

    fn base() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(
            "oracle".into(),
            stats(&[("t1", &[1.0]), ("t2", &[1.0])], 0.0, "oracle", 1000.0),
        );
        m.insert(
            "null".into(),
            stats(&[("t1", &[0.0]), ("t2", &[0.5])], 0.0, "null", 1000.0),
        );
        m.insert(
            "probe-oneshot".into(),
            stats(&[("t1", &[1.0]), ("t2", &[1.0])], 0.001, "oneshot", 1000.0),
        );
        m.insert(
            "base".into(),
            s(&[("t1", &[0.6, 0.6]), ("t2", &[0.5, 0.5])], 0.05),
        );
        m
    }

    enum Item {
        Child(&'static str, Vec<(&'static str, Vec<f64>)>, f64),
        Fail,
    }

    struct FakeWorld {
        summary: Map<String, Value>,
        script: Vec<Item>,
        pending: Map<String, Value>,
        avoid_seen: Vec<(String, Vec<String>)>,
    }

    impl FakeWorld {
        fn new(candidates: Map<String, Value>, script: Vec<Item>) -> Self {
            FakeWorld {
                summary: candidates,
                script,
                pending: Map::new(),
                avoid_seen: Vec::new(),
            }
        }
    }

    impl SearchWorld for FakeWorld {
        fn summary(&mut self) -> Map<String, Value> {
            self.summary.clone()
        }
        fn propose(
            &mut self,
            parent: &str,
            _gen: u64,
            _attempt: usize,
            avoid: &[String],
        ) -> Result<(String, Value), String> {
            self.avoid_seen.push((parent.to_string(), avoid.to_vec()));
            if self.script.is_empty() {
                return Err("script exhausted".to_string());
            }
            match self.script.remove(0) {
                Item::Fail => Err("optimizer returned garbage".to_string()),
                Item::Child(id, rewards, cost) => {
                    let rs: Vec<(&str, &[f64])> =
                        rewards.iter().map(|(t, r)| (*t, r.as_slice())).collect();
                    self.pending.insert(id.to_string(), s(&rs, cost));
                    Ok((
                        id.to_string(),
                        json!({"child_id": id, "slot_changed": "prompt_packet", "hypothesis": "h"}),
                    ))
                }
            }
        }
        fn run_child(&mut self, child_id: &str) {
            let v = self.pending.remove(child_id).unwrap();
            self.summary.insert(child_id.to_string(), v);
        }
    }

    /// Monitor-injecting world wrapper for the two monitor tests.
    struct MonitoredWorld {
        inner: FakeWorld,
        alarm: Value,
        calls: Vec<u64>,
    }
    impl SearchWorld for MonitoredWorld {
        fn summary(&mut self) -> Map<String, Value> {
            self.inner.summary()
        }
        fn propose(
            &mut self,
            p: &str,
            g: u64,
            a: usize,
            av: &[String],
        ) -> Result<(String, Value), String> {
            self.inner.propose(p, g, a, av)
        }
        fn run_child(&mut self, c: &str) {
            self.inner.run_child(c)
        }
        fn monitor(&mut self, _summary: &Map<String, Value>, generation: u64) -> Vec<Value> {
            self.calls.push(generation);
            vec![self.alarm.clone()]
        }
    }

    /// Wraps a `FakeWorld` and captures every `record_history` call, to prove the
    /// streamed incremental log equals the `history` `run_search` returns.
    struct RecordingWorld {
        inner: FakeWorld,
        recorded: Vec<Value>,
    }
    impl SearchWorld for RecordingWorld {
        fn summary(&mut self) -> Map<String, Value> {
            self.inner.summary()
        }
        fn propose(
            &mut self,
            p: &str,
            g: u64,
            a: usize,
            av: &[String],
        ) -> Result<(String, Value), String> {
            self.inner.propose(p, g, a, av)
        }
        fn run_child(&mut self, c: &str) {
            self.inner.run_child(c)
        }
        fn record_history(&mut self, entry: &Value) {
            self.recorded.push(entry.clone());
        }
    }

    fn search(world: &mut dyn SearchWorld, params: SearchParams) -> Value {
        run_search(world, &params, &mut PyRandom::new(0))
    }

    fn defaults() -> SearchParams {
        SearchParams {
            budget_usd: Some(100.0),
            ..SearchParams::default()
        }
    }

    fn hist_len(out: &Value) -> usize {
        out["history"].as_array().unwrap().len()
    }

    #[test]
    fn plateau_stops_after_non_improving_generations() {
        let mut w = FakeWorld::new(
            base(),
            vec![
                Item::Child("c1", vec![("t1", vec![0.58]), ("t2", vec![0.5])], 0.01),
                Item::Child("c2", vec![("t1", vec![0.55]), ("t2", vec![0.5])], 0.01),
                Item::Child("c3", vec![("t1", vec![0.59]), ("t2", vec![0.5])], 0.01),
                Item::Child("c4", vec![("t1", vec![0.3]), ("t2", vec![0.2])], 0.01),
            ],
        );
        let out = search(&mut w, defaults());
        assert_eq!(out["stop_reason"], "plateau");
        assert_eq!(hist_len(&out), 4);
        assert_eq!(out["generations"], 2);
        assert_eq!(out["best_id"], "base");
    }

    #[test]
    fn record_history_streams_every_entry_in_order() {
        // The incremental log (what `daedalus view` tails) must equal the
        // `history` `run_search` returns — same entries, same order. A `Fail`
        // proves proposal-error entries stream too, not just successful children.
        let inner = FakeWorld::new(
            base(),
            vec![
                Item::Child("c1", vec![("t1", vec![0.58]), ("t2", vec![0.5])], 0.01),
                Item::Fail,
                Item::Child("c3", vec![("t1", vec![0.59]), ("t2", vec![0.5])], 0.01),
                Item::Child("c4", vec![("t1", vec![0.3]), ("t2", vec![0.2])], 0.01),
            ],
        );
        let mut w = RecordingWorld {
            inner,
            recorded: Vec::new(),
        };
        let out = search(&mut w, defaults());
        let history = out["history"].as_array().unwrap();
        assert!(!w.recorded.is_empty(), "entries were streamed");
        assert_eq!(
            &w.recorded, history,
            "the streamed log equals the returned history exactly"
        );
        assert!(
            w.recorded.iter().any(|e| e.get("proposal_error").is_some()),
            "proposal-error entries stream too"
        );
    }

    #[test]
    fn clear_improvement_resets_plateau_and_wins() {
        let mut w = FakeWorld::new(
            base(),
            vec![
                Item::Child(
                    "c1",
                    vec![("t1", vec![0.9, 0.9]), ("t2", vec![0.9, 0.9])],
                    0.01,
                ),
                Item::Child("c2", vec![("t1", vec![0.5]), ("t2", vec![0.4])], 0.01),
                Item::Child("c3", vec![("t1", vec![0.9]), ("t2", vec![0.9])], 0.01),
                Item::Child("c4", vec![("t1", vec![0.85]), ("t2", vec![0.85])], 0.01),
                Item::Child("c5", vec![("t1", vec![0.9]), ("t2", vec![0.88])], 0.01),
            ],
        );
        let out = search(&mut w, defaults());
        assert_eq!(out["stop_reason"], "plateau");
        assert_eq!(out["best_id"], "c1");
        let gen1: Vec<&Value> = out["history"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|h| h["generation"] == 1)
            .collect();
        assert_eq!(gen1[0]["improved"], true);
    }

    #[test]
    fn improvement_inside_noise_band_does_not_count() {
        let mut candidates = base();
        candidates.insert(
            "base".into(),
            s(&[("t1", &[0.8, 0.4]), ("t2", &[0.7, 0.3])], 0.05),
        );
        let mut w = FakeWorld::new(
            candidates,
            vec![
                Item::Child("c1", vec![("t1", vec![0.7]), ("t2", vec![0.6])], 0.01),
                Item::Child("c2", vec![("t1", vec![0.65]), ("t2", vec![0.6])], 0.01),
                Item::Child("c3", vec![("t1", vec![0.7]), ("t2", vec![0.6])], 0.01),
                Item::Child("c4", vec![("t1", vec![0.6]), ("t2", vec![0.6])], 0.01),
            ],
        );
        let out = search(&mut w, defaults());
        assert_eq!(out["stop_reason"], "plateau");
        assert!(!out["history"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["improved"] == true));
    }

    #[test]
    fn improved_over_clears_noise_threshold_directly() {
        let parent = s(&[("t1", &[0.8, 0.4]), ("t2", &[0.7, 0.3])], 0.05);
        let inside = s(&[("t1", &[0.7]), ("t2", &[0.6])], 0.05);
        let beyond = s(&[("t1", &[0.95]), ("t2", &[0.9])], 0.05);
        let (ok, delta) = improved_over(&inside, &parent, "max-quality", 0.01);
        assert!(!ok && (delta - 0.1).abs() < 1e-9);
        let (ok, delta) = improved_over(&beyond, &parent, "max-quality", 0.01);
        assert!(ok && delta > 0.2);
    }

    #[test]
    fn parent_pool_includes_per_task_specialist() {
        let mut summary = base();
        summary.insert(
            "generalist".into(),
            s(&[("t1", &[0.8]), ("t2", &[0.8])], 0.05),
        );
        summary.insert(
            "specialist".into(),
            s(&[("t1", &[0.2]), ("t2", &[0.95])], 0.05),
        );
        let pool = parent_pool(&summary);
        assert!(pool.contains(&"generalist".to_string()));
        assert!(pool.contains(&"specialist".to_string()));
        assert!(!pool.contains(&"probe-oneshot".to_string()));
        assert!(!pool.contains(&"oracle".to_string()));
    }

    #[test]
    fn competing_hypotheses_avoid_each_others_slot() {
        let mut w = FakeWorld::new(
            base(),
            vec![
                Item::Child("c1", vec![("t1", vec![0.6]), ("t2", vec![0.5])], 0.01),
                Item::Child("c2", vec![("t1", vec![0.6]), ("t2", vec![0.5])], 0.01),
            ],
        );
        let params = SearchParams {
            max_children: 2,
            ..defaults()
        };
        run_search(&mut w, &params, &mut PyRandom::new(0));
        assert_eq!(w.avoid_seen[0], ("base".to_string(), vec![]));
        assert_eq!(
            w.avoid_seen[1],
            ("base".to_string(), vec!["prompt_packet".to_string()])
        );
    }

    #[test]
    fn budget_stop() {
        let mut w = FakeWorld::new(
            base(),
            vec![
                Item::Child("c1", vec![("t1", vec![0.9]), ("t2", vec![0.9])], 3.0),
                Item::Child("c2", vec![("t1", vec![0.95]), ("t2", vec![0.95])], 3.0),
            ],
        );
        let params = SearchParams {
            budget_usd: Some(3.0),
            children_per_generation: 1,
            ..defaults()
        };
        let out = search(&mut w, params);
        assert_eq!(out["stop_reason"], "budget");
        assert_eq!(hist_len(&out), 1);
    }

    #[test]
    fn max_candidates_stop() {
        let script: Vec<Item> = (1..4)
            .map(|i| {
                Item::Child(
                    Box::leak(format!("c{i}").into_boxed_str()),
                    vec![("t1", vec![0.9 + i as f64 * 0.01]), ("t2", vec![0.9])],
                    0.01,
                )
            })
            .collect();
        let mut w = FakeWorld::new(base(), script);
        let params = SearchParams {
            max_children: 3,
            plateau_limit: 99,
            ..defaults()
        };
        let out = search(&mut w, params);
        assert_eq!(out["stop_reason"], "max-candidates");
        assert_eq!(hist_len(&out), 3);
    }

    #[test]
    fn proposal_failures_stop() {
        let mut w = FakeWorld::new(base(), vec![Item::Fail, Item::Fail]);
        let out = search(&mut w, defaults());
        assert_eq!(out["stop_reason"], "proposal-failures");
        assert!(out["history"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h.get("proposal_error").is_some()));
    }

    #[test]
    fn best_candidate_ignores_references_and_breaks_ties_by_cost() {
        let summary: Map<String, Value> = serde_json::from_value(json!({
            "oracle": {"reward_mean": 1.0, "cost_usd_total": 0.0},
            "a": {"reward_mean": 0.8, "cost_usd_total": 0.50},
            "b": {"reward_mean": 0.8, "cost_usd_total": 0.10},
            "c": {"reward_mean": 0.8, "cost_usd_total": null},
        }))
        .unwrap();
        assert_eq!(best_candidate(&summary), "b");
    }

    #[test]
    fn best_candidate_ignores_oneshot_probe_by_kind() {
        let summary: Map<String, Value> = serde_json::from_value(json!({
            "probe-oneshot": {"reward_mean": 1.0, "cost_usd_total": 0.001, "kind": "oneshot"},
            "agent": {"reward_mean": 0.6, "cost_usd_total": 0.50, "kind": "pi"},
        }))
        .unwrap();
        assert_eq!(best_candidate(&summary), "agent");
    }

    #[test]
    fn held_reward_with_cost_cut_improves_under_cheap_modes() {
        let parent = s(&[("t1", &[1.0, 1.0]), ("t2", &[1.0, 1.0])], 0.40);
        let child = s(&[("t1", &[1.0, 1.0]), ("t2", &[1.0, 1.0])], 0.24);
        let (ok, delta) = improved_over(&child, &parent, "threshold-then-cheap", 0.01);
        assert!(ok && delta == 0.0);
        let (ok, _) = improved_over(&child, &parent, "max-quality", 0.01);
        assert!(!ok);
        let wiggle = s(&[("t1", &[1.0, 1.0]), ("t2", &[1.0, 1.0])], 0.39);
        let (ok, _) = improved_over(&wiggle, &parent, "threshold-then-cheap", 0.01);
        assert!(!ok);
    }

    #[test]
    fn held_reward_with_wall_cut_improves_under_fast_enough() {
        let parent = stats(&[("t1", &[0.8, 0.8])], 0.05, "pi", 10000.0);
        let child = stats(&[("t1", &[0.8, 0.8])], 0.05, "pi", 6000.0);
        let (ok, _) = improved_over(&child, &parent, "fast-enough", 0.01);
        assert!(ok);
        let (ok, _) = improved_over(&child, &parent, "threshold-then-cheap", 0.01);
        assert!(!ok);
    }

    #[test]
    fn reward_drop_is_never_excused_by_cheapness() {
        let parent = s(&[("t1", &[1.0, 1.0]), ("t2", &[1.0, 1.0])], 0.40);
        let worse = s(&[("t1", &[0.5, 0.5]), ("t2", &[0.5, 0.5])], 0.04);
        let (ok, _) = improved_over(&worse, &parent, "threshold-then-cheap", 0.01);
        assert!(!ok);
    }

    #[test]
    fn parent_pool_includes_cost_frontier_near_tie() {
        let mut summary = base();
        summary.insert("champ".into(), s(&[("t1", &[1.0]), ("t2", &[1.0])], 0.50));
        summary.insert(
            "frugal".into(),
            s(&[("t1", &[0.95]), ("t2", &[0.96])], 0.04),
        );
        let pool = parent_pool(&summary);
        assert!(pool.contains(&"champ".to_string()));
        assert!(pool.contains(&"frugal".to_string()));
    }

    #[test]
    fn monitor_alarms_recorded_deduped_and_can_stop() {
        let inner = FakeWorld::new(
            base(),
            vec![Item::Child(
                "c1",
                vec![("t1", vec![0.7]), ("t2", vec![0.7])],
                0.01,
            )],
        );
        let mut w = MonitoredWorld {
            inner,
            alarm: json!({"kind": "saturation-at-top", "detail": "best at ceiling", "stop": "arena-saturated-at-top"}),
            calls: Vec::new(),
        };
        let out = search(&mut w, defaults());
        assert_eq!(out["stop_reason"], "arena-saturated-at-top");
        assert_eq!(out["history"].as_array().unwrap().len(), 0);
        assert_eq!(out["alarms"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn monitor_note_alarms_do_not_stop_and_do_not_repeat() {
        let inner = FakeWorld::new(
            base(),
            vec![
                Item::Child("c1", vec![("t1", vec![0.58]), ("t2", vec![0.5])], 0.01),
                Item::Child("c2", vec![("t1", vec![0.55]), ("t2", vec![0.5])], 0.01),
                Item::Child("c3", vec![("t1", vec![0.59]), ("t2", vec![0.5])], 0.01),
                Item::Child("c4", vec![("t1", vec![0.3]), ("t2", vec![0.2])], 0.01),
            ],
        );
        let mut w = MonitoredWorld {
            inner,
            alarm: json!({"kind": "variance", "detail": "x flip-flops on t1"}),
            calls: Vec::new(),
        };
        let out = search(&mut w, defaults());
        assert_eq!(out["stop_reason"], "plateau");
        assert_eq!(out["alarms"].as_array().unwrap().len(), 1);
    }
}
