//! Offline two-run delta (041): mechanical comparison of two committed run
//! directories, so cross-run comparison is not manual report-diffing.
//!
//! Reads each run's `pareto.json` (the per-candidate leaderboard) and
//! `loop.json` (the run-level verdict). Emits, per candidate present in either
//! run, the reward delta, the rank change (its index in
//! `loop.json.pareto_front`), and the cost-per-trial delta — plus the run-level
//! spend and stop-reason deltas.
//!
//! Unknown cost is never 0: a candidate with no recorded `cost_usd_per_trial`
//! has `cost = None`, and any delta touching an unknown side is also `None`. The
//! renderer prints "—" / "unknown" for these (AGENTS: unknown cost is null,
//! never an estimate).

use std::collections::BTreeMap;

use serde_json::Value;

/// One run's relevant contents, read from `pareto.json` + `loop.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    /// Run directory label (the basename).
    pub label: String,
    /// Per-candidate reward mean, keyed by candidate id.
    pub reward: BTreeMap<String, f64>,
    /// Per-candidate cost per trial, `None` when not recorded.
    pub cost_per_trial: BTreeMap<String, Option<f64>>,
    /// Candidate id → its rank (0-based index) in `loop.json.pareto_front`.
    pub rank: BTreeMap<String, usize>,
    /// `loop.json.spend_known_usd`, when recorded.
    pub spend_known_usd: Option<f64>,
    /// `loop.json.stop_reason`, when recorded.
    pub stop_reason: Option<String>,
}

/// Parse a run's `pareto.json` array + `loop.json` object into a `RunSummary`.
/// `pareto.json` carries reward/cost; `loop.json.pareto_front` carries the rank
/// order. Tolerant of missing fields — absent cost stays `None`.
pub fn summarize_run(label: &str, pareto: &Value, loop_json: &Value) -> RunSummary {
    let mut reward = BTreeMap::new();
    let mut cost_per_trial = BTreeMap::new();
    if let Some(arr) = pareto.as_array() {
        for c in arr {
            let Some(id) = c.get("candidate_id").and_then(Value::as_str) else {
                continue;
            };
            reward.insert(
                id.to_string(),
                c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0),
            );
            // null / absent → None (unknown), never 0.
            let cost = match c.get("cost_usd_per_trial") {
                Some(Value::Null) | None => None,
                Some(v) => v.as_f64(),
            };
            cost_per_trial.insert(id.to_string(), cost);
        }
    }
    let mut rank = BTreeMap::new();
    if let Some(front) = loop_json.get("pareto_front").and_then(Value::as_array) {
        for (i, v) in front.iter().enumerate() {
            if let Some(id) = v.as_str() {
                rank.insert(id.to_string(), i);
            }
        }
    }
    RunSummary {
        label: label.to_string(),
        reward,
        cost_per_trial,
        rank,
        spend_known_usd: loop_json.get("spend_known_usd").and_then(Value::as_f64),
        stop_reason: loop_json
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(String::from),
    }
}

/// A candidate's delta across the two runs. Any field is `None` when the
/// candidate is absent from one run or the underlying value is unknown.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateDelta {
    pub candidate_id: String,
    /// `reward_b − reward_a`; `None` when the candidate is in only one run.
    pub reward_delta: Option<f64>,
    /// `rank_b − rank_a` (index in `pareto_front`); negative = moved up toward
    /// the top. `None` when the candidate is unranked in either run.
    pub rank_delta: Option<i64>,
    /// `cost_b − cost_a` per trial; `None` when either side's cost is unknown or
    /// the candidate is in only one run. Never fabricated as 0.
    pub cost_per_trial_delta: Option<f64>,
    /// Whether the candidate appears in run A.
    pub in_a: bool,
    /// Whether the candidate appears in run B.
    pub in_b: bool,
}

/// The full two-run comparison: per-candidate deltas plus run-level deltas.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub label_a: String,
    pub label_b: String,
    /// One row per candidate present in either run, sorted by id.
    pub candidates: Vec<CandidateDelta>,
    /// `spend_b − spend_a`; `None` when either run did not record spend.
    pub spend_delta: Option<f64>,
    pub spend_a: Option<f64>,
    pub spend_b: Option<f64>,
    pub stop_reason_a: Option<String>,
    pub stop_reason_b: Option<String>,
}

/// Compute the delta between two run summaries. Pure; no I/O.
pub fn compare(a: &RunSummary, b: &RunSummary) -> Comparison {
    // Union of candidate ids across both runs, sorted (BTreeMap keys are sorted;
    // BTreeSet gives a stable union order).
    let mut ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    ids.extend(a.reward.keys().cloned());
    ids.extend(b.reward.keys().cloned());

    let candidates = ids
        .into_iter()
        .map(|id| {
            let in_a = a.reward.contains_key(&id);
            let in_b = b.reward.contains_key(&id);
            let reward_delta = match (a.reward.get(&id), b.reward.get(&id)) {
                (Some(ra), Some(rb)) => Some(rb - ra),
                _ => None,
            };
            let rank_delta = match (a.rank.get(&id), b.rank.get(&id)) {
                (Some(ra), Some(rb)) => Some(*rb as i64 - *ra as i64),
                _ => None,
            };
            // Cost delta only when BOTH sides have a known cost; an unknown on
            // either side leaves the delta unknown rather than fabricating 0.
            let cost_per_trial_delta = match (
                a.cost_per_trial.get(&id).copied().flatten(),
                b.cost_per_trial.get(&id).copied().flatten(),
            ) {
                (Some(ca), Some(cb)) => Some(cb - ca),
                _ => None,
            };
            CandidateDelta {
                candidate_id: id,
                reward_delta,
                rank_delta,
                cost_per_trial_delta,
                in_a,
                in_b,
            }
        })
        .collect();

    let spend_delta = match (a.spend_known_usd, b.spend_known_usd) {
        (Some(sa), Some(sb)) => Some(sb - sa),
        _ => None,
    };

    Comparison {
        label_a: a.label.clone(),
        label_b: b.label.clone(),
        candidates,
        spend_delta,
        spend_a: a.spend_known_usd,
        spend_b: b.spend_known_usd,
        stop_reason_a: a.stop_reason.clone(),
        stop_reason_b: b.stop_reason.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run_a() -> RunSummary {
        summarize_run(
            "runA",
            &json!([
                {"candidate_id": "alpha", "reward_mean": 0.50, "cost_usd_per_trial": 0.020},
                {"candidate_id": "beta",  "reward_mean": 0.40, "cost_usd_per_trial": null},
                {"candidate_id": "gamma", "reward_mean": 0.30, "cost_usd_per_trial": 0.010},
            ]),
            &json!({
                "pareto_front": ["alpha", "beta", "gamma"],
                "spend_known_usd": 1.0,
                "stop_reason": "max-candidates",
            }),
        )
    }

    fn run_b() -> RunSummary {
        summarize_run(
            "runB",
            &json!([
                // alpha improves and gets cheaper; rank holds at top.
                {"candidate_id": "alpha", "reward_mean": 0.60, "cost_usd_per_trial": 0.015},
                // gamma jumps above beta (rank up) — cost unknown in B now.
                {"candidate_id": "gamma", "reward_mean": 0.55, "cost_usd_per_trial": null},
                // beta drops below gamma (rank down).
                {"candidate_id": "beta",  "reward_mean": 0.45, "cost_usd_per_trial": 0.030},
                // delta only in B.
                {"candidate_id": "delta", "reward_mean": 0.70, "cost_usd_per_trial": 0.005},
            ]),
            &json!({
                "pareto_front": ["delta", "alpha", "gamma", "beta"],
                "spend_known_usd": 1.6,
                "stop_reason": "plateau",
            }),
        )
    }

    #[test]
    fn reward_and_cost_deltas_for_shared_candidate() {
        let cmp = compare(&run_a(), &run_b());
        let alpha = cmp
            .candidates
            .iter()
            .find(|c| c.candidate_id == "alpha")
            .unwrap();
        assert!((alpha.reward_delta.unwrap() - 0.10).abs() < 1e-9);
        // 0.015 − 0.020 = −0.005 cheaper per trial.
        assert!((alpha.cost_per_trial_delta.unwrap() - (-0.005)).abs() < 1e-9);
        assert!(alpha.in_a && alpha.in_b);
    }

    #[test]
    fn rank_change_is_index_delta_in_pareto_front() {
        let cmp = compare(&run_a(), &run_b());
        // gamma: rank 2 in A → rank 2 in B... wait: A front [alpha,beta,gamma]
        // → gamma=2; B front [delta,alpha,gamma,beta] → gamma=2. Delta 0.
        let gamma = cmp
            .candidates
            .iter()
            .find(|c| c.candidate_id == "gamma")
            .unwrap();
        assert_eq!(gamma.rank_delta, Some(0));
        // beta: A rank 1 → B rank 3 ⇒ +2 (moved down).
        let beta = cmp
            .candidates
            .iter()
            .find(|c| c.candidate_id == "beta")
            .unwrap();
        assert_eq!(beta.rank_delta, Some(2));
    }

    #[test]
    fn unknown_cost_yields_none_delta_never_zero() {
        let cmp = compare(&run_a(), &run_b());
        // gamma cost: A=0.010, B=unknown ⇒ delta None, not 0.0 and not -0.010.
        let gamma = cmp
            .candidates
            .iter()
            .find(|c| c.candidate_id == "gamma")
            .unwrap();
        assert_eq!(gamma.cost_per_trial_delta, None);
    }

    #[test]
    fn candidate_only_in_one_run_has_no_deltas() {
        let cmp = compare(&run_a(), &run_b());
        let delta = cmp
            .candidates
            .iter()
            .find(|c| c.candidate_id == "delta")
            .unwrap();
        assert_eq!(delta.reward_delta, None);
        assert_eq!(delta.rank_delta, None);
        assert_eq!(delta.cost_per_trial_delta, None);
        assert!(!delta.in_a && delta.in_b);
    }

    #[test]
    fn run_level_spend_and_stop_reason_deltas() {
        let cmp = compare(&run_a(), &run_b());
        assert!((cmp.spend_delta.unwrap() - 0.6).abs() < 1e-9);
        assert_eq!(cmp.stop_reason_a.as_deref(), Some("max-candidates"));
        assert_eq!(cmp.stop_reason_b.as_deref(), Some("plateau"));
    }

    #[test]
    fn missing_spend_leaves_delta_unknown() {
        let a = summarize_run("a", &json!([]), &json!({"pareto_front": []}));
        let b = summarize_run(
            "b",
            &json!([]),
            &json!({"pareto_front": [], "spend_known_usd": 2.0}),
        );
        let cmp = compare(&a, &b);
        assert_eq!(cmp.spend_delta, None);
        assert_eq!(cmp.spend_a, None);
        assert_eq!(cmp.spend_b, Some(2.0));
    }
}
