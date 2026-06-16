//! CLI parity smoke tests: run both `python3 bin/daedalus <cmd>` and
//! `cargo run -p daedalus-cli -- <cmd>` for a set of deterministic subcommands
//! and assert their stdout matches line-by-line.
//!
//! Tests are skipped if `python3` is absent (CI without Python runtime).
//! Delivery files are never overwritten — the CLI re-exports to committed
//! paths, which is idempotent for these subcommands.

use std::path::PathBuf;
use std::process::Command;

/// Return the workspace root (the directory containing `Cargo.toml`).
fn repo_root() -> PathBuf {
    // Walk up from CARGO_MANIFEST_DIR until we find the workspace Cargo.toml
    let start = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = start.as_path();
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(text) = std::fs::read_to_string(&candidate) {
                if text.contains("[workspace]") {
                    return dir.to_path_buf();
                }
            }
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => return start,
        }
    }
}

/// Path to the compiled `daedalus` binary produced by `cargo test`.
fn rust_bin() -> PathBuf {
    // CARGO_BIN_EXE_daedalus is set by cargo test for [[bin]] entries.
    if let Some(path) = option_env!("CARGO_BIN_EXE_daedalus") {
        return PathBuf::from(path);
    }
    // Fallback: look in the target directory.
    let root = repo_root();
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    root.join("target").join(profile).join("daedalus")
}

fn python3_available() -> bool {
    Command::new("python3").arg("--version").output().is_ok()
}

fn run_python(repo: &PathBuf, args: &[&str]) -> String {
    let out = Command::new("python3")
        .arg(repo.join("bin/daedalus"))
        .args(args)
        .current_dir(repo)
        .output()
        .expect("python3 bin/daedalus failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn run_rust(repo: &PathBuf, args: &[&str]) -> String {
    let bin = rust_bin();
    let out = Command::new(&bin)
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap_or_else(|e| panic!("daedalus binary failed: {e}: {}", bin.display()));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn assert_parity(label: &str, py: &str, rs: &str) {
    if py == rs {
        return;
    }
    // Compare line-by-line for better diffs in test output
    let py_lines: Vec<&str> = py.lines().collect();
    let rs_lines: Vec<&str> = rs.lines().collect();
    if py_lines == rs_lines {
        return; // trailing newline difference only
    }
    panic!("{label}: stdout mismatch\n--- python ---\n{py}\n--- rust ---\n{rs}");
}

// ---------------------------------------------------------------------------
// doctor
// ---------------------------------------------------------------------------

#[test]
fn test_doctor_parity() {
    if !python3_available() {
        eprintln!("python3 not found — skipping doctor parity test");
        return;
    }
    let repo = repo_root();
    let py = run_python(&repo, &["doctor", "--today", "2026-06-16"]);
    let rs = run_rust(&repo, &["doctor", "--today", "2026-06-16"]);
    assert_parity("doctor --today 2026-06-16", &py, &rs);
}

// ---------------------------------------------------------------------------
// trace
// ---------------------------------------------------------------------------

#[test]
fn test_trace_parity() {
    if !python3_available() {
        eprintln!("python3 not found — skipping trace parity test");
        return;
    }
    let repo = repo_root();
    // Use the first available run directory that has a trials.jsonl
    let run_dir = repo.join("runs/20260610T015204Z-search-pr-review-v0");
    if !run_dir.join("trials.jsonl").exists() {
        eprintln!("run dir not found — skipping trace parity test");
        return;
    }
    let run_rel = "runs/20260610T015204Z-search-pr-review-v0";

    // Python: `bin/daedalus trace <run_dir>`
    let py = run_python(&repo, &["trace", run_rel]);
    // Rust: `daedalus trace --run-dir <run_dir>`
    let rs = run_rust(&repo, &["trace", "--run-dir", run_rel]);

    // Both should print "trace: <path>" — compare the path suffix
    let py_path = py.trim().strip_prefix("trace: ").unwrap_or(py.trim());
    let rs_path = rs.trim().strip_prefix("trace: ").unwrap_or(rs.trim());
    assert_eq!(
        py_path, rs_path,
        "trace path mismatch: python={py_path} rust={rs_path}"
    );
}

// ---------------------------------------------------------------------------
// arena-disagreements
// ---------------------------------------------------------------------------

#[test]
fn test_arena_disagreements_parity() {
    if !python3_available() {
        eprintln!("python3 not found — skipping disagreements parity test");
        return;
    }
    let repo = repo_root();
    let tmp = std::env::temp_dir();

    // Write minimal test fixtures
    let findings = tmp.join("parity_findings.json");
    let expected = tmp.join("parity_expected.json");
    std::fs::write(
        &findings,
        r#"{"findings": [{"file": "src/cart.js", "line": 3, "category": "logic", "description": "test"}]}"#,
    )
    .unwrap();
    std::fs::write(
        &expected,
        r#"{"defects": [{"id": "d1", "file": "src/cart.js", "line_start": 1, "line_end": 5, "category": "logic"}]}"#,
    )
    .unwrap();

    let findings_str = findings.to_str().unwrap();
    let expected_str = expected.to_str().unwrap();

    let py = run_python(
        &repo,
        &[
            "arena-disagreements",
            "--findings",
            findings_str,
            "--expected",
            expected_str,
        ],
    );
    let rs = run_rust(
        &repo,
        &[
            "arena-disagreements",
            "--findings",
            findings_str,
            "--expected",
            expected_str,
        ],
    );

    // Parse both as JSON arrays and compare
    let py_json: serde_json::Value = serde_json::from_str(&py).expect("python output is not JSON");
    let rs_json: serde_json::Value = serde_json::from_str(&rs).expect("rust output is not JSON");
    assert_eq!(
        py_json, rs_json,
        "arena-disagreements JSON mismatch: python={py_json} rust={rs_json}"
    );
}

// ---------------------------------------------------------------------------
// taxonomy-validate
// ---------------------------------------------------------------------------

#[test]
fn test_taxonomy_validate_parity() {
    if !python3_available() {
        eprintln!("python3 not found — skipping taxonomy-validate parity test");
        return;
    }
    let repo = repo_root();
    let taxonomy = "docs/review-swarm-taxonomy.md";
    let suite = "specs/pr-review-suite/taskspec.toml";

    if !repo.join(taxonomy).exists() || !repo.join(suite).exists() {
        eprintln!("taxonomy or suite not found — skipping taxonomy-validate parity test");
        return;
    }

    let py = run_python(&repo, &["taxonomy-validate", taxonomy, "--suite", suite]);
    let rs = run_rust(&repo, &["taxonomy-validate", taxonomy, "--suite", suite]);
    assert_parity("taxonomy-validate", &py, &rs);
}

// ---------------------------------------------------------------------------
// export — copy delivery to a tmp dir so we never overwrite committed files
// ---------------------------------------------------------------------------

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap().flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap();
        let dest = dst.join(name);
        if path.is_dir() {
            copy_dir_all(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).unwrap();
        }
    }
}

#[test]
fn test_export_parity() {
    if !python3_available() {
        eprintln!("python3 not found — skipping export parity test");
        return;
    }
    let repo = repo_root();
    let src_delivery = repo.join("deliveries/pr-review");
    let spec = "specs/pr-review/taskspec.toml";

    if !src_delivery.exists() || !repo.join(spec).exists() {
        eprintln!("delivery or spec not found — skipping export parity test");
        return;
    }

    // Copy the delivery into a temp dir so we don't mutate committed files.
    let tmp_delivery = std::env::temp_dir().join("daedalus-parity-export-test-pr-review");
    if tmp_delivery.exists() {
        std::fs::remove_dir_all(&tmp_delivery).ok();
    }
    copy_dir_all(&src_delivery, &tmp_delivery);

    let tmp_delivery_str = tmp_delivery.to_str().unwrap();

    let py = run_python(&repo, &["export", tmp_delivery_str, "--spec", spec]);
    let rs = run_rust(&repo, &["export", tmp_delivery_str, "--spec", spec]);

    // Both print "kind: path" lines — compare just the output key names
    let py_keys: Vec<&str> = py.lines().filter_map(|l| l.split(':').next()).collect();
    let rs_keys: Vec<&str> = rs.lines().filter_map(|l| l.split(':').next()).collect();

    assert_eq!(
        py_keys, rs_keys,
        "export output keys differ: python={py_keys:?} rust={rs_keys:?}"
    );
}
