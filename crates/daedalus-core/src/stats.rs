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

/// The standard-normal two-sided 95% multiplier — the large-sample limit of the
/// Student-t critical value. Used where the degrees of freedom are not a fixed
/// small number: the power projection (`min_clusters_to_significance`, an
/// asymptotic planning figure) and the basin pooled-noise band (a coarse
/// between-seed heuristic). The reward-delta CI itself uses [`t_975`] instead.
const Z_95: f64 = 1.96;

/// Two-sided 95% Student-t critical value for `df` degrees of freedom.
///
/// Backlog 040: once tasks cluster by source repo, the cluster count `G` can be
/// tiny (pr-review-v2 has 2 repos → `df = 1`). The cluster-robust SE is then
/// estimated from very few clusters, and the normal `1.96` is badly
/// anticonservative — the correct critical value is `t_{G−1}` (Cameron & Miller,
/// "A Practitioner's Guide to Cluster-Robust Inference"). At `df = 1` that is
/// 12.706, so a 2-repo arena honestly certifies almost nothing; precision comes
/// from *more clusters*, not more tasks in the same repos.
///
/// Table for `df` 1–30; the normal limit `1.96` for `df ≥ 31` (the residual
/// understatement there is < 5% and arenas rarely have that many clusters).
fn t_975(df: usize) -> f64 {
    const TABLE: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160,
        2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056,
        2.052, 2.048, 2.045, 2.042,
    ];
    match df {
        0 => TABLE[0], // unreachable in practice (CI requires G ≥ 2 → df ≥ 1)
        d if d <= 30 => TABLE[d - 1],
        d => {
            // df > 30: Cornish–Fisher refinement of the normal quantile,
            // z + (z³ + z)/(4·df) — within ~0.2% of the true t (e.g. df=31 →
            // 2.0365 vs 2.0395), not the flat 1.96 (which would be ~4% narrow).
            let z = Z_95;
            z + (z * z * z + z) / (4.0 * d as f64)
        }
    }
}

/// A confidence-bounded reward delta `(candidate − baseline)`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaCi {
    /// Mean per-task delta (candidate − baseline) over the common tasks.
    pub point: f64,
    /// Cluster-robust (CR1) standard error of `point`.
    pub se: f64,
    /// Lower bound of the 95% CI (`point − t_{G−1}·se`).
    pub lo: f64,
    /// Upper bound of the 95% CI (`point + t_{G−1}·se`).
    pub hi: f64,
    /// Number of common tasks (the items differenced).
    pub n_tasks: usize,
    /// Number of distinct clusters those tasks fall into.
    pub n_clusters: usize,
    /// Whether the CI excludes 0 — i.e. the delta is significant at 95%.
    pub excludes_zero: bool,
    /// Unrounded lower bound (`point − t_{G−1}·se`), kept for threshold tests
    /// like the certification gate so they don't inherit display rounding. Not
    /// serialized — `to_value` emits the 4-dp `lo`.
    lo_raw: f64,
}

impl DeltaCi {
    /// Whether the delta is significantly greater than `min_effect` — the lower
    /// bound of the 95% CI clears the threshold. `beats(0.0)` is "significantly
    /// better than the baseline." Uses the unrounded lower bound so a true
    /// `+3.7e-5` win that displays as `0.0000` still counts.
    pub fn beats(&self, min_effect: f64) -> bool {
        self.lo_raw > min_effect
    }

    /// Backlog 039 child-5 — the power note: the minimum number of independent
    /// **clusters** (the unit the cluster-robust SE is computed over — tasks
    /// today, source repos once 040 lands labels) needed to make the *observed*
    /// effect marginally significant at 95% (the expected CI just reaches 0).
    ///
    /// The CR1 SE shrinks with the cluster count as `1/√G`, not with tasks added
    /// inside existing clusters. Projecting the SE to a candidate cluster count
    /// `G` gives `se·√(n_clusters/G)`, and significance there uses the **same
    /// t_{G−1} critical value as the CI** — so this is consistent with
    /// [`reward_delta_ci`]: it returns the smallest `G ≥ 2` at which
    /// `point > t_{G−1}·se·√(n_clusters/G)`. Because `t` is large at few clusters,
    /// the answer is honestly bigger than a normal-approximation estimate. `None`
    /// when `point ≤ 0` (no positive effect). A candidate already significant
    /// returns `≤ n_clusters`.
    ///
    /// This is the marginal threshold (≈50% chance of actually clearing 0 at
    /// exactly this G); budget more clusters for reliable detection. It assumes
    /// added clusters carry similar between-cluster variance.
    pub fn min_clusters_to_significance(&self) -> Option<usize> {
        if self.point <= 0.0 {
            return None;
        }
        if self.se <= 0.0 {
            return Some(2); // already perfectly precise; 2 clusters suffice
        }
        // t_{G−1} shrinks and the projected SE shrinks as G grows, so for any
        // positive effect this crosses; 4096 is a safety cap, not a real bound.
        for g in 2..=4096 {
            let se_at = self.se * (self.n_clusters as f64 / g as f64).sqrt();
            if self.point > t_975(g - 1) * se_at {
                return Some(g);
            }
        }
        None
    }

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
        m.insert(
            "min_clusters_95".into(),
            self.min_clusters_to_significance()
                .map(|n| Value::from(n as u64))
                .unwrap_or(Value::Null),
        );
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
/// SE = sqrt(V),  CI = point ± t_{G−1}·SE
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

    // t_{G−1} critical value: with few clusters the normal 1.96 is
    // anticonservative; the cluster-robust SE has G−1 degrees of freedom.
    let crit = t_975(g - 1);
    let lo_raw = point_raw - crit * se_raw;
    let hi_raw = point_raw + crit * se_raw;
    let point = round_half_even(point_raw, 4);
    let se = round_half_even(se_raw, 6);
    let lo = round_half_even(lo_raw, 4);
    let hi = round_half_even(hi_raw, 4);
    // Significance reads the unrounded bounds: a true +3.7e-5 lower bound that
    // displays as 0.0000 is still a win, not a tie.
    let excludes_zero = lo_raw > 0.0 || hi_raw < 0.0;

    Some(DeltaCi {
        point,
        se,
        lo,
        hi,
        n_tasks: t,
        n_clusters: g,
        excludes_zero,
        lo_raw,
    })
}

/// Whether `candidate` is *significantly* better than `baseline` by more than
/// `min_effect`: the `(candidate − baseline)` reward-delta CI is defined and its
/// lower bound clears `min_effect`. An undefined CI (too few tasks/clusters to
/// bound) is not significant — the foundry cannot prove a win it cannot bound.
/// This is backlog 039 child-2's certification gate.
pub fn passes_significance(
    candidate: &Value,
    baseline: &Value,
    cluster_of: &dyn Fn(&str) -> String,
    min_effect: f64,
) -> bool {
    reward_delta_ci(candidate, baseline, cluster_of).is_some_and(|ci| ci.beats(min_effect))
}

/// Split trial-complete candidates into `(certified, underpowered)` by the
/// significance gate (backlog 039 child-2): a candidate is **certified** when
/// its `(candidate − baseline)` reward-delta CI lower bound clears `min_effect`,
/// and **underpowered** otherwise — including when the CI is undefined or no
/// baseline of `baseline_kind` exists, since an unprovable win is not a win.
/// Both lists are returned sorted.
pub fn partition_certified(
    cands: &Map<String, Value>,
    trial_certified: &std::collections::HashSet<String>,
    baseline_kind: &str,
    cluster_of: &dyn Fn(&str) -> String,
    min_effect: f64,
) -> (Vec<String>, Vec<String>) {
    let baseline = cands
        .iter()
        .find(|(_, c)| c.get("kind").and_then(Value::as_str) == Some(baseline_kind))
        .map(|(_, v)| v);
    let mut ids: Vec<&String> = trial_certified.iter().collect();
    ids.sort();
    let mut certified = Vec::new();
    let mut underpowered = Vec::new();
    for cid in ids {
        let significant = match (cands.get(cid), baseline) {
            (Some(c), Some(nb)) => passes_significance(c, nb, cluster_of, min_effect),
            _ => false,
        };
        if significant {
            certified.push(cid.clone());
        } else {
            underpowered.push(cid.clone());
        }
    }
    (certified, underpowered)
}

/// Pick the certification baseline kind for a candidate set (055): the
/// incumbent (the deployed config) when one was run, else the null floor.
/// "Provably beats what we ship" supersedes "provably beats silence" whenever an
/// incumbent reference is present.
pub fn certification_baseline_kind(cands: &Map<String, Value>) -> &'static str {
    if cands
        .values()
        .any(|c| c.get("kind").and_then(Value::as_str) == Some("incumbent"))
    {
        "incumbent"
    } else {
        "null"
    }
}

/// Split certified candidates into `(reliable, demoted)` by the reliability
/// gate (backlog 056): a candidate is **reliable** only when its pass^k at `k`
/// — estimated at reward floor `consistency_floor` — is defined and at least
/// `reliability_floor`. A high mean reward over a config that fails most of its
/// runs is not deployable (τ-bench, arXiv 2605.10516), so a certified-but-
/// unreliable candidate is *demoted* out of the recommendation set.
///
/// `reliability_floor <= 0.0` disables the gate (every input is reliable), so
/// the default preserves pre-056 behaviour. A candidate whose pass^k is
/// undefined (fewer than `k` trials) fails a positive floor — an unprovable
/// reliability is not a reliability. Both lists are returned sorted.
pub fn partition_reliable(
    cands: &Map<String, Value>,
    certified: &std::collections::HashSet<String>,
    consistency_floor: f64,
    k: usize,
    reliability_floor: f64,
) -> (Vec<String>, Vec<String>) {
    let mut ids: Vec<&String> = certified.iter().collect();
    ids.sort();
    let mut reliable = Vec::new();
    let mut demoted = Vec::new();
    for cid in ids {
        let ok = reliability_floor <= 0.0
            || cands
                .get(cid)
                .map(|c| candidate_consistency(c, consistency_floor))
                .and_then(|con| con.pass_hat_k(k))
                .is_some_and(|p| p >= reliability_floor);
        if ok {
            reliable.push(cid.clone());
        } else {
            demoted.push(cid.clone());
        }
    }
    (reliable, demoted)
}

/// Compute the reward-delta CI for every certified candidate against the
/// reference whose `kind` is `baseline_kind` (e.g. `"null"` or `"incumbent"`).
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
        "\n## Reward delta vs baseline (95% CI)\n\nCluster-robust 95% CI on (candidate − `{baseline_id}`) mean reward, tasks clustered by source repo, using t_(G−1) critical values — honest with few clusters (a 2-repo arena gives df=1, t=12.7, so it certifies almost nothing). A CI that excludes 0 is an improvement over the selected baseline at 95% confidence. `clstr→95%` is the power note (039 child-5): the number of independent clusters (tasks today, source repos once 040 lands labels) at which the *observed* effect's CI is expected to just reach 0 — compare it to n_clusters. Adding tasks within existing clusters does not shrink the SE; clusters do.\n\n"
    ));
    s.push_str("| candidate | Δ reward | 95% CI | n_tasks | n_clusters | clstr→95% | sig |\n");
    s.push_str("|---|---|---|---|---|---|---|\n");
    for (cid, ci) in cis {
        let need = ci
            .min_clusters_to_significance()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".to_string());
        s.push_str(&format!(
            "| {cid} | {:+.4} | [{:+.4}, {:+.4}] | {} | {} | {need} | {} |\n",
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

// ---------------------------------------------------------------------------
// Backlog 039 child-3: per-candidate consistency (reliability), reported
// separately from mean reward. Mean hides reliability — a reviewer right 60% of
// the time is not shippable at any mean (τ-bench / Sierra; "Consistency as a
// Testable Property", arXiv 2605.10516). We report the pass rate at a reward
// floor and pass^k, the chance that all k independent trials clear the floor.
// ---------------------------------------------------------------------------

/// A candidate's reliability at a reward floor, over all its trials.
#[derive(Debug, Clone, PartialEq)]
pub struct Consistency {
    /// Total trials counted (across every task).
    pub n_trials: usize,
    /// Trials whose reward reached the floor.
    pub passes: usize,
    /// The reward floor a trial must reach to count as a pass.
    pub floor: f64,
    /// Pass rate `passes / n_trials` (pass^1), rounded to 4 dp. 0 when no trials.
    pub rate: f64,
}

impl Consistency {
    /// pass^k: the probability that all `k` independent trials clear the floor,
    /// estimated unbiasedly from the observed trials as `C(passes,k)/C(n,k)`.
    /// `None` when `k > n_trials` (not enough trials to estimate). `Some(1.0)`
    /// for `k == 0`; `Some(0.0)` once `passes < k`.
    pub fn pass_hat_k(&self, k: usize) -> Option<f64> {
        pass_hat_k(self.n_trials, self.passes, k)
    }

    /// Serialize for `loop.json`, including pass^k at the given `k`.
    pub fn to_value(&self, k: usize) -> Value {
        let mut m = Map::new();
        m.insert("n_trials".into(), Value::from(self.n_trials as u64));
        m.insert("passes".into(), Value::from(self.passes as u64));
        m.insert("floor".into(), Value::from(self.floor));
        m.insert("rate".into(), Value::from(self.rate));
        m.insert("pass_k_k".into(), Value::from(k as u64));
        m.insert(
            "pass_k".into(),
            self.pass_hat_k(k)
                .map(|p| Value::from(round_half_even(p, 4)))
                .unwrap_or(Value::Null),
        );
        Value::Object(m)
    }
}

/// `pass^k = C(c,k)/C(n,k)`, computed as the product `Π_{i<k} (c−i)/(n−i)` to
/// avoid binomial overflow. The unbiased estimator (τ-bench) of "all k
/// independent draws succeed" given `c` successes in `n` trials.
pub fn pass_hat_k(n: usize, c: usize, k: usize) -> Option<f64> {
    if c > n {
        return None; // more successes than trials is undefined input
    }
    if k == 0 {
        return Some(1.0);
    }
    if k > n {
        return None;
    }
    if c < k {
        return Some(0.0);
    }
    let mut p = 1.0_f64;
    for i in 0..k {
        p *= (c - i) as f64 / (n - i) as f64;
    }
    Some(p)
}

/// A candidate's consistency at `floor`: count every trial across its tasks and
/// the fraction that reached the floor. Consumes the `report::aggregate` shape
/// (`{"tasks": {task_id: [reward, ...]}}`).
pub fn candidate_consistency(candidate: &Value, floor: f64) -> Consistency {
    let mut n_trials = 0usize;
    let mut passes = 0usize;
    if let Some(tasks) = candidate.get("tasks").and_then(Value::as_object) {
        for v in tasks.values() {
            if let Some(arr) = v.as_array() {
                for r in arr.iter().filter_map(Value::as_f64) {
                    n_trials += 1;
                    if r >= floor {
                        passes += 1;
                    }
                }
            }
        }
    }
    let rate = if n_trials == 0 {
        0.0
    } else {
        round_half_even(passes as f64 / n_trials as f64, 4)
    };
    Consistency {
        n_trials,
        passes,
        floor,
        rate,
    }
}

/// Render the per-candidate reliability section for `report.md`. Empty string
/// when there are no rows.
pub fn consistency_markdown(rows: &[(String, Consistency)], k: usize) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let floor = rows[0].1.floor;
    let mut s = String::new();
    s.push_str(&format!(
        "\n## Reliability (pass rate at reward ≥ {floor:.2})\n\nFraction of trials that reach the floor, and pass^{k} — the chance all {k} independent trials reach it. Reliability, reported separately from mean reward: a high mean with low pass^{k} is not deployable (τ-bench; arXiv 2605.10516). Lower `--consistency-floor` to discriminate mid-tier candidates.\n\n"
    ));
    s.push_str(&format!("| candidate | n | pass≥{floor:.2} | pass^{k} |\n"));
    s.push_str("|---|---|---|---|\n");
    for (cid, c) in rows {
        let pk = c
            .pass_hat_k(k)
            .map(|p| format!("{p:.4}"))
            .unwrap_or_else(|| "n/a".to_string());
        s.push_str(&format!(
            "| {cid} | {} | {:.4} | {pk} |\n",
            c.n_trials, c.rate
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// Backlog 039 child-4: basin-trap / trajectory-divergence detector. The search
// is single-population reflective hill-climbing with no basin escape — so its
// answer can depend on the RNG seed. Run it from ≥2 seeds and compare the
// certified tops: if different seeds crown different compositions whose rewards
// differ by more than the pooled noise, the search is seed-trapped, not robust.
// ---------------------------------------------------------------------------

/// The certified top of one seed trajectory.
#[derive(Debug, Clone, PartialEq)]
pub struct RunTop {
    /// Run label (dir name or seed) for reporting.
    pub label: String,
    /// Recommended candidate id.
    pub top_id: String,
    /// Its composition hash — the identity we compare across seeds. Empty when
    /// the run did not record one (then convergence cannot be asserted).
    pub top_hash: String,
    /// Its reward estimate.
    pub reward: f64,
    /// Standard error of that reward. `None` when the run carries no CI data
    /// (e.g. a run produced before 039 child-1) — distinct from a genuine 0,
    /// and it makes the significance test indeterminate rather than trivially
    /// "significant".
    pub se: Option<f64>,
}

/// Verdict of comparing certified tops across seed trajectories.
#[derive(Debug, Clone, PartialEq)]
pub struct BasinVerdict {
    pub n_runs: usize,
    /// Distinct winning composition hashes (non-empty) across the runs.
    pub distinct_winners: usize,
    /// All seeds crowned the same composition (and all identities are present).
    pub converged: bool,
    /// Any run is missing a composition hash — convergence cannot be asserted.
    pub missing_identity: bool,
    /// Reward gap between the best- and worst-scoring *distinct compositions*
    /// (each reduced to its best-scoring run), not the global best/worst run.
    pub reward_gap: f64,
    /// Pooled SE of that pair. `None` when either side lacks CI data — then the
    /// gap cannot be tested against noise.
    pub pooled_se: Option<f64>,
    /// Whether the gap exceeds the 95% pooled-noise band. `None` when untestable
    /// (missing SE) — never silently treated as significant.
    pub gap_significant: Option<bool>,
    /// Basin trap: seeds crown different compositions AND the gap is testably
    /// beyond pooled noise (`gap_significant == Some(true)`).
    pub flag: bool,
}

/// Compare the certified tops of ≥2 seed trajectories. `None` with fewer than 2
/// runs (a robustness check needs at least two).
///
/// Each distinct winning composition is reduced to its best-scoring run, so the
/// reward gap measures *between-composition* divergence, not within-composition
/// trial noise. Flags a basin trap only when the seeds crown different
/// compositions whose gap testably exceeds the pooled 95% noise band. Different
/// winners within noise → non-converged "equivalent optima" (no flag); missing
/// CI data → untestable (no flag, surfaced via `gap_significant = None`); a
/// missing composition hash → `missing_identity` (never reported as converged).
pub fn basin_divergence(tops: &[RunTop]) -> Option<BasinVerdict> {
    if tops.len() < 2 {
        return None;
    }
    let missing_identity = tops.iter().any(|t| t.top_hash.is_empty());

    // Reduce each distinct composition to its best-scoring run.
    let mut best_by_hash: std::collections::BTreeMap<&str, &RunTop> =
        std::collections::BTreeMap::new();
    for t in tops {
        if t.top_hash.is_empty() {
            continue;
        }
        best_by_hash
            .entry(t.top_hash.as_str())
            .and_modify(|cur| {
                if t.reward > cur.reward {
                    *cur = t;
                }
            })
            .or_insert(t);
    }
    let distinct_winners = best_by_hash.len();
    let converged = !missing_identity && distinct_winners == 1;

    let reps: Vec<&RunTop> = best_by_hash.values().copied().collect();
    let (reward_gap, pooled_se, gap_significant) = if reps.len() < 2 {
        (0.0, None, None)
    } else {
        let best = reps
            .iter()
            .max_by(|a, b| a.reward.total_cmp(&b.reward))
            .expect("reps >= 2");
        let worst = reps
            .iter()
            .min_by(|a, b| a.reward.total_cmp(&b.reward))
            .expect("reps >= 2");
        let gap = best.reward - worst.reward;
        match (best.se, worst.se) {
            (Some(sb), Some(sw)) => {
                let ps = (sb * sb + sw * sw).sqrt();
                (gap, Some(ps), Some(gap > Z_95 * ps))
            }
            _ => (gap, None, None), // missing CI data → cannot test against noise
        }
    };
    let flag = !converged && gap_significant == Some(true);

    Some(BasinVerdict {
        n_runs: tops.len(),
        distinct_winners,
        converged,
        missing_identity,
        reward_gap: round_half_even(reward_gap, 4),
        pooled_se: pooled_se.map(|s| round_half_even(s, 6)),
        gap_significant,
        flag,
    })
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
    fn t_975_matches_the_standard_critical_values() {
        assert_eq!(t_975(1), 12.706); // df=1 (a 2-cluster arena) — huge
        assert_eq!(t_975(2), 4.303);
        assert_eq!(t_975(6), 2.447); // df=6 (7 clusters)
        assert_eq!(t_975(30), 2.042);
        // df > 30: Cornish–Fisher refinement, close to the true t, above 1.96.
        assert!((t_975(31) - 2.0365).abs() < 0.001); // true ≈ 2.0395
        assert!(t_975(31) > Z_95);
        assert!((t_975(100_000) - Z_95).abs() < 0.001); // → normal limit
    }

    #[test]
    fn singleton_clusters_reduce_to_standard_se_of_the_mean() {
        // Per-task deltas 0.2, 0.4, 0.6 (baseline all 0.0).
        // point = 0.4; standard SE = stdev/√3 = 0.2/√3 = 0.115470 (the headline
        // property — unchanged). The CI uses t_{G−1}=t_2=4.303, so with only 3
        // clusters it honestly spans 0: 0.4 ± 4.303·0.11547 = [−0.0969, 0.8969].
        let c = cand(&[("a", &[0.2]), ("b", &[0.4]), ("c", &[0.6])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let ci = reward_delta_ci(&c, &b, &singleton).expect("defined");
        assert_eq!(ci.point, 0.4);
        assert_eq!(ci.se, 0.11547);
        assert_eq!(ci.lo, -0.0969);
        assert_eq!(ci.hi, 0.8969);
        assert_eq!(ci.n_tasks, 3);
        assert_eq!(ci.n_clusters, 3);
        assert!(!ci.excludes_zero); // 3 clusters is too few to clear 0
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
        // With G=2 the CI uses t_1=12.706, so it spans a very wide range.
        assert_eq!(clustered.point, 0.4);
        assert_eq!(clustered.se, 0.266667);
        assert_eq!(clustered.lo, -2.9883);
        assert_eq!(clustered.hi, 3.7883);
        assert_eq!(clustered.n_clusters, 2);
        // Pooling the correlated repo widened the SE and the CI spans 0.
        assert!(clustered.se > per_task.se);
        assert!(!clustered.excludes_zero);
    }

    #[test]
    fn significance_uses_unrounded_bounds_not_the_displayed_4dp() {
        // Two singleton clusters → t_1=12.706, and SE = |d1−d2|/2 = 0.01, so the
        // raw lower bound is 0.127065 − 12.706·0.01 = +5e-6 — strictly above 0, a
        // genuine win — but it rounds to 0.0000 for display. excludes_zero must
        // reflect the true CI, not the rounded bound.
        let c = cand(&[("a", &[0.137065]), ("b", &[0.117065])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0])]);
        let ci = reward_delta_ci(&c, &b, &singleton).expect("defined");
        assert_eq!(ci.lo, 0.0); // displays as +0.0000
        assert!(ci.excludes_zero); // …yet the unrounded CI is strictly above 0
    }

    #[test]
    fn passes_significance_requires_the_ci_to_clear_the_mde() {
        // Perfectly consistent +0.5 (zero variance → significant at any cluster
        // count): lower bound is exactly 0.5, independent of the t multiplier.
        let c = cand(&[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        assert!(passes_significance(&c, &b, &singleton, 0.0)); // beats the floor
        assert!(passes_significance(&c, &b, &singleton, 0.4)); // lower bound > 0.4
        assert!(!passes_significance(&c, &b, &singleton, 0.5)); // not > 0.5
    }

    #[test]
    fn passes_significance_false_when_ci_spans_the_mde() {
        // Correlated repo widens the CI to span 0 (from the clustering test).
        let c = cand(&[("a", &[0.6]), ("b", &[0.6]), ("c", &[0.0])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let by_repo = |t: &str| match t {
            "a" | "b" => "R1".to_string(),
            _ => "R2".to_string(),
        };
        assert!(!passes_significance(&c, &b, &by_repo, 0.0));
    }

    #[test]
    fn passes_significance_uses_the_raw_lower_bound_at_the_display_boundary() {
        // Raw lower bound +5e-6 displays as 0.0000 but is a genuine win.
        let c = cand(&[("a", &[0.137065]), ("b", &[0.117065])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0])]);
        assert!(passes_significance(&c, &b, &singleton, 0.0));
    }

    #[test]
    fn passes_significance_false_for_an_undefined_ci() {
        // One common task → CI undefined → cannot prove a win.
        let c = cand(&[("a", &[0.9]), ("x", &[0.9])]);
        let b = cand(&[("a", &[0.0])]);
        assert!(!passes_significance(&c, &b, &singleton, 0.0));
    }

    #[test]
    fn partition_certified_splits_by_the_significance_gate() {
        let cands = cands_map(&[
            (
                "null",
                "null",
                &[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])],
            ),
            // sig: perfectly consistent +0.5 (zero variance → CI excludes 0).
            ("sig", "pi", &[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])]),
            // not sig: deltas spread across 0 → CI spans 0.
            (
                "weak",
                "pi",
                &[("a", &[0.6]), ("b", &[0.0]), ("c", &[-0.2])],
            ),
        ]);
        let trial: HashSet<String> = ["sig".to_string(), "weak".to_string()]
            .into_iter()
            .collect();
        let (certified, underpowered) =
            partition_certified(&cands, &trial, "null", &singleton, 0.0);
        assert_eq!(certified, vec!["sig".to_string()]);
        assert_eq!(underpowered, vec!["weak".to_string()]);
    }

    #[test]
    fn partition_certified_raising_the_mde_demotes_a_candidate() {
        let cands = cands_map(&[
            (
                "null",
                "null",
                &[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])],
            ),
            ("sig", "pi", &[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])]),
        ]);
        let trial: HashSet<String> = ["sig".to_string()].into_iter().collect();
        // Consistent +0.5 → lower bound 0.5: certifies at MDE 0.4, demoted at 0.6.
        assert_eq!(
            partition_certified(&cands, &trial, "null", &singleton, 0.4).0,
            vec!["sig".to_string()]
        );
        assert_eq!(
            partition_certified(&cands, &trial, "null", &singleton, 0.6).1,
            vec!["sig".to_string()]
        );
    }

    #[test]
    fn partition_certified_without_a_baseline_is_all_underpowered() {
        let cands = cands_map(&[("sig", "pi", &[("a", &[0.5]), ("b", &[0.5])])]);
        let trial: HashSet<String> = ["sig".to_string()].into_iter().collect();
        let (certified, underpowered) =
            partition_certified(&cands, &trial, "null", &singleton, 0.0);
        assert!(certified.is_empty());
        assert_eq!(underpowered, vec!["sig".to_string()]);
    }

    #[test]
    fn partition_reliable_demotes_a_low_pass_k_candidate() {
        // "solid" clears the reward floor on every trial (pass^2 = 1.0); "flaky"
        // clears 2 of 5 (pass^2 = C(2,2)/C(5,2) = 0.1). At a 0.5 floor the flaky
        // one is demoted out of the recommendation set; the solid one survives.
        let cands = cands_map(&[
            ("solid", "pi", &[("a", &[1.0, 1.0, 1.0, 1.0, 1.0])]),
            ("flaky", "pi", &[("a", &[1.0, 1.0, 0.0, 0.0, 0.0])]),
        ]);
        let certified: HashSet<String> = ["solid".to_string(), "flaky".to_string()]
            .into_iter()
            .collect();
        let (reliable, demoted) = partition_reliable(&cands, &certified, 1.0, 2, 0.5);
        assert_eq!(reliable, vec!["solid".to_string()]);
        assert_eq!(demoted, vec!["flaky".to_string()]);
    }

    #[test]
    fn partition_reliable_floor_zero_disables_the_gate() {
        // The default floor (0.0) keeps every certified candidate, preserving
        // pre-056 behaviour even for an all-fail config.
        let cands = cands_map(&[("flaky", "pi", &[("a", &[0.0, 0.0, 0.0])])]);
        let certified: HashSet<String> = ["flaky".to_string()].into_iter().collect();
        let (reliable, demoted) = partition_reliable(&cands, &certified, 1.0, 2, 0.0);
        assert_eq!(reliable, vec!["flaky".to_string()]);
        assert!(demoted.is_empty());
    }

    #[test]
    fn partition_reliable_undefined_pass_k_fails_a_positive_floor() {
        // Only 2 trials but k=5 → pass^5 undefined → not provably reliable, so a
        // positive floor demotes it; the gate-off floor keeps it.
        let cands = cands_map(&[("thin", "pi", &[("a", &[1.0, 1.0])])]);
        let certified: HashSet<String> = ["thin".to_string()].into_iter().collect();
        assert_eq!(
            partition_reliable(&cands, &certified, 1.0, 5, 0.1).1,
            vec!["thin".to_string()]
        );
        assert_eq!(
            partition_reliable(&cands, &certified, 1.0, 5, 0.0).0,
            vec!["thin".to_string()]
        );
    }

    #[test]
    fn partition_reliable_demotes_the_real_cerberus_recommendation() {
        // The 2026-06-23 recommended config (seed2-kimi) reached the reward floor
        // on 17 of 30 trials → pass^5 = C(17,5)/C(30,5) ≈ 0.0434, the run's own
        // reported number. Any deployable floor (0.10) demotes it — yet the run
        // recommended it anyway. This is the bug 056 fixes.
        let mut trials = vec![1.0_f64; 17];
        trials.extend(vec![0.0_f64; 13]);
        let cands = cands_map(&[("seed2", "pi", &[("rs-retry-backoff", trials.as_slice())])]);
        let certified: HashSet<String> = ["seed2".to_string()].into_iter().collect();
        assert_eq!(
            partition_reliable(&cands, &certified, 1.0, 5, 0.10).1,
            vec!["seed2".to_string()]
        );
        assert_eq!(
            partition_reliable(&cands, &certified, 1.0, 5, 0.04).0,
            vec!["seed2".to_string()]
        );
    }

    #[test]
    fn certification_baseline_kind_prefers_incumbent_then_null() {
        let with_inc = cands_map(&[
            ("null", "null", &[("a", &[0.0])]),
            ("inc", "incumbent", &[("a", &[0.5])]),
            ("cand", "pi", &[("a", &[0.7])]),
        ]);
        assert_eq!(certification_baseline_kind(&with_inc), "incumbent");
        let no_inc = cands_map(&[
            ("null", "null", &[("a", &[0.0])]),
            ("cand", "pi", &[("a", &[0.7])]),
        ]);
        assert_eq!(certification_baseline_kind(&no_inc), "null");
    }

    #[test]
    fn certifying_vs_incumbent_rejects_a_beats_null_not_incumbent_candidate() {
        // null=0, incumbent=0.5, weak=0.5 (ties the incumbent), strong=0.9.
        let cands = cands_map(&[
            (
                "null",
                "null",
                &[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])],
            ),
            (
                "inc",
                "incumbent",
                &[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])],
            ),
            ("weak", "pi", &[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])]),
            (
                "strong",
                "pi",
                &[("a", &[0.9]), ("b", &[0.9]), ("c", &[0.9])],
            ),
        ]);
        let trial: HashSet<String> = ["weak".to_string(), "strong".to_string()]
            .into_iter()
            .collect();
        // vs the incumbent: only `strong` (+0.4, consistent) certifies; `weak`
        // ties the incumbent and is rejected — it beats silence, not what we ship.
        let (cert_inc, _) = partition_certified(&cands, &trial, "incumbent", &singleton, 0.0);
        assert_eq!(cert_inc, vec!["strong".to_string()]);
        // vs the null floor both clear — the old, weaker bar 055 replaces.
        let (cert_null, _) = partition_certified(&cands, &trial, "null", &singleton, 0.0);
        assert_eq!(cert_null, vec!["strong".to_string(), "weak".to_string()]);
    }

    #[test]
    fn certified_delta_cis_difference_against_the_incumbent() {
        let cands = cands_map(&[
            (
                "inc",
                "incumbent",
                &[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])],
            ),
            (
                "strong",
                "pi",
                &[("a", &[0.9]), ("b", &[0.9]), ("c", &[0.9])],
            ),
        ]);
        let certified: HashSet<String> = ["strong".to_string()].into_iter().collect();
        let (base, cis) = certified_delta_cis(&cands, &certified, "incumbent", &singleton);
        assert_eq!(base.as_deref(), Some("inc"));
        assert_eq!(cis.len(), 1);
        assert_eq!(cis[0].0, "strong");
        assert_eq!(cis[0].1.point, 0.4); // 0.9 − 0.5, the win over the incumbent
    }

    #[test]
    fn min_clusters_to_significance_is_small_when_already_significant() {
        // Perfectly consistent +0.5 (se 0) is significant at the floor of 2
        // clusters, so the power note returns 2 (≤ current).
        let c = cand(&[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let ci = reward_delta_ci(&c, &b, &singleton).unwrap();
        assert!(ci.excludes_zero);
        assert_eq!(ci.min_clusters_to_significance(), Some(2));
    }

    #[test]
    fn min_clusters_to_significance_scales_the_cluster_count_not_tasks() {
        // +0.4 effect, se 0.266667 over G=2 clusters (3 tasks). The power note
        // iterates G with the matching t_{G−1} critical value — at G=6 the
        // projected t-CI just clears 0 — so it needs 6 clusters, NOT a function of
        // the 3 tasks. Adding tasks to these 2 repos would not help.
        let c = cand(&[("a", &[0.6]), ("b", &[0.6]), ("c", &[0.0])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let by_repo = |t: &str| match t {
            "a" | "b" => "R1".to_string(),
            _ => "R2".to_string(),
        };
        let ci = reward_delta_ci(&c, &b, &by_repo).unwrap();
        assert!(!ci.excludes_zero);
        assert_eq!(ci.n_clusters, 2);
        assert_eq!(ci.min_clusters_to_significance(), Some(6));
    }

    #[test]
    fn min_clusters_to_significance_is_none_for_a_non_positive_effect() {
        // Candidate is worse than baseline → no positive effect to certify.
        let c = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let b = cand(&[("a", &[0.2]), ("b", &[0.4]), ("c", &[0.6])]);
        let ci = reward_delta_ci(&c, &b, &singleton).unwrap();
        assert!(ci.point < 0.0);
        assert_eq!(ci.min_clusters_to_significance(), None);
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
        // Consistently worse by 0.5 (zero variance) → point −0.5, CI entirely
        // below 0 regardless of the t multiplier.
        let c = cand(&[("a", &[0.2]), ("b", &[0.2]), ("c", &[0.2])]);
        let b = cand(&[("a", &[0.7]), ("b", &[0.7]), ("c", &[0.7])]);
        let ci = reward_delta_ci(&c, &b, &singleton).expect("defined");
        assert_eq!(ci.point, -0.5);
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
        // Consistent +0.5 → significant, for a clean excludes_zero=true.
        let c = cand(&[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])]);
        let b = cand(&[("a", &[0.0]), ("b", &[0.0]), ("c", &[0.0])]);
        let ci = reward_delta_ci(&c, &b, &singleton).unwrap();
        let v = ci.to_value("null");
        assert_eq!(v["baseline"], json!("null"));
        assert_eq!(v["point"], json!(0.5));
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
                &[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])],
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
        assert_eq!(cis[0].1.point, 0.5);
        assert!(cis[0].1.excludes_zero); // consistent +0.5 clears 0
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
    fn pass_hat_k_is_the_combinatorial_all_k_succeed_rate() {
        assert_eq!(pass_hat_k(5, 3, 2), Some(0.3)); // C(3,2)/C(5,2) = 3/10
        assert_eq!(pass_hat_k(5, 5, 3), Some(1.0)); // all trials pass
        assert_eq!(pass_hat_k(5, 2, 3), Some(0.0)); // fewer passes than k
        assert_eq!(pass_hat_k(3, 3, 0), Some(1.0)); // k=0 is vacuously certain
        assert_eq!(pass_hat_k(2, 2, 3), None); // k > n: not enough trials
        assert_eq!(pass_hat_k(5, 7, 2), None); // c > n: undefined input, never > 1
    }

    #[test]
    fn candidate_consistency_counts_trials_reaching_the_floor() {
        // 4 trials, 3 reach 1.0 → rate 0.75; pass^2 = (3/4)(2/3) = 0.5.
        let c = cand(&[("a", &[1.0, 0.5]), ("b", &[1.0, 1.0])]);
        let con = candidate_consistency(&c, 1.0);
        assert_eq!(con.n_trials, 4);
        assert_eq!(con.passes, 3);
        assert_eq!(con.rate, 0.75);
        assert_eq!(con.pass_hat_k(2), Some(0.5));
    }

    #[test]
    fn candidate_consistency_floor_changes_the_pass_count() {
        let c = cand(&[("a", &[0.8, 0.6]), ("b", &[0.4, 0.9])]);
        assert_eq!(candidate_consistency(&c, 1.0).passes, 0); // none perfect
        assert_eq!(candidate_consistency(&c, 0.7).passes, 2); // 0.8, 0.9
        assert_eq!(candidate_consistency(&c, 0.0).passes, 4); // all
    }

    #[test]
    fn consistency_markdown_renders_a_reliability_row() {
        let c = cand(&[("a", &[1.0, 0.5]), ("b", &[1.0, 1.0])]);
        let rows = vec![("cand-x".to_string(), candidate_consistency(&c, 1.0))];
        let md = consistency_markdown(&rows, 2);
        assert!(md.contains("## Reliability (pass rate at reward ≥ 1.00)"));
        assert!(md.contains("| cand-x | 4 | 0.7500 | 0.5000 |"));
        assert_eq!(consistency_markdown(&[], 2), "");
    }

    #[test]
    fn consistency_serializes_with_pass_k() {
        let c = cand(&[("a", &[1.0, 0.5]), ("b", &[1.0, 1.0])]);
        let v = candidate_consistency(&c, 1.0).to_value(2);
        assert_eq!(v["n_trials"], json!(4));
        assert_eq!(v["passes"], json!(3));
        assert_eq!(v["rate"], json!(0.75));
        assert_eq!(v["pass_k"], json!(0.5));
        assert_eq!(v["pass_k_k"], json!(2));
    }

    fn top(label: &str, hash: &str, reward: f64, se: f64) -> RunTop {
        RunTop {
            label: label.to_string(),
            top_id: format!("cand-{hash}"),
            top_hash: hash.to_string(),
            reward,
            se: Some(se),
        }
    }

    fn top_no_ci(label: &str, hash: &str, reward: f64) -> RunTop {
        RunTop {
            label: label.to_string(),
            top_id: format!("cand-{hash}"),
            top_hash: hash.to_string(),
            reward,
            se: None,
        }
    }

    #[test]
    fn basin_converged_when_all_seeds_crown_the_same_composition() {
        let tops = vec![top("s1", "H", 0.80, 0.05), top("s2", "H", 0.78, 0.05)];
        let v = basin_divergence(&tops).unwrap();
        assert!(v.converged);
        assert_eq!(v.distinct_winners, 1);
        assert!(!v.flag);
    }

    #[test]
    fn basin_flags_different_winners_with_a_significant_reward_gap() {
        // gap 0.40, pooled_se √(0.05²+0.05²)=0.0707, 1.96·=0.139 < 0.40 → trap.
        let tops = vec![top("s1", "HX", 0.90, 0.05), top("s2", "HY", 0.50, 0.05)];
        let v = basin_divergence(&tops).unwrap();
        assert!(!v.converged);
        assert_eq!(v.distinct_winners, 2);
        assert_eq!(v.gap_significant, Some(true));
        assert!(v.flag);
    }

    #[test]
    fn basin_different_winners_within_noise_is_not_a_hard_trap() {
        // gap 0.02, pooled_se √(0.2²+0.2²)=0.283, 1.96·=0.554 > 0.02 → equivalent.
        let tops = vec![top("s1", "HX", 0.80, 0.20), top("s2", "HY", 0.78, 0.20)];
        let v = basin_divergence(&tops).unwrap();
        assert!(!v.converged); // seeds disagree on the winner
        assert_eq!(v.gap_significant, Some(false)); // but not beyond pooled noise
        assert!(!v.flag); // equivalent optima, not a trap
    }

    #[test]
    fn basin_detects_a_divergent_third_seed() {
        let tops = vec![
            top("s1", "H", 0.85, 0.04),
            top("s2", "H", 0.83, 0.04),
            top("s3", "OTHER", 0.50, 0.04),
        ];
        let v = basin_divergence(&tops).unwrap();
        assert_eq!(v.n_runs, 3);
        assert_eq!(v.distinct_winners, 2);
        assert!(v.flag);
    }

    #[test]
    fn basin_compares_distinct_compositions_not_the_global_best_worst() {
        // HX best is 0.82, HY is 0.80 (gap 0.02, within noise → equivalent). A
        // same-composition outlier HX@0.50 must NOT widen the between-winner gap
        // (global best/worst would be 0.82 vs 0.50 = 0.32 and falsely flag).
        let tops = vec![
            top("s1", "HX", 0.82, 0.01),
            top("s2", "HY", 0.80, 0.01),
            top("s3", "HX", 0.50, 0.01),
        ];
        let v = basin_divergence(&tops).unwrap();
        assert_eq!(v.distinct_winners, 2);
        assert_eq!(v.reward_gap, 0.02); // HX-best vs HY, not vs HX@0.50
        assert_eq!(v.gap_significant, Some(false));
        assert!(!v.flag);
    }

    #[test]
    fn basin_without_ci_data_is_untestable_not_a_trap() {
        // Different winners but no SE (pre-039 runs): cannot test the gap against
        // noise, so it must NOT silently flag a trap.
        let tops = vec![top_no_ci("s1", "HX", 0.90), top_no_ci("s2", "HY", 0.50)];
        let v = basin_divergence(&tops).unwrap();
        assert!(!v.converged);
        assert_eq!(v.distinct_winners, 2);
        assert_eq!(v.pooled_se, None);
        assert_eq!(v.gap_significant, None);
        assert!(!v.flag); // untestable, not a hard trap
    }

    #[test]
    fn basin_missing_composition_hash_is_never_reported_robust() {
        // Blank hashes (missing identity) must not collapse to "converged".
        let tops = vec![top("s1", "", 0.90, 0.05), top("s2", "", 0.20, 0.05)];
        let v = basin_divergence(&tops).unwrap();
        assert!(v.missing_identity);
        assert!(!v.converged); // no false robustness from missing data
        assert_eq!(v.distinct_winners, 0);
        assert!(!v.flag);
    }

    #[test]
    fn basin_needs_at_least_two_runs() {
        assert!(basin_divergence(&[top("s1", "H", 0.8, 0.05)]).is_none());
        assert!(basin_divergence(&[]).is_none());
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
                &[("a", &[0.5]), ("b", &[0.5]), ("c", &[0.5])],
            ),
        ]);
        let certified: HashSet<String> = ["cand-x".to_string()].into_iter().collect();
        let (base, cis) = certified_delta_cis(&cands, &certified, "null", &singleton);
        let md = delta_ci_markdown(base.as_deref().unwrap(), &cis);
        assert!(md.contains("## Reward delta vs baseline (95% CI)"));
        // Consistent +0.5 → zero-width CI, significant, power note 2 clusters.
        assert!(md.contains("| cand-x | +0.5000 | [+0.5000, +0.5000] | 3 | 3 | 2 | ✓ |"));
        // Nothing to render → empty string, safe to append unconditionally.
        assert_eq!(delta_ci_markdown("null", &[]), "");
    }
}
