//! Live terminal dashboard over a run in flight (backlog 049).
//!
//! `daedalus view <run-dir>` polls `trials.jsonl` and reprints a roll-up —
//! per-candidate running mean reward, trials so far, and cumulative known spend
//! — the live companion to the post-run static report ([`crate::report_html`]).
//! The rich surfaces (CI forest, certification) are inherently post-run: there is
//! no certified verdict until the search ends, so the valuable *live* signal is
//! the roll-up and the spend ticking up.
//!
//! `trials.jsonl` stays the source of truth; the snapshot reuses
//! [`crate::report::aggregate`], so the live view can never drift from the batch
//! report. The poll/redraw loop is a thin IO shell in the CLI; everything here is
//! pure and tested.

use std::cmp::Ordering;
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

/// A point-in-time roll-up of a run directory's trials.
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
    // Non-reference candidates first, by descending mean reward; references last.
    rows.sort_by(|a, b| {
        a.reference.cmp(&b.reference).then_with(|| {
            b.reward_mean
                .partial_cmp(&a.reward_mean)
                .unwrap_or(Ordering::Equal)
        })
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
    let spend = if complete {
        // At completion loop.json carries the run total (optimizer + certification
        // + holdout), which the trial-level sum understates; prefer it.
        match authoritative_spend {
            // "run total" signals this exceeds the per-candidate cost column: it
            // includes optimizer, certification, and holdout spend from loop.json.
            Some(a) => format!("spend {} · run total", money(a)),
            None => format!("known spend {}", money(snap.known_spend)),
        }
    } else {
        let more = if snap.any_unknown_cost { "+" } else { "" };
        format!("known spend {}{more}", money(snap.known_spend))
    };
    let _ = writeln!(
        s,
        "RUN {run_label} · {state} · trials {} · {spend}",
        snap.total_trials
    );

    if snap.rows.is_empty() {
        s.push_str("  (no trials yet — waiting for the first result…)\n");
        return s;
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
    if let Some(lt) = &snap.last_trial {
        let _ = writeln!(
            s,
            "  last: {} × {} → {:.2}",
            lt.candidate, lt.task, lt.reward
        );
    }
    s
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
