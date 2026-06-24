//! Self-contained static HTML report over an experiment run directory.
//!
//! The visual companion to [`crate::report`]'s markdown: the same aggregate,
//! drawn in the Misty Step / `lab.css` design language. `report-html <run-dir>`
//! emits a single offline file — CSS inlined, no network, opens from `file://`,
//! PR-attachable — that renders four review surfaces and the rig panel that says
//! whether the ranking can be trusted at all:
//!
//! 1. **Leaderboard** — candidates by mean reward with cost/latency/trials,
//!    certified and recommended marked, reference rows (oracle/null/oneshot)
//!    receding.
//! 2. **CI forest** — each certified candidate's `(candidate − null)`
//!    reward-delta 95% CI drawn as a caterpillar, with the cluster-robust width,
//!    the `sig` verdict, and the `clstr→95%` power note (backlog 039).
//! 3. **Coverage heatmap** — the candidate × task grid in the lawful encoding
//!    (status glyph + tabular figure, best-in-row heavy), exposing the
//!    Simpson's-paradox wins a single mean hides (backlog 040).
//! 4. **Transcript drill** — every score cell anchors into the representative
//!    trial: its lineage, the candidate's findings, and the scorer's
//!    matched / missed / false-positive verdict.
//!
//! Layered architecture (backlog 044, after Inspect AI): `trials.jsonl` is the
//! source of truth and `loop.json` the run's own verdict; this is a derived
//! viewer, never authoritative. Confidence intervals are drawn only from
//! `loop.json.reward_delta_cis` — the bounds the run actually certified with.
//! A run that predates CI persistence gets an honest "not recorded" notice, not
//! a recomputed band: the cluster-robust interval depends on the arena's
//! source-repo clustering, which a bare run-dir does not carry, and a per-task
//! guess is anticonservative and can contradict the run's own verdict.
//!
//! This module is split by concern: `mod.rs` derives the model from the run
//! data; [`render`] turns that model into HTML.

mod render;

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use serde_json::{Map, Value};

use crate::report;
use crate::stats;

/// Render the self-contained HTML report for a run directory. Reads
/// `trials.jsonl`, `loop.json`, and `rig.json` (the last two optional) and
/// returns the complete document as a string.
pub fn render_html(run_dir: &Path) -> std::io::Result<String> {
    let records = report::load_records(&[run_dir]);
    let loop_json = read_json(&run_dir.join("loop.json")).unwrap_or(Value::Null);
    let rig = read_json(&run_dir.join("rig.json"));
    let label = run_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("run");

    // The contamination + redteam advisory lives in the arena dir, not the run
    // dir; compute it here in the IO shell so build_html_from stays disk-free.
    let arena_id = records
        .first()
        .and_then(|r| r.get("arena_id"))
        .and_then(Value::as_str);
    let sanity = arena_id.and_then(|id| compute_sanity_audit(id, run_dir));

    Ok(build_html_from(
        &records,
        &loop_json,
        rig.as_ref(),
        sanity.as_ref(),
        label,
    ))
}

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Locate an arena dir for `arena_id`. Runs live at `<repo>/runs/<stamp>`, so
/// the repo root is the run-dir's grandparent; arenas live at
/// `<repo>/arenas/<id>`. Try the CWD-relative path first (the common
/// `daedalus report-html runs/…` invocation), then the run-dir's repo root.
/// `None` when neither is a directory — the section then degrades gracefully.
fn locate_arena_dir(arena_id: &str, run_dir: &Path) -> Option<std::path::PathBuf> {
    let cwd_rel = Path::new("arenas").join(arena_id);
    if cwd_rel.is_dir() {
        return Some(cwd_rel);
    }
    let from_run = run_dir
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("arenas").join(arena_id))
        .filter(|p| p.is_dir());
    from_run
}

/// The IO half of the sanity audit: read the arena's `contamination.toml` and
/// red-team every answer key on disk into the pure [`SanityAudit`] model that
/// [`render::sanity_section`] draws. `None` when the arena dir is not reachable
/// — the section then renders an honest "not reachable" note rather than
/// fabricating a verdict.
fn compute_sanity_audit(arena_id: &str, run_dir: &Path) -> Option<SanityAudit> {
    let arena_dir = locate_arena_dir(arena_id, run_dir)?;
    Some(SanityAudit::from_arena_dir(&arena_dir))
}

/// Pure renderer: build the HTML document from already-loaded run data. The IO
/// shell [`render_html`] loads these from disk; tests build them in memory.
///
/// `sanity` is the contamination + redteam advisory derived from the arena dir
/// (the IO shell computes it; `None` means the arena was not reachable, and the
/// sanity section renders an honest "not reachable" note rather than crashing).
pub fn build_html_from(
    records: &[Value],
    loop_json: &Value,
    rig: Option<&Value>,
    sanity: Option<&SanityAudit>,
    run_label: &str,
) -> String {
    let cands = report::aggregate(records);

    // The run's own verdict, preferred over recomputation; falls back to the
    // report kernel when loop.json is absent (e.g. a bare records dir).
    let front =
        arr_strings(loop_json, "pareto_front").unwrap_or_else(|| report::pareto_front(&cands));
    let certified: HashSet<String> = arr_strings(loop_json, "certified")
        .unwrap_or_default()
        .into_iter()
        .collect();
    let recommended = loop_json
        .get("recommended")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| report::recommend(&cands, &front, None));

    let arena_id = records
        .first()
        .and_then(|r| r.get("arena_id"))
        .and_then(Value::as_str)
        .unwrap_or("—");
    let arena_version = records
        .first()
        .and_then(|r| r.get("arena_version"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let tasks = sorted_tasks(&cands);
    let order = ordered_candidates(&cands);

    let (baseline_id, ci_rows) = gather_delta_cis(loop_json);
    let consistency = gather_consistency(&cands, loop_json);

    let mut body = String::new();
    body.push_str(&render::header_section(
        &cands,
        &tasks,
        loop_json,
        recommended.as_deref(),
        run_label,
        arena_id,
        arena_version,
    ));
    body.push_str(&render::rig_section(rig));
    body.push_str(&render::sanity_section(sanity, arena_id));
    body.push_str(&render::leaderboard_section(
        &cands,
        &order,
        &certified,
        recommended.as_deref(),
    ));
    body.push_str(&render::stats_section(
        &baseline_id,
        &ci_rows,
        &consistency,
        &order,
        &cands,
    ));
    body.push_str(&render::heatmap_section(
        &cands,
        &tasks,
        &order,
        recommended.as_deref(),
    ));
    body.push_str(&render::drill_section(records, &cands, &order, &tasks));
    body.push_str(&render::footer_section(loop_json));

    render::document(run_label, arena_id, &body)
}

// ---------------------------------------------------------------------------
// The derived model
// ---------------------------------------------------------------------------

/// A reward-delta confidence interval, parsed once from `loop.json` so the
/// forest and the numeric table read identical typed fields rather than each
/// re-deserializing the same JSON keys (the shape [`stats::DeltaCi::to_value`]
/// writes). A row that lacks the required bounds is dropped rather than rendered
/// with silent zeros.
struct Ci {
    point: f64,
    lo: f64,
    hi: f64,
    excludes_zero: bool,
    n_tasks: u64,
    n_clusters: u64,
    min_clusters_95: Option<u64>,
}

impl Ci {
    fn from_value(v: &Value) -> Option<Ci> {
        Some(Ci {
            point: v.get("point")?.as_f64()?,
            lo: v.get("lo")?.as_f64()?,
            hi: v.get("hi")?.as_f64()?,
            excludes_zero: v
                .get("excludes_zero")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            n_tasks: v.get("n_tasks").and_then(Value::as_u64).unwrap_or(0),
            n_clusters: v.get("n_clusters").and_then(Value::as_u64).unwrap_or(0),
            min_clusters_95: v.get("min_clusters_95").and_then(Value::as_u64),
        })
    }

    /// The interval is too thin to bound a win (fewer than 2 tasks/clusters).
    fn small_n(&self) -> bool {
        self.n_clusters < 2 || self.n_tasks < 2
    }
}

/// The sanity-check affordance beyond the rig panel (backlog 044 oracle 4):
/// whether the arena's fixtures are contamination-resistant, and how many of
/// its answer keys carry spans wide enough to game. Both are derived from the
/// arena dir on disk by [`SanityAudit::from_arena_dir`]; the renderer
/// ([`render::sanity_section`]) only formats this typed model, so it stays unit
/// testable without disk.
pub struct SanityAudit {
    /// `Some(true)` = at least one source is public (score-inflation risk);
    /// `Some(false)` = all sources private/synthetic (contamination-resistant);
    /// `None` = no `contamination.toml` recorded for this arena.
    public: Option<bool>,
    /// The contamination record's free-text note, if any.
    note: Option<String>,
    /// The wide-span threshold the audit flagged against (lines).
    wide_threshold: i64,
    /// Total answer-key defects audited across the arena's tasks.
    n_defects: usize,
    /// Tasks carrying at least one gameable wide span, each with its count.
    wide_tasks: Vec<(String, usize)>,
    /// Total gameable wide-span defects across the arena.
    total_wide: usize,
    /// `true` when at least one task's answer key could not be audited (parse or
    /// IO error) — surfaced so a partial audit is not read as a clean one.
    audit_incomplete: bool,
}

impl SanityAudit {
    /// The threshold the report audits answer-key spans against — matches the
    /// `arena-redteam` CLI default ([`crate::score::redteam_audit`]).
    const WIDE_THRESHOLD: i64 = 8;

    /// Read `<arena_dir>/contamination.toml` and red-team every task's
    /// `expected.json`, reusing the existing core functions
    /// ([`crate::workbench::load_contamination`] and
    /// [`crate::score::redteam_audit`]) rather than reimplementing either.
    fn from_arena_dir(arena_dir: &Path) -> SanityAudit {
        let contam = crate::workbench::load_contamination(arena_dir)
            .ok()
            .flatten();
        let public = contam.as_ref().map(|c| c.source.iter().any(|s| s.public));
        let note = contam.and_then(|c| c.notes);

        let mut n_defects = 0usize;
        let mut total_wide = 0usize;
        let mut wide_tasks: Vec<(String, usize)> = Vec::new();
        let mut audit_incomplete = false;

        // Reuse the canonical sorted task walk. No expected.json pre-filter:
        // redteam_audit returns Err for a missing/unreadable key, and the
        // Err arm below honestly counts that task as audit-incomplete.
        for td in crate::workbench::task_dirs(arena_dir) {
            let tid = td
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            let expected = td.join("tests").join("expected.json");
            match crate::score::redteam_audit(&expected, Self::WIDE_THRESHOLD) {
                Ok(a) => {
                    n_defects += a.n_defects;
                    let wide = a.wide_defects.len();
                    if wide > 0 {
                        total_wide += wide;
                        wide_tasks.push((tid, wide));
                    }
                }
                Err(_) => audit_incomplete = true,
            }
        }

        SanityAudit {
            public,
            note,
            wide_threshold: Self::WIDE_THRESHOLD,
            n_defects,
            wide_tasks,
            total_wide,
            audit_incomplete,
        }
    }

    // Read accessors for the renderer (in the `render` submodule).
    pub(super) fn public(&self) -> Option<bool> {
        self.public
    }
    pub(super) fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
    pub(super) fn wide_threshold(&self) -> i64 {
        self.wide_threshold
    }
    pub(super) fn n_defects(&self) -> usize {
        self.n_defects
    }
    pub(super) fn wide_tasks(&self) -> &[(String, usize)] {
        &self.wide_tasks
    }
    pub(super) fn total_wide(&self) -> usize {
        self.total_wide
    }
    pub(super) fn audit_incomplete(&self) -> bool {
        self.audit_incomplete
    }
}

fn is_reference(c: &Value) -> bool {
    report::is_reference_kind(c.get("kind").and_then(Value::as_str))
}

fn arr_strings(v: &Value, key: &str) -> Option<Vec<String>> {
    v.get(key).and_then(Value::as_array).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect()
    })
}

fn sorted_tasks(cands: &Map<String, Value>) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for c in cands.values() {
        if let Some(tasks) = c.get("tasks").and_then(Value::as_object) {
            for tid in tasks.keys() {
                set.insert(tid.clone());
            }
        }
    }
    set.into_iter().collect()
}

/// Non-reference candidates first (by descending mean reward), then references —
/// the leaderboard reading order ([`report::cmp_leaderboard`]), reused for the
/// heatmap columns.
fn ordered_candidates(cands: &Map<String, Value>) -> Vec<String> {
    let key = |c: &Value| {
        (
            is_reference(c),
            c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0),
        )
    };
    let mut ids: Vec<String> = cands.keys().cloned().collect();
    ids.sort_by(|a, b| report::cmp_leaderboard(key(&cands[a]), key(&cands[b])));
    ids
}

fn cell_mean(c: &Value, task: &str) -> Option<(f64, usize)> {
    let arr = c
        .get("tasks")
        .and_then(|t| t.get(task))
        .and_then(Value::as_array)?;
    let vals: Vec<f64> = arr.iter().filter_map(Value::as_f64).collect();
    if vals.is_empty() {
        None
    } else {
        Some((vals.iter().sum::<f64>() / vals.len() as f64, vals.len()))
    }
}

/// Read the run's persisted reward-delta CIs, parsed into typed [`Ci`] rows
/// sorted by candidate id.
///
/// The report never *recomputes* a CI: the cluster-robust interval depends on
/// the arena's `source_repo` clustering, which the run applied and a bare
/// run-dir does not carry. Recomputing with a guessed (per-task) clustering
/// yields anticonservative bars that can contradict the run's own
/// certification — a lie is worse than a gap. Runs that predate CI persistence
/// get an honest "not recorded" notice instead (see [`render::stats_section`]).
fn gather_delta_cis(loop_json: &Value) -> (String, Vec<(String, Ci)>) {
    let baseline = loop_json
        .get("reward_delta_baseline")
        .and_then(Value::as_str)
        .unwrap_or("null")
        .to_string();

    let rows = loop_json
        .get("reward_delta_cis")
        .and_then(Value::as_object)
        .map(|obj| {
            let mut rows: Vec<(String, Ci)> = obj
                .iter()
                .filter_map(|(k, v)| Ci::from_value(v).map(|ci| (k.clone(), ci)))
                .collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            rows
        })
        .unwrap_or_default();
    (baseline, rows)
}

fn gather_consistency(cands: &Map<String, Value>, loop_json: &Value) -> Map<String, Value> {
    if let Some(obj) = loop_json.get("consistency").and_then(Value::as_object) {
        if !obj.is_empty() {
            return obj.clone();
        }
    }
    let floor = loop_json
        .get("consistency_floor")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let mut out = Map::new();
    for (cid, c) in cands {
        if is_reference(c) {
            continue;
        }
        // Recompute the pass *rate* from the trials, but leave pass^k null: a
        // bare run-dir doesn't record the certification k, and pass^(n_trials)
        // would print a near-zero that reads as a finding rather than a gap.
        let con = stats::candidate_consistency(c, floor);
        let mut m = Map::new();
        m.insert("n_trials".into(), Value::from(con.n_trials as u64));
        m.insert("passes".into(), Value::from(con.passes as u64));
        m.insert("floor".into(), Value::from(con.floor));
        m.insert("rate".into(), Value::from(con.rate));
        m.insert("pass_k".into(), Value::Null);
        out.insert(cid.clone(), Value::Object(m));
    }
    out
}

fn ci_axis(rows: &[(String, Ci)]) -> (f64, f64) {
    let mut lo = 0.0_f64;
    let mut hi = 0.0_f64;
    for (_, ci) in rows {
        lo = lo.min(ci.lo);
        hi = hi.max(ci.hi);
    }
    let pad = (hi - lo).max(0.1) * 0.08;
    (lo - pad, hi + pad)
}

fn representative_trial<'a>(records: &'a [Value], cid: &str, task: &str) -> Option<&'a Value> {
    records.iter().find(|r| {
        r.get("candidate_id").and_then(Value::as_str) == Some(cid)
            && r.get("task_id").and_then(Value::as_str) == Some(task)
    })
}

/// Format `Some(v)` via `f`, or an em-dash for `None` — the report's one way to
/// render a missing cell.
fn or_dash<T>(v: Option<T>, f: impl FnOnce(T) -> String) -> String {
    v.map(f).unwrap_or_else(|| "—".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A synthetic run: a `null` floor and one real candidate across two tasks,
    /// with findings and a scorer verdict on each trial, plus a `loop.json` that
    /// certifies the candidate and persists its reward-delta CI, and a `rig.json`.
    fn fixture() -> (Vec<Value>, Value, Value) {
        let records = vec![
            json!({
                "candidate_id": "null", "candidate_kind": "null", "model": null,
                "composition_hash": "nullhash", "task_id": "alpha", "trial": 1,
                "reward": 0.0, "recall": 0.0, "matched": [], "false_positives": 0,
                "expected_defects": 1, "cost_usd": null, "wall_ms": 10.0,
                "findings": [], "error": null,
            }),
            json!({
                "candidate_id": "null", "candidate_kind": "null", "model": null,
                "composition_hash": "nullhash", "task_id": "beta", "trial": 1,
                "reward": 0.0, "recall": 0.0, "matched": [], "false_positives": 0,
                "expected_defects": 1, "cost_usd": null, "wall_ms": 10.0,
                "findings": [], "error": null,
            }),
            json!({
                "candidate_id": "skeptic-R&D", "candidate_kind": "pi",
                "model": "z-ai/glm-4.7-flash", "composition_hash": "abc123",
                "task_id": "alpha", "trial": 1, "reward": 1.0, "recall": 1.0,
                "matched": ["alpha-defect"], "false_positives": 0,
                "expected_defects": 1, "cost_usd": 0.012, "wall_ms": 42000.0,
                "findings": [{
                    "file": "src/<core>.rs", "line": 12, "category": "logic-invariant",
                    "severity": "blocking",
                    "description": "aliases the live list & empties it before render <hack>",
                }],
                "error": null,
            }),
            json!({
                "candidate_id": "skeptic-R&D", "candidate_kind": "pi",
                "model": "z-ai/glm-4.7-flash", "composition_hash": "abc123",
                "task_id": "beta", "trial": 1, "reward": 0.0, "recall": 0.0,
                "matched": [], "false_positives": 1, "expected_defects": 1,
                "cost_usd": 0.012, "wall_ms": 51000.0,
                "findings": [], "error": null,
            }),
        ];
        let loop_json = json!({
            "stop_reason": "max-candidates", "mode": "threshold-then-cheap",
            "recommended": "skeptic-R&D", "certified": ["skeptic-R&D"],
            "pareto_front": ["skeptic-R&D"], "spend_known_usd": 1.3002,
            "trial_complete": ["skeptic-R&D"], "min_effect": 0.0,
            "consistency_floor": 1.0, "reward_delta_baseline": "null",
            "reward_delta_cis": {
                "skeptic-R&D": {
                    "baseline": "null", "point": 0.5, "se": 0.5,
                    "lo": -0.05, "hi": 1.05, "ci": 0.95, "n_tasks": 2,
                    "n_clusters": 2, "excludes_zero": false, "min_clusters_95": 8,
                },
            },
            "consistency": {
                "skeptic-R&D": {
                    "n_trials": 2, "passes": 1, "floor": 1.0, "rate": 0.5,
                    "pass_k_k": 5, "pass_k": null,
                },
            },
        });
        let rig = json!({
            "oracle_mean": 1.0, "null_mean": 0.0, "probe_mean": 0.0, "saturated": false,
        });
        (records, loop_json, rig)
    }

    #[test]
    fn renders_a_self_contained_document() {
        let (records, loop_json, rig) = fixture();
        let html = build_html_from(
            &records,
            &loop_json,
            Some(&rig),
            None,
            "20260613T000000Z-test",
        );

        assert!(
            html.starts_with("<!doctype html>"),
            "must be a full HTML doc"
        );
        // CSS is inlined, not linked — the vendored token proves the <style> shipped.
        assert!(html.contains("<style>"), "CSS must be inlined in a <style>");
        assert!(
            html.contains("--ae-accent"),
            "vendored design tokens must be inlined"
        );

        // Self-contained / offline invariant: no external CSS/JS, no network, no
        // relative asset paths that would 404 from a PR attachment.
        assert!(
            !html.contains("<link rel=\"stylesheet\""),
            "must not link external stylesheets"
        );
        assert!(
            !html.contains("<script src="),
            "must not load external scripts"
        );
        assert!(
            !html.contains("href=\"http"),
            "must not reference the network"
        );
        assert!(
            !html.contains("href=\"../") && !html.contains("src=\"../"),
            "must not reference sibling files by relative path"
        );
    }

    #[test]
    fn draws_the_four_review_surfaces() {
        let (records, loop_json, rig) = fixture();
        let html = build_html_from(
            &records,
            &loop_json,
            Some(&rig),
            None,
            "20260613T000000Z-test",
        );

        // (1) leaderboard — recommended candidate is named and marked.
        assert!(
            html.contains("skeptic-R&amp;D"),
            "candidate id must appear (escaped)"
        );
        // (2) CI forest — a *positioned* interval band is drawn (the class alone
        // also lives in the vendored CSS, so assert the inline position too).
        assert!(
            html.contains("ae-ci-band\" style=\"left:"),
            "a positioned CI band must be drawn"
        );
        assert!(html.contains("95%"), "the confidence level must be shown");
        // (3) coverage heatmap — the lawful candidate × task matrix.
        assert!(
            html.contains("class=\"matrix\""),
            "coverage heatmap must render"
        );
        // (4) transcript drill — every score reaches its trial behind an anchor.
        assert!(
            html.contains("id=\"trial-"),
            "score cells must drill to a transcript"
        );

        // rig panel — the sanity gate (oracle ceiling / null floor / probe).
        assert!(html.contains("id=\"rig\""), "rig panel must render");
    }

    #[test]
    fn omits_forest_and_notices_when_cis_absent() {
        // A run predating CI persistence: no reward_delta_cis recorded. The
        // report must NOT recompute a (potentially contradictory) band — it
        // shows an honest notice and still renders reliability + the verdict.
        let (records, mut loop_json, rig) = fixture();
        loop_json
            .as_object_mut()
            .unwrap()
            .remove("reward_delta_cis");
        let html = build_html_from(&records, &loop_json, Some(&rig), None, "t");
        assert!(
            !html.contains("ae-ci-band\" style=\"left:"),
            "no recomputed band may be drawn for a run that didn't record CIs"
        );
        assert!(
            html.contains("were not recorded by this run"),
            "the gap must be stated honestly"
        );
        assert!(html.contains("pass rate"), "reliability still renders");
        // the run's verdict is still trusted from loop.json.
        assert!(
            html.contains("recommended"),
            "the certified pick still shows"
        );
    }

    #[test]
    fn drill_anchors_are_unique_and_resolve_despite_punctuation() {
        // Two candidate ids that differ only in punctuation (a dotted model
        // version vs a hyphenated one) must NOT collapse to the same anchor —
        // otherwise a score row drills into the wrong transcript.
        let mk = |cid: &str| {
            json!({
                "candidate_id": cid, "candidate_kind": "pi", "model": "m",
                "composition_hash": "h", "task_id": "t", "trial": 1,
                "reward": 1.0, "recall": 1.0, "matched": ["t-defect"],
                "false_positives": 0, "expected_defects": 1, "cost_usd": 0.01,
                "wall_ms": 1000.0, "findings": [], "error": null,
            })
        };
        let records = vec![mk("seed3-qwen3.7-plus"), mk("seed3-qwen3-7-plus")];
        let html = build_html_from(&records, &Value::Null, None, None, "t");

        // Pull the fragment after a marker up to the closing quote.
        let frags = |pat: &str| -> Vec<String> {
            html.match_indices(pat)
                .map(|(i, _)| {
                    let rest = &html[i + pat.len()..];
                    rest[..rest.find('"').unwrap()].to_string()
                })
                .collect()
        };
        let ids = frags("id=\"trial-");
        let unique: HashSet<&String> = ids.iter().collect();
        assert_eq!(
            ids.len(),
            unique.len(),
            "drill anchors must be unique: {ids:?}"
        );
        assert_eq!(
            ids.len(),
            2,
            "both candidates must get their own transcript"
        );

        // Every heatmap link must resolve to one of those transcript ids.
        for frag in frags("href=\"#trial-") {
            assert!(
                ids.contains(&frag),
                "heatmap link #trial-{frag} has no matching transcript id"
            );
        }
    }

    #[test]
    fn escapes_candidate_and_finding_text() {
        let (records, loop_json, rig) = fixture();
        let html = build_html_from(&records, &loop_json, Some(&rig), None, "t");
        // The injected markup must be escaped, never emitted raw.
        assert!(html.contains("R&amp;D"), "ampersands escaped");
        assert!(
            !html.contains("<hack>"),
            "angle brackets in findings escaped"
        );
        assert!(!html.contains("src/<core>.rs"), "file paths escaped");
    }

    // -----------------------------------------------------------------------
    // Sanity affordances: contamination advisory + redteam span audit (044
    // oracle 4). These exercise the IO half (SanityAudit::from_arena_dir reads
    // a real arena dir on disk) since that is the seam that can mislead.
    // -----------------------------------------------------------------------

    use std::sync::atomic::{AtomicU64, Ordering};
    static SANITY_SEQ: AtomicU64 = AtomicU64::new(0);

    /// A throwaway arena dir with a contamination record and an answer key
    /// carrying one gameable wide span (30 lines > the 8-line threshold) and
    /// one tight span. Mirrors the `arenas/<id>/tasks/<t>/tests/expected.json`
    /// layout the real audit reads.
    fn write_arena_fixture(public: bool, note: &str) -> std::path::PathBuf {
        let n = SANITY_SEQ.fetch_add(1, Ordering::SeqCst);
        let arena =
            std::env::temp_dir().join(format!("daedalus-sanity-{}-{n}", std::process::id()));
        let task = arena.join("tasks").join("wide-key").join("tests");
        std::fs::create_dir_all(&task).unwrap();
        std::fs::write(
            arena.join("contamination.toml"),
            format!(
                "defects_novel = true\nnotes = \"{note}\"\n[[source]]\nrepo = \"src\"\npublic = {public}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            task.join("expected.json"),
            r#"{"defects":[
              {"id":"wide","file":"a.py","line_start":1,"line_end":30,"category":"correctness"},
              {"id":"tight","file":"a.py","line_start":50,"line_end":51,"category":"security"}
            ]}"#,
        )
        .unwrap();
        arena
    }

    fn records_for(arena_id: &str) -> Vec<Value> {
        vec![json!({
            "candidate_id": "c1", "candidate_kind": "pi", "model": "m",
            "composition_hash": "h", "arena_id": arena_id, "arena_version": "0.1.0",
            "task_id": "wide-key", "trial": 1, "reward": 1.0, "recall": 1.0,
            "matched": ["wide"], "false_positives": 0, "expected_defects": 1,
            "cost_usd": 0.01, "wall_ms": 1000.0, "findings": [], "error": null,
        })]
    }

    #[test]
    fn sanity_section_shows_contamination_verdict_and_wide_span_count() {
        let arena = write_arena_fixture(true, "drawn from public rich");
        let sanity = SanityAudit::from_arena_dir(&arena);
        let records = records_for("my-arena");
        let html = build_html_from(&records, &Value::Null, None, Some(&sanity), "t");

        // The sanity cluster renders beside the rig panel.
        assert!(html.contains("id=\"sanity\""), "sanity section must render");
        // (a) contamination advisory: a public source → score-inflation verdict.
        assert!(
            html.contains("public-derived"),
            "public-derived contamination verdict must show: {}",
            &html[html.find("id=\"sanity\"").unwrap()..][..1200]
        );
        assert!(
            html.contains("drawn from public rich"),
            "the contamination note must surface"
        );
        // (b) redteam span audit: the 30-line key is flagged, the 2-line one not.
        assert!(
            html.contains("REDTEAM SPAN AUDIT"),
            "the span audit must render"
        );
        assert!(
            html.contains("1 of 2 answer-key defects carry a wide span"),
            "the wide-span count must show (1 of 2)"
        );
        assert!(
            html.contains("wide-key:1"),
            "the flagged task must be named with its wide-span count"
        );
        // Regression guard for the double-escape fix: the threshold `>` must be
        // escaped exactly once (`&gt;`), never the literal `&amp;gt;` that a
        // pre-escaped string passed back through esc() would produce.
        assert!(
            html.contains("wide span (&gt; 8 lines)"),
            "the threshold > must be escaped exactly once"
        );
        assert!(
            !html.contains("&amp;gt;"),
            "no double-escaped entity may appear"
        );

        // A contamination-resistant arena reads the other way.
        let arena2 = write_arena_fixture(false, "synthetic, author-written");
        let sanity2 = SanityAudit::from_arena_dir(&arena2);
        let html2 = build_html_from(&records, &Value::Null, None, Some(&sanity2), "t");
        assert!(
            html2.contains("contamination-resistant"),
            "all-private sources → contamination-resistant verdict"
        );

        let _ = std::fs::remove_dir_all(&arena);
        let _ = std::fs::remove_dir_all(&arena2);
    }

    #[test]
    fn sanity_section_degrades_gracefully_when_arena_unreachable() {
        // The arena dir is not reachable (the common case for a bare run-dir
        // attachment): the IO shell passes None, and the section must render an
        // honest "not reachable" note rather than crash or fabricate a verdict.
        let records = records_for("absent-arena");
        let html = build_html_from(&records, &Value::Null, None, None, "t");
        assert!(
            html.contains("id=\"sanity\""),
            "the section renders even when the arena is unreachable"
        );
        assert!(
            html.contains("not reachable for the sanity audit"),
            "the gap must be stated honestly"
        );
        assert!(
            html.contains("absent-arena"),
            "the unreachable arena id must be named"
        );
        // No verdict may be fabricated for an unreachable arena.
        assert!(
            !html.contains("public-derived") && !html.contains("contamination-resistant"),
            "no contamination verdict may be fabricated when the arena is absent"
        );
    }

    #[test]
    fn locate_arena_dir_resolves_relative_to_the_run_dirs_repo_root() {
        // Layout: <root>/runs/<stamp> as the run dir, <root>/arenas/<id> the
        // arena — the IO shell must find the arena via the run-dir's grandparent
        // even when CWD is elsewhere.
        let n = SANITY_SEQ.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("daedalus-root-{}-{n}", std::process::id()));
        let run_dir = root.join("runs").join("20260101T000000Z-x");
        let arena_dir = root.join("arenas").join("the-arena").join("tasks");
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::create_dir_all(&arena_dir).unwrap();

        let found = locate_arena_dir("the-arena", &run_dir);
        assert_eq!(found, Some(root.join("arenas").join("the-arena")));
        assert_eq!(
            locate_arena_dir("no-such-arena", &run_dir),
            None,
            "an absent arena is not located"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
