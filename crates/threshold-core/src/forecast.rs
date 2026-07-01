//! Offline cost/scale forecast for a search, before any spend (041).
//!
//! `threshold run --estimate` projects how many trials a search will run and what
//! it could cost, then exits — zero trials, no `runs/` directory. The operator
//! can size a search before committing budget instead of learning the spend
//! only post-hoc.
//!
//! Cost is projected ONLY when the taskspec declares
//! `[budget].max_cost_per_trial_usd`; otherwise the projection is "unknown"
//! (AGENTS: unknown cost is null, never an estimate; VISION: no cost claim
//! without recorded usage or an honest "unknown"). The per-trial ceiling is a
//! worst-case bound, not a prediction — the forecast labels it as such.

/// The inputs that drive trial count and cost, lifted from the run flags + the
/// frozen arena split + the taskspec budget. Pure data; no I/O.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForecastInputs {
    /// `--max-candidates`: the search's candidate budget.
    pub max_candidates: usize,
    /// Reference candidates that run alongside the search (null, oracle,
    /// one-shot probe) — they consume trials too.
    pub reference_kinds: usize,
    /// Number of search tasks (train + validation).
    pub n_search_tasks: usize,
    /// `--trials`: trials per (candidate × search task).
    pub trials: u32,
    /// `--certify-top`: how many top candidates race to certification.
    pub certify_top: usize,
    /// Number of holdout tasks the certified racers replay against.
    pub n_holdout: usize,
    /// `--certify-trials`: trials per (certified candidate × holdout task).
    pub certify_trials: u32,
    /// Whether the taskspec declares an `[incumbent]` (055). When set, the
    /// incumbent reference runs `certify_trials` deep on every task (search +
    /// holdout), so its trials are projected separately from the single-shot
    /// references.
    pub has_incumbent: bool,
    /// `[budget].max_cost_per_trial_usd` from the taskspec, when declared.
    pub max_cost_per_trial_usd: Option<f64>,
}

/// A projected search: the trial-count breakdown and an optional cost ceiling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Forecast {
    /// Trials spent on the candidate search: `max_candidates × n_search_tasks ×
    /// trials`. Candidates run `--trials`-deep per search task.
    pub candidate_trials: u64,
    /// Trials spent on the reference candidates (null floor, oracle ceiling, and
    /// the one-shot saturation probe): `reference_kinds × n_search_tasks × 1`.
    /// References run ONCE per task by design (the stage-1 rig check and the
    /// stage-1b probe are single-shot), not `--trials`-deep.
    pub reference_trials: u64,
    /// Trials spent across all search stages: `candidate_trials +
    /// reference_trials`.
    pub search_trials: u64,
    /// Trials spent certifying the top candidates on the holdout.
    /// `certify_top × n_holdout × certify_trials`.
    pub certify_trials_total: u64,
    /// Trials spent on the incumbent baseline (055): `certify_trials ×
    /// (n_search_tasks + n_holdout)` when declared, else 0. The incumbent runs
    /// every task `certify_trials`-deep, unlike the single-shot references.
    pub incumbent_trials: u64,
    /// Total projected trials.
    pub total_trials: u64,
    /// Worst-case cost ceiling = `total_trials × max_cost_per_trial_usd`, or
    /// `None` when the taskspec declares no per-trial ceiling.
    pub max_cost_usd: Option<f64>,
}

/// Project a search's scale and cost ceiling from its inputs. Pure arithmetic.
///
/// Trial count = `max_candidates × |search_tasks| × trials` (candidates run
/// `--trials`-deep) `+ reference_kinds × |search_tasks| × 1` (references run
/// once per task — single-shot rig check + saturation probe)
/// `+ certify_top × |holdout| × certify_trials`. Cost is the worst case
/// (`total_trials × per-trial ceiling`) and only when the ceiling is declared.
pub fn forecast(inputs: &ForecastInputs) -> Forecast {
    let candidate_trials =
        inputs.max_candidates as u64 * inputs.n_search_tasks as u64 * inputs.trials as u64;
    // References run once per task, not `trials`-deep — see `reference_trials`.
    let reference_trials = inputs.reference_kinds as u64 * inputs.n_search_tasks as u64;
    let search_trials = candidate_trials + reference_trials;
    let certify_trials_total =
        inputs.certify_top as u64 * inputs.n_holdout as u64 * inputs.certify_trials as u64;
    // The incumbent (055) runs every task (search + holdout) `certify_trials`-deep.
    let incumbent_trials = if inputs.has_incumbent {
        inputs.certify_trials as u64 * (inputs.n_search_tasks as u64 + inputs.n_holdout as u64)
    } else {
        0
    };
    let total_trials = search_trials + certify_trials_total + incumbent_trials;
    let max_cost_usd = inputs
        .max_cost_per_trial_usd
        .map(|ceiling| total_trials as f64 * ceiling);
    Forecast {
        candidate_trials,
        reference_trials,
        search_trials,
        certify_trials_total,
        incumbent_trials,
        total_trials,
        max_cost_usd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cerberus-reviewer shape: 6 search tasks, 3 reference kinds
    /// (null/oracle/one-shot), default flags, a declared per-trial ceiling.
    fn cerberus_like() -> ForecastInputs {
        ForecastInputs {
            max_candidates: 6,
            reference_kinds: 3,
            n_search_tasks: 6,
            trials: 3,
            certify_top: 3,
            n_holdout: 4,
            certify_trials: 5,
            has_incumbent: false,
            max_cost_per_trial_usd: Some(0.50),
        }
    }

    #[test]
    fn trial_count_follows_the_documented_formula() {
        let f = forecast(&cerberus_like());
        // Candidates run `trials`-deep: 6 × 6 × 3 = 108.
        assert_eq!(f.candidate_trials, 108);
        // References run ONCE per task (single-shot rig + probe): 3 × 6 = 18,
        // NOT 3 × 6 × 3. This is the 041 review-blocker fix — references must not
        // be multiplied by `--trials`.
        assert_eq!(f.reference_trials, 18);
        // Search total: 108 + 18 = 126 (was 162 when references were over-counted).
        assert_eq!(f.search_trials, 126);
        // 3 × 4 × 5 = 60 certification trials.
        assert_eq!(f.certify_trials_total, 60);
        assert_eq!(f.total_trials, 186);
    }

    #[test]
    fn cost_ceiling_is_total_trials_times_per_trial_ceiling() {
        let f = forecast(&cerberus_like());
        // 186 trials × $0.50 worst-case ceiling.
        assert_eq!(f.max_cost_usd, Some(93.0));
    }

    #[test]
    fn cost_is_unknown_without_a_declared_ceiling() {
        let mut inputs = cerberus_like();
        inputs.max_cost_per_trial_usd = None;
        let f = forecast(&inputs);
        // Trial count is still projectable; cost is honestly unknown — never 0.
        assert_eq!(f.total_trials, 186);
        assert_eq!(f.max_cost_usd, None);
    }

    #[test]
    fn incumbent_adds_certify_deep_trials_on_every_task() {
        let mut inputs = cerberus_like();
        inputs.has_incumbent = true;
        let f = forecast(&inputs);
        // The incumbent runs certify_trials (5) deep on every task: 5 × (6 + 4) = 50.
        assert_eq!(f.incumbent_trials, 50);
        // Folded into the total: 186 + 50 = 236, and the cost ceiling tracks it.
        assert_eq!(f.total_trials, 236);
        assert_eq!(f.max_cost_usd, Some(118.0));
        // Off by default — no incumbent declared.
        assert_eq!(forecast(&cerberus_like()).incumbent_trials, 0);
    }

    #[test]
    fn no_holdout_means_no_certification_trials() {
        let mut inputs = cerberus_like();
        inputs.n_holdout = 0;
        let f = forecast(&inputs);
        assert_eq!(f.certify_trials_total, 0);
        assert_eq!(f.total_trials, f.search_trials);
    }
}
