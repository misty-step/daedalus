//! The `daedalus` command-line tool — Rust port of `bin/daedalus`.
//!
//! Subcommands are added as the underlying modules reach parity in
//! `daedalus-core`. Today: `score`, mirroring the standalone `runner/score.py`
//! CLI. Argument parsing stays hand-rolled until the surface grows enough to
//! warrant `clap`; see docs/rust-migration.md.

use std::path::PathBuf;
use std::process::ExitCode;

use daedalus_core::score::score;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("score") => cmd_score(&args[2..]),
        Some("trace") => cmd_trace(&args[2..]),
        Some(other) => {
            eprintln!("daedalus: unknown command '{other}'");
            usage();
            ExitCode::from(2)
        }
        None => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn usage() {
    eprintln!("usage: daedalus <command> [args]");
    eprintln!("  score <findings.json> <expected.json>   score findings against an answer key");
    eprintln!("  trace <exp-dir>                          write trace.otel.json for an experiment");
}

fn cmd_score(rest: &[String]) -> ExitCode {
    let [findings, expected] = rest else {
        eprintln!("usage: daedalus score <findings.json> <expected.json>");
        return ExitCode::from(2);
    };
    match score(&PathBuf::from(findings), &PathBuf::from(expected)) {
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

fn cmd_trace(rest: &[String]) -> ExitCode {
    let [exp_dir] = rest else {
        eprintln!("usage: daedalus trace <exp-dir>");
        return ExitCode::from(2);
    };
    match daedalus_core::trace::write_trace(&PathBuf::from(exp_dir)) {
        Ok(out) => {
            println!("wrote {}", out.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
