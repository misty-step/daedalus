//! Inferential statistics for the foundry: turning a tournament's point
//! estimates into confidence-bounded claims.
//!
//! Backlog 039 ("prove better, not just rank") child-1: a 95% confidence
//! interval on the reward delta `(candidate − baseline)`, computed with
//! **cluster-robust** standard errors so that correlated tasks (multiple tasks
//! drawn from the same source repo) do not understate the variance.
//!
//! Grounding: Evan Miller, "Adding Error Bars to Evals" (arXiv 2411.00640) —
//! an eval score is a sample statistic; report `point ± 1.96·SE`, and when the
//! items are correlated (clustered) use a cluster-robust SE rather than the
//! naive one. pr-review-v2's tasks come from two repos (rich, pygments), so the
//! naive per-trial SE understates the true sampling variance.
//!
//! Scope boundary: this module is the *statistics engine*. The cluster key
//! (which source repo a task belongs to) is injected by the caller. Live
//! arenas gain `source_repo` labels under backlog 040 (contamination records);
//! until then the default per-task clustering applies and the emitted
//! `n_clusters` makes the fallback visible.

use crate::pycompat::{mean, round_half_even};
use serde_json::{Map, Value};

/// The standard-normal two-sided 95% multiplier. Miller's headline form is
/// `point ± 1.96·SE`. This is a normal approximation; with very few clusters it
/// is anticonservative (the cluster-count degrees of freedom would call for a
/// Student-t multiplier). The small-sample correction and the matching power
/// note are backlog 039 child-5; `n_clusters` is emitted so the caveat is
/// legible at the point of use.
const Z_95: f64 = 1.96;

/// A confidence-bounded reward delta `(candidate − baseline)`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaCi {
    /// Mean per-task delta (candidate − baseline) over the common tasks.
    pub point: f64,
    /// Cluster-robust (CR1) standard error of `point`.
    pub se: f64,
    /// Lower bound of the 95% CI (`point − 1.96·se`).
    pub lo: f64,
    /// Upper bound of the 95% CI (`point + 1.96·se`).
    pub hi: f64,
    /// Number of common tasks (the items differenced).
    pub n_tasks: usize,
    /// Number of distinct clusters those tasks fall into.
    pub n_clusters: usize,
    /// Whether the CI excludes 0 — i.e. the delta is significant at 95%.
    pub excludes_zero: bool,
}

impl DeltaCi {
    /// Serialize for `loop.json` / `pareto.json`, tagged with the baseline id
    /// it was computed against.
    pub fn to_value(&self, baseline_id: &str) -> Value {
        let mut m = Map::new();
        m.insert("baseline".into(), Value::String(baseline_id.to_string()));
        m.insert("point".into(), Value::from(self.point));
        m.insert("se".into(), Value::from(self.se));
        m.insert("lo".into(), Value::from(self.lo));
        m.insert("hi".into(), Value::from(self.hi));
        m.insert("ci".into(), Value::from(0.95));
        m.insert("n_tasks".into(), Value::from(self.n_tasks as u64));
        m.insert("n_clusters".into(), Value::from(self.n_clusters as u64));
        m.insert("excludes_zero".into(), Value::Bool(self.excludes_zero));
        Value::Object(m)
    }
}

/// Pull a candidate's per-task reward vectors out of the `report::aggregate`
/// shape (`{"tasks": {task_id: [reward, ...]}}`). Tasks with no rewards are
/// dropped so a later `mean` never sees an empty slice.
fn task_rewards(candidate: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(tasks) = candidate.get("tasks").and_then(Value::as_object) {
        for (tid, v) in tasks {
            if let Some(arr) = v.as_array() {
                if !arr.is_empty() {
                    out.insert(tid.clone(), Value::Array(arr.clone()));
                }
            }
        }
    }
    out
}

fn rewards_mean(v: &Value) -> Option<f64> {
    let xs: Vec<f64> = v.as_array()?.iter().filter_map(Value::as_f64).collect();
    if xs.is_empty() {
        None
    } else {
        Some(mean(&xs))
    }
}

/// 95% CI on the mean reward delta `(candidate − baseline)`, with a
/// cluster-robust standard error.
///
/// The delta is *paired per task*: for each task common to both candidates,
/// `d_t = mean(candidate rewards on t) − mean(baseline rewards on t)`. The
/// point estimate is the equal-weight mean of `d_t` over common tasks (tasks
/// are the experimental unit, matching `search_loop::improved_over`). The
/// standard error is the CR1 cluster-robust SE of that mean, where
/// `cluster_of(task_id)` assigns each task to a cluster:
///
/// ```text
/// V = (G / (G − 1)) · (1 / T²) · Σ_g ( Σ_{t∈g} (d_t − point) )²
/// SE = sqrt(V),  CI = point ± 1.96·SE
/// ```
///
/// with `T` common tasks across `G` clusters. When every task is its own
/// cluster (`G = T`, the default), this reduces exactly to the ordinary
/// standard error of the mean `stdev(d_t)/√T`; pooling correlated tasks into a
/// shared cluster widens it.
///
/// Returns `None` when the interval is undefined: fewer than 2 common tasks, or
/// fewer than 2 clusters (a single cluster cannot estimate between-cluster
/// variance).
pub fn reward_delta_ci(
    candidate: &Value,
    baseline: &Value,
    cluster_of: &dyn Fn(&str) -> String,
) -> Option<DeltaCi> {
    let cand = task_rewards(candidate);
    let base = task_rewards(baseline);

    // Common tasks, in the candidate's task order for stable output.
    let mut deltas: Vec<(String, f64)> = Vec::new();
    for (tid, c_rewards) in &cand {
        let Some(b_rewards) = base.get(tid) else {
            continue;
        };
        let (Some(cm), Some(bm)) = (rewards_mean(c_rewards), rewards_mean(b_rewards)) else {
            continue;
        };
        deltas.push((tid.clone(), cm - bm));
    }

    let t = deltas.len();
    if t < 2 {
        return None;
    }

    let point_raw = mean(&deltas.iter().map(|(_, d)| *d).collect::<Vec<_>>());

    // Group residuals by cluster, summing within each cluster.
    let mut cluster_sums: std::collections::BTreeMap<String, f64> =
        std::collections::BTreeMap::new();
    for (tid, d) in &deltas {
        let key = cluster_of(tid);
        *cluster_sums.entry(key).or_insert(0.0) += d - point_raw;
    }

    let g = cluster_sums.len();
    if g < 2 {
        return None;
    }

    let sum_sq: f64 = cluster_sums.values().map(|s| s * s).sum();
    let tf = t as f64;
    let gf = g as f64;
    let variance = (gf / (gf - 1.0)) * (1.0 / (tf * tf)) * sum_sq;
    let se_raw = variance.max(0.0).sqrt();

    let point = round_half_even(point_raw, 4);
    let se = round_half_even(se_raw, 6);
    let lo = round_half_even(point_raw - Z_95 * se_raw, 4);
    let hi = round_half_even(point_raw + Z_95 * se_raw, 4);
    let excludes_zero = lo > 0.0 || hi < 0.0;

    Some(DeltaCi {
        point,
        se,
        lo,
        hi,
        n_tasks: t,
        n_clusters: g,
        excludes_zero,
    })
}

/// Compute the reward-delta CI for every certified candidate against the
/// reference whose `kind` is `baseline_kind` (e.g. `"null"`, the floor).
///
/// Returns `(baseline_id, cis)` where `cis` is sorted by candidate id and skips
/// the baseline itself and any candidate whose interval is undefined. If no
/// baseline of that kind is present, returns `(None, [])`.
pub fn certified_delta_cis(
    cands: &Map<String, Value>,
    certified: &std::collections::HashSet<String>,
    baseline_kind: &str,
    cluster_of: &dyn Fn(&str) -> String,
) -> (Option<String>, Vec<(String, DeltaCi)>) {
    let baseline = cands
        .iter()
        .find(|(_, c)| c.get("kind").and_then(Value::as_str) == Some(baseline_kind));
    let Some((base_id, base_val)) = baseline else {
        return (None, Vec::new());
    };
    let mut ids: Vec<&String> = certified.iter().collect();
    ids.sort();
    let mut out: Vec<(String, DeltaCi)> = Vec::new();
    for cid in ids {
        if cid == base_id {
            continue;
        }
        if let Some(cv) = cands.get(cid) {
            if let Some(ci) = reward_delta_ci(cv, base_val, cluster_of) {
                out.push((cid.clone(), ci));
            }
        }
    }
    (Some(base_id.clone()), out)
}

/// Render the certified-candidate CIs as a `report.md` section. Returns an
/// empty string when there is nothing to show, so callers can append
/// unconditionally.
pub fn delta_ci_markdown(baseline_id: &str, cis: &[(String, DeltaCi)]) -> String {
    if cis.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    s.push_str(&format!(
        "\n## Reward delta vs baseline (95% CI)\n\nCluster-robust 95% CI on (candidate − `{baseline_id}`) mean reward, tasks clustered by source repo. A CI that excludes 0 is an improvement over the floor at 95% confidence. _Normal approximation; with few clusters (see n_clusters) it is anticonservative — backlog 039 child-5._\n\n"
    ));
    s.push_str("| candidate | Δ reward | 95% CI | n_tasks | n_clusters | sig |\n");
    s.push_str("|---|---|---|---|---|---|\n");
    for (cid, ci) in cis {
        s.push_str(&format!(
            "| {cid} | {:+.4} | [{:+.4}, {:+.4}] | {} | {} | {} |\n",
            ci.point,
            ci.lo,
            ci.hi,
            ci.n_tasks,
            ci.n_clusters,
            if ci.excludes_zero { "✓" } else { "—" }
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    /// Build a candidate in the `report::aggregate` shape from (task, rewards).
    fn cand(tasks: &[(&str, &[f64])]) -> Value {
        let mut m = Map::new();
        for (tid, rs) in tasks {
            m.insert(
                tid.to_string(),
                Value::Array(rs.iter().map(|&r| json!(r)).collect()),
            );
        }
        json!({ "tasks": Value::Object(m) })
    }

    /// Default per-task clustering: each task is its own cluster.
    fn singleton(t: &str) -> String {
        t.to_string()
    }

    #[test]
    fn singleton_clusters_reduce_to_standard_se_of_the_mean() {
        // Per-task deltas 0.2, 0.4, 0.6 (baseline all 0.0).
        // point = 0.4; standard SE = stdev/√3 = 0.2/√3 = 0.115470.
        let c = cand(&[("a", &[0.2]), ("b", &[0.4]), ("c", &[0.6])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let ci = reward_delta_ci(&c, &b, &singleton).expect("defined");
        assert_eq!(ci.point, 0.4);
        assert_eq!(ci.se, 0.11547);
        assert_eq!(ci.lo, 0.1737);
        assert_eq!(ci.hi, 0.6263);
        assert_eq!(ci.n_tasks, 3);
        assert_eq!(ci.n_clusters, 3);
        assert!(ci.excludes_zero);
    }

    #[test]
    fn clustering_correlated_tasks_widens_the_interval() {
        // Same three deltas (0.6, 0.6, 0.0) but tasks a,b share repo R1 and
        // their residuals reinforce: clustered SE must exceed the per-task SE.
        let c = cand(&[("a", &[0.6]), ("b", &[0.6]), ("c", &[0.0])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);

        let per_task = reward_delta_ci(&c, &b, &singleton).expect("defined");
        // point = 0.4; per-task SE = stdev([0.6,0.6,0.0])/√3.
        assert_eq!(per_task.point, 0.4);
        assert_eq!(per_task.se, 0.2);
        assert_eq!(per_task.n_clusters, 3);

        let by_repo = |t: &str| match t {
            "a" | "b" => "R1".to_string(),
            _ => "R2".to_string(),
        };
        let clustered = reward_delta_ci(&c, &b, &by_repo).expect("defined");
        // CR1 with G=2: V = 2·(1/9)·((+0.4)² + (−0.4)²) = 0.071111; SE = 0.266667.
        assert_eq!(clustered.point, 0.4);
        assert_eq!(clustered.se, 0.266667);
        assert_eq!(clustered.lo, -0.1227);
        assert_eq!(clustered.hi, 0.9227);
        assert_eq!(clustered.n_clusters, 2);
        // Pooling the correlated repo widened the SE and the CI now spans 0.
        assert!(clustered.se > per_task.se);
        assert!(!clustered.excludes_zero);
    }

    #[test]
    fn averages_trials_within_a_task_before_differencing() {
        // Candidate task a: trials [0.4, 0.6] → mean 0.5; baseline 0.1.
        // task b: [0.2, 0.4] → 0.3; baseline 0.1. deltas: 0.4, 0.2 → point 0.3.
        let c = cand(&[("a", &[0.4, 0.6]), ("b", &[0.2, 0.4])]);
        let b = cand(&[("a", &[0.1]), ("b", &[0.1])]);
        let ci = reward_delta_ci(&c, &b, &singleton).expect("defined");
        assert_eq!(ci.point, 0.3);
        assert_eq!(ci.n_tasks, 2);
    }

    #[test]
    fn zero_variance_gives_a_point_interval() {
        // Every task improves by exactly 0.5 → SE 0, CI collapses to the point.
        let c = cand(&[("a", &[0.7]), ("b", &[0.7]), ("c", &[0.7])]);
        let b = cand(&[("a", &[0.2]), ("b", &[0.2]), ("c", &[0.2])]);
        let ci = reward_delta_ci(&c, &b, &singleton).expect("defined");
        assert_eq!(ci.point, 0.5);
        assert_eq!(ci.se, 0.0);
        assert_eq!(ci.lo, 0.5);
        assert_eq!(ci.hi, 0.5);
        assert!(ci.excludes_zero);
    }

    #[test]
    fn negative_delta_interval_is_reported() {
        // Candidate is worse: deltas -0.2, -0.4, -0.6 → point -0.4, CI below 0.
        let c = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let b = cand(&[("a", &[0.2]), ("b", &[0.4]), ("c", &[0.6])]);
        let ci = reward_delta_ci(&c, &b, &singleton).expect("defined");
        assert_eq!(ci.point, -0.4);
        assert!(ci.hi < 0.0);
        assert!(ci.excludes_zero);
    }

    #[test]
    fn single_common_task_is_undefined() {
        let c = cand(&[("a", &[0.5]), ("x", &[0.5])]);
        let b = cand(&[("a", &[0.1])]); // only "a" is common
        assert!(reward_delta_ci(&c, &b, &singleton).is_none());
    }

    #[test]
    fn no_common_tasks_is_undefined() {
        let c = cand(&[("a", &[0.5]), ("b", &[0.5])]);
        let b = cand(&[("x", &[0.1]), ("y", &[0.1])]);
        assert!(reward_delta_ci(&c, &b, &singleton).is_none());
    }

    #[test]
    fn single_cluster_is_undefined() {
        // Two tasks but the caller maps both to one repo: between-cluster
        // variance is unestimable, so the clustered CI is undefined.
        let c = cand(&[("a", &[0.6]), ("b", &[0.4])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0])]);
        let one_repo = |_t: &str| "R1".to_string();
        assert!(reward_delta_ci(&c, &b, &one_repo).is_none());
    }

    #[test]
    fn serializes_with_baseline_tag() {
        let c = cand(&[("a", &[0.2]), ("b", &[0.4]), ("c", &[0.6])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let ci = reward_delta_ci(&c, &b, &singleton).unwrap();
        let v = ci.to_value("null");
        assert_eq!(v["baseline"], json!("null"));
        assert_eq!(v["point"], json!(0.4));
        assert_eq!(v["n_clusters"], json!(3));
        assert_eq!(v["excludes_zero"], json!(true));
        assert_eq!(v["ci"], json!(0.95));
    }

    /// (task_id, rewards) and (candidate_id, kind, tasks) for the test builders.
    type TaskSpec<'a> = (&'a str, &'a [f64]);
    type CandSpec<'a> = (&'a str, &'a str, &'a [TaskSpec<'a>]);

    /// Build the `report::aggregate` candidate map: (id, kind, tasks).
    fn cands_map(entries: &[CandSpec]) -> Map<String, Value> {
        let mut m = Map::new();
        for (id, kind, tasks) in entries {
            let mut t = Map::new();
            for (tid, rs) in *tasks {
                t.insert(
                    tid.to_string(),
                    Value::Array(rs.iter().map(|&r| json!(r)).collect()),
                );
            }
            m.insert(
                id.to_string(),
                json!({ "kind": kind, "tasks": Value::Object(t) }),
            );
        }
        m
    }

    #[test]
    fn certified_cis_difference_each_candidate_against_the_null_floor() {
        let cands = cands_map(&[
            (
                "null",
                "null",
                &[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])],
            ),
            (
                "oracle",
                "oracle",
                &[("a", &[1.0]), ("b", &[1.0]), ("c", &[1.0])],
            ),
            (
                "cand-x",
                "pi",
                &[("a", &[0.2]), ("b", &[0.4]), ("c", &[0.6])],
            ),
        ]);
        let certified: HashSet<String> = ["cand-x".to_string(), "null".to_string()]
            .into_iter()
            .collect();
        let (base, cis) = certified_delta_cis(&cands, &certified, "null", &singleton);
        assert_eq!(base.as_deref(), Some("null"));
        // Baseline skips itself; only cand-x carries a CI.
        assert_eq!(cis.len(), 1);
        assert_eq!(cis[0].0, "cand-x");
        assert_eq!(cis[0].1.point, 0.4);
        assert!(cis[0].1.excludes_zero);
    }

    #[test]
    fn certified_cis_empty_when_no_baseline_of_that_kind() {
        let cands = cands_map(&[("cand-x", "pi", &[("a", &[0.2]), ("b", &[0.4])])]);
        let certified: HashSet<String> = ["cand-x".to_string()].into_iter().collect();
        let (base, cis) = certified_delta_cis(&cands, &certified, "null", &singleton);
        assert!(base.is_none());
        assert!(cis.is_empty());
    }

    #[test]
    fn markdown_renders_a_signed_ci_row_per_candidate() {
        let cands = cands_map(&[
            (
                "null",
                "null",
                &[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])],
            ),
            (
                "cand-x",
                "pi",
                &[("a", &[0.2]), ("b", &[0.4]), ("c", &[0.6])],
            ),
        ]);
        let certified: HashSet<String> = ["cand-x".to_string()].into_iter().collect();
        let (base, cis) = certified_delta_cis(&cands, &certified, "null", &singleton);
        let md = delta_ci_markdown(base.as_deref().unwrap(), &cis);
        assert!(md.contains("## Reward delta vs baseline (95% CI)"));
        assert!(md.contains("| cand-x | +0.4000 | [+0.1737, +0.6263] | 3 | 3 | ✓ |"));
        // Nothing to render → empty string, safe to append unconditionally.
        assert_eq!(delta_ci_markdown("null", &[]), "");
    }
}
