//! The `daedalus` CLI — Rust port of `bin/daedalus`.
//!
//! Every subcommand delegates to its counterpart in `daedalus_core`; this file
//! is pure composition root (arg parsing + wiring). See `docs/rust-migration.md`
//! for migration status.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{Map, Value};

use daedalus_core::mutate::call_optimizer;
use daedalus_core::pyrandom::PyRandom;
use daedalus_core::run::{summarize, toml_to_json, ArenaInputs};
use daedalus_core::search_loop::{
    is_reference, known_spend, run_search, SearchParams, SearchWorld,
};

// ---------------------------------------------------------------------------
// Top-level CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "daedalus", about = "Daedalus agent foundry CLI")]
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
    /// Export a delivery as control-plane artifacts.
    Export {
        delivery: PathBuf,
        #[arg(long)]
        spec: PathBuf,
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
    /// Port an arena into Harbor format (Rust replacement for runner/port_harbor.py).
    PortHarbor {
        arena: PathBuf,
        #[arg(long, default_value = "harbor-build")]
        out: String,
        /// Path to the prebuilt daedalus-score musl binary.
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
        #[arg(long, default_value = "moonshotai/kimi-k2.6")]
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
        /// only when its (candidate − null) reward-delta 95% CI lower bound
        /// exceeds this. 0.0 = "provably better than the floor."
        #[arg(long, default_value_t = 0.0)]
        min_effect: f64,
        /// Reward a trial must reach to count as a "pass" for the reliability
        /// (pass-rate / pass^k) metric. 1.0 = perfect trials only; lower it to
        /// discriminate mid-tier candidates.
        #[arg(long, default_value_t = 1.0)]
        consistency_floor: f64,
        #[arg(long)]
        max_errors_per_candidate: Option<usize>,
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
        Cmd::Export { delivery, spec } => cmd_export(&delivery, &spec),
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
            max_errors_per_candidate,
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
            max_errors_per_candidate,
        ),
    }
}

// ---------------------------------------------------------------------------
// score
// ---------------------------------------------------------------------------

fn cmd_score(findings: &std::path::Path, expected: &std::path::Path) -> ExitCode {
    match daedalus_core::score::score(findings, expected) {
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
    match daedalus_core::trace::write_trace(run_dir) {
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
    match daedalus_core::export::export_delivery(delivery, &spec_json, None, None, &repo) {
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
    match daedalus_core::swarm::export_suite(delivery, &suite_json, None, &repo) {
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
    match daedalus_core::launch::write_import_packet(delivery, plane, dry_run, None, out_dir, &repo)
    {
        Ok(out) => {
            println!("import_packet: {}", out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            // Check if it's an UnsignedLaunchError
            if e.downcast_ref::<daedalus_core::launch::UnsignedLaunchError>()
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

    let stamp = daedalus_core::run::utc_stamp();
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
            "python3 runner/run.py --candidate {} --arena {} --exp-dir {} --split holdout --trials {} --final\n",
            delivery.join("agent.toml").display(),
            fixtures_rel,
            computed_exp_dir.display(),
            trials
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
    match daedalus_core::run::run_arena(inputs) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("regression failed for {}: {e}", delivery.display());
            return ExitCode::FAILURE;
        }
    }
    match daedalus_core::trace::write_trace(&computed_exp_dir) {
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
    match daedalus_core::workbench::scaffold_task(arena, task_id, taskspec) {
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
    match daedalus_core::workbench::validate_arena(arena, probe_run, holdout_burn) {
        Ok(result) => {
            let text = daedalus_core::workbench::render_validation_report(&result);
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
    match daedalus_core::workbench::record_adjudication(
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
    match daedalus_core::workbench::disagreements(findings, expected) {
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
    let report = daedalus_core::taxonomy::validate_taxonomy(taxonomy, suite);
    print!("{}", daedalus_core::taxonomy::render_report(&report));
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
    let checks = daedalus_core::doctor::run_checks(&repo, today_tuple, stale_days, true);
    print!("{}", daedalus_core::doctor::render(&checks));
    if daedalus_core::doctor::has_failures(&checks) {
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
    use daedalus_core::port_harbor::port_task;

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
    max_errors_per_candidate: Option<usize>,
) -> ExitCode {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // A minimum detectable effect is a non-negative reward delta; the is_nan
    // arm also rejects garbage. A negative MDE would certify candidates provably
    // *worse* than the floor — the opposite of what certification means.
    if min_effect < 0.0 || min_effect.is_nan() {
        eprintln!(
            "error: --min-effect must be >= 0 (got {min_effect}); it is the minimum reward \
             delta a candidate must provably beat the null floor by to certify."
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

    let arena_cfg_val: Value = match daedalus_core::run::load_toml(&arena_dir.join("arena.toml")) {
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

    let stamp = daedalus_core::run::utc_stamp();
    let spec_id = spec.get("id").and_then(Value::as_str).unwrap_or("unknown");
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
        if let Err(e) = daedalus_core::run::run_arena(inputs) {
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
    let probe_mean = rig2
        .get("probe-oneshot")
        .and_then(|s| s.get("reward_mean"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let saturated = probe_mean >= oracle_mean - 0.1;

    let rig_json = serde_json::json!({
        "oracle_mean": oracle_mean,
        "null_mean": null_mean,
        "probe_mean": probe_mean,
        "saturated": saturated,
    });
    let _ = std::fs::write(
        exp_dir.join("rig.json"),
        serde_json::to_string_pretty(&rig_json).unwrap(),
    );

    if saturated {
        println!(
            "!! ARENA SATURATED: probe scored {probe_mean} vs oracle {oracle_mean}. \
             This arena cannot rank agent configurations."
        );
        if !allow_saturated {
            eprintln!("aborting search on a saturated arena (--allow-saturated to override)");
            return ExitCode::FAILURE;
        }
    }

    // ── Stage 2: seed population ────────────────────────────────────────────
    println!("== stage 2: seed population (landscape scan)");

    let packets_dir = exp_dir.join("packets");
    let manifests_dir = exp_dir.join("manifests");

    let seed_result = daedalus_core::seed::seed_population(
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

    let (seeds, seed_meta) = match seed_result {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("seed_population: {e}");
            return ExitCode::FAILURE;
        }
    };

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

            let (manifest_path, meta) = daedalus_core::mutate::propose(
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
                if let Err(e) = daedalus_core::run::run_arena(inputs) {
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
                let _ = daedalus_core::run::run_arena(inputs);
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

    let cands = daedalus_core::report::aggregate(&records_map);
    let front = daedalus_core::report::pareto_front(&cands);

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
            let _ = daedalus_core::run::run_arena(inputs);
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
            let row = daedalus_core::workbench::format_holdout_ledger_row(
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
    let cands2 = daedalus_core::report::aggregate(&final_records_raw);
    let front2 = daedalus_core::report::pareto_front(&cands2);

    // Trial-count gate: ≥ certify_trials per search task — the mechanical floor.
    let trial_certified: std::collections::HashSet<String> = cands2
        .iter()
        .filter(|(cid, c)| {
            let kind = c.get("kind").and_then(Value::as_str).unwrap_or("");
            !["null", "oracle", "oneshot"].contains(&kind)
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

    // Backlog 039 child-1 + child-2: a cluster-robust 95% CI on (candidate −
    // null floor), and certification gated on it. A candidate certifies only if
    // it clears the trial count AND its CI lower bound exceeds the minimum
    // detectable effect — the foundry can *prove* it beats the floor, not merely
    // rank it. Tasks cluster per-task until 040 lands `source_repo` labels.
    let cluster_of = |t: &str| t.to_string();
    let (certified_vec, underpowered) = daedalus_core::stats::partition_certified(
        &cands2,
        &trial_certified,
        "null",
        &cluster_of,
        min_effect,
    );
    let certified: std::collections::HashSet<String> = certified_vec.into_iter().collect();

    let pick = if !certified.is_empty() {
        daedalus_core::report::recommend(&cands2, &front2, Some(&certified))
    } else {
        None
    };

    // CI table covers every trial-complete candidate; the sig column (CI
    // excludes the MDE) is what distinguishes certified from underpowered.
    let (baseline_id, delta_cis) =
        daedalus_core::stats::certified_delta_cis(&cands2, &trial_certified, "null", &cluster_of);
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
    let consistency_rows: Vec<(String, daedalus_core::stats::Consistency)> = consistency_ids
        .iter()
        .filter_map(|cid| {
            cands2.get(cid).map(|c| {
                (
                    cid.clone(),
                    daedalus_core::stats::candidate_consistency(c, consistency_floor),
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
    let mut report_text = daedalus_core::report::render(&cands2, &front2, pick.as_deref());
    if !certified.is_empty() {
        let mut sorted_cert: Vec<String> = certified.iter().cloned().collect();
        sorted_cert.sort();
        report_text.push_str(&format!(
            "\n_Certified (≥{certify_trials} trials per search task AND 95% CI lower bound > {min_effect:+.4} vs the null floor): {}. Recommendation restricted to certified candidates._\n",
            sorted_cert.join(", ")
        ));
    }
    if !underpowered.is_empty() {
        report_text.push_str(&format!(
            "\n_Trial-complete but NOT certified ({} trials, but the reward-delta CI spans the {min_effect:+.4} minimum effect — no provable win over the floor): {}. See the CI table; raise --certify-trials or task count, or widen the arena (040)._\n",
            certify_trials,
            underpowered.join(", ")
        ));
    }
    if certified.is_empty() && !trial_certified.is_empty() {
        report_text.push_str(
            "\n> **No candidate is provably better than the null floor.** Every trial-complete \
             candidate's 95% reward-delta CI spans the minimum detectable effect — the tournament \
             is underpowered, not necessarily the candidates. Add trials/tasks (see the power note) \
             or accept a wider MDE before trusting a ranking.\n",
        );
    }
    report_text.push_str(&daedalus_core::stats::delta_ci_markdown(
        &baseline_id_str,
        &delta_cis,
    ));
    report_text.push_str(&daedalus_core::stats::consistency_markdown(
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
    let lineage_text = daedalus_core::lineage::render(&exp_dir);
    let _ = std::fs::write(exp_dir.join("lineage.md"), lineage_text);

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
        let entry = daedalus_core::lineage::notebook_entry(
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
