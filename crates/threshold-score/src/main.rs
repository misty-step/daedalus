//! Drop-in Rust replacement for `runner/score.py`.
//!
//! Usage (mirrors the Python `__main__` block exactly):
//!   threshold-score <findings.json> <expected.json>
//!
//! Prints the score JSON to stdout (with indent=2, matching Python's
//! `json.dumps(..., indent=2)`). Exits non-zero on bad arguments.
//! A missing or malformed findings file is not a non-zero exit — it
//! scores 0, mirroring the Python behaviour.

use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: threshold-score <findings.json> <expected.json>");
        process::exit(1);
    }

    let findings_path = Path::new(&args[1]);
    let expected_path = Path::new(&args[2]);

    match threshold_core::score::score(findings_path, expected_path) {
        Ok(result) => {
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}
