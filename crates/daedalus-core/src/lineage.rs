//! Render a run's experiment lineage: the traceable story of how the final
//! agent contract was discovered — rig, sampled landscape, every hypothesis
//! with its measured outcome and decision, alarms, certification, outcome.
//!
//! Port of `runner/lineage.py`. A pure function of the run's own artifacts
//! (rig.json, seed.json, loop.json, pareto.json, trials.jsonl), so it works
//! retroactively on any recorded experiment.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::pycompat::{is_truthy, py_str};

// ─── file loading helpers ────────────────────────────────────────────────────

fn load_json(path: &std::path::PathBuf, default: Value) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(default)
}

fn records(exp_dir: &Path) -> Vec<Value> {
    let path = exp_dir.join("trials.jsonl");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

// ─── candidate stats ─────────────────────────────────────────────────────────

/// Per-candidate aggregates preserved in insertion order (mirrors Python dict).
struct CandidateStat {
    rewards: Vec<f64>,
    cost: f64,
    mean: f64,
    n: usize,
}

/// Build per-candidate stats in the order candidates first appear in `records`.
/// Mirrors Python's `dict.setdefault` + insertion-order traversal.
fn candidate_stats(recs: &[Value]) -> Vec<(String, CandidateStat)> {
    // Insertion-order map without the `indexmap` crate: Vec<(key, val)> + linear find.
    let mut out: Vec<(String, CandidateStat)> = Vec::new();
    for r in recs {
        let cid = match r.get("candidate_id").and_then(Value::as_str) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let reward = r.get("reward").and_then(Value::as_f64).unwrap_or(0.0);
        // `r.get("cost_usd") or 0` — Python truthiness: null/0/false → 0
        let cost = match r.get("cost_usd") {
            Some(v) if is_truthy(v) => v.as_f64().unwrap_or(0.0),
            _ => 0.0,
        };
        if let Some((_, stat)) = out.iter_mut().find(|(k, _)| k == &cid) {
            stat.rewards.push(reward);
            stat.cost += cost;
        } else {
            out.push((
                cid,
                CandidateStat {
                    rewards: vec![reward],
                    cost,
                    mean: 0.0,
                    n: 0,
                },
            ));
        }
    }
    for (_, stat) in &mut out {
        stat.n = stat.rewards.len();
        stat.mean = stat.rewards.iter().sum::<f64>() / stat.n as f64;
    }
    out
}

// ─── seed index ──────────────────────────────────────────────────────────────

/// Replicate `re.match(r"seed(\d+)-", candidate_id)`.
/// Returns the integer from the matched group, or None.
/// Plain string parsing: strip "seed" prefix, collect leading ASCII digits,
/// require a following '-'. No regex crate needed.
fn seed_index(candidate_id: &str) -> Option<usize> {
    let rest = candidate_id.strip_prefix("seed")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    // require '-' immediately after the digits
    if !rest[digits.len()..].starts_with('-') {
        return None;
    }
    digits.parse::<usize>().ok()
}

// ─── one_line ────────────────────────────────────────────────────────────────

/// Replicate `" ".join((text or "").split())` — collapses all whitespace.
fn one_line(text: Option<&str>) -> String {
    text.unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ─── hypothesis_verdict ──────────────────────────────────────────────────────

/// Score a generation's predicted_effect against its measurement.
/// Returns `Some((label, detail))` or `None` when no structured prediction exists.
pub fn hypothesis_verdict(h: &Value) -> Option<(String, String)> {
    let pe = h.get("predicted_effect")?;
    // `if not pe:` — falsy (None, empty dict, false, …) returns None
    if !is_truthy(pe) {
        return None;
    }

    let mut axes: Vec<(&'static str, String, bool, String)> = Vec::new();

    // reward axis: present only when mean_task_delta is not None
    if let Some(delta_v) = h.get("mean_task_delta") {
        if !delta_v.is_null() {
            let delta = delta_v.as_f64().unwrap_or(0.0);
            let reward_pred = pe.get("reward").and_then(Value::as_str).unwrap_or("");
            let ok = if reward_pred == "up" {
                delta > 0.02
            } else {
                delta >= -0.05
            };
            let meas = format!("Δ{delta:+.3}");
            axes.push(("reward", reward_pred.to_string(), ok, meas));
        }
    }

    // cost axis: `if pcpt and ccpt is not None:`
    //   pcpt must be truthy (non-zero, non-null); ccpt must not be null but may be 0.0
    let pcpt = h.get("parent_cost_per_trial");
    let ccpt = h.get("child_cost_per_trial");
    if let (Some(pcpt_v), Some(ccpt_v)) = (pcpt, ccpt) {
        if is_truthy(pcpt_v) && !ccpt_v.is_null() {
            let pcpt_f = pcpt_v.as_f64().unwrap_or(0.0);
            let ccpt_f = ccpt_v.as_f64().unwrap_or(0.0);
            let ratio = ccpt_f / pcpt_f;
            let cost_pred = pe.get("cost").and_then(Value::as_str).unwrap_or("");
            let ok = match cost_pred {
                "down" => ratio <= 0.9,
                "hold" => ratio > 0.9 && ratio < 1.1,
                "up" => true,
                // Python dict.get with default True for unknown keys
                _ => true,
            };
            let meas = format!("×{ratio:.2}");
            axes.push(("cost", cost_pred.to_string(), ok, meas));
        }
    }

    if axes.is_empty() {
        return None;
    }

    let all_ok = axes.iter().all(|(_, _, ok, _)| *ok);
    let any_ok = axes.iter().any(|(_, _, ok, _)| *ok);
    let label = if all_ok {
        "prediction confirmed"
    } else if !any_ok {
        "prediction refuted"
    } else {
        "prediction partially confirmed"
    };

    let detail = axes
        .iter()
        .map(|(axis, pred, ok, meas)| {
            let mark = if *ok { "✓" } else { "✗" };
            format!("{axis} {pred}: {mark} ({meas})")
        })
        .collect::<Vec<_>>()
        .join(", ");

    Some((label.to_string(), detail))
}

// ─── render ──────────────────────────────────────────────────────────────────

/// Render the full lineage markdown for an experiment directory.
/// Mirrors `lineage.render(exp_dir)` exactly.
pub fn render(exp_dir: &Path) -> String {
    let rig = load_json(&exp_dir.join("rig.json"), Value::Object(Default::default()));
    let seedj = load_json(
        &exp_dir.join("seed.json"),
        Value::Object(Default::default()),
    );
    let loopj = load_json(
        &exp_dir.join("loop.json"),
        Value::Object(Default::default()),
    );
    let pareto = load_json(
        &exp_dir.join("pareto.json"),
        Value::Array(Default::default()),
    );
    let stats = candidate_stats(&records(exp_dir));
    let stats_map: HashMap<&str, &CandidateStat> =
        stats.iter().map(|(k, v)| (k.as_str(), v)).collect();

    let exp_name = exp_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("# Experiment lineage — {exp_name}"));
    lines.push(String::new());

    // ── Rig ──────────────────────────────────────────────────────────────────
    lines.push("## Rig".to_string());
    lines.push(String::new());
    if is_truthy(&rig) {
        let saturated = rig.get("saturated").map(is_truthy).unwrap_or(false);
        let verdict = if saturated {
            "**saturated**"
        } else {
            "arena discriminates"
        };
        lines.push(format!(
            "- oracle {} · null {} · one-shot probe {} — {verdict}",
            py_str(&rig.get("oracle_mean").cloned().unwrap_or(Value::Null)),
            py_str(&rig.get("null_mean").cloned().unwrap_or(Value::Null)),
            py_str(&rig.get("probe_mean").cloned().unwrap_or(Value::Null)),
        ));
    } else {
        lines.push("- (no rig.json recorded)".to_string());
    }

    // ── Landscape scan (seed population) ─────────────────────────────────────
    lines.push(String::new());
    lines.push("## Landscape scan (seed population)".to_string());
    lines.push(String::new());
    if is_truthy(&seedj) {
        let rng_seed = py_str(&seedj.get("rng_seed").cloned().unwrap_or(Value::Null));
        let stances: Vec<String> = seedj
            .get("packet_stances")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().map(py_str).collect())
            .unwrap_or_default();
        lines.push(format!(
            "rng_seed {rng_seed} · packet stances: {}",
            stances.join(", ")
        ));

        let combos = seedj
            .get("combos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        // seeds: all candidate_ids with a seed_index, sorted by seed_index (stable)
        // Python: sorted((c for c in stats if _seed_index(c) is not None), key=_seed_index)
        let mut seeds: Vec<&str> = stats
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| seed_index(k).is_some())
            .collect();
        // Python `sorted` is stable; Rust slice::sort_by_key is stable
        seeds.sort_by_key(|k| seed_index(k).unwrap());

        if !seeds.is_empty() {
            lines.push(String::new());
            lines.push("| seed | model | thinking | tools | mean | n | cost |".to_string());
            lines.push("|---|---|---|---|---|---|---|".to_string());
            for sid in &seeds {
                let i = seed_index(sid).unwrap() - 1;
                let combo = combos
                    .get(i)
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));
                let s = stats_map[sid];
                lines.push(format!(
                    "| {sid} | {} | {} | {} | {:.3} | {} | ${:.3} |",
                    py_str(
                        &combo
                            .get("model")
                            .cloned()
                            .unwrap_or(Value::String("?".to_string()))
                    ),
                    py_str(
                        &combo
                            .get("thinking")
                            .cloned()
                            .unwrap_or(Value::String("?".to_string()))
                    ),
                    py_str(
                        &combo
                            .get("policy_name")
                            .cloned()
                            .unwrap_or(Value::String("?".to_string()))
                    ),
                    s.mean,
                    s.n,
                    s.cost,
                ));
            }
        }
    } else {
        lines.push("- (no seed.json recorded)".to_string());
    }

    // ── Generations (hypothesis → measurement → decision) ────────────────────
    lines.push(String::new());
    lines.push("## Generations (hypothesis → measurement → decision)".to_string());
    lines.push(String::new());
    let history = loopj
        .get("history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if history.is_empty() {
        lines.push("- (no search generations recorded)".to_string());
    }
    for h in &history {
        // f"g{h.get('generation')}.{h.get('attempt', '?')}"
        let gen = format!(
            "g{}.{}",
            py_str(&h.get("generation").cloned().unwrap_or(Value::Null)),
            py_str(
                &h.get("attempt")
                    .cloned()
                    .unwrap_or(Value::String("?".to_string()))
            )
        );
        if h.get("proposal_error").is_some() {
            lines.push(format!(
                "- {gen} parent `{}` — **proposal rejected**: {}",
                py_str(&h.get("parent_id").cloned().unwrap_or(Value::Null)),
                py_str(&h.get("proposal_error").cloned().unwrap_or(Value::Null)),
            ));
            continue;
        }
        let improved = h.get("improved").map(is_truthy).unwrap_or(false);
        let verdict = if improved {
            "**improvement — kept as a direction**"
        } else {
            "no improvement — direction discarded"
        };
        let donor_str = match h.get("donor") {
            Some(d) if is_truthy(d) => format!(" (transplant from `{}`)", py_str(d)),
            _ => String::new(),
        };
        lines.push(format!(
            "- {gen} `{}` ← `{}` (slot `{}`){donor_str}",
            py_str(&h.get("child_id").cloned().unwrap_or(Value::Null)),
            py_str(&h.get("parent_id").cloned().unwrap_or(Value::Null)),
            py_str(&h.get("slot_changed").cloned().unwrap_or(Value::Null)),
        ));
        lines.push(format!(
            "  - hypothesis: {}",
            py_str(&h.get("hypothesis").cloned().unwrap_or(Value::Null)),
        ));
        lines.push(format!(
            "  - measured: reward {} vs parent {} (paired Δ {}) → {verdict}",
            py_str(&h.get("reward_mean").cloned().unwrap_or(Value::Null)),
            py_str(&h.get("parent_reward_mean").cloned().unwrap_or(Value::Null)),
            py_str(&h.get("mean_task_delta").cloned().unwrap_or(Value::Null)),
        ));
        if let Some((label, detail)) = hypothesis_verdict(h) {
            lines.push(format!("  - {label}: {detail}"));
        }
    }

    // ── Meta-eval alarms ─────────────────────────────────────────────────────
    let alarms = loopj
        .get("alarms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !alarms.is_empty() {
        lines.push(String::new());
        lines.push("## Meta-eval alarms".to_string());
        lines.push(String::new());
        for a in &alarms {
            lines.push(format!(
                "- **{}**: {}",
                py_str(&a.get("kind").cloned().unwrap_or(Value::Null)),
                py_str(&a.get("detail").cloned().unwrap_or(Value::Null)),
            ));
        }
    }

    // ── Outcome ──────────────────────────────────────────────────────────────
    lines.push(String::new());
    lines.push("## Outcome".to_string());
    lines.push(String::new());
    lines.push(format!(
        "- stop: {} · generations {} · known spend ${}",
        py_str(&loopj.get("stop_reason").cloned().unwrap_or(Value::Null)),
        py_str(&loopj.get("generations").cloned().unwrap_or(Value::Null)),
        py_str(&loopj.get("spend_known_usd").cloned().unwrap_or(Value::Null)),
    ));
    // `certified = loopj.get("certified"); if certified is not None:`
    if let Some(certified) = loopj.get("certified").and_then(Value::as_array) {
        let joined: Vec<String> = certified.iter().map(py_str).collect();
        let s = joined.join(", ");
        // `', '.join(certified) or 'none'` — empty string is falsy → "none"
        let display = if s.is_empty() { "none" } else { s.as_str() };
        lines.push(format!("- certified: {display}"));
    }
    let pareto_arr = pareto.as_array().cloned().unwrap_or_default();
    for e in &pareto_arr {
        let mark = if e.get("recommended").map(is_truthy).unwrap_or(false) {
            " ← **recommended**"
        } else {
            ""
        };
        // `cpt = e.get("cost_usd_per_trial"); f", ${cpt:.4f}/trial" if cpt is not None else ""`
        let cpt = e.get("cost_usd_per_trial");
        let cost_str = match cpt {
            Some(v) if !v.is_null() => format!(", ${:.4}/trial", v.as_f64().unwrap_or(0.0)),
            _ => String::new(),
        };
        lines.push(format!(
            "- {} (hash {}): reward {}{}{}",
            py_str(&e.get("candidate_id").cloned().unwrap_or(Value::Null)),
            py_str(&e.get("composition_hash").cloned().unwrap_or(Value::Null)),
            py_str(&e.get("reward_mean").cloned().unwrap_or(Value::Null)),
            cost_str,
            mark,
        ));
    }

    // ── What this run taught us ───────────────────────────────────────────────
    lines.push(String::new());
    lines.push("## What this run taught us".to_string());
    lines.push(String::new());
    let mut taught = false;
    for h in &history {
        if h.get("proposal_error").is_some() {
            continue;
        }
        taught = true;
        let tag = if let Some((label, detail)) = hypothesis_verdict(h) {
            // `label.replace('prediction ', '')`
            let label_short = label.replace("prediction ", "");
            format!("{label_short}: {detail}")
        } else {
            let improved = h.get("improved").map(is_truthy).unwrap_or(false);
            if improved {
                "confirmed".to_string()
            } else {
                format!(
                    "not confirmed (Δ {})",
                    py_str(&h.get("mean_task_delta").cloned().unwrap_or(Value::Null))
                )
            }
        };
        lines.push(format!(
            "- [{tag}] {}",
            py_str(&h.get("hypothesis").cloned().unwrap_or(Value::Null)),
        ));
    }
    for a in &alarms {
        taught = true;
        lines.push(format!(
            "- [arena] {}",
            py_str(&a.get("detail").cloned().unwrap_or(Value::Null)),
        ));
    }
    if !taught {
        lines.push("- (none recorded)".to_string());
    }

    lines.join("\n") + "\n"
}

// ─── notebook_entry ──────────────────────────────────────────────────────────

/// A short committed lab-notebook entry.
/// Mirrors `lineage.notebook_entry(exp_dir, spec, arena_cfg)` exactly.
pub fn notebook_entry(exp_dir: &Path, spec: &Value, arena_cfg: &Value) -> String {
    let loopj = load_json(
        &exp_dir.join("loop.json"),
        Value::Object(Default::default()),
    );
    let pareto = load_json(
        &exp_dir.join("pareto.json"),
        Value::Array(Default::default()),
    );
    let pareto_arr = pareto.as_array().cloned().unwrap_or_default();

    // `next((e for e in pareto if e.get("recommended")), None)`
    let pick = pareto_arr
        .iter()
        .find(|e| e.get("recommended").map(is_truthy).unwrap_or(false));

    let exp_name = exp_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut lines: Vec<String> = Vec::new();
    // Python: first element is `f"\n## {exp_dir.name}"` (literal leading newline)
    lines.push(format!("\n## {exp_name}"));
    lines.push(String::new());
    lines.push(format!(
        "- spec `{}` (mode {}) on arena `{}` v{}",
        py_str(&spec.get("id").cloned().unwrap_or(Value::Null)),
        py_str(&spec.get("mode").cloned().unwrap_or(Value::Null)),
        py_str(&arena_cfg.get("id").cloned().unwrap_or(Value::Null)),
        py_str(&arena_cfg.get("version").cloned().unwrap_or(Value::Null)),
    ));
    lines.push(format!(
        "- stop: {} · spend ${} · generations {}",
        py_str(&loopj.get("stop_reason").cloned().unwrap_or(Value::Null)),
        py_str(&loopj.get("spend_known_usd").cloned().unwrap_or(Value::Null)),
        py_str(&loopj.get("generations").cloned().unwrap_or(Value::Null)),
    ));

    if let Some(p) = pick {
        lines.push(format!(
            "- recommended: `{}` (hash {}, reward {}, certified={})",
            py_str(&p.get("candidate_id").cloned().unwrap_or(Value::Null)),
            py_str(&p.get("composition_hash").cloned().unwrap_or(Value::Null)),
            py_str(&p.get("reward_mean").cloned().unwrap_or(Value::Null)),
            py_str(&p.get("certified").cloned().unwrap_or(Value::Null)),
        ));
    }

    let history = loopj
        .get("history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let confirmed: Vec<&Value> = history
        .iter()
        .filter(|h| h.get("improved").map(is_truthy).unwrap_or(false))
        .collect();
    if !confirmed.is_empty() {
        let hyps: Vec<String> = confirmed
            .iter()
            .map(|h| {
                let raw = h.get("hypothesis").and_then(Value::as_str);
                // `_one_line(h.get("hypothesis"))[:110]`
                one_line(raw).chars().take(110).collect()
            })
            .collect();
        lines.push(format!("- confirmed hypotheses: {}", hyps.join("; ")));
    }

    let alarms = loopj
        .get("alarms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    // `for a in loopj.get("alarms", [])[:3]:`
    for a in alarms.iter().take(3) {
        // `(a.get('detail') or '')[:140]` — None/falsy detail → empty string
        let detail_raw = a.get("detail").and_then(Value::as_str).unwrap_or("");
        let detail: String = detail_raw.chars().take(140).collect();
        lines.push(format!(
            "- alarm: {} — {detail}",
            py_str(&a.get("kind").cloned().unwrap_or(Value::Null)),
        ));
    }

    lines.push(format!("- full story: {exp_name}/lineage.md"));

    lines.join("\n") + "\n"
}

// ─── unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn fresh_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("daedalus-lineage-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn build_run(tmp: &std::path::Path) -> std::path::PathBuf {
        let exp = tmp.join("20260610T000000Z-search-demo");
        std::fs::create_dir_all(&exp).unwrap();
        std::fs::write(
            exp.join("rig.json"),
            r#"{"oracle_mean":1.0,"null_mean":0.25,"probe_mean":0.0,"saturated":false}"#,
        )
        .unwrap();
        std::fs::write(
            exp.join("seed.json"),
            r#"{"rng_seed":7,"seed_count":2,"packet_stances":["spec-first","skeptic"],
                "optimizer_costs":[0.01],
                "combos":[
                  {"model":"z-ai/glm-5","thinking":"high","policy_name":"full"},
                  {"model":"openai/gpt-5-mini","thinking":"low","policy_name":"explore"}
                ]}"#,
        )
        .unwrap();
        std::fs::write(
            exp.join("loop.json"),
            r#"{"stop_reason":"plateau","mode":"threshold-then-cheap",
                "generations":2,"spend_known_usd":1.23,
                "certified":["seed1-glm-5-spec-first"],
                "alarms":[{"kind":"saturation-at-top",
                           "detail":"seed1 at ceiling; cost search only"}],
                "history":[
                  {"generation":1,"attempt":0,"child_id":"g1a-seed1",
                   "parent_id":"seed1-glm-5-spec-first","slot_changed":"thinking",
                   "hypothesis":"medium thinking keeps reward and cuts cost",
                   "predicted_effect":{"reward":"hold","cost":"down"},
                   "parent_cost_per_trial":0.0171,"child_cost_per_trial":0.0138,
                   "parent_reward_mean":1.0,"reward_mean":1.0,
                   "mean_task_delta":0.0,"improved":true},
                  {"generation":2,"attempt":0,"child_id":"g2a-g1a",
                   "parent_id":"g1a-seed1","slot_changed":"prompt_packet",
                   "hypothesis":"stop instruction reduces spend",
                   "parent_reward_mean":1.0,"reward_mean":0.66,
                   "mean_task_delta":-0.33,"improved":false},
                  {"generation":2,"attempt":1,
                   "parent_id":"g1a-seed1","proposal_error":"slot not mutable"}
                ]}"#,
        )
        .unwrap();
        std::fs::write(
            exp.join("pareto.json"),
            r#"[{"candidate_id":"g1a-seed1","composition_hash":"abc123",
                 "reward_mean":1.0,"cost_usd_per_trial":0.0138,
                 "certified":true,"recommended":true}]"#,
        )
        .unwrap();
        let trials = [
            r#"{"candidate_id":"seed1-glm-5-spec-first","candidate_kind":"pi","task_id":"t1","reward":1.0,"cost_usd":0.02}"#,
            r#"{"candidate_id":"seed2-gpt-5-mini-skeptic","candidate_kind":"pi","task_id":"t1","reward":0.2,"cost_usd":0.01}"#,
            r#"{"candidate_id":"oracle","candidate_kind":"oracle","task_id":"t1","reward":1.0,"cost_usd":null}"#,
        ];
        std::fs::write(exp.join("trials.jsonl"), trials.join("\n") + "\n").unwrap();
        exp
    }

    #[test]
    fn render_tells_the_whole_story() {
        let tmp = fresh_dir();
        let exp = build_run(&tmp);
        let text = render(&exp);
        assert!(text.contains("arena discriminates"), "rig verdict");
        assert!(text.contains("z-ai/glm-5"), "model in landscape");
        assert!(text.contains("spec-first"), "stance in landscape");
        assert!(
            text.contains("medium thinking keeps reward and cuts cost"),
            "hypothesis"
        );
        assert!(text.contains("prediction confirmed"), "structured verdict");
        assert!(text.contains("reward hold: ✓"), "reward axis");
        assert!(text.contains("cost down: ✓"), "cost axis");
        assert!(text.contains("[not confirmed (Δ -0.33)]"), "legacy entry");
        assert!(text.contains("proposal rejected"), "error entry");
        assert!(text.contains("saturation-at-top"), "alarm");
        assert!(text.contains("certified: seed1-glm-5-spec-first"), "cert");
        assert!(text.contains("← **recommended**"), "recommendation");
    }

    #[test]
    fn hypothesis_verdict_refuted_and_partial() {
        let refuted = hypothesis_verdict(&json!({
            "predicted_effect": {"reward": "up", "cost": "down"},
            "mean_task_delta": -0.2,
            "parent_cost_per_trial": 0.01,
            "child_cost_per_trial": 0.02,
        }));
        assert_eq!(refuted.unwrap().0, "prediction refuted");

        let partial = hypothesis_verdict(&json!({
            "predicted_effect": {"reward": "up", "cost": "down"},
            "mean_task_delta": 0.3,
            "parent_cost_per_trial": 0.01,
            "child_cost_per_trial": 0.02,
        }));
        assert_eq!(partial.unwrap().0, "prediction partially confirmed");

        assert!(hypothesis_verdict(&json!({"improved": true})).is_none());
    }

    #[test]
    fn render_survives_missing_artifacts() {
        let tmp = fresh_dir();
        let exp = tmp.join("bare");
        std::fs::create_dir_all(&exp).unwrap();
        let text = render(&exp);
        assert!(text.contains("no rig.json recorded"));
        assert!(text.contains("no search generations recorded"));
    }

    #[test]
    fn notebook_entry_summarizes() {
        let tmp = fresh_dir();
        let exp = build_run(&tmp);
        let entry = notebook_entry(
            &exp,
            &json!({"id": "pr-review", "mode": "threshold-then-cheap"}),
            &json!({"id": "pr-review-v2", "version": "0.1.0"}),
        );
        assert!(entry.contains("pr-review-v2"));
        assert!(entry.contains("g1a-seed1"));
        assert!(entry.contains("certified=True"));
        assert!(entry.contains("lineage.md"));
    }

    #[test]
    fn notebook_entry_collapses_hypothesis_whitespace() {
        let tmp = fresh_dir();
        let exp = build_run(&tmp);
        let loop_path = exp.join("loop.json");
        let mut loopj: Value =
            serde_json::from_str(&std::fs::read_to_string(&loop_path).unwrap()).unwrap();
        loopj["history"][0]["hypothesis"] = json!("first line\nsecond line   ");
        std::fs::write(&loop_path, serde_json::to_string(&loopj).unwrap()).unwrap();

        let entry = notebook_entry(
            &exp,
            &json!({"id": "pr-review", "mode": "threshold-then-cheap"}),
            &json!({"id": "pr-review-v2", "version": "0.1.0"}),
        );
        assert!(entry.contains("first line second line"));
        assert!(!entry.contains("  \n"));
    }

    #[test]
    fn seed_index_parsing() {
        assert_eq!(seed_index("seed1-foo"), Some(1));
        assert_eq!(seed_index("seed42-bar-baz"), Some(42));
        assert_eq!(seed_index("oracle"), None);
        assert_eq!(seed_index("seed-no-number"), None);
        assert_eq!(seed_index("g1a-seed1"), None);
        assert_eq!(seed_index("seed"), None);
        assert_eq!(seed_index("seed123"), None); // no trailing '-'
    }
}
