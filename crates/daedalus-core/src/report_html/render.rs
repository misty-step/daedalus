//! Turn the derived model into HTML. Each section renderer takes the pieces it
//! draws; [`super::build_html_from`] assembles them. The `format!`/`write!`
//! string-building idiom matches [`crate::report`]'s markdown renderer — no
//! template engine, no dependency.

use std::collections::HashMap;
use std::fmt::Write as _;

use serde_json::{Map, Value};

use super::{cell_mean, ci_axis, is_reference, or_dash, representative_trial, Ci, SanityAudit};

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn header_section(
    cands: &Map<String, Value>,
    tasks: &[String],
    loop_json: &Value,
    recommended: Option<&str>,
    run_label: &str,
    arena_id: &str,
    arena_version: &str,
) -> String {
    let n_cands = cands.len();
    let n_real = cands.values().filter(|c| !is_reference(c)).count();
    let n_trials: u64 = cands
        .values()
        .filter_map(|c| c.get("trials").and_then(Value::as_u64))
        .sum();
    let spend = loop_json.get("spend_known_usd").and_then(Value::as_f64);
    let stop = loop_json
        .get("stop_reason")
        .and_then(Value::as_str)
        .unwrap_or("—");
    let mode = loop_json.get("mode").and_then(Value::as_str).unwrap_or("—");
    let baseline = loop_json
        .get("reward_delta_baseline")
        .and_then(Value::as_str)
        .unwrap_or("null");
    let baseline_label = if baseline == "null" {
        "the null floor".to_string()
    } else {
        format!("the {baseline}")
    };

    let (hero_num, hero_unit, hero_note) = match recommended.and_then(|r| cands.get(r)) {
        Some(c) => {
            let rm = c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
            (
                format!("{rm:.2}"),
                "mean reward".to_string(),
                format!("recommended · {}", esc(recommended.unwrap_or("—"))),
            )
        }
        None => (
            "—".to_string(),
            "no certified pick".to_string(),
            format!("no candidate is provably better than {baseline_label}"),
        ),
    };

    let mut s = String::new();
    let _ = write!(
        s,
        r#"<header>
  <div class="lab-bar">
    <div><h1 style="display:inline">Run report</h1> <span class="ae-dim">— {arena}{ver}</span></div>
    <div class="ae-chrome" style="font-family:var(--ae-font-mono)">{run}</div>
  </div>
  <div class="lab-hero" style="margin:0.2em 0 0.1em">
    <span class="lab-hero-num">{hero_num}</span>
    <span class="lab-hero-unit">{hero_unit}</span>
  </div>
  <p class="ae-chrome" style="margin:0 0 1.2em">{hero_note}</p>
  <div class="lab-figrow" style="margin-bottom:1.8em">
    <div class="lab-fig"><p class="lab-fig-k">candidates</p><p class="lab-fig-v">{n_real} <span class="ae-dim">/ {n_cands} incl. ref</span></p></div>
    <div class="lab-fig"><p class="lab-fig-k">tasks</p><p class="lab-fig-v">{n_tasks}</p></div>
    <div class="lab-fig"><p class="lab-fig-k">trials</p><p class="lab-fig-v">{n_trials}</p></div>
    <div class="lab-fig"><p class="lab-fig-k">known spend</p><p class="lab-fig-v">{spend}</p></div>
    <div class="lab-fig"><p class="lab-fig-k">stop · mode</p><p class="lab-fig-v">{stop} <span class="ae-dim">{mode}</span></p></div>
  </div>
</header>
"#,
        arena = esc(arena_id),
        ver = if arena_version.is_empty() {
            String::new()
        } else {
            format!(" v{}", esc(arena_version))
        },
        run = esc(run_label),
        n_tasks = tasks.len(),
        spend = match spend {
            Some(v) => format!("${v:.4}"),
            None => "—".to_string(),
        },
        stop = esc(stop),
        mode = esc(mode),
    );
    s
}

pub(super) fn rig_section(rig: Option<&Value>) -> String {
    // The sanity gate: a ranking is only trustworthy if the rig is calibrated —
    // the oracle nears 1.0, the floor sits low, and the saturation probe has not
    // collapsed the reward scale. The panel renders even when rig.json is
    // missing, so its absence is visible rather than silent.
    let f = |key: &str| rig.and_then(|r| r.get(key)).and_then(Value::as_f64);
    let oracle = f("oracle_mean");
    let null = f("null_mean");
    let probe = f("probe_mean");
    let saturated = rig
        .and_then(|r| r.get("saturated"))
        .and_then(Value::as_bool);

    let cell = |v: Option<f64>| match v {
        Some(x) => format!("{x:.2}"),
        None => "—".to_string(),
    };
    let (verdict_icon, verdict) = match (saturated, probe, oracle) {
        (Some(true), _, _) => (
            icon("i-alert", "ae-err"),
            "SATURATED — rewards cannot rank configurations; fix the arena first".to_string(),
        ),
        (Some(false), Some(p), Some(o)) => (
            icon("i-check", "ae-ok"),
            format!("clear — probe {p:.2} sits below the oracle ceiling {o:.2}"),
        ),
        _ => (
            icon("i-minus", "ae-warn"),
            "inconclusive — no probe verdict recorded for this run".to_string(),
        ),
    };

    format!(
        r#"<section id="rig" style="margin-bottom:1.8em">
  <p class="ae-plate-cap">RIG · is this ranking trustworthy?</p>
  <div class="lab-figrow">
    <div class="lab-fig"><p class="lab-fig-k">oracle ceiling</p><p class="lab-fig-v">{oracle}</p></div>
    <div class="lab-fig"><p class="lab-fig-k">null floor</p><p class="lab-fig-v">{null}</p></div>
    <div class="lab-fig"><p class="lab-fig-k">probe</p><p class="lab-fig-v">{probe}</p></div>
  </div>
  <p class="ae-chrome" style="margin-top:0.7em">{vi} {verdict}</p>
</section>
"#,
        oracle = cell(oracle),
        null = cell(null),
        probe = cell(probe),
        vi = verdict_icon,
        verdict = esc(&verdict),
    )
}

pub(super) fn sanity_section(sanity: Option<&SanityAudit>, arena_id: &str) -> String {
    // The second sanity gate, beside the rig panel: even a calibrated rig can
    // rank on a flawed arena. Two design flaws a reviewer must catch *before*
    // trusting a ranking — (a) is the arena contamination-resistant, or
    // public-derived with score-inflation risk, and (b) how many answer keys
    // carry spans wide enough that a candidate could guess file+category without
    // locating the defect. The advisory lives in the arena dir, not the run dir,
    // so it degrades gracefully when the arena is not reachable.
    let Some(s) = sanity else {
        return format!(
            r#"<section id="sanity" style="margin-bottom:1.8em">
  <p class="ae-plate-cap">ARENA SANITY · is this arena sound enough to rank on?</p>
  <p class="ae-chrome" style="margin-top:0.7em">{vi} arena <span class="ae-num">{arena}</span> is not reachable for the sanity audit — its <span class="ae-num">contamination.toml</span> and answer keys were not found beside this run, so the contamination verdict and the wide-span audit cannot be drawn. Run from the repo root (so <span class="ae-num">arenas/{arena}</span> resolves) to surface them.</p>
</section>
"#,
            vi = icon("i-minus", "ae-warn"),
            arena = esc(arena_id),
        );
    };

    // (a) Contamination advisory.
    let (contam_icon, contam_verdict) = match s.public() {
        Some(true) => (
            icon("i-alert", "ae-warn"),
            "public-derived — scores may be inflated by training contamination; pair with a contamination-resistant holdout before trusting rankings".to_string(),
        ),
        Some(false) => (
            icon("i-check", "ae-ok"),
            "contamination-resistant — synthetic/private sources, so a config cannot inflate its score from training-data familiarity".to_string(),
        ),
        None => (
            icon("i-minus", "ae-warn"),
            "no contamination.toml recorded for this arena — contamination resistance is unverified".to_string(),
        ),
    };
    let note_html = match s.note() {
        Some(n) if !n.trim().is_empty() => format!(
            "<p class=\"ae-plate-note\" style=\"border:0;padding:0;margin-top:0.4em\">{}</p>",
            esc(n.trim()),
        ),
        _ => String::new(),
    };

    // (b) Red-team wide-span audit.
    // These verdict strings are passed through `esc()` below, so they carry a
    // plain `>` (esc renders it as `&gt;`) — never a pre-escaped entity, which
    // would double-escape to a literal "&gt;".
    let (span_icon, span_verdict) = if s.n_defects() == 0 {
        (
            icon("i-minus", "ae-warn"),
            format!(
                "no answer-key defects found to audit (threshold > {} lines)",
                s.wide_threshold()
            ),
        )
    } else if s.total_wide() == 0 {
        (
            icon("i-check", "ae-ok"),
            format!(
                "0 of {} answer-key defects carry a wide span (> {} lines): the line constraint demands real localization",
                s.n_defects(),
                s.wide_threshold(),
            ),
        )
    } else {
        // Task ids are interpolated into a string that is then esc()'d as a
        // whole, so they must NOT be pre-escaped here (that would double-escape).
        let tasks = s
            .wide_tasks()
            .iter()
            .map(|(tid, n)| format!("{tid}:{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        (
            icon("i-alert", "ae-warn"),
            format!(
                "{} of {} answer-key defects carry a wide span (> {} lines) — a candidate could score by guessing file+category at any in-span line without locating the defect. Tasks: {}",
                s.total_wide(),
                s.n_defects(),
                s.wide_threshold(),
                tasks,
            ),
        )
    };
    let incomplete_html = if s.audit_incomplete() {
        format!(
            "<p class=\"ae-chrome\" style=\"margin-top:0.4em\">{} some answer keys could not be parsed — the wide-span count is a lower bound.</p>",
            icon("i-alert", "ae-warn"),
        )
    } else {
        String::new()
    };

    format!(
        r#"<section id="sanity" style="margin-bottom:1.8em">
  <p class="ae-plate-cap">ARENA SANITY · is this arena sound enough to rank on?</p>
  <div class="lab-figrow">
    <div class="lab-fig"><p class="lab-fig-k">contamination</p><p class="lab-fig-v">{contam_state}</p></div>
    <div class="lab-fig"><p class="lab-fig-k">wide spans</p><p class="lab-fig-v">{total_wide} <span class="ae-dim">/ {n_defects} keys</span></p></div>
    <div class="lab-fig"><p class="lab-fig-k">threshold</p><p class="lab-fig-v">&gt;{threshold}</p></div>
  </div>
  <p class="ae-chrome" style="margin-top:0.7em">{ci} <span class="ae-h">CONTAMINATION</span> {cv}</p>
  {note}
  <p class="ae-chrome" style="margin-top:0.5em">{si} <span class="ae-h">REDTEAM SPAN AUDIT</span> {sv}</p>
  {incomplete}
</section>
"#,
        contam_state = match s.public() {
            Some(true) => "public",
            Some(false) => "resistant",
            None => "—",
        },
        total_wide = s.total_wide(),
        n_defects = s.n_defects(),
        threshold = s.wide_threshold(),
        ci = contam_icon,
        cv = esc(&contam_verdict),
        note = note_html,
        si = span_icon,
        sv = esc(&span_verdict),
        incomplete = incomplete_html,
    )
}

pub(super) fn leaderboard_section(
    cands: &Map<String, Value>,
    order: &[String],
    certified: &std::collections::HashSet<String>,
    recommended: Option<&str>,
) -> String {
    let mut rows = String::new();
    for cid in order {
        let c = &cands[cid];
        let reference = is_reference(c);
        let model = c
            .get("model")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("—");
        let reward = c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
        let per = opt_money(c.get("cost_per_trial"));
        let total = opt_money(c.get("cost"));
        let wall = c.get("wall_mean").and_then(Value::as_f64).unwrap_or(0.0);
        let trials = c.get("trials").and_then(Value::as_u64).unwrap_or(0);
        let voided = c.get("voided").and_then(Value::as_u64).unwrap_or(0);

        let is_rec = recommended == Some(cid.as_str());
        let status = if reference {
            "<span class=\"lab-state\">reference</span>".to_string()
        } else if is_rec {
            format!(
                "{} <span class=\"lab-state\">recommended</span>",
                icon("i-check", "ae-ok")
            )
        } else if certified.contains(cid) {
            format!("{} certified", icon("i-check", "ae-ok"))
        } else {
            "<span class=\"lab-state\">trial</span>".to_string()
        };
        let row_class = if is_rec {
            " class=\"rep-rec\""
        } else if reference {
            " class=\"mx-ref\""
        } else {
            ""
        };
        let voided_cell = if voided > 0 {
            format!("<span style=\"color:var(--ae-warn)\">{voided}</span>")
        } else {
            "0".to_string()
        };
        let _ = write!(
            rows,
            r#"<tr{row_class}>
  <th scope="row" class="rep-cand">{cid}<span class="rep-model">{model}</span></th>
  <td class="rep-num{best}">{reward:.2}</td>
  <td class="rep-num">{per}</td>
  <td class="rep-num">{total}</td>
  <td class="rep-num">{wall:.1}s</td>
  <td class="rep-num">{trials}</td>
  <td class="rep-num">{voided_cell}</td>
  <td>{status}</td>
</tr>
"#,
            cid = esc(cid),
            model = esc(model),
            best = if is_rec { " rep-best" } else { "" },
        );
    }

    format!(
        r#"<section style="margin-bottom:1.8em">
  <p class="ae-plate-cap">LEADERBOARD · candidates by mean reward</p>
  <div class="mx-scroll">
    <table class="rep-board">
      <thead><tr>
        <th scope="col">candidate</th><th scope="col" class="rep-num">reward</th>
        <th scope="col" class="rep-num">$/trial</th><th scope="col" class="rep-num">$ total</th>
        <th scope="col" class="rep-num">wall</th><th scope="col" class="rep-num">trials</th>
        <th scope="col" class="rep-num">voided</th><th scope="col">status</th>
      </tr></thead>
      <tbody>
{rows}      </tbody>
    </table>
  </div>
</section>
"#
    )
}

pub(super) fn stats_section(
    baseline_id: &str,
    ci_rows: &[(String, Ci)],
    consistency: &Map<String, Value>,
    order: &[String],
    cands: &Map<String, Value>,
) -> String {
    let has_real = order.iter().any(|cid| !is_reference(&cands[cid]));
    let mut s = String::from(
        r#"<section style="margin-bottom:1.8em">
  <p class="ae-plate-cap">STATISTICS · is a win real? · the 95% CI even commercial eval tools omit</p>
"#,
    );
    if !has_real {
        s.push_str("</section>\n");
        return s;
    }

    if !ci_rows.is_empty() {
        s.push_str(&forest_block(baseline_id, ci_rows));
    } else {
        // No forest: this run predates CI persistence. Recomputing here would
        // need the arena's source-repo clustering, which a bare run-dir does not
        // carry — and a per-task guess yields anticonservative bars that can
        // contradict the run's own certification. Show the gap honestly.
        let _ = writeln!(
            s,
            "  <p class=\"ae-chrome\" style=\"margin-bottom:0.6em\">{} Reward-delta CIs were not recorded by this run — re-run to draw the forest. The leaderboard's certified marks still reflect the run's own clustered certification; reliability below is recomputed from the trials.</p>",
            icon("i-alert", "ae-warn"),
        );
    }
    s.push_str(&stats_table(order, cands, ci_rows, consistency));
    s.push_str("</section>\n");
    s
}

/// The caterpillar: one band per candidate on a shared delta axis, with the zero
/// line drawn so a CI crossing it reads as "not yet certified".
fn forest_block(baseline_id: &str, ci_rows: &[(String, Ci)]) -> String {
    let (axis_lo, axis_hi) = ci_axis(ci_rows);
    let span = (axis_hi - axis_lo).max(1e-9);
    let pos = |x: f64| ((x - axis_lo) / span * 100.0).clamp(0.0, 100.0);
    let zero = pos(0.0);

    let mut s = String::new();
    let _ = write!(
        s,
        "  <p class=\"ae-chrome\" style=\"margin-bottom:0.6em\">Δ reward vs <span class=\"ae-num\">{}</span> · cluster-robust 95% CI from the run's record · the zero line is the baseline</p>\n  <div class=\"rep-forest\">\n",
        esc(baseline_id),
    );
    for (cid, ci) in ci_rows {
        let point = ci.point;
        let lo = ci.lo;
        let hi = ci.hi;
        let sig_mark = if ci.excludes_zero {
            icon("i-check", "ae-ok")
        } else {
            icon("i-minus", "ae-warn")
        };
        let _ = write!(
            s,
            r#"    <div class="rep-forest-label"><span class="ae-item">{cid}</span> <span class="ae-num ae-dim">{point:+.3}</span> {sig_mark}{warn}</div>
    <div class="rep-forest-track">
      <span class="rep-zero" style="left:{zero:.2}%"></span>
      <span class="ae-ci"><span class="ae-ci-band" style="left:{l:.2}%;right:{r:.2}%"></span><span class="ae-ci-mean" style="left:{m:.2}%"></span></span>
      <span class="rep-forest-ci ae-num ae-dim">[{lo:+.3}, {hi:+.3}]</span>
    </div>
"#,
            cid = esc(cid),
            warn = if ci.small_n() {
                format!(" {}", icon("i-alert", "ae-warn"))
            } else {
                String::new()
            },
            l = pos(lo),
            r = 100.0 - pos(hi),
            m = pos(point),
        );
    }
    s.push_str("  </div>\n");
    s
}

/// The exact numbers behind the caterpillar, including the 039 power note and
/// reliability, for anyone who needs to cite them.
fn stats_table(
    order: &[String],
    cands: &Map<String, Value>,
    ci_rows: &[(String, Ci)],
    consistency: &Map<String, Value>,
) -> String {
    let mut s = String::from(
        r#"  <div class="mx-scroll" style="margin-top:1.2em">
    <table class="rep-board">
      <thead><tr>
        <th scope="col">candidate</th><th scope="col" class="rep-num">Δ reward</th>
        <th scope="col" class="rep-num">95% CI</th><th scope="col" class="rep-num">tasks</th>
        <th scope="col" class="rep-num">clusters</th><th scope="col" class="rep-num">clstr→95%</th>
        <th scope="col" class="rep-num">pass rate</th><th scope="col" class="rep-num">pass^k</th>
        <th scope="col">sig</th>
      </tr></thead>
      <tbody>
"#,
    );
    let ci_by_id: HashMap<&str, &Ci> = ci_rows.iter().map(|(k, v)| (k.as_str(), v)).collect();
    for cid in order {
        if is_reference(&cands[cid]) {
            continue;
        }
        let ci = ci_by_id.get(cid.as_str()).copied();
        let con = consistency.get(cid);
        let dpt = or_dash(ci.map(|c| c.point), |x| format!("{x:+.4}"));
        let interval = or_dash(ci, |c| format!("[{:+.4}, {:+.4}]", c.lo, c.hi));
        let ntasks = or_dash(ci.map(|c| c.n_tasks), |n| n.to_string());
        let nclusters = or_dash(ci.map(|c| c.n_clusters), |n| n.to_string());
        let need = or_dash(ci.and_then(|c| c.min_clusters_95), |n| n.to_string());
        let rate = or_dash(
            con.and_then(|c| c.get("rate")).and_then(Value::as_f64),
            |x| format!("{x:.2}"),
        );
        let passk = or_dash(
            con.and_then(|c| c.get("pass_k")).and_then(Value::as_f64),
            |x| format!("{x:.2}"),
        );
        let sig = ci.map(|c| c.excludes_zero).unwrap_or(false);
        let _ = writeln!(
            s,
            "        <tr><th scope=\"row\" class=\"rep-cand\">{cid}</th><td class=\"rep-num\">{dpt}</td><td class=\"rep-num\">{interval}</td><td class=\"rep-num\">{ntasks}</td><td class=\"rep-num\">{nclusters}</td><td class=\"rep-num\">{need}</td><td class=\"rep-num\">{rate}</td><td class=\"rep-num\">{passk}</td><td>{sigmark}</td></tr>",
            cid = esc(cid),
            sigmark = if sig { icon("i-check", "ae-ok") } else { icon("i-minus", "ae-warn") },
        );
    }
    s.push_str("      </tbody>\n    </table>\n  </div>\n");
    s.push_str("  <p class=\"ae-plate-note\" style=\"border:0;padding:0;margin-top:0.8em\"><span class=\"ae-num\">clstr→95%</span> is the power note: the cluster count at which the observed effect would just reach significance — adding trials inside existing clusters does not shrink the interval, clusters do. The <span class=\"ae-num\">!</span> flag marks intervals too thin to bound (n&lt;2).</p>\n");
    s
}

pub(super) fn heatmap_section(
    cands: &Map<String, Value>,
    tasks: &[String],
    order: &[String],
    recommended: Option<&str>,
) -> String {
    let mut head = String::from("        <th class=\"mx-corner\" scope=\"col\">task</th>\n");
    for cid in order {
        let c = &cands[cid];
        let reference = is_reference(c);
        let model = c
            .get("model")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(short_model)
            .unwrap_or_else(|| {
                if reference {
                    "ref".into()
                } else {
                    "—".into()
                }
            });
        let class = if recommended == Some(cid.as_str()) {
            " class=\"mx-pick\""
        } else if reference {
            " class=\"mx-ref\""
        } else {
            ""
        };
        let _ = writeln!(
            head,
            "        <th scope=\"col\"{class}>{cid}<span class=\"cand-meta\">{model}</span></th>",
            cid = esc(cid),
            model = esc(&model),
        );
    }
    head.push_str("        <th scope=\"col\">n</th>\n");

    let mut rows = String::new();
    let mut foot = String::from("        <th class=\"mx-task\" scope=\"row\">mean · /100</th>\n");
    for (ti, task) in tasks.iter().enumerate() {
        // Best non-reference mean on this task → heavy figure (the lawful winner).
        let best = order
            .iter()
            .filter(|cid| !is_reference(&cands[*cid]))
            .filter_map(|cid| cell_mean(&cands[cid], task).map(|(m, _)| m))
            .fold(f64::NEG_INFINITY, f64::max);
        let mut n_row = 0u64;
        let mut cells = String::new();
        for (ci, cid) in order.iter().enumerate() {
            let c = &cands[cid];
            let reference = is_reference(c);
            match cell_mean(c, task) {
                Some((mean, n)) => {
                    n_row = n_row.max(n as u64);
                    let is_best =
                        !reference && (mean - best).abs() < 1e-9 && best > f64::NEG_INFINITY;
                    let (ic, cls) = glyph_for(mean, reference);
                    let anchor = if reference {
                        // references bound the verifier; no transcript to drill.
                        format!(
                            "<span class=\"cell {cls}\"><span class=\"cell-head\">{ic}<span class=\"cell-fig\">{fig}</span></span></span>",
                            fig = pct(mean),
                        )
                    } else {
                        // Anchor by position (candidate index, task index), not a
                        // slug of the ids: punctuation-only differences in model
                        // ids would otherwise collapse two cells to one anchor and
                        // drill into the wrong transcript. The drill emits the same
                        // c{ci}-t{ti} ids over the same order/tasks.
                        format!(
                            "<a class=\"cell {cls}{best}\" href=\"#trial-c{ci}-t{ti}\"><span class=\"cell-head\">{ic}<span class=\"cell-fig\">{fig}</span></span></a>",
                            best = if is_best { " is-best" } else { "" },
                            fig = pct(mean),
                        )
                    };
                    let _ = writeln!(cells, "        <td>{anchor}</td>");
                }
                None => {
                    cells.push_str("        <td><span class=\"cell is-none\">—</span></td>\n");
                }
            }
        }
        let n_cell = if n_row < 5 {
            format!("<span style=\"color:var(--ae-warn)\">{n_row} !</span>")
        } else {
            n_row.to_string()
        };
        let _ = write!(
            rows,
            "      <tr>\n        <th class=\"mx-task\" scope=\"row\">{task}</th>\n{cells}        <td>{n_cell}</td>\n      </tr>\n",
            task = esc(task),
        );
    }
    for cid in order {
        let c = &cands[cid];
        let rm = c.get("reward_mean").and_then(Value::as_f64).unwrap_or(0.0);
        let cls = if is_reference(c) {
            " class=\"mx-ref\""
        } else {
            ""
        };
        let _ = writeln!(foot, "        <td{cls}>{}</td>", pct(rm));
    }
    foot.push_str("        <td></td>\n");

    let recommended_legend = if recommended.is_some() {
        "    <span class=\"lg\" style=\"color:var(--ae-accent)\">recommended column accented</span>\n"
    } else {
        ""
    };

    format!(
        r#"<section style="margin-bottom:1.8em">
  <p class="ae-plate-cap">COVERAGE · task × candidate · reward 0–100 · click a cell for its transcript</p>
  <div class="mx-scroll">
    <table class="matrix">
      <thead><tr>
{head}      </tr></thead>
      <tbody>
{rows}      </tbody>
      <tfoot><tr>
{foot}      </tr></tfoot>
    </table>
  </div>
  <div class="mx-legend">
    <span class="lg">{ok} pass</span>
    <span class="lg">{warn} partial</span>
    <span class="lg">{err} miss</span>
{recommended_legend}
    <span class="lg"><span style="color:var(--ae-warn)">!</span> n &lt; 5 · thin</span>
  </div>
</section>
"#,
        ok = icon("i-check", "ae-ok"),
        warn = icon("i-minus", "ae-warn"),
        err = icon("i-x", "ae-err"),
        recommended_legend = recommended_legend,
    )
}

pub(super) fn drill_section(
    records: &[Value],
    cands: &Map<String, Value>,
    order: &[String],
    tasks: &[String],
) -> String {
    let mut blocks = String::new();
    // Iterate the SAME order/tasks the heatmap uses, so the c{ci}-t{ti} anchors
    // line up exactly with the heatmap's cell links.
    for (ci, cid) in order.iter().enumerate() {
        if is_reference(&cands[cid]) {
            continue;
        }
        for (ti, task) in tasks.iter().enumerate() {
            let Some(trial) = representative_trial(records, cid, task) else {
                continue;
            };
            blocks.push_str(&drill_block(ci, ti, cid, task, trial));
        }
    }
    if blocks.is_empty() {
        return String::new();
    }
    format!(
        r#"<section>
  <p class="ae-plate-cap">TRANSCRIPTS · the evidence behind every score</p>
{blocks}</section>
"#
    )
}

fn drill_block(ci: usize, ti: usize, cid: &str, task: &str, trial: &Value) -> String {
    let reward = trial.get("reward").and_then(Value::as_f64).unwrap_or(0.0);
    let recall = trial.get("recall").and_then(Value::as_f64);
    let fp = trial
        .get("false_positives")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let expected = trial
        .get("expected_defects")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let matched = trial
        .get("matched")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let cost = opt_money(trial.get("cost_usd"));
    let wall = trial.get("wall_ms").and_then(Value::as_f64).unwrap_or(0.0) / 1000.0;
    let errored = trial.get("error").map(|e| !e.is_null()).unwrap_or(false);
    let scorer_error = trial
        .get("scorer_error")
        .map(|e| !e.is_null())
        .unwrap_or(false);

    let (vi, verdict) = if errored || scorer_error {
        (
            icon("i-alert", "ae-err"),
            "infrastructure error — not a clean score",
        )
    } else if reward >= 0.999 {
        (icon("i-check", "ae-ok"), "matched the keyed defect")
    } else if reward > 0.0 {
        (
            icon("i-minus", "ae-warn"),
            "partial — some defects matched, others missed or false-flagged",
        )
    } else if fp > 0 {
        (
            icon("i-x", "ae-err"),
            "false positives, no keyed match — noise, not signal",
        )
    } else {
        (
            icon("i-x", "ae-err"),
            "a clean miss — valid empty output, defect left unfound",
        )
    };

    // Findings the candidate emitted, vs the scorer's match summary.
    let findings = trial.get("findings").and_then(Value::as_array);
    let findings_html = match findings {
        Some(arr) if !arr.is_empty() => {
            let mut f = String::new();
            for finding in arr {
                let file = finding.get("file").and_then(Value::as_str).unwrap_or("—");
                let line = finding.get("line").and_then(Value::as_i64);
                let cat = finding.get("category").and_then(Value::as_str).unwrap_or("");
                let sev = finding.get("severity").and_then(Value::as_str).unwrap_or("");
                let desc = finding.get("description").and_then(Value::as_str).unwrap_or("");
                let _ = write!(
                    f,
                    "<pre class=\"lab-code\"><span class=\"k\">{file}</span>{line} <span class=\"ae-dim\">{cat} · {sev}</span>\n{desc}</pre>",
                    file = esc(file),
                    line = line.map(|l| format!(":{l}")).unwrap_or_default(),
                    cat = esc(cat),
                    sev = esc(sev),
                    desc = esc(desc),
                );
            }
            f
        }
        _ => "<div class=\"ae-empty\" style=\"border:0;padding:0.8em 0\"><p class=\"ae-item\">No findings emitted.</p><p class=\"ae-dim\">The candidate returned <span class=\"ae-num\">[]</span> — a valid, empty result, scored as a clean miss.</p></div>".to_string(),
    };

    format!(
        r#"  <details id="trial-c{ci}-t{ti}" class="rep-trial">
    <summary><span class="rep-trial-h">{cid} <span class="ae-dim">×</span> {task}</span> {vi} <span class="ae-num">{reward:.2}</span> <span class="ae-dim">reward</span></summary>
    <div class="lab-cols is-2" style="margin-top:1em">
      <div>
        <div class="lab-figrow">
          <div class="lab-fig"><p class="lab-fig-k">recall</p><p class="lab-fig-v">{matched} / {expected}{recall}</p></div>
          <div class="lab-fig"><p class="lab-fig-k">false pos</p><p class="lab-fig-v">{fp}</p></div>
          <div class="lab-fig"><p class="lab-fig-k">cost</p><p class="lab-fig-v">{cost}</p></div>
          <div class="lab-fig"><p class="lab-fig-k">wall</p><p class="lab-fig-v">{wall:.0}s</p></div>
        </div>
        <p class="ae-h" style="margin-top:1.1em">VERDICT</p>
        <p>{vi} {verdict}</p>
      </div>
      <div>
        <p class="ae-h">CANDIDATE FINDINGS · scored vs the hidden key</p>
        {findings_html}
      </div>
    </div>
  </details>
"#,
        cid = esc(cid),
        task = esc(task),
        recall = match recall {
            Some(r) => format!(" <span class=\"ae-dim\">· {r:.2}</span>"),
            None => String::new(),
        },
        verdict = esc(verdict),
    )
}

pub(super) fn footer_section(loop_json: &Value) -> String {
    let baseline = loop_json
        .get("reward_delta_baseline")
        .and_then(Value::as_str)
        .unwrap_or("null");
    format!(
        r#"<footer style="margin-top:2em;border-top:1px solid var(--ae-line);padding-top:1.2em">
  <p class="ae-plate-note" style="border:0;padding:0">References (oracle / null / one-shot probe / incumbent when declared) bound the verifier and are excluded from the leaderboard ranking, Pareto set, and recommendation: oracle and null fix the ceiling and floor; the one-shot probe only detects arena saturation; the incumbent is the baseline-to-beat. Every recommendable candidate is an agent composition, certified only when its reward-delta 95% CI clears the selected baseline (<span class="ae-num">{baseline}</span>). Generated from <span class="ae-num">trials.jsonl</span> + <span class="ae-num">loop.json</span> — a derived view, never the source of truth.</p>
</footer>
"#,
        baseline = esc(baseline),
    )
}

// ---------------------------------------------------------------------------
// Formatting primitives
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn pct(reward: f64) -> i64 {
    (reward * 100.0).round() as i64
}

fn opt_money(v: Option<&Value>) -> String {
    match v {
        Some(Value::Null) | None => "—".to_string(),
        Some(x) => match x.as_f64() {
            Some(f) => format!("${f:.4}"),
            None => "—".to_string(),
        },
    }
}

fn glyph_for(mean: f64, reference: bool) -> (String, &'static str) {
    if reference {
        // bounds, not competitors: muted glyph, no win/lose hue.
        let id = if mean >= 0.999 { "i-check" } else { "i-x" };
        (icon(id, "ae-dim"), "")
    } else if mean >= 0.999 {
        (icon("i-check", "ae-ok"), "")
    } else if mean > 0.0 {
        (icon("i-minus", "ae-warn"), "")
    } else {
        (icon("i-x", "ae-err"), "is-fail")
    }
}

fn icon(id: &str, class: &str) -> String {
    format!("<svg class=\"ae-icon {class}\"><use href=\"#{id}\" /></svg>")
}

fn short_model(m: &str) -> String {
    // Drop the provider prefix for column headers: "z-ai/glm-4.7-flash" → "glm-4.7-flash".
    m.rsplit('/').next().unwrap_or(m).to_string()
}

// ---------------------------------------------------------------------------
// Document shell
// ---------------------------------------------------------------------------

/// The vendored Misty Step design system + the Daedalus lab extensions, inlined
/// into every report so the file is self-contained and offline. Provenance and
/// sync instructions: `crates/daedalus-core/assets/VENDORED.md`.
const AESTHETIC_CSS: &str = include_str!("../../assets/aesthetic.css");
const LAB_CSS: &str = include_str!("../../assets/lab.css");

const SPRITE: &str = r##"<svg style="display:none" xmlns="http://www.w3.org/2000/svg">
  <symbol id="i-check" viewBox="0 0 24 24"><path d="M20 6 9 17l-5-5" /></symbol>
  <symbol id="i-x" viewBox="0 0 24 24"><path d="M18 6 6 18" /><path d="m6 6 12 12" /></symbol>
  <symbol id="i-minus" viewBox="0 0 24 24"><path d="M5 12h14" /></symbol>
  <symbol id="i-alert" viewBox="0 0 24 24"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" /><path d="M12 9v4" /><path d="M12 17h.01" /></symbol>
  <symbol id="i-sun" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" /></symbol>
  <symbol id="i-moon" viewBox="0 0 24 24"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" /></symbol>
</svg>"##;

/// Report-specific CSS layered over the vendored system: this is a scrolling
/// document, not a fixed-viewport app shell, so the no-scroll law is lifted and
/// the desk is centred at a readable measure. The rest is the leaderboard table,
/// the forest tracks, and the drill disclosure — all in the house idiom.
const REPORT_CSS: &str = r#"
html.rep, html.rep body { height: auto; min-height: 100%; overflow-y: auto; }
html.rep body { display: block; }
.rep-desk { max-width: 72rem; margin: 0 auto; }
.rep-board { border-collapse: separate; border-spacing: 0; width: 100%; font-family: var(--ae-font-mono); font-size: 13px; }
.rep-board th, .rep-board td { padding: 0.5em 0.8em; border-bottom: 1px solid var(--ae-line); text-align: left; white-space: nowrap; }
.rep-board thead th { background: var(--ae-wash); color: var(--ae-ink-muted); font-weight: var(--ae-w-regular); letter-spacing: 0.06em; position: sticky; top: 0; }
.rep-board .rep-num { text-align: right; font-variant-numeric: tabular-nums; }
.rep-board .rep-cand { color: var(--ae-ink); font-weight: var(--ae-w-medium); }
.rep-board .rep-model { display: block; color: var(--ae-ink-faint); font-weight: var(--ae-w-regular); letter-spacing: 0.04em; }
.rep-board tr.rep-rec th, .rep-board tr.rep-rec .rep-best { color: var(--ae-accent); font-weight: var(--ae-w-black); }
.rep-board tr.mx-ref, .rep-board tr.mx-ref .rep-cand { color: var(--ae-ink-faint); }
.cell { text-decoration: none; }
a.cell:hover .cell-fig { color: var(--ae-accent); }
.rep-forest { display: grid; grid-template-columns: minmax(12em, 18em) minmax(0, 1fr); gap: 0.5em 1.2em; align-items: center; }
.rep-forest-label { font-size: 13px; }
.rep-forest-track { position: relative; display: flex; align-items: center; gap: 0.8em; }
.rep-forest-track .ae-ci { flex: 1 1 auto; }
.rep-forest-ci { flex: none; font-size: 11px; }
.rep-zero { position: absolute; top: -2px; bottom: -2px; width: 1px; background: var(--ae-ink-faint); z-index: 1; }
.rep-trial { border-top: 1px solid var(--ae-line); padding: 0.6em 0; }
.rep-trial summary { cursor: pointer; font-size: 14px; }
.rep-trial summary::marker { color: var(--ae-ink-faint); }
.rep-trial-h { font-weight: var(--ae-w-medium); }
.rep-trial .lab-code { background: var(--ae-wash); padding: 0.8em 1em; margin: 0.5em 0; border: 1px solid var(--ae-line); }
"#;

const MODE_JS: &str = r#"<script>
(function(){
  var root = document.documentElement;
  try { var m = localStorage.getItem('ae-mode'); if (m==='dark'||m==='light'){ root.classList.add(m); root.style.colorScheme=m; } } catch(e){}
  var btn = document.querySelector('.ae-mode');
  if (btn) btn.addEventListener('click', function(){
    var dark = root.classList.contains('dark');
    root.classList.remove('dark','light');
    var next = dark ? 'light' : 'dark';
    root.classList.add(next); root.style.colorScheme = next;
    try { localStorage.setItem('ae-mode', next); } catch(e){}
  });
})();
</script>"#;

pub(super) fn document(run_label: &str, arena_id: &str, body: &str) -> String {
    format!(
        r##"<!doctype html>
<html lang="en" class="rep">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Daedalus — {arena} · {run}</title>
<style>
{aesthetic}
{lab}
{report}
</style>
</head>
<body>
{sprite}
<main class="ae-desk lab-desk rep-desk">
<div style="display:flex;justify-content:flex-end;margin-bottom:0.4em">
  <button class="ae-mode" aria-label="toggle color mode"><svg class="ae-icon ae-sun"><use href="#i-sun" /></svg><svg class="ae-icon ae-moon"><use href="#i-moon" /></svg></button>
</div>
{body}</main>
{mode_js}
</body>
</html>
"##,
        arena = esc(arena_id),
        run = esc(run_label),
        aesthetic = AESTHETIC_CSS,
        lab = LAB_CSS,
        report = REPORT_CSS,
        sprite = SPRITE,
        body = body,
        mode_js = MODE_JS,
    )
}
