//! Live terminal cockpit over a run in flight (backlog 049 roll-up → 050 cockpit).
//!
//! `daedalus view <run-dir>` polls the run dir and reprints a single-screen
//! cockpit — the live companion to the post-run static report
//! ([`crate::report_html`]). The rich post-run surfaces (CI forest, certified
//! verdict) are inherently post-run; the valuable *live* signals are: the
//! candidate roll-up (running mean reward, trials), spend ticking up against the
//! budget cap, the headroom rig ([`Rig`], from `rig.json`), and — the point of
//! the cockpit — the optimizer's hypotheses streamed as it proposes them
//! ([`Hypo`], from `loop.history.jsonl`), so a run is legible *as a search*
//! while it runs, not only at completion.
//!
//! `trials.jsonl` stays the source of truth; the snapshot reuses
//! [`crate::report::aggregate`], so the live view can never drift from the batch
//! report. The rig, hypotheses, and cap are attached via [`Snapshot::with_rig`],
//! [`Snapshot::with_hypotheses`], and [`Snapshot::with_cap`] — each degrades
//! gracefully when its source file is absent. The poll/redraw loop is a thin IO
//! shell in the CLI; everything here is pure and tested.

use std::fmt::Write as _;

use serde_json::Value;

use crate::report;

/// One candidate's live standing.
#[derive(Debug, Clone, PartialEq)]
pub struct CandRow {
    pub id: String,
    pub reward_mean: f64,
    pub trials: u64,
    /// Known spend on this candidate, or `None` when a trial reported unknown cost.
    pub cost: Option<f64>,
    /// A reference (oracle / null / one-shot probe), not a deliverable candidate.
    pub reference: bool,
}

/// The most recent trial to land — the streaming heartbeat.
#[derive(Debug, Clone, PartialEq)]
pub struct LastTrial {
    pub candidate: String,
    pub task: String,
    pub reward: f64,
}

/// The headroom rig: oracle ceiling, null floor, and the one-shot saturation
/// probe — the at-a-glance "is this arena measuring real skill?" panel. Parsed
/// from `rig.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct Rig {
    pub oracle: f64,
    pub null: f64,
    pub probe: Option<f64>,
    pub saturated: bool,
}

/// Parse `rig.json` (`{oracle_mean, null_mean, probe_mean, saturated}`). `None`
/// when the ceiling/floor are absent, so the cockpit degrades gracefully.
pub fn parse_rig(v: &Value) -> Option<Rig> {
    Some(Rig {
        oracle: v.get("oracle_mean").and_then(Value::as_f64)?,
        null: v.get("null_mean").and_then(Value::as_f64)?,
        probe: v.get("probe_mean").and_then(Value::as_f64),
        saturated: v.get("saturated").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// One streamed hypothesis from `loop.history.jsonl`: what the optimizer mutated
/// and whether it stuck. An `error` makes it a hypothesis that never ran.
#[derive(Debug, Clone, PartialEq)]
pub struct Hypo {
    pub generation: u64,
    pub parent_id: String,
    pub slot: String,
    pub parent_reward: Option<f64>,
    pub reward: Option<f64>,
    pub improved: Option<bool>,
    /// `Some` ⇒ the proposal failed and never ran (carries the optimizer error).
    pub error: Option<String>,
}

/// Parse `loop.history.jsonl` rows into the latest `n` hypotheses, oldest-first.
pub fn parse_hypotheses(rows: &[Value], n: usize) -> Vec<Hypo> {
    let start = rows.len().saturating_sub(n);
    rows[start..]
        .iter()
        .map(|r| Hypo {
            generation: r.get("generation").and_then(Value::as_u64).unwrap_or(0),
            parent_id: r
                .get("parent_id")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string(),
            slot: r
                .get("slot_changed")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            parent_reward: r.get("parent_reward_mean").and_then(Value::as_f64),
            reward: r.get("reward_mean").and_then(Value::as_f64),
            improved: r.get("improved").and_then(Value::as_bool),
            error: r
                .get("proposal_error")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
        .collect()
}

/// A point-in-time roll-up of a run directory's live state.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Non-reference candidates first (by descending mean reward), then references.
    pub rows: Vec<CandRow>,
    pub total_trials: u64,
    /// Sum of known per-candidate spend (a lower bound while costs are unknown).
    pub known_spend: f64,
    /// Some candidate's cost is unknown, so `known_spend` understates the truth.
    pub any_unknown_cost: bool,
    pub last_trial: Option<LastTrial>,
    /// The headroom rig, when `rig.json` is present.
    pub rig: Option<Rig>,
    /// The latest streamed hypotheses, when `loop.history.jsonl` is present.
    pub hypotheses: Vec<Hypo>,
    /// The run's budget cap (from `seed.json`), so spend is shown against it.
    pub budget_cap: Option<f64>,
}

impl Snapshot {
    /// Attach the headroom rig (from `rig.json`); a no-op when absent.
    #[must_use]
    pub fn with_rig(mut self, rig_json: Option<&Value>) -> Self {
        self.rig = rig_json.and_then(parse_rig);
        self
    }

    /// Attach the budget cap (from `seed.json`'s `budget_usd`), for spend/cap.
    #[must_use]
    pub fn with_cap(mut self, cap: Option<f64>) -> Self {
        self.budget_cap = cap;
        self
    }

    /// Attach the latest `n` streamed hypotheses (from `loop.history.jsonl`).
    #[must_use]
    pub fn with_hypotheses(mut self, rows: &[Value], n: usize) -> Self {
        self.hypotheses = parse_hypotheses(rows, n);
        self
    }
}

/// Roll up the trial records into a [`Snapshot`]. Reuses [`report::aggregate`] so
/// the numbers match the batch report exactly.
pub fn snapshot(records: &[Value]) -> Snapshot {
    let cands = report::aggregate(records);
    let mut rows: Vec<CandRow> = cands
        .values()
        .map(|c| CandRow {
            id: c
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            reward_mean: c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0),
            trials: c.get("trials").and_then(Value::as_u64).unwrap_or(0),
            // aggregate leaves cost null when a non-costless candidate had an
            // unknown-cost trial; costless refs (oracle/null) keep a known 0.0.
            cost: c.get("cost").and_then(Value::as_f64),
            reference: report::is_reference_kind(c.get("kind").and_then(Value::as_str)),
        })
        .collect();
    // The canonical leaderboard order — shared with report_html so the live view
    // and the HTML report can never disagree on who leads.
    rows.sort_by(|a, b| {
        report::cmp_leaderboard((a.reference, a.reward_mean), (b.reference, b.reward_mean))
    });

    let total_trials = rows.iter().map(|r| r.trials).sum();
    let known_spend = rows.iter().filter_map(|r| r.cost).sum();
    let any_unknown_cost = rows.iter().any(|r| r.cost.is_none());
    let last_trial = records.last().map(|r| LastTrial {
        candidate: r
            .get("candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string(),
        task: r
            .get("task_id")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string(),
        reward: r.get("reward").and_then(Value::as_f64).unwrap_or(0.0),
    });

    Snapshot {
        rows,
        total_trials,
        known_spend,
        any_unknown_cost,
        last_trial,
        rig: None,
        hypotheses: Vec::new(),
        budget_cap: None,
    }
}

/// Render the snapshot as a terminal block. `complete` flips the header to the
/// finished state; `authoritative_spend` (from `loop.json` once the run ends) is
/// shown as the run total instead of the trial-level lower bound.
pub fn render(
    snap: &Snapshot,
    run_label: &str,
    complete: bool,
    authoritative_spend: Option<f64>,
) -> String {
    let mut s = String::new();
    let state = if complete { "complete" } else { "running" };
    // The budget denominator, shown beside spend so an operator can see how close
    // a paid run is to its cap (the most actionable live signal).
    let cap = snap
        .budget_cap
        .map(|c| format!(" / {} cap", money(c)))
        .unwrap_or_default();
    let spend = if complete {
        // At completion loop.json carries the run total (optimizer + certification
        // + holdout), which the trial-level sum understates; prefer it.
        match authoritative_spend {
            // "run total" signals this exceeds the per-candidate cost column: it
            // includes optimizer, certification, and holdout spend from loop.json.
            Some(a) => format!("spend {}{cap} · run total", money(a)),
            None => format!("known spend {}{cap}", money(snap.known_spend)),
        }
    } else {
        let more = if snap.any_unknown_cost { "+" } else { "" };
        format!("known spend {}{more}{cap}", money(snap.known_spend))
    };
    let _ = writeln!(
        s,
        "RUN {run_label} · {state} · trials {} · {spend}",
        snap.total_trials
    );

    // HEADROOM strip — is the arena measuring real skill, or saturated? Shown
    // even before the first trial, since the rig is computed up front.
    if let Some(rig) = &snap.rig {
        let probe = rig
            .probe
            .map(|p| format!("{p:.2}"))
            .unwrap_or_else(|| "—".to_string());
        let verdict = if rig.saturated {
            "saturated"
        } else {
            "unsaturated"
        };
        let _ = writeln!(
            s,
            "  headroom · oracle {:.2} · null {:.2} · probe {probe} — {verdict}",
            rig.oracle, rig.null
        );
    }

    if snap.rows.is_empty() {
        s.push_str("  (no trials yet — waiting for the first result…)\n");
    } else {
        // LEADER — the top deliverable candidate (references never lead). `cost`
        // is the candidate's total known spend, so divide by trials per-trial.
        if let Some(leader) = snap.rows.iter().find(|r| !r.reference) {
            let per_trial = match (leader.cost, leader.trials) {
                (Some(c), t) if t > 0 => format!("{}/trial", money(c / t as f64)),
                _ => "—".to_string(),
            };
            let _ = writeln!(
                s,
                "  ▶ leader {} · reward {:.2} · {per_trial}",
                leader.id, leader.reward_mean
            );
        }

        let _ = writeln!(
            s,
            "  {:>6}  {:>6}  {:>10}  candidate",
            "reward", "trials", "cost"
        );
        for r in &snap.rows {
            let cost = match r.cost {
                Some(c) => money(c),
                None => "—".to_string(),
            };
            let tag = if r.reference { " (ref)" } else { "" };
            let _ = writeln!(
                s,
                "  {:>6.2}  {:>6}  {:>10}  {}{}",
                r.reward_mean, r.trials, cost, r.id, tag
            );
        }
    }
    // HYPOTHESES — what the optimizer is trying, streamed live from
    // loop.history.jsonl. Rendered even before the first trial lands (the
    // window where watching the search propose is most valuable).
    if !snap.hypotheses.is_empty() {
        s.push_str("  hypotheses (latest):\n");
        for h in &snap.hypotheses {
            let _ = writeln!(s, "{}", hypo_line(h));
        }
    }

    if let Some(lt) = &snap.last_trial {
        let _ = writeln!(
            s,
            "  last: {} × {} → {:.2}",
            lt.candidate, lt.task, lt.reward
        );
    }
    s
}

/// One hypotheses-panel line: the mutation, its reward move, and the verdict —
/// or, for a failed proposal, the optimizer error (it never ran).
fn hypo_line(h: &Hypo) -> String {
    let what = if h.slot.is_empty() {
        h.parent_id.clone()
    } else {
        h.slot.clone()
    };
    let head = format!("g{} {what}", h.generation);
    if let Some(err) = &h.error {
        return format!("    {head} · ✗ proposal error: {err}");
    }
    let delta = match (h.parent_reward, h.reward) {
        (Some(p), Some(c)) => format!("{p:.2}→{c:.2}"),
        (None, Some(c)) => format!("→{c:.2}"),
        _ => "—".to_string(),
    };
    let verdict = match h.improved {
        Some(true) => "kept",
        Some(false) => "discarded",
        None => "?",
    };
    format!("    {head} · {delta} · {verdict}")
}

/// Format dollars, normalizing negative zero — `f64`'s `Sum` seeds its fold with
/// `-0.0`, so an empty spend total prints as `-0.0000` without this.
fn money(x: f64) -> String {
    let x = if x == 0.0 { 0.0 } else { x };
    format!("${x:.4}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn trial(cand: &str, kind: &str, task: &str, reward: f64, cost: Option<f64>) -> Value {
        json!({
            "candidate_id": cand, "candidate_kind": kind, "model": "m",
            "composition_hash": "h", "task_id": task, "reward": reward,
            "cost_usd": cost, "wall_ms": 1000.0, "error": null,
        })
    }

    /// null floor + two real candidates across two tasks; one real candidate has
    /// an unknown-cost trial.
    fn records() -> Vec<Value> {
        vec![
            trial("null", "null", "alpha", 0.0, None),
            trial("strong", "pi", "alpha", 1.0, Some(0.01)),
            trial("weak", "pi", "alpha", 0.0, Some(0.02)),
            trial("strong", "pi", "beta", 1.0, Some(0.01)),
            trial("weak", "pi", "beta", 0.5, None), // unknown cost
        ]
    }

    #[test]
    fn snapshot_matches_aggregate() {
        let recs = records();
        let s = snapshot(&recs);
        assert_eq!(s.total_trials, 5, "every trial counted");
        // strong: known cost 0.02 over 2 trials; weak: one unknown → cost None.
        let strong = s.rows.iter().find(|r| r.id == "strong").unwrap();
        assert_eq!(strong.reward_mean, 1.0);
        assert_eq!(strong.trials, 2);
        assert_eq!(strong.cost, Some(0.02));
        let weak = s.rows.iter().find(|r| r.id == "weak").unwrap();
        assert_eq!(
            weak.cost, None,
            "an unknown-cost trial makes the candidate cost unknown"
        );
        assert!(s.any_unknown_cost, "weak's unknown cost must surface");
        // known_spend = strong's 0.02 (+ null's costless 0.0); weak excluded.
        assert!(
            (s.known_spend - 0.02).abs() < 1e-9,
            "known spend sums only known costs: {}",
            s.known_spend
        );
        // non-reference first, by reward desc: strong, weak, then null (ref).
        let order: Vec<&str> = s.rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            order,
            vec!["strong", "weak", "null"],
            "non-ref by reward desc, refs last"
        );
        assert!(s.rows.iter().find(|r| r.id == "null").unwrap().reference);
        // last trial is the most recent record.
        let last = s.last_trial.as_ref().unwrap();
        assert_eq!(
            (last.candidate.as_str(), last.task.as_str(), last.reward),
            ("weak", "beta", 0.5)
        );
    }

    #[test]
    fn snapshot_grows_as_rows_append() {
        let recs = records();
        let early = snapshot(&recs[..2]);
        let later = snapshot(&recs);
        assert!(
            later.total_trials > early.total_trials,
            "trials accumulate as rows append"
        );
    }

    #[test]
    fn render_shows_the_live_rollup() {
        let s = snapshot(&records());
        let txt = render(&s, "20260613T-search", false, None);
        assert!(txt.contains("20260613T-search"), "run label shown");
        assert!(
            txt.contains("running"),
            "running state shown while in flight"
        );
        assert!(txt.contains("strong"), "candidates listed");
        assert!(txt.contains('$'), "spend shown");
        assert!(txt.contains("last:"), "streaming heartbeat shown");
    }

    fn rig_json() -> Value {
        json!({"oracle_mean": 1.0, "null_mean": 0.1667, "probe_mean": 0.6, "saturated": false})
    }

    fn history_rows() -> Vec<Value> {
        vec![
            json!({"generation": 1, "parent_id": "base", "slot_changed": "prompt_packet",
                   "parent_reward_mean": 0.5, "reward_mean": 0.75, "improved": true}),
            json!({"generation": 2, "parent_id": "base", "slot_changed": "tools",
                   "parent_reward_mean": 0.5, "reward_mean": 0.48, "improved": false}),
            json!({"generation": 2, "parent_id": "base", "proposal_error": "optimizer returned garbage"}),
        ]
    }

    #[test]
    fn parse_rig_reads_ceiling_floor_probe() {
        let rig = parse_rig(&rig_json()).unwrap();
        assert_eq!(rig.oracle, 1.0);
        assert_eq!(rig.null, 0.1667);
        assert_eq!(rig.probe, Some(0.6));
        assert!(!rig.saturated);
        // Missing ceiling/floor → no rig (graceful degrade), not a panic.
        assert!(parse_rig(&json!({"saturated": true})).is_none());
    }

    #[test]
    fn parse_hypotheses_takes_latest_n_oldest_first() {
        let rows = history_rows();
        let h = parse_hypotheses(&rows, 2);
        assert_eq!(h.len(), 2, "only the latest 2 of 3");
        assert_eq!(h[0].generation, 2, "oldest-of-the-tail first");
        assert_eq!(h[1].error.as_deref(), Some("optimizer returned garbage"));
        // n larger than available returns all.
        assert_eq!(parse_hypotheses(&rows, 99).len(), 3);
    }

    #[test]
    fn cockpit_renders_rig_leader_and_hypotheses() {
        let snap = snapshot(&records())
            .with_rig(Some(&rig_json()))
            .with_hypotheses(&history_rows(), 5)
            .with_cap(Some(2.52));
        let txt = render(&snap, "run-x", false, None);
        // spend against the budget cap — the actionable live signal
        assert!(txt.contains("/ $2.5200 cap"), "spend shown against the cap");
        // headroom strip
        assert!(txt.contains("headroom"), "rig strip shown");
        assert!(txt.contains("oracle 1.00"), "ceiling shown");
        assert!(txt.contains("null 0.17"), "floor shown");
        assert!(txt.contains("probe 0.60"), "probe shown");
        assert!(txt.contains("unsaturated"), "verdict shown");
        // leader callout — `strong` leads the records() fixture; cost is shown
        // per-trial (total 0.02 over 2 trials = $0.0100/trial), not the total.
        assert!(txt.contains("▶ leader strong"), "leader callout shown");
        assert!(
            txt.contains("$0.0100/trial"),
            "leader cost is per-trial, not the summed total: {txt}"
        );
        // hypotheses panel, streamed
        assert!(
            txt.contains("hypotheses (latest):"),
            "hypotheses panel shown"
        );
        assert!(txt.contains("prompt_packet"), "the mutated slot shown");
        assert!(txt.contains("0.50→0.75"), "reward move shown");
        assert!(txt.contains("kept"), "improved verdict shown");
        assert!(txt.contains("discarded"), "non-improving verdict shown");
        assert!(
            txt.contains("proposal error: optimizer returned garbage"),
            "a failed proposal still streams as a hypothesis that never ran"
        );
    }

    #[test]
    fn hypotheses_render_before_the_first_trial_lands() {
        // The early window — optimizer proposing, no trial result yet — is when
        // watching the search is most valuable, so the panel must not be gated
        // behind trials existing.
        let snap = snapshot(&[]).with_hypotheses(&history_rows(), 5);
        let txt = render(&snap, "run-x", false, None);
        assert!(txt.contains("waiting"), "still shows the no-trials line");
        assert!(
            txt.contains("hypotheses (latest):"),
            "hypotheses stream even with zero trials"
        );
        assert!(txt.contains("prompt_packet"), "the mutation is shown");
    }

    #[test]
    fn cockpit_degrades_without_rig_or_hypotheses() {
        // The plain snapshot (no rig, no history) renders the roll-up only —
        // no empty panels.
        let txt = render(&snapshot(&records()), "run-x", false, None);
        assert!(!txt.contains("headroom"), "no rig strip without rig.json");
        assert!(
            !txt.contains("hypotheses"),
            "no hypotheses panel without loop.history.jsonl"
        );
        assert!(txt.contains("strong"), "candidate roll-up still renders");
    }

    #[test]
    fn render_complete_uses_authoritative_spend() {
        let s = snapshot(&records());
        let txt = render(&s, "run-x", true, Some(2.5));
        assert!(txt.contains("complete"), "finished state shown");
        assert!(
            txt.contains("2.5000"),
            "authoritative run-total spend shown"
        );
    }

    #[test]
    fn empty_run_renders_a_waiting_line() {
        let s = snapshot(&[]);
        assert_eq!(s.total_trials, 0);
        let txt = render(&s, "run-x", false, None);
        assert!(txt.to_lowercase().contains("waiting") || txt.to_lowercase().contains("no trials"));
    }

    #[test]
    fn zero_spend_is_not_negative_zero() {
        // f64's Sum seeds with -0.0, so an empty/zero total must be normalized.
        let txt = render(&snapshot(&[]), "run-x", false, None);
        assert!(txt.contains("$0.0000"), "zero spend shown as +0: {txt}");
        assert!(!txt.contains("-0.0000"), "no negative-zero spend");
    }
}
