//! Parity oracle for the doctor port.
//!
//! For each fixture, run BOTH the original Python `runner/doctor.py` functions
//! and the Rust port over identical inputs and assert the outputs agree.
//! The Python is called via `python3 -c "..."` with injected `today` so results
//! are fully deterministic.
//!
//! Skips (does not fail) when python3 is unavailable, mirroring `bin/gate`.
//!
//! ## Parity cases covered
//!
//! 1. **fresh-primitives-known-harness** — fresh date, known harness, g3 unsigned →
//!    model-primitives=ok, harness-versions=ok, parallel-pi=ok, approvals=warn.
//! 2. **stale-primitives-unknown-harness** — old date, harness="unknown" →
//!    model-primitives=fail, harness-versions=fail.
//! 3. **signed-approvals** — g3_signed=true → approvals=ok.
//! 4. **missing-approvals-file** — g3_approval path does not exist → approvals=fail.
//! 5. **run-artifacts-warn** — file under runs/*/artifacts/ → run-artifacts=warn.
//! 6. **no-deliveries** — deliveries/ absent → approvals=ok, harness-versions=ok.
//!
//! ## Boundary checks (unit-tested in doctor.rs, not parity-tested here)
//!
//! - **no-primitives-file** — Python's `_check_parallel_pi` calls
//!   `(repo / "docs" / "primitives.md").read_text()` without an existence guard and
//!   raises `FileNotFoundError` when the file is absent; this is a Python bug that
//!   does not exist in the Rust port. The Rust behavior (return fail) is covered
//!   by the `check_primitives` unit test in `doctor::tests`.
//! - `check_run_artifacts` git subprocess path (live I/O, not injectable).
//! - Live clock path in `run_checks(today=None)` (wall time, non-deterministic).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::doctor::{has_failures, render, run_checks, Check};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/daedalus-core")
        .to_path_buf()
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Fixture writer (mirrors write_minimal_repo in test_doctor.py)
// ---------------------------------------------------------------------------

fn tmpdir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "daedalus-doctor-parity-{}-{}-{n}",
        std::process::id(),
        tag
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct RepoOpts<'a> {
    primitive_date: &'a str,
    harness: &'a str,
    g3_signed: bool,
    /// Override the g3_approval path in the contract (relative to repo root).
    /// If None, uses "approvals/G3-demo.md" (the default).
    g3_approval_override: Option<&'a str>,
    /// Whether to write the approvals/G3-demo.md file.
    write_approval_file: bool,
    /// Whether to write docs/primitives.md at all.
    write_primitives: bool,
    /// Whether to write the deliveries/ directory.
    write_deliveries: bool,
    /// Extra file to create under runs/ (relative to repo root, e.g. "runs/exp/artifacts/f.txt").
    run_artifact: Option<&'a str>,
}

impl Default for RepoOpts<'_> {
    fn default() -> Self {
        RepoOpts {
            primitive_date: "2026-06-10",
            harness: "0.78.1",
            g3_signed: false,
            g3_approval_override: None,
            write_approval_file: true,
            write_primitives: true,
            write_deliveries: true,
            run_artifact: None,
        }
    }
}

fn write_repo(tmp: &Path, opts: &RepoOpts<'_>) -> PathBuf {
    // docs/primitives.md
    if opts.write_primitives {
        let docs = tmp.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(
            docs.join("primitives.md"),
            format!(
                "Verified live against the OpenRouter `/api/v1/models` endpoint on\n\
                 **{}**.\n\n\
                 Run pi trials **sequentially** per machine; parallel pi can deadlock.\n",
                opts.primitive_date
            ),
        )
        .unwrap();
    }

    if opts.write_deliveries {
        // deliveries/demo/contract.toml
        let delivery = tmp.join("deliveries").join("demo");
        fs::create_dir_all(&delivery).unwrap();

        let approval_ref = opts.g3_approval_override.unwrap_or("approvals/G3-demo.md");
        let signed_str = if opts.g3_signed { "true" } else { "false" };
        fs::write(
            delivery.join("contract.toml"),
            format!(
                r#"contract = 1
agent = "demo"

[composition]
harness = "pi"
harness_version = "{harness}"

[approval]
g3_signed = {signed_str}
g3_approval = "{approval_ref}"
"#,
                harness = opts.harness,
            ),
        )
        .unwrap();

        // approvals/G3-demo.md
        if opts.write_approval_file {
            let approvals = tmp.join("approvals");
            fs::create_dir_all(&approvals).unwrap();
            fs::write(approvals.join("G3-demo.md"), "**Status:** pending\n").unwrap();
        }
    }

    // runs/
    fs::create_dir_all(tmp.join("runs")).unwrap();

    // optional run artifact
    if let Some(rel) = opts.run_artifact {
        let full = tmp.join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, "raw\n").unwrap();
    }

    tmp.to_path_buf()
}

// ---------------------------------------------------------------------------
// Python driver: returns Vec of (name, status, message) triples
// ---------------------------------------------------------------------------

/// Call Python `doctor.run_checks(repo, today=date(y,m,d), stale_days=30, use_git=False)`
/// and return the list of (name, status, message) tuples as JSON.
fn py_run_checks(
    root: &Path,
    repo: &Path,
    today: (i64, u32, u32),
) -> Vec<(String, String, String)> {
    let (y, m, d) = today;
    let snippet = format!(
        "import sys, json; sys.path.insert(0,'runner'); \
         from datetime import date; \
         import doctor; \
         checks = doctor.run_checks('{repo}', today=date({y},{m},{d}), stale_days=30, use_git=False); \
         print(json.dumps([{{'name': c.name, 'status': c.status, 'message': c.message}} for c in checks]))",
        repo = repo.display()
    );
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("python3 did not emit valid JSON");
    v.as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["name"].as_str().unwrap().to_string(),
                item["status"].as_str().unwrap().to_string(),
                item["message"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// Call Python `doctor.render(checks)` and return the string.
fn py_render(root: &Path, repo: &Path, today: (i64, u32, u32)) -> String {
    let (y, m, d) = today;
    let snippet = format!(
        "import sys; sys.path.insert(0,'runner'); \
         from datetime import date; \
         import doctor; \
         checks = doctor.run_checks('{repo}', today=date({y},{m},{d}), stale_days=30, use_git=False); \
         sys.stdout.write(doctor.render(checks))",
        repo = repo.display()
    );
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("python3 output is utf-8")
}

/// Call Python `doctor.has_failures(checks)` and return the bool.
fn py_has_failures(root: &Path, repo: &Path, today: (i64, u32, u32)) -> bool {
    let (y, m, d) = today;
    let snippet = format!(
        "import sys, json; sys.path.insert(0,'runner'); \
         from datetime import date; \
         import doctor; \
         checks = doctor.run_checks('{repo}', today=date({y},{m},{d}), stale_days=30, use_git=False); \
         print(json.dumps(doctor.has_failures(checks)))",
        repo = repo.display()
    );
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("python3 did not emit valid JSON");
    v.as_bool().unwrap()
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

fn checks_as_triples(checks: &[Check]) -> Vec<(String, String, String)> {
    checks
        .iter()
        .map(|c| (c.name.clone(), c.status.clone(), c.message.clone()))
        .collect()
}

fn assert_parity(label: &str, py: &[(String, String, String)], rust: &[(String, String, String)]) {
    assert_eq!(
        py.len(),
        rust.len(),
        "[{label}] check count differs: py={} rust={}",
        py.len(),
        rust.len()
    );
    for (i, (p, r)) in py.iter().zip(rust.iter()).enumerate() {
        assert_eq!(
            p, r,
            "[{label}] check[{i}] differs\npy =  ({:?}, {:?}, {:?})\nrust= ({:?}, {:?}, {:?})",
            p.0, p.1, p.2, r.0, r.1, r.2
        );
    }
}

// ---------------------------------------------------------------------------
// Parity oracle tests
// ---------------------------------------------------------------------------

#[test]
fn doctor_parity_across_fixtures() {
    if !python_available() {
        eprintln!("skipping doctor parity: python3 not available");
        return;
    }
    let root = repo_root();

    // --- Case 1: fresh primitives, known harness, g3 unsigned ---
    {
        let label = "fresh-primitives-known-harness";
        let dir = tmpdir(label);
        let repo = write_repo(
            &dir,
            &RepoOpts {
                primitive_date: "2026-06-10",
                harness: "0.78.1",
                ..Default::default()
            },
        );
        let today = (2026i64, 6u32, 12u32);

        let py = py_run_checks(&root, &repo, today);
        let rust_checks = run_checks(&repo, Some(today), 30, false);
        let rust = checks_as_triples(&rust_checks);
        assert_parity(label, &py, &rust);

        // Also assert render() and has_failures() agree
        let py_rendered = py_render(&root, &repo, today);
        let rust_rendered = render(&rust_checks);
        assert_eq!(
            py_rendered, rust_rendered,
            "[{label}] render differs\npy=>>>\n{py_rendered}<<<\nrust=>>>\n{rust_rendered}<<<"
        );

        let py_hf = py_has_failures(&root, &repo, today);
        let rust_hf = has_failures(&rust_checks);
        assert_eq!(
            py_hf, rust_hf,
            "[{label}] has_failures differs: py={py_hf} rust={rust_hf}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Case 2: stale primitives and unknown harness ---
    {
        let label = "stale-primitives-unknown-harness";
        let dir = tmpdir(label);
        let repo = write_repo(
            &dir,
            &RepoOpts {
                primitive_date: "2026-04-01",
                harness: "unknown",
                ..Default::default()
            },
        );
        let today = (2026i64, 6u32, 12u32);

        let py = py_run_checks(&root, &repo, today);
        let rust_checks = run_checks(&repo, Some(today), 30, false);
        let rust = checks_as_triples(&rust_checks);
        assert_parity(label, &py, &rust);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Case 3: signed approvals ---
    {
        let label = "signed-approvals";
        let dir = tmpdir(label);
        let repo = write_repo(
            &dir,
            &RepoOpts {
                primitive_date: "2026-06-10",
                harness: "0.78.1",
                g3_signed: true,
                ..Default::default()
            },
        );
        let today = (2026i64, 6u32, 12u32);

        let py = py_run_checks(&root, &repo, today);
        let rust_checks = run_checks(&repo, Some(today), 30, false);
        let rust = checks_as_triples(&rust_checks);
        assert_parity(label, &py, &rust);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Case 4: missing approval file on disk ---
    {
        let label = "missing-approvals-file";
        let dir = tmpdir(label);
        let repo = write_repo(
            &dir,
            &RepoOpts {
                primitive_date: "2026-06-10",
                harness: "0.78.1",
                g3_signed: false,
                write_approval_file: false, // approval file does NOT exist
                ..Default::default()
            },
        );
        let today = (2026i64, 6u32, 12u32);

        let py = py_run_checks(&root, &repo, today);
        let rust_checks = run_checks(&repo, Some(today), 30, false);
        let rust = checks_as_triples(&rust_checks);
        assert_parity(label, &py, &rust);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Case 5: run artifacts warn ---
    {
        let label = "run-artifacts-warn";
        let dir = tmpdir(label);
        let repo = write_repo(
            &dir,
            &RepoOpts {
                primitive_date: "2026-06-10",
                harness: "0.78.1",
                run_artifact: Some("runs/exp/artifacts/response.txt"),
                ..Default::default()
            },
        );
        let today = (2026i64, 6u32, 12u32);

        let py = py_run_checks(&root, &repo, today);
        let rust_checks = run_checks(&repo, Some(today), 30, false);
        let rust = checks_as_triples(&rust_checks);
        assert_parity(label, &py, &rust);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Case 6: no deliveries directory ---
    {
        let label = "no-deliveries";
        let dir = tmpdir(label);
        let repo = write_repo(
            &dir,
            &RepoOpts {
                write_deliveries: false,
                ..Default::default()
            },
        );
        let today = (2026i64, 6u32, 12u32);

        let py = py_run_checks(&root, &repo, today);
        let rust_checks = run_checks(&repo, Some(today), 30, false);
        let rust = checks_as_triples(&rust_checks);
        assert_parity(label, &py, &rust);

        let _ = fs::remove_dir_all(&dir);
    }
}
