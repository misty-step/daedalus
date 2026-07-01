//! The `threshold` CLI.
//!
//! Every subcommand delegates to its counterpart in `threshold_core`; this file
//! is pure composition root (arg parsing + wiring). See `docs/rust-migration.md`
//! for migration status.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{Map, Value};

use threshold_core::mutate::call_optimizer;
use threshold_core::pyrandom::PyRandom;
use threshold_core::run::{summarize, toml_to_json, ArenaInputs};
use threshold_core::search_loop::{
    is_reference, known_spend, run_search, SearchParams, SearchWorld,
};

// ---------------------------------------------------------------------------
// Top-level CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "threshold", about = "Threshold agent foundry CLI")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Score findings against an answer key.
    Score {
        findings: PathBuf,
        expected: PathBuf,
    },
    /// Render committed run records as trace.otel.json.
    Trace {
        #[arg(long = "run-dir")]
        run_dir: PathBuf,
    },
    /// Render a run directory as a self-contained static HTML report — the
    /// visual companion to report.md (leaderboard, CI forest, coverage heatmap,
    /// transcript drill). Opens offline from file://; PR-attachable.
    ReportHtml {
        run_dir: PathBuf,
        /// Output path. Default: `<run-dir>/report.html`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Watch a run in flight: a live terminal roll-up (per-candidate running
    /// mean, trials so far, cumulative known spend) that polls trials.jsonl and
    /// reprints until the run completes. The live companion to report-html.
    View {
        run_dir: PathBuf,
        /// Print one snapshot and exit (no follow) — for scripts and CI.
        #[arg(long)]
        once: bool,
        /// Seconds between refreshes while following.
        #[arg(long, default_value_t = 2)]
        interval: u64,
    },
    /// Basin-trap detector: compare the certified tops of >=2 seed runs and flag
    /// when different seeds crown different compositions beyond the pooled noise.
    Basin {
        /// Run directories (each with a pareto.json), one per seed trajectory.
        run_dirs: Vec<PathBuf>,
    },
    /// Export a delivery as control-plane artifacts.
    Export {
        delivery: PathBuf,
        #[arg(long)]
        spec: PathBuf,
    },
    /// Export a Cerberus ReviewerConfigPacket.v1 JSON handoff.
    ExportCerberus {
        delivery: PathBuf,
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Import Cerberus review artifacts into Threshold lab evidence.
    CerberusLab {
        #[command(subcommand)]
        command: CerberusLabCmd,
    },
    /// Export a review-swarm suite contract.
    ExportSuite {
        delivery: PathBuf,
        #[arg(long)]
        suite: PathBuf,
    },
    /// Emit approval-aware control-plane import packets.
    LaunchPack {
        delivery: PathBuf,
        #[arg(long)]
        plane: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
    /// Replay a delivery against the arena holdout and emit trace output.
    Regression {
        delivery: PathBuf,
        #[arg(long)]
        spec: PathBuf,
        #[arg(long, default_value_t = 5)]
        trials: u32,
        #[arg(long)]
        exp_dir: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Create Harbor-format task placeholders.
    ArenaScaffold {
        arena: PathBuf,
        task_id: String,
        #[arg(long, default_value = "specs/TODO/taskspec.toml")]
        taskspec: String,
    },
    /// Validate an arena freeze gate without model spend.
    ArenaValidate {
        arena: PathBuf,
        #[arg(long)]
        probe_run: Option<PathBuf>,
        #[arg(long, default_value_t = 5)]
        holdout_burn: i64,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Generate an arena freeze packet: oracle, null, one-shot probe, report.
    ArenaFreeze {
        arena: PathBuf,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long)]
        report: Option<PathBuf>,
        #[arg(long, default_value_t = 5)]
        holdout_burn: i64,
        #[arg(long)]
        probe_model: Option<String>,
        #[arg(long)]
        probe_context_window: Option<u64>,
    },
    /// Red-team an arena's answer keys: flag wide line-spans a candidate could
    /// game by guessing file+category without locating the defect (040).
    ArenaRedteam {
        arena: PathBuf,
        /// Spans wider than this (lines) are flagged as gameable.
        #[arg(long, default_value_t = 8)]
        wide_threshold: i64,
    },
    /// Append an ACCEPT or OUT-OF-SCOPE adjudication.
    ArenaAdjudicate {
        arena: PathBuf,
        #[arg(long)]
        task: String,
        #[arg(long)]
        finding: String,
        #[arg(long)]
        ruling: String,
        #[arg(long)]
        rationale: String,
        #[arg(long)]
        new_version: Option<String>,
        #[arg(long)]
        baseline_run: Option<PathBuf>,
    },
    /// Report category/span misses for findings vs an answer key.
    ArenaDisagreements {
        #[arg(long)]
        findings: PathBuf,
        #[arg(long)]
        expected: PathBuf,
    },
    /// Validate a review-swarm taxonomy against a suite taskspec.
    TaxonomyValidate {
        taxonomy: PathBuf,
        #[arg(long)]
        suite: PathBuf,
    },
    /// Print cold-start readiness checks without model spend.
    Doctor {
        #[arg(long, default_value_t = 30)]
        stale_days: i64,
        /// Override current date for tests (YYYY-MM-DD).
        #[arg(long)]
        today: Option<String>,
    },
    /// Port an arena into Harbor format.
    PortHarbor {
        arena: PathBuf,
        #[arg(long, default_value = "harbor-build")]
        out: String,
        /// Path to the prebuilt threshold-score musl binary.
        #[arg(long)]
        scorer_bin: PathBuf,
    },
    /// Search compositions for a task spec.
    Run {
        taskspec: PathBuf,
        #[arg(long)]
        arena: Option<PathBuf>,
        #[arg(long, default_value_t = 2.0)]
        budget_usd: f64,
        #[arg(long, default_value_t = 6)]
        max_candidates: usize,
        #[arg(long, default_value_t = 3)]
        trials: u32,
        /// Model that generates and mutates candidate configs. Needs strong
        /// reasoning + reliable structured output; cost-tolerant (low volume).
        /// Default = DeepSeek V4 Pro (SOTA reasoning + structured output at
        /// ~1/10 frontier price). Escalate to `openai/gpt-5.5` or
        /// `anthropic/claude-opus-4.8` for a high-stakes final search.
        #[arg(long, default_value = "deepseek/deepseek-v4-pro")]
        optimizer_model: String,
        #[arg(long, default_value_t = 2)]
        plateau: usize,
        #[arg(long)]
        allow_saturated: bool,
        #[arg(long)]
        rng_seed: Option<u64>,
        #[arg(long, default_value_t = 2)]
        children_per_gen: usize,
        #[arg(long, default_value_t = 3)]
        certify_top: usize,
        #[arg(long, default_value_t = 5)]
        certify_trials: u32,
        /// Minimum detectable effect for certification: a candidate certifies
        /// only when its reward-delta 95% CI lower bound against the selected
        /// baseline exceeds this. 0.0 = "provably better than baseline."
        #[arg(long, default_value_t = 0.0)]
        min_effect: f64,
        /// Reward a trial must reach to count as a "pass" for the reliability
        /// (pass-rate / pass^k) metric. 1.0 = perfect trials only; lower it to
        /// discriminate mid-tier candidates.
        #[arg(long, default_value_t = 1.0)]
        consistency_floor: f64,
        /// Reliability gate (056): a candidate is recommended only when its
        /// pass^k (k = --certify-trials, at the --consistency-floor reward) is at
        /// least this. 0.0 = gate off (pre-056 behaviour). A high mean over a
        /// config that fails most of its runs is not deployable (τ-bench).
        #[arg(long, default_value_t = 0.0)]
        reliability_floor: f64,
        #[arg(long)]
        max_errors_per_candidate: Option<usize>,
        /// Offline cost/scale forecast: project the trial count and (when the
        /// taskspec declares a per-trial ceiling) the worst-case cost, then exit
        /// before any trial runs. No spend, no `runs/` directory created.
        #[arg(long)]
        estimate: bool,
    },
    /// Offline two-run delta: compare two existing run directories' pareto.json +
    /// loop.json without spending. Per-candidate reward/rank/cost deltas, plus
    /// spend and stop-reason deltas, so cross-run comparison is mechanical.
    Compare { run_a: PathBuf, run_b: PathBuf },
}

#[derive(Subcommand)]
enum CerberusLabCmd {
    /// Import and optionally score one Cerberus ReviewArtifact.v1.
    Import {
        #[arg(long)]
        arena: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long)]
        candidate_id: String,
        #[arg(long)]
        substrate: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        transcript: Option<PathBuf>,
        #[arg(long)]
        receipt: Option<PathBuf>,
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Compare previously imported Cerberus lab run directories.
    Compare {
        #[arg(long = "run-dir", required = true, num_args = 1..)]
        run_dirs: Vec<PathBuf>,
        #[arg(long)]
        out_dir: PathBuf,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Score { findings, expected } => cmd_score(&findings, &expected),
        Cmd::Trace { run_dir } => cmd_trace(&run_dir),
        Cmd::ReportHtml { run_dir, out } => cmd_report_html(&run_dir, out.as_deref()),
        Cmd::View {
            run_dir,
            once,
            interval,
        } => cmd_view(&run_dir, once, interval),
        Cmd::Basin { run_dirs } => cmd_basin(&run_dirs),
        Cmd::Export { delivery, spec } => cmd_export(&delivery, &spec),
        Cmd::ExportCerberus {
            delivery,
            spec,
            out,
        } => cmd_export_cerberus(&delivery, &spec, &out),
        Cmd::CerberusLab { command } => match command {
            CerberusLabCmd::Import {
                arena,
                request,
                artifact,
                candidate_id,
                substrate,
                model,
                task_id,
                transcript,
                receipt,
                out_dir,
            } => cmd_cerberus_lab_import(threshold_core::cerberus_lab::ImportOptions {
                arena,
                request,
                artifact,
                candidate_id,
                substrate,
                model,
                task_id,
                transcript,
                receipt,
                out_dir,
                repo_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            }),
            CerberusLabCmd::Compare { run_dirs, out_dir } => {
                cmd_cerberus_lab_compare(threshold_core::cerberus_lab::CompareOptions {
                    run_dirs,
                    out_dir,
                })
            }
        },
        Cmd::ExportSuite { delivery, suite } => cmd_export_suite(&delivery, &suite),
        Cmd::LaunchPack {
            delivery,
            plane,
            dry_run,
            out_dir,
        } => cmd_launch_pack(&delivery, &plane, dry_run, out_dir.as_deref()),
        Cmd::Regression {
            delivery,
            spec,
            trials,
            exp_dir,
            dry_run,
        } => cmd_regression(&delivery, &spec, trials, exp_dir.as_deref(), dry_run),
        Cmd::ArenaScaffold {
            arena,
            task_id,
            taskspec,
        } => cmd_arena_scaffold(&arena, &task_id, &taskspec),
        Cmd::ArenaValidate {
            arena,
            probe_run,
            holdout_burn,
            report,
        } => cmd_arena_validate(
            &arena,
            probe_run.as_deref(),
            holdout_burn,
            report.as_deref(),
        ),
        Cmd::ArenaFreeze {
            arena,
            out_dir,
            report,
            holdout_burn,
            probe_model,
            probe_context_window,
        } => cmd_arena_freeze(
            &arena,
            out_dir.as_deref(),
            report.as_deref(),
            holdout_burn,
            probe_model.as_deref(),
            probe_context_window,
        ),
        Cmd::ArenaRedteam {
            arena,
            wide_threshold,
        } => cmd_arena_redteam(&arena, wide_threshold),
        Cmd::ArenaAdjudicate {
            arena,
            task,
            finding,
            ruling,
            rationale,
            new_version,
            baseline_run,
        } => cmd_arena_adjudicate(
            &arena,
            &task,
            &finding,
            &ruling,
            &rationale,
            new_version.as_deref(),
            baseline_run.as_deref(),
        ),
        Cmd::ArenaDisagreements { findings, expected } => {
            cmd_arena_disagreements(&findings, &expected)
        }
        Cmd::TaxonomyValidate { taxonomy, suite } => cmd_taxonomy_validate(&taxonomy, &suite),
        Cmd::Doctor { stale_days, today } => cmd_doctor(stale_days, today.as_deref()),
        Cmd::PortHarbor {
            arena,
            out,
            scorer_bin,
        } => cmd_port_harbor(&arena, &out, &scorer_bin),
        Cmd::Run {
            taskspec,
            arena,
            budget_usd,
            max_candidates,
            trials,
            optimizer_model,
            plateau,
            allow_saturated,
            rng_seed,
            children_per_gen,
            certify_top,
            certify_trials,
            min_effect,
            consistency_floor,
            reliability_floor,
            max_errors_per_candidate,
            estimate,
        } => cmd_run(
            &taskspec,
            arena.as_deref(),
            budget_usd,
            max_candidates,
            trials,
            &optimizer_model,
            plateau,
            allow_saturated,
            rng_seed,
            children_per_gen,
            certify_top,
            certify_trials,
            min_effect,
            consistency_floor,
            reliability_floor,
            max_errors_per_candidate,
            estimate,
        ),
        Cmd::Compare { run_a, run_b } => cmd_compare(&run_a, &run_b),
    }
}

// ---------------------------------------------------------------------------
// score
// ---------------------------------------------------------------------------

fn cmd_score(findings: &std::path::Path, expected: &std::path::Path) -> ExitCode {
    match threshold_core::score::score(findings, expected) {
        Ok(result) => {
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// trace
// ---------------------------------------------------------------------------

fn cmd_trace(run_dir: &std::path::Path) -> ExitCode {
    match threshold_core::trace::write_trace(run_dir) {
        Ok(out) => {
            println!("trace: {}", out.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_report_html(run_dir: &std::path::Path, out: Option<&std::path::Path>) -> ExitCode {
    if !run_dir.join("trials.jsonl").exists() {
        eprintln!("no trials.jsonl in {}", run_dir.display());
        return ExitCode::FAILURE;
    }
    let html = match threshold_core::report_html::render_html(run_dir) {
        Ok(h) => h,
        Err(err) => {
            eprintln!("render report.html: {err}");
            return ExitCode::FAILURE;
        }
    };
    let dest = out
        .map(PathBuf::from)
        .unwrap_or_else(|| run_dir.join("report.html"));
    if let Err(err) = std::fs::write(&dest, &html) {
        eprintln!("write {}: {err}", dest.display());
        return ExitCode::FAILURE;
    }
    println!("report: {}", dest.display());
    ExitCode::SUCCESS
}

/// Watch a run in flight. Polls `trials.jsonl` and reprints the live roll-up
/// until `loop.json` appears (run complete) or, with `--once`, after one frame.
/// The poll/redraw loop is the only IO here; the roll-up and rendering are the
/// tested pure core in `threshold_core::view`.
fn cmd_view(run_dir: &std::path::Path, once: bool, interval: u64) -> ExitCode {
    use std::io::Write;
    if !run_dir.is_dir() {
        eprintln!("no such run dir: {}", run_dir.display());
        return ExitCode::FAILURE;
    }
    let label = run_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("run");
    let period = std::time::Duration::from_secs(interval.max(1));
    loop {
        let records = threshold_core::report::load_records(&[run_dir]);
        let loop_path = run_dir.join("loop.json");
        let complete = loop_path.exists();
        // At completion, loop.json carries the authoritative run-total spend.
        let auth_spend = complete
            .then(|| std::fs::read_to_string(&loop_path).ok())
            .flatten()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.get("spend_known_usd").and_then(Value::as_f64));
        // The headroom rig + the streamed hypotheses — both optional; the cockpit
        // degrades gracefully when a source is absent.
        let rig = std::fs::read_to_string(run_dir.join("rig.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok());
        let history = read_jsonl(&run_dir.join("loop.history.jsonl"));
        // The budget cap, persisted in seed.json at run start.
        let cap = std::fs::read_to_string(run_dir.join("seed.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.get("budget_usd").and_then(Value::as_f64));
        let snap = threshold_core::view::snapshot(&records)
            .with_rig(rig.as_ref())
            .with_hypotheses(&history, 6)
            .with_cap(cap);
        let body = threshold_core::view::render(&snap, label, complete, auth_spend);
        if once {
            print!("{body}");
        } else {
            // Clear screen + home cursor, then redraw the frame in place. (No
            // alt-screen: it would corrupt the user's terminal on Ctrl-C, which
            // skips any restore. The final frame persists on the normal buffer.)
            print!("\x1b[2J\x1b[H{body}");
        }
        let _ = std::io::stdout().flush();
        if once || complete {
            return ExitCode::SUCCESS;
        }
        std::thread::sleep(period);
    }
}

/// Read a `.jsonl` file into its parsed rows, skipping blank/unparseable lines.
/// Returns empty when the file is absent — callers treat that as "no data yet".
fn read_jsonl(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

// ---------------------------------------------------------------------------
// basin (039 child-4): trajectory-divergence detector over >=2 seed runs
// ---------------------------------------------------------------------------

/// A run's display label: its directory basename, or `"?"` when it has none.
/// Shared by the read-only multi-run readers (`read_run_top`,
/// `read_run_for_compare`).
fn run_dir_label(dir: &std::path::Path) -> String {
    dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string()
}

/// Read one seed run's certified top from its `pareto.json` (the recommended
/// candidate). `None` when the file is absent/unparseable or nothing certified.
fn read_run_top(dir: &std::path::Path) -> Option<threshold_core::stats::RunTop> {
    let text = std::fs::read_to_string(dir.join("pareto.json")).ok()?;
    let arr: Value = serde_json::from_str(&text).ok()?;
    let rec = arr
        .as_array()?
        .iter()
        .find(|c| c.get("recommended").and_then(Value::as_bool) == Some(true))?;
    Some(threshold_core::stats::RunTop {
        label: run_dir_label(dir),
        top_id: rec
            .get("candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string(),
        top_hash: rec
            .get("composition_hash")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        reward: rec
            .get("reward_mean")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        // `None` when the run carries no CI data (pre-039-child-1 runs) — the
        // detector then reports the gap as untestable rather than trivially
        // significant.
        se: rec
            .get("reward_delta_ci")
            .and_then(|c| c.get("se"))
            .and_then(Value::as_f64),
    })
}

fn cmd_basin(run_dirs: &[PathBuf]) -> ExitCode {
    let mut tops = Vec::new();
    for dir in run_dirs {
        match read_run_top(dir) {
            Some(t) => tops.push(t),
            None => eprintln!(
                "note: {} has no certified recommendation — skipped",
                dir.display()
            ),
        }
    }
    match threshold_core::stats::basin_divergence(&tops) {
        None => {
            eprintln!(
                "error: need >= 2 seed runs with a certified top (have {})",
                tops.len()
            );
            ExitCode::FAILURE
        }
        Some(v) => {
            println!("Basin-trap check over {} seed runs:", v.n_runs);
            for t in &tops {
                let short: String = t.top_hash.chars().take(12).collect();
                let se =
                    t.se.map(|s| format!("± se {s:.6}"))
                        .unwrap_or_else(|| "(no CI)".to_string());
                println!(
                    "  {} → {} (hash {}…, reward {:.4} {se})",
                    t.label, t.top_id, short, t.reward
                );
            }
            let pooled = v
                .pooled_se
                .map(|s| format!("{s:.6}"))
                .unwrap_or_else(|| "n/a".to_string());
            let gap_test = match v.gap_significant {
                Some(true) => "yes",
                Some(false) => "no",
                None => "untestable",
            };
            println!(
                "distinct winners: {}  reward gap: {:.4}  pooled SE: {pooled}  gap>noise: {gap_test}",
                v.distinct_winners, v.reward_gap
            );
            if v.missing_identity {
                println!(
                    "INDETERMINATE — at least one run has no composition hash; convergence cannot \
                     be asserted. Re-run the affected seeds."
                );
            } else if v.flag {
                println!(
                    "BASIN TRAP — seeds crown different compositions whose reward differs beyond \
                     pooled noise; the search is seed-dependent, not robust. Widen seeds/budget."
                );
            } else if !v.converged && v.gap_significant.is_none() {
                println!(
                    "DIVERGENT (untestable) — seeds crown different compositions, but the runs \
                     carry no CI data to test the gap against noise. Re-run with the post-039 CI \
                     emission for a verdict."
                );
            } else if !v.converged {
                println!(
                    "DIVERGENT (equivalent) — different winners, but within pooled noise: multiple \
                     equally-good optima, not a quality trap."
                );
            } else {
                println!("ROBUST — every seed converged to the same composition.");
            }
            ExitCode::SUCCESS
        }
    }
}

// ---------------------------------------------------------------------------
// compare (041): offline two-run delta over pareto.json + loop.json
// ---------------------------------------------------------------------------

/// Read one run's `pareto.json` + `loop.json` into a `compare::RunSummary`.
/// `None` when `pareto.json` is absent/unparseable; a missing `loop.json` is
/// tolerated (rank/spend/stop-reason simply stay empty). Mirrors the read-only
/// multi-run pattern of `read_run_top`.
fn read_run_for_compare(dir: &std::path::Path) -> Option<threshold_core::compare::RunSummary> {
    let pareto: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("pareto.json")).ok()?).ok()?;
    let loop_json: Value = std::fs::read_to_string(dir.join("loop.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);
    let label = run_dir_label(dir);
    Some(threshold_core::compare::summarize_run(
        &label, &pareto, &loop_json,
    ))
}

fn cmd_compare(run_a: &std::path::Path, run_b: &std::path::Path) -> ExitCode {
    let a = match read_run_for_compare(run_a) {
        Some(s) => s,
        None => {
            eprintln!(
                "error: {} has no readable pareto.json — not a completed run dir",
                run_a.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let b = match read_run_for_compare(run_b) {
        Some(s) => s,
        None => {
            eprintln!(
                "error: {} has no readable pareto.json — not a completed run dir",
                run_b.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let cmp = threshold_core::compare::compare(&a, &b);

    // Unknown is never 0: a missing delta prints "—" / "unknown".
    let f4 = |x: Option<f64>| match x {
        Some(v) => format!("{v:+.4}"),
        None => "—".to_string(),
    };
    let rank = |x: Option<i64>| match x {
        Some(0) => "0".to_string(),
        Some(v) => format!("{v:+}"),
        None => "—".to_string(),
    };
    let presence = |c: &threshold_core::compare::CandidateDelta| match (c.in_a, c.in_b) {
        (true, true) => "",
        (true, false) => " (only A)",
        (false, true) => " (only B)",
        (false, false) => "",
    };

    println!("Compare A → B:");
    println!("  A: {}", cmp.label_a);
    println!("  B: {}", cmp.label_b);
    println!();
    println!("| candidate | Δ reward | Δ rank | Δ cost/trial |");
    println!("|---|---|---|---|");
    for c in &cmp.candidates {
        println!(
            "| {}{} | {} | {} | {} |",
            c.candidate_id,
            presence(c),
            f4(c.reward_delta),
            rank(c.rank_delta),
            f4(c.cost_per_trial_delta),
        );
    }
    println!();
    // Intentionally NOT `view::money`: that takes a bare f64 and collapses -0.0
    // → 0.0; this takes Option and must render an absent spend as "unknown"
    // (AGENTS: unknown cost is null, never 0).
    let money = |x: Option<f64>| match x {
        Some(v) => format!("${v:.4}"),
        None => "unknown".to_string(),
    };
    println!(
        "spend:       A {}  →  B {}  (Δ {})",
        money(cmp.spend_a),
        money(cmp.spend_b),
        f4(cmp.spend_delta),
    );
    println!(
        "stop reason: A {}  →  B {}",
        cmp.stop_reason_a.as_deref().unwrap_or("unknown"),
        cmp.stop_reason_b.as_deref().unwrap_or("unknown"),
    );
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// export
// ---------------------------------------------------------------------------

fn cmd_export(delivery: &std::path::Path, spec_path: &std::path::Path) -> ExitCode {
    let spec_text = match std::fs::read_to_string(spec_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let spec_toml: toml::Value = match toml::from_str(&spec_text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse spec: {e}");
            return ExitCode::FAILURE;
        }
    };
    let spec_json = toml_to_json(spec_toml);
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match threshold_core::export::export_delivery(delivery, &spec_json, None, None, &repo) {
        Ok(paths) => {
            // Sort for deterministic output (mirrors Python's dict insertion order: contract, persona, handoff)
            for kind in &["contract", "persona", "handoff"] {
                if let Some(path) = paths.get(*kind) {
                    println!("{kind}: {}", path.display());
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// export-cerberus
// ---------------------------------------------------------------------------

fn cmd_export_cerberus(
    delivery: &std::path::Path,
    spec_path: &std::path::Path,
    out: &std::path::Path,
) -> ExitCode {
    let spec_text = match std::fs::read_to_string(spec_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let spec_toml: toml::Value = match toml::from_str(&spec_text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse spec: {e}");
            return ExitCode::FAILURE;
        }
    };
    let spec_json = toml_to_json(spec_toml);
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match threshold_core::cerberus::export_reviewer_config_packet(
        delivery, &spec_json, out, None, &repo,
    ) {
        Ok(path) => {
            println!("packet: {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("export-cerberus failed: {e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// cerberus-lab import
// ---------------------------------------------------------------------------

fn cmd_cerberus_lab_import(options: threshold_core::cerberus_lab::ImportOptions) -> ExitCode {
    match threshold_core::cerberus_lab::import_review_artifact(&options) {
        Ok(result) => {
            println!("out-dir: {}", result.out_dir.display());
            println!("summary: {}", result.summary.display());
            println!("score: {}", result.score.display());
            println!("findings: {}", result.findings.display());
            println!("report: {}", result.report.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("cerberus-lab import failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_cerberus_lab_compare(options: threshold_core::cerberus_lab::CompareOptions) -> ExitCode {
    match threshold_core::cerberus_lab::compare_imports(&options) {
        Ok(result) => {
            println!("out-dir: {}", result.out_dir.display());
            println!("summary: {}", result.summary.display());
            println!("report: {}", result.report.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("cerberus-lab compare failed: {err}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// export-suite
// ---------------------------------------------------------------------------

fn cmd_export_suite(delivery: &std::path::Path, suite_path: &std::path::Path) -> ExitCode {
    let suite_text = match std::fs::read_to_string(suite_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let suite_toml: toml::Value = match toml::from_str(&suite_text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse suite: {e}");
            return ExitCode::FAILURE;
        }
    };
    let suite_json = toml_to_json(suite_toml);
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match threshold_core::swarm::export_suite(delivery, &suite_json, None, &repo) {
        Ok(result) => {
            println!("contract: {}", result.contract.display());
            println!("handoff: {}", result.handoff.display());
            println!("summary: {}", result.summary.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("export-suite failed: {e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// launch-pack
// ---------------------------------------------------------------------------

fn cmd_launch_pack(
    delivery: &std::path::Path,
    plane: &str,
    dry_run: bool,
    out_dir: Option<&std::path::Path>,
) -> ExitCode {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match threshold_core::launch::write_import_packet(
        delivery, plane, dry_run, None, out_dir, &repo,
    ) {
        Ok(out) => {
            println!("import_packet: {}", out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            // Check if it's an UnsignedLaunchError
            if e.downcast_ref::<threshold_core::launch::UnsignedLaunchError>()
                .is_some()
            {
                eprintln!("launch refused: {e} (use --dry-run for review artifacts)");
            } else {
                eprintln!("{e}");
            }
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// regression
// ---------------------------------------------------------------------------

fn cmd_regression(
    delivery: &std::path::Path,
    spec_path: &std::path::Path,
    trials: u32,
    exp_dir: Option<&std::path::Path>,
    dry_run: bool,
) -> ExitCode {
    let spec_text = match std::fs::read_to_string(spec_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let spec_toml: toml::Value = match toml::from_str(&spec_text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse spec: {e}");
            return ExitCode::FAILURE;
        }
    };
    let spec_json = toml_to_json(spec_toml);
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let fixtures_rel = spec_json
        .get("inputs")
        .and_then(|i| i.get("fixtures"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let arena_dir = repo.join(fixtures_rel);

    let stamp = threshold_core::run::utc_stamp();
    let spec_id = spec_json
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let computed_exp_dir = exp_dir.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        repo.join("runs")
            .join(format!("{stamp}-regression-{spec_id}"))
    });

    if dry_run {
        if let Err(e) = std::fs::create_dir_all(&computed_exp_dir) {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
        let plan_path = computed_exp_dir.join("regression-command.txt");
        let cmd_str = format!(
            "cargo run --quiet --bin threshold -- regression {} --spec {} --trials {} --exp-dir {}\n",
            delivery.display(),
            spec_path.display(),
            trials,
            computed_exp_dir.display()
        );
        if let Err(e) = std::fs::write(&plan_path, &cmd_str) {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
        println!("regression_command: {}", plan_path.display());
        return ExitCode::SUCCESS;
    }

    // Live run: use run_arena
    let inputs = ArenaInputs {
        candidate_path: delivery.join("agent.toml"),
        arena_dir,
        task_filter: None,
        trials,
        exp_dir: Some(computed_exp_dir.clone()),
        split: "holdout".to_string(),
        is_final: true,
        max_errors: None,
        repo_root: repo.clone(),
        runs_root: repo.join("runs"),
    };
    match threshold_core::run::run_arena(inputs) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("regression failed for {}: {e}", delivery.display());
            return ExitCode::FAILURE;
        }
    }
    match threshold_core::trace::write_trace(&computed_exp_dir) {
        Ok(trace_out) => {
            println!("regression: {}", computed_exp_dir.display());
            println!("trace: {}", trace_out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// arena-scaffold
// ---------------------------------------------------------------------------

fn cmd_arena_scaffold(arena: &std::path::Path, task_id: &str, taskspec: &str) -> ExitCode {
    match threshold_core::workbench::scaffold_task(arena, task_id, taskspec) {
        Ok(task) => {
            println!("task: {}", task.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// arena-validate
// ---------------------------------------------------------------------------

fn cmd_arena_validate(
    arena: &std::path::Path,
    probe_run: Option<&std::path::Path>,
    holdout_burn: i64,
    report_path: Option<&std::path::Path>,
) -> ExitCode {
    match threshold_core::workbench::validate_arena(arena, probe_run, holdout_burn) {
        Ok(result) => {
            let text = threshold_core::workbench::render_validation_report(&result);
            if let Some(rp) = report_path {
                if let Err(e) = std::fs::write(rp, &text) {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
                println!("freeze report: {}", rp.display());
            } else {
                print!("{text}");
            }
            if !result.ok {
                eprintln!("arena validation failed");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// arena-freeze
// ---------------------------------------------------------------------------

fn cmd_arena_freeze(
    arena: &Path,
    out_dir: Option<&Path>,
    report_path: Option<&Path>,
    holdout_burn: i64,
    probe_model: Option<&str>,
    probe_context_window: Option<u64>,
) -> ExitCode {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let arena_cfg = match threshold_core::run::load_toml(&arena.join("arena.toml")) {
        Ok(Value::Object(m)) => m,
        Ok(_) => {
            eprintln!("arena.toml must be a TOML table");
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("load arena.toml: {e}");
            return ExitCode::FAILURE;
        }
    };
    let arena_id = arena_cfg
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("arena");
    let computed_out = out_dir.map(Path::to_path_buf).unwrap_or_else(|| {
        repo.join("runs").join(format!(
            "{}-freeze-{}",
            threshold_core::run::utc_stamp(),
            sanitize_path_segment(arena_id)
        ))
    });
    if let Err(e) = std::fs::create_dir_all(&computed_out) {
        eprintln!("create out dir: {e}");
        return ExitCode::FAILURE;
    }

    let probe_manifest =
        match freeze_probe_manifest(&repo, &computed_out, probe_model, probe_context_window) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };

    for manifest in [
        repo.join("candidates/oracle.toml"),
        repo.join("candidates/null.toml"),
        probe_manifest,
    ] {
        let inputs = ArenaInputs {
            candidate_path: manifest,
            arena_dir: arena.to_path_buf(),
            task_filter: None,
            trials: 1,
            exp_dir: Some(computed_out.clone()),
            split: "all".to_string(),
            is_final: true,
            max_errors: None,
            repo_root: repo.clone(),
            runs_root: repo.join("runs"),
        };
        if let Err(e) = threshold_core::run::run_arena(inputs) {
            eprintln!("freeze runner failed: {e}");
            return ExitCode::FAILURE;
        }
    }

    if let Err(e) = summarize(&computed_out.join("trials.jsonl")) {
        eprintln!("summarize freeze run: {e}");
        return ExitCode::FAILURE;
    }

    let result =
        match threshold_core::workbench::validate_arena(arena, Some(&computed_out), holdout_burn) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };
    let text = threshold_core::workbench::render_validation_report(&result);
    let computed_report = report_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| computed_out.join("freeze-report.md"));
    if let Err(e) = std::fs::write(&computed_report, &text) {
        eprintln!("write freeze report: {e}");
        return ExitCode::FAILURE;
    }

    println!("freeze run: {}", computed_out.display());
    println!("freeze report: {}", computed_report.display());
    if result.ok {
        ExitCode::SUCCESS
    } else {
        eprintln!("arena validation failed");
        ExitCode::FAILURE
    }
}

fn freeze_probe_manifest(
    repo: &Path,
    out_dir: &Path,
    probe_model: Option<&str>,
    probe_context_window: Option<u64>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source = repo.join("candidates/probe-oneshot.toml");
    if !source.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("missing one-shot probe manifest: {}", source.display()),
        )
        .into());
    }
    if probe_model.is_none() && probe_context_window.is_none() {
        return Ok(source);
    }
    let text = std::fs::read_to_string(&source)?;
    let mut manifest: toml::Value = toml::from_str(&text)?;
    let table = manifest
        .as_table_mut()
        .ok_or("probe manifest must be a TOML table")?;
    if let Some(model) = probe_model {
        table.insert("model".to_string(), toml::Value::String(model.to_string()));
    }
    if let Some(window) = probe_context_window {
        let window =
            i64::try_from(window).map_err(|_| "probe context window exceeds TOML integer range")?;
        table.insert("context_window".to_string(), toml::Value::Integer(window));
    }
    let manifest_dir = out_dir.join("manifests");
    std::fs::create_dir_all(&manifest_dir)?;
    let path = manifest_dir.join("probe-oneshot.toml");
    std::fs::write(&path, toml::to_string_pretty(&manifest)?)?;
    Ok(path)
}

fn sanitize_path_segment(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// arena-redteam (040): flag gameable answer keys
// ---------------------------------------------------------------------------

fn cmd_arena_redteam(arena: &std::path::Path, wide_threshold: i64) -> ExitCode {
    let tasks_dir = arena.join("tasks");
    let entries = match std::fs::read_dir(&tasks_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("read {}: {e}", tasks_dir.display());
            return ExitCode::FAILURE;
        }
    };
    let mut task_dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("tests").join("expected.json").is_file())
        .collect();
    task_dirs.sort();

    println!(
        "Red-team audit of {} (spans > {wide_threshold} lines are flagged gameable):\n",
        arena.display()
    );
    println!("| task | defects | max span | mean span | gaming reward | wide |");
    println!("|---|---|---|---|---|---|");
    let mut total_wide = 0usize;
    let mut arena_max_span = 0i64;
    let mut any_gaming = false;
    for td in &task_dirs {
        let tid = td.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let expected = td.join("tests").join("expected.json");
        match threshold_core::score::redteam_audit(&expected, wide_threshold) {
            Ok(a) => {
                total_wide += a.wide_defects.len();
                arena_max_span = arena_max_span.max(a.max_span);
                if a.n_defects > 0 && a.gaming_reward >= 1.0 {
                    any_gaming = true;
                }
                let wide = if a.wide_defects.is_empty() {
                    "—".to_string()
                } else {
                    a.wide_defects
                        .iter()
                        .map(|(id, s)| format!("{id}:{s}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                // A clean (0-defect) key has nothing to game; show n/a, not 1.0.
                let gaming = if a.n_defects == 0 {
                    "n/a".to_string()
                } else {
                    format!("{:.4}", a.gaming_reward)
                };
                println!(
                    "| {tid} | {} | {} | {:.1} | {gaming} | {wide} |",
                    a.n_defects, a.max_span, a.mean_span
                );
            }
            Err(e) => println!("| {tid} | key error | — | — | — | {e} |"),
        }
    }
    println!(
        "\nSummary: {} task(s), {total_wide} wide-span defect(s), max span {arena_max_span} lines.",
        task_dirs.len()
    );
    // The lever is span width, not gaming_reward — which is 1.0 by construction
    // (a structure-aware adversary that knows file+category always scores). The
    // actionable risk is *wide* spans, where "any in-span line" demands no real
    // localization.
    if total_wide > 0 {
        println!(
            "⚠ {total_wide} wide-span defect(s) (up to {arena_max_span} lines): a candidate can \
             score by emitting the right file+category at any in-span line, without locating the \
             defect. Tighten these keys (re-baseline) or add description matching before trusting \
             close rankings."
        );
    } else if any_gaming {
        println!(
            "✓ No wide spans (max {arena_max_span} lines): the line constraint demands real \
             localization. (gaming reward is 1.0 by construction — the lever is span width.)"
        );
    }
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// arena-adjudicate
// ---------------------------------------------------------------------------

fn cmd_arena_adjudicate(
    arena: &std::path::Path,
    task: &str,
    finding: &str,
    ruling: &str,
    rationale: &str,
    new_version: Option<&str>,
    baseline_run: Option<&std::path::Path>,
) -> ExitCode {
    match threshold_core::workbench::record_adjudication(
        arena,
        task,
        finding,
        ruling,
        rationale,
        new_version,
        baseline_run,
    ) {
        Ok(path) => {
            println!("adjudications: {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// arena-disagreements
// ---------------------------------------------------------------------------

fn cmd_arena_disagreements(findings: &std::path::Path, expected: &std::path::Path) -> ExitCode {
    match threshold_core::workbench::disagreements(findings, expected) {
        Ok(rows) => {
            let v: Value = Value::Array(rows);
            match serde_json::to_string_pretty(&v) {
                Ok(s) => {
                    println!("{s}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// taxonomy-validate
// ---------------------------------------------------------------------------

fn cmd_taxonomy_validate(taxonomy: &std::path::Path, suite: &std::path::Path) -> ExitCode {
    let report = threshold_core::taxonomy::validate_taxonomy(taxonomy, suite);
    print!("{}", threshold_core::taxonomy::render_report(&report));
    if !report.ok {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

// ---------------------------------------------------------------------------
// doctor
// ---------------------------------------------------------------------------

fn cmd_doctor(stale_days: i64, today: Option<&str>) -> ExitCode {
    let today_tuple: Option<(i64, u32, u32)> = if let Some(s) = today {
        match parse_date(s) {
            Some(t) => Some(t),
            None => {
                eprintln!("invalid --today format (expected YYYY-MM-DD): {s}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let checks = threshold_core::doctor::run_checks(&repo, today_tuple, stale_days, true);
    print!("{}", threshold_core::doctor::render(&checks));
    if threshold_core::doctor::has_failures(&checks) {
        eprintln!("doctor found blocking readiness issues");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn parse_date(s: &str) -> Option<(i64, u32, u32)> {
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    Some((y, m, d))
}

// ---------------------------------------------------------------------------
// port-harbor
// ---------------------------------------------------------------------------

fn cmd_port_harbor(
    arena_path: &std::path::Path,
    out: &str,
    scorer_bin: &std::path::Path,
) -> ExitCode {
    use threshold_core::port_harbor::port_task;

    // Locate repo root as the cwd (harbor-run cds into repo root before exec).
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let arena_dir = if arena_path.is_absolute() {
        arena_path.to_path_buf()
    } else {
        repo.join(arena_path)
    };

    let scorer_bin_abs = if scorer_bin.is_absolute() {
        scorer_bin.to_path_buf()
    } else {
        repo.join(scorer_bin)
    };

    // Read arena.toml
    let arena_toml_text = match std::fs::read_to_string(arena_dir.join("arena.toml")) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("read arena.toml: {e}");
            return ExitCode::FAILURE;
        }
    };
    let arena: toml::Value = match arena_toml_text.parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse arena.toml: {e}");
            return ExitCode::FAILURE;
        }
    };

    let arena_id = arena
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let build_root = repo.join(out).join(arena_id);

    // Iterate tasks (sorted)
    let tasks_dir = arena_dir.join("tasks");
    let mut task_dirs: Vec<std::path::PathBuf> = match std::fs::read_dir(&tasks_dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(e) => {
            eprintln!("read tasks dir: {e}");
            return ExitCode::FAILURE;
        }
    };
    task_dirs.sort();

    if task_dirs.is_empty() {
        eprintln!("no tasks found");
        return ExitCode::FAILURE;
    }

    for task_dir in &task_dirs {
        let task_name = task_dir.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let out_dir = build_root.join(task_name);
        if let Err(e) = port_task(&arena_dir, &arena, task_dir, &out_dir, &scorer_bin_abs) {
            eprintln!("port_task {task_name}: {e}");
            return ExitCode::FAILURE;
        }
        println!("ported {task_name}");
    }
    println!("harbor dataset: {}", build_root.display());
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// run (the search)
// ---------------------------------------------------------------------------

/// Materialize the incumbent reference manifest (055) from the taskspec
/// `[incumbent]` table. The incumbent is the config we would otherwise deploy;
/// it executes as a `pi` agent but is tagged `kind = "incumbent"` so it is
/// excluded from the Pareto front, recommendation, and mutation, and becomes the
/// certification baseline in place of the null floor. `prompt_packet` resolves to
/// an absolute path under `repo` (falling back to the taskspec `base_packet`).
fn write_incumbent_manifest(
    inc: &Map<String, Value>,
    base_packet: Option<&str>,
    timeout_sec: i64,
    repo: &std::path::Path,
    out_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let model = inc
        .get("model")
        .and_then(Value::as_str)
        .ok_or("[incumbent] requires a `model`")?;
    let packet_rel = inc
        .get("prompt_packet")
        .and_then(Value::as_str)
        .or(base_packet)
        .ok_or("[incumbent] needs a `prompt_packet` (or the taskspec a `search.base_packet`)")?;
    let packet_abs = repo.join(packet_rel);
    if !packet_abs.exists() {
        return Err(format!(
            "[incumbent] prompt_packet not found: {}",
            packet_abs.display()
        ));
    }

    let mut m: Map<String, Value> = Map::new();
    m.insert("composition".into(), Value::Number(1.into()));
    m.insert("id".into(), Value::String("incumbent".into()));
    m.insert("kind".into(), Value::String("incumbent".into()));
    m.insert("provider_name".into(), Value::String("openrouter".into()));
    m.insert("model".into(), Value::String(model.to_string()));
    m.insert(
        "prompt_packet".into(),
        Value::String(packet_abs.to_string_lossy().into_owned()),
    );
    m.insert(
        "thinking".into(),
        Value::String(
            inc.get("thinking")
                .and_then(Value::as_str)
                .unwrap_or("medium")
                .to_string(),
        ),
    );
    let tools: Vec<Value> = inc
        .get("tools")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|t| t.as_str().map(|s| Value::String(s.to_string())))
                .collect()
        })
        .unwrap_or_else(|| {
            ["read", "bash", "edit", "write"]
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect()
        });
    m.insert("tools".into(), Value::Array(tools));
    m.insert("timeout_sec".into(), Value::Number(timeout_sec.into()));
    // Optional passthroughs, matching the seed manifest shape.
    if let Some(spm) = inc.get("system_prompt_mode").and_then(Value::as_str) {
        if spm != "append" {
            m.insert("system_prompt_mode".into(), Value::String(spm.to_string()));
        }
    }
    if let Some(skills) = inc.get("skills").and_then(Value::as_array) {
        m.insert("skills".into(), Value::Array(skills.clone()));
    }
    if let Some(a) = inc.get("agents_md").and_then(Value::as_str) {
        m.insert("agents_md".into(), Value::String(a.to_string()));
    }

    threshold_core::mutate::write_manifest(&m, &out_dir.join("incumbent.toml"))
        .map_err(|e| format!("write incumbent manifest: {e}"))
}

#[allow(clippy::too_many_arguments)]
fn cmd_run(
    taskspec_path: &std::path::Path,
    arena_override: Option<&std::path::Path>,
    budget_usd: f64,
    max_candidates: usize,
    trials: u32,
    optimizer_model: &str,
    plateau: usize,
    allow_saturated: bool,
    rng_seed: Option<u64>,
    children_per_gen: usize,
    certify_top: usize,
    certify_trials: u32,
    min_effect: f64,
    consistency_floor: f64,
    reliability_floor: f64,
    max_errors_per_candidate: Option<usize>,
    estimate: bool,
) -> ExitCode {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // A minimum detectable effect is a non-negative reward delta; the is_nan
    // arm also rejects garbage. A negative MDE would certify candidates provably
    // *worse* than baseline — the opposite of what certification means.
    if min_effect < 0.0 || min_effect.is_nan() {
        eprintln!(
            "error: --min-effect must be >= 0 (got {min_effect}); it is the minimum reward \
             delta a candidate must provably beat the selected baseline by to certify."
        );
        return ExitCode::FAILURE;
    }

    // The reliability floor is a pass^k probability; outside [0, 1] it is
    // meaningless. 0.0 leaves the 056 gate inert (pre-056 behaviour).
    if !(0.0..=1.0).contains(&reliability_floor) || reliability_floor.is_nan() {
        eprintln!(
            "error: --reliability-floor must be in [0, 1] (got {reliability_floor}); it is the \
             minimum pass^k a candidate must reach to be recommendable. 0 = gate off."
        );
        return ExitCode::FAILURE;
    }

    let spec_text = match std::fs::read_to_string(taskspec_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let spec_toml: toml::Value = match toml::from_str(&spec_text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse taskspec: {e}");
            return ExitCode::FAILURE;
        }
    };
    let spec: Map<String, Value> = match toml_to_json(spec_toml) {
        Value::Object(m) => m,
        _ => {
            eprintln!("taskspec must be a TOML table");
            return ExitCode::FAILURE;
        }
    };

    let mode = spec
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("max-quality")
        .to_string();

    let fixtures_rel = spec
        .get("inputs")
        .and_then(|i| i.get("fixtures"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let arena_dir = arena_override
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| repo.join(fixtures_rel));

    let arena_cfg_val: Value = match threshold_core::run::load_toml(&arena_dir.join("arena.toml")) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("load arena.toml: {e}");
            return ExitCode::FAILURE;
        }
    };
    let arena_cfg = arena_cfg_val.as_object().cloned().unwrap_or_default();

    let split_cfg = arena_cfg
        .get("split")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let holdout_ids: Vec<String> = split_cfg
        .get("holdout")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let search_tasks: Vec<String> = {
        let train: Vec<String> = split_cfg
            .get("train")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let val: Vec<String> = split_cfg
            .get("validation")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        train.into_iter().chain(val).collect()
    };

    let stamp = threshold_core::run::utc_stamp();
    let spec_id = spec.get("id").and_then(Value::as_str).unwrap_or("unknown");

    // 041: `--estimate` is an offline forecast — project the trial count and
    // (when the taskspec declares a per-trial ceiling) the worst-case cost, then
    // EARLY-RETURN before any `runs/` directory is created and before any stage
    // runs. Zero trials, zero spend.
    if estimate {
        // References that run alongside the search: null floor, oracle ceiling,
        // and the one-shot saturation probe (stages 1, 1b — see cmd_run below).
        const REFERENCE_KINDS: usize = 3;
        let max_cost_per_trial_usd = spec
            .get("budget")
            .and_then(|b| b.get("max_cost_per_trial_usd"))
            .and_then(Value::as_f64);
        let has_incumbent = spec.get("incumbent").and_then(Value::as_object).is_some();
        let inputs = threshold_core::forecast::ForecastInputs {
            max_candidates,
            reference_kinds: REFERENCE_KINDS,
            n_search_tasks: search_tasks.len(),
            trials,
            certify_top,
            n_holdout: holdout_ids.len(),
            certify_trials,
            has_incumbent,
            max_cost_per_trial_usd,
        };
        let f = threshold_core::forecast::forecast(&inputs);
        let n_tasks = search_tasks.len();
        let n_holdout = holdout_ids.len();
        println!("Forecast for {spec_id} (offline — NO trials run, NO spend):");
        // Candidates run `--trials`-deep; references (null/oracle/one-shot probe)
        // run once per task — so they are shown at ×1, not ×trials.
        println!(
            "  candidates:    {max_candidates} × {n_tasks} search tasks × {trials} trials \
             = {} trials",
            f.candidate_trials,
        );
        println!(
            "  references:    {REFERENCE_KINDS} × {n_tasks} search tasks × 1 trial (single-shot) \
             = {} trials",
            f.reference_trials,
        );
        println!(
            "  certification: {certify_top} top × {n_holdout} holdout × {certify_trials} trials \
             = {} trials",
            f.certify_trials_total,
        );
        if has_incumbent {
            println!(
                "  incumbent:     1 × {} all tasks × {certify_trials} trials = {} trials (055 baseline)",
                n_tasks + n_holdout,
                f.incumbent_trials,
            );
        }
        // "up to" — the plateau/budget stops can end the search early, so this is
        // an upper bound, not a definite count.
        println!("  total:         up to {} trials", f.total_trials);
        match f.max_cost_usd {
            Some(c) => println!(
                "  max cost:      up to ${c:.4} (worst case = {} trials × ${:.4}/trial ceiling)",
                f.total_trials,
                max_cost_per_trial_usd.unwrap_or(0.0),
            ),
            None => println!(
                "  max cost:      unknown (taskspec declares no [budget].max_cost_per_trial_usd)"
            ),
        }
        return ExitCode::SUCCESS;
    }

    let exp_dir = repo.join("runs").join(format!("{stamp}-search-{spec_id}"));
    if let Err(e) = std::fs::create_dir_all(&exp_dir) {
        eprintln!("create exp_dir: {e}");
        return ExitCode::FAILURE;
    }

    // Track manifests: candidate_id → manifest_path
    let mut manifests: HashMap<String, PathBuf> = HashMap::new();
    let mut optimizer_costs: Vec<Option<f64>> = Vec::new();

    // ── Helper: run a candidate split via run_arena ─────────────────────────
    let run_candidate_split = |candidate_path: &PathBuf,
                               split: &str,
                               n_trials: u32,
                               is_final: bool,
                               max_errors: Option<usize>| {
        let cid = {
            let text = std::fs::read_to_string(candidate_path).unwrap_or_default();
            let t: toml::Value =
                toml::from_str(&text).unwrap_or(toml::Value::Table(Default::default()));
            toml_to_json(t)
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string()
        };
        if let Some(limit) = max_errors_per_candidate {
            // count errors
            let trials_path = exp_dir.join("trials.jsonl");
            let errors = if trials_path.exists() {
                std::fs::read_to_string(&trials_path)
                    .unwrap_or_default()
                    .lines()
                    .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                    .filter(|r| {
                        r.get("candidate_id").and_then(Value::as_str) == Some(&cid)
                            && r.get("error").map(|v| !v.is_null()).unwrap_or(false)
                    })
                    .count()
            } else {
                0
            };
            if errors >= limit {
                println!("  skip {cid} split={split}: max error limit {limit} already reached",);
                // Record cutoff
                let entry = serde_json::json!({
                    "candidate_id": cid,
                    "split": split,
                    "errors": errors,
                    "max_errors_per_candidate": limit,
                    "reason": "max-errors-per-candidate",
                });
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(exp_dir.join("candidate-cutoffs.jsonl"))
                {
                    let _ = writeln!(f, "{}", entry);
                }
                return;
            }
        }
        println!(
            "\n=== run {} split={split} trials={n_trials}",
            candidate_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
        );
        let inputs = ArenaInputs {
            candidate_path: candidate_path.clone(),
            arena_dir: arena_dir.clone(),
            task_filter: None,
            trials: n_trials,
            exp_dir: Some(exp_dir.clone()),
            split: split.to_string(),
            is_final,
            max_errors,
            repo_root: repo.clone(),
            runs_root: repo.join("runs"),
        };
        if let Err(e) = threshold_core::run::run_arena(inputs) {
            eprintln!(
                "runner failed for {} (split={split}): {e}",
                candidate_path.display()
            );
        }
    };

    // ── Stage 1: rig validation ─────────────────────────────────────────────
    println!("== stage 1: rig validation (oracle ceiling, null floor)");
    for ref_name in &["candidates/oracle.toml", "candidates/null.toml"] {
        let manifest = repo.join(ref_name);
        run_candidate_split(&manifest, "all", 1, true, None);
    }

    let rig = match summarize(&exp_dir.join("trials.jsonl")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("summarize after references: {e}");
            return ExitCode::FAILURE;
        }
    };
    let oracle_mean = rig
        .get("oracle")
        .and_then(|s| s.get("reward_mean"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let null_mean = rig
        .get("null")
        .and_then(|s| s.get("reward_mean"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    if oracle_mean != 1.0 {
        eprintln!("rig broken: oracle scored {oracle_mean}, not 1.0");
        return ExitCode::FAILURE;
    }
    if null_mean >= oracle_mean {
        eprintln!("rig broken: null scored {null_mean} == oracle (no discrimination)");
        return ExitCode::FAILURE;
    }

    // ── Stage 1b: saturation probe ─────────────────────────────────────────
    println!("== stage 1b: saturation probe (one-shot reference)");
    run_candidate_split(
        &repo.join("candidates/probe-oneshot.toml"),
        "all",
        1,
        true,
        None,
    );

    let rig2 = match summarize(&exp_dir.join("trials.jsonl")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("summarize after probe: {e}");
            return ExitCode::FAILURE;
        }
    };
    let probe = rig2.get("probe-oneshot");
    let probe_mean = probe
        .and_then(|s| s.get("reward_mean"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let probe_errors = probe
        .and_then(|s| s.get("errors"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let probe_trials = probe
        .and_then(|s| s.get("trials"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    // Backlog 040: an errored probe (e.g. context overflow → reward 0.0) must not
    // pass as "unsaturated" — its low mean is an artifact, not evidence.
    use threshold_core::workbench::ProbeVerdict;
    let verdict = threshold_core::workbench::probe_saturation_verdict(
        probe_mean,
        oracle_mean,
        probe_errors,
        probe_trials,
    );
    let saturated = verdict == ProbeVerdict::Saturated;
    let verdict_str = match verdict {
        ProbeVerdict::Saturated => "saturated",
        ProbeVerdict::Unsaturated => "unsaturated",
        ProbeVerdict::Inconclusive => "inconclusive",
    };

    let rig_json = serde_json::json!({
        "oracle_mean": oracle_mean,
        "null_mean": null_mean,
        "probe_mean": probe_mean,
        "saturated": saturated,
        "probe_verdict": verdict_str,
        "probe_errors": probe_errors,
        "probe_trials": probe_trials,
    });
    let _ = std::fs::write(
        exp_dir.join("rig.json"),
        serde_json::to_string_pretty(&rig_json).unwrap(),
    );

    match verdict {
        ProbeVerdict::Saturated => {
            println!(
                "!! ARENA SATURATED: probe scored {probe_mean} vs oracle {oracle_mean}. \
                 This arena cannot rank agent configurations."
            );
            if !allow_saturated {
                eprintln!("aborting search on a saturated arena (--allow-saturated to override)");
                return ExitCode::FAILURE;
            }
        }
        ProbeVerdict::Inconclusive => {
            println!(
                "!! PROBE INCONCLUSIVE: {probe_errors}/{probe_trials} one-shot probe trials \
                 errored (e.g. context overflow); the {probe_mean} mean is NOT evidence the \
                 arena is unsaturated. Fix the probe before trusting this arena."
            );
            if !allow_saturated {
                eprintln!(
                    "aborting search on an inconclusive saturation probe \
                     (--allow-saturated to override)"
                );
                return ExitCode::FAILURE;
            }
        }
        ProbeVerdict::Unsaturated => {}
    }

    // ── Stage 1c: incumbent baseline (055) ──────────────────────────────────
    // If the taskspec declares an [incumbent], run the deployed config as a
    // kind="incumbent" reference on every task. Certification then differences
    // candidates against it instead of the null floor — "beats what we ship,"
    // not "beats silence." Run after the saturation check so a dead arena never
    // pays for it.
    if let Some(inc) = spec.get("incumbent").and_then(Value::as_object) {
        println!("== stage 1c: incumbent baseline (055)");
        let base_packet = spec
            .get("search")
            .and_then(|s| s.get("base_packet"))
            .and_then(Value::as_str);
        let inc_timeout = spec
            .get("budget")
            .and_then(|b| b.get("max_wall_per_trial_sec"))
            .and_then(Value::as_i64)
            .unwrap_or(600);
        match write_incumbent_manifest(inc, base_packet, inc_timeout, &repo, &exp_dir) {
            Ok(path) => run_candidate_split(&path, "all", certify_trials, true, None),
            Err(e) => {
                eprintln!("incumbent baseline: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    // ── Stage 2: seed population ────────────────────────────────────────────
    println!("== stage 2: seed population (landscape scan)");

    let packets_dir = exp_dir.join("packets");
    let manifests_dir = exp_dir.join("manifests");

    let seed_result = threshold_core::seed::seed_population(
        &spec,
        optimizer_model,
        &packets_dir,
        &manifests_dir,
        rng_seed,
        Some(&repo),
        &mut |prompt: &str, model: &str| -> Result<(String, f64), String> {
            call_optimizer(prompt, model, 120, 3)
                .map(|(content, cost)| (content, cost.unwrap_or(0.0)))
        },
    );

    let (seeds, mut seed_meta) = match seed_result {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("seed_population: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Persist the budget cap so `threshold view` can show live spend against it
    // (the cap is a CLI arg, not otherwise on disk before loop.json).
    seed_meta.insert("budget_usd".to_string(), serde_json::json!(budget_usd));

    // Gather optimizer costs from seed phase
    if let Some(costs) = seed_meta.get("optimizer_costs").and_then(Value::as_array) {
        for c in costs {
            optimizer_costs.push(c.as_f64());
        }
    }

    let _ = std::fs::write(
        exp_dir.join("seed.json"),
        serde_json::to_string_pretty(&Value::Object(seed_meta.clone())).unwrap(),
    );

    let rng_seed_used = seed_meta
        .get("rng_seed")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    println!("  rng_seed={rng_seed_used}  seeds={}", seeds.len());

    let seed_budget = budget_usd * 0.6;
    for (sid, manifest) in &seeds {
        // Budget check
        if let Ok(ref current_rig) = summarize(&exp_dir.join("trials.jsonl")) {
            let spent = known_spend(current_rig, &optimizer_costs);
            if spent >= seed_budget {
                println!(
                    "  seed budget ${seed_budget:.2} reached (${spent:.4} spent); stopping before {sid}"
                );
                break;
            }
        }
        manifests.insert(sid.clone(), manifest.clone());
        run_candidate_split(manifest, "train", trials, false, None);
        run_candidate_split(manifest, "validation", trials, false, None);
    }

    if manifests.is_empty() {
        eprintln!("no seeds ran within the seed budget; raise --budget-usd");
        return ExitCode::FAILURE;
    }

    // ── Stage 3: search loop ────────────────────────────────────────────────
    println!("== stage 3: reflective search");

    let taskspec_for_prompt: Map<String, Value> = {
        let mut m = Map::new();
        if let Some(g) = spec.get("goal") {
            m.insert("goal".to_string(), g.clone());
        }
        if let Some(mo) = spec.get("mode") {
            m.insert("mode".to_string(), mo.clone());
        }
        m
    };
    let search_space = spec
        .get("search")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    // Clone what we need into the SearchWorld impl
    let exp_dir_c = exp_dir.clone();
    let repo_c = repo.clone();
    let arena_dir_c = arena_dir.clone();
    let manifests_ref = &mut manifests;
    let optimizer_costs_ref = &mut optimizer_costs;
    let optimizer_model_s = optimizer_model.to_string();
    let mode_c = mode.clone();

    // We need a mutable struct that implements SearchWorld
    struct World<'a> {
        exp_dir: PathBuf,
        repo: PathBuf,
        arena_dir: PathBuf,
        manifests: &'a mut HashMap<String, PathBuf>,
        optimizer_costs: &'a mut Vec<Option<f64>>,
        optimizer_model: String,
        trials: u32,
        max_errors_per_candidate: Option<usize>,
        taskspec_for_prompt: Map<String, Value>,
        search_space: Map<String, Value>,
        mode: String,
    }

    impl<'a> SearchWorld for World<'a> {
        fn summary(&mut self) -> Map<String, Value> {
            summarize(&self.exp_dir.join("trials.jsonl")).unwrap_or_default()
        }

        fn record_history(&mut self, entry: &Value) {
            // Stream each hypothesis to loop.history.jsonl the instant it lands,
            // so `threshold view` can tail the search live (loop.json only appears
            // at completion). Best-effort: a failed append never derails a run.
            use std::io::Write as _;
            if let Ok(line) = serde_json::to_string(entry) {
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(self.exp_dir.join("loop.history.jsonl"))
                {
                    let _ = writeln!(f, "{line}");
                }
            }
        }

        fn propose(
            &mut self,
            parent: &str,
            generation: u64,
            attempt: usize,
            avoid_slots: &[String],
        ) -> Result<(String, Value), String> {
            let alpha = b"abcdefgh";
            let letter = alpha.get(attempt).copied().unwrap_or(b'z') as char;
            let child_id_long = format!("g{generation}{letter}-{parent}");
            let child_id: String = child_id_long.chars().take(48).collect();

            // Load parent snapshot
            let snap_path = self
                .exp_dir
                .join("compositions")
                .join(format!("{parent}.json"));
            let snap_text = std::fs::read_to_string(&snap_path)
                .map_err(|e| format!("load parent snapshot: {e}"))?;
            let parent_snapshot: Map<String, Value> = serde_json::from_str::<Value>(&snap_text)
                .map_err(|e| format!("parse parent snapshot: {e}"))?
                .as_object()
                .cloned()
                .ok_or("snapshot is not an object")?;

            // Load parent manifest
            let manifest_path = self
                .manifests
                .get(parent)
                .ok_or_else(|| format!("manifest for {parent} not found"))?
                .clone();
            let manifest_text = std::fs::read_to_string(&manifest_path)
                .map_err(|e| format!("load parent manifest: {e}"))?;
            let parent_manifest_toml: toml::Value = toml::from_str(&manifest_text)
                .map_err(|e| format!("parse parent manifest: {e}"))?;
            let parent_manifest: Map<String, Value> = match toml_to_json(parent_manifest_toml) {
                Value::Object(m) => m,
                _ => return Err("parent manifest is not a table".to_string()),
            };

            // Build archive
            let mut archive: HashMap<String, Map<String, Value>> = HashMap::new();
            if let Ok(entries) = std::fs::read_dir(self.exp_dir.join("compositions")) {
                for entry in entries.flatten() {
                    if let Ok(text) = std::fs::read_to_string(entry.path()) {
                        if let Ok(Value::Object(snap)) = serde_json::from_str::<Value>(&text) {
                            let kind = snap
                                .get("kind")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string();
                            if kind == "pi" {
                                let cid = snap
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                                archive.insert(cid, snap);
                            }
                        }
                    }
                }
            }

            // Load records
            let records: Vec<Map<String, Value>> = {
                let path = self.exp_dir.join("trials.jsonl");
                if path.exists() {
                    std::fs::read_to_string(&path)
                        .unwrap_or_default()
                        .lines()
                        .filter_map(|l| {
                            serde_json::from_str::<Value>(l)
                                .ok()
                                .and_then(|v| v.into_object())
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            };

            // Build archive_summary
            let summary = self.summary();
            let archive_summary: Map<String, Value> = summary
                .iter()
                .map(|(cid, stats)| {
                    let mut m = Map::new();
                    if let Some(r) = stats.get("reward_mean") {
                        m.insert("reward_mean".to_string(), r.clone());
                    }
                    if let Some(c) = stats.get("cost_usd_total") {
                        m.insert("cost_usd_total".to_string(), c.clone());
                    }
                    (cid.clone(), Value::Object(m))
                })
                .collect();

            // Extract search space fields
            let tool_policies: Option<HashMap<String, Vec<String>>> =
                self.search_space.get("tool_policies").and_then(|tp| {
                    tp.as_object().map(|m| {
                        m.iter()
                            .filter_map(|(k, v)| {
                                v.as_array().map(|arr| {
                                    (
                                        k.clone(),
                                        arr.iter()
                                            .filter_map(Value::as_str)
                                            .map(String::from)
                                            .collect(),
                                    )
                                })
                            })
                            .collect()
                    })
                });
            let allowed_models: Option<Vec<String>> =
                self.search_space.get("models").and_then(|m| {
                    m.as_array().map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    })
                });
            let allowed_thinking: Option<Vec<String>> =
                self.search_space.get("thinking_levels").and_then(|t| {
                    t.as_array().map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    })
                });
            let skill_sets: Option<HashMap<String, Vec<String>>> =
                self.search_space.get("skill_sets").and_then(|ss| {
                    ss.as_object().map(|m| {
                        m.iter()
                            .filter_map(|(k, v)| {
                                v.as_array().map(|arr| {
                                    (
                                        k.clone(),
                                        arr.iter()
                                            .filter_map(Value::as_str)
                                            .map(String::from)
                                            .collect(),
                                    )
                                })
                            })
                            .collect()
                    })
                });

            let packets_dir = self.exp_dir.join("packets");
            let manifests_dir = self.exp_dir.join("manifests");

            let (manifest_path, meta) = threshold_core::mutate::propose(
                &self.taskspec_for_prompt,
                &parent_snapshot,
                &parent_manifest,
                &records,
                Some(&self.exp_dir),
                &child_id,
                &self.optimizer_model,
                &packets_dir,
                &manifests_dir,
                Some(&Value::Object(archive_summary)),
                tool_policies.as_ref(),
                allowed_models.as_deref(),
                allowed_thinking.as_deref(),
                avoid_slots,
                skill_sets.as_ref(),
                Some(&archive),
                Some(&self.mode),
                |prompt: &str, model: &str| -> Result<(String, Option<f64>), String> {
                    call_optimizer(prompt, model, 120, 3)
                },
            )?;

            self.manifests.insert(child_id.clone(), manifest_path);
            self.optimizer_costs
                .push(meta.get("optimizer_cost_usd").and_then(Value::as_f64));

            let slot = meta
                .get("slot_changed")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let hyp = meta
                .get("hypothesis")
                .and_then(Value::as_str)
                .unwrap_or("?");
            println!("  proposed {child_id} (parent {parent}): slot={slot} — {hyp}");

            Ok((child_id, Value::Object(meta)))
        }

        fn run_child(&mut self, child_id: &str) {
            let manifest = match self.manifests.get(child_id).cloned() {
                Some(p) => p,
                None => {
                    eprintln!("run_child: manifest for {child_id} not found");
                    return;
                }
            };
            for split in &["train", "validation"] {
                let max_err = self.max_errors_per_candidate;
                let cid = child_id.to_string();
                let trials_path = self.exp_dir.join("trials.jsonl");
                if let Some(limit) = max_err {
                    let errors = if trials_path.exists() {
                        std::fs::read_to_string(&trials_path)
                            .unwrap_or_default()
                            .lines()
                            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                            .filter(|r| {
                                r.get("candidate_id").and_then(Value::as_str) == Some(&cid)
                                    && r.get("error").map(|v| !v.is_null()).unwrap_or(false)
                            })
                            .count()
                    } else {
                        0
                    };
                    if errors >= limit {
                        println!(
                            "  skip {cid} split={split}: max error limit {limit} already reached"
                        );
                        continue;
                    }
                }
                println!(
                    "\n=== run {} split={split} trials={}",
                    manifest.file_stem().and_then(|n| n.to_str()).unwrap_or("?"),
                    self.trials
                );
                let inputs = ArenaInputs {
                    candidate_path: manifest.clone(),
                    arena_dir: self.arena_dir.clone(),
                    task_filter: None,
                    trials: self.trials,
                    exp_dir: Some(self.exp_dir.clone()),
                    split: split.to_string(),
                    is_final: false,
                    max_errors: self.max_errors_per_candidate,
                    repo_root: self.repo.clone(),
                    runs_root: self.repo.join("runs"),
                };
                if let Err(e) = threshold_core::run::run_arena(inputs) {
                    eprintln!(
                        "runner failed for {} (split={split}): {e}",
                        manifest.display()
                    );
                }
            }
        }
    }

    let mut world = World {
        exp_dir: exp_dir_c.clone(),
        repo: repo_c.clone(),
        arena_dir: arena_dir_c.clone(),
        manifests: manifests_ref,
        optimizer_costs: optimizer_costs_ref,
        optimizer_model: optimizer_model_s.clone(),
        trials,
        max_errors_per_candidate,
        taskspec_for_prompt,
        search_space,
        mode: mode_c.clone(),
    };

    let params = SearchParams {
        max_children: max_candidates,
        budget_usd: Some(budget_usd),
        optimizer_costs: world.optimizer_costs.clone(),
        plateau_limit: plateau,
        max_proposal_failures: 2,
        children_per_generation: children_per_gen,
        mode: mode_c.clone(),
    };

    let mut rng = PyRandom::new(0);
    let outcome = run_search(&mut world, &params, &mut rng);

    // ── Stage 3.5: certification racing ─────────────────────────────────────
    println!("== stage 3.5: certification racing");

    let summary_now = summarize(&exp_dir.join("trials.jsonl")).unwrap_or_default();
    let real: Vec<String> = summary_now
        .iter()
        .filter(|(_, v)| !is_reference(v.get("id").and_then(Value::as_str).unwrap_or("?"), v))
        .map(|(k, _)| k.clone())
        .collect();

    let mut topk: Vec<String> = real
        .iter()
        .filter(|cid| manifests.contains_key(*cid))
        .cloned()
        .collect();

    // Sort by (reward_mean desc, cost_per_trial asc)
    topk.sort_by(|a, b| {
        let ra = summary_now
            .get(a)
            .and_then(|s| s.get("reward_mean"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let rb = summary_now
            .get(b)
            .and_then(|s| s.get("reward_mean"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });
    topk.truncate(certify_top);

    for cid in &topk {
        let current_summary = summarize(&exp_dir.join("trials.jsonl")).unwrap_or_default();
        let spent = known_spend(&current_summary, &optimizer_costs);
        if spent >= budget_usd {
            println!("  budget reached (${spent:.4}); certification stops before {cid}");
            break;
        }
        let manifest = match manifests.get(cid).cloned() {
            Some(p) => p,
            None => continue,
        };
        let records: Vec<Value> = {
            let path = exp_dir.join("trials.jsonl");
            if path.exists() {
                std::fs::read_to_string(&path)
                    .unwrap_or_default()
                    .lines()
                    .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                    .collect()
            } else {
                Vec::new()
            }
        };
        for split_name in ["train", "validation"] {
            let task_ids: Vec<String> = split_cfg
                .get(split_name)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            if task_ids.is_empty() {
                continue;
            }
            let have = task_ids
                .iter()
                .map(|tid| {
                    records
                        .iter()
                        .filter(|r| {
                            r.get("candidate_id").and_then(Value::as_str) == Some(cid)
                                && r.get("task_id").and_then(Value::as_str) == Some(tid.as_str())
                        })
                        .count()
                })
                .min()
                .unwrap_or(0);
            let need = (certify_trials as usize).saturating_sub(have) as u32;
            if need > 0 {
                let inputs = ArenaInputs {
                    candidate_path: manifest.clone(),
                    arena_dir: arena_dir.clone(),
                    task_filter: None,
                    trials: need,
                    exp_dir: Some(exp_dir.clone()),
                    split: split_name.to_string(),
                    is_final: false,
                    max_errors: None,
                    repo_root: repo.clone(),
                    runs_root: repo.join("runs"),
                };
                let _ = threshold_core::run::run_arena(inputs);
            }
        }
    }

    // ── Stage 4: holdout final ──────────────────────────────────────────────
    let records_now: Vec<Value> = {
        let path = exp_dir.join("trials.jsonl");
        if path.exists() {
            std::fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .collect()
        } else {
            Vec::new()
        }
    };
    let records_map: Vec<Value> = records_now;

    let cands = threshold_core::report::aggregate(&records_map);
    let front = threshold_core::report::pareto_front(&cands);

    if !holdout_ids.is_empty() {
        println!("== stage 4: holdout final evaluation");
        let exposed: Vec<String> = front
            .iter()
            .filter(|cid| manifests.contains_key(*cid))
            .cloned()
            .collect();
        for cid in &exposed {
            let manifest = match manifests.get(cid).cloned() {
                Some(p) => p,
                None => continue,
            };
            let inputs = ArenaInputs {
                candidate_path: manifest,
                arena_dir: arena_dir.clone(),
                task_filter: None,
                trials: certify_trials,
                exp_dir: Some(exp_dir.clone()),
                split: "holdout".to_string(),
                is_final: true,
                max_errors: None,
                repo_root: repo.clone(),
                runs_root: repo.join("runs"),
            };
            let _ = threshold_core::run::run_arena(inputs);
        }
        // Holdout ledger append (mirrors Python)
        let ledger = arena_dir.join("holdout-ledger.md");
        if ledger.exists() && !exposed.is_empty() {
            let ledger_header = std::fs::read_to_string(&ledger)
                .unwrap_or_default()
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            let arena_version = if ledger_header.contains("arena version") {
                arena_cfg.get("version").and_then(Value::as_str)
            } else {
                None
            };
            let exposed_refs: Vec<&str> = exposed.iter().map(|s| s.as_str()).collect();
            let holdout_refs: Vec<&str> = holdout_ids.iter().map(|s| s.as_str()).collect();
            let exp_dir_name = exp_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let row = threshold_core::workbench::format_holdout_ledger_row(
                &stamp,
                exp_dir_name,
                &exposed_refs,
                &holdout_refs,
                certify_trials as usize,
                arena_version,
            );
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&ledger) {
                let _ = writeln!(f, "{row}");
            }
        }
    } else {
        println!("== stage 4: skipped (arena declares no holdout tasks)");
    }

    // ── Stage 5: report ─────────────────────────────────────────────────────
    println!("== stage 5: report");

    let final_records_raw: Vec<Value> = {
        let path = exp_dir.join("trials.jsonl");
        if path.exists() {
            std::fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .collect()
        } else {
            Vec::new()
        }
    };
    let final_records: Vec<Map<String, Value>> = final_records_raw
        .iter()
        .filter_map(|v| v.as_object().cloned())
        .collect();
    let cands2 = threshold_core::report::aggregate(&final_records_raw);
    let front2 = threshold_core::report::pareto_front(&cands2);

    // Trial-count gate: ≥ certify_trials per search task — the mechanical floor.
    let trial_certified: std::collections::HashSet<String> = cands2
        .iter()
        .filter(|(cid, c)| {
            let kind = c.get("kind").and_then(Value::as_str).unwrap_or("");
            // References (null/oracle/oneshot/incumbent) are never certified
            // candidates; the incumbent in particular is the baseline, not a win.
            !threshold_core::report::is_reference_kind(Some(kind))
                && !search_tasks.is_empty()
                && search_tasks.iter().all(|tid| {
                    final_records
                        .iter()
                        .filter(|r| {
                            r.get("candidate_id").and_then(Value::as_str) == Some(cid.as_str())
                                && r.get("task_id").and_then(Value::as_str) == Some(tid.as_str())
                        })
                        .count()
                        >= certify_trials as usize
                })
        })
        .map(|(cid, _)| cid.clone())
        .collect();

    // Backlog 039 child-1 + child-2: a cluster-robust 95% CI on reward delta,
    // and certification gated on it. A candidate certifies only if it clears the
    // trial count AND its CI lower bound exceeds the minimum detectable effect —
    // the foundry can *prove* it beats the selected baseline, not merely rank it.
    //
    // Backlog 040: cluster tasks by their declared `source_repo` (tasks from the
    // same upstream repo share variance, so the clustered SE must pool them);
    // unlabeled tasks fall back to per-task clustering. With labels, n_clusters
    // collapses to the repo count and the CI widens to the honest width.
    let tasks_dir = arena_dir.join("tasks");
    let repo_of: std::collections::HashMap<String, String> = search_tasks
        .iter()
        .chain(holdout_ids.iter())
        .filter_map(|tid| {
            threshold_core::run::source_repo(&tasks_dir.join(tid)).map(|r| (tid.clone(), r))
        })
        .collect();
    let cluster_of = |t: &str| repo_of.get(t).cloned().unwrap_or_else(|| t.to_string());
    // Backlog 055: certify against the incumbent (the deployed config) when one
    // was run, else the null floor. "Provably beats what we ship" > "beats
    // silence." The CI table and certified note name whichever baseline is used.
    let baseline_kind = threshold_core::stats::certification_baseline_kind(&cands2);
    // 055: a declared incumbent that errored every trial (provider down, all
    // context overflow) yields no usable rewards; certification then differences
    // against an empty baseline and bounds nothing. It fails safe — never a false
    // certification — but the operator should know rather than read a silent
    // "nothing certified" as a verdict about the candidates.
    if baseline_kind == "incumbent" {
        let incumbent_has_rewards = cands2
            .values()
            .find(|c| c.get("kind").and_then(Value::as_str) == Some("incumbent"))
            .and_then(|c| c.get("tasks").and_then(Value::as_object))
            .is_some_and(|tasks| {
                tasks
                    .values()
                    .any(|v| v.as_array().is_some_and(|a| !a.is_empty()))
            });
        if !incumbent_has_rewards {
            eprintln!(
                "warning: the incumbent baseline produced no usable rewards (every trial may \
                 have errored); certification differences against it but may bound nothing. \
                 Inspect the incumbent's trials before trusting an empty certified set."
            );
        }
    }
    let baseline_label = if baseline_kind == "incumbent" {
        "the incumbent"
    } else {
        "the null floor"
    };
    let (certified_vec, underpowered) = threshold_core::stats::partition_certified(
        &cands2,
        &trial_certified,
        baseline_kind,
        &cluster_of,
        min_effect,
    );
    let certified: std::collections::HashSet<String> = certified_vec.into_iter().collect();

    // Backlog 056: the reliability gate. A certified candidate is recommendable
    // only if its pass^k clears --reliability-floor; a high mean over a config
    // that fails most of its runs is not deployable (τ-bench). With the default
    // floor (0.0) the gate is inert and `recommendable == certified`.
    let (reliable_vec, unreliable_vec) = threshold_core::stats::partition_reliable(
        &cands2,
        &certified,
        consistency_floor,
        certify_trials as usize,
        reliability_floor,
    );
    let recommendable: std::collections::HashSet<String> = reliable_vec.into_iter().collect();
    let demoted_unreliable: Vec<String> = unreliable_vec;

    let pick = if !recommendable.is_empty() {
        threshold_core::report::recommend(&cands2, &front2, Some(&recommendable))
    } else {
        None
    };

    // CI table covers every trial-complete candidate; the sig column (CI
    // excludes the MDE) is what distinguishes certified from underpowered.
    let (baseline_id, delta_cis) = threshold_core::stats::certified_delta_cis(
        &cands2,
        &trial_certified,
        baseline_kind,
        &cluster_of,
    );
    let baseline_id_str = baseline_id.clone().unwrap_or_else(|| "null".to_string());
    let mut ci_values: Map<String, Value> = Map::new();
    for (cid, ci) in &delta_cis {
        ci_values.insert(cid.clone(), ci.to_value(&baseline_id_str));
    }

    // Backlog 039 child-3: per-candidate reliability — pass rate at the
    // consistency floor and pass^certify_trials — for every trial-complete
    // candidate (independent of whether its CI is defined), reported separately
    // from mean reward.
    let mut consistency_ids: Vec<String> = trial_certified.iter().cloned().collect();
    consistency_ids.sort();
    let consistency_rows: Vec<(String, threshold_core::stats::Consistency)> = consistency_ids
        .iter()
        .filter_map(|cid| {
            cands2.get(cid).map(|c| {
                (
                    cid.clone(),
                    threshold_core::stats::candidate_consistency(c, consistency_floor),
                )
            })
        })
        .collect();
    let pass_k = certify_trials as usize;
    let mut consistency_values: Map<String, Value> = Map::new();
    for (cid, con) in &consistency_rows {
        consistency_values.insert(cid.clone(), con.to_value(pass_k));
    }

    // Meta-eval alarms (post-run)
    let mut alarms: Vec<Value> = outcome
        .get("alarms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let clean_tasks: std::collections::BTreeSet<String> = final_records
        .iter()
        .filter(|r| {
            r.get("expected_defects")
                .and_then(Value::as_i64)
                .unwrap_or(-1)
                == 0
        })
        .filter_map(|r| r.get("task_id").and_then(Value::as_str).map(String::from))
        .collect();

    for task in &clean_tasks {
        // Only "pi" candidates count for the FP-trap alarm — it asks whether the
        // arena discriminates *candidate* false-positive discipline. The incumbent
        // (kind="incumbent") and other references are deliberately excluded.
        let agent_trials: Vec<&Map<String, Value>> = final_records
            .iter()
            .filter(|r| {
                r.get("task_id").and_then(Value::as_str) == Some(task.as_str())
                    && r.get("candidate_kind").and_then(Value::as_str) == Some("pi")
            })
            .collect();
        if !agent_trials.is_empty()
            && agent_trials
                .iter()
                .all(|r| r.get("reward").and_then(Value::as_f64) == Some(1.0))
        {
            alarms.push(serde_json::json!({
                "kind": "fp-trap-never-fired",
                "detail": format!(
                    "every agent passed clean task {task}; the trap may be too easy to discriminate FP discipline"
                ),
            }));
        }
    }

    let final_summary = summarize(&exp_dir.join("trials.jsonl")).unwrap_or_default();
    let total_known_spend =
        (known_spend(&final_summary, &optimizer_costs) * 10000.0).round() / 10000.0;

    // Render report
    let mut report_text = threshold_core::report::render(&cands2, &front2, pick.as_deref());
    // 041: surface WHY the search stopped + the recommendation/certified summary
    // in report.md, not only loop.json/stdout/HTML. `outcome` carries stop_reason
    // and mode; `recommended`/`certified`/`spend_known_usd` are not folded into
    // loop.json until after this point, so we hand verdict_markdown a verdict-
    // shaped Value built from the values already in scope.
    {
        let mut sorted_cert_v: Vec<String> = certified.iter().cloned().collect();
        sorted_cert_v.sort();
        let verdict_view = serde_json::json!({
            "stop_reason": outcome.get("stop_reason").cloned().unwrap_or(Value::Null),
            "mode": outcome.get("mode").cloned().unwrap_or(Value::Null),
            "recommended": pick.as_deref().map(Value::from).unwrap_or(Value::Null),
            "certified": sorted_cert_v,
            "spend_known_usd": total_known_spend,
        });
        report_text.push_str(&threshold_core::report::verdict_markdown(&verdict_view));
    }
    if !certified.is_empty() {
        let mut sorted_cert: Vec<String> = certified.iter().cloned().collect();
        sorted_cert.sort();
        let gate_note = if reliability_floor > 0.0 {
            format!(
                " Recommendation further restricted to candidates whose pass^{certify_trials} clears the reliability floor {reliability_floor:.2} (056)."
            )
        } else {
            " Recommendation restricted to certified candidates.".to_string()
        };
        report_text.push_str(&format!(
            "\n_Certified (≥{certify_trials} trials per search task AND 95% CI lower bound > {min_effect:+.4} vs {baseline_label}): {}.{gate_note}_\n",
            sorted_cert.join(", ")
        ));
    }
    if reliability_floor > 0.0 && !demoted_unreliable.is_empty() {
        let mut demoted = demoted_unreliable.clone();
        demoted.sort();
        report_text.push_str(&format!(
            "\n> **Demoted by the reliability gate (056):** {}. Certified — provably beats {baseline_label} — but pass^{certify_trials} < {reliability_floor:.2}, so not deployable (a high mean over a config that fails most of its runs; τ-bench). Excluded from the recommendation.\n",
            demoted.join(", ")
        ));
    }
    if reliability_floor > 0.0 && recommendable.is_empty() && !certified.is_empty() {
        report_text.push_str(&format!(
            "\n> **No deployable candidate.** Certified candidates exist, but none clear the reliability floor (pass^{certify_trials} ≥ {reliability_floor:.2}). Search more, harden the arena, or lower the floor with eyes open.\n"
        ));
    }
    if !underpowered.is_empty() {
        report_text.push_str(&format!(
            "\n_Trial-complete but NOT certified ({} trials, but the reward-delta CI spans the {min_effect:+.4} minimum effect — no provable win over {baseline_label}): {}. See the CI table; raise --certify-trials or task count, or widen the arena (040)._\n",
            certify_trials,
            underpowered.join(", ")
        ));
    }
    if certified.is_empty() && !trial_certified.is_empty() {
        report_text.push_str(&format!(
            "\n> **No candidate is provably better than {baseline_label}.** Every trial-complete \
             candidate's 95% reward-delta CI spans the minimum detectable effect — the tournament \
             is underpowered, not necessarily the candidates. Add trials/tasks (see the power note) \
             or accept a wider MDE before trusting a ranking.\n",
        ));
    }
    report_text.push_str(&threshold_core::stats::delta_ci_markdown(
        &baseline_id_str,
        &delta_cis,
    ));
    report_text.push_str(&threshold_core::stats::consistency_markdown(
        &consistency_rows,
        pass_k,
    ));
    if !alarms.is_empty() {
        report_text.push_str("\n## Meta-eval alarms\n\n");
        for a in &alarms {
            let kind = a.get("kind").and_then(Value::as_str).unwrap_or("?");
            let detail = a.get("detail").and_then(Value::as_str).unwrap_or("?");
            report_text.push_str(&format!("- **{kind}**: {detail}\n"));
        }
        let arena_note = format!(
            "# Draft arena-iteration note (promote to a backlog ticket)\n\nRun: {}  mode: {mode_c}\n\n{}",
            exp_dir.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            alarms
                .iter()
                .map(|a| format!(
                    "- {}: {}\n",
                    a.get("kind").and_then(Value::as_str).unwrap_or("?"),
                    a.get("detail").and_then(Value::as_str).unwrap_or("?")
                ))
                .collect::<String>()
        );
        let _ = std::fs::write(exp_dir.join("arena-findings.md"), arena_note);
    }
    if saturated {
        report_text = format!(
            "> **WARNING — arena saturated.** The one-shot probe scored {probe_mean} vs oracle \
             {oracle_mean}; rewards here cannot rank agent configurations. Fix the arena before \
             trusting this table.\n\n{report_text}"
        );
    }
    let cutoffs_path = exp_dir.join("candidate-cutoffs.jsonl");
    if cutoffs_path.exists() {
        let cutoffs_text = std::fs::read_to_string(&cutoffs_path).unwrap_or_default();
        if !cutoffs_text.is_empty() {
            report_text.push_str("\n## Candidate cutoffs\n\n");
            for line in cutoffs_text.lines() {
                if let Ok(c) = serde_json::from_str::<Value>(line) {
                    let cid = c.get("candidate_id").and_then(Value::as_str).unwrap_or("?");
                    let split = c.get("split").and_then(Value::as_str).unwrap_or("?");
                    let errors = c.get("errors").and_then(Value::as_u64).unwrap_or(0);
                    let limit = c
                        .get("max_errors_per_candidate")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    report_text.push_str(&format!(
                        "- `{cid}` skipped split `{split}` after {errors} errors (limit {limit}).\n"
                    ));
                }
            }
        }
    }
    if !optimizer_costs.is_empty() {
        report_text.push_str(&format!(
            "\n## Spend accounting\n\nKnown spend including optimizer calls, certification, and holdout: \
             ${total_known_spend:.4}.\n"
        ));
    }
    let _ = std::fs::write(exp_dir.join("report.md"), &report_text);

    // Pareto JSON
    let mut sorted_cert2: Vec<String> = certified.iter().cloned().collect();
    sorted_cert2.sort();
    let pareto_arr: Vec<Value> = front2
        .iter()
        .map(|cid| {
            let c = cands2.get(cid).cloned().unwrap_or_default();
            serde_json::json!({
                "candidate_id": cid,
                "composition_hash": c.get("hash").cloned().unwrap_or(Value::Null),
                "reward_mean": c.get("reward_mean").cloned().unwrap_or(Value::Null),
                "cost_usd_total": c.get("cost").cloned().unwrap_or(Value::Null),
                "cost_usd_per_trial": c.get("cost_per_trial").cloned().unwrap_or(Value::Null),
                "wall_mean_s": c.get("wall_mean").cloned().unwrap_or(Value::Null),
                "trials": c.get("trials").cloned().unwrap_or(Value::Null),
                "certified": certified.contains(cid),
                "recommended": pick.as_deref() == Some(cid),
                "reward_delta_ci": ci_values.get(cid).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    let _ = std::fs::write(
        exp_dir.join("pareto.json"),
        serde_json::to_string_pretty(&Value::Array(pareto_arr)).unwrap(),
    );

    // loop.json
    let mut outcome_obj = match outcome {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    outcome_obj.insert("alarms".to_string(), Value::Array(alarms));
    outcome_obj.insert(
        "recommended".to_string(),
        pick.as_deref().map(Value::from).unwrap_or(Value::Null),
    );
    outcome_obj.insert(
        "pareto_front".to_string(),
        Value::Array(front2.iter().map(|s| Value::from(s.as_str())).collect()),
    );
    outcome_obj.insert(
        "certified".to_string(),
        Value::Array(
            sorted_cert2
                .iter()
                .map(|s| Value::from(s.as_str()))
                .collect(),
        ),
    );
    outcome_obj.insert(
        "spend_known_usd".to_string(),
        Value::from(total_known_spend),
    );
    outcome_obj.insert(
        "reward_delta_baseline".to_string(),
        baseline_id
            .as_deref()
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    outcome_obj.insert("reward_delta_cis".to_string(), Value::Object(ci_values));
    outcome_obj.insert("consistency".to_string(), Value::Object(consistency_values));
    outcome_obj.insert(
        "consistency_floor".to_string(),
        Value::from(consistency_floor),
    );
    outcome_obj.insert("min_effect".to_string(), Value::from(min_effect));
    // Backlog 056: the reliability gate's inputs and verdict, so a run records
    // which certified candidates were deployable and which were demoted.
    outcome_obj.insert(
        "reliability_floor".to_string(),
        Value::from(reliability_floor),
    );
    outcome_obj.insert("recommendable".to_string(), {
        let mut rec: Vec<String> = recommendable.iter().cloned().collect();
        rec.sort();
        Value::Array(rec.into_iter().map(Value::from).collect())
    });
    outcome_obj.insert("reliability_demoted".to_string(), {
        let mut dem = demoted_unreliable.clone();
        dem.sort();
        Value::Array(dem.into_iter().map(Value::from).collect())
    });
    outcome_obj.insert(
        "trial_complete".to_string(),
        Value::Array({
            let mut tc: Vec<String> = trial_certified.iter().cloned().collect();
            tc.sort();
            tc.into_iter().map(Value::from).collect()
        }),
    );
    let _ = std::fs::write(
        exp_dir.join("loop.json"),
        serde_json::to_string_pretty(&Value::Object(outcome_obj.clone())).unwrap(),
    );

    // Lineage
    let lineage_text = threshold_core::lineage::render(&exp_dir);
    let _ = std::fs::write(exp_dir.join("lineage.md"), lineage_text);

    // Visual companion to report.md: a self-contained static report.html, drawn
    // from the same trials.jsonl plus the loop.json verdict and rig.json just
    // written. Emitted last so it reads the run's certified set and CIs (044).
    if let Ok(html) = threshold_core::report_html::render_html(&exp_dir) {
        let _ = std::fs::write(exp_dir.join("report.html"), html);
    }

    let notebook = repo.join("runs").join("NOTEBOOK.md");
    if !notebook.exists() {
        let _ = std::fs::write(
            &notebook,
            "# Lab notebook\n\nOne entry per run: what was tried, what was \
             learned. lineage.md in each run dir has the full story.\n",
        );
    }
    {
        use std::io::Write;
        let entry = threshold_core::lineage::notebook_entry(
            &exp_dir,
            &Value::Object(spec.clone()),
            &Value::Object(arena_cfg.clone()),
        );
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&notebook) {
            let _ = write!(f, "{entry}");
        }
    }

    let stop_reason = outcome_obj
        .get("stop_reason")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let cert_label = if pick
        .as_deref()
        .map(|p| certified.contains(p))
        .unwrap_or(false)
    {
        ""
    } else {
        "  (UNCERTIFIED)"
    };

    println!();
    println!("stop reason: {stop_reason}");
    println!(
        "recommended: {}{}",
        pick.as_deref().unwrap_or("none"),
        cert_label
    );
    let mut sorted_cert3: Vec<String> = certified.iter().cloned().collect();
    sorted_cert3.sort();
    println!(
        "certified:   {}",
        if sorted_cert3.is_empty() {
            "none".to_string()
        } else {
            format!("{sorted_cert3:?}")
        }
    );
    println!("known spend: ${total_known_spend}");
    println!(
        "experiment:  {}",
        exp_dir.strip_prefix(&repo).unwrap_or(&exp_dir).display()
    );

    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// Helper trait extension for Value → Object
// ---------------------------------------------------------------------------

trait IntoObject {
    fn into_object(self) -> Option<Map<String, Value>>;
}

impl IntoObject for Value {
    fn into_object(self) -> Option<Map<String, Value>> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn regression_dry_run_writes_rust_cli_replay_command() {
        let root = tempdir("regression-dry-run");
        let spec = root.join("taskspec.toml");
        let delivery = root.join("deliveries").join("pr-review");
        let exp_dir = root.join("runs").join("regression-test");

        fs::create_dir_all(&delivery).unwrap();
        fs::write(
            &spec,
            r#"
id = "pr-review"

[inputs]
fixtures = "arenas/pr-review-v0"
"#,
        )
        .unwrap();

        let code = cmd_regression(&delivery, &spec, 3, Some(&exp_dir), true);
        assert_eq!(code, ExitCode::SUCCESS);

        let command = fs::read_to_string(exp_dir.join("regression-command.txt")).unwrap();
        assert!(command.starts_with("cargo run --quiet --bin threshold -- regression "));
        assert!(command.contains("--spec "));
        assert!(command.contains("--trials 3"));
        assert!(command.contains("--exp-dir "));
        assert!(!command.contains("--dry-run"));
        let retired_python = ["python", "3"].concat();
        let retired_runner = ["runner", &["run", "py"].join(".")].join("/");
        assert!(!command.contains(&retired_python));
        assert!(!command.contains(&retired_runner));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn arena_freeze_probe_override_writes_temp_manifest() {
        let root = tempdir("arena-freeze-probe");
        let candidates = root.join("candidates");
        let out = root.join("runs").join("freeze");
        fs::create_dir_all(&candidates).unwrap();
        fs::write(
            candidates.join("probe-oneshot.toml"),
            r#"id = "probe-oneshot"
kind = "oneshot"
model = "moonshotai/kimi-k2.6"
prompt_packet = "packets/reviewer-v1.md"
max_tokens = 8192
"#,
        )
        .unwrap();

        let manifest = freeze_probe_manifest(
            &root,
            &out,
            Some("deepseek/deepseek-v4-pro"),
            Some(1_000_000),
        )
        .unwrap();
        let text = fs::read_to_string(manifest).unwrap();

        assert!(text.contains("model = \"deepseek/deepseek-v4-pro\""));
        assert!(text.contains("context_window = 1000000"));
        assert!(text.contains("prompt_packet = \"packets/reviewer-v1.md\""));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn arena_freeze_probe_manifest_requires_probe() {
        let root = tempdir("arena-freeze-missing-probe");
        let out = root.join("runs").join("freeze");
        let err = freeze_probe_manifest(&root, &out, None, None).unwrap_err();

        assert!(err.to_string().contains("missing one-shot probe manifest"));

        fs::remove_dir_all(root).unwrap();
    }

    fn tempdir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "threshold-cli-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
